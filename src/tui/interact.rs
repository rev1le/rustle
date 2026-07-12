//! TUI input loop: terminal capture + event polling

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::app::{App, TICK_MS};
use super::views::Action;
use crate::classifier::Classifier;
use crate::error::RustleError;
use crate::scanner::ScanResult;

/// RAII: captures raw-mode + alt screen, restores on drop/panic
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
        // can't recover on drop — ignore errors
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

/// Run the interactive TUI over scan results
pub fn run(root: PathBuf, scan: ScanResult, classifier: Classifier) -> Result<(), RustleError> {
    let mut app = App::new(root, scan, classifier);
    let mut guard = TerminalGuard::new()?;

    loop {
        guard.terminal.draw(|frame| app.draw_active(frame))?;

        // poll up to TICK_MS; on timeout, advance animation and redraw
        if event::poll(Duration::from_millis(TICK_MS))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if let Action::Quit = app.handle_key(key) {
                return Ok(());
            }
        } else {
            // timer tick: marquee + statuses
            app.tick();
        }
    }
}
