# Stealth Text Input Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a stealth text input bar to the CopilotX overlay that captures keystrokes via a Win32 low-level keyboard hook in the Rust sidecar, forwards them to Electron, and renders them without the overlay ever receiving OS-level keyboard focus.

**Architecture:** A writer thread owns stdout exclusively (Option C). The keyboard hook runs on a dedicated thread with `WH_KEYBOARD_LL`, sending key events through the same `mpsc::channel`. A 2.5-second inactivity timeout auto-unregisters the hook as a safety net. The overlay stays `focusable: false` at all times.

**Tech Stack:** Rust (sidecar with `windows` crate, `mpsc::channel`, `WH_KEYBOARD_LL`), TypeScript/Electron (IPC, preload bridge), React/TSX (TextInputBar component, input mode state)

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `copilotx/sidecar/src/keyboard.rs` | Keyboard hook module: `start_keyboard_hook`, `stop_keyboard_hook`, `HookHandle`, `map_key_event`, `is_input_key`. Windows impl behind `#[cfg(target_os = "windows")]`, stubs behind `#[cfg(not)]`. |
| `copilotx/src/renderer/src/TextInputBar.tsx` | React component rendering typed text with blinking cursor at bottom of overlay |

### Modified Files
| File | Responsibility Change |
|------|----------------------|
| `copilotx/sidecar/src/protocol.rs` | Add `StartInputMode`, `StopInputMode`, `CaptureWithText` to `Command`; add `KeyEvent`, `InputModeState` to `Message` |
| `copilotx/sidecar/src/main.rs` | Replace `print_message`/`print_error` with writer thread + `mpsc::channel`; add `mod keyboard`; handle new commands |
| `copilotx/sidecar/src/llm.rs` | Remove duplicate `print_message`; accept `tx: &Sender<Message>` and `user_text: Option<&str>` params |
| `copilotx/sidecar/src/config.rs` | Add `input_hotkey` and `overlay_height` fields; update test helper |
| `copilotx/sidecar/Cargo.toml` | Add target-specific `windows` crate dependency |
| `copilotx/sidecar/tests/integration.rs` | Update config JSON; add `start_input_mode`/`stop_input_mode`/`capture_with_text` integration tests |
| `copilotx/config/config.json` | Add `inputHotkey` field |
| `copilotx/config/schemas/config.schema.json` | Add `inputHotkey` property |
| `copilotx/src/main/config.ts` | Add `inputHotkey` to `AppConfig` interface and validation |
| `copilotx/src/main/hotkey.ts` | Add `isInputMode` flag, `registerInputHotkey`, `setInputModeActive`, `setInputModeInactive`, `isInInputMode` |
| `copilotx/src/main/ipc.ts` | Extend `SidecarMessage` with new types; add `sendStartInputMode`, `sendStopInputMode`, `sendCaptureWithText`; widen `writeSidecar` signature |
| `copilotx/src/main/index.ts` | Handle `key_event` and `input_mode_state` messages; register input hotkey; add IPC handlers for renderer |
| `copilotx/src/preload/index.ts` | Add `onKeyEvent`, `onInputModeState`, `sendTextInput`, `stopInputMode` |
| `copilotx/src/preload/index.d.ts` | Add new methods to `Window.api` interface |
| `copilotx/src/renderer/src/App.tsx` | Add input mode state, key event listener, TextInputBar rendering, dimmed answer panel |
| `copilotx/src/renderer/src/AnswerPanel.tsx` | Add `dimmed` prop |
| `copilotx/src/renderer/src/styles.css` | Add input mode styles (dimmed, text-input-bar, cursor blink, status dot amber) |
| `copilotx/src/main/__tests__/config.test.ts` | Add `inputHotkey` to valid config fixture and validation test |
| `copilotx/src/main/__tests__/ipc.test.ts` | Add tests for new `SidecarMessage` types |
| `copilotx/src/main/__tests__/hotkey.test.ts` | Add tests for input mode state guards |

---

## Phase 1: Sidecar Protocol Extensions & Writer Thread

### Task 1: Extend protocol types with new Command and Message variants

**Files:**
- Modify: `copilotx/sidecar/src/protocol.rs`

- [ ] **Step 1: Write failing tests for new Command variants**

Add these tests inside the `#[cfg(test)] mod tests` block in `copilotx/sidecar/src/protocol.rs`, after the existing `test_message_error_to_ndjson` test (after line 89):

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd copilotx/sidecar && cargo test -- protocol::tests`
Expected: FAIL — `StartInputMode`, `StopInputMode`, `CaptureWithText`, `KeyEvent`, `InputModeState` variants not found

- [ ] **Step 3: Add new Command and Message variants**

Replace the `Command` enum (lines 3-14) with:

```rust
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
```

Replace the `Message` enum (lines 16-27) with:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd copilotx/sidecar && cargo test -- protocol::tests`
Expected: PASS (all 14 protocol tests)

- [ ] **Step 5: Commit**

```bash
cd copilotx
git add copilotx/sidecar/src/protocol.rs
git commit -m "feat(sidecar): add StartInputMode, StopInputMode, CaptureWithText commands and KeyEvent, InputModeState messages to protocol"
```

---

### Task 2: Add writer thread and send_error helper to main.rs

**Files:**
- Modify: `copilotx/sidecar/src/main.rs`

- [ ] **Step 1: Replace print_message/print_error with writer thread and send_error**

Replace lines 8-23 of `copilotx/sidecar/src/main.rs` (the `use` block and the two functions) with:

```rust
use protocol::{Command, Message};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

fn send_error(tx: &mpsc::Sender<Message>, message: &str) {
    tx.send(Message::Error { message: message.to_string() }).ok();
}
```

- [ ] **Step 2: Add writer thread spawn and replace all print_message/print_error calls in main()**

Replace the entire `main()` function body (lines 25-113) with:

