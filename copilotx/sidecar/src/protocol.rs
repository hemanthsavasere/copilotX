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
}
