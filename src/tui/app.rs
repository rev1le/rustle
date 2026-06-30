//! Состояние приложения: агрегированные данные сканирования и навигация.

use std::path::PathBuf;

use ratatui::widgets::ListState;

use crate::models::{DirAggregate, FileMeta};
use crate::scanner::ScanResult;

/// Сколько каталогов показывать в основной панели.
pub(super) const MAX_DIRS: usize = 50;
/// Сколько файлов показывать в боковой панели.
pub(super) const MAX_FILES: usize = 30;

pub(super) const TICK_MS: u64 = 200;
pub(super) const MARQUEE_PAUSE_TICKS: usize = 4;

/// Какая из двух панелей сейчас принимает ввод.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Directories,
    Files,
}

/// Состояние TUI: агрегированные данные сканирования + навигация.
pub(super) struct App {
    pub(super) root: PathBuf,
    pub(super) total_files: usize,
    pub(super) skipped: usize,
    pub(super) total_size: u64,
    pub(super) directories: Vec<DirAggregate>,
    pub(super) top_files: Vec<FileMeta>,
    pub(super) focus: Focus,
    pub(super) scroll_phase: usize,
}

impl App {
    pub(super) fn new(root: PathBuf, scan: ScanResult) -> Self {
        let total_size: u64 = scan.files.iter().map(|f| f.size).sum();
        let total_files = scan.files.len();
        let skipped = scan.skipped;

        // Только подкаталоги внутри корня — отсекаем сам корень и родителей.
        let mut directories = crate::scanner::aggregator::aggregate(&scan.files);
        directories.retain(|d| d.path != root && d.path.starts_with(&root));
        directories.truncate(MAX_DIRS);

        let top_files = crate::scanner::aggregator::top_largest(&scan.files, MAX_FILES);

        Self {
            root,
            total_files,
            skipped,
            total_size,
            directories,
            top_files,
            focus: Focus::Directories,
            scroll_phase: 0,
        }
    }

    /// Сдвинуть выделение в списке каталогов, ограничив границы.
    /// Смена выделения сбрасывает фазу анимации бегущей строки.
    pub(super) fn move_directory_selection(&mut self, state: &mut ListState, delta: isize) {
        move_stateful_selection(state, self.directories.len(), delta);
        self.scroll_phase = 0;
    }

    /// Сдвинуть выделение в списке файлов, ограничив границы.
    /// Смена выделения сбрасывает фазу анимации бегущей строки.
    pub(super) fn move_file_selection(&mut self, state: &mut ListState, delta: isize) {
        move_stateful_selection(state, self.top_files.len(), delta);
        self.scroll_phase = 0;
    }

    /// Переключить фокус между панелями.
    ///
    /// `direction > 0` — вправо (каталоги → файлы), `< 0` — влево.
    /// На краях переход игнорируется. Смена фокуса сбрасывает анимацию.
    pub(super) fn switch_focus(&mut self, direction: isize) {
        self.focus = match (self.focus, direction) {
            (Focus::Directories, 1) => Focus::Files,
            (Focus::Files, -1) => Focus::Directories,
            (current, _) => current,
        };
        self.scroll_phase = 0;
    }

    /// Продвинуть анимацию бегущей строки на один шаг (вызывается по таймеру).
    pub(super) fn tick(&mut self) {
        self.scroll_phase = self.scroll_phase.wrapping_add(1);
    }
}

/// Общий helper сдвига выделения в `ListState` для списка длины `len`.
fn move_stateful_selection(state: &mut ListState, len: usize, delta: isize) {
    if len == 0 {
        return;
    }
    let current = state.selected().unwrap_or(0);
    let next = (current as isize + delta).clamp(0, (len - 1) as isize) as usize;
    state.select(Some(next));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_fixtures::{file, scan_of};

    #[test]
    fn move_directory_selection_clamps_to_bounds() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub1/x", 1, None),
                file("/root/sub2/y", 2, None),
            ]),
        );
        // После агрегации остаются два подкаталога (сам /root отсечён).
        assert_eq!(app.directories.len(), 2);

        let mut state = ListState::default();
        state.select(Some(0));

        app.move_directory_selection(&mut state, 100);
        assert_eq!(state.selected(), Some(1));

        app.move_directory_selection(&mut state, -100);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_file_selection_clamps_to_bounds() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/a", 1, None),
                file("/root/b", 2, None),
                file("/root/c", 3, None),
            ]),
        );
        assert_eq!(app.top_files.len(), 3);

        let mut state = ListState::default();
        state.select(Some(0));

        app.move_file_selection(&mut state, 100);
        assert_eq!(state.selected(), Some(2));

        app.move_file_selection(&mut state, -100);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_directory_selection_is_noop_without_directories() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert!(app.directories.is_empty());

        let mut state = ListState::default();
        state.select(Some(0));
        app.move_directory_selection(&mut state, 5);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn switch_focus_toggles_between_panels() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub/a", 1, None),
                file("/root/sub2/b", 2, None),
            ]),
        );

        // По умолчанию активна панель каталогов.
        assert_eq!(app.focus, Focus::Directories);

        // Влево из каталогов — ничего не делаем (уже на левом краю).
        app.switch_focus(-1);
        assert_eq!(app.focus, Focus::Directories);

        // Вправо → файлы.
        app.switch_focus(1);
        assert_eq!(app.focus, Focus::Files);

        // Вправо из файлов — ничего не делаем (уже на правом краю).
        app.switch_focus(1);
        assert_eq!(app.focus, Focus::Files);

        // Влево → обратно в каталоги.
        app.switch_focus(-1);
        assert_eq!(app.focus, Focus::Directories);
    }

    #[test]
    fn tick_advances_scroll_phase() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/a", 1, None)]),
        );
        assert_eq!(app.scroll_phase, 0);

        app.tick();
        app.tick();
        assert_eq!(app.scroll_phase, 2);
    }

    #[test]
    fn navigation_resets_scroll_phase() {
        let mut app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub1/a", 1, None),
                file("/root/sub2/b", 2, None),
            ]),
        );
        // «накрутим» фазу анимации
        app.tick();
        app.tick();
        app.tick();
        assert_eq!(app.scroll_phase, 3);

        // Смена выделения в каталогах сбрасывает фазу.
        let mut state = ListState::default();
        state.select(Some(0));
        app.move_directory_selection(&mut state, 1);
        assert_eq!(app.scroll_phase, 0);

        // Переключение фокуса тоже сбрасывает.
        app.tick();
        assert_eq!(app.scroll_phase, 1);
        app.switch_focus(1);
        assert_eq!(app.scroll_phase, 0);
    }
}
