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
    /// Build from TOML config
    ///
    /// @todo lowercase exts, compile patterns to glob/regex for fast matching
    pub fn from_config(config: &RulesConfig) -> Self {
        let _ = config;
        todo!("compile L1 rules from RulesConfig")
    }
}

impl ClassifierLayer for Rules {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo keep_exts→Keep, trash_exts→Trash, pattern match→Trash, else None (→L2)
        todo!("apply hard rules to {:?}", file.path)
    }
}
