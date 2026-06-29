//! Загрузка конфигурации правил из TOML (Этап 2).
//!
//! "Правила конфигурируемые (TOML)" — ключевое решение проекта.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Корневой конфиг rustle (десериализуется из TOML).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Настройки сканера ФС.
    #[serde(default)]
    pub scanner: ScannerConfig,
    /// Правила классификатора L1/L2.
    #[serde(default)]
    pub rules: RulesConfig,
    /// Настройки локальной LLM (Этап 3).
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Каталоги, которые всегда пропускаются (например, `.git`).
    #[serde(default)]
    pub ignore_dirs: Vec<String>,
    /// Расширения, которые всегда пропускаются.
    #[serde(default)]
    pub ignore_exts: Vec<String>,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            ignore_dirs: Vec::new(),
            ignore_exts: Vec::new(),
        }
    }
}

/// Пользовательские правила для L1 (расширения/паттерны) и L2 (эвристики).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RulesConfig {
    /// Whitelist расширений (точно Keep).
    #[serde(default)]
    pub keep_exts: Vec<String>,
    /// Blacklist расширений (точно Trash).
    #[serde(default)]
    pub trash_exts: Vec<String>,
    /// Паттерны имён мусора (thumbs.db, .DS_Store, ...).
    #[serde(default)]
    pub trash_patterns: Vec<String>,
    /// Порог уверенности для LLM: если Uncertain(score) и score < threshold,
    /// файл уходит в L3.
    #[serde(default = "default_threshold")]
    pub llm_threshold: f32,
}

fn default_threshold() -> f32 {
    0.5
}

/// Настройки подключения к локальной LLM (Ollama / llama.cpp).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// URL endpoint Ollama (по умолчанию http://localhost:11434).
    pub endpoint: String,
    /// Имя модели (например, "qwen2.5:7b" или "llama3:8b").
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: "qwen2.5:7b".to_string(),
        }
    }
}

impl Config {
    /// Загрузить конфиг из TOML-файла.
    ///
    /// @todo: реализовать чтение и парсинг TOML через `toml::from_str`.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let _ = path;
        todo!("прочитать TOML из {:?} и десериализовать в Config", path)
    }

    /// Сохранить конфиг в TOML-файл (для команды `init`).
    ///
    /// @todo: реализовать сериализацию через `toml::to_string`.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let _ = path;
        todo!("сериализовать Config в TOML и записать в {:?}", path)
    }

    /// Найти конфиг по умолчанию: `./rustle.toml`, затем `~/.config/rustle/`.
    ///
    /// @todo: реализовать поиск в стандартных расположениях.
    pub fn discover() -> Option<PathBuf> {
        todo!("найти rustle.toml в текущем каталоге и XDG-путях")
    }
}
