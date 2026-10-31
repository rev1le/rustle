use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn session_with(paths: &[&str]) -> Session {
    Session {
        folders: paths.iter().map(PathBuf::from).collect(),
    }
}

#[test]
fn esc_should_request_back() {
    let mut view = MainView::new();
    let mut session = Session::default();

    assert_eq!(
        view.handle_key(key(KeyCode::Esc), &mut session),
        Action::Back
    );
}

#[test]
fn enter_should_open_scan_once_folders_exist() {
    let mut view = MainView::new();
    let mut session = session_with(&["/a"]);

    assert_eq!(
        view.handle_key(key(KeyCode::Enter), &mut session),
        Action::Navigate(View::Scan)
    );
}

#[test]
fn enter_without_folders_should_stay_on_main() {
    let mut view = MainView::new();
    let mut session = Session::default();

    assert_eq!(
        view.handle_key(key(KeyCode::Enter), &mut session),
        Action::None
    );
}

#[test]
fn a_should_open_add_path_view() {
    let mut view = MainView::new();
    let mut session = Session::default();

    assert_eq!(
        view.handle_key(key(KeyCode::Char('a')), &mut session),
        Action::Navigate(View::AddPath)
    );
}

#[test]
fn ctrl_a_should_not_open_add_path_view() {
    let mut view = MainView::new();
    let mut session = Session::default();

    let action = view.handle_key(
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        &mut session,
    );

    assert_eq!(action, Action::None);
}

#[test]
fn arrows_should_move_selection_and_clamp_at_bounds() {
    let mut view = MainView::new();
    let mut session = session_with(&["/a", "/b", "/c"]);

    view.handle_key(key(KeyCode::Up), &mut session);
    assert_eq!(view.list_state.selected(), Some(0), "top edge clamps");

    view.handle_key(key(KeyCode::Down), &mut session);
    assert_eq!(view.list_state.selected(), Some(1));

    view.handle_key(key(KeyCode::Down), &mut session);
    view.handle_key(key(KeyCode::Down), &mut session);
    assert_eq!(view.list_state.selected(), Some(2), "bottom edge clamps");

    view.handle_key(key(KeyCode::Up), &mut session);
    assert_eq!(view.list_state.selected(), Some(1));
}

#[test]
fn r_should_remove_selected_folder() {
    let mut view = MainView::new();
    let mut session = session_with(&["/a", "/b", "/c"]);
    view.handle_key(key(KeyCode::Down), &mut session);
    view.handle_key(key(KeyCode::Down), &mut session);

    assert_eq!(
        view.handle_key(key(KeyCode::Char('r')), &mut session),
        Action::None
    );

    assert_eq!(
        session.folders,
        vec![PathBuf::from("/a"), PathBuf::from("/b")]
    );
    assert_eq!(view.list_state.selected(), Some(1));
}

#[test]
fn r_should_keep_selection_in_bounds_after_removing_last() {
    let mut view = MainView::new();
    let mut session = session_with(&["/a", "/b"]);
    view.handle_key(key(KeyCode::Down), &mut session);

    view.handle_key(key(KeyCode::Char('r')), &mut session);

    assert_eq!(session.folders, vec![PathBuf::from("/a")]);
    assert_eq!(view.list_state.selected(), Some(0));
}

#[test]
fn r_should_clear_selection_after_removing_the_only_folder() {
    let mut view = MainView::new();
    let mut session = session_with(&["/a"]);

    view.handle_key(key(KeyCode::Char('r')), &mut session);

    assert!(session.folders.is_empty());
    assert_eq!(view.list_state.selected(), None);
}

#[test]
fn r_without_folders_should_do_nothing() {
    let mut view = MainView::new();
    let mut session = Session::default();

    assert_eq!(
        view.handle_key(key(KeyCode::Char('r')), &mut session),
        Action::None
    );
    assert!(session.folders.is_empty());
}

#[test]
fn ctrl_r_should_not_remove_a_folder() {
    let mut view = MainView::new();
    let mut session = session_with(&["/a"]);

    view.handle_key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &mut session,
    );

    assert_eq!(session.folders, vec![PathBuf::from("/a")]);
}
