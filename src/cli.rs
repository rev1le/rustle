//! Определение CLI через `clap` (Этап 1 MVP).
//!
//! Подкоманды отражают дорожную карту: сканирование, классификация,
//! отчёты, TUI.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Утилита для анализа дискового пространства с AI-классификацией файлов.
#[derive(Debug, Parser)]
#[command(name = "rustle", version, about)]
pub struct Cli {
    /// Уровень логирования.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Этап 1: рекурсивный обход ФС и сбор метаданных.
    Scan {
        /// Корневая директория для сканирования.
        root: PathBuf,
        /// Максимальная глубина обхода (None — без ограничений).
        #[arg(short, long)]
        depth: Option<usize>,
        /// Показать топ-N самых больших файлов.
        #[arg(short, long, default_value_t = 10)]
        top: usize,
    },

    /// Этап 2/3: классификация найденных файлов по правилам и LLM.
    Classify {
        /// Корневая директория.
        root: PathBuf,
        /// Путь к TOML-конфигу пользовательских правил.
        #[arg(short, long, default_value = "rustle.toml")]
        config: PathBuf,
        /// Использовать локальную LLM для uncertain-файлов.
        #[arg(long, default_value_t = false)]
        llm: bool,
    },

    /// Этап 4: интерактивный TUI (дерево + heatmap).
    Tui {
        /// Корневая директория.
        root: PathBuf,
    },

    /// Этап 4: сгенерировать отчёт (JSON).
    Report {
        /// Корневая директория.
        root: PathBuf,
        /// Куда записать отчёт (stdout, если не указано).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
