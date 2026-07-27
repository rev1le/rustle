//! L3: local LLM agent — only for uncertain files

use crate::classifier::ClassifierLayer;
use crate::config::LlmConfig;
use crate::error::RustleError;
use crate::models::{Classification, FileMeta};

use chrono::Utc;
use serde::Deserialize;

/// Recommended small CPU-friendly models.
///
/// For a home machine running on CPU, prefer GGUF-quantized models in the
/// 1–4B range. They comfortably fit in 8–16 GB of RAM and give fast inference
/// through Ollama / llama.cpp. Good picks, from smallest to strongest:
///
/// | Model        | Params | RAM (Q4) | Notes                                   |
/// |--------------|--------|----------|-----------------------------------------|
/// | qwen2.5:0.5b | 0.5B   | ~0.6 GB  | Fastest; okay for simple yes/no files   |
/// | llama3.2:1b  | 1.2B   | ~1.3 GB  | Meta edge model; strong for its size    |
/// | qwen2.5:1.5b | 1.5B   | ~1.7 GB  | Great instruction following + JSON      |
/// | gemma2:2b    | 2.6B   | ~2.5 GB  | Google; solid reasoning                 |
/// | qwen2.5:3b   | 3B     | ~2.8 GB  | Best quality/speed balance on CPU       |
/// | llama3.2:3b  | 3B     | ~2.8 GB  | Meta; good JSON obedience               |
/// | phi3.5       | 3.8B   | ~3.5 GB  | Microsoft; excellent at reasoning       |
/// | qwen2.5:7b   | 7B     | ~5.5 GB  | Strong but noticeably slower on CPU     |
///
/// `qwen2.5:3b` is the default — the sweet spot for home CPUs.
const MAX_SIBLINGS: usize = 30;

/// Client for a local LLM (Ollama, GGUF: Qwen / Llama 3 / Gemma / Phi).
///
/// HTTP calls require the `llm` cargo feature (`reqwest`). Without it, `ask`
/// returns an error and `classify` declines (`None`), so the cascade simply
/// keeps the heuristic verdict.
/// Client for a local LLM (Ollama, GGUF: Qwen / Llama 3 / Gemma / Phi).
///
/// HTTP calls require the `llm` cargo feature (`reqwest`). Without it, `ask`
/// returns an error and `classify` declines (`None`), so the cascade simply
/// keeps the heuristic verdict.
#[derive(Clone)]
pub struct LlmClient {
    /// Endpoint URL (e.g. `http://localhost:11434`).
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    endpoint: String,
    /// Model name (e.g. `qwen2.5:3b`).
    model: String,
    /// Sampling temperature.
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    temperature: f32,
    /// Request timeout, seconds.
    #[cfg_attr(not(feature = "llm"), allow(dead_code))]
    timeout_secs: u64,
}

impl LlmClient {
    /// Build a client from config.
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
            temperature: config.temperature,
            timeout_secs: config.timeout_secs,
        }
    }

    /// Model name (for display/diagnostics).
    pub(crate) fn model_name(&self) -> &str {
        &self.model
    }

    /// Build the context (prompt) for the LLM for a given file.
    /// `{verdict, confidence, reason}`.
    pub fn build_prompt(&self, file: &FileMeta) -> String {
        let path = file.path.display();
        let size = file.human_size();
        let ext = file.ext.as_deref().unwrap_or("(none)");
        let age_days = Utc::now()
            .signed_duration_since(file.mtime)
            .num_days()
            .max(0);
        let siblings = describe_siblings(&file.path);

        let raw_prompt = r#"
            You are a disk-cleanup assistant. Decide whether a file should be KEPT, TRASHED, or is UNCERTAIN. Reply ONLY with JSON:
            {
                "verdict": "keep"|"trash"|"uncertain",
                "confidence": 0.0-1.0,
                "reason": "short"
            }
            Rules of thumb:
            - KEEP: source code, documents, media masters, config, project files, recent work (< 30 days).
            - TRASH: caches, build artifacts, logs, temp files, duplicates, OS junk (Thumbs.db, .DS_Store), stale large downloads.
            - UNCERTAIN: ambiguous extensions, system files, or missing context.
            "#;

        let file_prompt = format!(
            "File: \n
            - path: {path}\n
            - extension: {ext}\n\
            - size: {size}\n\
            - age: {age_days} days since modified\n"
        );

        format!("{raw_prompt}{file_prompt}")
    }

    /// Send a request to the local LLM and parse the verdict.
    pub fn ask(&self, prompt: &str) -> Result<Classification, RustleError> {
        Ok(self.fetch_verdict(prompt)?.to_classification())
    }

    /// POST the prompt to Ollama and decode the raw [`Verdict`].
    fn fetch_verdict(&self, prompt: &str) -> Result<Verdict, RustleError> {
        use std::time::Duration;

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| RustleError::Llm(format!("build HTTP client: {e}")))?;

        let base = self.endpoint.trim_end_matches('/');
        let url = format!("{base}/api/generate");

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "format": verdict_schema(),
            "options": { "temperature": self.temperature },
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| RustleError::Llm(format!("POST {url}: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(RustleError::Llm(format!(
                "Ollama returned {status}: {text}"
            )));
        }

        let payload: GenerateResponse = resp
            .json()
            .map_err(|e| RustleError::Llm(format!("decode Ollama response: {e}")))?;

        parse_verdict(&payload.response)
    }

    // /// Fallback when the `llm` feature is disabled.
    // #[cfg(not(feature = "llm"))]
    // pub fn ask(&self, _prompt: &str) -> Result<Classification, RustleError> {
    //     Err(RustleError::Llm(
    //         "LLM feature is not enabled; rebuild with --features llm".into(),
    //     ))
    // }
}

