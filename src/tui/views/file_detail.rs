//! File detail screen: shows all `FileMeta` fields + LLM classification

use std::sync::mpsc::{self, Receiver};
use std::thread;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{Action, Screen};
use crate::classifier::llm::LlmClient;
use crate::models::{Classification, FileMeta, human_readable_size};

/// Braille spinner frames for the "analyzing" state.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Async LLM classification status, updated from a background thread.
#[derive(Debug, Clone, PartialEq)]
enum LlmStatus {
    /// Not run (no LLM configured).
    Disabled,
    /// Request in flight.
    Loading,
    /// Model returned a verdict.
    Done(Classification),
    /// Request failed (Ollama down, parse error, etc.).
    Failed(String),
}

/// File detail state
pub(in crate::tui) struct FileDetailScreen {
    file: FileMeta,
    /// Local LLM client (kept for the model name display).
    llm: Option<LlmClient>,
    /// Async classification status.
    status: LlmStatus,
    /// Receiver for the background classification worker (cleared on done).
    rx: Option<Receiver<Result<Classification, String>>>,
    /// Spinner frame index, advanced while loading.
    spin: usize,
}

impl FileDetailScreen {
    pub(in crate::tui) fn new(file: FileMeta, llm: Option<LlmClient>) -> Self {
        let mut screen = Self {
            file,
            llm: llm.clone(),
            status: LlmStatus::Disabled,
            rx: None,
            spin: 0,
        };
        if let Some(client) = llm {
            screen.start_classification(client);
        }
        screen
    }

    /// Spawn a background thread that runs the LLM verdict for `self.file`.
    ///
    /// The blocking HTTP call would freeze the TUI, so it runs off the render
    /// loop and reports back through a channel polled on each tick.
    fn start_classification(&mut self, client: LlmClient) {
        let (tx, rx) = mpsc::channel::<Result<Classification, String>>();
        self.rx = Some(rx);
        self.status = LlmStatus::Loading;

        let file = self.file.clone();
        thread::spawn(move || {
            let prompt = client.build_prompt(&file);
            let result = client.ask(&prompt).map_err(|e| e.to_string());
            // best-effort send; if the screen was dropped, the receiver is gone
            let _ = tx.send(result);
        });
    }

    /// Poll the classification worker; advance the spinner while loading.
    fn poll_status(&mut self) {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(Ok(class)) => {
                    self.status = LlmStatus::Done(class);
                    self.rx = None;
                }
                Ok(Err(msg)) => {
                    self.status = LlmStatus::Failed(msg);
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => self.spin = (self.spin + 1) % SPINNER.len(),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = LlmStatus::Failed("worker disappeared".into());
                    self.rx = None;
                }
            }
        }
    }
}

/// Build the colored status line for the classification block.
fn status_spans(status: &LlmStatus, spin: usize, model: &str) -> Vec<Span<'static>> {
    match status {
        LlmStatus::Disabled => vec![Span::styled(
            format!(" LLM disabled (rebuild with --features llm) · model={model}"),
            Style::default().fg(Color::DarkGray),
        )],
        LlmStatus::Loading => vec![
            Span::styled(
                format!(" {} ", SPINNER[spin]),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!(
                "Analyzing with {model}… (CPU inference can take a while)"
            )),
        ],
        LlmStatus::Done(c) => verdict_spans(c),
        LlmStatus::Failed(msg) => {
            let first_line = msg.lines().next().unwrap_or("unknown error");
            vec![
                Span::styled(" ⚠ ", Style::default().fg(Color::LightRed)),
                Span::styled(
                    "LLM unavailable: ".to_string(),
                    Style::default().fg(Color::LightRed),
                ),
                Span::styled(first_line.to_string(), Style::default().fg(Color::Gray)),
            ]
        }
    }
}

/// Colored spans for a concrete verdict.
fn verdict_spans(c: &Classification) -> Vec<Span<'static>> {
    let score_note = match c {
        Classification::Uncertain(s) => format!(" (confidence {s:.2})"),
        _ => String::new(),
    };
    let (label, color, detail) = match c {
        Classification::Keep => ("KEEP", Color::Green, "the model suggests keeping it"),
        Classification::Trash => ("TRASH", Color::Red, "safe to delete"),
        Classification::Uncertain(_) => (
            "UNCERTAIN",
            Color::Yellow,
            "model is unsure; review manually",
        ),
    };
    vec![
        Span::styled(
            format!("  {label}  "),
            Style::default()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {detail}{score_note}")),
    ]
}

