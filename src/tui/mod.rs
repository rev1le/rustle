//! Interactive TUI via `ratatui` + `crossterm`

mod app;
mod interact;
mod views;

pub use interact::run;

#[cfg(test)]
mod test_fixtures;
