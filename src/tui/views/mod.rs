pub mod add_path_view;
pub mod main_view;
pub mod scan_view;

pub use add_path_view::AddPathView;
pub use main_view::MainView;
pub use scan_view::ScanView;

use ratatui::{Frame, crossterm::event::KeyEvent};

use crate::tui::session::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
    AddPath,
    Scan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    /// Move forward to another view.
    Navigate(View),
    /// Go one step back; quitting on the first view is the app's call.
    Back,
}

/// Common behavior for all screens.
///
/// Screens never switch themselves: they return an [`Action`] and
/// [`crate::tui::app::App`] owns the active view and the shared [`Session`].
pub(super) trait Screen {
    /// Draw into the frame
    fn draw(&mut self, frame: &mut Frame, session: &Session);

    /// Handle a key, return an action
    fn handle_key(&mut self, key: KeyEvent, session: &mut Session) -> Action;

    /// Advance timers/animation one step
    fn tick(&mut self) {}
}
