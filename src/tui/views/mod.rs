//! Экраны TUI и их общий интерфейс.

mod file_detail;
mod main;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::models::FileMeta;

pub(super) use file_detail::FileDetailScreen;
pub(super) use main::MainScreen;

/// Идентификатор экрана
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

/// Поведение, общее для всех экранов TUI.
pub(super) trait Screen {
    /// Отрисовать экран в переданный кадр.
    fn draw(&mut self, frame: &mut Frame);

    /// Обработать нажатие клавиши и вернуть действие для приложения.
    fn handle_key(&mut self, key: KeyEvent) -> Action;

    /// Продвинуть таймеры/анимацию на один шаг (вызывается по таймеру).
    fn tick(&mut self) {}
}
