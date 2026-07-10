//! TUI screens + common interface

mod file_detail;
mod main;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::models::FileMeta;

pub(super) use file_detail::FileDetailScreen;
pub(super) use main::MainScreen;

/// Screen id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum View {
    Main,
    FileDetail,
}

pub(super) enum Action {
    None,
    Quit,
    Back,
    OpenDetail(FileMeta),
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
