use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState, Padding, Paragraph},
};

use crate::tui::{
    session::Session,
    views::{Action, Screen, View},
};

#[cfg(test)]
mod tests;

const HOME_LABEL: &str = if cfg!(target_os = "linux") {
    "(Linux home)"
} else if cfg!(target_os = "macos") {
    "(macOS home)"
} else if cfg!(windows) {
    "(Windows home)"
} else {
    "(home)"
};

struct DiskInfo {
    total: u64,
    free: u64,
    fs: String,
}

pub struct MainView {
    /// Selected scan target; arrows move it, `r` removes it.
    list_state: ListState,
    /// Per-folder `df` snapshot, keyed by path; `None` when unavailable.
    disk: HashMap<PathBuf, Option<DiskInfo>>,
}

impl MainView {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            disk: HashMap::new(),
        }
    }

    fn title_widget(&self) -> Paragraph<'static> {
        Paragraph::new(Line::from(vec![
            Span::styled("🦀 ", Style::default()),
            Span::styled(
                format!("rustle v{}", env!("CARGO_PKG_VERSION")),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " — Select Scan Targets",
                Style::default().fg(Color::DarkGray),
            ),
        ]))
    }

    fn draw_panel(&mut self, frame: &mut Frame, area: Rect, session: &Session) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(2));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [
            header_area,
            _,
            list_area,
            divider_area,
            _,
            actions_area,
            _,
            summary_area,
            _,
            nav_area,
        ] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .areas(inner);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("📂 ", Style::default()),
                Span::styled(
                    "Какие папки отслеживать?",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])),
            header_area,
        );

        self.ensure_selection(session.folders.len());
        frame.render_stateful_widget(
            self.folder_list_widget(&session.folders, inner.width),
            list_area,
            &mut self.list_state,
        );
        frame.render_widget(divider_widget(inner.width), divider_area);
        self.draw_actions(frame, actions_area);
        self.draw_summary(frame, summary_area, &session.folders);
        frame.render_widget(
            Paragraph::new(vec![
                action_line("Enter", "▶ Начать сканирование"),
                action_line("Esc", "← Назад к выбору профиля"),
            ]),
            nav_area,
        );
    }

    fn folder_list_widget(&self, folders: &[PathBuf], width: u16) -> List<'static> {
        if folders.is_empty() {
            return List::new(vec![ListItem::new(Line::from(Span::styled(
                "Пока пусто — нажмите [a], чтобы добавить путь",
                Style::default().fg(Color::DarkGray),
            )))]);
        }

        let width = width as usize;
        let home = dirs::home_dir();
        let items = folders
            .iter()
            .map(|path| {
                let is_home = home.as_deref() == Some(path.as_path());
                let icon = if is_home { "🏠" } else { "📁" };
                let label = if is_home {
                    HOME_LABEL
                } else {
                    "(добавлено)"
                };
                let info = self.disk.get(path).and_then(Option::as_ref);
                ListItem::new(vec![
                    folder_title_line(path, icon, label, width),
                    folder_stats_line(info),
                    Line::default(),
                ])
            })
            .collect::<Vec<_>>();

        List::new(items).highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
    }

    fn draw_actions(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new(vec![
                action_line("a", "➕ Добавить папку"),
                action_line("r", "🗑️ Убрать выбранную"),
                action_line("↑/↓", "Выбрать папку"),
            ]),
            area,
        );
    }

    fn draw_summary(&self, frame: &mut Frame, area: Rect, folders: &[PathBuf]) {
        let mut seen = HashSet::new();
        let total: u64 = folders
            .iter()
            .filter_map(|path| self.disk.get(path).and_then(Option::as_ref))
            .filter(|info| seen.insert((info.total, info.free)))
            .map(|info| info.total)
            .sum();

        let mut spans = vec![
            Span::styled("Выбрано: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} {}", folders.len(), folder_word(folders.len()))),
        ];
        if total > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("~{} для анализа", format_size(total)),
                Style::default().fg(Color::Yellow),
            ));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Keep the selection valid when the folder list changes.
    fn ensure_selection(&mut self, len: usize) {
        if len == 0 {
            self.list_state.select(None);
        } else if self.list_state.selected().is_none_or(|index| index >= len) {
            self.list_state.select(Some(0));
        }
    }

    fn move_selection(&mut self, len: usize, delta: isize) {
        if len == 0 {
            self.list_state.select(None);
            return;
        }

        let current = self.list_state.selected().unwrap_or(0);
        let next = (current as isize + delta).clamp(0, len as isize - 1) as usize;
        self.list_state.select(Some(next));
    }

    fn remove_selected(&mut self, session: &mut Session) {
        let Some(index) = self.list_state.selected() else {
            return;
        };
        if index >= session.folders.len() {
            return;
        }

        let path = session.folders.remove(index);
        self.disk.remove(&path);

        let len = session.folders.len();
        self.list_state.select(if len == 0 {
            None
        } else {
            Some(index.min(len - 1))
        });
    }

    /// Snapshot disk stats once per folder; refreshed only when the view is recreated.
    fn cache_disk_info(&mut self, folders: &[PathBuf]) {
        for path in folders {
            if !self.disk.contains_key(path) {
                self.disk.insert(path.clone(), disk_info(path));
            }
        }
    }
}

