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
            heuristics: Heuristics::from_config(config),
            llm,
            cache,
        }
    }

    /// Classify a file via the L1 → L2 → L3 cascade
    ///
    /// L1: final answer for known ext/patterns
    /// L2: heuristic score; confident → answer
    /// L3: Uncertain → LLM if present, else stays Uncertain
    ///
    /// @todo implement with caching by file hash
    pub fn classify(&self, file: &FileMeta) -> Recommendation {
        let _ = (&self.rules, &self.heuristics, &self.llm, &self.cache);
        todo!("classify {:?}", file.path)
    }
}

/// Common trait for classifier layers
pub trait ClassifierLayer {
    /// Classification, or `None` if this layer declines to decide
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)>;
}

// @todo impl ClassifierLayer for Rules/Heuristics/LlmClient
