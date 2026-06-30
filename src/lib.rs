//! rustle — анализатор дискового пространства с многоуровневой AI-классификацией.

pub mod cache;
pub mod classifier;
pub mod cli;
pub mod config;
pub mod error;
pub mod logger;
pub mod models;
pub mod progress;
pub mod report;
pub mod scanner;
pub mod tui;

pub use error::RustleError;
pub use models::{Classification, FileMeta};
