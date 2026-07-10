//! Экран деталей выбранного файла: показывает все поля `FileMeta`.
//!
//! Логика простая — отрисовать сведения о файле и вернуться назад по Esc.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{Action, Screen};
use crate::models::{FileMeta, human_readable_size};

/// Состояние экрана деталей файла.
pub(in crate::tui) struct FileDetailScreen {
    file: FileMeta,
}

impl FileDetailScreen {
    pub(in crate::tui) fn new(file: FileMeta) -> Self {
        Self { file }
    }

    /// Отрисовка деталей файла (вынесена отдельно от `draw`, чтобы принимать
    /// неизменяемую ссылку на файл).
    fn render_detail(file: &FileMeta, frame: &mut Frame) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());

        // Заголовок: имя файла + краткая сводка (размер, расширение).
        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.path.display().to_string());
        let ext = file.ext.as_deref().unwrap_or("без расширения");
        let header_title = format!(" rustle — {name} ");
        let header_stats = format!("{} · {ext}", human_readable_size(file.size));
        frame.render_widget(
            Paragraph::new(header_stats)
                .block(Block::default().borders(Borders::ALL).title(header_title)),
            header,
        );

        // Тело: все поля FileMeta.
        let lines = vec![
            Line::raw(format!(" {:<13}{}", "Путь:", file.path.display())),
            Line::raw(format!(
                " {:<13}{} ({} байт)",
                "Размер:",
                human_readable_size(file.size),
                file.size
            )),
            Line::raw(format!(
                " {:<13}{}",
                "Расширение:",
                file.ext.as_deref().unwrap_or("—")
            )),
            Line::raw(format!(
                " {:<13}{} (UTC)",
                "Изменён:",
                file.mtime.format("%Y-%m-%d %H:%M:%S")
            )),
        ];
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Информация о файле ");
        frame.render_widget(Paragraph::new(lines).block(block), body);

        // Футер: подсказка возврата.
        frame.render_widget(
            Paragraph::new(" Esc/Backspace — назад ").block(Block::default().borders(Borders::ALL)),
            footer,
        );
    }
}

impl Screen for FileDetailScreen {
    fn draw(&mut self, frame: &mut Frame) {
        Self::render_detail(&self.file, frame);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => Action::Back,
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    fn sample_file() -> FileMeta {
        FileMeta {
            path: PathBuf::from("/root/sub/big.bin"),
            size: 4096,
            ext: Some("bin".to_string()),
            mtime: chrono::Utc::now(),
        }
    }

    #[test]
    fn draw_renders_filemeta_fields_without_panic() {
        let mut screen = FileDetailScreen::new(sample_file());

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| screen.draw(frame)).unwrap();

        let buf = terminal.backend().buffer();
        let content: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect();

        assert!(
            content.contains("Информация о файле"),
            "должен быть заголовок"
        );
        assert!(content.contains("big.bin"), "должен быть путь");
        assert!(
            content.contains("Расширение"),
            "должна быть строка расширения"
        );
    }

    #[test]
    fn esc_returns_back() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut screen = FileDetailScreen::new(sample_file());
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Action::Back
        ));
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            Action::Back
        ));
    }

    #[test]
    fn unknown_key_is_ignored() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut screen = FileDetailScreen::new(sample_file());
        assert!(matches!(
            screen.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Action::None
        ));
    }
}
