//! L1: hard rules — extensions and name patterns

use crate::classifier::ClassifierLayer;
use crate::config::RulesConfig;
use crate::models::{Classification, FileMeta};

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
    ///
    /// Extensions are lowercased for case-insensitive matching; patterns are
    /// stored verbatim (matched as simple `*` globs by the heuristic layer).
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
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo keep_exts→Keep, trash_exts→Trash, pattern match→Trash, else None (→L2)
        todo!("apply hard rules to {:?}", file.path)
    }
}
