//! Открытие путей во внешнем файловом менеджере ОС.
//!
//! Кроссплатформенный запуск «показать в проводнике»:
//! - **Windows** — `explorer.exe` (для файла — с ключом `/select`,`);
//! - **macOS** — `open -R` (reveal в Finder);
//! - **Linux** — `xdg-open` родительского каталога (универсального способа
//!   выделить файл в произвольном DE нет, поэтому открываем папку).
//!
//! Процесс запускается отсоединённым (`spawn`) и не блокирует TUI.

use std::io;
use std::path::Path;
use std::process::Command;

/// Показать путь в файловом менеджере ОС.
///
/// Каталоги открываются напрямую, файлы — выделяются (reveal) в их каталоге.
pub fn open_in_explorer(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        open_dir(path)
    } else {
        reveal_file(path)
    }
}

/// Скопировать текст в системный буфер обмена.
///
/// Используется крейт `arboard`, т.к. он корректно работает с Unicode
/// (пути могут содержать кириллицу), в отличие от пайпа в `clip.exe`.
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
    // raw_arg нужен, чтобы Rust не переэкранировал уже закавыченный путь
    // после `/select,` (обычный `.arg()` ломается на путях с пробелами).
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
    // -R — reveal в Finder (выделить файл в его каталоге).
    Command::new("open").arg("-R").arg(path).spawn().map(drop)
}

// --- Linux / прочие Unix ---------------------------------------------------

#[cfg(not(any(windows, target_os = "macos")))]
fn open_dir(path: &Path) -> io::Result<()> {
    Command::new("xdg-open").arg(path).spawn().map(drop)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn reveal_file(path: &Path) -> io::Result<()> {
    // Универсального «выделить файл» в Linux нет — открываем родительский каталог.
    let parent = path.parent().unwrap_or(path);
    Command::new("xdg-open").arg(parent).spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "обращается к реальному системному буферу обмена; запускать вручную"]
    fn copy_to_clipboard_roundtrips_text() {
        // cargo test copy_to_clipboard -- --ignored
        let text = "rustle-clipboard-test-путь/файл.txt";
        copy_to_clipboard(text).expect("должен записать в буфер");

        let readback = arboard::Clipboard::new()
            .and_then(|mut cb| cb.get_text())
            .expect("должен прочитать буфер");
        assert_eq!(readback, text);
    }
}
