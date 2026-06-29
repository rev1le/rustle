//! Типизированные ошибки библиотеки.
//!
//! Согласно best practices (Chapter 4): библиотека возвращает типизированные
//! ошибки через `thiserror`, а `anyhow` зарезервирован для бинарника
//! (`src/main.rs`). `RustleError` реализует `std::error::Error`, поэтому
//! бинарник свободно конвертирует его через `?` в `anyhow::Error`.

use std::io;

use thiserror::Error;

/// Ошибка любой операции библиотеки rustle.
#[derive(Debug, Error)]
pub enum RustleError {
    /// Ошибка ввода-вывода: чтение/запись файлов, отчётов, кэша.
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] io::Error),

    /// Ошибка (де)сериализации JSON.
    #[error("ошибка сериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
}
