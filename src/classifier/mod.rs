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

    /// L2 verdict for a file, always returning a concrete classification.
    ///
    /// Confident results come back as `Keep`/`Trash`; the uncertain band is
    /// reported as `Uncertain(score)` so a caller can show a decision before
    /// the LLM has run.
    pub fn classify_l2(&self, file: &FileMeta) -> (Classification, String) {
        if let Some(verdict) = self.heuristics.classify(file) {
            return verdict;
        }
        let score = self.heuristics.score(file);
        (
            Classification::Uncertain(score),
            format!("L2 heuristics: uncertain (score {score:.2})"),
        )
    }

    /// L3 LLM verdict for a file. Returns `None` when no LLM is configured or
    /// it declines (e.g. the local model is unreachable).
    pub fn classify_llm(&self, file: &FileMeta) -> Option<(Classification, String)> {
        self.llm.as_ref()?.classify(file)
    }

    /// Configured LLM model name, for display. `None` when no LLM is set up.
    pub fn llm_model_name(&self) -> Option<&str> {
        self.llm.as_ref().map(LlmClient::model_name)
    }
}

/// Common trait for classifier layers
pub trait ClassifierLayer {
    /// Classification, or `None` if this layer declines to decide
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)>;
}

// @todo impl ClassifierLayer for Rules/Heuristics/LlmClient
