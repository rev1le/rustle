//! Рекурсивный обход ФС и сбор метаданных (Этап 1 MVP).
//!
//! Стек: `walkdir`/`ignore` — обход, `rayon` — параллелизм.

pub mod aggregator;
mod utils;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::{DirEntry, WalkDir};

use crate::config::IgnoreConfig;
use crate::error::RustleError;
use crate::models::FileMeta;

/// Настройки конкретного прохода сканера.
pub struct ScanOptions {
    /// Путь для обхода
    pub root: PathBuf,
    /// Максимальная глубина (None — без ограничений)
    pub max_depth: Option<usize>,
    /// Игнорируемые каталоги/расширения
    pub ignore: IgnoreConfig,
}

/// Результат сканирования: все собранные метаданные файлов.
pub struct ScanResult {
    pub files: Vec<FileMeta>,
    /// Количество пропущенных файлов (нет прав, симлинки и т.п.).
    pub skipped: usize,
}

fn get_file_meta(entry: &DirEntry, ignore_exts: &[String]) -> Option<FileMeta> {
    let is_ignore_ext = |ext: &str| -> bool {
        let ext_lower = ext.to_lowercase();
        return ignore_exts.iter().any(|e| e == &ext_lower);
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

    return Some(FileMeta {
        path: path.to_path_buf(),
        size: meta.len(),
        ext: ext_lower,
        mtime,
    });
}

/// Рекурсивно обойти путь
pub fn scan(options: &ScanOptions) -> Result<ScanResult, RustleError> {
    let IgnoreConfig {
        ref ignore_dirs,
        ref ignore_exts,
    } = options.ignore;

    let ignore_dirs = utils::to_lowercase_vec(ignore_dirs);
    let ignore_exts = utils::to_lowercase_vec(ignore_exts);

    let walker = WalkDir::new(&options.root);
    let walker = match options.max_depth {
        Some(d) => walker.max_depth(d),
        None => walker,
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} {pos} файлов просканировано").unwrap());

    log::info!(
        "сканирование: {} (макс. глубина: {:?})",
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

        return ignore_dirs.iter().any(|ignore_dir| ignore_dir == &name);
    };

    let it = walker
        .into_iter()
        .filter_entry(|entry| !is_ignore_entry(entry));

    for result in it {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                log::debug!("пропуск записи walkdir: {e}");
                skipped += 1;
                continue;
            }
        };

        if let Some(file_meta) = get_file_meta(&entry, &ignore_exts) {
            files.push(file_meta);
        } else {
            let path = entry.path();
            log::debug!("пропуск {}", path.display());
            skipped += 1;
        }
        pb.inc(1);
    }

    pb.finish_with_message(format!("{} файлов", files.len()));
    log::info!(
        "сканирование завершено: {} файлов, {} пропущено",
        files.len(),
        skipped
    );

    Ok(ScanResult { files, skipped })
}
