//! Core data models

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result of classifying a file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Classification {
    /// Definitely keep
    Keep,
    /// Definitely delete
    Trash,
    /// Needs a closer look; score 0.0..=1.0 is confidence it's trash
    Uncertain(f32),
}

impl Classification {
    /// Whether this needs no LLM
    pub fn is_final(&self) -> bool {
        !matches!(self, Classification::Uncertain(_))
    }
}

/// File metadata collected by the scanner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    /// Full path
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Lowercase ext without the dot; `None` if none
    pub ext: Option<String>,
    /// Last modification time
    pub mtime: DateTime<Utc>,
}

impl FileMeta {
    /// Human-readable size
    pub fn human_size(&self) -> String {
        human_readable_size(self.size)
    }
}

/// Bytes → human-readable string (powers of 1024)
pub fn human_readable_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

/// Per-directory aggregate stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirAggregate {
    /// Directory path
    pub path: PathBuf,
    /// Total size of files inside (recursive)
    pub total_size: u64,
    /// File count
    pub file_count: usize,
    /// (ext, size) breakdown
    pub by_extension: Vec<(String, u64)>,
}

/// Per-file recommendation: classification + rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// The file
    pub file: FileMeta,
    /// Keep / Trash / Uncertain
    pub classification: Classification,
    /// Deciding layer (L1/L2/L3)
    pub source: ClassifierLevel,
    /// Human-readable reason
    pub reason: String,
}

/// Which layer issued the verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassifierLevel {
    /// Hard rules
    L1Rules,
    /// Heuristics
    L2Heuristics,
    /// Local LLM
    L3Llm,
}

/// Top-N largest files (Stage 1 MVP)
pub struct TopFiles {
    pub entries: Vec<FileMeta>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_is_final_matches_spec() {
        // final verdicts skip the LLM
        assert!(Classification::Keep.is_final());
        assert!(Classification::Trash.is_final());
        // Uncertain always needs a closer look
        assert!(!Classification::Uncertain(0.0).is_final());
        assert!(!Classification::Uncertain(0.5).is_final());
        assert!(!Classification::Uncertain(0.99).is_final());
    }

    #[test]
    fn human_readable_size_boundary_values() {
        assert_eq!(human_readable_size(0), "0 B");
        assert_eq!(human_readable_size(1), "1 B");
        assert_eq!(human_readable_size(512), "512 B");
        assert_eq!(human_readable_size(1023), "1023 B");
    }

    #[test]
    fn human_readable_size_units() {
        assert_eq!(human_readable_size(1024), "1.00 KiB");
        assert_eq!(human_readable_size(1024 * 1024), "1.00 MiB");
        assert_eq!(human_readable_size(1024u64.pow(3)), "1.00 GiB");
        assert_eq!(human_readable_size(1024u64.pow(4)), "1.00 TiB");
    }

    #[test]
    fn human_readable_size_fractional() {
        // 1 500 000 bytes ≈ 1.43 MiB
        assert_eq!(human_readable_size(1_500_000), "1.43 MiB");
        // 2.5 KiB
        assert_eq!(human_readable_size(2 * 1024 + 512), "2.50 KiB");
    }
}
