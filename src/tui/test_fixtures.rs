use std::path::PathBuf;

use chrono::Utc;

use crate::cache::ClassificationCache;
use crate::classifier::Classifier;
use crate::config::RulesConfig;
use crate::models::FileMeta;
use crate::scanner::ScanResult;

/// Build a [`Classifier`] backed by a throwaway sled cache and no LLM.
///
/// The temp dir is intentionally leaked: the sled DB inside the returned
/// `Classifier` keeps files open for the test's lifetime, so deleting it early
/// would break things on Windows. Test artifacts are ephemeral.
pub(super) fn classifier() -> Classifier {
    let dir = tempfile::tempdir().unwrap();
    let cache = ClassificationCache::open(dir.path()).unwrap();
    std::mem::forget(dir);
    Classifier::new(&RulesConfig::default(), None, cache)
}

/// Build a [`FileMeta`] with the given path, size, and extension
pub(super) fn file(path: &str, size: u64, ext: Option<&str>) -> FileMeta {
    FileMeta {
        path: PathBuf::from(path),
        size,
        ext: ext.map(|e| e.to_string()),
        mtime: Utc::now(),
    }
}

/// Assemble a [`ScanResult`] from a list of files (no skips)
pub(super) fn scan_of(files: Vec<FileMeta>) -> ScanResult {
    ScanResult { files, skipped: 0 }
}
