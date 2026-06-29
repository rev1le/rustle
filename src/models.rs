//! Основные модели данных проекта.
//!
//! См. раздел "Модель классификации" в ABOUT.md.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Результат классификации файла.
///
/// Из ABOUT.md:
/// ```rust,ignore
/// enum Classification {
///     Keep,
///     Trash,
///     Uncertain(f32), // score → порог вызова LLM
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Classification {
    /// Файл точно нужно оставить.
    Keep,
    /// Файл точно нужно удалить (мусор).
    Trash,
    /// Нужен уточняющий анализ (вызов LLM, если score ниже порога).
    /// `score` в диапазоне 0.0..=1.0 — уверенность в том, что это мусор.
    Uncertain(f32),
}

impl Classification {
    /// Является ли классификация окончательной (не требует LLM).
    pub fn is_final(&self) -> bool {
        !matches!(self, Classification::Uncertain(_))
    }
}

/// Метаданные файла, собираемые сканером ФС.
///
/// Соответствует "Сбор метаданных: path, size, ext, mtime" (Этап 1 MVP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    /// Полный путь к файлу.
    pub path: PathBuf,
    /// Размер в байтах.
    pub size: u64,
    /// Расширение в нижнем регистре без точки (например, "raw", "log").
    /// `None` у файлов без расширения.
    pub ext: Option<String>,
    /// Время последней модификации.
    pub mtime: DateTime<Utc>,
}

impl FileMeta {
    /// Размер в человекочитаемом виде (КиБ, МиБ, ГиБ).
    pub fn human_size(&self) -> String {
        human_readable_size(self.size)
    }
}

/// Перевести размер в байтах в человекочитаемую строку (степени 1024).
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

/// Агрегированная статистика по директории.
///
/// Соответствует "Агрегация по директориям" (Этап 1 MVP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirAggregate {
    /// Путь к директории.
    pub path: PathBuf,
    /// Суммарный размер всех файлов внутри (рекурсивно).
    pub total_size: u64,
    /// Количество файлов.
    pub file_count: usize,
    /// Распределение по расширениям: (ext, размер).
    pub by_extension: Vec<(String, u64)>,
}

/// Итоговая рекомендация по файлу: классификация + обоснование.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Файл, к которому относится рекомендация.
    pub file: FileMeta,
    /// Классификация (Keep / Trash / Uncertain).
    pub classification: Classification,
    /// Уровень классификатора, вынесший решение: L1, L2 или L3.
    pub source: ClassifierLevel,
    /// Человекочитаемая причина (для UI/отчётов).
    pub reason: String,
}

/// Уровень классификатора, вынесший вердикт.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassifierLevel {
    /// Жёсткие правила (расширения, паттерны имён).
    L1Rules,
    /// Эвристики (размер, возраст, путь).
    L2Heuristics,
    /// Локальная LLM.
    L3Llm,
}

/// Топ-N самых больших файлов (Этап 1 MVP).
pub struct TopFiles {
    pub entries: Vec<FileMeta>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_is_final_matches_spec() {
        // Окончательные вердикты не требуют вызова LLM.
        assert!(Classification::Keep.is_final());
        assert!(Classification::Trash.is_final());
        // Uncertain всегда требует уточнения.
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
        // 1 500 000 байт ≈ 1.43 MiB
        assert_eq!(human_readable_size(1_500_000), "1.43 MiB");
        // 2.5 KiB
        assert_eq!(human_readable_size(2 * 1024 + 512), "2.50 KiB");
    }
}
