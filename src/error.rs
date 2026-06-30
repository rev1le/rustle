//! Типизированные ошибки библиотеки.

use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustleError {
    /// Ошибка ввода-вывода: чтение/запись файлов, отчётов, кэша.
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] io::Error),

    /// Ошибка (де)сериализации JSON.
    #[error("ошибка сериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
}
