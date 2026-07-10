//! Rule config loaded from TOML

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RustleError;

/// Root config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Scanner settings
    #[serde(default)]
    pub scanner: IgnoreConfig,
    /// L1/L2 rules
    #[serde(default)]
    pub rules: RulesConfig,
    /// Local LLM settings
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IgnoreConfig {
    /// Always-skipped dirs (e.g. `.git`)
    #[serde(default)]
    pub ignore_dirs: Vec<String>,
    /// Always-skipped extensions
    #[serde(default)]
    pub ignore_exts: Vec<String>,
}

/// User-defined L1 (exts/patterns) and L2 (heuristics) rules
///
/// Manual `Default` so `llm_threshold` is 0.5, not `f32::default()`'s 0.0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
    /// Extensions → Keep
    #[serde(default)]
    pub keep_exts: Vec<String>,
    /// Extensions → Trash
    #[serde(default)]
    pub trash_exts: Vec<String>,
    /// Trash name patterns (thumbs.db, .DS_Store)
    #[serde(default)]
    pub trash_patterns: Vec<String>,
    /// Uncertain score below this goes to L3
    #[serde(default = "default_threshold")]
    pub llm_threshold: f32,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            keep_exts: Vec::new(),
            trash_exts: Vec::new(),
            trash_patterns: Vec::new(),
            llm_threshold: default_threshold(),
        }
    }
}

fn default_threshold() -> f32 {
    0.5
}

/// Local LLM connection (Ollama / llama.cpp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Endpoint URL (default http://localhost:11434)
    pub endpoint: String,
    /// Model name (e.g. qwen2.5:7b)
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
    /// Load from a TOML file
    ///
    /// @todo parse via `toml::from_str`
    pub fn load(path: &Path) -> Result<Self, RustleError> {
        let _ = path;
        todo!("read TOML from {:?} and deserialize into Config", path)
    }

    /// Save to TOML (for the `init` command)
    ///
    /// @todo serialize via `toml::to_string`
    pub fn save(&self, path: &Path) -> Result<(), RustleError> {
        let _ = path;
        todo!("serialize Config to TOML and write to {:?}", path)
    }

    /// Find default config: `./rustle.toml`, then `~/.config/rustle/`
    ///
    /// @todo search standard locations
    pub fn discover() -> Option<PathBuf> {
        todo!("find rustle.toml in the cwd and XDG paths")
    }
}
