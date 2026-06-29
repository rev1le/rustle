//! Интеграционные тесты: проверяют CLI-интерфейс бинарника как есть.
//!
//! Запускают собранный `rustle` через `std::process::Command` (без внешних
//! зависимостей для тестов) и проверяют, что аргументы/вывод работают.

use std::process::Command;

/// Путь к собранному бинарнику (cargo подставляет его в integration-тестах).
const BIN: &str = env!("CARGO_BIN_EXE_rustle");

#[test]
fn cli_version_flag_works() {
    let output = Command::new(BIN).arg("--version").output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rustle"),
        "ожидается имя пакета в --version: {stdout}"
    );
}

#[test]
fn cli_help_lists_all_subcommands() {
    let output = Command::new(BIN).arg("--help").output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Все подкоманды из дорожной карты должны быть видны в --help.
    for cmd in ["scan", "classify", "tui", "report"] {
        assert!(
            stdout.contains(cmd),
            "в --help нет подкоманды `{cmd}`:\n{stdout}"
        );
    }
}

#[test]
fn cli_scan_help_describes_mvp_flags() {
    let output = Command::new(BIN).args(["scan", "--help"]).output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--top"),
        "у scan должен быть флаг --top:\n{stdout}"
    );
    assert!(
        stdout.contains("--depth"),
        "у scan должен быть флаг --depth:\n{stdout}"
    );
}
