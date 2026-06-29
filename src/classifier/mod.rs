//! Роутер классификации: оркеструет L1 → L2 → L3.
//!
//! Из ABOUT.md:
//! ```text
//! [роутер классификации]
//!     ├─ L1: жёсткие правила (ext, паттерны)
//!     ├─ L2: эвристики (размер, возраст, путь)
//!     └─ L3: LLM-агент (только uncertain)
//! ```

pub mod heuristics;
pub mod llm;
pub mod rules;

use crate::cache::ClassificationCache;
use crate::classifier::heuristics::Heuristics;
use crate::classifier::llm::LlmClient;
use crate::classifier::rules::Rules;
use crate::config::RulesConfig;
use crate::models::{Classification, ClassifierLevel, FileMeta, Recommendation};

/// Контекст классификатора: скомпилированные правила + LLM + кэш.
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

    /// Классифицировать один файл по цепочке L1 → L2 → L3.
    ///
    /// Логика (см. ABOUT.md):
    ///   1. L1 даёт окончательный ответ для известных ext/паттернов.
    ///   2. Иначе L2 оценивает эвристиками; если уверенно — ответ.
    ///   3. Иначе (Uncertain) — при наличии LLM вызываем её, иначе оставляем
    ///      Uncertain.
    ///
    /// @todo: реализовать каскад L1 → L2 → L3 с кэшированием по хэшу файла.
    pub fn classify(&self, file: &FileMeta) -> Recommendation {
        let _ = (&self.rules, &self.heuristics, &self.llm, &self.cache);
        todo!("классифицировать {:?}", file.path)
    }
}

/// Общий трейт уровней классификатора.
pub trait ClassifierLayer {
    /// Вернуть классификацию или `None`, если уровень не принял решения.
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)>;
}

// Уровни классификатора реализуют `ClassifierLayer`.
// @todo: реализовать `impl ClassifierLayer for Rules/Heuristics/LlmClient`.

/// Утилита: какому `ClassifierLevel` соответствует уровень.
pub(crate) fn level_of(layer: &dyn ClassifierLayer) -> ClassifierLevel {
    // Идём по типу слоя через Any в реальной реализации.
    let _ = layer;
    ClassifierLevel::L1Rules
}
