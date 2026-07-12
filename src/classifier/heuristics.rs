//! L2: heuristics — size, age, path, and extension bias

use std::path::Path;

use chrono::Utc;

use crate::classifier::ClassifierLayer;
use crate::config::RulesConfig;
use crate::models::{Classification, FileMeta};

/// Weight of the age signal in the overall score.
const AGE_WEIGHT: f32 = 0.4;
/// Weight of the size signal in the overall score.
const SIZE_WEIGHT: f32 = 0.3;
/// Weight of the path signal in the overall score.
const PATH_WEIGHT: f32 = 0.3;

/// `score >= this` is confident enough for [`Classification::Trash`].
const TRASH_THRESHOLD: f32 = 0.8;
/// `score <= this` is confident enough for [`Classification::Keep`].
const KEEP_THRESHOLD: f32 = 0.2;

/// How strongly a `keep_exts` match pulls the score toward Keep (0.0).
///
/// A high weight: valuable file types (source, docs, media masters) are
/// protected unless the other signals are overwhelmingly trashy.
const KEEP_EXT_WEIGHT: f32 = 0.8;

/// How strongly a `trash_exts` / `trash_patterns` match pulls the score toward
/// Trash (1.0). Lower than [`KEEP_EXT_WEIGHT`] on purpose: disposable-looking
/// extensions can still matter (active logs), so the pull is softer and the
/// L3 LLM gets a look at borderline cases.
const TRASH_EXT_WEIGHT: f32 = 0.5;

/// Path components that mark a file as disposable.
///
/// Matched case-insensitively against each path segment, so both directory
/// names (`node_modules`) and dotted ones (`.cache`) are recognised.
const TRASHY_SEGMENTS: &[&str] = &[
    "cache",
    ".cache",
    "__pycache__",
    "temp",
    "tmp",
    ".tmp",
    "build",
    "dist",
    "out",
    "target",
    "node_modules",
    ".gradle",
    ".next",
    ".nuxt",
    ".venv",
    "venv",
];

/// Heuristic layer: signal-based trash score refined by configured extensions.
pub struct Heuristics {
    /// User-defined exts/patterns used to bias the score.
    config: RulesConfig,
    /// Files older than this are "stale".
    stale_age_days: u32,
    /// Files larger than this are "suspiciously large".
    large_size_bytes: u64,
}

impl Heuristics {
    /// Build the layer, storing the config for later extension/pattern lookups.
    pub fn new(config: RulesConfig) -> Self {
        Self {
            config,
            stale_age_days: 365,
            large_size_bytes: 512 * 1024 * 1024,
        }
    }

    /// Trash-confidence score in `0.0..=1.0`.
    ///
    /// Starts from a weighted sum of three independent signals:
    /// - **age** (40%): older → more likely disposable
    /// - **size** (30%): larger → costlier to keep
    /// - **path** (30%): cache/temp/build locations → disposable
    ///
    /// Then refined by the file's extension/name against the stored config:
    /// - `keep_exts` → strong pull toward Keep
    /// - `trash_exts` / `trash_patterns` → softer pull toward Trash
    pub fn score(&self, file: &FileMeta) -> f32 {
        let age_days = Utc::now()
            .signed_duration_since(file.mtime)
            .num_days()
            .max(0);
        let age = self.age_signal(age_days);
        let size = self.size_signal(file.size);
        let path = path_signal(&file.path);
        let base = (AGE_WEIGHT * age + SIZE_WEIGHT * size + PATH_WEIGHT * path).clamp(0.0, 1.0);
        self.ext_bias(file, base)
    }

    /// Age signal `0.0..=1.0`: rises linearly, saturating at `stale_age_days`.
    fn age_signal(&self, age_days: i64) -> f32 {
        let stale = self.stale_age_days.max(1) as f32;
        (age_days as f32 / stale).clamp(0.0, 1.0)
    }

    /// Size signal `0.0..=1.0`: rises linearly, saturating at `large_size_bytes`.
    fn size_signal(&self, size: u64) -> f32 {
        let large = self.large_size_bytes.max(1) as f32;
        (size as f32 / large).clamp(0.0, 1.0)
    }

