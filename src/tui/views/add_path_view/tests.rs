use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(view: &mut AddPathView, session: &mut Session, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)), session);
    }
}

#[test]
fn enter_should_save_trimmed_existing_path_and_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let mut view = AddPathView::new();
    let mut session = Session::default();
    view.input = format!("  {}  ", path.display());

    let action = view.handle_key(key(KeyCode::Enter), &mut session);

    assert_eq!(action, Action::Back);
    assert_eq!(session.folders, vec![path]);
    assert!(view.toast.is_none());
}

#[test]
fn enter_should_ignore_blank_input() {
    let mut view = AddPathView::new();
    let mut session = Session::default();
    type_text(&mut view, &mut session, "   ");

    let action = view.handle_key(key(KeyCode::Enter), &mut session);

    assert_eq!(action, Action::None);
    assert!(session.folders.is_empty());
    assert!(view.toast.is_none());
}

#[test]
fn enter_should_reject_missing_path_with_toast_and_keep_input() {
    let missing = "definitely/not/a/real/path/rustle";
    let mut view = AddPathView::new();
    let mut session = Session::default();
    type_text(&mut view, &mut session, missing);

    let action = view.handle_key(key(KeyCode::Enter), &mut session);

    assert_eq!(action, Action::None);
    assert!(session.folders.is_empty());
    assert_eq!(view.toast.as_deref(), Some(missing));
    assert_eq!(view.input, missing);
}

#[test]
fn enter_should_dismiss_toast_without_adding_folder() {
    let mut view = AddPathView::new();
    let mut session = Session::default();
    type_text(&mut view, &mut session, "definitely/not/a/real/path/rustle");
    view.handle_key(key(KeyCode::Enter), &mut session);

    let action = view.handle_key(key(KeyCode::Enter), &mut session);

    assert_eq!(action, Action::None);
    assert!(view.toast.is_none());
    assert!(view.input.is_empty());
    assert!(session.folders.is_empty());
}

#[test]
fn esc_should_dismiss_toast_and_stay_on_view() {
    let mut view = AddPathView::new();
    let mut session = Session::default();
    type_text(&mut view, &mut session, "definitely/not/a/real/path/rustle");
    view.handle_key(key(KeyCode::Enter), &mut session);
    assert!(view.toast.is_some());

    let action = view.handle_key(key(KeyCode::Esc), &mut session);

    assert_eq!(action, Action::None);
    assert!(view.toast.is_none());
    assert!(view.input.is_empty());
    assert!(session.folders.is_empty());
}

#[test]
fn keys_other_than_enter_should_still_edit_input_while_toast_is_open() {
    let missing = "definitely/not/a/real/path/rustle";
    let mut view = AddPathView::new();
    let mut session = Session::default();
    type_text(&mut view, &mut session, missing);
    view.handle_key(key(KeyCode::Enter), &mut session);

    type_text(&mut view, &mut session, "xyz");
    view.handle_key(key(KeyCode::Backspace), &mut session);

    assert_eq!(view.input, format!("{missing}xy"));
    assert!(view.toast.is_some());
}

#[test]
fn enter_should_not_add_duplicate_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let mut view = AddPathView::new();
    let mut session = Session::default();

    view.input = path.display().to_string();
    assert_eq!(
        view.handle_key(key(KeyCode::Enter), &mut session),
        Action::Back
    );
    view.input = path.display().to_string();
    assert_eq!(
        view.handle_key(key(KeyCode::Enter), &mut session),
        Action::Back
    );

    assert_eq!(session.folders, vec![path]);
}

#[test]
fn backspace_should_remove_last_character() {
    let mut view = AddPathView::new();
    let mut session = Session::default();
    type_text(&mut view, &mut session, "abc");

    view.handle_key(key(KeyCode::Backspace), &mut session);

    assert_eq!(view.input, "ab");
}

#[test]
fn ctrl_chords_should_not_be_inserted() {
    let mut view = AddPathView::new();
    let mut session = Session::default();

    view.handle_key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &mut session,
    );

    assert!(view.input.is_empty());
}

#[test]
fn esc_should_request_back() {
    let mut view = AddPathView::new();
    let mut session = Session::default();

    assert_eq!(
        view.handle_key(key(KeyCode::Esc), &mut session),
        Action::Back
    );
}
