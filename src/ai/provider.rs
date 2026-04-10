use std::os::unix::io::AsRawFd;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use std::num::NonZeroU32;

use super::tools::DmResponse;
use super::{AiProvider, DmMode};

/// llama.cpp-based provider for Gemma 4 GGUF models.
pub struct CandleProvider {
    model: LlamaModel,
    backend: LlamaBackend,
    model_id: String,
}

/// Suppress stderr (llama.cpp Metal shader spam), run closure, restore
fn with_stderr_suppressed<T, F: FnOnce() -> T>(f: F) -> T {
    let devnull = std::fs::File::open("/dev/null").ok();
    let saved = unsafe { libc::dup(2) };
    if let Some(ref null) = devnull {
        unsafe {
            libc::dup2(null.as_raw_fd(), 2);
        }
    }
    let result = f();
    if saved >= 0 {
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
    result
}

impl CandleProvider {
    pub fn load_gguf(gguf_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Initializing llama.cpp backend...");
        let backend = with_stderr_suppressed(|| {
            LlamaBackend::init().map_err(|e| format!("Backend init: {}", e))
        })?;

        let model_params = LlamaModelParams::default();
        tracing::info!("Loading GGUF: {}", gguf_path);
        let model = with_stderr_suppressed(|| {
            LlamaModel::load_from_file(&backend, gguf_path, &model_params)
                .map_err(|e| format!("Model load: {}", e))
        })?;

        tracing::info!("Model loaded!");
        Ok(Self {
            model,
            backend,
            model_id: gguf_path.to_string(),
        })
    }

    pub fn load(
        model_id: &str,
        hf_token: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let api = if let Some(token) = hf_token {
            hf_hub::api::sync::ApiBuilder::new()
                .with_token(Some(token.to_string()))
                .build()?
        } else {
            hf_hub::api::sync::Api::new()?
        };

        let (gguf_repo, gguf_file) = match model_id {
            "google/gemma-4-E2B-it" | "gemma-4-e2b" => {
                ("unsloth/gemma-4-E2B-it-GGUF", "gemma-4-E2B-it-Q4_K_M.gguf")
            }
            "google/gemma-4-E4B-it" | "gemma-4-e4b" => {
                ("unsloth/gemma-4-E4B-it-GGUF", "gemma-4-E4B-it-Q4_K_M.gguf")
            }
            other => {
                return Err(
                    format!("Unknown model: {}. Use gemma-4-e2b or gemma-4-e4b", other).into(),
                );
            }
        };

        tracing::info!("Resolving GGUF: {}/{}", gguf_repo, gguf_file);
        let repo = api.repo(hf_hub::Repo::with_revision(
            gguf_repo.to_string(),
            hf_hub::RepoType::Model,
            "main".to_string(),
        ));
        let gguf_path = repo.get(gguf_file)?;
        Self::load_gguf(gguf_path.to_str().unwrap())
    }

    fn generate_text(
        &self,
        prompt: &str,
        max_tokens: usize,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(2048))
            .with_n_threads(4)
            .with_n_threads_batch(4);

        let mut ctx = with_stderr_suppressed(|| {
            self.model
                .new_context(&self.backend, ctx_params)
                .map_err(|e| format!("Context: {}", e))
        })?;

        let tokens = self
            .model
            .str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| format!("Tokenize: {}", e))?;

        tracing::info!(
            "Prompt: {} tokens, generating up to {}",
            tokens.len(),
            max_tokens
        );

        // Evaluate prompt
        let mut batch = LlamaBatch::new(tokens.len().max(1), 1);
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch
                .add(token, i as i32, &[0], is_last)
                .map_err(|e| format!("Batch: {}", e))?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("Decode prompt: {}", e))?;

        // Set up sampler: temp → top-k → top-p → dist
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.7),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(42),
        ]);

        // Generate
        let mut output_tokens = Vec::new();
        let mut n_cur = tokens.len() as i32;

        for _ in 0..max_tokens {
            let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(new_token);

            if self.model.is_eog_token(new_token) {
                break;
            }

            output_tokens.push(new_token);

            batch.clear();
            batch
                .add(new_token, n_cur, &[0], true)
                .map_err(|e| format!("Batch gen: {}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Decode gen: {}", e))?;

            n_cur += 1;
        }

        // Detokenize
        let mut text = String::new();
        for &token in &output_tokens {
            match self.model.token_to_str(token, Special::Tokenize) {
                Ok(s) => text.push_str(&s),
                Err(_) => {} // skip special tokens
            }
        }

        Ok(text)
    }
}

impl AiProvider for CandleProvider {
    fn name(&self) -> &str {
        &self.model_id
    }

    fn generate(
        &mut self,
        prompt: &str,
        _mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Suppress stderr for the entire inference call (Metal shader spam)
        let output = with_stderr_suppressed(|| self.generate_text(prompt, 512))?;

        match serde_json::from_str::<DmResponse>(&output) {
            Ok(response) => Ok(response),
            Err(_) => Ok(DmResponse {
                narration: output,
                tool_calls: vec![],
            }),
        }
    }
}
