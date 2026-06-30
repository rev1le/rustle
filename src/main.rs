use clap::Parser;

use rustle::cli::{Cli, Command};
use rustle::config::IgnoreConfig;
use rustle::logger::init_logger;
use rustle::scanner::{ScanOptions, scan};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logger(cli.verbose);

    match cli.command {
        Command::Classify { root, config, llm } => {
            // @todo: загрузить Config, построить Classifier (с LLM при `llm`),
            //       классифицировать все файлы и вывести рекомендации.
            run_classify(root, config, llm)?
        }
        Command::Tui { root } => {
            // @todo: отсканировать + классифицировать, затем `tui::run`.
            run_tui(root)?
        }
        Command::Report { root, output } => {
            // @todo: добавить классификацию (Этап 2/3) и наполнить рекомендации.
            run_report(root, output)?
        }
    }

    Ok(())
}

fn run_classify(
    root: std::path::PathBuf,
    config: std::path::PathBuf,
    llm: bool,
) -> anyhow::Result<()> {
    let _ = (root, config, llm);
    todo!("обработать команду `classify`")
}

fn run_tui(root: std::path::PathBuf) -> anyhow::Result<()> {
    use rustle::tui;

    eprintln!("Сканирование: {} …", root.display());
    let result = scan(&ScanOptions {
        root: root.clone(),
        max_depth: None,
        ignore: IgnoreConfig::default(),
    })?;

    tui::run(root, result)?;
    Ok(())
}

fn run_report(root: std::path::PathBuf, output: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    use rustle::models::Recommendation;
    use rustle::report::Report;

    eprintln!("Сканирование: {}", root.display());
    let result = scan(&ScanOptions {
        root: root.clone(),
        max_depth: None,
        ignore: IgnoreConfig::default(),
    })?;

    // Классификатор пока не реализован (Этап 2/3) — рекомендации пусты.
    let recommendations: Vec<Recommendation> = Vec::new();
    let report = Report::new(result, recommendations);
    report.render(output.as_deref())?;
    Ok(())
}