```rust
#[tokio::main]
async fn main() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    let validation_errors = config.validate();
    if !validation_errors.is_empty() {
        eprintln!("Config validation: {}", validation_errors.join("; "));
        std::process::exit(1);
    }

    let (tx, rx) = mpsc::channel::<Message>();
    std::thread::spawn(move || {
        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        for msg in rx {
            writeln!(writer, "{}", msg.to_ndjson()).ok();
            writer.flush().ok();
        }
    });

    let is_processing = Arc::new(AtomicBool::new(false));
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cmd: Command = match serde_json::from_str(trimmed) {
            Ok(c) => c,
            Err(e) => {
                send_error(&tx, &format!("Parse error: {}", e));
                continue;
            }
        };

        match cmd {
            Command::Ping => {
                tx.send(Message::Pong).ok();
            }
            Command::Capture => {
                if is_processing.load(Ordering::SeqCst) {
                    send_error(&tx, "Already processing");
                    continue;
                }
                is_processing.store(true, Ordering::SeqCst);

                let system_prompt = match profiles::get_system_prompt(&config.profile) {
                    Some(p) => p,
                    None => {
                        send_error(&tx, &format!("Unknown profile: {}", config.profile));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let image_b64 = match capture::capture_primary_monitor() {
                    Ok(img) => img,
                    Err(e) => {
                        send_error(&tx, &format!("Capture failed: {}", e));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let result = match config.model.as_str() {
                    "gpt-4o" => {
                        llm::stream_openai(&tx, &config.openai_api_key, &system_prompt, &image_b64, None).await
                    }
                    "claude" | "claude-sonnet" => {
                        llm::stream_anthropic(&tx, &config.anthropic_api_key, &system_prompt, &image_b64, None).await
                    }
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    send_error(&tx, &format!("LLM error: {}", e));
                }

                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Stop => {
                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Shutdown => break,
            Command::StartInputMode => {
                #[cfg(target_os = "windows")]
                {
                    send_error(&tx, "Input mode not yet implemented");
                    tx.send(Message::InputModeState { state: "error".into() }).ok();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    send_error(&tx, "Input mode not supported on this platform");
                    tx.send(Message::InputModeState { state: "error".into() }).ok();
                }
            }
            Command::StopInputMode => {
                tx.send(Message::InputModeState { state: "inactive".into() }).ok();
            }
            Command::CaptureWithText { content } => {
                if is_processing.load(Ordering::SeqCst) {
                    send_error(&tx, "Already processing");
                    continue;
                }
                if content.trim().is_empty() {
                    continue;
                }
                is_processing.store(true, Ordering::SeqCst);

                let system_prompt = match profiles::get_system_prompt(&config.profile) {
                    Some(p) => p,
                    None => {
                        send_error(&tx, &format!("Unknown profile: {}", config.profile));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let image_b64 = match capture::capture_primary_monitor() {
                    Ok(img) => img,
                    Err(e) => {
                        send_error(&tx, &format!("Capture failed: {}", e));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let result = match config.model.as_str() {
                    "gpt-4o" => {
                        llm::stream_openai(&tx, &config.openai_api_key, &system_prompt, &image_b64, Some(&content)).await
                    }
                    "claude" | "claude-sonnet" => {
                        llm::stream_anthropic(&tx, &config.anthropic_api_key, &system_prompt, &image_b64, Some(&content)).await
                    }
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    send_error(&tx, &format!("LLM error: {}", e));
                }

                is_processing.store(false, Ordering::SeqCst);
            }
        }
    }
}
```

Note: Startup errors (`Config error`, `Config validation`) now go to `eprintln!` instead of `print_error` because the writer thread hasn't been spawned yet at that point, and these are fatal errors that should go to stderr anyway.

- [ ] **Step 3: Build to verify compilation**

Run: `cd copilotx/sidecar && cargo build`
Expected: FAIL — `llm::stream_openai` and `llm::stream_anthropic` signatures don't match yet (they still accept old params without `tx` and `user_text`). This is expected; Task 3 fixes this.

- [ ] **Step 4: Commit (will commit after Task 3 completes the build fix)**

---

### Task 3: Refactor llm.rs — remove duplicate print_message, add tx and user_text params

**Files:**
- Modify: `copilotx/sidecar/src/llm.rs`

- [ ] **Step 1: Replace entire llm.rs with channel-based version**

Replace the entire content of `copilotx/sidecar/src/llm.rs` with:

```rust
use anyhow::{Result, bail};
use std::sync::mpsc::Sender;

use crate::protocol::Message;

pub async fn stream_openai(
    tx: &Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()> {
    use async_openai::{
        Client,
        config::OpenAIConfig,
        types::chat::{
            ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
            ChatCompletionRequestUserMessageContentPart,
            ChatCompletionRequestMessageContentPartTextArgs,
            ChatCompletionRequestMessageContentPartImageArgs,
            CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
        },
    };
    use futures::StreamExt;

    let prompt_text = user_text.unwrap_or("Analyze this screenshot and provide the answer.");

    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .stream(true)
        .messages(vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()?,
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::Array(vec![
                        ChatCompletionRequestUserMessageContentPart::Text(
                            ChatCompletionRequestMessageContentPartTextArgs::default()
                                .text(prompt_text)
                                .build()?,
                        ),
                        ChatCompletionRequestUserMessageContentPart::ImageUrl(
                            ChatCompletionRequestMessageContentPartImageArgs::default()
                                .image_url(
                                    ImageUrlArgs::default()
                                        .url(format!("data:image/png;base64,{}", image_base64))
                                        .detail(ImageDetail::High)
                                        .build()?,
                                )
                                .build()?,
                        ),
                    ]))
                    .build()?,
            ),
        ])
        .build()?;

    let mut stream = client.chat().create_stream(request).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                for choice in response.choices {
                    if let Some(content) = choice.delta.content {
                        tx.send(Message::Token { content }).ok();
                    }
                }
            }
            Err(e) => {
                tx.send(Message::Error {
                    message: e.to_string(),
                }).ok();
                return Err(e.into());
            }
        }
    }

    tx.send(Message::Done).ok();
    Ok(())
}

pub async fn stream_anthropic(
    tx: &Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()> {
    use reqwest::Client as HttpClient;
    use reqwest_eventsource::{Event, EventSource};
    use futures::StreamExt;

    let prompt_text = user_text.unwrap_or("Analyze this screenshot and provide the answer.");

    let client = HttpClient::new();
    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "stream": true,
        "system": system_prompt,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": image_base64
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt_text
                    }
                ]
            }
        ]
    });

    let mut es = EventSource::new(
        client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body.to_string()),
    )?;

    while let Some(event) = es.next().await {
        match event? {
            Event::Open => continue,
            Event::Message(msg) => {
                let parsed: serde_json::Value = serde_json::from_str(&msg.data)?;
                let event_type = parsed["type"].as_str().unwrap_or("");
                match event_type {
                    "content_block_delta" => {
                        if let Some(text) = parsed["delta"]["text"].as_str() {
                            tx.send(Message::Token {
                                content: text.to_string(),
                            }).ok();
                        }
                    }
                    "message_stop" => {
                        tx.send(Message::Done).ok();
                        es.close();
                        return Ok(());
                    }
                    "error" => {
                        let err_msg = parsed["error"]["message"]
                            .as_str()
                            .unwrap_or("Unknown Anthropic error");
                        tx.send(Message::Error {
                            message: err_msg.to_string(),
                        }).ok();
                        bail!("Anthropic API error: {}", err_msg);
                    }
                    _ => {}
                }
            }
        }
    }

    tx.send(Message::Done).ok();
    Ok(())
}
```

- [ ] **Step 2: Build to verify compilation**

Run: `cd copilotx/sidecar && cargo build`
Expected: PASS (both main.rs and llm.rs now use the same `tx: &Sender<Message>` pattern)

- [ ] **Step 3: Run existing tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS (all unit tests; integration tests use stdin so they go through the writer thread now)

- [ ] **Step 4: Run integration tests**

Run: `cd copilotx/sidecar && cargo test --test integration`
Expected: PASS (ping/pong still works through writer thread)

- [ ] **Step 5: Commit**

