use std::os::unix::io::AsRawFd;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use std::num::NonZeroU32;

use super::tools::{DmResponse, ToolCall};
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

        let model_params = {
            let mut p = LlamaModelParams::default();
            // Offload ALL layers to Metal GPU — massive speed improvement on Apple Silicon
            p = p.with_n_gpu_layers(999);
            p
        };
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
                ("unsloth/gemma-4-E2B-it-GGUF", "gemma-4-E2B-it-Q8_0.gguf")
            }
            "google/gemma-4-E4B-it" | "gemma-4-e4b" => {
                ("unsloth/gemma-4-E4B-it-GGUF", "gemma-4-E4B-it-Q8_0.gguf")
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
            .with_n_ctx(NonZeroU32::new(4096))
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

        // Temp 0.9: slightly tighter than Google's 1.0 to reduce verbosity
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.9),
            LlamaSampler::top_k(64),
            LlamaSampler::top_p(0.95, 1),
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

impl CandleProvider {
    /// Generate raw text without any parsing — for use by the agent layer
    pub fn generate_raw(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        with_stderr_suppressed(|| self.generate_text(prompt, 768))
    }
}

impl AiProvider for CandleProvider {
    fn name(&self) -> &str {
        &self.model_id
    }

    fn generate(
        &mut self,
        prompt: &str,
        mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Mode-aware token limits (bench tested):
        // Reasoning block: ~250-400 tokens, narration: ~200-400, tool calls: ~50-100 each
        // Must leave enough room for reasoning + narration + tools without cutoff
        let max_tokens = match mode {
            DmMode::CharacterCreation => 1024,
            DmMode::Conversation => 768,
            _ => 1024,
        };

        // Suppress stderr for the entire inference call (Metal shader spam)
        let output = with_stderr_suppressed(|| self.generate_text(prompt, max_tokens))?;

        tracing::debug!("Raw model output ({} chars): {}", output.len(), &output[..output.len().min(300)]);

        // Try native Gemma 4 format first
        let native = super::native_format::parse_response(&output);
        if !native.tool_calls.is_empty() || native.narration.len() > 10 {
            tracing::info!(
                "Native parse: narration={} chars, {} tool calls, reasoning={}",
                native.narration.len(),
                native.tool_calls.len(),
                native.reasoning.is_some(),
            );
            if let Some(ref reasoning) = native.reasoning {
                tracing::info!("Agent reasoning: {}", reasoning);
            }
            return Ok(DmResponse {
                narration: native.narration,
                tool_calls: native.tool_calls,
            });
        }

        // Fall back to JSON parsing
        tracing::info!("Native format parse insufficient, falling back to JSON");
        let parsed = parse_dm_response(&output);
        tracing::info!(
            "JSON parse: narration={} chars, {} tool calls",
            parsed.narration.len(),
            parsed.tool_calls.len()
        );

        Ok(parsed)
    }
}

/// Parse model output into DmResponse. Tries multiple strategies:
/// 1. Direct JSON parse of entire output
/// 2. Extract JSON object from mixed text
/// 3. Parse narration + tool_calls separately
/// 4. Fall back to cleaned text with no tools
pub fn parse_dm_response(raw: &str) -> DmResponse {
    let cleaned = strip_special_tokens(raw);

    // Strategy 1: Direct JSON parse
    if let Ok(response) = serde_json::from_str::<DmResponse>(&cleaned) {
        // Log reasoning if present
        log_reasoning(&cleaned);
        return response;
    }

    // Strategy 2: Find JSON object in the text (model might prefix with text)
    if let Some(json_str) = extract_json_object(&cleaned) {
        if let Ok(response) = serde_json::from_str::<DmResponse>(&json_str) {
            log_reasoning(&json_str);
            return response;
        }

        // Strategy 3: Parse as generic JSON and extract fields
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&json_str) {
            // Log reasoning if present (scratchpad — never shown to player)
            if let Some(reasoning) = obj.get("reasoning").and_then(|v| v.as_str()) {
                tracing::info!("Agent reasoning: {}", reasoning);
            }

            let narration = obj
                .get("narration")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut tool_calls = Vec::new();
            if let Some(tools_arr) = obj.get("tool_calls").and_then(|v| v.as_array()) {
                for tool_val in tools_arr {
                    if let Ok(tool) = serde_json::from_value::<ToolCall>(tool_val.clone()) {
                        tool_calls.push(tool);
                    } else {
                        tracing::warn!("Failed to parse tool call: {}", tool_val);
                    }
                }
            }

            if !narration.is_empty() || !tool_calls.is_empty() {
                return DmResponse {
                    narration,
                    tool_calls,
                };
            }
        }
    }

    // Strategy 4: Extract narration from partial JSON
    if let Some(narration) = extract_narration_field(&cleaned) {
        return DmResponse {
            narration,
            tool_calls: vec![],
        };
    }

    // Strategy 5: Fall back to cleaned text
    let text = clean_model_output(&cleaned);
    DmResponse {
        narration: text,
        tool_calls: vec![],
    }
}

/// Log the reasoning field from model output (scratchpad — never shown to player)
fn log_reasoning(json_str: &str) {
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(reasoning) = obj.get("reasoning").and_then(|v| v.as_str()) {
            if !reasoning.is_empty() {
                tracing::info!("Agent reasoning: {}", reasoning);
            }
        }
    }
}

/// Strip Gemma special tokens from output
fn strip_special_tokens(raw: &str) -> String {
    let mut text = raw.to_string();
    for token in &[
        "<start_of_turn>",
        "<end_of_turn>",
        "<eos>",
        "</s>",
        "<bos>",
        "model\n",
        "user\n",
    ] {
        text = text.replace(token, "");
    }
    text.trim().to_string()
}

/// Find the outermost JSON object { ... } in text
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;

    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Try to extract just the "narration" field value from partial JSON
fn extract_narration_field(text: &str) -> Option<String> {
    let start = text.find("\"narration\"")?;
    let after = &text[start..];
    let colon = after.find(':')?;
    let after_colon = after[colon + 1..].trim();

    if !after_colon.starts_with('"') {
        return None;
    }

    let inner = &after_colon[1..];
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(result),
            '\\' => {
                if let Some(escaped) = chars.next() {
                    match escaped {
                        'n' => result.push('\n'),
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        _ => {
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                }
            }
            _ => result.push(c),
        }
    }
    if !result.is_empty() {
        Some(result)
    } else {
        None
    }
}

/// Clean raw model output: strip JSON artifacts for pure text fallback
pub fn clean_model_output(raw: &str) -> String {
    let mut text = raw.to_string();

    // Strip any remaining JSON syntax
    text = text.replace("{", "").replace("}", "");
    text = text.replace("\"narration\":", "");
    text = text.replace("\"tool_calls\": []", "");
    text = text.replace("\"tool_calls\":[]", "");

    // Remove lines that look like JSON keys
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('"') || !trimmed.contains(':') || trimmed.len() > 60
        })
        .collect();

    let result = lines.join("\n").trim().to_string();
    if result.is_empty() {
        raw.trim().to_string()
    } else {
        result
    }
}
