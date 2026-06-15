# Stealth Text Input Bar Design

**Date:** 2026-06-15  
**Status:** Approved  
**Supersedes:** 2026-06-14-text-input-audio-design.md

## Context

CopilotX is an Electron + Rust sidecar desktop app that captures screenshots via hotkey, sends them to OpenAI/Anthropic LLMs, and streams AI responses in a stealth overlay. The overlay is `focusable: false` with Win32 `WS_EX_NOACTIVATE` flags — it never receives OS-level keyboard focus.

Currently there is no way to type questions to the LLM. This spec adds a stealth text input bar that preserves the overlay's zero-focus architecture.

**Core constraint:** The overlay BrowserWindow must remain `focusable: false` at all times. No compromise to stealth.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Input mode activation | Dedicated configurable hotkey (default `Ctrl+Shift+K`) | No click interaction needed — overlay stays unfocused |
| Input mode exit | Auto-exit on Enter (send) or Escape (cancel) | No toggle-to-remember; predictable |
| Keystroke capture | Low-level keyboard hook in Rust sidecar (`WH_KEYBOARD_LL`) | Consistent with "all I/O in sidecar" architecture; hook runs in `system-helper` process |
| Keystroke swallowing | Yes — intercepted keys do not pass through to other apps | Prevents typed text from appearing in interview app |
| Visual design | Blinking cursor + typed text at bottom, no border, no send button, same font/color as AI answers | Indistinguishable from streaming state to an observer |
| Send action | Enter only, no visible send button | Maximum stealth |
| LLM context on send | Text + auto-screenshot | Full context in one action |
| Platform | Windows only | Simplifies keyboard hook (Win32 API only) |
| IME support | Not supported (known limitation) | Raw key events cannot compose CJK characters; future enhancement |

## 1. Input Mode Flow

1. User presses `inputHotkey` (default `Ctrl+Shift+K`) — intercepted by Electron `globalShortcut`
2. Electron sends `{ "type": "start_input_mode" }` to Rust sidecar via NDJSON stdin
3. Rust sidecar registers `WH_KEYBOARD_LL` keyboard hook
4. Every keystroke is intercepted by Rust, mapped to character, forwarded to Electron as `{ "type": "key_event", "key": "a", "shift": false, "ctrl": false }`
5. Electron renderer displays characters in TextInputBar — blinking cursor + typed text, no border
6. **Enter** → Rust sends `key_event: Enter` → Electron triggers auto-screenshot + sends text via `capture_with_text` command → Rust unregisters hook
7. **Escape** → Rust sends `key_event: Escape` → Electron clears text, sends `stop_input_mode` → Rust unregisters hook

## 2. Overlay Visual Behavior

### Normal state (no input mode)

- Overlay shows answer panel as-is (idle/processing/streaming/error)
- No input bar visible

### Input mode active

- Thin text area appears at bottom of overlay
- Blinking cursor (`|`) and typed text — same font, same color as AI answer text
- No border, no background, no placeholder text, no send button
- Answer panel above dims slightly (reduced opacity) to differentiate
- Status dot changes to subtle amber blink (same as "processing" — reinforces "AI is working" illusion)

### On Enter (send)

- Text bar disappears immediately
- Overlay transitions to `processing` state (screen capture triggered)
- Then `streaming` state (AI response flows in) — normal flow resumes

### On Escape (cancel)

- Text clears, text bar disappears
- Status dot returns to previous state
- Overlay returns to whatever state it was in before input mode

## 3. IPC Protocol Extensions

### New Commands (Electron → Sidecar)

```json
{ "type": "start_input_mode" }
{ "type": "stop_input_mode" }
{ "type": "capture_with_text", "content": "What does this error mean?" }
```

### New Messages (Sidecar → Electron)

```json
{ "type": "key_event", "key": "a", "shift": false, "ctrl": false }
{ "type": "key_event", "key": "Enter" }
{ "type": "key_event", "key": "Escape" }
{ "type": "input_mode_state", "state": "active" }
{ "type": "input_mode_state", "state": "inactive" }
{ "type": "input_mode_state", "state": "error" }
```

### Key event handling rules in Electron

- Printable characters → append to input text
- `Backspace` → delete last character
- `Enter` → send (auto-screenshot + text via `capture_with_text`)
- `Escape` → cancel (clear text, send `stop_input_mode`)
- Other modifier combos → ignored

### Preload bridge additions

| Method | Direction | Channel | Purpose |
|--------|-----------|---------|---------|
| `onKeyEvent(cb)` | Main → Renderer | `ipcRenderer.on('key-event')` | Receive keystrokes |
| `onInputModeState(cb)` | Main → Renderer | `ipcRenderer.on('input-mode-state')` | Receive input mode state changes |
| `sendTextInput(text)` | Renderer → Main | `ipcRenderer.invoke('send-text-input', text)` | Send typed text with auto-screenshot |

