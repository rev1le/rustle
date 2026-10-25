use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

use crate::tui::views::{Action, Screen};

#[cfg(test)]
mod tests;

pub struct MainView {
    /// Folders entered by the user, in insertion order.
    folders: Vec<PathBuf>,
    /// Current contents of the path input.
    input: String,
    /// Keeps the folder list scrolled to the newest entry.
    list_state: ListState,
    /// Path rejected by validation, shown as a centered toast until dismissed.
    toast: Option<String>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            folders: Vec::new(),
            input: String::new(),
            list_state: ListState::default(),
            toast: None,
        }
    }

    fn title_widget(&self) -> Paragraph<'_> {
        let title = Line::from(vec![
            Span::styled("📂 ", Style::default()),
            Span::styled(
                "Какие папки отслеживать?",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]);

        Paragraph::new(title).alignment(ratatui::layout::Alignment::Left)
    }

    /// Folders entered so far; empty state shows a hint instead of a list.
    fn folder_list_widget(&mut self) -> List<'static> {
        let items: Vec<ListItem<'static>> = if self.folders.is_empty() {
            self.list_state.select(None);
            vec![ListItem::new(Span::styled(
                " Пока пусто — добавьте путь ниже",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            self.list_state.select(Some(self.folders.len() - 1));
            self.folders
                .iter()
                .map(|path| {
                    ListItem::new(Line::from(vec![
                        Span::styled(" 📁 ", Style::default()),
                        Span::raw(path.display().to_string()),
                    ]))
                })
                .collect()
        };

        List::new(items).block(Block::bordered().title(format!(" Папки ({}) ", self.folders.len())))
    }

    /// Path input pinned above the bottom hint; scrolls to keep the cursor visible.
    fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let inner_width = area.width.saturating_sub(2).max(1) as usize;
        let cursor_col = self.input.chars().count();
        let scroll = cursor_col.saturating_sub(inner_width.saturating_sub(1)) as u16;

        let input = Paragraph::new(self.input.as_str())
            .block(Block::bordered().title(" Путь к папке "))
            .scroll((0, scroll));
        frame.render_widget(input, area);

        let x = area.x + 1 + (cursor_col - scroll as usize) as u16;
        let y = area.y + 1;
        frame.set_cursor_position((x, y));
    }

    fn hint_widget(&self) -> Paragraph<'static> {
        Paragraph::new(Span::styled(
            " Enter — добавить · Backspace — удалить символ · Esc — выход",
            Style::default().fg(Color::DarkGray),
        ))
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
                "Enter — закрыть",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::bordered().title(" Ошибка "));

        frame.render_widget(Clear, rect);
        frame.render_widget(toast, rect);
    }

    /// Validate and store the trimmed input as a folder, resetting the field.
    fn add_folder(&mut self) {
        let path = self.input.trim();
        if path.is_empty() {
            return;
        }

        let path = PathBuf::from(path);
        if !path.exists() {
            self.toast = Some(path.display().to_string());
            return;
        }

        if !self.folders.contains(&path) {
            self.folders.push(path);
        }
        self.input.clear();
    }
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

impl Screen for MainView {
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        frame.render_widget(Clear, area);

        let [title_area, list_area, input_area, hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(area);

        frame.render_widget(self.title_widget(), title_area);
        frame.render_stateful_widget(self.folder_list_widget(), list_area, &mut self.list_state);

        self.draw_input(frame, input_area);
        frame.render_widget(self.hint_widget(), hint_area);
        self.draw_toast(frame, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::Quit,
            KeyCode::Enter if self.toast.is_some() => {
                self.toast = None;
                self.input = String::new();

                Action::None
            }
            KeyCode::Enter => {
                self.add_folder();
                Action::None
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

    fn get_type(&self) -> super::View {
        super::View::Main
    }
}
