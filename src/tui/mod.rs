mod app;
mod terminal;
mod views;

use std::time::Duration;

use crate::{
    error::RustleError,
    tui::{terminal::TerminalGuard, views::Action},
};
use app::App;
use crossterm::event::{self, Event, KeyEventKind};

const TICK_MS: u64 = 200;

pub fn run() -> Result<(), RustleError> {
    let mut app = App::new();
    let mut guard = TerminalGuard::new()?;

    loop {
        guard.terminal.draw(|frame| app.draw_active(frame))?;

        if !event::poll(Duration::from_millis(TICK_MS))? {
            app.tick();
            continue;
        }

        let handle_action: Action;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                handle_action = app.handle_key(key);
            }
            _ => continue,
        }

        if let Action::Quit = handle_action {
            break;
        }
    }

    Ok(())
}
