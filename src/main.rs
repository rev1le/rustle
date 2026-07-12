use std::ops::Sub;
use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, Utc};
use clap::Parser;

use rustle::FileMeta;
use rustle::classifier::ClassifierLayer;
use rustle::cli::{self, Cli, Command};
use rustle::config::ScannerConfig;
use rustle::logger::init_logger;
use rustle::scanner::{ScanOptions, scan};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logger(cli.verbose);

    match cli.command {
        Command::Classify { root, config, llm } => {
            // @todo load Config, build Classifier (LLM if `llm`), classify, print recommendations
            run_classify(root, config, llm)?
        }
        Command::Tui { root } => run_tui(root)?,
        Command::Report { root, output } => run_report(root, output)?,
    }

    Ok(())
}

fn run_classify(
    root: std::path::PathBuf,
    config: std::path::PathBuf,
    llm: bool,
) -> anyhow::Result<()> {
    let _ = (root, config, llm);
    todo!("handle the `classify` command")
}

fn run_tui(root: std::path::PathBuf) -> anyhow::Result<()> {
    use rustle::classifier::llm::LlmClient;
    use rustle::config;
    use rustle::tui;

    let config = config::Config::load()?;

    eprintln!("Scanning: {} ...", root.display());
    let result = scan(&ScanOptions {
        root: root.clone(),
        max_depth: None,
        ignore: config.scanner,
    })?;

    let llm: Option<LlmClient> = {
        #[cfg(feature = "llm")]
        {
            if let Some(c) = config.llm {
                Some(LlmClient::new(&c))
            } else {
                None
            }
        }
        #[cfg(not(feature = "llm"))]
        None
    };

    // if let Some(client) = llm {
    //     let fm = FileMeta {
    //         path: "./test/test.log".into(),
    //         size: 6_0000,
    //         ext: Some("log".into()),
    //         mtime: Utc::now() - TimeDelta::days(40),
    //     };
    //     dbg!(client.classify(&fm));
    // }

    tui::run(root, result, llm)?;
    Ok(())
}

fn run_report(root: std::path::PathBuf, output: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    use rustle::models::Recommendation;
    use rustle::report::Report;

    eprintln!("Scanning: {}", root.display());
    let result = scan(&ScanOptions {
        root: root.clone(),
        max_depth: None,
        ignore: ScannerConfig::default(),
    })?;

    // classifier not implemented yet (Stage 2/3) — empty recommendations
    let recommendations: Vec<Recommendation> = Vec::new();
    let report = Report::new(result, recommendations);
    report.render(output.as_deref())?;
    Ok(())
}
