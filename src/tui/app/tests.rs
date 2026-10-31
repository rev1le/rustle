use super::*;
use crossterm::event::{KeyCode, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn add_folder(app: &mut App, path: &std::path::Path) {
    assert_eq!(app.handle_key(key(KeyCode::Char('a'))), Action::None);
    assert!(matches!(app.active, Active::AddPath(_)));

    for c in path.display().to_string().chars() {
        app.handle_key(key(KeyCode::Char(c)));
    }
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
}

#[test]
fn esc_on_main_should_quit() {
    let mut app = App::new();

    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::Quit);
}

#[test]
fn esc_on_add_path_should_return_to_main() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('a')));
    assert!(matches!(app.active, Active::AddPath(_)));

    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::None);
    assert!(app.active.is_main());
}

#[test]
fn enter_after_folder_should_open_scan_and_esc_should_return() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new();
    add_folder(&mut app, dir.path());

    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
    assert!(matches!(app.active, Active::Scan(_)));

    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::None);
    assert!(app.active.is_main());
    assert_eq!(app.session.folders.len(), 1, "folders live in the session");
}

#[test]
fn enter_without_folders_should_stay_on_main() {
    let mut app = App::new();

    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::None);
    assert!(app.active.is_main());
}

#[test]
fn r_on_main_should_remove_the_selected_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new();
    add_folder(&mut app, dir.path());

    assert_eq!(app.handle_key(key(KeyCode::Char('r'))), Action::None);
    assert!(app.session.folders.is_empty());
}
