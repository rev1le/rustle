//! `tui_demo` — standalone demo binary for the rustle TUI.
//!
//! Opens a test terminal window and renders a scrollable file list built
//! from in-memory mock data. Nothing is scanned on disk; the binary exists
//! to exercise TUI rendering and list navigation in isolation.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use rustle::models::{FileMeta, human_readable_size};

/// Event poll timeout; also the idle redraw cadence
const TICK_MS: u64 = 200;
/// Rows jumped by PageUp/PageDown
const PAGE_STEP: usize = 10;

fn main() -> Result<()> {
    run(mock_files())
}

/// RAII: captures raw mode + alternate screen, restores on drop/panic
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // can't recover on drop — ignore errors
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

/// Event loop: draw, poll keys, quit on demand
fn run(files: Vec<FileMeta>) -> Result<()> {
    let mut app = DemoApp::new(files);
    let mut guard = TerminalGuard::new()?;

    loop {
        guard.terminal.draw(|frame| app.draw(frame))?;

        if !event::poll(Duration::from_millis(TICK_MS))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if app.handle_key(key) {
            break;
        }
    }

    Ok(())
}

/// Static mock entries shaped like scanner output: big downloads, caches,
/// project artifacts and small docs. Timestamps are fixed literals so the
/// demo output is deterministic across runs.
fn mock_files() -> Vec<FileMeta> {
    const ENTRIES: &[(&str, u64, Option<&str>, &str)] = &[
        (
            r"C:\Users\dev\Downloads\ubuntu-24.04.1-desktop-amd64.iso",
            6_120_738_816,
            Some("iso"),
            "2025-03-02T18:41:07Z",
        ),
        (
            r"D:\Backups\photos-2024.zip",
            4_293_120_000,
            Some("zip"),
            "2025-01-15T10:02:33Z",
        ),
        (
            r"C:\Users\dev\Videos\OBS\meeting-recording-2025-06-12.mkv",
            1_842_468_352,
            Some("mkv"),
            "2025-06-12T14:30:05Z",
        ),
        (
            r"C:\Users\dev\AppData\Local\pip\cache\torch-2.3.0-cp312.whl",
            786_912_256,
            Some("whl"),
            "2025-05-20T09:12:48Z",
        ),
        (
            r"C:\Users\dev\Music\pink_floyd-wish_you_were_here.flac",
            312_555_520,
            Some("flac"),
            "2024-11-03T21:47:10Z",
        ),
        (
            r"C:\Users\dev\Projects\rustle\target\debug\deps\rustle-9f2a1c.exe",
            42_311_680,
            Some("exe"),
            "2025-08-28T17:55:41Z",
        ),
        (
            r"C:\Users\dev\logs\rustle-debug-2025-08-29.log",
            15_728_640,
            Some("log"),
            "2025-08-29T08:00:00Z",
        ),
        (
            r"C:\Users\dev\Projects\rustle\node_modules\.bin\esbuild.exe",
            10_142_912,
            Some("exe"),
            "2025-07-19T12:03:59Z",
        ),
        (
            r"C:\Windows\Temp\cabA1B2C3\tmp000.dll",
            5_505_024,
            Some("dll"),
            "2025-08-01T06:22:14Z",
        ),
        (
            r"C:\Users\dev\Desktop\screenshot-2025-07-01.png",
            2_402_304,
            Some("png"),
            "2025-07-01T16:45:52Z",
        ),
        (
            r"C:\Users\dev\.cargo\registry\cache\chrono-0.4.38.crate",
            1_204_224,
            Some("crate"),
            "2025-04-11T19:38:27Z",
        ),
        (
            r"C:\Users\dev\Downloads\invoice_0427.pdf",
            231_448,
            Some("pdf"),
            "2025-02-27T11:11:11Z",
        ),
        (
            r"C:\Users\dev\Projects\rustle\Cargo.lock",
            96_768,
            Some("lock"),
            "2025-08-29T07:31:03Z",
        ),
        (
            r"C:\Users\dev\Documents\budget-2025.xlsx",
            87_040,
            Some("xlsx"),
            "2025-06-30T13:05:19Z",
        ),
        (
            r"C:\Users\dev\Projects\rustle\README.md",
            8_192,
            Some("md"),
            "2025-08-10T22:14:36Z",
        ),
        (
            r"C:\Users\dev\.gitconfig",
            512,
            None,
            "2024-09-08T08:58:44Z",
        ),
    ];

    ENTRIES
        .iter()
        .map(|(path, size, ext, mtime)| FileMeta {
            path: PathBuf::from(path),
            size: *size,
            ext: ext.map(|e| e.to_string()),
            mtime: parse_mtime(mtime),
        })
        .collect()
}

/// Parse a static RFC 3339 timestamp from [`mock_files`]
fn parse_mtime(ts: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(ts)
        .expect("mock timestamps are valid RFC 3339 literals")
        .with_timezone(&Utc)
}

/// Demo state: mock files + list selection
struct DemoApp {
    files: Vec<FileMeta>,
    selected: usize,
    list_state: ListState,
}

impl DemoApp {
    fn new(files: Vec<FileMeta>) -> Self {
        Self {
            files,
            selected: 0,
            list_state: ListState::default(),
        }
    }