```bash
cd copilotx
git add copilotx/sidecar/src/main.rs copilotx/sidecar/src/llm.rs
git commit -m "refactor(sidecar): replace print_message with writer thread and mpsc channel, add tx+user_text params to LLM functions"
```

---

### Task 4: Add placeholder keyboard.rs and mod keyboard to main.rs

**Files:**
- Create: `copilotx/sidecar/src/keyboard.rs`
- Modify: `copilotx/sidecar/src/main.rs:1` (add `mod keyboard;`)

- [ ] **Step 1: Create keyboard.rs with non-Windows stubs**

Create `copilotx/sidecar/src/keyboard.rs` with:

```rust
use crate::protocol::Message;
use std::sync::mpsc::Sender;

pub struct HookHandle {
    _private: (),
}

#[cfg(not(target_os = "windows"))]
pub fn start_keyboard_hook(_tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    anyhow::bail!("Input mode is not supported on this platform")
}

#[cfg(not(target_os = "windows"))]
pub fn stop_keyboard_hook(_handle: HookHandle) {}
```

- [ ] **Step 2: Add mod keyboard to main.rs**

Add `mod keyboard;` after line 4 (`mod profiles;`) in `copilotx/sidecar/src/main.rs`, so the module declarations become:

```rust
mod capture;
mod config;
mod keyboard;
mod llm;
mod profiles;
mod protocol;
```

- [ ] **Step 3: Build to verify compilation**

Run: `cd copilotx/sidecar && cargo build`
Expected: PASS

- [ ] **Step 4: Run all tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd copilotx
git add copilotx/sidecar/src/keyboard.rs copilotx/sidecar/src/main.rs
git commit -m "feat(sidecar): add keyboard.rs module with non-Windows stubs"
```

---

### Task 5: Add input_hotkey and overlay_height to Rust config

**Files:**
- Modify: `copilotx/sidecar/src/config.rs`

- [ ] **Step 1: Write failing test for input_hotkey deserialization**

Add this test inside the `#[cfg(test)] mod tests` block in `copilotx/sidecar/src/config.rs`, after the `test_load_missing_file` test:

```rust
#[test]
fn test_load_config_with_input_hotkey() {
    let json = r#"{
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
    }"#;
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", json).unwrap();
    let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
    assert_eq!(config.input_hotkey, "Ctrl+Shift+K");
    assert_eq!(config.overlay_height, 600);
}

#[test]
fn test_input_hotkey_defaults_when_missing() {
    let json = r#"{
        "hotkey": "CommandOrControl+Shift+Space",
        "model": "gpt-4o",
        "openaiApiKey": "sk-test",
        "anthropicApiKey": "",
        "profile": "interview",
        "overlayOpacity": 0.85,
        "overlayWidth": 320,
        "overlayPosition": "right"
    }"#;
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", json).unwrap();
    let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
    assert_eq!(config.input_hotkey, "Ctrl+Shift+K");
    assert_eq!(config.overlay_height, 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd copilotx/sidecar && cargo test config`
Expected: FAIL — no field `input_hotkey` on struct `Config`

- [ ] **Step 3: Add input_hotkey and overlay_height fields to Config struct**

Replace the `Config` struct (lines 6-19 of `copilotx/sidecar/src/config.rs`) with:

```rust
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub hotkey: String,
    #[serde(default = "default_input_hotkey")]
    pub input_hotkey: String,
    pub model: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: String,
    pub profile: String,
    pub overlay_opacity: f64,
    pub overlay_width: u32,
    #[serde(default)]
    pub overlay_height: u32,
    pub overlay_position: String,
}

fn default_input_hotkey() -> String {
    "Ctrl+Shift+K".to_string()
}
```

Also update the `make_valid_config_json()` helper (lines 94-106) to include both fields:

```rust
fn make_valid_config_json() -> String {
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
```

Also update the `test_validate_missing_api_key` test's JSON (lines 127-136) to include the new fields:

```rust
let json = r#"{
            "hotkey": "CommandOrControl+Shift+Space",
            "inputHotkey": "Ctrl+Shift+K",
            "model": "gpt-4o",
            "openaiApiKey": "",
            "anthropicApiKey": "",
            "profile": "interview",
            "overlayOpacity": 0.85,
            "overlayWidth": 320,
            "overlayHeight": 600,
            "overlayPosition": "right"
        }"#;
```

Also update the `test_validate_unknown_model` test's JSON (lines 145-155) similarly:

```rust
let json = r#"{
            "hotkey": "CommandOrControl+Shift+Space",
            "inputHotkey": "Ctrl+Shift+K",
            "model": "gpt-3",
            "openaiApiKey": "sk-test",
            "anthropicApiKey": "",
            "profile": "interview",
            "overlayOpacity": 0.85,
            "overlayWidth": 320,
            "overlayHeight": 600,
            "overlayPosition": "right"
        }"#;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd copilotx/sidecar && cargo test config`
Expected: PASS

- [ ] **Step 5: Run all tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
cd copilotx
git add copilotx/sidecar/src/config.rs
git commit -m "feat(sidecar): add input_hotkey and overlay_height to Config with defaults"
```

---

### Task 6: Update integration tests for new commands

**Files:**
- Modify: `copilotx/sidecar/tests/integration.rs`

- [ ] **Step 1: Write failing tests for new commands**

Replace the `valid_config_json()` function (lines 6-18) with:

```rust
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
```

Add these tests after the existing `test_shutdown` test:

```rust
#[test]
fn test_start_input_mode_non_windows() {
    let mut f = NamedTempFile::new().unwrap();
    write!(f, "{}", valid_config_json()).unwrap();
    Command::cargo_bin("system-helper")
        .unwrap()
        .env("COPILOTX_CONFIG", f.path())
        .write_stdin(r#"{"type":"start_input_mode"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"{"type":"input_mode_state","state":"error"}"#));
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
```

- [ ] **Step 2: Run integration tests**

Run: `cd copilotx/sidecar && cargo test --test integration`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
cd copilotx
git add copilotx/sidecar/tests/integration.rs
git commit -m "test(sidecar): add integration tests for start_input_mode, stop_input_mode, capture_with_text"
```

---

### Task 7: Verify no print_message duplication remains

**Files:**
- Verify: `copilotx/sidecar/src/main.rs`, `copilotx/sidecar/src/llm.rs`

- [ ] **Step 1: Grep for print_message**

Run: `cd copilotx/sidecar && grep -rn "print_message" src/`
Expected: No results — all `print_message` calls have been replaced with `tx.send()`

- [ ] **Step 2: Grep for print_error**

Run: `cd copilotx/sidecar && grep -rn "print_error" src/`
Expected: No results — all `print_error` calls have been replaced with `send_error()`

- [ ] **Step 3: Full cargo test suite**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS (all unit + integration tests)

---

## Phase 2: Keyboard Hook Module (Windows-Only)

### Task 8: Add windows crate dependency to Cargo.toml

**Files:**
- Modify: `copilotx/sidecar/Cargo.toml`

- [ ] **Step 1: Add target-specific windows crate dependency**

Add after line 23 (`xcap = "0.9"`), before the `[dev-dependencies]` section:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_Input_KeyboardAndMouse",
    "Win32_System_Threading",
    "Win32_Foundation",
] }
```

- [ ] **Step 2: Build to verify it resolves on non-Windows**

Run: `cd copilotx/sidecar && cargo build`
Expected: PASS (the target-specific dependency is skipped on Linux/macOS)

- [ ] **Step 3: Commit**

```bash
cd copilotx
git add copilotx/sidecar/Cargo.toml
git commit -m "feat(sidecar): add windows crate as target-specific dependency for keyboard hook"
```

---

### Task 9: Implement keyboard.rs for Windows

**Files:**
- Modify: `copilotx/sidecar/src/keyboard.rs`

- [ ] **Step 1: Replace keyboard.rs with full Windows implementation**

Replace the entire content of `copilotx/sidecar/src/keyboard.rs` with:

```rust
use crate::protocol::Message;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const INACTIVITY_TIMEOUT_MS: u32 = 2500;

