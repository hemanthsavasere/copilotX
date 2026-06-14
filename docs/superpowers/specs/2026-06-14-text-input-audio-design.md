# Text Input Bar & Audio Capture Design

**Date:** 2026-06-14  
**Status:** Draft  
**Scope:** Add an "Ask anything" text input bar and real-time audio capture with Gemini Live STT to the CopilotX overlay

## Context

CopilotX is an Electron + Rust sidecar desktop app that captures screenshots via hotkey, sends them to OpenAI/Anthropic LLMs, and streams AI responses in a stealth overlay. Currently it has:

- Screenshot capture via hotkey
- LLM streaming (OpenAI GPT-4o, Anthropic Claude)
- A read-only overlay showing AI answers
- No text input and no audio capability

This spec adds two features:
1. A persistent text input bar at the bottom of the overlay
2. Real-time audio capture with speech-to-text via Google Gemini Live API

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Input bar location | Bottom of existing overlay | Keeps everything in one window |
| Audio scope | Real-time auto-listen with configurable source (system/mic/both) | Matches cheating-daddy's architecture; maximum audio context |
| STT provider | Google Gemini Live API | Speaker diarization, high quality, matches cheating-daddy |
| Architecture | All audio capture + STT in Rust sidecar | Consistent with existing pattern (all I/O in sidecar), renderer stays thin, no browser permission prompts |
| Platform support | Windows only | Simplifies audio capture (WASAPI loopback), removes SystemAudioDump dependency |
| LLM flow | STT text feeds into existing OpenAI/Anthropic pipeline | Gemini for transcription only, responses still from configured LLM |

## 1. Text Input Bar

### Layout

The input bar sits at the bottom of the overlay, always visible:

```
┌─────────────────────────────────┐
│ TitleBar  [status] CopilotX  ✕  │
├─────────────────────────────────┤
│                                 │
│        AnswerPanel              │
│   (streaming/completed          │
│    AI responses)                │
│                                 │
├─────────────────────────────────┤
│ [🎤] Ask anything...    [➤]    │
└─────────────────────────────────┘
```

### Components

**TextInputBar** (new React component):
- Text input field with placeholder "Ask anything..."
- Send button (arrow icon) on the right
- Mic toggle button (icon) on the left
- Enter key submits text
- Input is cleared after submission

**Mic button states:**
- Inactive: muted mic icon, default border
- Active: highlighted mic icon with red pulse animation, real-time transcription preview shown in/at the input bar

**Interaction:**
- Text submit while audio is active: stops recording and submits captured transcription + typed text
- Text submit without audio: sends typed text only
- Enter key submits; input clears after send

### New React state

| State | Type | Purpose |
|-------|------|---------|
| `audioActive` | `boolean` | Whether audio capture is running |
| `liveTranscription` | `string` | Real-time transcript shown in input area |
| `textInputValue` | `string` | Current text input content |

## 2. Audio Capture Pipeline

### Architecture

All audio capture and Gemini Live STT runs in the Rust sidecar. Audio is captured via `cpal` (WASAPI on Windows), streamed to Gemini Live for transcription, and transcribed text is sent back to Electron via the existing NDJSON-over-stdio IPC channel.

### Audio format

- PCM 16-bit signed integer, little-endian
- 24000 Hz sample rate
- Mono (1 channel)
- Chunk size: 2400 samples per 100ms → 4800 bytes per chunk

### Windows audio capture

| Audio Mode | Capture Method |
|------------|---------------|
| System audio | `cpal` with WASAPI loopback (captures what plays through speakers) |
| Microphone | `cpal` default input device |
| Both | Two concurrent `cpal` streams merged with speaker labels |

### New sidecar modules

| Module | Responsibility |
|--------|---------------|
| `audio.rs` | Platform-specific audio capture using `cpal`. Enumerates input devices, captures PCM audio at 24kHz/16-bit/mono. Supports `system`, `mic`, and `both` modes. On Windows, uses WASAPI loopback for system audio. |
| `stt.rs` | Gemini Live API client. Opens a WebSocket to `generativelanguage.googleapis.com`, streams audio chunks, receives transcription responses with speaker diarization. Handles session lifecycle (connect, stream, disconnect). |
| `audio_pipeline.rs` | Orchestrates audio capture → STT → text flow. Buffers transcribed text, emits NDJSON messages back to Electron. |

