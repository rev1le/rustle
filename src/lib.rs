//! rustle — disk space analyzer with multi-level AI classification

pub mod cache;
pub mod classifier;
pub mod cli;
pub mod config;
pub mod error;
pub mod explain_classification;
pub mod logger;
pub mod models;
pub mod platform;
pub mod progress;
pub mod report;
pub mod scanner;
pub mod tui;

pub use error::RustleError;
pub use models::{Classification, FileMeta};