pub struct HookHandle {
    pub thread_id: u32,
    pub join_handle: Option<std::thread::JoinHandle<()>>,
    pub stop_flag: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_TX: RefCell<Option<Sender<Message>>> = RefCell::new(None);
    static HOOK_STOP_FLAG: RefCell<Arc<AtomicBool>> = RefCell::new(Arc::new(AtomicBool::new(false)));
    static INACTIVITY_TIMER: RefCell<std::time::Instant> = RefCell::new(std::time::Instant::now());
}

#[cfg(target_os = "windows")]
fn is_input_key(vk_code: u32) -> bool {
    matches!(
        vk_code,
        0x30..=0x39
        | 0x41..=0x5A
        | 0x20
        | 0x08
        | 0x0D
        | 0x1B
        | 0xBA..=0xC0
        | 0xDB..=0xDD
        | 0xDE
    )
}

#[cfg(target_os = "windows")]
fn map_key_event(vk_code: u32, flags: u32) -> Option<(String, bool, bool)> {
    if flags & (1 << 7) != 0 {
        return None;
    }

    let shift_pressed = {
        use windows::Win32::Input::KeyboardAndMouse::GetKeyState;
        unsafe { GetKeyState(windows::Win32::Input::KeyboardAndMouse::VK_SHIFT.0 as i32) < 0 }
    };
    let ctrl_pressed = {
        use windows::Win32::Input::KeyboardAndMouse::GetKeyState;
        unsafe { GetKeyState(windows::Win32::Input::KeyboardAndMouse::VK_CONTROL.0 as i32) < 0 }
    };

    if ctrl_pressed {
        return None;
    }

    match vk_code {
        0x10 | 0x11 | 0x12 => return None,
        _ => {}
    }

    let key = match vk_code {
        0x0D => "Enter".to_string(),
        0x08 => "Backspace".to_string(),
        0x1B => "Escape".to_string(),
        0x20 => " ".to_string(),
        0x30..=0x39 => (vk_code as u8 - 0x30 + b'0') as char,
        0x41..=0x5A => {
            if shift_pressed {
                (vk_code as u8 - 0x41 + b'A') as char
            } else {
                (vk_code as u8 - 0x41 + b'a') as char
            }
        }
        0xBA => if shift_pressed { ':' } else { ';' },
        0xBB => if shift_pressed { '+' } else { '=' },
        0xBC => if shift_pressed { '<' } else { ',' },
        0xBD => if shift_pressed { '_' } else { '-' },
        0xBE => if shift_pressed { '>' } else { '.' },
        0xBF => if shift_pressed { '?' } else { '/' },
        0xC0 => if shift_pressed { '~' } else { '`' },
        0xDB => if shift_pressed { '{' } else { '[' },
        0xDC => if shift_pressed { '|' } else { '\\' },
        0xDD => if shift_pressed { '}' } else { ']' },
        0xDE => if shift_pressed { '"' } else { '\'' },
        _ => return None,
    };

    Some((key.to_string(), shift_pressed, ctrl_pressed))
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_callback(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Input::KeyboardAndMouse::KBDLLHOOKSTRUCT;

    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let kb_struct: &KBDLLHOOKSTRUCT = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    let vk_code = kb_struct.vkCode;
    let flags = kb_struct.flags;

    let should_pass_through = HOOK_STOP_FLAG.with(|f| f.borrow().load(Ordering::SeqCst));
    if should_pass_through {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    if is_input_key(vk_code) {
        if let Some((key, shift, ctrl)) = map_key_event(vk_code, flags) {
            HOOK_TX.with(|tx_cell| {
                if let Some(tx) = tx_cell.borrow().as_ref() {
                    tx.send(Message::KeyEvent { key, shift, ctrl }).ok();
                }
            });
            INACTIVITY_TIMER.with(|t| *t.borrow_mut() = std::time::Instant::now());
        }
        return windows::Win32::Foundation::LRESULT(1);
    }

    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(target_os = "windows")]
pub fn start_keyboard_hook(tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::System::Threading::GetCurrentThreadId;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    let (thread_id_tx, thread_id_rx) = std::sync::mpsc::channel::<u32>();

    let join_handle = std::thread::spawn(move || {
        INACTIVITY_TIMER.with(|t| *t.borrow_mut() = std::time::Instant::now());

        HOOK_TX.with(|tx_cell| *tx_cell.borrow_mut() = Some(tx));
        HOOK_STOP_FLAG.with(|f_cell| *f_cell.borrow_mut() = stop_flag_clone.clone());

        let hook = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_callback),
                None,
                0,
            )
        };

        let thread_id = unsafe { GetCurrentThreadId() };

        if hook.is_invalid() {
            thread_id_tx.send(0).ok();
            return;
        }

        thread_id_tx.send(thread_id).ok();

        let mut msg = MSG::default();
        loop {
            let remaining_ms = INACTIVITY_TIMEOUT_MS.saturating_sub(
                INACTIVITY_TIMER.with(|t| t.borrow().elapsed().as_millis() as u32)
            );

            let wait_result = unsafe {
                MsgWaitForMultipleObjectsEx(
                    None,
                    remaining_ms,
                    QS_ALLINPUT,
                    MWMO_ALERTABLE,
                )
            };

            if wait_result == WAIT_TIMEOUT {
                tx.send(Message::InputModeState { state: "inactive".into() }).ok();
                unsafe { UnhookWindowsHookEx(hook).ok() };
                return;
            }

            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                if msg.message == WM_QUIT {
                    unsafe { UnhookWindowsHookEx(hook).ok() };
                    return;
                }
                unsafe { TranslateMessage(&msg); }
                unsafe { DispatchMessageW(&msg); }
            }
        }
    });

    let thread_id = thread_id_rx.recv()?;
    if thread_id == 0 {
        anyhow::bail!("Failed to register keyboard hook");
    }

    Ok(HookHandle {
        thread_id,
        join_handle: Some(join_handle),
        stop_flag,
    })
}

