use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::{Action, Screen};
use crate::models::{DirAggregate, FileMeta, human_readable_size};
use crate::platform;
use crate::scanner::ScanResult;

/// Size bar width in chars
const BAR_WIDTH: usize = 20;
/// Dirs shown in the main panel
const MAX_DIRS: usize = 50;
/// Files shown in the side panel
const MAX_FILES: usize = 30;
const MARQUEE_PAUSE_TICKS: usize = 4;
/// Ticks a footer status lives
const STATUS_TTL_TICKS: usize = 20;

/// Which panel has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Directories,
    Files,
}

pub(in crate::tui) struct MainScreen {
    root: PathBuf,
    total_files: usize,
    skipped: usize,
    total_size: u64,
    directories: Vec<DirAggregate>,
    top_files: Vec<FileMeta>,
    focus: Focus,
    dir_state: ListState,
    file_state: ListState,
    /// Marquee phase
    scroll_phase: usize,
    /// Temporary status (e.g. open result)
    status: Option<String>,
    status_ttl: usize,
}

impl MainScreen {
    pub(in crate::tui) fn new(root: PathBuf, scan: ScanResult) -> Self {
        let total_size: u64 = scan.files.iter().map(|f| f.size).sum();
        let total_files = scan.files.len();
        let skipped = scan.skipped;

        // only subdirs inside root — drop root and parents
        let mut directories = crate::scanner::aggregator::aggregate(&scan.files);
        directories.retain(|d| d.path != root && d.path.starts_with(&root));
        directories.truncate(MAX_DIRS);

        let top_files = crate::scanner::aggregator::top_largest(&scan.files, MAX_FILES);

        let mut dir_state = ListState::default();
        dir_state.select(Some(0));
        let mut file_state = ListState::default();
        file_state.select(Some(0));

        Self {
            root,
            total_files,
            skipped,
            total_size,
            directories,
            top_files,
            focus: Focus::Directories,
            dir_state,
            file_state,
            scroll_phase: 0,
            status: None,
            status_ttl: 0,
        }
    }

    fn move_directory_selection(&mut self, delta: isize) {
        move_stateful_selection(&mut self.dir_state, self.directories.len(), delta);
        self.scroll_phase = 0;
    }

    fn move_file_selection(&mut self, delta: isize) {
        move_stateful_selection(&mut self.file_state, self.top_files.len(), delta);
        self.scroll_phase = 0;
    }

    fn move_focused(&mut self, delta: isize) {
        match self.focus {
            Focus::Directories => self.move_directory_selection(delta),
            Focus::Files => self.move_file_selection(delta),
        }
    }

    /// Switch focus between panels
    fn switch_focus(&mut self, direction: isize) {
        self.focus = match (self.focus, direction) {
            (Focus::Directories, 1) => Focus::Files,
            (Focus::Files, -1) => Focus::Directories,
            (current, _) => current,
        };
        self.scroll_phase = 0;
    }

    /// Highlighted file in the files panel
    fn selected_file(&self) -> Option<FileMeta> {
        if self.focus != Focus::Files {
            return None;
        }
        self.file_state
            .selected()
            .and_then(|i| self.top_files.get(i).cloned())
    }

    /// Highlighted path in the active panel
    fn focused_selected_path(&self) -> Option<PathBuf> {
        match self.focus {
            Focus::Files => self
                .file_state
                .selected()
                .and_then(|i| self.top_files.get(i).map(|f| f.path.clone())),
            Focus::Directories => self
                .dir_state
                .selected()
                .and_then(|i| self.directories.get(i).map(|d| d.path.clone())),
        }
    }

    /// Open the highlighted path in the OS file manager
    fn open_selected_in_explorer(&mut self) {
        if let Some(path) = self.focused_selected_path() {
            match platform::open_in_explorer(&path) {
                Ok(()) => self.set_status(format!("Opening: {}", path.display())),
                Err(e) => self.set_status(format!("Error: {e}")),
            }
        }
    }

