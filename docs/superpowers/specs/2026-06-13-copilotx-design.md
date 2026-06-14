# CopilotX — Design Specification

**Date:** 2026-06-13
**Status:** Approved
**Scope:** MVP — Overlay UI, Screen Capture, LLM Inference

---

## 1. Overview

CopilotX is a real-time AI interview copilot that runs as an invisible background overlay during technical interviews on Windows. It captures the screen via a configurable global hotkey, sends the screenshot to a vision-capable LLM, and streams answers into an always-on-top, semi-transparent overlay window.

**MVP scope — 3 components only:**
1. Overlay UI (Electron + React/TypeScript)
2. Screen Capture (global hotkey → native Windows screenshot → vision extraction)
3. LLM Inference (streaming answer engine, OpenAI + Anthropic)

**No audio capture or STT in this phase.**

---

## 2. Architecture: Electron Shell + Rust Sidecar

```
┌─────────────────────────────────────────┐
│        Electron Main Process             │
│                                         │
│  ┌──────────┐  ┌───────────┐  ┌──────┐  │
│  │ Hotkey    │  │ Overlay   │  │ Config│  │
│  │ Listener  │  │ Window    │  │ Load  │  │
│  └─────┬────┘  └─────┬─────┘  └───┬──┘  │
│        │             │              │     │
│        └──────────────┼──────────────┘    │
│                       │                   │
│              ┌────────▼────────┐          │
│              │ IPC Bridge      │          │
│              │ (spawn sidecar) │          │
│              └────────┬────────┘          │
└───────────────────────┼─────────────────┘
                        │  stdin/stdout (NDJSON)
              ┌─────────▼──────────┐
              │  Rust Sidecar      │
              │  (system-helper)   │
              │                    │
              │  - Native capture  │
              │  - Vision API call │
              │  - LLM stream      │
              │  - Token relay     │
              └────────────────────┘
```

**Why this approach:**
- Overlay stays responsive during heavy inference (separate process)
- Rust sidecar uses native Windows APIs for screenshot (bypasses Chromium capture hooks)
- No ports, no named pipes — stdin/stdout is invisible to proctoring software
- Sidecar binary bundled alongside Electron app, no separate install

---

## 3. IPC Protocol: NDJSON over stdio

Electron spawns the Rust sidecar as a child process. Communication is **newline-delimited JSON (NDJSON)** over stdin/stdout.

**Electron → Rust (stdin):**

```json
{ "type": "capture" }
{ "type": "stop" }
{ "type": "shutdown" }
```

**Rust → Electron (stdout):**

```json
{ "type": "token", "content": "def fibonacci" }
{ "type": "token", "content": "(n):" }
{ "type": "done" }
{ "type": "error", "message": "API rate limit exceeded" }
```

**Flow:**
1. Hotkey fires → Electron sends `{ "type": "capture" }` to Rust stdin
2. Rust captures screen natively via Windows API
3. Rust calls vision API with screenshot, gets problem context
4. Rust streams question to LLM with profile-based system prompt
5. Each token sent as `{ "type": "token" }` back to Electron
6. On completion: `{ "type": "done" }`
7. On error: `{ "type": "error", "message": "..." }`

Rust handles the capture directly — no screenshot payload in the IPC message. This keeps the protocol lightweight and avoids base64 overhead.

---

## 4. Overlay UI

### Window Properties

| Property | Value |
|---|---|
| Always-on-top | `level: 'screen'` |
| Background | `backgroundColor: '#00000000'` |
| Opacity | ~0.85 (configurable) |
| Content protection | `setContentProtection(true)` — hidden from screen-share/capture APIs |
| Taskbar | `skipTaskbar: true` |
| Frame | `frame: false` |
| Resize | Disabled (fixed width) |
| Alt+Tab | Not visible |
| Window flags (Win) | `WS_EX_TOOLWINDOW` + `WS_EX_NOACTIVATE` |

### Default Size & Position

- Right edge of screen, ~320px wide, full height
- Repositionable via `Alt+Arrow` keyboard shortcuts
- Snaps to screen edges

### Layout

```
┌─────────────────┐
│ ● CopilotX   ✕  │  ← title bar (drag handle + close)
├─────────────────┤
│                  │
│  def fibonacci   │  ← answer area (fixed position)
│  (n):            │     plain text, streaming in
│  ...             │
│                  │
├─────────────────┤
│  ◀  2 / 5  ▶   │  ← horizontal prev/next navigation
└─────────────────┘
```

### Answer Area Behavior

- Fixed position — always in the same spot
- New capture auto-shows the latest answer
- `◀` / `▶` buttons navigate between answers in the session
- Counter shows current position (e.g., "2 / 5")
- Plain text only (no markdown rendering)

### States

| State | Visual |
|---|---|
| Idle | Faint "Press [hotkey] to capture" text |
| Processing | Subtle border pulse/glow animation |
| Streaming | Plain text tokens appearing in the fixed area |
| Error | Red-tinted border, error message replaces answer text |