#[cfg(target_os = "windows")]
pub fn stop_keyboard_hook(handle: HookHandle) {
    use windows::Win32::UI::WindowsAndMessaging::*;

    handle.stop_flag.store(true, Ordering::SeqCst);
    unsafe {
        PostThreadMessageW(handle.thread_id, WM_QUIT, None, None).ok();
    }
    if let Some(jh) = handle.join_handle {
        let _ = jh.join();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn start_keyboard_hook(_tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    anyhow::bail!("Input mode is not supported on this platform")
}

#[cfg(not(target_os = "windows"))]
pub fn stop_keyboard_hook(_handle: HookHandle) {}
```

- [ ] **Step 2: Build on non-Windows to verify stubs compile**

Run: `cd copilotx/sidecar && cargo build`
Expected: PASS (only the `#[cfg(not(target_os = "windows"))]` stubs compile on Linux/macOS)

- [ ] **Step 3: Run all tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
cd copilotx
git add copilotx/sidecar/src/keyboard.rs
git commit -m "feat(sidecar): implement WH_KEYBOARD_LL keyboard hook for Windows with inactivity timeout"
```

---

### Task 10: Wire keyboard hook into main.rs StartInputMode/StopInputMode handlers

**Files:**
- Modify: `copilotx/sidecar/src/main.rs`

- [ ] **Step 1: Add hook_handle tracking and update StartInputMode/StopInputMode/CaptureWithText handlers**

In `copilotx/sidecar/src/main.rs`, after the `let is_processing = Arc::new(AtomicBool::new(false));` line, add:

```rust
let mut hook_handle: Option<keyboard::HookHandle> = None;
```

Replace the `Command::StartInputMode` arm with:

```rust
Command::StartInputMode => {
    match keyboard::start_keyboard_hook(tx.clone()) {
        Ok(handle) => {
            hook_handle = Some(handle);
            tx.send(Message::InputModeState { state: "active".into() }).ok();
        }
        Err(e) => {
            send_error(&tx, &format!("Hook registration failed: {}", e));
            tx.send(Message::InputModeState { state: "error".into() }).ok();
        }
    }
}
```

Replace the `Command::StopInputMode` arm with:

```rust
Command::StopInputMode => {
    if let Some(handle) = hook_handle.take() {
        keyboard::stop_keyboard_hook(handle);
    }
    tx.send(Message::InputModeState { state: "inactive".into() }).ok();
}
```

In the `Command::CaptureWithText` arm, add hook cleanup at the start (before the `is_processing` check):

```rust
Command::CaptureWithText { content } => {
    if let Some(handle) = hook_handle.take() {
        keyboard::stop_keyboard_hook(handle);
    }
    tx.send(Message::InputModeState { state: "inactive".into() }).ok();

    if is_processing.load(Ordering::SeqCst) {
        send_error(&tx, "Already processing");
        continue;
    }
    // ... rest unchanged
```

- [ ] **Step 2: Build**

Run: `cd copilotx/sidecar && cargo build`
Expected: PASS

- [ ] **Step 3: Run all tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS

- [ ] **Step 4: Run integration tests**

Run: `cd copilotx/sidecar && cargo test --test integration`
Expected: PASS (`test_start_input_mode_non_windows` still returns error state on Linux)

- [ ] **Step 5: Commit**

```bash
cd copilotx
git add copilotx/sidecar/src/main.rs
git commit -m "feat(sidecar): wire keyboard hook into StartInputMode/StopInputMode/CaptureWithText handlers"
```

---

## Phase 3: Electron Main Process & Preload Bridge

### Task 11: Add inputHotkey to config files

**Files:**
- Modify: `copilotx/config/config.json`
- Modify: `copilotx/config/schemas/config.schema.json`
- Modify: `copilotx/src/main/config.ts`

- [ ] **Step 1: Add inputHotkey to config.json**

Add `"inputHotkey": "Ctrl+Shift+K"` after the `"hotkey"` line in `copilotx/config/config.json`, so it becomes:

```json
{
  "hotkey": "Ctrl+Shift+Space",
  "inputHotkey": "Ctrl+Shift+K",
  "model": "gpt-4o",
  "openaiApiKey": "",
  "anthropicApiKey": "",
  "profile": "interview",
  "overlayOpacity": 0.85,
  "overlayWidth": 320,
  "overlayHeight": 600,
  "overlayPosition": "right"
}
```

- [ ] **Step 2: Add inputHotkey to config.schema.json**

Add after the `"hotkey"` property definition (after line 10) in `copilotx/config/schemas/config.schema.json`:

```json
"inputHotkey": {
  "type": "string",
  "description": "Electron accelerator string for the input mode hotkey",
  "default": "Ctrl+Shift+K"
},
```

Also add `"inputHotkey"` to the `"required"` array:

```json
"required": ["hotkey", "inputHotkey", "model"],
```

- [ ] **Step 3: Add inputHotkey to AppConfig TypeScript interface**

Add `inputHotkey: string` to the `AppConfig` interface in `copilotx/src/main/config.ts` (after the `hotkey` field):

```typescript
export interface AppConfig {
  hotkey: string
  inputHotkey: string
  model: string
  openaiApiKey: string
  anthropicApiKey: string
  profile: string
  overlayOpacity: number
  overlayWidth: number
  overlayHeight: number
  overlayPosition: string
}
```

Add validation in `validateConfig` (after the `if (!config.hotkey)` check):

```typescript
if (!config.inputHotkey) {
  errors.push('inputHotkey is required')
}
```

- [ ] **Step 4: Commit**

```bash
cd copilotx
git add copilotx/config/config.json copilotx/config/schemas/config.schema.json copilotx/src/main/config.ts
git commit -m "feat(config): add inputHotkey field to config files and AppConfig validation"
```

---

### Task 12: Add input mode state to hotkey module

**Files:**
- Modify: `copilotx/src/main/hotkey.ts`

- [ ] **Step 1: Replace hotkey.ts with input mode support**

Replace the entire content of `copilotx/src/main/hotkey.ts` with:

```typescript
import { globalShortcut, BrowserWindow } from 'electron'
import { sendCapture, sendStartInputMode } from './ipc'

let isProcessing = false
let isInputMode = false

export function registerHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isInputMode) return
    if (isProcessing) {
      window.webContents.send('capture-state', 'already-processing')
      return
    }

    isProcessing = true
    window.show()
    window.webContents.send('capture-state', 'processing')
    sendCapture()
  })

  if (!registered) {
    console.error(`Failed to register hotkey: ${accelerator}`)
  }

  return registered
}

export function registerInputHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isProcessing || isInputMode) return
    isInputMode = true
    sendStartInputMode()
    window.webContents.send('input-mode-state', 'active')
  })

  if (!registered) {
    console.error(`Failed to register input hotkey: ${accelerator}`)
  }

  return registered
}

export function setProcessingComplete(): void {
  isProcessing = false
}

export function setInputModeActive(): void {
  isInputMode = true
}

