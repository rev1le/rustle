//! Reveal paths in the OS file manager

use std::io;
use std::path::Path;
use std::process::Command;

/// Reveal a path in the OS file manager
///
/// Dirs open directly; files are highlighted in their folder
pub fn open_in_explorer(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        open_dir(path)
    } else {
        reveal_file(path)
    }
}

/// Copy text to the clipboard
///
/// `arboard` handles Unicode (paths may be non-ASCII), unlike piping to `clip.exe`
pub fn copy_to_clipboard(text: &str) -> Result<(), arboard::Error> {
    arboard::Clipboard::new()?.set_text(text)?;
    Ok(())
}

// --- Windows ---------------------------------------------------------------

#[cfg(windows)]
fn open_dir(path: &Path) -> io::Result<()> {
    Command::new("explorer.exe").arg(path).spawn().map(drop)
}

#[cfg(windows)]
fn reveal_file(path: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    // raw_arg avoids re-escaping the quoted path after /select,; .arg() breaks on spaces
    Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map(drop)
}

// --- macOS -----------------------------------------------------------------

#[cfg(target_os = "macos")]
fn open_dir(path: &Path) -> io::Result<()> {
    Command::new("open").arg(path).spawn().map(drop)
}

#[cfg(target_os = "macos")]
fn reveal_file(path: &Path) -> io::Result<()> {
    // -R reveals in Finder
    Command::new("open").arg("-R").arg(path).spawn().map(drop)
}

// --- Linux / other Unix ----------------------------------------------------

#[cfg(not(any(windows, target_os = "macos")))]
fn open_dir(path: &Path) -> io::Result<()> {
    Command::new("xdg-open").arg(path).spawn().map(drop)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn reveal_file(path: &Path) -> io::Result<()> {
    // no universal reveal on Linux — open the parent dir
    let parent = path.parent().unwrap_or(path);
    Command::new("xdg-open").arg(parent).spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "touches the real clipboard; run manually"]
    fn copy_to_clipboard_roundtrips_text() {
        // cargo test copy_to_clipboard -- --ignored
        let text = "rustle-clipboard-test-path/file.txt";
        copy_to_clipboard(text).expect("should write to the clipboard");

        let readback = arboard::Clipboard::new()
            .and_then(|mut cb| cb.get_text())
            .expect("should read the clipboard");
        assert_eq!(readback, text);
    }
}