### New Cargo dependencies

- `cpal` — Cross-platform audio I/O (WASAPI on Windows)
- `tokio-tungstenite` — Async WebSocket client for Gemini Live
- `base64` — Audio chunk encoding
- `serde_json` — Already present; used for Gemini Live message framing

## 3. IPC Protocol Extensions

### New Commands (Electron → Sidecar)

```json
{ "type": "start_audio", "mode": "system" }
{ "type": "start_audio", "mode": "mic" }
{ "type": "start_audio", "mode": "both" }
{ "type": "stop_audio" }
{ "type": "text_input", "content": "What does this error mean?" }
```

### New Messages (Sidecar → Electron)

```json
{ "type": "transcription", "content": "Speaker 1: Can you explain this function?" }
{ "type": "audio_state", "state": "capturing" }
{ "type": "audio_state", "state": "stopped" }
{ "type": "audio_state", "state": "error", "error": "No system audio device available" }
```

### Preload bridge additions

| Method | Direction | Channel | Purpose |
|--------|-----------|---------|---------|
| `startAudio(mode)` | Renderer → Main | `ipcRenderer.invoke('start-audio', mode)` | Start audio capture |
| `stopAudio()` | Renderer → Main | `ipcRenderer.invoke('stop-audio')` | Stop audio capture |
| `sendTextInput(text)` | Renderer → Main | `ipcRenderer.invoke('send-text-input', text)` | Send typed text |
| `onTranscription(cb)` | Main → Renderer | `ipcRenderer.on('transcription')` | Receive transcription updates |
| `onAudioState(cb)` | Main → Renderer | `ipcRenderer.on('audio-state')` | Receive audio state changes |

## 4. LLM Integration

### Context combination

The LLM now receives three possible input types combined into a single prompt:

| Input Type | Source | When |
|-----------|--------|------|
| Screenshot | `xcap` screen capture | Hotkey press |
| Transcription | Gemini Live STT | When audio is active |
| Text prompt | User typing | Enter key or send button |

### Prompt construction

The system prompt (from `profiles.rs`) is sent first, followed by whichever context blocks are available:

If screenshot is present:
```xml
<image>data:image/png;base64,{base64_screenshot}</image>
```

If transcription is present:
```xml
<transcription>
Speaker 1: Can you explain this function?
Speaker 2: It's a recursive descent parser...
</transcription>
```

If text input is present:
```xml
<user_question>What does this error mean?</user_question>
```

### Capture scenarios

| Trigger | Audio active? | What gets sent to LLM |
|---------|--------------|----------------------|
| Hotkey | No | Screenshot only (current behavior) |
| Hotkey | Yes | Screenshot + accumulated transcription |
| Text submit | No | Text only, no screenshot |
| Text submit | Yes | Text + accumulated transcription, no screenshot |
| Text submit (with `includeScreenshotWithText=true`) | Either | Screenshot + text + transcription (if audio active) |

### Sidecar state additions

- `transcription_buffer: String` — accumulated transcription while audio is active
- When LLM call is triggered, all available context (screenshot, transcription buffer, text input) is included
- After LLM call completes, the transcription buffer is **not** cleared (audio continues capturing; only cleared when audio stops)

## 5. Config Extensions

New fields added to `AppConfig` (TypeScript) and `Config` (Rust):

```typescript
interface AppConfig {
  // ... existing fields ...
  audioMode: "system" | "mic" | "both";
  geminiApiKey: string;
  audioEnabled: boolean;
  textInputEnabled: boolean;
  includeScreenshotWithText: boolean;
}
```

**Validation rules:**
- `audioMode` must be one of `system`, `mic`, `both`
- When `audioEnabled` is `true`, `geminiApiKey` must be non-empty
- `audioEnabled` and `textInputEnabled` are booleans
- `includeScreenshotWithText` is a boolean

**Overlay dimension adjustment:**
- Minimum overlay height raised from 200px to 280px to accommodate the input bar (~48px)

