use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn valid_config_json() -> String {
    r#"{
        "hotkey": "CommandOrControl+Shift+Space",
        "inputHotkey": "Ctrl+Shift+K",
        "model": "gpt-4o",
        "openaiApiKey": "sk-test",
        "anthropicApiKey": "",
        "profile": "interview",
        "overlayOpacity": 0.85,
        "overlayWidth": 320,
        "overlayHeight": 600,
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

#[test]
fn test_start_input_mode() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    let expected = if cfg!(target_os = "windows") {
        r#"{"type":"input_mode_state","state":"active"}"#
    } else {
        r#"{"type":"input_mode_state","state":"error"}"#
    };
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"start_input_mode"}"#)
        .assert()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn test_stop_input_mode() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"stop_input_mode"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"{"type":"input_mode_state","state":"inactive"}"#));
}

#[test]
fn test_capture_with_text_empty() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"capture_with_text","content":"   "}{"type":"shutdown"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"token"#).not());
}
