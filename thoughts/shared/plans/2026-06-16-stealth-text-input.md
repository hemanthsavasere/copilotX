# Stealth Text Input Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Overview

Add a stealth text input bar to the CopilotX overlay that lets users type questions to the LLM without the overlay ever receiving OS-level keyboard focus. Keystrokes are captured by a Win32 low-level keyboard hook in the Rust sidecar, forwarded to Electron via NDJSON, and rendered in the overlay. The overlay remains `focusable: false` at all times.

**Architecture choice:** Option C (dedicated writer thread with `mpsc::channel`) — all stdout writes go through a single writer thread, eliminating `print_message` duplication and race conditions. The keyboard hook thread sends `key_event` messages through the same channel.

## Current State Analysis

- **Sidecar** uses a synchronous stdin loop (`for line in stdin.lock().lines()`) with a local `print_message` function that writes to stdout directly. There are duplicate `print_message` functions in `main.rs:12-17` and `llm.rs:6-11`.
- **IPC** is NDJSON-over-stdio with 4 command types (`ping`, `capture`, `stop`, `shutdown`) and 4 message types (`pong`, `token`, `done`, `error`). All use `#[serde(tag = "type")]` internally-tagged enum format.
- **Electron main process** has no input mode state — the overlay has `OverlayState` of `idle | processing | streaming | error`.
- **Preload bridge** exposes `onToken`, `onCaptureState`, `triggerCapture`, `close`.
- **Renderer** has `useEffect`-based state management with refs for stale-closure workarounds.
- **Config** has no `inputHotkey` field.

### Key Discoveries

- `sidecar/src/main.rs:44` — Synchronous blocking stdin loop; cannot poll channels concurrently without Option C's writer thread architecture.
- `sidecar/src/llm.rs:6-11` — Duplicate `print_message`; both copies write directly to `stdout`. This is eliminated by Option C.
- `sidecar/src/protocol.rs:3-27` — Internally-tagged serde enums; new variants follow the same pattern exactly.
- `src/main/hotkey.ts:8-11` — `isProcessing` guard has no `isInputMode` counterpart yet.
- `src/main/index.ts:53-57` — `trigger-capture` handler bypasses `isProcessing` guard; new `sendTextInput` should not bypass it.
- `WH_KEYBOARD_LL` requires a Windows message pump on the installing thread; the sidecar's stdin loop is synchronous so the hook **must** run on a separate thread.
- The `windows` crate is the standard Rust binding for Win32 API (Microsoft-maintained, idiomatic, feature-gated).

## Desired End State

### Key Behaviors

1. User presses `Ctrl+Shift+K` (configurable) → input mode activates, keyboard hook registered in sidecar
2. Keystrokes are intercepted (swallowed), mapped to characters, forwarded to Electron as `key_event` NDJSON messages
3. Electron renders typed text in a TextInputBar at the bottom of the overlay (blinking cursor, same font/color as answers)
4. Enter → auto-screenshot + `capture_with_text` → LLM processes text+image → hook unregistered → overlay transitions to streaming
5. Escape → text cleared, input mode deactivated, hook unregistered → overlay returns to previous state
6. Empty text + Enter → exit input mode without sending (brief amber flash on status dot)
7. Answer panel dims during input mode; status dot changes to amber blink

### Verification

- Full end-to-end: press input hotkey → type text → press Enter → see streaming response containing context from both screenshot and typed question
- Keystroke swallowing: open Notepad, activate input mode, type text → nothing appears in Notepad
- Escape: activate input mode → press Escape → text cleared, overlay returns to previous state
- Error recovery: kill sidecar during input mode → Windows auto-unregisters hook, keys passthrough again
- Config: change `inputHotkey` in `config.json` → new hotkey works on next launch

## What We're NOT Doing

- IME/composite input (CJK languages) — raw key events cannot compose characters
- Audio capture / mic button (superseded by this spec)
- Linux/macOS keyboard hooks (Win32 API only)
- Settings UI (config is still JSON-only)
- Text selection, clipboard operations (Ctrl+C/V), or rich text editing
- Auto-unregister after prolonged inactivity (2.5-second timeout is included as a safety net — see Phase 2)

## Implementation Approach

**Threading model (Option C):**

