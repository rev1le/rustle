//! Отрисовка панелей TUI (заголовок, каталоги, файлы, футер).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::app::{App, Focus, MARQUEE_PAUSE_TICKS};
use crate::models::human_readable_size;

/// Ширина текстовой полоски размера (в символах).
const BAR_WIDTH: usize = 20;

/// Отрисовать всё приложение: заголовок, две панели и футер.
pub(super) fn draw(
    app: &App,
    frame: &mut Frame,
    dir_state: &mut ListState,
    file_state: &mut ListState,
) {
    let [header, main, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .areas(frame.area());

    draw_header(app, frame, header);

    let [dirs, files] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(main);
    draw_directories(app, frame, dirs, dir_state);
    draw_files(app, frame, files, file_state);

    draw_footer(frame, footer);
}

fn draw_header(app: &App, frame: &mut Frame, area: Rect) {
    let title = format!(" rustle — {} ", app.root.display());
    let stats = format!(
        "файлов: {} | пропущено: {} | всего: {}",
        app.total_files,
        app.skipped,
        human_readable_size(app.total_size)
    );
    let paragraph =
        Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(paragraph, area);
}

fn draw_directories(app: &App, frame: &mut Frame, area: Rect, state: &mut ListState) {
    let max_size = app.directories.first().map_or(0, |d| d.total_size);
    let selected = state.selected();
    // Доступная ширина имени каталога: [полоса:21] [размер:11] + рамка(2) + маркер(2)
    let name_width = name_width_for(area, 21 + 11);

    let items: Vec<ListItem<'_>> = app
        .directories
        .iter()
        .enumerate()
        .map(|(i, dir)| {
            let rel = dir.path.strip_prefix(&app.root).unwrap_or(&dir.path);
            let ratio = if max_size == 0 {
                0.0
            } else {
                dir.total_size as f64 / max_size as f64
            };
            let filled = (ratio * BAR_WIDTH as f64).round() as usize;
            // Unicode-эскейпы намеренно: сырые глифы █/░ искажались при правках.
            let bar = format!(
                "{}{}",
                "\u{2588}".repeat(filled),
                "\u{2591}".repeat(BAR_WIDTH - filled)
            );

            let label = display_label(
                &rel.to_string_lossy(),
                name_width,
                app.focus == Focus::Directories && Some(i) == selected,
                app.scroll_phase,
            );

            ListItem::new(Line::from(vec![
                Span::styled(format!("{bar} "), Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:>10} ", human_readable_size(dir.total_size))),
                Span::raw(label),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(app.focus, Focus::Directories))
                .title(panel_title(
                    "Каталоги по размеру",
                    app.focus,
                    Focus::Directories,
                )),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, state);
}

fn draw_files(app: &App, frame: &mut Frame, area: Rect, state: &mut ListState) {
    let selected = state.selected();
    // Доступная ширина имени файла: [размер:11] [расш:9] + рамка(2) + маркер(2)
    let name_width = name_width_for(area, 11 + 9);

    let items: Vec<ListItem<'_>> = app
        .top_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let rel = file.path.strip_prefix(&app.root).unwrap_or(&file.path);
            let ext = file.ext.as_deref().unwrap_or("-");
            let label = display_label(
                &rel.to_string_lossy(),
                name_width,
                app.focus == Focus::Files && Some(i) == selected,
                app.scroll_phase,
            );
            ListItem::new(Line::from(vec![
                Span::raw(format!("{:>10} ", human_readable_size(file.size))),
                Span::styled(format!("[{ext:^6}] "), Style::default().fg(Color::Yellow)),
                Span::raw(label),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border_style(app.focus, Focus::Files))
                .title(panel_title("Самые большие файлы", app.focus, Focus::Files)),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, state);
}

/// Доступная ширина текста строки в панели.
///
/// `area.width` включает рамки (2), маркер выделения занимает ещё 2.
/// `prefix_len` — суммарная ширина предшествующих полей строки (полоса+размер
/// для каталогов; размер+расширение для файлов).
fn name_width_for(area: Rect, prefix_len: usize) -> usize {
    (area.width as usize).saturating_sub(2 + 2 + prefix_len)
}

/// Текст метки строки с учётом переполнения.
///
/// - Выделенная строка активной панели (`active_and_selected`) прокручивается
///   (бегущая строка) по фазе `phase`.
/// - Остальные строки обрезаются по ширине с маркером `…`.
fn display_label(text: &str, width: usize, active_and_selected: bool, phase: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if active_and_selected {
        marquee_window(text, width, phase)
    } else {
        truncate_to(text, width)
    }
}

/// Окно бегущей строки: прокрутка с паузами в начале и в конце.
///
/// Ширина указывается в символах (Unicode). Символы с нулевой/двойной шириной
/// в путях встречаются редко, поэтому char-счёт считается достаточным.
fn marquee_window(text: &str, width: usize, phase: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= width {
        return text.to_string();
    }
    let scroll_range = len - width;
    let period = MARQUEE_PAUSE_TICKS * 2 + scroll_range;
    let p = phase % period;
    let offset = if p < MARQUEE_PAUSE_TICKS {
        // пауза в начале — показываем старт строки
        0
    } else if p < MARQUEE_PAUSE_TICKS + scroll_range {
        // прокрутка
        p - MARQUEE_PAUSE_TICKS
    } else {
        // пауза в конце — показываем хвост строки
        scroll_range
    };
    chars[offset..offset + width].iter().collect()
}

/// Обрезать текст до `width` символов, добавляя `…`, если он не влезает.
fn truncate_to(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        text.to_string()
    } else if width == 0 {
        String::new()
    } else {
        let mut s: String = chars[..width - 1].iter().collect();
        s.push('…');
        s
    }
}

/// Стиль рамки панели: активная подсвечена, неактивная приглушена.
fn panel_border_style(focus: Focus, panel: Focus) -> Style {
    if focus == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Заголовок панели: у активной добавлен маркер `◀`.
fn panel_title(title: &str, focus: Focus, panel: Focus) -> String {
    if focus == panel {
        format!(" {title} ◀ ")
    } else {
        format!(" {title} ")
    }
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let help =
        " ↑/↓ или j/k — навигация   ←/→ или h/l — переключение панелей   q/Esc/Ctrl-C — выход ";
    let paragraph = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_fixtures::{file, scan_of};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    #[test]
    fn draw_renders_layout_without_panic() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![
                file("/root/sub/big.bin", 4096, Some("bin")),
                file("/root/sub/small.txt", 100, Some("txt")),
            ]),
        );

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut dir_state = ListState::default();
        let mut file_state = ListState::default();
        dir_state.select(Some(0));
        file_state.select(Some(0));

        terminal
            .draw(|frame| draw(&app, frame, &mut dir_state, &mut file_state))
            .unwrap();
    }

    #[test]
    fn draw_contains_panel_titles_and_bar_glyphs() {
        let app = App::new(
            PathBuf::from("/root"),
            scan_of(vec![file("/root/sub/big.bin", 4096, Some("bin"))]),
        );

        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut dir_state = ListState::default();
        let mut file_state = ListState::default();
        dir_state.select(Some(0));
        file_state.select(Some(0));
        terminal
            .draw(|frame| draw(&app, frame, &mut dir_state, &mut file_state))
            .unwrap();

        let buf = terminal.backend().buffer();
        let content: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| buf[(x, y)].symbol().to_string()))
            .collect();

        assert!(
            content.contains("Каталоги"),
            "должен быть заголовок панели каталогов"
        );
        assert!(content.contains("файлы"), "должна быть сводка по файлам");
        // Полоска размера содержит заполнители (█) или фон (░).
        assert!(content.contains('█') || content.contains('░'));
    }

    #[test]
    fn truncate_to_keeps_short_text_intact() {
        assert_eq!(truncate_to("abc", 5), "abc");
        assert_eq!(truncate_to("abc", 3), "abc");
    }

    #[test]
    fn truncate_to_appends_ellipsis_on_overflow() {
        assert_eq!(truncate_to("abcdef", 4), "abc…");
        assert_eq!(truncate_to("abcdef", 1), "…");
    }

    #[test]
    fn marquee_window_shows_full_text_when_it_fits() {
        assert_eq!(marquee_window("abc", 5, 0), "abc");
        assert_eq!(marquee_window("abc", 3, 100), "abc");
    }

    #[test]
    fn marquee_window_pauses_at_start_then_scrolls() {
        // текст длиной 6, окно 3 → scroll_range = 3, period = 4*2 + 3 = 11
        // фазы 0..4 — пауза в начале (видим «abc»)
        for phase in 0..MARQUEE_PAUSE_TICKS {
            assert_eq!(marquee_window("abcdef", 3, phase), "abc");
        }
        // фаза MARQUEE_PAUSE_TICKS — начало прокрутки, сдвиг 0 (ещё «abc»)
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS), "abc");
        // следующий тик → сдвиг 1 → «bcd»
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS + 1), "bcd");
        // фаза +3 → сдвиг 3 → «def» (хвост)
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS + 3), "def");
    }

    #[test]
    fn marquee_window_pauses_at_end_then_loops() {
        // scroll_range = 3, фазы 7..11 — пауза в конце (видим хвост «def»)
        assert_eq!(marquee_window("abcdef", 3, MARQUEE_PAUSE_TICKS + 4), "def");
        // period = 11 → phase 11 == phase 0 (снова начало)
        assert_eq!(marquee_window("abcdef", 3, 11), "abc");
    }

    #[test]
    fn display_label_marquees_only_for_active_selected() {
        // активная выделенная строка — прокрутка
        assert_eq!(display_label("abcdef", 3, true, 0), "abc");
        // неактивная/невыделенная — обрезка с …
        assert_eq!(display_label("abcdef", 3, false, 0), "ab…");
    }
}
