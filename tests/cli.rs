//! Integration tests

use std::process::Command;

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
        "expected the package name in --version: {stdout}"
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

    for cmd in ["classify", "tui", "report"] {
        assert!(
            stdout.contains(cmd),
            "subcommand `{cmd}` missing from --help:\n{stdout}"
        );
    }
}
