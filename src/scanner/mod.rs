//! Recursive FS traversal and metadata collection

pub mod aggregator;
mod utils;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::{DirEntry, WalkDir};

use crate::config::ScannerConfig;
use crate::error::RustleError;
use crate::models::FileMeta;

/// Scanner pass settings
pub struct ScanOptions {
    /// Path to traverse
    pub root: PathBuf,
    /// Maximum depth (None — unlimited)
    pub max_depth: Option<usize>,
    /// Ignored directories/extensions
    pub ignore: ScannerConfig,
}

/// Collected file metadata
pub struct ScanResult {
    pub files: Vec<FileMeta>,
    /// Skipped (no perms, symlinks, etc.)
    pub skipped: usize,
}

fn get_file_meta(entry: &DirEntry, ignore_exts: &[String]) -> Option<FileMeta> {
    let is_ignore_ext = |ext: &str| -> bool {
        let ext_lower = ext.to_lowercase();
        ignore_exts.iter().any(|e| e == &ext_lower)
    };

    if !entry.file_type().is_file() {
        return None;
    }

    let path = entry.path();
    let ext_lower = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    if let Some(ref ext) = ext_lower
        && is_ignore_ext(ext)
    {
        return None;
    }

    let meta = match entry.metadata() {
        Ok(m) => m,
        Err(_) => return None,
    };
    let mtime = meta
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());

    Some(FileMeta {
        path: path.to_path_buf(),
        size: meta.len(),
        ext: ext_lower,
        mtime,
    })
}

/// Recursively traverse `root`
pub fn scan(options: &ScanOptions) -> Result<ScanResult, RustleError> {
    let ScannerConfig {
        ref ignored_dirs,
        ref ignored_exts,
    } = options.ignore;

    let ignore_dirs = utils::to_lowercase_vec(ignored_dirs);
    let ignore_exts = utils::to_lowercase_vec(ignored_exts);

    let walker = WalkDir::new(&options.root);
    let walker = match options.max_depth {
        Some(d) => walker.max_depth(d),
        None => walker,
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} {pos} files scanned").unwrap());

    log::info!(
        "scanning: {} (max depth: {:?})",
        options.root.display(),
        options.max_depth
    );

    let mut files = Vec::new();
    let mut skipped = 0_usize;

    let is_ignore_entry = |entry: &DirEntry| -> bool {
        if !entry.file_type().is_dir() {
            return false;
        }

        let name = entry.file_name().to_string_lossy().to_lowercase();

        ignore_dirs.iter().any(|ignore_dir| ignore_dir == &name)
    };

    let it = walker
        .into_iter()
        .filter_entry(|entry| !is_ignore_entry(entry));

    for result in it {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                log::debug!("skipping walkdir entry: {e}");
                skipped += 1;
                continue;
            }
        };

        if let Some(file_meta) = get_file_meta(&entry, &ignore_exts) {
            files.push(file_meta);
        } else {
            let path = entry.path();
            log::debug!("skipping {}", path.display());
            skipped += 1;
        }
        pb.inc(1);
    }

    pb.finish_with_message(format!("{} files", files.len()));
    log::info!("scan complete: {} files, {} skipped", files.len(), skipped);

    Ok(ScanResult { files, skipped })
}
