//! L3: локальная LLM-агент — только для uncertain-файлов (Этап 3).
//!
//! "Интеграция с локальной LLM (Ollama API)" и "Приватность: LLM работает
//! локально" — ключевое решение проекта. Активируется только за фичей `llm`.

use crate::classifier::ClassifierLayer;
use crate::config::LlmConfig;
use crate::models::{Classification, FileMeta};

/// Клиент к локальной LLM (Ollama / llama.cpp, GGUF: Qwen / Llama 3).
pub struct LlmClient {
    /// URL endpoint.
    endpoint: String,
    /// Имя модели.
    model: String,
}

impl LlmClient {
    /// Создать клиент из конфига.
    ///
    /// @todo: проверить доступность модели на endpoint.
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
        }
    }

    /// Сформировать контекст (промпт) для LLM по файлу.
    ///
    /// @todo: собрать компактное описание файла (path, ext, size, mtime,
    /// соседние файлы) и инструкцию вернуть JSON `{verdict, confidence}`.
    pub fn build_prompt(&self, file: &FileMeta) -> String {
        let _ = file;
        todo!("построить промпт для LLM по {:?}", file.path)
    }

    /// Выполнить запрос к локальной LLM и распарсить вердикт.
    ///
    /// @todo: POST на `{endpoint}/api/generate` (формат Ollama) с моделью
    /// `self.model`, распарсить ответ в `Classification`.
    pub fn ask(&self, _prompt: &str) -> anyhow::Result<Classification> {
        todo!("вызвать Ollama API и вернуть Classification")
    }
}

impl ClassifierLayer for LlmClient {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo: построить промпт, вызвать LLM, кэшировать по хэшу файла.
        todo!("классифицировать {:?} через локальную LLM", file.path)
    }
}
