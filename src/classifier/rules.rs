//! L1: жёсткие правила — расширения и паттерны имён.
//!
//! "Система whitelist/blacklist расширений" и "Паттерны мусора
//! (thumbs.db, .DS_Store, node_modules)" (Этап 2).

use crate::classifier::ClassifierLayer;
use crate::config::RulesConfig;
use crate::models::{Classification, FileMeta};

/// Скомпилированный набор правил L1.
pub struct Rules {
    /// Whitelist расширений → Keep.
    keep_exts: Vec<String>,
    /// Blacklist расширений → Trash.
    trash_exts: Vec<String>,
    /// Паттерны имён → Trash (например, "thumbs.db", ".DS_Store").
    trash_patterns: Vec<String>,
}

impl Rules {
    /// Собрать правила из TOML-конфига.
    ///
    /// @todo: привести расширения к нижнему регистру и скомпилировать
    /// паттерны в `glob::Pattern` (или `regex`) для быстрой проверки.
    pub fn from_config(config: &RulesConfig) -> Self {
        let _ = config;
        todo!("скомпилировать правила L1 из RulesConfig")
    }
}

impl ClassifierLayer for Rules {
    fn classify(&self, file: &FileMeta) -> Option<(Classification, String)> {
        let _ = file;
        // @todo:
        //   1. Если ext в keep_exts → Some((Keep, reason)).
        //   2. Если ext в trash_exts → Some((Trash, reason)).
        //   3. Если имя файла совпадает с trash_patterns → Some((Trash, reason)).
        //   4. Иначе None (передать на L2).
        todo!("применить жёсткие правила к {:?}", file.path)
    }
}
