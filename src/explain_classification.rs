//! Human-readable reason strings for classification verdicts.

use crate::models::Classification;

#[derive(Debug, Clone, Copy)]
pub enum Layer {
    Rules,
    Heuristics,
    Llm,
}

impl Layer {
    fn tag(self) -> String {
        match self {
            Layer::Rules => "L1 rules".into(),
            Layer::Heuristics => "L2 heuristics".into(),
            Layer::Llm => "L3 LLM".into(),
        }
    }
}

/// Reason line for a heuristic verdict.
pub fn heuristics(class: Classification, score: f32) -> String {
    match class {
        Classification::Trash => {
            format!("{}: high trash score ({score:.2})", Layer::Heuristics.tag())
        }
        Classification::Keep => {
            format!("{}: low trash score ({score:.2})", Layer::Heuristics.tag())
        }
        Classification::Uncertain => {
            format!("{}: uncertain (score {score:.2})", Layer::Heuristics.tag())
        }
    }
}

/// Reason line for an LLM verdict, naming the model that produced it.
pub fn llm(model: &str) -> String {
    format!("{} ({model}): verdict from local model", Layer::Llm.tag())
}

/// Reason line for a rules-based verdict (L1).
pub fn rules(class: Classification) -> String {
    format!("{}: {class:?}", Layer::Rules.tag())
}

/// Placeholder line shown when a layer declined to decide (uncertain band).
pub fn declined() -> String {
    "L2 heuristics: uncertain — awaiting LLM".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristics_trash_includes_score() {
        let s = heuristics(Classification::Trash, 0.92);
        assert!(s.contains("L2 heuristics"), "missing tag: {s}");
        assert!(s.contains("0.92"), "missing score: {s}");
    }

    #[test]
    fn heuristics_keep_includes_score() {
        let s = heuristics(Classification::Keep, 0.13);
        assert!(s.contains("L2 heuristics"), "missing tag: {s}");
        assert!(s.contains("0.13"), "missing score: {s}");
    }

    #[test]
    fn heuristics_uncertain_uses_passed_score() {
        // `Uncertain` carries no score anymore — the caller passes it in.
        let s = heuristics(Classification::Uncertain, 0.45);
        assert!(s.contains("0.45"), "missing score: {s}");
        assert!(s.contains("uncertain"), "missing word: {s}");
    }

    #[test]
    fn llm_includes_model_name() {
        let s = llm("qwen2.5:3b");
        assert!(s.contains("L3 LLM"), "missing tag: {s}");
        assert!(s.contains("qwen2.5:3b"), "missing model: {s}");
    }

    #[test]
    fn rules_includes_tag_and_verdict() {
        let s = rules(Classification::Trash);
        assert!(s.contains("L1 rules"), "missing tag: {s}");
        assert!(s.contains("Trash"), "missing verdict: {s}");
    }

    #[test]
    fn declined_is_a_static_placeholder() {
        assert!(declined().contains("uncertain"));
        assert!(declined().contains("awaiting LLM"));
    }
}
