//! Рекурсивный обход ФС и сбор метаданных (Этап 1 MVP).
//!
//! Стек: `walkdir`/`ignore` — обход, `rayon` — параллелизм.

pub mod aggregator;

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;

use crate::config::ScannerConfig;
use crate::error::RustleError;
use crate::models::FileMeta;

/// Настройки конкретного прохода сканера.
pub struct ScanOptions {
    /// Корень обхода.
    pub root: PathBuf,
    /// Максимальная глубина (None — без ограничений).
    pub max_depth: Option<usize>,
    /// Игнорируемые каталоги/расширения (из TOML-конфига).
    pub ignore: ScannerConfig,
}

/// Результат сканирования: все собранные метаданные файлов.
pub struct ScanResult {
    pub files: Vec<FileMeta>,
    /// Количество пропущенных файлов (нет прав, симлинки и т.п.).
    pub skipped: usize,
}

/// Рекурсивно обойти ФС от `root` и собрать метаданные файлов.
///
/// Использует `walkdir` для обхода. `rayon` оставлен для будущей
/// параллелизации сбора метаданных (см. roadmap). Каталоги/расширения
/// из `options.ignore` отсекаются через `filter_entry` (без спуска внутрь).
pub fn scan(options: &ScanOptions) -> Result<ScanResult, RustleError> {
    let ignore_dirs: Vec<String> = options
        .ignore
        .ignore_dirs
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let ignore_exts: Vec<String> = options
        .ignore
        .ignore_exts
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let walker = WalkDir::new(&options.root).follow_links(false);
    let walker = match options.max_depth {
        Some(d) => walker.max_depth(d),
        None => walker,
    };

    // Спиннер: суммарное число файлов заранее неизвестно.
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} {pos} файлов просканировано").unwrap());

    let mut files = Vec::new();
    let mut skipped = 0usize;

    let it = walker.into_iter().filter_entry(|entry| {
        // Не спускаемся в игнорируемые каталоги.
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            !ignore_dirs.iter().any(|d| d == &name)
        } else {
            true
        }
    });

    for result in it {
        let entry = match result {
            Ok(e) => e,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext_lower = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        // Пропуск игнорируемых расширений (let-chain, edition 2024).
        if let Some(ref ext) = ext_lower
            && ignore_exts.iter().any(|e| e == ext)
        {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let mtime = meta
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());

        files.push(FileMeta {
            path: path.to_path_buf(),
            size: meta.len(),
            ext: ext_lower,
            mtime,
        });
        pb.inc(1);
    }

    pb.finish_with_message(format!("{} файлов", files.len()));

    Ok(ScanResult { files, skipped })
}
