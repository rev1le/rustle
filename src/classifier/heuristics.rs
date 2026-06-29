//! L2: эвристики — размер, возраст, путь.
//!
//! "Эвристики: возраст + размер" (Этап 2).

use crate::classifier::ClassifierLayer;
use crate::config::RulesConfig;
use crate::models::{Classification, FileMeta};

/// Пороговые значения для эвристик (могут быть переопределены конфигом).
///
/// Поля читаются после реализации `score`/`classify` (Этап 2).
#[expect(
    dead_code,
    reason = "заглушка: поля используются после реализации todo в score/classify"
)]
pub struct Heuristics {
    /// Файлы старше N дней считаются "давно не использованными".
    stale_age_days: u32,
    /// Файлы крупнее N байт попадают в "подозрительно большие".
    large_size_bytes: u64,
}

impl Heuristics {
    pub fn from_config(config: &RulesConfig) -> Self {
        let _ = config;
        // Значения по умолчанию.
        Self {
            stale_age_days: 365,
            large_size_bytes: 512 * 1024 * 1024,
        }
    }

    /// Посчитать "мусорный score" файла в диапазоне 0.0..=1.0 на основе
    /// комбинации возраста, размера и пути.
    ///
    /// @todo: реализовать взвешенную сумму сигналов:
    ///   - возраст (старее → выше score),
    ///   - размер (очень крупный и при этом старый → выше),
    ///   - путь (внутри cache/temp/build → выше).
    pub fn score(&self, file: &FileMeta) -> f32 {
        let _ = file;
        todo!("посчитать эвристический score для {:?}", file.path)
    }
}

impl ClassifierLayer for Heuristics {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo:
        //   1. Посчитать score.
        //   2. Если score >= 0.8 → Some((Trash, ...)).
        //   3. Если score <= 0.2 → Some((Keep, ...)).
        //   4. Иначе → Some((Uncertain(score), ...)) для передачи в L3.
        todo!("применить эвристики к {:?}", file.path)
    }
}
