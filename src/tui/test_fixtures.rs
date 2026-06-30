//! Общие тестовые фикстуры для подмодулей TUI.

use std::path::PathBuf;

use chrono::Utc;

use crate::models::FileMeta;
use crate::scanner::ScanResult;

/// Построить [`FileMeta`] с заданными путём, размером и расширением.
pub(super) fn file(path: &str, size: u64, ext: Option<&str>) -> FileMeta {
    FileMeta {
        path: PathBuf::from(path),
        size,
        ext: ext.map(|e| e.to_string()),
        mtime: Utc::now(),
    }
}

/// Собрать [`ScanResult`] из списка файлов (без пропусков).
pub(super) fn scan_of(files: Vec<FileMeta>) -> ScanResult {
    ScanResult { files, skipped: 0 }
}
