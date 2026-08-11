use crossterm::event::KeyCode;
use ratatui::{Frame, crossterm::event::KeyEvent};

use crate::tui::views::{Action, Screen};

pub struct MainView {}

impl MainView {
    pub fn new() -> Self {
        Self {}
    }
}

impl Screen for MainView {
    fn draw(&mut self, frame: &mut Frame) {}

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        return match key.code {
            KeyCode::Esc => Action::Quit,
            _ => Action::None,
        };
    }
}
