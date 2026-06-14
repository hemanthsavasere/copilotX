use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn valid_config_json() -> String {
    r#"{
        "hotkey": "CommandOrControl+Shift+Space",
        "model": "gpt-4o",
        "openaiApiKey": "sk-test",
        "anthropicApiKey": "",
        "profile": "interview",
        "overlayOpacity": 0.85,
        "overlayWidth": 320,
        "overlayPosition": "right"
    }"#
    .to_string()
}

#[test]
fn test_ping_pong() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"ping"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"{"type":"pong"}"#));
}

#[test]
fn test_unknown_command() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"unknown_command"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"Parse error"#));
}

#[test]
fn test_shutdown() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"shutdown"}"#)
        .assert()
        .success();
}