## 6. Error Handling

| Scenario | Handling |
|----------|----------|
| Gemini API key missing when audio is toggled | Show error in input bar: "Set your Gemini API key in config to enable audio" |
| Gemini WebSocket connection fails | `audio_state: error` message, mic button shows error state, auto-retry up to 3 times with exponential backoff |
| Audio device not found | `audio_state: error` message with specific text ("No system audio device available") |
| STT returns empty/unclear transcription | Silently ignore, don't block LLM submission — user can still type and submit |
| LLM call triggered with no context | Don't send empty request, show brief "Enter a question or capture a screen" message |

## 7. Testing Strategy

| Layer | Test Type | What to test |
|-------|-----------|-------------|
| `protocol.rs` | Unit | New NDJSON message types serialize/deserialize correctly (`start_audio`, `stop_audio`, `text_input`, `transcription`, `audio_state`) |
| `audio.rs` | Unit | Audio format conversion (stereo→mono, resampling if needed) |
| `stt.rs` | Unit | Gemini Live WebSocket message framing |
| `audio_pipeline.rs` | Integration (mock) | Audio → transcription → NDJSON output flow |
| `config.rs` / `config.ts` | Unit | New config fields validation (`audioMode`, `geminiApiKey`, `audioEnabled`, `textInputEnabled`) |
| Preload bridge | Unit | New IPC channels (`start-audio`, `stop-audio`, `send-text-input`, `transcription`, `audio-state`) |
| React components | Component tests | TextInputBar state transitions, mic toggle, transcription display |
| E2E | Manual | Full flow: text input, audio capture, combined capture+audio |

## 8. Files to Create/Modify

### New files

| File | Purpose |
|------|---------|
| `copilotx/src/renderer/src/TextInputBar.tsx` | Text input bar React component |
| `copilotx/sidecar/src/audio.rs` | Windows audio capture via `cpal` + WASAPI |
| `copilotx/sidecar/src/stt.rs` | Gemini Live WebSocket STT client |
| `copilotx/sidecar/src/audio_pipeline.rs` | Audio → STT orchestration |

### Modified files

| File | Changes |
|------|---------|
| `copilotx/sidecar/src/protocol.rs` | Add `StartAudio`, `StopAudio`, `TextInput`, `Transcription`, `AudioState` message types |
| `copilotx/sidecar/src/main.rs` | Handle new command types, wire up audio pipeline |
| `copilotx/sidecar/src/llm.rs` | Accept transcription + text alongside screenshot in prompt construction |
| `copilotx/sidecar/src/config.rs` | Add `audioMode`, `geminiApiKey`, `audioEnabled`, `textInputEnabled`, `includeScreenshotWithText` |
| `copilotx/sidecar/Cargo.toml` | Add `cpal`, `tokio-tungstenite` dependencies |
| `copilotx/src/main/config.ts` | Add new config fields with validation |
| `copilotx/src/main/index.ts` | Handle new IPC channels (`start-audio`, `stop-audio`, `send-text-input`, forward `transcription` and `audio-state`) |
| `copilotx/src/main/ipc.ts` | Forward audio commands to sidecar |
| `copilotx/src/main/hotkey.ts` | Update capture trigger to include available transcription context |
| `copilotx/src/preload/index.ts` | Add `startAudio`, `stopAudio`, `sendTextInput`, `onTranscription`, `onAudioState` bridge methods |
| `copilotx/src/preload/index.d.ts` | TypeScript declarations for new bridge methods |
| `copilotx/src/renderer/src/App.tsx` | Add `audioActive`, `liveTranscription`, `textInputValue` state; wire TextInputBar; handle new IPC events |
| `copilotx/src/renderer/src/AnswerPanel.tsx` | Adjust layout for reduced height |
| `copilotx/src/renderer/src/styles.css` | Styles for TextInputBar, mic button states, transcription preview |

## Out of scope

- macOS and Linux audio capture (Windows only)
- Gemini Live for LLM response generation (used for STT only)
- Settings UI (config is still JSON-only)
- Region-select capture
- Runtime model switching
- Audio recording/playback of past sessions