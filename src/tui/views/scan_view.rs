//! Placeholder for the next step: shows the folders picked in [`MainView`].
//! Actual scanning and results land here later.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
};

use crate::tui::{
    session::Session,
    views::{Action, Screen},
};

pub struct ScanView;

impl Screen for ScanView {
    fn draw(&mut self, frame: &mut Frame, session: &Session) {
        let [title_area, list_area, hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        let title = Paragraph::new(Line::from(vec![
            Span::styled("🔍 ", Style::default()),
            Span::styled(
                "К сканированию",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        frame.render_widget(title, title_area);

        let items: Vec<ListItem<'static>> = session
            .folders
            .iter()
            .map(|path| {
                ListItem::new(Line::from(vec![
                    Span::styled(" 📁 ", Style::default()),
                    Span::raw(path.display().to_string()),
                ]))
            })
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(format!(" Папки ({}) ", session.folders.len())));
        frame.render_widget(list, list_area);

        let hint = Paragraph::new(Span::styled(
            " Esc — назад",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(hint, hint_area);
    }

    fn handle_key(&mut self, key: KeyEvent, _session: &mut Session) -> Action {
        match key.code {
            KeyCode::Esc => Action::Back,
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_should_request_back() {
        let mut view = ScanView;
        let mut session = Session::default();

        assert_eq!(
            view.handle_key(
                KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE),
                &mut session
            ),
            Action::Back
        );
    }
}
