use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Command {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "capture")]
    Capture,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "shutdown")]
    Shutdown,
    #[serde(rename = "start_input_mode")]
    StartInputMode,
    #[serde(rename = "stop_input_mode")]
    StopInputMode,
    #[serde(rename = "capture_with_text")]
    CaptureWithText { content: String },
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "token")]
    Token { content: String },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "key_event")]
    KeyEvent {
        key: String,
        shift: bool,
        ctrl: bool,
    },
    #[serde(rename = "input_mode_state")]
    InputModeState { state: String },
}

impl Message {
    pub fn to_ndjson(&self) -> String {
        serde_json::to_string(self).expect("Message serialization should not fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_deserialize_ping() {
        let cmd: Command = serde_json::from_str(r#"{"type":"ping"}"#).unwrap();
        assert_eq!(cmd, Command::Ping);
    }

    #[test]
    fn test_command_deserialize_capture() {
        let cmd: Command = serde_json::from_str(r#"{"type":"capture"}"#).unwrap();
        assert_eq!(cmd, Command::Capture);
    }

    #[test]
    fn test_command_deserialize_shutdown() {
        let cmd: Command = serde_json::from_str(r#"{"type":"shutdown"}"#).unwrap();
        assert_eq!(cmd, Command::Shutdown);
    }

    #[test]
    fn test_command_deserialize_invalid_type() {
        let result = serde_json::from_str::<Command>(r#"{"type":"unknown"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_pong_to_ndjson() {
        let msg = Message::Pong;
        assert_eq!(msg.to_ndjson(), r#"{"type":"pong"}"#);
    }

    #[test]
    fn test_message_token_to_ndjson() {
        let msg = Message::Token {
            content: "hello".to_string(),
        };
        assert_eq!(msg.to_ndjson(), r#"{"type":"token","content":"hello"}"#);
    }

    #[test]
    fn test_message_done_to_ndjson() {
        let msg = Message::Done;
        assert_eq!(msg.to_ndjson(), r#"{"type":"done"}"#);
    }

    #[test]
    fn test_message_error_to_ndjson() {
        let msg = Message::Error {
            message: "fail".to_string(),
        };
        assert_eq!(msg.to_ndjson(), r#"{"type":"error","message":"fail"}"#);
    }

    #[test]
    fn test_command_start_input_mode() {
        let cmd: Command = serde_json::from_str(r#"{"type":"start_input_mode"}"#).unwrap();
        assert_eq!(cmd, Command::StartInputMode);
    }

    #[test]
    fn test_command_stop_input_mode() {
        let cmd: Command = serde_json::from_str(r#"{"type":"stop_input_mode"}"#).unwrap();
        assert_eq!(cmd, Command::StopInputMode);
    }

    #[test]
    fn test_command_capture_with_text() {
        let cmd: Command = serde_json::from_str(r#"{"type":"capture_with_text","content":"hello"}"#).unwrap();
        assert_eq!(cmd, Command::CaptureWithText { content: "hello".to_string() });
    }

    #[test]
    fn test_message_key_event() {
        let msg = Message::KeyEvent { key: "a".into(), shift: false, ctrl: false };
        assert_eq!(msg.to_ndjson(), r#"{"type":"key_event","key":"a","shift":false,"ctrl":false}"#);
    }

    #[test]
    fn test_message_input_mode_state_active() {
        let msg = Message::InputModeState { state: "active".into() };
        assert_eq!(msg.to_ndjson(), r#"{"type":"input_mode_state","state":"active"}"#);
    }

    #[test]
    fn test_message_input_mode_state_inactive() {
        let msg = Message::InputModeState { state: "inactive".into() };
        assert_eq!(msg.to_ndjson(), r#"{"type":"input_mode_state","state":"inactive"}"#);
    }

    #[test]
    fn test_message_input_mode_state_error() {
        let msg = Message::InputModeState { state: "error".into() };
        assert_eq!(msg.to_ndjson(), r#"{"type":"input_mode_state","state":"error"}"#);
    }
}
