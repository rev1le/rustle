//! Состояние приложения: карта экранов и диспетчеризация ввода/отрисовки.

use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use super::views::{Action, FileDetailScreen, MainScreen, Screen, View};
use crate::models::FileMeta;
use crate::scanner::ScanResult;

/// Период опроса ввода / тика анимации.
pub(super) const TICK_MS: u64 = 200;

/// Корневое состояние TUI: карта экранов и активный экран.
pub(super) struct App {
    /// Активный экран — ключ в `views`.
    pub(super) active: View,
    /// Сохранённые состояния всех посещённых экранов.
    /// При переключении значения не удаляются — данные прошлого экрана
    /// сохраняются и восстанавливаются при возврате.
    views: HashMap<View, Box<dyn Screen>>,
}

impl App {
    pub(super) fn new(root: PathBuf, scan: ScanResult) -> Self {
        let mut views: HashMap<View, Box<dyn Screen>> = HashMap::new();
        views.insert(View::Main, Box::new(MainScreen::new(root, scan)));
        Self {
            active: View::Main,
            views,
        }
    }

    /// Активный экран (мутабельно).
    fn screen_mut(&mut self) -> &mut dyn Screen {
        self.views
            .get_mut(&self.active)
            .expect("активный экран всегда присутствует в карте")
            .as_mut()
    }

    /// Отрисовать активный экран.
    pub(super) fn draw_active(&mut self, frame: &mut Frame) {
        self.screen_mut().draw(frame);
    }

    /// Продвинуть таймеры/анимацию активного экрана на один шаг.
    pub(super) fn tick(&mut self) {
        self.screen_mut().tick();
    }

    /// Обработать нажатие клавиши активным экраном.
    ///
    /// Возвращает `Action::Quit`, если приложение должно завершиться; переходы
    /// между экранами (`OpenDetail`, `Back`) выполняются здесь же, над картой.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Запрос экрана — владеемое значение, поэтому заимновение карты здесь
        // заканчивается до того, как `apply` потребует мутабельный доступ.
        let action = self.screen_mut().handle_key(key);
        self.apply(action)
    }

    /// Применить запрос экрана к карте: переходы гасятся в `None`, выход
    /// прокидывается наверх к циклу ввода.
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

    /// Открыть экран деталей файла, замещая прежнее состояние деталей.
    /// Состояние основного экрана сохраняется в карте.
    fn open_detail(&mut self, file: FileMeta) {
        self.views
            .insert(View::FileDetail, Box::new(FileDetailScreen::new(file)));
        self.active = View::FileDetail;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_fixtures::{file, scan_of};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Послать в приложение «нажатие» символьной клавиши без модификаторов.
    fn press(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
    }

    #[test]
    fn active_starts_on_main() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert_eq!(app.active, View::Main);
    }

    #[test]
    fn enter_on_files_opens_detail() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a.bin", 42, Some("bin"))]),
        );

        // Переключаемся на панель файлов и открываем детали первого файла.
        app.handle_key(press('l'));
        let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);
        // Переход обработан на уровне App — наружу возвращается None.
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn esc_from_detail_returns_to_main() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a.bin", 42, Some("bin"))]),
        );
        app.handle_key(press('l'));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);

        // Esc на экране деталей → назад.
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active, View::Main);
    }

    #[test]
    fn q_from_main_requests_quit() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert!(matches!(app.handle_key(press('q')), Action::Quit));
    }

    #[test]
    fn ctrl_c_from_main_requests_quit() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
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
        );

        // Основной экран продолжает отвечать после возврата из деталей:
        // состояние основного экрана не удалялось из карты.
        app.handle_key(press('l'));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active, View::Main);

        // Повторный переход работает — основной экран «жив».
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active, View::FileDetail);
    }
}
