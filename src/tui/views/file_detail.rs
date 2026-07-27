//! File detail screen: shows `FileMeta` + L2 heuristics, L3 LLM on demand

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{Action, Screen};
use crate::classifier::Classifier;
use crate::models::{Classification, FileMeta, human_readable_size};

/// Braille spinner frames for the "analyzing" state.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Async LLM classification status, updated from a background thread.
#[derive(Debug, Clone, PartialEq)]
enum LlmStatus {
    Disabled,
    Idle,
    Loading,
    Done(Classification),
    Failed(String),
}

/// File detail state
pub(in crate::tui) struct FileDetailScreen {
    file: FileMeta,
    /// Classifier used for both the L2 verdict and the on-demand L3 verdict.
    classifier: Arc<Classifier>,
    heuristics_verdict: Classification,
    heuristics_score: f32,
    /// On-demand LLM classification status.
    status: LlmStatus,
    /// Receiver for the background classification worker (cleared on done).
    rx: Option<Receiver<Result<Classification, String>>>,
    /// Spinner frame index, advanced while loading.
    spin: usize,
}

impl FileDetailScreen {
    pub(in crate::tui) fn new(file: FileMeta, classifier: Arc<Classifier>) -> Self {
        let heuristics_verdict = classifier.classify_l2(&file);
        let heuristics_score = classifier.score_l2(&file);
        let status = if classifier.llm_model_name().is_some() {
            LlmStatus::Idle
        } else {
            LlmStatus::Disabled
        };
        Self {
            file,
            classifier,
            heuristics_verdict,
            heuristics_score,
            status,
            rx: None,
            spin: 0,
        }
    }

    /// Trigger LLM analysis on `k`, but only from a state where it makes sense.
    fn maybe_start_llm(&mut self) {
        match self.status {
            LlmStatus::Idle | LlmStatus::Failed(_) => self.start_llm(),
            // Disabled (no LLM), Loading (already running), Done (have a verdict).
            _ => {}
        }
    }