impl ClassifierLayer for LlmClient {
    fn score(&self, file: &FileMeta) -> f32 {
        let prompt = self.build_prompt(file);
        let verdict = match self.fetch_verdict(&prompt) {
            Ok(v) => v,
            // Network/HTTP/parse failure → no opinion, defer to the cascade.
            Err(_) => return 0.5,
        };
        score_from_verdict(&verdict)
    }
}

fn score_from_verdict(verdict: &Verdict) -> f32 {
    let conf = verdict
        .confidence
        .map(|c| (c as f32).clamp(0.0, 1.0))
        .unwrap_or(0.5);

    let label = verdict
        .verdict
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase);

    match label.as_deref() {
        Some(v) if v.starts_with("trash") || v.starts_with("delete") => conf,
        Some(v) if v.starts_with("keep") => 1.0 - conf,
        _ => 0.5,
    }
}

/// JSON schema for Ollama's `format` field → guaranteed structured output.
fn verdict_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "verdict": {
                "type": "string",
                "enum": ["keep", "trash", "uncertain"]
            },
            "confidence": {
                "type": "number"
            },
            "reason": {
                "type": "string"
            }
        },
        "required": ["verdict", "confidence", "reason"]
    })
}

/// `/api/generate` response envelope (streaming disabled).
// #[cfg(feature = "llm")]
#[derive(Debug, Deserialize)]
struct GenerateResponse {
    /// The model's text output — expected to be a JSON object string.
    response: String,
}

/// Parsed model verdict.
// #[cfg(feature = "llm")]
#[derive(Debug, Deserialize)]
struct Verdict {
    verdict: Option<String>,
    confidence: Option<f64>,
    #[serde(default)]
    reason: Option<String>,
}

// #[cfg(feature = "llm")]
impl Verdict {
    /// Map the model's verdict onto the domain `Classification`.
    fn to_classification(&self) -> Classification {
        let verdict = self
            .verdict
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase);

        match verdict.as_deref() {
            Some(v) if v.starts_with("keep") => Classification::Keep,
            Some(v) if v.starts_with("trash") || v.starts_with("delete") => Classification::Trash,
            // "uncertain", unknown verdict, or missing — let the caller
            // escalate via the cascade.
            _ => Classification::Uncertain,
        }
    }

    /// Reason text for logging/diagnostics (never empty).
    #[expect(dead_code, reason = "useful for future richer reasons in L3")]
    fn reason_or_default(&self) -> &str {
        self.reason.as_deref().unwrap_or("LLM verdict")
    }
}

/// Decode the model's JSON reply, tolerating surrounding prose.
// #[cfg(feature = "llm")]
fn parse_verdict(raw: &str) -> Result<Verdict, RustleError> {
    serde_json::from_str::<Verdict>(raw).or_else(|_| match extract_json_object(raw) {
        Some(slice) => serde_json::from_str::<Verdict>(slice)
            .map_err(|e| RustleError::Llm(format!("parse verdict JSON: {e}"))),
        None => Err(RustleError::Llm(format!("no JSON object found in: {raw}"))),
    })
}

/// Pull the first balanced `{...}` substring out of arbitrary text.
// #[cfg(feature = "llm")]
fn extract_json_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let mut depth = 0;
    for (i, b) in raw[start..].bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..=start + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Build a compact description of the sibling files (same directory).