    fn next(&mut self) {
        if self.selected + 1 < self.files.len() {
            self.selected += 1;
        }
    }

    fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn first(&mut self) {
        self.selected = 0;
    }

    fn last(&mut self) {
        self.selected = self.files.len().saturating_sub(1);
    }

    fn page_down(&mut self) {
        self.selected = (self.selected + PAGE_STEP).min(self.files.len().saturating_sub(1));
    }

    fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_STEP);
    }

    /// Handle a key press; returns `true` when the app should quit
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return key.code == KeyCode::Char('c');
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => true,
            KeyCode::Down | KeyCode::Char('j') => {
                self.next();
                false
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous();
                false
            }
            KeyCode::PageDown => {
                self.page_down();
                false
            }
            KeyCode::PageUp => {
                self.page_up();
                false
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.first();
                false
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.last();
                false
            }
            _ => false,
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [header_area, list_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        frame.render_widget(self.header_line(), header_area);
        self.render_file_list(frame, list_area);
        frame.render_widget(self.footer_line(), footer_area);
    }

    fn header_line(&self) -> Paragraph<'_> {
        let total: u64 = self.files.iter().map(|f| f.size).sum();
        Paragraph::new(Line::from(vec![
            Span::styled(
                " rustle tui demo ",
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " {} files · {} total",
                self.files.len(),
                human_readable_size(total)
            )),
        ]))
    }

    fn footer_line(&self) -> Paragraph<'_> {
        Paragraph::new(Span::styled(
            " ↑/↓ or j/k: move · PgUp/PgDn: page · Home/End: edges · q/Esc: quit ",
            Style::new().fg(Color::DarkGray),
        ))
    }

    fn render_file_list(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self.files.iter().map(|f| file_item(f)).collect();

        let list = List::new(items)
            .block(Block::bordered().title(" Mock file list "))
            .highlight_style(
                Style::new()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        self.list_state.select(Some(self.selected));
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

/// One row: name, extension, human size, mtime
fn file_item(file: &FileMeta) -> ListItem<'_> {
    let name = file
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.path.display().to_string());

    ListItem::new(Line::from(vec![
        Span::styled(format!("{name:<52}"), Style::new().fg(Color::White)),
        Span::styled(
            format!("{:<6}", file.ext.as_deref().unwrap_or("-")),
            Style::new().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:>10}  ", file.human_size()),
            Style::new().fg(Color::Yellow),
        ),
        Span::styled(
            file.mtime.format("%Y-%m-%d %H:%M").to_string(),
            Style::new().fg(Color::DarkGray),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn mock_files_should_be_non_empty_with_unique_paths() {
        let files = mock_files();
        assert!(!files.is_empty());

        let mut paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), files.len());
    }

    #[test]
    fn mock_files_should_have_valid_metadata() {
        let files = mock_files();
        for file in &files {
            assert!(
                file.path.as_os_str().to_string_lossy().starts_with('C')
                    || file.path.as_os_str().to_string_lossy().starts_with('D')
            );
            assert!(file.ext.as_deref().is_none_or(|e| !e.is_empty()));
        }
    }

    #[test]
    fn navigation_should_clamp_at_list_bounds() {
        let mut app = DemoApp::new(mock_files());
        let last = app.files.len() - 1;

        app.previous();
        assert_eq!(app.selected, 0, "previous at top should stay at 0");

        for _ in 0..last + 5 {
            app.next();
        }
        assert_eq!(app.selected, last, "next should clamp at the last item");

        app.next();
        assert_eq!(app.selected, last);
    }

    #[test]
    fn page_navigation_should_jump_by_page_step() {
        let mut app = DemoApp::new(mock_files());

        app.page_down();
        assert_eq!(app.selected, PAGE_STEP);

        app.page_down();
        app.page_down();
        assert_eq!(app.selected, app.files.len() - 1, "page_down should clamp");

        app.page_up();
        assert_eq!(app.selected, app.files.len() - 1 - PAGE_STEP);

        for _ in 0..10 {
            app.page_up();
        }
        assert_eq!(app.selected, 0, "page_up should clamp at 0");
    }

    #[test]
    fn home_and_end_should_jump_to_edges() {
        let mut app = DemoApp::new(mock_files());

        app.next();
        app.next();
        app.last();
        assert_eq!(app.selected, app.files.len() - 1);

        app.first();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn handle_key_should_quit_on_q_esc_and_ctrl_c() {
        let mut app = DemoApp::new(mock_files());

        assert!(app.handle_key(key(KeyCode::Char('q'))));
        assert!(app.handle_key(key(KeyCode::Esc)));
        assert!(app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
        assert!(!app.handle_key(key(KeyCode::Char('x'))));
    }

    #[test]
    fn handle_key_should_move_selection() {
        let mut app = DemoApp::new(mock_files());

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected, 1);

        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.selected, 0);

        app.handle_key(key(KeyCode::End));
        assert_eq!(app.selected, app.files.len() - 1);

        app.handle_key(key(KeyCode::Home));
        assert_eq!(app.selected, 0);
    }
}