    /// Copy the highlighted path to the clipboard
    fn copy_selected_path(&mut self) {
        if let Some(path) = self.focused_selected_path() {
            match platform::copy_to_clipboard(&path.to_string_lossy()) {
                Ok(()) => self.set_status(format!("Copied: {}", path.display())),
                Err(e) => self.set_status(format!("Error: {e}")),
            }
        }
    }

    /// Show a temporary footer status
    fn set_status(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
        self.status_ttl = STATUS_TTL_TICKS;
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" rustle — {} ", self.root.display());
        let stats = format!(
            "files: {} | skipped: {} | total: {}",
            self.total_files,
            self.skipped,
            human_readable_size(self.total_size)
        );
        let paragraph =
            Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(paragraph, area);
    }

    fn render_directories(&mut self, frame: &mut Frame, area: Rect) {
        let max_size = self.directories.first().map_or(0, |d| d.total_size);
        let selected = self.dir_state.selected();
        // available dir name width
        let name_width = name_width_for(area, 21 + 11);

        let items: Vec<ListItem<'_>> = self
            .directories
            .iter()
            .enumerate()
            .map(|(i, dir)| {
                let rel = dir.path.strip_prefix(&self.root).unwrap_or(&dir.path);
                let ratio = if max_size == 0 {
                    0.0
                } else {
                    dir.total_size as f64 / max_size as f64
                };
                let filled = (ratio * BAR_WIDTH as f64).round() as usize;
                // escapes intentional: raw █/░ glyphs got mangled in edits
                let bar = format!(
                    "{}{}",
                    "\u{2588}".repeat(filled),
                    "\u{2591}".repeat(BAR_WIDTH - filled)
                );

                let label = display_label(
                    &rel.to_string_lossy(),
                    name_width,
                    self.focus == Focus::Directories && Some(i) == selected,
                    self.scroll_phase,
                );

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{bar} "), Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:>10} ", human_readable_size(dir.total_size))),
                    Span::raw(label),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(panel_border_style(self.focus, Focus::Directories))
                    .title(panel_title(
                        "Directories by size",
                        self.focus,
                        Focus::Directories,
                    )),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.dir_state);
    }

    fn render_files(&mut self, frame: &mut Frame, area: Rect) {
        let selected = self.file_state.selected();
        // available name width: [size:11] [ext:9] + border(2) + marker(2)
        let name_width = name_width_for(area, 11 + 9);

        let items: Vec<ListItem<'_>> = self
            .top_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let rel = file.path.strip_prefix(&self.root).unwrap_or(&file.path);
                let ext = file.ext.as_deref().unwrap_or("-");
                let label = display_label(
                    &rel.to_string_lossy(),
                    name_width,
                    self.focus == Focus::Files && Some(i) == selected,
                    self.scroll_phase,
                );
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{:>10} ", human_readable_size(file.size))),
                    Span::styled(format!("[{ext:^6}] "), Style::default().fg(Color::Yellow)),
                    Span::raw(label),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(panel_border_style(self.focus, Focus::Files))
                    .title(panel_title("Largest files", self.focus, Focus::Files)),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.file_state);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help = " Up/Dn or j/k — select   ←/→ or h/l — panel   Enter — details   g — open   y — copy   q/Esc — quit ";
        let paragraph = match &self.status {
            Some(status) => Paragraph::new(format!(" {status} "))
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL)),
            None => Paragraph::new(help).block(Block::default().borders(Borders::ALL)),
        };
        frame.render_widget(paragraph, area);
    }
}