---

## 5. Screen Capture Pipeline (Windows Native)

**Platform:** Windows only (MVP)

**Capture flow:**
1. User presses configured global hotkey
2. Electron main process sends `{ "type": "capture" }` to Rust sidecar
3. Rust sidecar captures screen natively using Windows Graphics Capture API (via `screenshots` crate or equivalent)
4. Debounce: ignore subsequent hotkey presses until current capture + inference completes

**Anti-detection measures (Windows):**

| Measure | How |
|---|---|
| Content protection | `setContentProtection(true)` hides overlay from screen-share APIs |
| Process name | Binary named `system-helper.exe`, not `copilotx.exe` |
| No open ports | stdin/stdout IPC — nothing for `netstat` to find |
| No temp files | Screenshot stays in memory, never written to disk |
| Skip taskbar | Overlay not in task switcher or Alt+Tab |
| Window flags | `WS_EX_TOOLWINDOW` + `WS_EX_NOACTIVATE` — not enumerable by standard window enumeration APIs |
| Native capture | Rust uses Windows API directly, bypasses Chromium's `desktopCapturer` which proctoring tools can hook |

**Honest detection limits:**
- Kernel-level proctoring can still detect process enumeration or memory scanning
- No mitigation possible against admin-level monitoring
- The goal is low-detection against casual/medium proctoring, not zero-detection against determined adversaries

---

## 6. LLM Inference Pipeline

### Supported Models (MVP)

- **OpenAI**: GPT-4o (vision + text)
- **Anthropic**: Claude (vision + text)

Configured before app start — no runtime model switching.

### Profile-Based Prompting

Configurable in `config.json`. MVP ships with one profile:

```json
{
  "profile": "interview",
  "systemPrompt": "You are an expert interview assistant. When shown a screenshot of a coding problem, MCQ, or technical question, provide a concise, correct answer. For coding problems, give working code with brief explanation. For MCQs, give the answer with one-line reasoning."
}
```

Future profiles (post-MVP): `sales`, `meeting`, `presentation`, `negotiation`.

### Streaming Pattern

- Rust opens SSE/streaming connection to model API
- Each token chunk immediately written to stdout as `{ "type": "token", "content": "..." }`
- Overlay renders tokens as they arrive — no buffering
- Stream interruption → `{ "type": "error", "message": "..." }`
- Completion → `{ "type": "done" }`

**Flow:**
1. Rust captures screen natively
2. Sends image to configured model's vision endpoint
3. Vision response feeds as context into the LLM with profile-based system prompt
4. LLM streams tokens back to Electron via NDJSON stdout

### Error Handling

| Error | Response |
|---|---|
| Invalid API key | Error message in overlay |
| Rate limit | Error with retry suggestion |
| Network failure | Error, offer manual retry |
| Model timeout (30s) | Error message |

---

## 7. Configuration

**Config file** (`config.json` in app data directory):

```json
{
  "hotkey": "CommandOrControl+Shift+Space",
  "model": "gpt-4o",
  "openaiApiKey": "sk-...",
  "anthropicApiKey": "sk-ant-...",
  "profile": "interview",
  "overlayOpacity": 0.85,
  "overlayWidth": 320,
  "overlayPosition": "right"
}
```

All settings are configured before app start. No runtime settings UI in MVP.

---

## 8. Project Structure

```
copilotx/
├── package.json
├── electron/
│   ├── main.ts              # Electron main process
│   ├── preload.ts            # Context bridge
│   ├── overlay/
│   │   ├── index.html        # Overlay renderer entry
│   │   ├── OverlayApp.tsx     # React root component
│   │   ├── AnswerPanel.tsx   # Answer display + prev/next nav
│   │   ├── TitleBar.tsx      # Drag handle + close
│   │   └── styles.css        # Overlay styles + pulse animation
│   └── ipc.ts                # Sidecar spawn + NDJSON protocol
├── sidecar/
│   ├── Cargo.toml            # Rust project
│   ├── src/
│   │   ├── main.rs           # Entry point, stdio loop
│   │   ├── capture.rs        # Windows native screenshot
│   │   ├── vision.rs         # Vision API call
│   │   ├── llm.rs            # LLM streaming (OpenAI + Anthropic)
│   │   ├── profiles.rs       # Profile-based system prompts
│   │   └── protocol.rs       # NDJSON message types
├── config/
│   ├── config.json           # Default config template
│   └── schemas/              # Config JSON schemas
└── assets/
    └── icon.png              # App icon
```

**Build output:**
- Electron packages via Electron Forge
- Rust sidecar cross-compiles to `system-helper.exe`
- Sidecar binary lives in `resources/`, spawned at app launch

---

## 9. Out of Scope (MVP)

- Audio capture / STT
- Region-select capture
- Runtime model switching
- Settings UI
- Cross-platform support (Windows only)
- Continuous/always-on streaming