export function setInputModeInactive(): void {
  isInputMode = false
}

export function isInInputMode(): boolean {
  return isInputMode
}

export function unregisterAll(): void {
  globalShortcut.unregisterAll()
}
```

- [ ] **Step 2: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: FAIL — `sendStartInputMode` not yet exported from `ipc.ts`. This is expected; Task 13 adds it.

---

### Task 13: Extend IPC for new commands and messages

**Files:**
- Modify: `copilotx/src/main/ipc.ts`

- [ ] **Step 1: Update SidecarMessage interface and add new send functions**

Replace the `SidecarMessage` interface (lines 6-10) with:

```typescript
export interface SidecarMessage {
  type: 'token' | 'done' | 'error' | 'pong' | 'key_event' | 'input_mode_state'
  content?: string
  message?: string
  key?: string
  shift?: boolean
  ctrl?: boolean
  state?: string
}
```

Add after the `sendPing` function (after line 111):

```typescript
export function sendStartInputMode(): void {
  writeSidecar({ type: 'start_input_mode' })
}

export function sendStopInputMode(): void {
  writeSidecar({ type: 'stop_input_mode' })
}

export function sendCaptureWithText(content: string): void {
  writeSidecar({ type: 'capture_with_text', content })
}
```

Replace the `writeSidecar` function (lines 117-120) with:

```typescript
function writeSidecar(msg: Record<string, unknown>): void {
  if (!sidecar?.stdin || sidecar.stdin.destroyed) return
  sidecar.stdin.write(JSON.stringify(msg) + '\n')
}
```

- [ ] **Step 2: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: PASS (both hotkey.ts and ipc.ts should now compile)

- [ ] **Step 3: Commit**

```bash
cd copilotx
git add copilotx/src/main/hotkey.ts copilotx/src/main/ipc.ts
git commit -m "feat(electron): add input mode hotkey registration and IPC send functions for new sidecar commands"
```

---

### Task 14: Update main index.ts to handle new messages

**Files:**
- Modify: `copilotx/src/main/index.ts`

- [ ] **Step 1: Update imports**

Replace lines 1-7 of `copilotx/src/main/index.ts` with:

```typescript
import { app, BrowserWindow, ipcMain } from 'electron'
import { electronApp } from '@electron-toolkit/utils'
import { createOverlayWindow } from './overlay'
import { startSidecar, stopSidecar, onSidecarMessage, sendCapture, sendStopInputMode, sendCaptureWithText } from './ipc'
import { loadConfig, validateConfig } from './config'
import { registerHotkey, setProcessingComplete, unregisterAll, registerInputHotkey, isInInputMode, setInputModeInactive } from './hotkey'
import { registerPositionHotkeys } from './position'
```

- [ ] **Step 2: Add key_event and input_mode_state cases to onSidecarMessage handler**

Replace the `onSidecarMessage` handler (lines 31-48) with:

```typescript
onSidecarMessage((msg) => {
    if (!overlayWindow) return
    switch (msg.type) {
      case 'token':
        overlayWindow.webContents.send('token', msg.content)
        break
      case 'done':
        setProcessingComplete()
        overlayWindow.webContents.send('capture-state', 'done')
        break
      case 'error':
        setProcessingComplete()
        overlayWindow.webContents.send('capture-state', 'error', msg.message)
        break
      case 'pong':
        break
      case 'key_event':
        if (isInInputMode()) {
          overlayWindow.webContents.send('key-event', msg.key, msg.shift, msg.ctrl)
        }
        break
      case 'input_mode_state':
        if (msg.state === 'inactive' || msg.state === 'error') {
          setInputModeInactive()
          overlayWindow.webContents.send('input-mode-state', msg.state)
        }
        break
    }
  })
```

- [ ] **Step 3: Register input hotkey**

After line 50 (`registerHotkey(config.hotkey, overlayWindow)`), add:

```typescript
registerInputHotkey(config.inputHotkey, overlayWindow)
```

- [ ] **Step 4: Add IPC handlers for renderer communication**

After the existing `ipcMain.handle('window-close', ...)` block (after line 61), add:

```typescript
ipcMain.handle('send-text-input', (_event, text: string) => {
    if (!overlayWindow) return
    overlayWindow.webContents.send('capture-state', 'processing')
    sendCaptureWithText(text)
    setInputModeInactive()
  })

  ipcMain.handle('stop-input-mode', () => {
    sendStopInputMode()
    setInputModeInactive()
  })
```

- [ ] **Step 5: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
cd copilotx
git add copilotx/src/main/index.ts
git commit -m "feat(electron): handle key_event/input_mode_state messages, register input hotkey, add IPC handlers for renderer"
```

---

### Task 15: Extend preload bridge

**Files:**
- Modify: `copilotx/src/preload/index.ts`
- Modify: `copilotx/src/preload/index.d.ts`

- [ ] **Step 1: Add new methods to preload bridge**

Replace the `api` object in `copilotx/src/preload/index.ts` (lines 4-10) with:

```typescript
const api = {
  onToken: (callback: (content: string) => void) =>
    ipcRenderer.on('token', (_event, content) => callback(content)),
  onCaptureState: (callback: (state: string, error?: string) => void) =>
    ipcRenderer.on('capture-state', (_event, state, error) => callback(state, error)),
  onKeyEvent: (callback: (key: string, shift: boolean, ctrl: boolean) => void) =>
    ipcRenderer.on('key-event', (_event, key, shift, ctrl) => callback(key, shift, ctrl)),
  onInputModeState: (callback: (state: string) => void) =>
    ipcRenderer.on('input-mode-state', (_event, state) => callback(state)),
  triggerCapture: () => ipcRenderer.invoke('trigger-capture'),
  sendTextInput: (text: string) => ipcRenderer.invoke('send-text-input', text),
  stopInputMode: () => ipcRenderer.invoke('stop-input-mode'),
  close: () => ipcRenderer.invoke('window-close')
}
```

- [ ] **Step 2: Update Window.api type declaration**

Replace the `api` interface in `copilotx/src/preload/index.d.ts` (lines 6-11) with:

```typescript
api: {
  onToken: (callback: (content: string) => void) => void
  onCaptureState: (callback: (state: string, error?: string) => void) => void
  onKeyEvent: (callback: (key: string, shift: boolean, ctrl: boolean) => void) => void
  onInputModeState: (callback: (state: string) => void) => void
  triggerCapture: () => Promise<void>
  sendTextInput: (text: string) => Promise<void>
  stopInputMode: () => Promise<void>
  close: () => Promise<void>
}
```