```
┌──────────────────┐         ┌──────────────────┐
│   Main Thread     │         │   Hook Thread      │
│                    │         │                    │
│  stdin loop        │  spawn  │  WH_KEYBOARD_LL   │
│  ├─ start_input_mode┼───────►│  MsgWaitForMulti  │
│  │  → send on tx   │         │  pleObjectsEx     │
│  ├─ stop_input_mode│         │  ├─ key callback: │
│  │  → post WM_QUIT │         │  │  map vk→char   │
│  ├─ capture_with_  │         │  │  send on tx     │
│  │  text           │         │  │  reset timer    │
│  ├─ capture/other  │         │  └─ on timeout/    │
│  └─ LLM tokens     │         │     stop: unreg   │
│     → send on tx   │         │  & send inactive  │
└──────────┬──────────┘         └────────┬──────────┘
           │                              │
           │  mpsc::channel (unbounded)    │
           ▼                              ▼
    ┌──────────────────────────────────────────┐
    │          Writer Thread                     │
    │  for msg in rx {                           │
    │    writeln!(writer, "{}", msg.to_ndjson()) │
    │    writer.flush();                         │
    │  }                                         │
    └────────────────────────────────────────────┘

 Inactivity timeout: if no key event for 2.5s, hook thread auto-unregisters
 and sends InputModeState "inactive" through the channel
```

- **Writer thread** is spawned at startup, owns `stdout` exclusively, receives `Message` via `mpsc::channel`
- **Hook thread** is spawned on `start_input_mode`, receives a `Sender<Message>` clone, sends `KeyEvent` and `InputModeState` messages through the channel
- **Main thread** sends all `Message` variants through `Sender<Message>` instead of calling `print_message`
- Stop signaling: `Arc<AtomicBool>` stop flag + `PostThreadMessageW(thread_id, WM_QUIT, ...)`
- Hook callback accesses `Sender<Message>` via `thread_local!` — set before `SetWindowsHookExW`, read in callback
- **Inactivity timeout:** Hook thread uses `MsgWaitForMultipleObjectsEx` with a 2.5-second timeout. If no key events arrive for 2.5 seconds, the hook auto-unregisters and sends `InputModeState { state: "inactive" }`, preventing indefinite key swallowing

**Key swallowing behavior (selective):**
- Swallow: letters A-Z, digits 0-9, Space, punctuation keys, Backspace, Enter, Escape
- Pass through: modifier-only keys (Shift, Ctrl, Alt), function keys, arrow keys, system combos (Alt+Tab detected via `WM_SYSKEYDOWN`)

---

## Phase 1: Sidecar Protocol Extensions & Writer Thread

### Overview

Refactor `print_message` into a thread-safe writer thread pattern, extend the NDJSON protocol with new command and message types, and add `CaptureWithText` command handling. This phase is testable on any platform (the keyboard hook module is Phase 2, Windows-only).

### Changes Required

#### 1. Extend protocol types

**File**: `copilotx/sidecar/src/protocol.rs`

Add new Command and Message variants following the existing `#[serde(tag = "type")]` pattern:

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

