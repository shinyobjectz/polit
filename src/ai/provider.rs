use super::tools::DmResponse;
use super::{AiProvider, DmMode};

/// ONNX Runtime provider for Gemma 4 models
/// Uses the `ort` crate with ONNX models from onnx-community/gemma-4-E*B-it-ONNX
pub struct OnnxProvider {
    model_id: String,
    loaded: bool,
    // Will hold: ort::Session for decoder, embed_tokens
    // Will hold: tokenizers::Tokenizer
}

impl OnnxProvider {
    /// Create provider targeting a specific model
    /// model_id: "onnx-community/gemma-4-E2B-it-ONNX" or "onnx-community/gemma-4-E4B-it-ONNX"
    pub fn new(
        model_id: &str,
        hf_token: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Initializing ONNX provider for {}", model_id);

        // TODO: Download model files via hf-hub
        // let api = hf_hub::api::sync::ApiBuilder::new()
        //     .with_token(Some(hf_token.to_string()))
        //     .build()?;
        // let repo = api.model(model_id.to_string());
        // let embed_path = repo.get("onnx/embed_tokens_q4.onnx")?;
        // let decoder_path = repo.get("onnx/decoder_model_merged_q4.onnx")?;
        // let tokenizer_path = repo.get("tokenizer.json")?;

        // TODO: Load ONNX sessions
        // let embed_session = ort::Session::builder()?
        //     .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
        //     .commit_from_file(embed_path)?;
        // let decoder_session = ort::Session::builder()?
        //     .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
        //     .commit_from_file(decoder_path)?;

        // TODO: Load tokenizer
        // let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)?;

        Ok(Self {
            model_id: model_id.to_string(),
            loaded: false,
        })
    }

    /// Check if model files are already cached locally
    pub fn is_cached(model_id: &str) -> bool {
        // Check ~/.cache/huggingface/hub/ for model files
        if let Some(home) = std::env::var_os("HOME") {
            let cache_dir = std::path::PathBuf::from(home)
                .join(".cache/huggingface/hub")
                .join(format!("models--{}", model_id.replace('/', "--")));
            cache_dir.exists()
        } else {
            false
        }
    }
}

impl AiProvider for OnnxProvider {
    fn name(&self) -> &str {
        &self.model_id
    }

    fn generate(
        &mut self,
        prompt: &str,
        mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        if !self.loaded {
            return Err("ONNX model not yet loaded. Run model download first.".into());
        }

        // TODO: Real inference pipeline:
        // 1. Tokenize prompt
        // 2. Run embed_tokens to get embeddings
        // 3. Run decoder with KV-cache for autoregressive generation
        // 4. Detokenize output
        // 5. Parse JSON response into DmResponse

        Err("ONNX inference not yet implemented".into())
    }
}

/// Model selection options
pub enum GemmaModel {
    /// 2B effective params, fastest, good for development
    E2B,
    /// 4B effective params, better quality, recommended
    E4B,
}

impl GemmaModel {
    pub fn model_id(&self) -> &str {
        match self {
            GemmaModel::E2B => "onnx-community/gemma-4-E2B-it-ONNX",
            GemmaModel::E4B => "onnx-community/gemma-4-E4B-it-ONNX",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            GemmaModel::E2B => "Gemma 4 E2B (2B params, fast)",
            GemmaModel::E4B => "Gemma 4 E4B (4B params, recommended)",
        }
    }
}
