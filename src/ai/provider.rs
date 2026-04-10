use super::quantized_gemma4 as qgemma;
use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use tokenizers::Tokenizer;

use super::tools::DmResponse;
use super::{AiProvider, DmMode};

/// Candle-based provider loading quantized GGUF models.
/// Uses quantized_gemma3 architecture which is compatible with Gemma 4 E2B GGUF.
pub struct CandleProvider {
    model: qgemma::ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    logits_processor: LogitsProcessor,
    model_id: String,
}

impl CandleProvider {
    /// Load a GGUF model file directly
    pub fn load_gguf(
        gguf_path: &str,
        tokenizer_repo: &str,
        hf_token: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let device = Device::Cpu;

        tracing::info!("Loading GGUF model from: {}", gguf_path);

        // Load GGUF file
        let mut file = std::fs::File::open(gguf_path)?;
        let gguf = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(|e| format!("GGUF read error: {}", e))?;

        let model = qgemma::ModelWeights::from_gguf(gguf, &mut file, &device)
            .map_err(|e| format!("Model load error: {}", e))?;

        tracing::info!("Model loaded from GGUF");

        // Download tokenizer from HF
        tracing::info!("Loading tokenizer from: {}", tokenizer_repo);
        let api = if let Some(token) = hf_token {
            hf_hub::api::sync::ApiBuilder::new()
                .with_token(Some(token.to_string()))
                .build()?
        } else {
            hf_hub::api::sync::Api::new()?
        };
        let repo = api.repo(hf_hub::Repo::with_revision(
            tokenizer_repo.to_string(),
            hf_hub::RepoType::Model,
            "main".to_string(),
        ));
        let tokenizer_path = repo.get("tokenizer.json")?;
        let tokenizer =
            Tokenizer::from_file(tokenizer_path).map_err(|e| format!("Tokenizer error: {}", e))?;

        let logits_processor = LogitsProcessor::from_sampling(
            42,
            Sampling::TopK {
                k: 40,
                temperature: 0.7,
            },
        );

        tracing::info!("Ready for inference");

        Ok(Self {
            model,
            tokenizer,
            device,
            logits_processor,
            model_id: gguf_path.to_string(),
        })
    }

    /// Load from HuggingFace model ID (downloads GGUF + tokenizer)
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

        // Determine GGUF repo and file based on model_id
        let (gguf_repo, gguf_file, tokenizer_repo) = match model_id {
            "google/gemma-4-E2B-it" | "gemma-4-e2b" => (
                "unsloth/gemma-4-E2B-it-GGUF",
                "gemma-4-E2B-it-Q4_K_M.gguf",
                "google/gemma-4-E2B-it",
            ),
            "google/gemma-4-E4B-it" | "gemma-4-e4b" => (
                "unsloth/gemma-4-E4B-it-GGUF",
                "gemma-4-E4B-it-Q4_K_M.gguf",
                "google/gemma-4-E4B-it",
            ),
            other => {
                // Assume it's a direct repo with GGUF files
                return Err(
                    format!("Unknown model: {}. Use gemma-4-e2b or gemma-4-e4b", other).into(),
                );
            }
        };

        tracing::info!("Resolving GGUF from: {}/{}", gguf_repo, gguf_file);
        let repo = api.repo(hf_hub::Repo::with_revision(
            gguf_repo.to_string(),
            hf_hub::RepoType::Model,
            "main".to_string(),
        ));
        let gguf_path = repo.get(gguf_file)?;

        Self::load_gguf(gguf_path.to_str().unwrap(), tokenizer_repo, hf_token)
    }

    fn generate_text(
        &mut self,
        prompt: &str,
        max_tokens: usize,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| format!("Tokenize error: {}", e))?;
        let mut tokens = encoding.get_ids().to_vec();
        tracing::info!("Prompt tokens: {} tokens", tokens.len());

        let eos_token = self
            .tokenizer
            .token_to_id("<end_of_turn>")
            .or_else(|| self.tokenizer.token_to_id("</s>"))
            .or_else(|| self.tokenizer.token_to_id("<eos>"))
            .unwrap_or(1);

        let mut generated = Vec::new();

        for index in 0..max_tokens {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let ctxt = &tokens[start_pos..];
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;

            let logits = if tokens.len() > 1 {
                let start_at = tokens.len().saturating_sub(64);
                candle_transformers::utils::apply_repeat_penalty(&logits, 1.1, &tokens[start_at..])?
            } else {
                logits
            };

            let next_token = self.logits_processor.sample(&logits)?;
            if next_token == eos_token {
                break;
            }
            tokens.push(next_token);
            generated.push(next_token);
        }

        let text = self
            .tokenizer
            .decode(&generated, true)
            .map_err(|e| format!("Decode error: {}", e))?;

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
        let output = self.generate_text(prompt, 512)?;

        match serde_json::from_str::<DmResponse>(&output) {
            Ok(response) => Ok(response),
            Err(_) => Ok(DmResponse {
                narration: output,
                tool_calls: vec![],
            }),
        }
    }
}