impl Screen for MainScreen {
    fn draw(&mut self, frame: &mut Frame) {
        let [header, main_area, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());

        self.render_header(frame, header);

        let [dirs, files] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(main_area);
        self.render_directories(frame, dirs);
        self.render_files(frame, files);

        self.render_footer(frame, footer);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            // horizontal: switch panel
            KeyCode::Right | KeyCode::Char('l') => {
                self.switch_focus(1);
                Action::None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.switch_focus(-1);
                Action::None
            }
            // vertical: moves active panel
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_focused(1);
                Action::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_focused(-1);
                Action::None
            }
            KeyCode::Enter => match self.selected_file() {
                Some(file) => Action::OpenDetail(file),
                None => Action::None,
            },
            // open in OS file manager
            KeyCode::Char('g') => {
                self.open_selected_in_explorer();
                Action::None
            }
            // copy path to clipboard
            KeyCode::Char('y') => {
                self.copy_selected_path();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn tick(&mut self) {
        self.scroll_phase = self.scroll_phase.wrapping_add(1);
        // clear status when TTL hits 0
        if self.status_ttl > 0 {
            self.status_ttl -= 1;
            if self.status_ttl == 0 {
                self.status = None;
            }
        }
    }
}

/// Shift a `ListState` selection, clamped to `len`
fn move_stateful_selection(state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        return;
    }
    let current = state.selected().unwrap_or(0);
    let next = (current as isize + delta).clamp(0, (len - 1) as isize) as usize;
    state.select(Some(next));
}

/// Available row text width in a panel
fn name_width_for(area: Rect, prefix_len: usize) -> usize {
    (area.width as usize).saturating_sub(2 + 2 + prefix_len)
}

/// Row label, handling overflow
fn display_label(text: &str, width: usize, active_and_selected: bool, phase: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if active_and_selected {
        marquee_window(text, width, phase)
    } else {
        truncate_to(text, width)
    }
}

/// Marquee window: scroll with pauses at both ends
fn marquee_window(text: &str, width: usize, phase: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= width {
        return text.to_string();
    }
    let scroll_range = len - width;
    let period = MARQUEE_PAUSE_TICKS * 2 + scroll_range;
    let p = phase % period;
    let offset = if p < MARQUEE_PAUSE_TICKS {
        // start pause: show beginning
        0
    } else if p < MARQUEE_PAUSE_TICKS + scroll_range {
        // scrolling
        p - MARQUEE_PAUSE_TICKS
    } else {
        // end pause: show tail
        scroll_range
    };
    chars[offset..offset + width].iter().collect()
}

/// Truncate to `width`, appending `…` on overflow
fn truncate_to(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        text.to_string()
    } else if width == 0 {
        String::new()
    } else {
        let mut s: String = chars[..width - 1].iter().collect();
        s.push('…');
        s
    }
}

