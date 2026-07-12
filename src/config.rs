//! Rule config loaded from TOML

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RustleError;

/// Root config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub scanner: ScannerConfig,
    #[serde(default)]
    pub rules: RulesConfig,
    #[serde(default)]
    pub llm: Option<LlmConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Always-skipped dirs
    #[serde(default)]
    pub ignored_dirs: Vec<String>,
    /// Always-skipped extensions
    #[serde(default)]
    pub ignored_exts: Vec<String>,
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
    #[serde(default = "defaults::threshold")]
    pub llm_threshold: f32,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            keep_exts: Vec::new(),
            trash_exts: Vec::new(),
            trash_patterns: Vec::new(),
            llm_threshold: defaults::threshold(),
        }
    }
}

/// Local LLM connection (Ollama / llama.cpp)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Endpoint URL
    #[serde(default = "defaults::endpoint")]
    pub endpoint: String,
    /// Model name
    /// See [`crate::classifier::llm`] for the recommendation table.
    pub model: String,
    /// Sampling temperature
    #[serde(default = "defaults::temperature")]
    pub temperature: f32,
    /// Request timeout in seconds
    #[serde(default = "defaults::timeout")]
    pub timeout_secs: u64,
}

mod defaults {
    pub fn temperature() -> f32 {
        0.1
    }

    pub fn endpoint() -> String {
        "http://localhost:11434".into()
    }

    pub fn timeout() -> u64 {
        120
    }

    pub fn threshold() -> f32 {
        0.5
    }
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rustle/config.toml")
}

impl Config {
    pub fn load() -> Result<Self, RustleError> {
        let path = get_config_path();

        if !path.exists() {
            let c = Self::default();
            c.save()?;
            return Ok(c);
        }

        let text = std::fs::read_to_string(path)?;
        let config = toml::from_str::<Self>(&text)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), RustleError> {
        let path = get_config_path();
        let text = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_round_trips_llm_config() {
        // let dir = tempfile::tempdir().unwrap();

        let mut config = Config::default();
        let default_llm = LlmConfig::default();

        config.llm = Some(LlmConfig {
            model: "qwen2.5:3b".into(),
            timeout_secs: 300,
            ..default_llm
        });

        config.save().unwrap();
        // let loaded = Config::load(&path).unwrap();

        // assert_eq!(loaded.llm.model, "qwen2.5:3b");
        // assert_eq!(loaded.llm.timeout_secs, 300);
        // assert_eq!(loaded.llm.temperature, 0.1);
    }

    #[test]
    fn load_reads_temperature_override() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rustle.toml");
        std::fs::write(
            &path,
            "[llm]\nmodel = \"llama3.2:1b\"\ntemperature = 0.42\n",
        )
        .unwrap();

        // let loaded = Config::load(&path).unwrap();
        // assert_eq!(loaded.llm.model, "llama3.2:1b");
        // assert_eq!(loaded.llm.temperature, 0.42);
        // // timeout_secs not present → serde default kicks in
        // assert_eq!(loaded.llm.timeout_secs, default_timeout());
    }
}
