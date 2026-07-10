//! Typed errors for the library

use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustleError {
    /// I/O error: reading/writing files, reports, cache
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON (de)serialization error
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Cache store error (sled)
    #[error("classification cache error: {0}")]
    Cache(#[from] sled::Error),
}
