//! Интерактивный цикл TUI: захват терминала, обработка ввода.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;

use super::app::{App, Focus, TICK_MS};
use super::draw;
use crate::error::RustleError;
use crate::platform;
use crate::scanner::ScanResult;

/// RAII-обёртка над терминалом: захватывает raw-mode + альт. экран при
/// создании и гарантированно восстанавливает состояние при падении/выходе.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self, RustleError> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // CONTEXT: восстановление терминала при выходе/панике — ошибки здесь
        // уже не исправить, поэтому сознательно игнорируем.
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

/// Запустить интерактивный TUI поверх результатов сканирования.
pub fn run(root: PathBuf, scan: ScanResult) -> Result<(), RustleError> {
    let mut app = App::new(root, scan);
    let mut guard = TerminalGuard::new()?;

    let mut dir_state = ListState::default();
    let mut file_state = ListState::default();
    dir_state.select(Some(0));
    file_state.select(Some(0));

    loop {
        guard
            .terminal
            .draw(|frame| draw::draw(&app, frame, &mut dir_state, &mut file_state))?;

        // Ждём событие не дольше TICK_MS: если ничего не нажато — продвигаем
        // анимацию бегущей строки и перерисовываемся.
        if event::poll(Duration::from_millis(TICK_MS))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                // Горизонтальная навигация: переключаем активную панель.
                KeyCode::Right | KeyCode::Char('l') => app.switch_focus(1),
                KeyCode::Left | KeyCode::Char('h') => app.switch_focus(-1),
                // Вертикальная навигация действует на активную панель.
                KeyCode::Down | KeyCode::Char('j') => match app.focus {
                    Focus::Directories => app.move_directory_selection(&mut dir_state, 1),
                    Focus::Files => app.move_file_selection(&mut file_state, 1),
                },
                KeyCode::Up | KeyCode::Char('k') => match app.focus {
                    Focus::Directories => app.move_directory_selection(&mut dir_state, -1),
                    Focus::Files => app.move_file_selection(&mut file_state, -1),
                },
                // Открыть выделенный путь в проводнике ОС.
                KeyCode::Char('g') => {
                    if let Some(path) = focused_selected_path(&app, &dir_state, &file_state) {
                        match platform::open_in_explorer(&path) {
                            Ok(()) => app.set_status(format!("Открываю: {}", path.display())),
                            Err(e) => app.set_status(format!("Ошибка: {e}")),
                        }
                    }
                }
                // Скопировать путь выделенного элемента в буфер обмена.
                KeyCode::Char('y') => {
                    if let Some(path) = focused_selected_path(&app, &dir_state, &file_state) {
                        match platform::copy_to_clipboard(&path.to_string_lossy()) {
                            Ok(()) => app.set_status(format!("Скопировано: {}", path.display())),
                            Err(e) => app.set_status(format!("Ошибка: {e}")),
                        }
                    }
                }
                _ => {}
            }
        } else {
            // Тик таймера — анимация бегущей строки.
            app.tick();
        }
    }
}

/// Путь выделенного элемента в активной панели (если есть).
fn focused_selected_path(
    app: &App,
    dir_state: &ListState,
    file_state: &ListState,
) -> Option<PathBuf> {
    match app.focus {
        Focus::Files => file_state
            .selected()
            .and_then(|i| app.top_files.get(i).map(|f| f.path.clone())),
        Focus::Directories => dir_state
            .selected()
            .and_then(|i| app.directories.get(i).map(|d| d.path.clone())),
    }
}
