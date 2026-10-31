use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};

use crate::tui::{
    session::Session,
    views::{Action, Screen},
};

#[cfg(test)]
mod tests;

pub struct AddPathView {
    /// Current contents of the path input.
    input: String,
    /// Path rejected by validation, shown as a centered toast until dismissed.
    toast: Option<String>,
}

impl AddPathView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            toast: None,
        }
    }

    /// Path input; scrolls to keep the cursor visible.
    fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let inner_width = area.width.saturating_sub(2).max(1) as usize;
        let cursor_col = self.input.chars().count();
        let scroll = cursor_col.saturating_sub(inner_width.saturating_sub(1));

        let input = Paragraph::new(self.input.as_str())
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Путь к папке "),
            )
            .scroll((0, scroll as u16));
        frame.render_widget(input, area);

        frame.set_cursor_position((area.x + 1 + (cursor_col - scroll) as u16, area.y + 1));
    }

    /// Centered overlay shown when validation rejects the entered path.
    fn draw_toast(&self, frame: &mut Frame, area: Rect) {
        let Some(path) = self.toast.as_deref() else {
            return;
        };

        let message = format!("⚠ Путь не найден: {path}");
        let width = (message.chars().count() as u16 + 4).max(24);
        let rect = centered_rect(area, width, 4);

        let toast = Paragraph::new(vec![
            Line::from(Span::styled(
                message,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Enter / Esc — закрыть",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Ошибка "),
        );

        frame.render_widget(Clear, rect);
        frame.render_widget(toast, rect);
    }

    /// Validate and store the trimmed input; `true` when the view may close.
    fn add_folder(&mut self, session: &mut Session) -> bool {
        let path = self.input.trim();
        if path.is_empty() {
            return false;
        }

        let path = PathBuf::from(path);
        if !path.exists() {
            self.toast = Some(path.display().to_string());
            return false;
        }

        if !session.folders.contains(&path) {
            session.folders.push(path);
        }
        true
    }
}

impl Screen for AddPathView {
    fn draw(&mut self, frame: &mut Frame, _session: &Session) {
        let area = frame.area();
        frame.render_widget(Clear, area);

        let outer = Block::bordered().border_type(BorderType::Rounded);
        let content = outer.inner(area).inner(Margin::new(2, 0));
        frame.render_widget(outer, area);

        let [title_area, _, panel_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas(content);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("🦀 ", Style::default()),
                Span::styled(
                    format!("rustle v{}", env!("CARGO_PKG_VERSION")),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(" — Add Scan Target", Style::default().fg(Color::DarkGray)),
            ])),
            title_area,
        );

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(2));
        let inner = block.inner(panel_area);
        frame.render_widget(block, panel_area);

        let [header_area, _, input_area, _, hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .areas(inner);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("➕ ", Style::default()),
                Span::styled(
                    "Добавить папку",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])),
            header_area,
        );

        self.draw_input(frame, input_area);
        frame.render_widget(
            Paragraph::new(vec![
                action_line("Enter", "Добавить"),
                action_line("Esc", "Отмена"),
            ]),
            hint_area,
        );

        self.draw_toast(frame, area);
    }

    fn handle_key(&mut self, key: KeyEvent, session: &mut Session) -> Action {
        match key.code {
            KeyCode::Enter | KeyCode::Esc if self.toast.is_some() => {
                self.toast = None;
                self.input.clear();

                Action::None
            }
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => {
                if self.add_folder(session) {
                    Action::Back
                } else {
                    Action::None
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
                Action::None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }
}

fn action_line(key: &str, label: &str) -> Line<'static> {
    let key = format!("[{key}]");
    let pad = 8usize.saturating_sub(key.chars().count());
    Line::from(vec![
        Span::styled(key, Style::default().fg(Color::Cyan)),
        Span::raw(" ".repeat(pad)),
        Span::raw(label.to_string()),
    ])
}

/// Largest `width`×`height` rectangle centered inside `area`.
fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
