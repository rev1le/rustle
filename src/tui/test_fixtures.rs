use std::path::PathBuf;

use chrono::Utc;

use crate::models::FileMeta;
use crate::scanner::ScanResult;

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
