//! Точка входа rustle.

use clap::Parser;

use rustle::cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    // @todo: инициализация `env_logger` с уровнем из `--verbose`.
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { root, depth, top } => {
            // @todo: собрать ScanOptions из конфига и аргументов, вызвать
            //       `scanner::scan`, вывести агрегаты и топ-N.
            run_scan(root, depth, top)?
        }
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

// @todo: реализовать обработчики подкоманд. Ниже — только сигнатуры-заглушки,
//        чтобы зафиксировать поток управления MVP.

fn run_scan(root: std::path::PathBuf, depth: Option<usize>, top: usize) -> anyhow::Result<()> {
    use rustle::config::ScannerConfig;
    use rustle::models::human_readable_size;
    use rustle::scanner::aggregator::{aggregate, top_largest};
    use rustle::scanner::{ScanOptions, scan};

    println!("Сканирование: {}", root.display());
    let options = ScanOptions {
        root: root.clone(),
        max_depth: depth,
        ignore: ScannerConfig::default(),
    };
    let result = scan(&options)?;

    let total_size: u64 = result.files.iter().map(|f| f.size).sum();
    println!();
    println!(
        "Файлов: {} | пропущено: {} | общий размер: {}",
        result.files.len(),
        result.skipped,
        human_readable_size(total_size)
    );

    // Топ каталогов по размеру (только внутри корня сканирования —
    // отсекаем шум от родительских каталогов вроде E:\, Projects).
    let dirs = aggregate(&result.files);
    let inner_dirs: Vec<_> = dirs.iter().filter(|d| d.path.starts_with(&root)).collect();
    println!("\nТоп-15 каталогов по размеру:");
    println!("  {:>12}  {:>8}  ПУТЬ", "РАЗМЕР", "ФАЙЛОВ");
    for d in inner_dirs.iter().take(15) {
        println!(
            "  {:>12}  {:>8}  {}",
            human_readable_size(d.total_size),
            d.file_count,
            d.path.display()
        );
    }

    // Топ-N самых больших файлов.
    let top_files = top_largest(&result.files, top);
    println!("\nТоп-{} самых больших файлов:", top);
    println!("  {:>12}  {:^8}  ПУТЬ", "РАЗМЕР", "EXT");
    for f in &top_files {
        let ext = f.ext.as_deref().unwrap_or("-");
        println!(
            "  {:>12}  {:^8}  {}",
            human_readable_size(f.size),
            ext,
            f.path.display()
        );
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
    let _ = root;
    todo!("обработать команду `tui`")
}

fn run_report(root: std::path::PathBuf, output: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    use rustle::config::ScannerConfig;
    use rustle::models::Recommendation;
    use rustle::report::Report;
    use rustle::scanner::{ScanOptions, scan};

    eprintln!("Сканирование: {}", root.display());
    let result = scan(&ScanOptions {
        root: root.clone(),
        max_depth: None,
        ignore: ScannerConfig::default(),
    })?;

    // Классификатор пока не реализован (Этап 2/3) — рекомендации пусты.
    let recommendations: Vec<Recommendation> = Vec::new();
    let report = Report::new(result, recommendations);
    report.render(output.as_deref())?;
    Ok(())
}