fn describe_siblings(path: &std::path::Path) -> String {
    let Some(dir) = path.parent() else {
        return "(no parent directory)".into();
    };

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return "(unreadable directory)".into(),
    };

    let mut names = Vec::new();
    let mut exts: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        if names.len() >= MAX_SIBLINGS {
            break;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(ext) = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            && !exts.contains(&ext)
        {
            exts.push(ext);
        }
        names.push(name);
    }

    if names.is_empty() {
        return "(empty directory)".into();
    }

    let examples = names.join(", ");
    if exts.is_empty() {
        examples
    } else {
        format!("extensions=[{}]; {examples}", exts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMeta;
    use chrono::Utc;
    use std::path::PathBuf;

    fn sample_file() -> FileMeta {
        FileMeta {
            path: PathBuf::from("/proj/node_modules/.cache/build.dat"),
            size: 1_500_000,
            ext: Some("dat".into()),
            mtime: Utc::now()
                .checked_sub_signed(chrono::Duration::days(400))
                .unwrap(),
        }
    }

    #[test]
    fn build_prompt_contains_core_signals() {
        let client = LlmClient {
            endpoint: "http://localhost:11434".into(),
            model: "qwen2.5:3b".into(),
            temperature: 0.1,
            timeout_secs: 120,
        };
        let prompt = client.build_prompt(&sample_file());

        assert!(prompt.contains("node_modules"));
        assert!(prompt.contains("dat"));
        assert!(prompt.contains("verdict"));
        assert!(prompt.contains("confidence"));
    }

    // #[cfg(feature = "llm")]
    #[test]
    fn verdict_to_classification_maps_branches() {
        let keep = Verdict {
            verdict: Some("keep".into()),
            confidence: Some(0.95),
            reason: None,
        };
        assert_eq!(keep.to_classification(), Classification::Keep);

        let trash = Verdict {
            verdict: Some("trash".into()),
            confidence: Some(0.9),
            reason: None,
        };
        assert_eq!(trash.to_classification(), Classification::Trash);

        let uncertain = Verdict {
            verdict: Some("uncertain".into()),
            confidence: Some(0.42),
            reason: None,
        };
        assert_eq!(uncertain.to_classification(), Classification::Uncertain);

        // Unknown verdict → Uncertain (confidence is dropped, see `to_classification`).
        let weird = Verdict {
            verdict: Some("maybe".into()),
            confidence: Some(5.0),
            reason: None,
        };
        assert_eq!(weird.to_classification(), Classification::Uncertain);
    }

    // #[cfg(feature = "llm")]
    #[test]
    fn verdict_without_fields_defaults_to_uncertain() {
        let empty = Verdict {
            verdict: None,
            confidence: None,
            reason: None,
        };
        assert_eq!(empty.to_classification(), Classification::Uncertain);
    }

    // #[cfg(feature = "llm")]
    #[test]
    fn parse_verdict_accepts_clean_json() {
        let v = parse_verdict(r#"{"verdict":"trash","confidence":0.8,"reason":"cache"}"#).unwrap();
        assert_eq!(v.to_classification(), Classification::Trash);
    }

    // #[cfg(feature = "llm")]
    #[test]
    fn parse_verdict_recovers_json_from_prose() {
        let raw = "Sure! Here is my decision:\n{\"verdict\":\"keep\",\"confidence\":0.9,\"reason\":\"src\"}\nThanks!";
        let v = parse_verdict(raw).unwrap();
        assert_eq!(v.to_classification(), Classification::Keep);
    }

    // #[cfg(feature = "llm")]
    #[test]
    fn parse_verdict_fails_on_garbage() {
        assert!(parse_verdict("totally not json at all").is_err());
    }

    // #[cfg(feature = "llm")]
    #[test]
    fn extract_json_object_handles_nested_and_text() {
        assert_eq!(
            extract_json_object("x {\"a\": {\"b\": 1}} y"),
            Some("{\"a\": {\"b\": 1}}")
        );
        assert_eq!(extract_json_object("no braces here"), None);
        assert_eq!(extract_json_object("{unbalanced"), None);
    }

    #[test]
    fn score_from_verdict_maps_axes() {
        // trash with confidence 0.9 → 0.9
        let trash = Verdict {
            verdict: Some("trash".into()),
            confidence: Some(0.9),
            reason: None,
        };
        assert!((score_from_verdict(&trash) - 0.9).abs() < 1e-6);

        // keep with confidence 0.9 → 1 - 0.9 = 0.1
        let keep = Verdict {
            verdict: Some("keep".into()),
            confidence: Some(0.9),
            reason: None,
        };
        assert!((score_from_verdict(&keep) - 0.1).abs() < 1e-6);

        // uncertain → 0.5 regardless of confidence
        let uncertain = Verdict {
            verdict: Some("uncertain".into()),
            confidence: Some(0.99),
            reason: None,
        };
        assert!((score_from_verdict(&uncertain) - 0.5).abs() < 1e-6);

        // unknown label → 0.5
        let weird = Verdict {
            verdict: Some("maybe".into()),
            confidence: Some(0.7),
            reason: None,
        };
        assert!((score_from_verdict(&weird) - 0.5).abs() < 1e-6);

        // missing confidence → defaults to 0.5
        let no_conf = Verdict {
            verdict: Some("trash".into()),
            confidence: None,
            reason: None,
        };
        assert!((score_from_verdict(&no_conf) - 0.5).abs() < 1e-6);

        // out-of-range confidence is clamped
        let overflow = Verdict {
            verdict: Some("trash".into()),
            confidence: Some(5.0),
            reason: None,
        };
        assert!((score_from_verdict(&overflow) - 1.0).abs() < 1e-6);
    }
}
