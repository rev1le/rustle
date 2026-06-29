//! Интерактивный TUI через `ratatui` + `crossterm` (Этап 4, опционально).
//!
//! "TUI через ratatui (дерево + heatmap)" и "Интерактивное удаление/архивация".

use std::path::Path;

use crate::models::Recommendation;

/// Запустить интерактивный TUI поверх уже собранных рекомендаций.
///
/// @todo: реализовать:
///   1. Инициализация терминала (`crossterm::terminal::enable_raw_mode`).
///   2. Главный цикл событий: навигация по дереву директорий, heatmap размеров.
///   3. Действия: отметить файлы к удалению/архивации, подтвердить.
///   4. Корректное восстановление терминала при выходе/панике.
pub fn run(root: &Path, recommendations: &[Recommendation]) -> anyhow::Result<()> {
    let _ = (root, recommendations);
    todo!("запустить TUI над {} рекомендациями", recommendations.len())
}
