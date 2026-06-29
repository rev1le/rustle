//! Генерация отчётов (Этап 4): JSON.
//!
//! "Отчёты (HTML/JSON)" из дорожной карты — пока реализован только JSON.

use std::path::Path;

use serde::Serialize;

use crate::error::RustleError;
use crate::models::{FileMeta, Recommendation, human_readable_size};
use crate::scanner::ScanResult;

/// Сколько самых больших файлов включать в отчёт.
const TOP_FILES_LIMIT: usize = 20;

/// Сводка по результатам сканирования для отчёта.
pub struct Report {
    pub scan: ScanResult,
    pub recommendations: Vec<Recommendation>,
}

impl Report {
    pub fn new(scan: ScanResult, recommendations: Vec<Recommendation>) -> Self {
        Self {
            scan,
            recommendations,
        }
    }

    /// Сгенерировать JSON-отчёт и записать по пути `output`
    /// (или вывести в stdout, если `output` — None).
    pub fn render(&self, output: Option<&Path>) -> Result<(), RustleError> {
        let json = self.render_json()?;
        match output {
            Some(path) => std::fs::write(path, json)?,
            None => println!("{json}"),
        }
        Ok(())
    }

    /// Сериализовать отчёт вpretty JSON.
    ///
    /// Включает сводку (количество/размер), топ самых больших файлов и
    /// рекомендации классификатора. Полный дамп всех файлов не выводится —
    /// отчёт должен быть компактным дайджестом.
    fn render_json(&self) -> Result<String, RustleError> {
        use crate::scanner::aggregator::top_largest;

        let total_size: u64 = self.scan.files.iter().map(|f| f.size).sum();

        let top_files: Vec<JsonFileEntry> = top_largest(&self.scan.files, TOP_FILES_LIMIT)
            .into_iter()
            .map(Into::into)
            .collect();

        let report = JsonReport {
            summary: Summary {
                total_files: self.scan.files.len(),
                skipped: self.scan.skipped,
                total_size_bytes: total_size,
                total_size_human: human_readable_size(total_size),
            },
            top_files,
            recommendations: &self.recommendations,
        };

        Ok(serde_json::to_string_pretty(&report)?)
    }
}

/// Сериализуемое представление отчёта.
#[derive(Serialize)]
struct JsonReport<'a> {
    summary: Summary,
    top_files: Vec<JsonFileEntry>,
    recommendations: &'a [Recommendation],
}

#[derive(Serialize)]
struct Summary {
    total_files: usize,
    skipped: usize,
    total_size_bytes: u64,
    total_size_human: String,
}

/// Запись о файле в отчёте.
#[derive(Serialize)]
struct JsonFileEntry {
    path: String,
    size_bytes: u64,
    size_human: String,
    ext: Option<String>,
}

impl From<FileMeta> for JsonFileEntry {
    fn from(f: FileMeta) -> Self {
        Self {
            path: f.path.to_string_lossy().to_string(),
            size_bytes: f.size,
            size_human: human_readable_size(f.size),
            ext: f.ext,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Classification;
    use crate::scanner::ScanResult;
    use chrono::Utc;
    use std::path::PathBuf;

    fn file(path: &str, size: u64, ext: Option<&str>) -> FileMeta {
        FileMeta {
            path: PathBuf::from(path),
            size,
            ext: ext.map(|e| e.to_string()),
            mtime: Utc::now(),
        }
    }

    fn empty_result(files: Vec<FileMeta>) -> ScanResult {
        ScanResult { files, skipped: 0 }
    }

    #[test]
    fn render_json_contains_summary_and_totals() {
        let report = Report::new(
            empty_result(vec![
                file("/a/big.bin", 1024, Some("bin")),
                file("/a/small.txt", 100, Some("txt")),
            ]),
            Vec::new(),
        );
        let json = report.render_json().unwrap();
        assert!(json.contains("\"total_files\": 2"));
        assert!(json.contains("\"total_size_bytes\": 1124"));
        assert!(json.contains("\"skipped\": 0"));
        assert!(json.contains("\"total_size_human\": \"1.10 KiB\""));
    }

    #[test]
    fn render_json_includes_top_files_sorted_desc() {
        let report = Report::new(
            empty_result(vec![
                file("/a/small.txt", 100, Some("txt")),
                file("/a/big.bin", 2048, Some("bin")),
            ]),
            Vec::new(),
        );
        let json = report.render_json().unwrap();
        // big.bin должен идти раньше small.txt (сортировка по убыванию).
        let big_pos = json.find("big.bin").unwrap();
        let small_pos = json.find("small.txt").unwrap();
        assert!(big_pos < small_pos);
        assert!(json.contains("\"size_bytes\": 2048"));
    }

    #[test]
    fn render_json_includes_recommendations() {
        use crate::models::{ClassifierLevel, FileMeta};
        let rec = Recommendation {
            file: FileMeta {
                path: PathBuf::from("/junk/thumbs.db"),
                size: 50,
                ext: Some("db".to_string()),
                mtime: Utc::now(),
            },
            classification: Classification::Trash,
            source: ClassifierLevel::L1Rules,
            reason: "совпадение с паттерном мусора".to_string(),
        };
        let report = Report::new(empty_result(vec![]), vec![rec]);
        let json = report.render_json().unwrap();
        assert!(json.contains("\"recommendations\""));
        assert!(json.contains("thumbs.db"));
        assert!(json.contains("\"Trash\""));
        assert!(json.contains("\"reason\""));
    }

    #[test]
    fn render_json_is_valid_json() {
        // Сериализация должна давать валидный JSON (переспарсим его).
        let report = Report::new(empty_result(vec![file("/a", 10, None)]), Vec::new());
        let json = report.render_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("summary").is_some());
    }
}
