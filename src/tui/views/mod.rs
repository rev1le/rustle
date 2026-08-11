pub mod main_view;

pub use main_view::MainView;

use ratatui::{Frame, crossterm::event::KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    Main,
}

pub enum Action {
    None,
    Quit,
}

/// Common behavior for all screens
pub(super) trait Screen {
    /// Draw into the frame
    fn draw(&mut self, frame: &mut Frame);

    /// Handle a key, return an action
    fn handle_key(&mut self, key: KeyEvent) -> Action;

    /// Advance timers/animation one step
    fn tick(&mut self) {}
}