Add tests for the new variants:

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
fn test_message_input_mode_state_error() {
    let msg = Message::InputModeState { state: "error".into() };
    assert_eq!(msg.to_ndjson(), r#"{"type":"input_mode_state","state":"error"}"#);
}
```

#### 2. Add writer thread to main.rs

**File**: `copilotx/sidecar/src/main.rs`

Add `std::sync::mpsc` channel creation and writer thread spawning at the top of `main()`. Replace all `print_message()` calls with `tx.send()`. Remove the local `print_message` and `print_error` functions — replace with a `send_error` helper that sends through the channel.

Key changes:

```rust
use std::sync::mpsc;

// Remove: fn print_message() and fn print_error()
// Add helper:
fn send_error(tx: &mpsc::Sender<Message>, message: &str) {
    tx.send(Message::Error { message: message.to_string() }).ok();
}

#[tokio::main]
async fn main() {
    let config = match config::Config::load() { /* ... unchanged ... */ };

    // Writer thread — sole owner of stdout
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
    let mut hook_handle: Option<keyboard::HookHandle> = None;
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        // ... parse Command ...
        match cmd {
            Command::Ping => { tx.send(Message::Pong).ok(); }
            Command::StartInputMode => {
                #[cfg(target_os = "windows")]
                {
                    let tx_clone = tx.clone();
                    match keyboard::start_keyboard_hook(tx_clone) {
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
                #[cfg(not(target_os = "windows"))]
                {
                    send_error(&tx, "Input mode not supported on this platform");
                    tx.send(Message::InputModeState { state: "error".into() }).ok();
                }
            }
            Command::StopInputMode => {
                if let Some(handle) = hook_handle.take() {
                    keyboard::stop_keyboard_hook(handle);
                }
                tx.send(Message::InputModeState { state: "inactive".into() }).ok();
            }
            Command::CaptureWithText { content } => {
                // Stop keyboard hook first
                if let Some(handle) = hook_handle.take() {
                    keyboard::stop_keyboard_hook(handle);
                }
                tx.send(Message::InputModeState { state: "inactive".into() }).ok();

                if is_processing.load(Ordering::SeqCst) {
                    send_error(&tx, "Already processing");
                    continue;
                }
                if content.trim().is_empty() {
                    // Empty text — don't send, just exit input mode
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
                    "gpt-4o" => llm::stream_openai(&tx, &config.openai_api_key, &system_prompt, &image_b64, Some(&content)).await,
                    "claude" | "claude-sonnet" => llm::stream_anthropic(&tx, &config.anthropic_api_key, &system_prompt, &image_b64, Some(&content)).await,
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    send_error(&tx, &format!("LLM error: {}", e));
                }
                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Capture => {
                // ... existing capture logic, but using tx.send() instead of print_message ...
            }
            Command::Stop => { is_processing.store(false, Ordering::SeqCst); }
            Command::Shutdown => break,
        }
    }
}
```

#### 3. Remove duplicate print_message from llm.rs and add user_text parameter

**File**: `copilotx/sidecar/src/llm.rs`

Remove the local `print_message` function. Change function signatures to accept `tx: &mpsc::Sender<Message>` as the first parameter. Add `user_text: Option<&str>` parameter. Replace all `print_message(&Message::Token{...})` with `tx.send(Message::Token{...}).ok()`, etc.

Updated signatures:

```rust
pub async fn stream_openai(
    tx: &mpsc::Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()> {
    // ...
    let prompt_text = user_text.unwrap_or("Analyze this screenshot and provide the answer.");
    // ... use prompt_text instead of hardcoded string ...
    // ... tx.send(Message::Token { content }) instead of print_message ...
    // ... tx.send(Message::Done).ok() instead of print_message(&Message::Done) ...
}

pub async fn stream_anthropic(
    tx: &mpsc::Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()> {
    // ...
    let prompt_text = user_text.unwrap_or("Analyze this screenshot and provide the answer.");
    // ... same pattern ...
}
```

For OpenAI, replace the hardcoded text at line 49 with `prompt_text`:

```rust
ChatCompletionRequestMessageContentPartTextArgs::default()
    .text(prompt_text)
    .build()?
```

For Anthropic, replace the hardcoded text at line 121 with `prompt_text`:

```rust
{
    "type": "text",
    "text": prompt_text
}
```

#### 4. Add placeholder keyboard.rs for non-Windows

**File**: `copilotx/sidecar/src/keyboard.rs` (NEW)

Create the module with a placeholder for non-Windows platforms:

```rust
use crate::protocol::Message;
use std::sync::mpsc::Sender;

pub struct HookHandle {
    // Platform-specific fields added in Phase 2
}

#[cfg(not(target_os = "windows"))]
pub fn start_keyboard_hook(_tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    anyhow::bail!("Input mode is not supported on this platform")
}

#[cfg(not(target_os = "windows"))]
pub fn stop_keyboard_hook(_handle: HookHandle) {
    // No-op on non-Windows
}
```

Add `mod keyboard;` to `copilotx/sidecar/src/main.rs`.

#### 5. Add `overlayHeight` to Rust config (preexisting gap)

**File**: `copilotx/sidecar/src/config.rs`

The Electron `AppConfig` has `overlayHeight` but the Rust `Config` struct is missing it. Add:

```rust
pub overlay_height: u32,  // after overlay_width
```

Also update the test helper `make_valid_config_json()` to include `"overlayHeight": 600`.

### Success Criteria

#### Automated Verification

- [ ] All Rust protocol tests pass: `cd copilotx/sidecar && cargo test`
- [ ] New Command variants deserialize correctly (start_input_mode, stop_input_mode, capture_with_text)
- [ ] New Message variants serialize correctly (key_event, input_mode_state)
- [ ] Sidecar compiles: `cd copilotx/sidecar && cargo build --release`
- [ ] Existing integration tests still pass: `cd copilotx/sidecar && cargo test --test integration`
- [ ] No `print_message` duplicate remains (grep confirms only the writer thread writes to stdout)
- [ ] TypeScript typecheck passes: `cd copilotx && pnpm run typecheck`

#### Manual Verification

- [ ] Sidecar responds to `{"type":"start_input_mode"}` with `{"type":"input_mode_state","state":"error"}` on Linux/macOS (expected — no keyboard hook)
- [ ] Sidecar responds to `{"type":"capture_with_text","content":"hello"}` with token/done messages (end-to-end capture+LLM still works)
- [ ] Sidecar responds to `{"type":"stop_input_mode"}` with `{"type":"input_mode_state","state":"inactive"}`

**Implementation Note:** After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the capture+LLM pipeline still works before proceeding to Phase 2.

---

## Phase 2: Keyboard Hook Module (Windows-Only)

### Overview

Implement the `WH_KEYBOARD_LL` keyboard hook in `keyboard.rs` for Windows. This module registers a low-level keyboard hook on a dedicated thread, maps virtual key codes to characters, forwards key events through the writer channel, and swallows keystrokes to prevent them from reaching foreground apps.

### Changes Required

#### 1. Add `windows` crate dependency

**File**: `copilotx/sidecar/Cargo.toml`

Add target-specific dependency:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_Input_KeyboardAndMouse",
    "Win32_System_Threading",
    "Win32_Foundation",
] }
```

Using target-specific dependency so it only compiles on Windows.

#### 2. Implement keyboard.rs for Windows

**File**: `copilotx/sidecar/src/keyboard.rs`

Replace the placeholder from Phase 1 with the full Windows implementation. Key implementation details:

**Thread-local storage** for hook callback access to `Sender<Message>` and `Arc<AtomicBool>` stop flag:

```rust
#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_TX: RefCell<Option<Sender<Message>>> = RefCell::new(None);
    static HOOK_STOP_FLAG: RefCell<Arc<AtomicBool>> = RefCell::new(Arc::new(AtomicBool::new(false)));
}
```

**`is_input_key` function** — determines which keys to swallow:

```rust
fn is_input_key(vk_code: u32) -> bool {
    // Letters A-Z (0x41-0x5A)
    // Digits 0-9 (0x30-0x39)
    // Space, Backspace, Enter, Escape
    // Punctuation keys: ;=,-./`[\]'
    // NOT: Modifier keys, function keys, arrow keys, system combos
}
```

**`map_key_event` function** — maps VK codes to `(key_string, shift, ctrl)`:

```rust
fn map_key_event(vk_code: u32, flags: u32) -> Option<(String, bool, bool)> {
    // Ignore key-up events (LLKHF_UP flag, bit 7 of flags)
    // Check Shift/Ctrl state via GetKeyState
    // Skip modifier-only presses (Shift, Ctrl, Alt)
    // Skip Ctrl+key combos (pass through)
    // Skip Alt+key combos (pass through)
    // Map VK codes to characters with Shift state
    // Special keys: Enter, Backspace, Escape, Space
}
```

**Hook callback** — `unsafe extern "system"` function:

```rust
unsafe extern "system" fn keyboard_hook_callback(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // If code < 0, call CallNextHookEx
    // Read KBDLLHOOKSTRUCT from lparam
    // Check stop_flag — if set, pass through
    // If is_input_key(vk_code):
    //   Forward key event through HOOK_TX channel
    //   Return LRESULT(1) to swallow
    // Else: call CallNextHookEx (pass through)
}
```

**Inactivity timeout safety net:**

The hook thread implements a **2.5-second inactivity timeout**. If no key events arrive for 2.5 seconds, the hook auto-unregisters and sends `input_mode_state: inactive` through the writer channel. This prevents keys from being swallowed indefinitely if the Electron↔sidecar IPC connection is disrupted. Normal use always ends with Enter or Escape within seconds.

The timeout is implemented using `MsgWaitForMultipleObjectsEx` with a calculated remaining timeout, replacing the infinite `GetMessageW` call. A `thread_local!` `Instant` tracks the last key event time — the hook callback resets it on each input key, and the message loop checks `elapsed()` before each wait.

```rust
const INACTIVITY_TIMEOUT_MS: u32 = 2500; // 2.5 seconds

#[cfg(target_os = "windows")]
thread_local! {
    static INACTIVITY_TIMER: RefCell<std::time::Instant> = RefCell::new(std::time::Instant::now());
}
```

In the hook callback, after forwarding a key event:
```rust
INACTIVITY_TIMER.with(|t| *t.borrow_mut() = std::time::Instant::now());
```

In the message loop (replacing `GetMessageW`):
```rust
loop {
    let remaining_ms = INACTIVITY_TIMEOUT_MS.saturating_sub(
        INACTIVITY_TIMER.with(|t| t.borrow().elapsed().as_millis() as u32)
    );

    let wait_result = unsafe {
        MsgWaitForMultipleObjectsEx(
            0,
            None,
            remaining_ms,
            QS_ALLEVENTS,
            MWMO_ALERTABLE,
        )
    };

    if wait_result == WAIT_TIMEOUT {
        // 2.5s inactivity — auto-unregister
        tx_clone.send(Message::InputModeState { state: "inactive".into() }).ok();
        unsafe { UnhookWindowsHookEx(hook).ok() };
        return; // Thread exits
    }

    // Process all pending messages
    while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
        if msg.message == WM_QUIT {
            unsafe { UnhookWindowsHookEx(hook).ok() };
            return;
        }
        unsafe { TranslateMessage(&msg); }
        unsafe { DispatchMessageW(&msg); }
    }
}
```

**`start_keyboard_hook` function**:

```rust
pub fn start_keyboard_hook(tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    let (thread_id_tx, thread_id_rx) = std::sync::mpsc::channel::<u32>();

    let join_handle = std::thread::spawn(move || {
        // Initialize thread-local INACTIVITY_TIMER to now
        // Set up thread-local TX and stop flag (HOOK_TX, HOOK_STOP_FLAG)
        // Register WH_KEYBOARD_LL via SetWindowsHookExW
        // Send thread_id via oneshot channel
        // Run MsgWaitForMultipleObjectsEx loop with inactivity timeout
        // On WM_QUIT: unhook and exit
        // On WAIT_TIMEOUT (inactivity): send InputModeState "inactive", unhook, exit
    });

    let thread_id = thread_id_rx.recv()?;
    if thread_id == 0 { /* hook registration failed */ }

    Ok(HookHandle { thread_id, join_handle: Some(join_handle), stop_flag: stop_flag_clone })
}
```

**`stop_keyboard_hook` function**:

```rust
pub fn stop_keyboard_hook(handle: HookHandle) {
    handle.stop_flag.store(true, Ordering::SeqCst);
    // Post WM_QUIT to hook thread
    unsafe { PostThreadMessageW(handle.thread_id, WM_QUIT, ...); }
    // Join thread
    if let Some(jh) = handle.join_handle { let _ = jh.join(); }
}
```

**`HookHandle` struct**:

```rust
pub struct HookHandle {
    pub thread_id: u32,
    pub join_handle: Option<std::thread::JoinHandle<()>>,
    pub stop_flag: Arc<AtomicBool>,
}
```

The full implementation should be placed behind `#[cfg(target_os = "windows")]` with the non-Windows stubs remaining in the `#[cfg(not(target_os = "windows"))]` block.

### Success Criteria

#### Automated Verification

- [ ] Sidecar compiles on Linux/macOS (non-Windows codepath): `cd copilotx/sidecar && cargo build`
- [ ] Sidecar compiles on Windows: `cd copilotx/sidecar && cargo build --release`
- [ ] All existing Rust tests pass: `cd copilotx/sidecar && cargo test`
- [ ] All existing integration tests pass: `cd copilotx/sidecar && cargo test --test integration`

#### Manual Verification (Windows only)

- [ ] Run sidecar, send `{"type":"start_input_mode"}` → receive `{"type":"input_mode_state","state":"active"}`
- [ ] Type in any text editor while input mode is active → keystrokes do NOT appear in the editor
- [ ] Verify key_event messages on stdout with correct character mappings
- [ ] Send `{"type":"stop_input_mode"}` → receive `{"type":"input_mode_state","state":"inactive"}`
- [ ] After stop_input_mode, keystrokes pass through normally again
- [ ] Press Enter in input mode → `{"type":"key_event","key":"Enter",...}` on stdout
- [ ] Press Escape in input mode → `{"type":"key_event","key":"Escape",...}` on stdout

**Implementation Note:** After completing this phase, pause for manual testing on Windows before proceeding to Phase 3.

---

## Phase 3: Electron Main Process & Preload Bridge

### Overview

Add the input hotkey, update the IPC layer to handle new message types, wire up input mode state management, and extend the preload bridge with new APIs for the renderer.

### Changes Required

#### 1. Add `inputHotkey` to config

**File**: `copilotx/src/main/config.ts`

Add `inputHotkey` to `AppConfig` interface:

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

Add validation in `validateConfig`:

```typescript
if (!config.inputHotkey) {
  errors.push('inputHotkey is required')
}
```

**File**: `copilotx/config/config.json`

Add field:

```json
{
  "hotkey": "Ctrl+Shift+Space",
  "inputHotkey": "Ctrl+Shift+K",
  "model": "gpt-4o",
  ...
}
```

**File**: `copilotx/config/schemas/config.schema.json`

Add `inputHotkey` property:

```json
"inputHotkey": {
  "type": "string",
  "description": "Electron accelerator string for the input mode hotkey",
  "default": "Ctrl+Shift+K"
}
```

**File**: `copilotx/sidecar/src/config.rs`

Add `input_hotkey` and `overlay_height` fields with serde defaults:

```rust
#[serde(default = "default_input_hotkey")]
pub input_hotkey: String,

#[serde(default)]
pub overlay_height: u32,

fn default_input_hotkey() -> String { "Ctrl+Shift+K".to_string() }
```

Also update the test helper `make_valid_config_json()` to include both fields.

#### 2. Add input mode state to hotkey module

**File**: `copilotx/src/main/hotkey.ts`

Add `isInputMode` flag and `registerInputHotkey` function:

```typescript
import { globalShortcut, BrowserWindow } from 'electron'
import { sendCapture, sendStartInputMode } from './ipc'

let isProcessing = false
let isInputMode = false

export function registerHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isInputMode) return  // Ignore capture hotkey when in input mode
    if (isProcessing) {
      window.webContents.send('capture-state', 'already-processing')
      return
    }
    isProcessing = true
    window.show()
    window.webContents.send('capture-state', 'processing')
    sendCapture()
  })
  if (!registered) console.error(`Failed to register hotkey: ${accelerator}`)
  return registered
}

export function registerInputHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isProcessing || isInputMode) return
    isInputMode = true
    sendStartInputMode()
    window.webContents.send('input-mode-state', 'active')
  })
  if (!registered) console.error(`Failed to register input hotkey: ${accelerator}`)
  return registered
}

export function setProcessingComplete(): void { isProcessing = false }
export function setInputModeActive(): void { isInputMode = true }
export function setInputModeInactive(): void { isInputMode = false }
export function isInInputMode(): boolean { return isInputMode }
export function unregisterAll(): void { globalShortcut.unregisterAll() }
```

#### 3. Extend IPC for new commands and messages

**File**: `copilotx/src/main/ipc.ts`

Update `SidecarMessage` interface:

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

Add new send functions:

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

Update `writeSidecar` signature to allow additional fields:

```typescript
function writeSidecar(msg: Record<string, unknown>): void {
  if (!sidecar?.stdin || sidecar.stdin.destroyed) return
  sidecar.stdin.write(JSON.stringify(msg) + '\n')
}
```

#### 4. Update main index.ts to handle new messages

**File**: `copilotx/src/main/index.ts`

Add imports for new functions:

```typescript
import {
  startSidecar, stopSidecar, onSidecarMessage, sendCapture,
  sendStartInputMode, sendStopInputMode, sendCaptureWithText
} from './ipc'
import {
  registerHotkey, setProcessingComplete, unregisterAll,
  registerInputHotkey, isInInputMode, setInputModeInactive
} from './hotkey'
```

Update `onSidecarMessage` handler:

```typescript
case 'key_event':
  if (isInInputMode()) {
    overlayWindow.webContents.send('key-event', msg.key, msg.shift, msg.ctrl)
  }
  break
case 'input_mode_state':
  if (msg.state === 'error') {
    overlayWindow.webContents.send('input-mode-state', 'error')
  }
  break
```

Register input hotkey:

```typescript
registerHotkey(config.hotkey, overlayWindow)
registerInputHotkey(config.inputHotkey, overlayWindow)
```

Add IPC handlers for renderer communication:

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

#### 5. Extend preload bridge

**File**: `copilotx/src/preload/index.ts`

Add new methods:

```typescript
onKeyEvent: (callback: (key: string, shift: boolean, ctrl: boolean) => void) =>
  ipcRenderer.on('key-event', (_event, key, shift, ctrl) => callback(key, shift, ctrl)),
onInputModeState: (callback: (state: string) => void) =>
  ipcRenderer.on('input-mode-state', (_event, state) => callback(state)),
sendTextInput: (text: string) => ipcRenderer.invoke('send-text-input', text),
stopInputMode: () => ipcRenderer.invoke('stop-input-mode'),
```

**File**: `copilotx/src/preload/index.d.ts`

Update `Window.api` interface:

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

### Success Criteria

#### Automated Verification

- [ ] TypeScript typecheck passes: `cd copilotx && pnpm run typecheck`
- [ ] ESLint passes: `cd copilotx && pnpm run lint`
- [ ] Existing unit tests pass: `cd copilotx && pnpm run test`
- [ ] Sidecar compiles with new config field: `cd copilotx/sidecar && cargo build`
- [ ] Rust config tests pass including `inputHotkey` deserialization: `cd copilotx/sidecar && cargo test config`

#### Manual Verification

- [ ] Press `Ctrl+Shift+K` → sidecar receives `start_input_mode` command, sends back `input_mode_state: active`
- [ ] Sidecar key events appear in Electron console as `key-event` IPC messages
- [ ] Press Escape → sidecar receives `stop_input_mode`, sends `input_mode_state: inactive`
- [ ] `capture_with_text` rounds through end-to-end: type text → Enter → screenshot → LLM response with question context

---

## Phase 4: Renderer — TextInputBar & Input Mode UI

### Overview

Add the `TextInputBar` React component, extend App.tsx with input mode state management, add CSS for the blinking cursor and dimmed answer panel, and wire the overlay states for input mode.

### Changes Required

#### 1. Create TextInputBar component

**File**: `copilotx/src/renderer/src/TextInputBar.tsx` (NEW)

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

#### 2. Update App.tsx for input mode state

**File**: `copilotx/src/renderer/src/App.tsx`

Add state and refs for input mode:

```tsx
const [inputModeActive, setInputModeActive] = useState(false)
const [inputText, setInputText] = useState('')
const inputModeActiveRef = useRef(inputModeActive)
inputModeActiveRef.current = inputModeActive
const inputTextRef = useRef(inputText)
inputTextRef.current = inputText
```

Add event listeners in `useEffect`:

```tsx
window.api.onKeyEvent((key: string, shift: boolean, ctrl: boolean) => {
  if (!inputModeActiveRef.current) return

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

  // Printable characters
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
```

Update JSX to include TextInputBar and wire state:

```tsx
<div className={`overlay ${state === 'error' ? 'error' : ''} ${inputModeActive ? 'input-mode' : ''}`}>
  <TitleBar state={inputModeActive ? 'processing' : state} onClose={() => window.api.close()} />
  <AnswerPanel content={displayContent} state={state} errorMessage={errorMessage} dimmed={inputModeActive} />
  <TextInputBar text={inputText} isActive={inputModeActive} />
  {/* Navigation only when idle and NOT in input mode */}
  {answers.length > 1 && state === 'idle' && !inputModeActive && (
    <div className="navigation">
      {/* ... existing navigation ... */}
    </div>
  )}
</div>
```

#### 3. Update AnswerPanel for dimmed state

**File**: `copilotx/src/renderer/src/AnswerPanel.tsx`

Add `dimmed` prop:

```tsx
interface AnswerPanelProps {
  content: string
  state: OverlayState
  errorMessage: string
  dimmed?: boolean
}

export function AnswerPanel({ content, state, errorMessage, dimmed }: AnswerPanelProps) {
  const dimmedClass = dimmed ? 'dimmed' : ''
  // Apply dimmedClass to the wrapper div's className
}
```

#### 4. Add input mode styles

**File**: `copilotx/src/renderer/src/styles.css`

Add:

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

### Success Criteria

#### Automated Verification

- [ ] TypeScript typecheck passes: `cd copilotx && pnpm run typecheck`
- [ ] ESLint passes: `cd copilotx && pnpm run lint`
- [ ] Existing unit tests pass: `cd copilotx && pnpm run test`

#### Manual Verification

- [ ] Press `Ctrl+Shift+K` → blinking cursor appears at bottom of overlay, answer panel dims, status dot turns amber
- [ ] Type text → characters appear next to blinking cursor
- [ ] Press Backspace → last character removed
- [ ] Press Escape → text cleared, input mode deactivated, answer panel returns to normal
- [ ] Type text + Enter → text bar disappears, overlay transitions to processing → streaming, AI response includes context from typed question
- [ ] Press Enter with empty text → input mode deactivates without sending, brief amber flash on status dot
- [ ] Visual: text input bar has same font and color as AI answer text, no border, no background, no placeholder text
- [ ] Visual: answer panel above dims to 50% opacity during input mode

---

## Phase 5: Integration Testing & Edge Cases

### Overview

End-to-end testing of the complete input mode flow, error handling, and edge cases.

### Changes Required

#### 1. Hotkey conflict logging

**File**: `copilotx/src/main/hotkey.ts`

Already handled — `registerInputHotkey` logs an error on failure. No additional changes needed.

#### 2. Prevent capture hotkey during input mode

Already handled in Phase 3 — `registerHotkey` callback checks `if (isInputMode) return`.

#### 3. Empty text Enter handling

Already handled in Phase 4 — the key event handler checks `if (inputTextRef.current.trim())`.

#### 4. Rust integration tests for new commands

**File**: `copilotx/sidecar/tests/integration.rs`

Update the config JSON in test helpers to include `inputHotkey` and `overlayHeight`. Add tests:

```rust
#[test]
fn test_start_stop_input_mode() {
    // Start input mode on non-Windows should return error state
    // ...
}

#[test]
fn test_capture_with_text() {
    // Send capture_with_text command, verify token/done flow
    // (Requires mock LLM or real API key)
    // ...
}
```

#### 5. Update Electron unit tests

**File**: `copilotx/src/main/__tests__/config.test.ts`

Add `inputHotkey` to valid config fixture and add validation test:

```typescript
it('returns error for empty inputHotkey', () => {
  const config = { ...validConfig, inputHotkey: '' }
  const errors = validateConfig(config)
  expect(errors).toContainEqual(expect.stringContaining('inputHotkey'))
})
```

#### 6. Update Rust config tests

**File**: `copilotx/sidecar/src/config.rs`

Update `make_valid_config_json()` to include `"inputHotkey": "Ctrl+Shift+K"` and `"overlayHeight": 600`.

### Success Criteria

#### Automated Verification

- [ ] All TypeScript tests pass: `cd copilotx && pnpm run test`
- [ ] All Rust tests pass: `cd copilotx/sidecar && cargo test`
- [ ] TypeScript typecheck passes: `cd copilotx && pnpm run typecheck`
- [ ] ESLint passes: `cd copilotx && pnpm run lint`
- [ ] Sidecar compiles: `cd copilotx/sidecar && cargo build`

#### Manual Verification (Windows)

- [ ] **End-to-end happy path:** Press `Ctrl+Shift+K` → type "What does this error mean?" → press Enter → overlay shows processing → shows streaming response that references typed question
- [ ] **Keystroke swallowing:** While Notepad is open in foreground, activate input mode, type text → nothing appears in Notepad
- [ ] **Keystroke passthrough:** While input mode is active, Alt+Tab still switches windows
- [ ] **Escape cancels:** Activate input mode, type some text, press Escape → text cleared, overlay returns to previous state
- [ ] **Empty text Enter:** Activate input mode, press Enter immediately → input mode exits, no capture initiated
- [ ] **Capture hotkey ignored during input mode:** While in input mode, press `Ctrl+Shift+Space` → nothing happens
- [ ] **Hook registration failure:** If hook fails, sidecar sends `input_mode_state: error`, renderer deactivates input mode
- [ ] **Sidecar crash recovery:** Kill sidecar process during input mode → Windows removes hook → keystrokes passthrough normally
- [ ] **Inactivity timeout:** Activate input mode, wait 2.5 seconds without typing → hook auto-unregisters, overlay exits input mode, keystrokes passthrough normally
- [ ] **Config hotkey change:** Change `inputHotkey` to `CmdOrControl+Shift+I` in config → restart app → new hotkey works
- [ ] **Visual consistency:** TextInputBar font and color match AI answer text, no border/background/placeholder/send button

---

## Testing Strategy

### Unit Tests

- **Protocol serialization/deserialization** (Rust): New Command and Message variants round-trip correctly through JSON
- **Config validation** (TypeScript + Rust): `inputHotkey` validation on both sides
- **Key mapping** (Rust): `map_key_event` produces correct characters for VK codes with/without Shift
- **Config loading** (Rust): Config with `inputHotkey` and `overlayHeight` deserializes correctly

### Integration Tests

- **Rust sidecar**: `start_input_mode` → `input_mode_state:active`; `stop_input_mode` → `input_mode_state:inactive`; `capture_with_text` with content → token/done flow
- **Electron IPC**: Verify new `SidecarMessage` types parse correctly; verify `writeSidecar` produces valid NDJSON for new commands

### Manual Testing Steps

1. Launch app on Windows
2. Press input hotkey → verify overlay shows blinking cursor and dimmed answer panel
3. Type "What does this error mean?" → verify text appears in input bar
4. Press Enter → verify overlay transitions to processing then streaming with contextual response
5. Press Escape while typing → verify text cleared and input mode exits
6. Open Notepad, activate input mode, type text → verify nothing appears in Notepad
7. Close and restart with different `inputHotkey` → verify new hotkey works

## Performance Considerations

- **Key event latency:** The hook callback → channel → writer thread → NDJSON → Electron path adds ~5-10ms latency per keystroke. This is imperceptible for typing.
- **Channel backpressure:** Using `mpsc::channel()` (unbounded) means no backpressure. This is fine because key events are low-frequency and the writer thread writes/flushes immediately.
- **Memory:** The hook thread has minimal memory overhead (single message queue entry at a time).
- **CPU:** `GetMessageW` is a blocking call — zero CPU when idle.

## Migration Notes

- **Config file:** Existing `config.json` files will need `inputHotkey` added. The Rust sidecar uses `#[serde(default)]` so missing fields default to `"Ctrl+Shift+K"`. The Electron side should handle missing `inputHotkey` gracefully with a fallback.
- **Sidecar binary:** The new build must be deployed alongside the Electron app; the `copy-sidecar.js` script handles this.

## References

- Spec: `docs/superpowers/specs/2026-06-15-stealth-text-input-design.md`
- Original design: `docs/superpowers/specs/2026-06-14-text-input-audio-design.md`
- MVP plan: `docs/superpowers/plans/2026-06-13-copilotx-mvp.md`
- Rust `windows` crate: https://microsoft.github.io/windows-docs-rs/
- `SetWindowsHookExW` docs: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw