//! L1: hard rules — extensions and name patterns

use crate::classifier::ClassifierLayer;
use crate::config::RulesConfig;
use crate::models::FileMeta;

/// Compiled L1 rules
#[expect(
    dead_code,
    reason = "stub: fields are used once the todo in classify is implemented"
)]
pub struct Rules {
    /// Extensions → Keep
    keep_exts: Vec<String>,
    /// Extensions → Trash
    trash_exts: Vec<String>,
    /// Name patterns → Trash (thumbs.db, .DS_Store)
    trash_patterns: Vec<String>,
}

impl Rules {
    /// Build from TOML config.
    pub fn from_config(config: &RulesConfig) -> Self {
        Self {
            keep_exts: config
                .keep_exts
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            trash_exts: config
                .trash_exts
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            trash_patterns: config.trash_patterns.clone(),
        }
    }
}

impl ClassifierLayer for Rules {
    fn score(&self, file: &FileMeta) -> f32 {
        let _ = file;
        // @todo keep_exts→0.0, trash_exts→1.0, pattern match→1.0, else 0.5
        // so the cascade escalates to L2 heuristics for files we have no
        // strong opinion on.
        todo!("score {:?} against hard rules", file.path)
    }
}
