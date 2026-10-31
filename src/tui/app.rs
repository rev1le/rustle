use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::tui::{session::Session, views::Action};

use super::views::{AddPathView, MainView, ScanView, Screen, View};

/// The single active screen. Adding a view means adding a variant here.
pub(super) enum Active {
    Main(MainView),
    AddPath(AddPathView),
    Scan(ScanView),
}

impl Active {
    /// Single factory for screens: adding a view means adding a variant here.
    fn new(view: View) -> Self {
        match view {
            View::Main => Active::Main(MainView::new()),
            View::AddPath => Active::AddPath(AddPathView::new()),
            View::Scan => Active::Scan(ScanView),
        }
    }

    fn screen_mut(&mut self) -> &mut dyn Screen {
        match self {
            Active::Main(view) => view,
            Active::AddPath(view) => view,
            Active::Scan(view) => view,
        }
    }

    fn is_main(&self) -> bool {
        matches!(self, Active::Main(_))
    }
}

pub(super) struct App {
    active: Active,
    session: Session,
}

impl App {
    pub(super) fn new() -> Self {
        Self {
            active: Active::new(View::Main),
            session: Session::default(),
        }
    }

    /// Draw the active view
    pub fn draw_view(&mut self, frame: &mut Frame) {
        let Self { active, session } = self;
        active.screen_mut().draw(frame, session);
    }

    /// Advance timers/animation one step
    pub fn tick(&mut self) {
        self.active.screen_mut().tick();
    }

    /// Dispatch a key to the active screen, resolving navigation actions.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        let action = {
            let Self { active, session } = self;
            active.screen_mut().handle_key(key, session)
        };
        self.apply(action)
    }

    fn apply(&mut self, action: Action) -> Action {
        match action {
            Action::Navigate(view) => {
                self.active = Active::new(view);
                Action::None
            }
            // Back on the first view means "there is nowhere left to go".
            Action::Back if self.active.is_main() => Action::Quit,
            // shortcut: Back re-creates MainView, so unfinished input is lost;
            // keep the view in App instead if that starts to matter.
            Action::Back => {
                self.active = Active::new(View::Main);
                Action::None
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests;
