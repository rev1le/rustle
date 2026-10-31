use std::path::PathBuf;

/// Domain state shared by every screen.
///
/// Views keep only their transient UI state (input, selection, toast);
/// everything another screen may need lives here.
#[derive(Default)]
pub(super) struct Session {
    /// Folders entered by the user, in insertion order.
    pub(super) folders: Vec<PathBuf>,
}
