//! rustle — анализатор дискового пространства с многоуровневой AI-классификацией.
//!
//! Архитектура (см. ABOUT.md):
//! ```text
//! [Rust: сканер ФС] → [кэш/БД] → [роутер классификации]
//!                                     ├─ L1: жёсткие правила (ext, паттерны)
//!                                     ├─ L2: эвристики (размер, возраст, путь)
//!                                     └─ L3: LLM-агент (только uncertain)
//!                                         ↓
//!                                     [рекомендации + UI]
//! ```

pub mod cache;
pub mod classifier;
pub mod cli;
pub mod config;
pub mod models;
pub mod progress;
pub mod report;
pub mod scanner;
pub mod tui;

pub use models::{Classification, FileMeta};