    /// Pull `base` toward Keep/Trash based on the file's extension and name.
    fn ext_bias(&self, file: &FileMeta, base: f32) -> f32 {
        let ext = file.ext.as_deref().filter(|e| !e.is_empty());

        // keep_exts → high-confidence keep: protect valuable file types.
        if ext.is_some_and(|e| {
            self.config
                .keep_exts
                .iter()
                .any(|k| k.eq_ignore_ascii_case(e))
        }) {
            return lerp(base, 0.0, KEEP_EXT_WEIGHT);
        }

        // trash_exts or trash_patterns → softer pull toward trash.
        let trash_ext = ext.is_some_and(|e| {
            self.config
                .trash_exts
                .iter()
                .any(|t| t.eq_ignore_ascii_case(e))
        });
        let trash_name = file_name(&file.path).is_some_and(|name| {
            self.config
                .trash_patterns
                .iter()
                .any(|p| glob_match(&name.to_ascii_lowercase(), &p.to_ascii_lowercase()))
        });
        if trash_ext || trash_name {
            return lerp(base, 1.0, TRASH_EXT_WEIGHT);
        }

        base
    }
}

impl ClassifierLayer for Heuristics {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let score = self.score(file);
        if score >= TRASH_THRESHOLD {
            Some((
                Classification::Trash,
                format!("L2 heuristics: high trash score ({score:.2})"),
            ))
        } else if score <= KEEP_THRESHOLD {
            Some((
                Classification::Keep,
                format!("L2 heuristics: low trash score ({score:.2})"),
            ))
        } else {
            // Uncertain: decline so the L3 LLM gets a chance to decide.
            None
        }
    }
}

/// Linear interpolation toward `target` by factor `t` (`0..=1`).
fn lerp(base: f32, target: f32, t: f32) -> f32 {
    base + t * (target - base)
}

/// Lowercased file name of `path`, or `None` for `/`-ending paths.
fn file_name(path: &Path) -> Option<String> {
    path.file_name().and_then(|n| n.to_str()).map(str::to_owned)
}

/// Path signal `0.0..=1.0`: `1.0` when any path component is disposable.
fn path_signal(path: &Path) -> f32 {
    for comp in path.components() {
        let comp_str = comp.as_os_str().to_string_lossy();
        if TRASHY_SEGMENTS
            .iter()
            .any(|seg| comp_str.eq_ignore_ascii_case(seg))
        {
            return 1.0;
        }
    }
    0.0
}