/// Border style: active highlighted, inactive dimmed
fn panel_border_style(focus: Focus, panel: Focus) -> Style {
    if focus == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Title: active panel gets a `◀`
fn panel_title(title: &str, focus: Focus, panel: Focus) -> String {
    if focus == panel {
        format!(" {title} ◀ ")
    } else {
        format!(" {title} ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_fixtures::{file, scan_of};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn move_directory_selection_clamps_to_bounds() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub1/x", 1, None),
                file("/root/sub2/y", 2, None),
            ]),
        );
        // two subdirs remain after aggregation (/root cut off)
        assert_eq!(screen.directories.len(), 2);

        screen.move_directory_selection(100);
        assert_eq!(screen.dir_state.selected(), Some(1));

        screen.move_directory_selection(-100);
        assert_eq!(screen.dir_state.selected(), Some(0));
    }

    #[test]
    fn move_file_selection_clamps_to_bounds() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/a", 1, None),
                file("/root/b", 2, None),
                file("/root/c", 3, None),
            ]),
        );
        assert_eq!(screen.top_files.len(), 3);

        screen.move_file_selection(100);
        assert_eq!(screen.file_state.selected(), Some(2));

        screen.move_file_selection(-100);
        assert_eq!(screen.file_state.selected(), Some(0));
    }

    #[test]
    fn move_directory_selection_is_noop_without_directories() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert!(screen.directories.is_empty());

        screen.move_directory_selection(5);
        assert_eq!(screen.dir_state.selected(), Some(0));
    }

    #[test]
    fn switch_focus_toggles_between_panels() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub/a", 1, None),
                file("/root/sub2/b", 2, None),
            ]),
        );

        // dirs panel active by default
        assert_eq!(screen.focus, Focus::Directories);

        screen.switch_focus(-1);
        assert_eq!(screen.focus, Focus::Directories);

        screen.switch_focus(1);
        assert_eq!(screen.focus, Focus::Files);

        screen.switch_focus(1);
        assert_eq!(screen.focus, Focus::Files);

        screen.switch_focus(-1);
        assert_eq!(screen.focus, Focus::Directories);
    }

    #[test]
    fn tick_advances_scroll_phase() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert_eq!(screen.scroll_phase, 0);

        screen.tick();
        screen.tick();
        assert_eq!(screen.scroll_phase, 2);
    }

    #[test]
    fn navigation_resets_scroll_phase() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub1/a", 1, None),
                file("/root/sub2/b", 2, None),
            ]),
        );
        screen.tick();
        screen.tick();
        screen.tick();
        assert_eq!(screen.scroll_phase, 3);

        // moving dir selection resets phase
        screen.move_directory_selection(1);
        assert_eq!(screen.scroll_phase, 0);

        // focus switch resets too
        screen.tick();
        assert_eq!(screen.scroll_phase, 1);
        screen.switch_focus(1);
        assert_eq!(screen.scroll_phase, 0);
    }

    #[test]
    fn set_status_shows_message_with_ttl() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert!(screen.status.is_none());

        screen.set_status("done");
        assert_eq!(screen.status.as_deref(), Some("done"));
        assert_eq!(screen.status_ttl, STATUS_TTL_TICKS);
    }

    #[test]
    fn status_expires_after_ttl_ticks() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        screen.set_status("temporary");

        for _ in 0..STATUS_TTL_TICKS - 1 {
            screen.tick();
            assert!(screen.status.is_some(), "status should still be shown");
        }
        screen.tick();
        assert!(screen.status.is_none());
        assert_eq!(screen.status_ttl, 0);
    }

    #[test]
    fn selected_file_only_when_files_panel_focused() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a.bin", 42, Some("bin"))]),
        );
        // focus on dirs by default — no file
        assert!(screen.selected_file().is_none());

        screen.switch_focus(1);
        assert_eq!(screen.selected_file().map(|f| f.size), Some(42));
    }

    #[test]
    fn draw_renders_layout_without_panic() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub/big.bin", 4096, Some("bin")),
                file("/root/sub/small.txt", 100, Some("txt")),
            ]),
        );

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.draw(frame)).unwrap();
    }

    #[test]
    fn draw_contains_panel_titles_and_bar_glyphs() {
        let mut screen = MainScreen::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/sub/big.bin", 4096, Some("bin"))]),
        );

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.draw(frame)).unwrap();

        let buf = terminal.backend().buffer();
        let content: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect();

        assert!(
            content.contains("Directories"),
            "directories panel title must be present"
        );
        assert!(content.contains("files"), "files summary must be present");
        // bar has fillers (█) or background (░)
        assert!(content.contains('█') || content.contains('░'));
    }

    #[test]
    fn truncate_to_keeps_short_text_intact() {
        assert_eq!(truncate_to("abc", 5), "abc");
        assert_eq!(truncate_to("abc", 3), "abc");
    }

    #[test]
    fn truncate_to_appends_ellipsis_on_overflow() {
        assert_eq!(truncate_to("abcdef", 4), "abc…");
        assert_eq!(truncate_to("abcdef", 1), "…");
    }

    #[test]
    fn marquee_window_shows_full_text_when_it_fits() {
        assert_eq!(marquee_window("abc", 5, 0), "abc");
        assert_eq!(marquee_window("abc", 3, 100), "abc");
    }

    #[test]
    fn marquee_window_pauses_at_start_then_scrolls() {
        for phase in 0..MARQUEE_PAUSE_TICKS {
            assert_eq!(marquee_window("abcdef", 3, phase), "abc");
        }
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS), "abc");
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS + 1), "bcd");
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS + 3), "def");
    }

    #[test]
    fn marquee_window_pauses_at_end_then_loops() {
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS + 4), "def");
        assert_eq!(marquee_window("abcdef", 3, 11), "abc");
    }

    #[test]
    fn display_label_marquees_only_for_active_selected() {
        assert_eq!(display_label("abcdef", 3, true, 0), "abc");
        assert_eq!(display_label("abcdef", 3, false, 0), "ab…");
    }
}
