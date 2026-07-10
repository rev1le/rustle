//! L3: local LLM agent — only for uncertain files

use crate::classifier::ClassifierLayer;
use crate::config::LlmConfig;
use crate::error::RustleError;
use crate::models::{Classification, FileMeta};

/// Client for a local LLM (Ollama / llama.cpp, GGUF: Qwen / Llama 3).
///
/// Fields are read once the Ollama requests are implemented (Stage 3).
#[expect(
    dead_code,
    reason = "stub: fields are used once the todo in ask/classify is implemented"
)]
pub struct LlmClient {
    /// Endpoint URL.
    endpoint: String,
    /// Model name.
    model: String,
}

impl LlmClient {
    /// Build a client from config.
    ///
    /// @todo: verify that the model is available at the endpoint.
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
        }
    }

    /// Build the context (prompt) for the LLM for a given file.
    ///
    /// @todo: assemble a compact description of the file (path, ext, size,
    /// mtime, neighboring files) and an instruction to return JSON
    /// `{verdict, confidence}`.
    pub fn build_prompt(&self, file: &FileMeta) -> String {
        let _ = file;
        todo!("build the LLM prompt for {:?}", file.path)
    }

    /// Send a request to the local LLM and parse the verdict.
    ///
    /// @todo: POST to `{endpoint}/api/generate` (Ollama format) with the
    /// `self.model` model, parse the response into a `Classification`.
    pub fn ask(&self, _prompt: &str) -> Result<Classification, RustleError> {
        todo!("call the Ollama API and return a Classification")
    }
}

impl ClassifierLayer for LlmClient {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo: build the prompt, call the LLM, cache by file hash.
        todo!("classify {:?} via the local LLM", file.path)
    }
}
