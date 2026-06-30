use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
    Classify {
        /// Корневая директория
        root: PathBuf,
        /// Путь к TOML-конфигу пользовательских правил
        #[arg(short, long, default_value = "rustle.toml")]
        config: PathBuf,
        /// Использовать LLM для анализа файлов
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
        /// Куда записать отчёт (по умолчанию — report.json).
        #[arg(short, long, default_value = "report.json")]
        output: Option<PathBuf>,
    },
}
