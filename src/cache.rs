//! Кэш результатов классификации (Этап 3).
//!
//! "Кэширование ответов по хэшу файла" и "Кэш ответов LLM для повторных
//! запусков" — ключевые решения проекта.

use std::path::Path;

use crate::error::RustleError;
use crate::models::{Classification, FileMeta};

/// Хранилище кэша классификаций, индексированное хэшем содержимого файла.
///
/// @todo: выбрать бэкенд — простой JSON-файл на диске или sled/SQLite
/// для больших наборов.
pub struct ClassificationCache {
    // @todo: HashMap<[u8; 32], Classification> + путь к файлу кэша.
}

impl ClassificationCache {
    pub fn new() -> Self {
        Self {}
    }

    /// Посчитать BLAKE3-хэш содержимого файла (для дедупликации и кэша).
    ///
    /// @todo: реализовать потоковое хэширование через `blake3::Hasher`.
    pub fn hash_file(path: &Path) -> Result<[u8; 32], RustleError> {
        let _ = path;
        todo!("посчитать blake3 от содержимого {:?}", path)
    }

    /// Найти закэшированную классификацию по хэшу файла.
    ///
    /// @todo: проверить `cache` по ключу-хэшу.
    pub fn get(&self, _hash: &[u8; 32]) -> Option<Classification> {
        todo!("найти классификацию в кэше по хэшу")
    }

    /// Сохранить классификацию в кэш.
    ///
    /// @todo: вставить запись и пометить кэш как "грязный" для последующего
    /// flush-а на диск.
    pub fn put(&mut self, _file: &FileMeta, _classification: Classification) {
        todo!("добавить классификацию в кэш")
    }

    /// Записать кэш на диск.
    ///
    /// @todo: сериализовать через `serde_json` в файл кэша.
    pub fn flush(&self) -> Result<(), RustleError> {
        todo!("записать кэш классификаций на диск")
    }
}

impl Default for ClassificationCache {
    fn default() -> Self {
        Self::new()
    }
}