fn folder_title_line(path: &Path, icon: &str, label: &str, width: usize) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            "[x] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{icon} ")),
        Span::raw(path.display().to_string()),
    ];
    let label_width = label.chars().count() + 1;
    let pad = width.saturating_sub(Line::from(spans.clone()).width() + label_width);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(
        format!("{label} "),
        Style::default().fg(Color::DarkGray),
    ));
    Line::from(spans)
}

fn folder_stats_line(info: Option<&DiskInfo>) -> Line<'static> {
    let Some(info) = info else {
        // shortcut: `df` is the only probe today; other platforms show a placeholder.
        return Line::from(Span::styled(
            "    └─ данные диска недоступны",
            Style::default().fg(Color::DarkGray),
        ));
    };

    let sep = || Span::styled(" │ ", Style::default().fg(Color::DarkGray));
    Line::from(vec![
        Span::styled("    └─ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} total", format_size(info.total)),
            Style::default().fg(Color::White),
        ),
        sep(),
        Span::styled(
            format!("{} free", format_size(info.free)),
            Style::default().fg(Color::White),
        ),
        sep(),
        Span::styled(info.fs.clone(), Style::default().fg(Color::DarkGray)),
    ])
}

fn divider_widget(width: u16) -> Paragraph<'static> {
    Paragraph::new(Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(Color::DarkGray),
    ))
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

fn folder_word(count: usize) -> &'static str {
    if matches!(count % 100, 11..=14) {
        "папок"
    } else {
        match count % 10 {
            1 => "папка",
            2..=4 => "папки",
            _ => "папок",
        }
    }
}

/// Binary units labelled the way the design does (GB/TB).
fn format_size(bytes: u64) -> String {
    let gb = bytes as f64 / 1024.0_f64.powi(3);
    if gb >= 1024.0 {
        format!("{:.1} TB", gb / 1024.0)
    } else {
        format!("{:.1} GB", gb)
    }
}

#[cfg(target_os = "linux")]
fn disk_info(path: &Path) -> Option<DiskInfo> {
    use std::process::Command;

    let output = Command::new("df")
        .args(["-k", "--output=fstype,size,avail"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut cols = stdout.lines().nth(1)?.split_whitespace();
    Some(DiskInfo {
        fs: cols.next()?.to_string(),
        total: cols.next()?.parse::<u64>().ok()? * 1024,
        free: cols.next()?.parse::<u64>().ok()? * 1024,
    })
}

#[cfg(all(unix, not(target_os = "linux")))]
fn disk_info(path: &Path) -> Option<DiskInfo> {
    use std::process::Command;

    let output = Command::new("df").args(["-Pk"]).arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let cols: Vec<&str> = stdout.lines().nth(1)?.split_whitespace().collect();
    Some(DiskInfo {
        fs: "unknown".to_string(),
        total: cols.get(1)?.parse::<u64>().ok()? * 1024,
        free: cols.get(3)?.parse::<u64>().ok()? * 1024,
    })
}

#[cfg(not(unix))]
fn disk_info(_path: &Path) -> Option<DiskInfo> {
    // shortcut: no Windows probe yet; add one when volume stats become a real feature.
    None
}

impl Screen for MainView {
    fn draw(&mut self, frame: &mut Frame, session: &Session) {
        let area = frame.area();
        frame.render_widget(Clear, area);

        self.cache_disk_info(&session.folders);

        let outer = Block::bordered().border_type(BorderType::Rounded);
        let content = outer.inner(area).inner(Margin::new(2, 0));
        frame.render_widget(outer, area);

        let [title_area, _, panel_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas(content);

        frame.render_widget(self.title_widget(), title_area);
        self.draw_panel(frame, panel_area, session);
    }

    fn handle_key(&mut self, key: KeyEvent, session: &mut Session) -> Action {
        self.ensure_selection(session.folders.len());
        let plain = !key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter if !session.folders.is_empty() => Action::Navigate(View::Scan),
            KeyCode::Up => {
                self.move_selection(session.folders.len(), -1);
                Action::None
            }
            KeyCode::Down => {
                self.move_selection(session.folders.len(), 1);
                Action::None
            }
            KeyCode::Char('a') if plain => Action::Navigate(View::AddPath),
            KeyCode::Char('r') if plain => {
                self.remove_selected(session);
                Action::None
            }
            _ => Action::None,
        }
    }
}
