use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::gemma4::config::Gemma4TextConfig;
use candle_transformers::models::gemma4::text::TextModel;
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;

use super::tools::DmResponse;
use super::{AiProvider, DmMode};

/// Candle-based provider for Gemma 4 models
pub struct CandleProvider {
    model: TextModel,
    tokenizer: Tokenizer,
    device: Device,
    logits_processor: LogitsProcessor,
    model_id: String,
}

impl CandleProvider {
    /// Load a Gemma 4 model from HuggingFace
    /// model_id: "google/gemma-4-E2B-it" or "google/gemma-4-E4B-it"
    pub fn load(
        model_id: &str,
        hf_token: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Loading Gemma 4 model: {}", model_id);

        let api = if let Some(token) = hf_token {
            hf_hub::api::sync::ApiBuilder::new()
                .with_token(Some(token.to_string()))
                .build()?
        } else {
            Api::new()?
        };

        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));

        // Download tokenizer
        tracing::info!("Downloading tokenizer...");
        let tokenizer_path = repo.get("tokenizer.json")?;
        let tokenizer =
            Tokenizer::from_file(tokenizer_path).map_err(|e| format!("Tokenizer error: {}", e))?;

        // Download model weights
        tracing::info!("Downloading model weights...");
        let config_path = repo.get("config.json")?;
        let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
        let config: Gemma4TextConfig = if let Some(text_cfg) = raw.get("text_config") {
            serde_json::from_value(text_cfg.clone())?
        } else {
            serde_json::from_value(raw)?
        };

        // Find safetensors files
        let filenames = match Self::hub_load_safetensors(&repo) {
            Ok(files) => files,
            Err(_) => vec![repo.get("model.safetensors")?],
        };

        tracing::info!("Loading model into memory...");
        let device = Device::Cpu; // Metal via feature flag
        let dtype = DType::F32;
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, dtype, &device)? };
        let model = TextModel::new(&config, vb)?;

        let logits_processor = LogitsProcessor::from_sampling(
            42,
            Sampling::TopK {
                k: 40,
                temperature: 0.7,
            },
        );

        tracing::info!("Model loaded successfully");

        Ok(Self {
            model,
            tokenizer,
            device,
            logits_processor,
            model_id: model_id.to_string(),
        })
    }

    fn hub_load_safetensors(
        repo: &hf_hub::api::sync::ApiRepo,
    ) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        let index_file = repo.get("model.safetensors.index.json")?;
        let index_data: serde_json::Value = serde_json::from_slice(&std::fs::read(&index_file)?)?;
        let weight_map = index_data
            .get("weight_map")
            .ok_or("No weight_map in index")?
            .as_object()
            .ok_or("weight_map not an object")?;

        let mut files: Vec<String> = weight_map
            .values()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        files.sort();
        files.dedup();

        let paths: Vec<std::path::PathBuf> = files
            .iter()
            .map(|f| repo.get(f))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(paths)
    }

    /// Generate text from a prompt
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

        let eos_token = self
            .tokenizer
            .token_to_id("</s>")
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

            // Apply repeat penalty
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

        // Try to parse as JSON DmResponse, fall back to plain narration
        match serde_json::from_str::<DmResponse>(&output) {
            Ok(response) => Ok(response),
            Err(_) => {
                // Model didn't produce structured JSON — wrap as narration
                Ok(DmResponse {
                    narration: output,
                    tool_calls: vec![],
                })
            }
        }
    }
}

/// Available Gemma 4 models
pub enum GemmaModel {
    /// 2B effective params, fastest
    E2B,
    /// 4B effective params, better quality
    E4B,
}

impl GemmaModel {
    pub fn model_id(&self) -> &str {
        match self {
            GemmaModel::E2B => "google/gemma-4-E2B-it",
            GemmaModel::E4B => "google/gemma-4-E4B-it",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            GemmaModel::E2B => "Gemma 4 E2B (2B params, fast)",
            GemmaModel::E4B => "Gemma 4 E4B (4B params, recommended)",
        }
    }
}
