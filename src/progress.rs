//! Прогресс-бары через `indicatif` (Этап 1 MVP).
//!
//! Используется при обходе ФС и классификации, чтобы показывать live-прогресс.

use indicatif::{ProgressBar, ProgressStyle};

/// Создать прогресс-бар для сканирования ФС.
///
/// @todo: вернуть `ProgressBar` со стилем "сканирование: {wide_bar} {pos}/{len}".
pub fn scan_progress(_total: u64) -> ProgressBar {
    let _ = ProgressStyle::default_bar();
    todo!("создать прогресс-бар сканирования")
}

/// Создать спиннер для классификации (количество файлов заранее неизвестно).
///
/// @todo: вернуть спиннер со стилем "{spinner} классификация: {pos} файлов".
pub fn classify_spinner() -> ProgressBar {
    let _ = ProgressStyle::default_spinner();
    todo!("создать спиннер классификации")
}
