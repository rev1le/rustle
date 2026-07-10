use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "rustle",
    version,
    about,
    disable_help_subcommand = true,
    disable_help_flag = true,
    disable_version_flag = true
)]
pub struct Cli {
    /// Log verbosity level
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Classify {
        /// Root directory
        root: PathBuf,
        /// TOML config of user rules
        #[arg(short, long, default_value = "rustle.toml")]
        config: PathBuf,
        /// Use the LLM
        #[arg(long, default_value_t = false)]
        llm: bool,
    },

    /// Interactive TUI (tree + heatmap)
    Tui {
        /// Root directory
        root: PathBuf,
    },

    /// Generate a report (JSON)
    Report {
        /// Root directory
        root: PathBuf,
        /// Where to write the report (default report.json)
        #[arg(short, long, default_value = "report.json")]
        output: Option<PathBuf>,
    },
}