/// Tiny glob matcher supporting only `*` (match any run of characters).
///
/// Case sensitivity is the caller's responsibility; both sides are expected
/// to be pre-normalised. Implements the standard iterative backtracking
/// wildcard algorithm (LeetCode 44).
fn glob_match(text: &str, pattern: &str) -> bool {
    let text = text.as_bytes();
    let pat = pattern.as_bytes();
    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None; // (pattern idx after '*', text idx)

    while ti < text.len() {
        if pi < pat.len() && pat[pi] == text[ti] {
            ti += 1;
            pi += 1;
        } else if pi < pat.len() && pat[pi] == b'*' {
            star = Some((pi + 1, ti));
            pi += 1;
        } else if let Some((sp, st)) = star {
            pi = sp;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }
    pi == pat.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::path::PathBuf;

    fn heuristics() -> Heuristics {
        Heuristics::new(RulesConfig::default())
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
                .checked_sub_signed(Duration::days(age_days))
                .unwrap(),
        }
    }

    // --- base scoring (no configured exts) ---------------------------------

    #[test]
    fn score_fresh_small_clean_file_is_low() {
        let h = heuristics();
        let f = file_at("/home/me/doc/report.pdf", 1_000, 5);
        let s = h.score(&f);
        assert!(s <= KEEP_THRESHOLD, "expected low score, got {s}");
    }

    #[test]
    fn score_stale_large_trashy_file_is_high() {
        let h = heuristics();
        let f = file_at(
            "/proj/node_modules/.cache/build.dat",
            600 * 1024 * 1024,
            400,
        );
        let s = h.score(&f);
        assert!(s >= TRASH_THRESHOLD, "expected high score, got {s}");
    }

    #[test]
    fn score_is_bounded_to_unit_interval() {
        let h = heuristics();
        let f = file_at("/x/node_modules/.cache/y", u64::MAX, 10_000);
        let s = h.score(&f);
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn classify_keeps_fresh_clean_file() {
        let h = heuristics();
        let f = file_at("/home/me/src/main.rs", 2_000, 3);
        let (class, _) = h.classify(&f).expect("L2 should decide");
        assert_eq!(class, Classification::Keep);
    }

    #[test]
    fn classify_declines_middle_band() {
        let h = heuristics();
        let f = file_at("/home/me/notes/old.txt", 500, 400);
        assert_eq!(h.classify(&f), None);
    }

    // --- extension / pattern bias ------------------------------------------

    #[test]
    fn keep_ext_protects_stale_large_file() {
        let mut cfg = RulesConfig::default();
        cfg.keep_exts.push("rs".into());
        let h = Heuristics::new(cfg);
        // Old + large but clean path: base = age(0.4) + size(0.3) = 0.7, which
        // on its own is Uncertain. With keep_ext it must be pulled to Keep.
        let f = file_at("/home/me/src/legacy.rs", 600 * 1024 * 1024, 400);
        let (class, _) = h.classify(&f).expect("L2 should decide");
        assert_eq!(class, Classification::Keep);
    }

    #[test]
    fn trash_ext_pushes_stale_large_file_to_trash() {
        let mut cfg = RulesConfig::default();
        cfg.trash_exts.push("log".into());
        let h = Heuristics::new(cfg);
        // base 0.7 → trash bias pulls it past the 0.8 threshold.
        let f = file_at("/var/app/server.log", 600 * 1024 * 1024, 400);
        let (class, _) = h.classify(&f).expect("L2 should decide");
        assert_eq!(class, Classification::Trash);
    }

    #[test]
    fn trash_pattern_matches_by_name() {
        let mut cfg = RulesConfig::default();
        // "db" is NOT in trash_exts, so only the pattern can fire.
        cfg.trash_patterns.push("thumbs.db".into());
        let h = Heuristics::new(cfg);
        let f = file_at("/pics/thumbs.db", 600 * 1024 * 1024, 400);
        let (class, _) = h.classify(&f).expect("L2 should decide");
        assert_eq!(class, Classification::Trash);
    }

    #[test]
    fn unknown_extension_leaves_score_unchanged() {
        let mut cfg = RulesConfig::default();
        cfg.keep_exts.push("rs".into());
        let h = Heuristics::new(cfg);
        // ".xyz" is in neither list → behaves like the base heuristic.
        let f = file_at("/home/me/data.xyz", 1_000, 5);
        let s = h.score(&f);
        assert!(s <= KEEP_THRESHOLD, "expected low score, got {s}");
    }

    #[test]
    fn keep_ext_takes_precedence_over_trash() {
        // Defensive: a file somehow listed in both → keep wins (don't delete).
        let mut cfg = RulesConfig::default();
        cfg.keep_exts.push("log".into());
        cfg.trash_exts.push("log".into());
        let h = Heuristics::new(cfg);
        let f = file_at("/var/app/server.log", 600 * 1024 * 1024, 400);
        let (class, _) = h.classify(&f).expect("L2 should decide");
        assert_eq!(class, Classification::Keep);
    }

    // --- signal helpers ----------------------------------------------------

    #[test]
    fn age_signal_saturates_at_threshold() {
        let h = heuristics();
        assert!(h.age_signal(0).abs() < 1e-6);
        assert!((h.age_signal(365) - 1.0).abs() < 1e-6);
        assert!((h.age_signal(1000) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn size_signal_saturates_at_threshold() {
        let h = heuristics();
        assert!(h.size_signal(0).abs() < 1e-6);
        assert!((h.size_signal(512 * 1024 * 1024) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn path_signal_detects_disposable_components() {
        assert_eq!(
            path_signal(std::path::Path::new("/x/node_modules/y.js")),
            1.0
        );
        assert_eq!(path_signal(std::path::Path::new("/x/Cache/z")), 1.0); // case-insensitive
        assert_eq!(
            path_signal(std::path::Path::new("/home/me/src/main.rs")),
            0.0
        );
    }

    #[test]
    fn glob_match_handles_wildcards() {
        assert!(glob_match("thumbs.db", "thumbs.db"));
        assert!(glob_match("foo.tmp", "*.tmp"));
        assert!(glob_match("npm-debug.log.1", "npm-debug.log*"));
        assert!(glob_match("backup~", "*~"));
        assert!(!glob_match("report.pdf", "*.tmp"));
        assert!(!glob_match("foo.txt", "thumbs.db"));
        assert!(glob_match("ab", "a*b"));
        assert!(glob_match("aXb", "a*b"));
    }
}
