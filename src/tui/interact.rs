//! Интерактивный цикл TUI: захват терминала, обработка ввода.
//!
//! Цикл максимально тонкий: отрисовку, ввод и таймеры делегирует активному
//! экрану через `App`; сам разбирается лишь с выходом (`Action::Quit`).

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::app::{App, TICK_MS};
use super::views::Action;
use crate::error::RustleError;
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

    loop {
        guard.terminal.draw(|frame| app.draw_active(frame))?;

        // Ждём событие не дольше TICK_MS: если ничего не нажато — продвигаем
        // анимацию активного экрана и перерисовываемся.
        if event::poll(Duration::from_millis(TICK_MS))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if let Action::Quit = app.handle_key(key) {
                return Ok(());
            }
        } else {
            // Тик таймера — анимация бегущей строки/статусы.
            app.tick();
        }
    }
}
