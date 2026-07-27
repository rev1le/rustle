//! Classification router: orchestrates L1 → L2 → L3

pub mod heuristics;
pub mod llm;
pub mod rules;

use crate::cache::ClassificationCache;
use crate::classifier::heuristics::Heuristics;
use crate::classifier::llm::LlmClient;
use crate::classifier::rules::Rules;
use crate::config::RulesConfig;
use crate::models::{Classification, FileMeta, Recommendation};

/// Classifier: compiled rules + LLM + cache
pub struct Classifier {
    rules: Rules,
    heuristics: Heuristics,
    llm: Option<LlmClient>,
    cache: ClassificationCache,
}

impl Classifier {
    pub fn new(config: &RulesConfig, llm: Option<LlmClient>, cache: ClassificationCache) -> Self {
        Self {
            rules: Rules::from_config(config),
            heuristics: Heuristics::new(config.clone()),
            llm,
            cache,
        }
    }

    /// Classify a file via the L1 → L2 → L3 cascade
    pub fn classify(&self, file: &FileMeta) -> Recommendation {
        let _ = (&self.rules, &self.heuristics, &self.llm, &self.cache);
        todo!("classify {:?}", file.path)
    }

    pub fn classify_l2(&self, file: &FileMeta) -> Classification {
        self.heuristics.classify(file)
    }

    pub fn score_l2(&self, file: &FileMeta) -> f32 {
        self.heuristics.score(file)
    }

    pub fn classify_llm(&self, file: &FileMeta) -> Option<Classification> {
        let llm = self.llm.as_ref()?;
        Some(llm.classify(file))
    }

    /// Configured LLM model name, for display. `None` when no LLM is set up.
    pub fn llm_model_name(&self) -> Option<&str> {
        self.llm.as_ref().map(LlmClient::model_name)
    }
}

/// `score >= this` is confident enough for [`Classification::Trash`].
pub(crate) const TRASH_THRESHOLD: f32 = 0.8;
/// `score <= this` is confident enough for [`Classification::Keep`].
pub(crate) const KEEP_THRESHOLD: f32 = 0.2;

/// Common trait for classifier layers.
pub trait ClassifierLayer {
    /// Trash-confidence score for `file` in `0.0..=1.0`.
    fn score(&self, file: &FileMeta) -> f32;

    /// Verdict derived from [`score`](Self::score) via the shared thresholds.
    fn classify(&self, file: &FileMeta) -> Classification {
        let score = self.score(file);
        if score >= TRASH_THRESHOLD {
            Classification::Trash
        } else if score <= KEEP_THRESHOLD {
            Classification::Keep
        } else {
            Classification::Uncertain
        }
    }
}

// @todo impl ClassifierLayer for Rules/Heuristics/LlmClient

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ClassificationCache;
    use crate::models::FileMeta;
    use chrono::Utc;
    use std::path::PathBuf;

    fn classifier() -> Classifier {
        let dir = tempfile::tempdir().unwrap();
        let cache = ClassificationCache::open(dir.path()).unwrap();
        std::mem::forget(dir);
        Classifier::new(&RulesConfig::default(), None, cache)
    }

    fn file_at(path: &str, size: u64, age_days: i64) -> FileMeta {
        let path_buf = PathBuf::from(path);
        let ext = path_buf
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        FileMeta {
            path: path_buf,
            size,
            ext,
            mtime: Utc::now()
                .checked_sub_signed(chrono::Duration::days(age_days))
                .unwrap(),
        }
    }

    #[test]
    fn classify_l2_returns_keep_for_fresh_small_file() {
        let clf = classifier();
        // Fresh, small, clean-path file → confident Keep.
        let f = file_at("/home/me/src/main.rs", 2_000, 3);
        assert_eq!(clf.classify_l2(&f), Classification::Keep);
    }

    #[test]
    fn classify_l2_returns_uncertain_in_middle_band() {
        let clf = classifier();
        // Old, small, clean-path file → middle score. L2 must return
        // `Uncertain` (not a confident verdict) so L3 can escalate. The
        // numeric score lives on the layer, not on the variant.
        let f = file_at("/home/me/notes/old.txt", 500, 400);
        assert_eq!(clf.classify_l2(&f), Classification::Uncertain);
    }
}
