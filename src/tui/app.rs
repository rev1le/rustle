use std::collections::HashMap;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::tui::views::Action;

use super::views::{MainView, Screen, View};

type ViewsMap = HashMap<View, Box<dyn Screen>>;

pub(super) struct App {
    active: View,
    views: ViewsMap,
}

impl App {
    pub(super) fn new() -> Self {
        let mut views: ViewsMap = HashMap::new();

        let main_view = MainView::new();
        views.insert(main_view.get_type(), Box::new(main_view));

        Self {
            active: View::Main,
            views,
        }
    }

    /// Active screen, mutably
    fn screen_mut(&mut self) -> &mut dyn Screen {
        self.views
            .get_mut(&self.active)
            .expect("active screen always present")
            .as_mut()
    }

    /// Draw the active view
    pub fn draw_view(&mut self, frame: &mut Frame) {
        self.screen_mut().draw(frame);
    }

    /// Advance timers/animation one step
    pub fn tick(&mut self) {
        self.screen_mut().tick();
    }

    /// Dispatch a key to the active screen
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        let action = self.screen_mut().handle_key(key);
        return action;
    }
}