/// Render the screen. Kept free of `&self` so it borrows nothing while drawing.
fn render_detail(file: &FileMeta, status: &LlmStatus, spin: usize, model: &str, frame: &mut Frame) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .areas(frame.area());

    // header: name + size/ext summary
    let name = file
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.path.display().to_string());
    let ext = file.ext.as_deref().unwrap_or("no extension");
    let header_title = format!(" rustle — {name} ");
    let header_stats = format!("{} · {ext}", human_readable_size(file.size));
    frame.render_widget(
        Paragraph::new(header_stats)
            .block(Block::default().borders(Borders::ALL).title(header_title)),
        header,
    );

    let [info_area, class_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(body);

    let info_lines = vec![
        Line::raw(format!(" {:<13}{}", "Path:", file.path.display())),
        Line::raw(format!(
            " {:<13}{} ({} bytes)",
            "Size:",
            human_readable_size(file.size),
            file.size
        )),
        Line::raw(format!(
            " {:<13}{}",
            "Extension:",
            file.ext.as_deref().unwrap_or("—")
        )),
        Line::raw(format!(
            " {:<13}{} (UTC)",
            "Modified:",
            file.mtime.format("%Y-%m-%d %H:%M:%S")
        )),
    ];
    frame.render_widget(
        Paragraph::new(info_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" File information "),
        ),
        info_area,
    );

    // classification block
    let class_title = format!(" Classification (LLM: {model}) ");
    let class_line = Line::from(status_spans(status, spin, model));
    frame.render_widget(
        Paragraph::new(class_line).block(Block::default().borders(Borders::ALL).title(class_title)),
        class_area,
    );

    // footer: back hint
    frame.render_widget(
        Paragraph::new(" Esc/Backspace — back ").block(Block::default().borders(Borders::ALL)),
        footer,
    );
}

impl Screen for FileDetailScreen {
    fn draw(&mut self, frame: &mut Frame) {
        self.poll_status();
        let model = self
            .llm
            .as_ref()
            .map(LlmClient::model_name)
            .unwrap_or("(none)");
        render_detail(&self.file, &self.status, self.spin, model, frame);
    }

    fn tick(&mut self) {
        self.poll_status();
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => Action::Back,
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    fn sample_file() -> FileMeta {
        FileMeta {
            path: PathBuf::from("/root/sub/big.bin"),
            size: 4096,
            ext: Some("bin".to_string()),
            mtime: chrono::Utc::now(),
        }
    }

    #[test]
    fn draw_renders_filemeta_fields_without_panic() {
        let mut screen = FileDetailScreen::new(sample_file(), None);

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.draw(frame)).unwrap();

        let buf = terminal.backend().buffer();
        let content: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect();

        assert!(content.contains("File information"), "info block missing");
        assert!(content.contains("big.bin"), "path missing");
        assert!(content.contains("Extension"), "extension row missing");
        assert!(
            content.contains("Classification"),
            "classification block must render even without an LLM"
        );
    }

    #[test]
    fn esc_returns_back() {
        use crossterm::event::KeyModifiers;

        let mut screen = FileDetailScreen::new(sample_file(), None);
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Action::Back
        ));
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            Action::Back
        ));
    }

    #[test]
    fn unknown_key_is_ignored() {
        use crossterm::event::KeyModifiers;

        let mut screen = FileDetailScreen::new(sample_file(), None);
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Action::None
        ));
    }

    #[test]
    fn status_spans_keep_is_green() {
        let spans = status_spans(&LlmStatus::Done(Classification::Keep), 0, "qwen2.5:3b");
        assert!(spans.iter().any(|s| s.content.contains("KEEP")));
    }

    #[test]
    fn status_spans_loading_shows_model() {
        let spans = status_spans(&LlmStatus::Loading, 0, "qwen2.5:3b");
        let joined: String = spans.iter().map(|s| s.content.clone()).collect();
        assert!(
            joined.contains("qwen2.5:3b"),
            "loading line must name the model"
        );
    }
}
