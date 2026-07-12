//! App state: screen map + input/draw dispatch

use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use super::views::{Action, FileDetailScreen, MainScreen, Screen, View};
use crate::classifier::llm::LlmClient;
use crate::models::FileMeta;
use crate::scanner::ScanResult;

/// Input poll / animation tick interval
pub(super) const TICK_MS: u64 = 200;

/// Root TUI state: screen map + active screen
pub(super) struct App {
    /// Key into `views`
    pub(super) active: View,
    /// Visited screens are kept on switch so their state survives a return
    views: HashMap<View, Box<dyn Screen>>,
    /// Optional local LLM client, passed into the FileDetail screen
    llm: Option<LlmClient>,
}

impl App {
    pub(super) fn new(root: PathBuf, scan: ScanResult, llm: Option<LlmClient>) -> Self {
        let mut views: HashMap<View, Box<dyn Screen>> = HashMap::new();
        views.insert(View::Main, Box::new(MainScreen::new(root, scan)));
        Self {
            active: View::Main,
            views,
            llm,
        }
    }

    /// Active screen, mutably
    fn screen_mut(&mut self) -> &mut dyn Screen {
        self.views
            .get_mut(&self.active)
            .expect("active screen always present")
            .as_mut()
    }

    /// Draw the active screen
    pub(super) fn draw_active(&mut self, frame: &mut Frame) {
        self.screen_mut().draw(frame);
    }

    /// Advance timers/animation one step
    pub(super) fn tick(&mut self) {
        self.screen_mut().tick();
    }

    /// Dispatch a key to the active screen
    ///
    /// Returns `Action::Quit` to exit; screen transitions are applied to the map here
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Action {
        // owned action ends the map borrow before `apply` mutates
        let action = self.screen_mut().handle_key(key);
        self.apply(action)
    }

    /// Apply transitions to the map; quit propagates up
    fn apply(&mut self, action: Action) -> Action {
        match action {
            Action::OpenDetail(file) => {
                self.open_detail(file);
                Action::None
            }
            Action::Back => {
                self.active = View::Main;
                Action::None
            }
            other => other,
        }
    }

    /// Open file detail, replacing prior detail state (main state preserved)
    fn open_detail(&mut self, file: FileMeta) {
        self.views.insert(
            View::FileDetail,
            Box::new(FileDetailScreen::new(file, self.llm.clone())),
        );
        self.active = View::FileDetail;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_fixtures::{file, scan_of};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Send a char key press with no modifiers
    fn press(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
    }

    #[test]
    fn active_starts_on_main() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
            None,
        );
        assert_eq!(app.active, View::Main);
    }

    #[test]
    fn enter_on_files_opens_detail() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a.bin", 42, Some("bin"))]),
            None,
        );

        // switch to files panel, open first file's details
        app.handle_key(press('l'));
        let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);
        // App handles the transition — returns None
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn esc_from_detail_returns_to_main() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a.bin", 42, Some("bin"))]),
            None,
        );
        app.handle_key(press('l'));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);

        // Esc on detail → back
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active, View::Main);
    }

    #[test]
    fn q_from_main_requests_quit() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
            None,
        );
        assert!(matches!(app.handle_key(press('q')), Action::Quit));
    }

    #[test]
    fn ctrl_c_from_main_requests_quit() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
            None,
        );
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(matches!(app.handle_key(ctrl_c), Action::Quit));
    }

    #[test]
    fn roundtrip_to_detail_and_back_keeps_app_working() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub1/a", 1, None),
                file("/root/sub2/b", 2, None),
            ]),
            None,
        );

        // main screen still responds after returning: its state was kept
        app.handle_key(press('l'));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active, View::Main);

        // second transition works — main screen is alive
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);
    }
}
