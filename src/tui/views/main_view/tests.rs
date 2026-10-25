use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(view: &mut MainView, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
}

#[test]
fn enter_should_save_trimmed_existing_path_and_clear_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let mut view = MainView::new();
    view.input = format!("  {}  ", path.display());

    view.handle_key(key(KeyCode::Enter));

    assert_eq!(view.folders, vec![path]);
    assert!(view.input.is_empty());
    assert!(view.toast.is_none());
}

#[test]
fn enter_should_ignore_blank_input() {
    let mut view = MainView::new();
    type_text(&mut view, "   ");

    view.handle_key(key(KeyCode::Enter));

    assert!(view.folders.is_empty());
    assert!(view.toast.is_none());
}

#[test]
fn enter_should_reject_missing_path_with_toast_and_keep_input() {
    let missing = "definitely/not/a/real/path/rustle";
    let mut view = MainView::new();
    type_text(&mut view, missing);

    view.handle_key(key(KeyCode::Enter));

    assert!(view.folders.is_empty());
    assert_eq!(view.toast.as_deref(), Some(missing));
    assert_eq!(view.input, missing);
}

#[test]
fn enter_should_dismiss_toast_without_adding_folder() {
    let mut view = MainView::new();
    type_text(&mut view, "definitely/not/a/real/path/rustle");
    view.handle_key(key(KeyCode::Enter));

    view.handle_key(key(KeyCode::Enter));

    assert!(view.toast.is_none());
    assert!(view.input.is_empty());
    assert!(view.folders.is_empty());
}

#[test]
fn keys_other_than_enter_should_still_edit_input_while_toast_is_open() {
    let missing = "definitely/not/a/real/path/rustle";
    let mut view = MainView::new();
    type_text(&mut view, missing);
    view.handle_key(key(KeyCode::Enter));

    type_text(&mut view, "xyz");
    view.handle_key(key(KeyCode::Backspace));

    assert_eq!(view.input, format!("{missing}xy"));
    assert!(view.toast.is_some());
}

#[test]
fn enter_should_not_add_duplicate_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let mut view = MainView::new();

    view.input = path.display().to_string();
    view.handle_key(key(KeyCode::Enter));
    view.input = path.display().to_string();
    view.handle_key(key(KeyCode::Enter));

    assert_eq!(view.folders, vec![path]);
    assert!(view.toast.is_none());
}

#[test]
fn backspace_should_remove_last_character() {
    let mut view = MainView::new();
    type_text(&mut view, "abc");

    view.handle_key(key(KeyCode::Backspace));

    assert_eq!(view.input, "ab");
}

#[test]
fn ctrl_chords_should_not_be_inserted() {
    let mut view = MainView::new();

    view.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert!(view.input.is_empty());
}

#[test]
fn esc_should_quit() {
    let mut view = MainView::new();

    assert!(matches!(view.handle_key(key(KeyCode::Esc)), Action::Quit));
}