    /// Spawn a background thread that runs the L3 verdict for `self.file`.
    ///
    /// The blocking HTTP call would freeze the TUI, so it runs off the render
    /// loop and reports back through a channel polled on each tick.
    fn start_llm(&mut self) {
        let (tx, rx) = mpsc::channel::<Result<Classification, String>>();
        self.rx = Some(rx);
        self.status = LlmStatus::Loading;

        let classifier = self.classifier.clone();
        let file = self.file.clone();
        thread::spawn(move || {
            let result = classifier
                .classify_llm(&file)
                .ok_or_else(|| "LLM declined to classify this file".to_string());
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

/// Colored verdict badge, reused by both the L2 and L3 lines.
fn verdict_badge(c: &Classification) -> Span<'static> {
    let (label, color) = match c {
        Classification::Keep => ("KEEP", Color::Green),
        Classification::Trash => ("TRASH", Color::Red),
        Classification::Uncertain => ("UNCERTAIN", Color::Yellow),
    };
    Span::styled(
        format!("  {label}  "),
        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

/// One-line description of an LLM verdict.
fn llm_detail(c: &Classification) -> String {
    match c {
        Classification::Keep => "the model suggests keeping it".into(),
        Classification::Trash => "safe to delete".into(),
        Classification::Uncertain => "model is unsure".into(),
    }
}

/// Spans for the L2 heuristics line.
fn heuristics_spans(verdict: Classification, score: f32) -> Vec<Span<'static>> {
    match verdict {
        Classification::Uncertain => vec![
            verdict_badge(&verdict),
            Span::raw(format!(" {}", crate::explain_classification::declined())),
        ],
        class => vec![
            verdict_badge(&class),
            Span::raw(format!(
                " {}",
                crate::explain_classification::heuristics(class, score)
            )),
        ],
    }
}

/// Spans for the L3 LLM line (status-driven).
fn llm_spans(status: &LlmStatus, spin: usize, model: &str) -> Vec<Span<'static>> {
    match status {
        LlmStatus::Disabled => vec![Span::styled(
            " L3 LLM: disabled (no model configured)",
            Style::default().fg(Color::DarkGray),
        )],
        LlmStatus::Idle => vec![Span::styled(
            format!(" L3 LLM: press 'k' to analyze with {model}"),
            Style::default().fg(Color::DarkGray),
        )],
        LlmStatus::Loading => vec![
            Span::styled(
                format!(" {} ", SPINNER[spin]),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!(" L3 LLM: analyzing with {model}…")),
        ],
        LlmStatus::Done(c) => vec![
            verdict_badge(c),
            Span::raw(format!(" L3 LLM: {}", llm_detail(c))),
        ],
        LlmStatus::Failed(msg) => {
            let first_line = msg.lines().next().unwrap_or("unknown error");
            vec![
                Span::styled(" ⚠ ", Style::default().fg(Color::LightRed)),
                Span::styled(
                    " L3 LLM unavailable: ",
                    Style::default().fg(Color::LightRed),
                ),
                Span::styled(first_line.to_string(), Style::default().fg(Color::Gray)),
            ]
        }
    }
}

/// Render the screen. Kept free of `&self` so it borrows nothing while drawing.
fn render_detail(
    file: &FileMeta,
    heuristics_verdict: Classification,
    heuristics_score: f32,
    status: &LlmStatus,
    spin: usize,
    model: &str,
    frame: &mut Frame,
) {
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
        Layout::vertical([Constraint::Fill(1), Constraint::Length(6)]).areas(body);

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

    // classification block: L2 line first, L3 line below
    let class_title = format!(" Classification (LLM: {model}) ");
    let class_lines = vec![
        Line::from(heuristics_spans(heuristics_verdict, heuristics_score)),
        Line::from(llm_spans(status, spin, model)),
    ];
    frame.render_widget(
        Paragraph::new(class_lines)
            .block(Block::default().borders(Borders::ALL).title(class_title)),
        class_area,
    );

    // footer: hints
    frame.render_widget(
        Paragraph::new(" k — LLM analyze · Esc/Backspace — back ")
            .block(Block::default().borders(Borders::ALL)),
        footer,
    );
}

impl Screen for FileDetailScreen {
    fn draw(&mut self, frame: &mut Frame) {
        self.poll_status();
        let model = self.classifier.llm_model_name().unwrap_or("(none)");
        render_detail(
            &self.file,
            self.heuristics_verdict,
            self.heuristics_score,
            &self.status,
            self.spin,
            model,
            frame,
        );
    }

    fn tick(&mut self) {
        self.poll_status();
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => Action::Back,
            KeyCode::Char('k') => {
                self.maybe_start_llm();
                Action::None
            }
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_fixtures::classifier;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;
    use std::sync::Arc;

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
        let mut screen = FileDetailScreen::new(sample_file(), Arc::new(classifier()));

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
            "classification block must render"
        );
        assert!(content.contains("L2 heuristics"), "L2 line missing");
    }

    #[test]
    fn heuristics_verdict_is_available_on_open() {
        // Fresh, small, clean-path file → L2 returns a confident Keep.
        let screen = FileDetailScreen::new(sample_file(), Arc::new(classifier()));
        assert_eq!(screen.heuristics_verdict, Classification::Keep);
        // LLM not requested yet → Idle (no LLM configured in the fixture).
        assert_eq!(screen.status, LlmStatus::Disabled);
    }

    #[test]
    fn esc_returns_back() {
        use crossterm::event::KeyModifiers;

        let mut screen = FileDetailScreen::new(sample_file(), Arc::new(classifier()));
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

        let mut screen = FileDetailScreen::new(sample_file(), Arc::new(classifier()));
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Action::None
        ));
    }

    #[test]
    fn k_is_accepted_and_does_nothing_without_llm() {
        use crossterm::event::KeyModifiers;

        let mut screen = FileDetailScreen::new(sample_file(), Arc::new(classifier()));
        // No LLM configured → k must not crash and stays Disabled.
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
            Action::None
        ));
        assert_eq!(screen.status, LlmStatus::Disabled);
    }

    #[test]
    fn heuristics_spans_shows_verdict_badge() {
        let spans = heuristics_spans(Classification::Keep, 0.1);
        assert!(spans.iter().any(|s| s.content.contains("KEEP")));
    }

    #[test]
    fn heuristics_spans_uncertain_shows_score_badge() {
        let spans = heuristics_spans(Classification::Uncertain, 0.45);
        assert!(spans.iter().any(|s| s.content.contains("UNCERTAIN")));
    }

    #[test]
    fn heuristics_spans_uncertain_uses_placeholder_text() {
        // Uncertain verdict → neutral placeholder, not the score line.
        let spans = heuristics_spans(Classification::Uncertain, 0.5);
        assert!(spans.iter().any(|s| s.content.contains("uncertain")));
        assert!(spans.iter().any(|s| s.content.contains("awaiting LLM")));
    }

    #[test]
    fn llm_spans_idle_mentions_model() {
        let spans = llm_spans(&LlmStatus::Idle, 0, "qwen2.5:3b");
        let joined: String = spans.iter().map(|s| s.content.clone()).collect();
        assert!(
            joined.contains("qwen2.5:3b"),
            "idle line must name the model"
        );
    }

    #[test]
    fn llm_spans_done_shows_verdict() {
        let spans = llm_spans(&LlmStatus::Done(Classification::Trash), 0, "qwen2.5:3b");
        assert!(spans.iter().any(|s| s.content.contains("TRASH")));
    }
}
