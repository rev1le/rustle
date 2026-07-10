//! L2: heuristics — size, age, path

use crate::classifier::ClassifierLayer;
use crate::config::RulesConfig;
use crate::models::{Classification, FileMeta};

/// Heuristic thresholds
#[expect(
    dead_code,
    reason = "stub: fields are used once the todo in score/classify is implemented"
)]
pub struct Heuristics {
    /// Files older than this are "stale"
    stale_age_days: u32,
    /// Files larger than this are "suspiciously large"
    large_size_bytes: u64,
}

impl Heuristics {
    pub fn from_config(config: &RulesConfig) -> Self {
        let _ = config;
        Self {
            stale_age_days: 365,
            large_size_bytes: 512 * 1024 * 1024,
        }
    }

    /// Trash score in 0.0..=1.0 from age/size/path signals
    ///
    /// @todo weighted sum: age (older→up), size (large+old→up), path (cache/temp/build→up)
    pub fn score(&self, file: &FileMeta) -> f32 {
        let _ = file;
        todo!("compute the heuristic score for {:?}", file.path)
    }
}

impl ClassifierLayer for Heuristics {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo score → >=0.8 Trash, <=0.2 Keep, else Uncertain for L3
        todo!("apply heuristics to {:?}", file.path)
    }
}