## 4. Rust Sidecar — Keyboard Hook Module

**New module: `keyboard.rs`**

### Responsibilities

- Register/unregister `WH_KEYBOARD_LL` via Win32 `SetWindowsHookExW`
- Map virtual key codes to characters (respecting Shift state)
- Forward key events to Electron via NDJSON stdout
- **Swallow keystrokes** — intercepted events do not propagate to other applications

### Hook behavior

- `start_input_mode` → register hook, send `input_mode_state: active`
- `stop_input_mode` → unregister hook, send `input_mode_state: inactive`
- Hook callback: intercept key, map to character, emit `key_event`, return value blocks propagation
- `Enter` and `Escape` are intercepted (not passed through)

### New Cargo dependency

| Crate | Purpose |
|-------|---------|
| `windows` | Win32 API for `SetWindowsHookExW`, `WH_KEYBOARD_LL`, virtual key mapping |

## 5. LLM Integration

### `capture_with_text` command handling

When Electron sends `capture_with_text`, the sidecar:

1. Captures screenshot via `xcap` (same as existing `capture` flow)
2. Constructs prompt with both screenshot and text:

```xml
<image>data:image/png;base64,{base64_screenshot}</image>
<user_question>What does this error mean?</user_question>
```

3. Sends to configured LLM (OpenAI/Anthropic) — same streaming pipeline
4. Tokens flow back to Electron as normal `token` messages

## 6. Config Extensions

**New field in `config.json` / `AppConfig`:**

```typescript
interface AppConfig {
  // ... existing fields ...
  inputHotkey: string;  // default: "Ctrl+Shift+K"
}
```

**Validation:** `inputHotkey` must be a valid Electron `globalShortcut` accelerator string. If empty or invalid, fallback to `Ctrl+Shift+K`.

## 7. Error Handling

| Scenario | Handling |
|----------|----------|
| Keyboard hook registration fails | Sidecar sends `input_mode_state: error`, Electron shows brief red flash on status dot, input mode does not activate |
| Hook callback errors | Log in sidecar, continue running — hook stays active |
| User presses Enter with empty text | Don't send `capture_with_text`, just exit input mode. Show brief amber flash on status dot |
| Screenshot capture fails during `capture_with_text` | Sidecar sends `error` message as normal, overlay shows error state |
| Input hotkey conflict (already registered by another app) | Electron `globalShortcut.register()` returns false — log warning on startup |
| User triggers normal capture hotkey while in input mode | Ignore the capture hotkey — input mode takes priority |

## 8. Files to Create/Modify

### New files

| File | Purpose |
|------|---------|
| `copilotx/sidecar/src/keyboard.rs` | Win32 keyboard hook — register/unregister, key mapping, event forwarding, key swallowing |
| `copilotx/src/renderer/src/TextInputBar.tsx` | React component — blinking cursor, typed text display, Enter/Escape handling |

### Modified files

| File | Changes |
|------|---------|
| `copilotx/sidecar/src/protocol.rs` | Add `StartInputMode`, `StopInputMode`, `CaptureWithText`, `KeyEvent`, `InputModeState` message types |
| `copilotx/sidecar/src/main.rs` | Handle new command types, wire up keyboard module |
| `copilotx/sidecar/src/llm.rs` | Accept text input alongside screenshot in prompt construction |
| `copilotx/sidecar/Cargo.toml` | Add `windows` crate dependency |
| `copilotx/src/main/config.ts` | Add `inputHotkey` field with validation |
| `copilotx/src/main/index.ts` | Register input hotkey, handle IPC for input mode, forward key events to renderer |
| `copilotx/src/main/hotkey.ts` | Ignore capture hotkey when input mode is active |
| `copilotx/src/preload/index.ts` | Add `onKeyEvent`, `onInputModeState`, `sendTextInput` bridge methods |
| `copilotx/src/preload/index.d.ts` | TypeScript declarations for new bridge methods |
| `copilotx/src/renderer/src/App.tsx` | Add `inputMode`, `inputText` state; wire TextInputBar; handle key events |
| `copilotx/src/renderer/src/styles.css` | Styles for TextInputBar (blinking cursor, same font/color as answers) |
| `copilotx/config/config.json` | Add `inputHotkey: "Ctrl+Shift+K"` default |
| `copilotx/config/schemas/config.schema.json` | Add `inputHotkey` schema definition |

## Known Limitations

- **IME/composite input (CJK languages) not supported** — raw key events cannot produce composed characters. Future enhancement.
- **Windows only** — keyboard hook uses Win32 API. Linux/macOS not supported.
- **No clipboard** — `Ctrl+C`/`Ctrl+V` are not handled to avoid complexity. Input is character-by-character only.

## Out of Scope

- Audio capture / mic button (was in superseded spec)
- IME support for non-Latin input
- Linux/macOS keyboard hooks
- Settings UI (config is still JSON-only)
- Text selection, clipboard, or rich text editing