- [ ] **Step 3: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
cd copilotx
git add copilotx/src/preload/index.ts copilotx/src/preload/index.d.ts
git commit -m "feat(preload): add onKeyEvent, onInputModeState, sendTextInput, stopInputMode to bridge"
```

---

### Task 16: Add Electron tests for new types and input mode guards

**Files:**
- Modify: `copilotx/src/main/__tests__/config.test.ts`
- Modify: `copilotx/src/main/__tests__/ipc.test.ts`
- Modify: `copilotx/src/main/__tests__/hotkey.test.ts`

- [ ] **Step 1: Add inputHotkey to config test fixture and add validation test**

In `copilotx/src/main/__tests__/config.test.ts`, add `inputHotkey` to the `validConfig` object (after the `hotkey` field):

```typescript
const validConfig: AppConfig = {
    hotkey: 'CommandOrControl+Shift+Space',
    inputHotkey: 'Ctrl+Shift+K',
    model: 'gpt-4o',
    // ... rest unchanged
```

Add this test after the `it('returns error for unknown profile'` test:

```typescript
it('returns error for empty inputHotkey', () => {
    const config = { ...validConfig, inputHotkey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('inputHotkey'))
  })
```

- [ ] **Step 2: Add IPC tests for new message types**

Add these tests at the end of `copilotx/src/main/__tests__/ipc.test.ts`:

```typescript
it('parses a key_event message', () => {
    const raw = '{"type":"key_event","key":"a","shift":false,"ctrl":false}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('key_event')
    expect(msg.key).toBe('a')
    expect(msg.shift).toBe(false)
    expect(msg.ctrl).toBe(false)
  })

  it('parses an input_mode_state message', () => {
    const raw = '{"type":"input_mode_state","state":"active"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('input_mode_state')
    expect(msg.state).toBe('active')
  })
```

- [ ] **Step 3: Add hotkey tests for input mode guards**

Add these tests at the end of `copilotx/src/main/__tests__/hotkey.test.ts`:

```typescript
describe('input mode state guards', () => {
  it('blocks capture hotkey when in input mode', () => {
    const isInputMode = true
    const isProcessing = false
    const result = !isInputMode && !isProcessing
    expect(result).toBe(false)
  })

  it('blocks input hotkey when already in input mode', () => {
    const isInputMode = true
    const result = !isInputMode
    expect(result).toBe(false)
  })

  it('allows input hotkey when not in input mode', () => {
    const isInputMode = false
    const isProcessing = false
    const result = !isInputMode && !isProcessing
    expect(result).toBe(true)
  })

  it('blocks input hotkey when processing', () => {
    const isInputMode = false
    const isProcessing = true
    const result = !isInputMode && !isProcessing
    expect(result).toBe(false)
  })
})
```

- [ ] **Step 4: Run all Electron tests**

Run: `cd copilotx && pnpm run test`
Expected: PASS

- [ ] **Step 5: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: PASS

- [ ] **Step 6: ESLint**

Run: `cd copilotx && pnpm run lint`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
cd copilotx
git add copilotx/src/main/__tests__/config.test.ts copilotx/src/main/__tests__/ipc.test.ts copilotx/src/main/__tests__/hotkey.test.ts
git commit -m "test(electron): add inputHotkey validation, key_event/input_mode_state parsing, and input mode guard tests"
```

---

## Phase 4: Renderer — TextInputBar & Input Mode UI

### Task 17: Create TextInputBar component

**Files:**
- Create: `copilotx/src/renderer/src/TextInputBar.tsx`

- [ ] **Step 1: Create TextInputBar.tsx**

Create `copilotx/src/renderer/src/TextInputBar.tsx` with:

```tsx
interface TextInputBarProps {
  text: string
  isActive: boolean
}

export function TextInputBar({ text, isActive }: TextInputBarProps) {
  if (!isActive) return null
  return (
    <div className="text-input-bar">
      <span className="input-text">{text}</span>
      <span className="input-cursor">|</span>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
cd copilotx
git add copilotx/src/renderer/src/TextInputBar.tsx
git commit -m "feat(renderer): create TextInputBar component with blinking cursor"
```

---

### Task 18: Update AnswerPanel for dimmed state

**Files:**
- Modify: `copilotx/src/renderer/src/AnswerPanel.tsx`

- [ ] **Step 1: Add dimmed prop to AnswerPanel**

Replace the `AnswerPanelProps` interface and function signature in `copilotx/src/renderer/src/AnswerPanel.tsx` with:

```tsx
interface AnswerPanelProps {
  content: string
  state: OverlayState
  errorMessage: string
  dimmed?: boolean
}

export function AnswerPanel({ content, state, errorMessage, dimmed }: AnswerPanelProps) {
```

Update each return's `<div className="answer-panel ...">` to include the dimmed class:

- Line 12: `<div className={`answer-panel idle${dimmed ? ' dimmed' : ''}`}>`
- Line 21: `<div className={`answer-panel processing${dimmed ? ' dimmed' : ''}`}>`
- Line 30: `<div className={`answer-panel error${dimmed ? ' dimmed' : ''}`}>`
- Line 36: `<div className={`answer-panel streaming${dimmed ? ' dimmed' : ''}`}>`

- [ ] **Step 2: Commit**

```bash
cd copilotx
git add copilotx/src/renderer/src/AnswerPanel.tsx
git commit -m "feat(renderer): add dimmed prop to AnswerPanel for input mode visual dimming"
```

---

### Task 19: Update App.tsx for input mode state management

**Files:**
- Modify: `copilotx/src/renderer/src/App.tsx`

- [ ] **Step 1: Replace App.tsx with full input mode support**

Replace the entire content of `copilotx/src/renderer/src/App.tsx` with:

```tsx
import { useState, useEffect, useRef } from 'react'
import { TitleBar } from './TitleBar'
import { AnswerPanel } from './AnswerPanel'
import { TextInputBar } from './TextInputBar'
import './styles.css'

interface Answer {
  id: number
  content: string
  error?: string
}

export type OverlayState = 'idle' | 'processing' | 'streaming' | 'error'

export default function App() {
  const [state, setState] = useState<OverlayState>('idle')
  const [answers, setAnswers] = useState<Answer[]>([])
  const [currentIndex, setCurrentIndex] = useState(0)
  const [streamingContent, setStreamingContent] = useState('')
  const [errorMessage, setErrorMessage] = useState('')
  const [inputModeActive, setInputModeActive] = useState(false)
  const [inputText, setInputText] = useState('')
  const streamingRef = useRef(streamingContent)
  const answersLengthRef = useRef(answers.length)
  const inputModeActiveRef = useRef(inputModeActive)
  const inputTextRef = useRef(inputText)
  streamingRef.current = streamingContent
  answersLengthRef.current = answers.length
  inputModeActiveRef.current = inputModeActive
  inputTextRef.current = inputText

  useEffect(() => {
    window.api.onToken((content: string) => {
      setState('streaming')
      setStreamingContent((prev) => prev + content)
    })

    window.api.onCaptureState((newState: string, error?: string) => {
      if (newState === 'processing') {
        setState('processing')
        setStreamingContent('')
      } else if (newState === 'done') {
        setAnswers((prev) => [
          ...prev,
          { id: prev.length, content: streamingRef.current }
        ])
        setCurrentIndex(answersLengthRef.current)
        setStreamingContent('')
        setState('idle')
      } else if (newState === 'error') {
        setState('error')
        setErrorMessage(error || 'Unknown error')
      }
    })

    window.api.onKeyEvent((key: string, shift: boolean, ctrl: boolean) => {
      if (!inputModeActiveRef.current) return

      if (ctrl) return

      if (key === 'Enter') {
        if (inputTextRef.current.trim()) {
          window.api.sendTextInput(inputTextRef.current)
          setInputText('')
          setInputModeActive(false)
          setState('processing')
          setStreamingContent('')
        } else {
          window.api.stopInputMode()
          setInputModeActive(false)
          setInputText('')
        }
        return
      }

      if (key === 'Escape') {
        window.api.stopInputMode()
        setInputModeActive(false)
        setInputText('')
        return
      }

      if (key === 'Backspace') {
        setInputText((prev) => prev.slice(0, -1))
        return
      }

      setInputText((prev) => prev + key)
    })

    window.api.onInputModeState((newState: string) => {
      if (newState === 'active') {
        setInputModeActive(true)
        setInputText('')
      } else if (newState === 'inactive' || newState === 'error') {
        setInputModeActive(false)
        setInputText('')
      }
    })
  }, [])

  const handlePrev = () => {
    if (currentIndex > 0) setCurrentIndex(currentIndex - 1)
  }

  const handleNext = () => {
    if (currentIndex < answers.length - 1) setCurrentIndex(currentIndex + 1)
  }

  const displayContent =
    state === 'streaming'
      ? streamingContent
      : answers[currentIndex]?.content || ''

  return (
    <div className={`overlay ${state === 'error' ? 'error' : ''} ${inputModeActive ? 'input-mode' : ''}`}>
      <TitleBar state={inputModeActive ? 'processing' : state} onClose={() => window.api.close()} />
      <AnswerPanel
        content={displayContent}
        state={state}
        errorMessage={errorMessage}
        dimmed={inputModeActive}
      />
      <TextInputBar text={inputText} isActive={inputModeActive} />
      {answers.length > 1 && state === 'idle' && !inputModeActive && (
        <div className="navigation">
          <button onClick={handlePrev} disabled={currentIndex === 0}>
            &#9664;
          </button>
          <span className="counter">
            {currentIndex + 1} / {answers.length}
          </span>
          <button onClick={handleNext} disabled={currentIndex === answers.length - 1}>
            &#9654;
          </button>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
cd copilotx
git add copilotx/src/renderer/src/App.tsx
git commit -m "feat(renderer): add input mode state, key event handling, TextInputBar rendering, dimmed answer panel"
```

---

### Task 20: Add input mode styles

**Files:**
- Modify: `copilotx/src/renderer/src/styles.css`

- [ ] **Step 1: Add input mode CSS**

Add these styles at the end of `copilotx/src/renderer/src/styles.css` (before the scrollbar styles, after the `@keyframes fadeIn` block):

```css
.answer-panel.dimmed {
  opacity: 0.5;
  transition: opacity 0.2s ease;
}

.text-input-bar {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  font-family: 'Cascadia Code', 'JetBrains Mono', 'Consolas', monospace;
  font-size: 13px;
  color: #e0e0e0;
  min-height: 32px;
}

.input-text {
  white-space: pre-wrap;
  word-wrap: break-word;
}

.input-cursor {
  animation: blink 1s step-end infinite;
  color: #e0e0e0;
  margin-left: 1px;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.overlay.input-mode .status-dot {
  background-color: #f0ad4e;
  animation: pulse 1s infinite;
}
```

- [ ] **Step 2: Commit**

```bash
cd copilotx
git add copilotx/src/renderer/src/styles.css
git commit -m "feat(renderer): add input mode styles — dimmed panel, text input bar, blinking cursor, amber status dot"
```

---

### Task 21: Full verification — typecheck, lint, and tests

**Files:**
- No file changes

- [ ] **Step 1: TypeScript typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: PASS

- [ ] **Step 2: ESLint**

Run: `cd copilotx && pnpm run lint`
Expected: PASS

- [ ] **Step 3: Vitest**

Run: `cd copilotx && pnpm run test`
Expected: PASS

- [ ] **Step 4: Rust tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: PASS

- [ ] **Step 5: Rust integration tests**

Run: `cd copilotx/sidecar && cargo test --test integration`
Expected: PASS

- [ ] **Step 6: Rust release build**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: PASS

---

## Self-Review Checklist

**1. Spec coverage:**

| Spec Requirement | Task |
|---|---|
| User presses Ctrl+Shift+K → input mode activates, keyboard hook registered | Task 12 (registerInputHotkey), Task 10 (start_keyboard_hook) |
| Keystrokes intercepted, mapped, forwarded as key_event NDJSON | Task 9 (keyboard.rs), Task 13 (SidecarMessage), Task 14 (onSidecarMessage) |
| Electron renders typed text in TextInputBar | Task 17, Task 19 |
| Enter → auto-screenshot + capture_with_text → LLM → hook unregistered → streaming | Task 19 (Enter handler), Task 10 (CaptureWithText cleans up hook), Task 3 (user_text param) |
| Escape → text cleared, hook unregistered, overlay returns | Task 19 (Escape handler), Task 10 (StopInputMode) |
| Empty text + Enter → exit input mode without sending | Task 19 (empty trim check) |
| Answer panel dims during input mode | Task 18, Task 19, Task 20 |
| Status dot amber blink during input mode | Task 19 (TitleBar state), Task 20 (CSS) |
| Writer thread (Option C) | Task 2 |
| Duplicate print_message removed | Task 2, Task 3 |
| Inactivity timeout 2.5s | Task 9 |
| Key swallowing (selective) | Task 9 (is_input_key, keyboard_hook_callback) |
| inputHotkey configurable | Task 5 (Rust), Task 11 (TS), Task 12 (registerInputHotkey) |
| Preload bridge extensions | Task 15 |
| Integration tests | Task 6, Task 16 |

**2. Placeholder scan:** No TBD, TODO, "implement later", "add appropriate error handling", "similar to Task N", or missing code blocks found.

**3. Type consistency:**
- `Sender<Message>` used consistently across `main.rs`, `llm.rs`, `keyboard.rs`
- `HookHandle` struct fields (`thread_id: u32`, `join_handle: Option<JoinHandle<()>>`, `stop_flag: Arc<AtomicBool>`) consistent between definition (Task 9) and usage (Task 10)
- `SidecarMessage` interface fields (`key`, `shift`, `ctrl`, `state`) match between `ipc.ts` (Task 13), `index.ts` handler (Task 14), and `preload/index.ts` (Task 15)
- `AppConfig.inputHotkey: string` consistent across `config.ts` (Task 11) and test fixture (Task 16)
- `Message::KeyEvent { key, shift, ctrl }` field names consistent between `protocol.rs` (Task 1) and `keyboard.rs` (Task 9)
- `Message::InputModeState { state }` field name consistent between `protocol.rs` (Task 1), `main.rs` (Task 10), `keyboard.rs` (Task 9), `ipc.ts` (Task 13), `index.ts` (Task 14), `preload/index.ts` (Task 15)
