//! Интерактивный TUI через `ratatui` + `crossterm` (Этап 4).
//!
//! Показывает дерево каталогов по размеру (heatmap-полосками), топ файлов
//! и сводку по сканированию. Навигация стрелками/j-k, выход — q/Esc/Ctrl-C.

use std::io::{self, Stdout};
use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};

use crate::error::RustleError;
use crate::models::{DirAggregate, FileMeta, human_readable_size};
use crate::scanner::ScanResult;

/// Сколько каталогов показывать в основной панели.
const MAX_DIRS: usize = 50;
/// Сколько файлов показывать в боковой панели.
const MAX_FILES: usize = 30;
/// Ширина текстовой полоски размера (в символах).
const BAR_WIDTH: usize = 20;

/// RAII-обёртка над терминалом: захватывает raw-mode + альт. экран при
/// создании и гарантированно восстанавливает состояние при падении/выходе.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self, RustleError> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // CONTEXT: восстановление терминала при выходе/панике — ошибки здесь
        // уже не исправить, поэтому сознательно игнорируем.
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

/// Состояние TUI: агрегированные данные сканирования + навигация.
struct App {
    root: PathBuf,
    total_files: usize,
    skipped: usize,
    total_size: u64,
    directories: Vec<DirAggregate>,
    top_files: Vec<FileMeta>,
}

impl App {
    fn new(root: PathBuf, scan: ScanResult) -> Self {
        let total_size: u64 = scan.files.iter().map(|f| f.size).sum();
        let total_files = scan.files.len();
        let skipped = scan.skipped;

        // Только подкаталоги внутри корня — отсекаем сам корень и родителей.
        let mut directories = crate::scanner::aggregator::aggregate(&scan.files);
        directories.retain(|d| d.path != root && d.path.starts_with(&root));
        directories.truncate(MAX_DIRS);

        let top_files = crate::scanner::aggregator::top_largest(&scan.files, MAX_FILES);

        Self {
            root,
            total_files,
            skipped,
            total_size,
            directories,
            top_files,
        }
    }

    /// Сдвинуть выделение в списке каталогов, ограничив границы.
    fn move_selection(&self, state: &mut ListState, delta: isize) {
        let len = self.directories.len();
        if len == 0 {
            return;
        }
        let current = state.selected().unwrap_or(0);
        let next = (current as isize + delta).clamp(0, (len - 1) as isize) as usize;
        state.select(Some(next));
    }

    fn draw(&self, frame: &mut Frame, state: &mut ListState) {
        let [header, main, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());

        self.draw_header(frame, header);

        let [dirs, files] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(main);
        self.draw_directories(frame, dirs, state);
        self.draw_files(frame, files);

        Self::draw_footer(frame, footer);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" rustle — {} ", self.root.display());
        let stats = format!(
            "файлов: {} | пропущено: {} | всего: {}",
            self.total_files,
            self.skipped,
            human_readable_size(self.total_size)
        );
        let paragraph =
            Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(paragraph, area);
    }

    fn draw_directories(&self, frame: &mut Frame, area: Rect, state: &mut ListState) {
        let max_size = self.directories.first().map_or(0, |d| d.total_size);

        let items: Vec<ListItem<'_>> = self
            .directories
            .iter()
            .map(|dir| {
                let rel = dir.path.strip_prefix(&self.root).unwrap_or(&dir.path);
                let ratio = if max_size == 0 {
                    0.0
                } else {
                    dir.total_size as f64 / max_size as f64
                };
                let filled = (ratio * BAR_WIDTH as f64).round() as usize;
                let bar = format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled));

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{bar} "), Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:>10} ", human_readable_size(dir.total_size))),
                    Span::raw(rel.to_string_lossy().into_owned()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Каталоги по размеру "),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, state);
    }

    fn draw_files(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem<'_>> = self
            .top_files
            .iter()
            .map(|file| {
                let rel = file.path.strip_prefix(&self.root).unwrap_or(&file.path);
                let ext = file.ext.as_deref().unwrap_or("-");
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{:>10} ", human_readable_size(file.size))),
                    Span::styled(format!("[{ext:^6}] "), Style::default().fg(Color::Yellow)),
                    Span::raw(rel.to_string_lossy().into_owned()),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Самые большие файлы "),
        );
        frame.render_widget(list, area);
    }

    fn draw_footer(frame: &mut Frame, area: Rect) {
        let help = " ↑/↓ или j/k — навигация   q/Esc/Ctrl-C — выход ";
        let paragraph = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
    }
}

/// Запустить интерактивный TUI поверх результатов сканирования.
pub fn run(root: PathBuf, scan: ScanResult) -> Result<(), RustleError> {
    let app = App::new(root, scan);
    let mut guard = TerminalGuard::new()?;
    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        guard.terminal.draw(|frame| app.draw(frame, &mut state))?;

        // Игнорируем все события, кроме нажатий клавиш.
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(());
            }
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(&mut state, 1),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(&mut state, -1),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ratatui::backend::TestBackend;

    fn file(path: &str, size: u64, ext: Option<&str>) -> FileMeta {
        FileMeta {
            path: PathBuf::from(path),
            size,
            ext: ext.map(|e| e.to_string()),
            mtime: Utc::now(),
        }
    }

    fn scan_of(files: Vec<FileMeta>) -> ScanResult {
        ScanResult { files, skipped: 0 }
    }

    #[test]
    fn move_selection_clamps_to_directory_bounds() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub1/x", 1, None),
                file("/root/sub2/y", 2, None),
            ]),
        );
        // После агрегации остаются два подкаталога (сам /root отсечён).
        assert_eq!(app.directories.len(), 2);

        let mut state = ListState::default();
        state.select(Some(0));

        app.move_selection(&mut state, 100);
        assert_eq!(state.selected(), Some(1));

        app.move_selection(&mut state, -100);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_selection_is_noop_without_directories() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert!(app.directories.is_empty());

        let mut state = ListState::default();
        state.select(Some(0));
        app.move_selection(&mut state, 5);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn draw_renders_layout_without_panic() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub/big.bin", 4096, Some("bin")),
                file("/root/sub/small.txt", 100, Some("txt")),
            ]),
        );

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(0));

        terminal.draw(|frame| app.draw(frame, &mut state)).unwrap();
    }

    #[test]
    fn draw_contains_panel_titles_and_bar_glyphs() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/sub/big.bin", 4096, Some("bin"))]),
        );

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(0));
        terminal.draw(|frame| app.draw(frame, &mut state)).unwrap();

        let buf = terminal.backend().buffer();
        let content: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect();

        assert!(
            content.contains("Каталоги"),
            "должен быть заголовок панели каталогов"
        );
        assert!(content.contains("файлы"), "должна быть сводка по файлам");
        // Полоска размера содержит заполнители (█) или фон (░).
        assert!(content.contains('█') || content.contains('░'));
    }
}
