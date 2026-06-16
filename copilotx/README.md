# CopilotX

Real-time AI interview copilot. Captures your screen on hotkey, sends it to a vision LLM, and streams answers into a stealthy overlay window.

## Architecture

```
┌──────────────────────────────────────────┐
│        Electron Main Process             │
│                                          │
│  ┌──────────┐  ┌───────────┐  ┌───────┐  │
│  │ Hotkey   │  │ Overlay   │  │ Config│  │
│  │ Listener │  │ Window    │  │ Load  │  │
│  └─────┬────┘  └──────┬────┘  └───┬───┘  │
│        │              │           │      │
│        └──────────────┼───────────┘      │
│                       │                  │
│              ┌────────▼────────┐         │
│              │ IPC Bridge      │         │
│              │ (spawn sidecar) │         │
│              └────────┬────────┘         │
└───────────────────────┼──────────────────┘
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

**Tech Stack:** Electron 33 / React 19 / TypeScript 5.7 / pnpm / Rust (stable) / xcap / async-openai

## Prerequisites

- **Node.js 22+** and **pnpm**
- **Rust** (stable toolchain; MSVC on Windows, GNU on Linux)
- **Windows** for stealth flags (`ffi-napi`); Linux is supported for screen capture via PipeWire

### Linux system packages

```bash
sudo apt install libpipewire-0.3-dev libgbm-dev
```

## Quick Start

### 1. Install dependencies

```bash
cd copilotx
pnpm install
```

### 2. Build the Rust sidecar

```bash
cd copilotx
pnpm run build:sidecar
```

### 3. Configure API keys

Copy the default config template and edit it with your API key:

```bash
# Windows: %APPDATA%/copilotx/config.json
# macOS:   ~/Library/Application Support/copilotx/config.json
# Linux:   ~/.config/copilotx/config.json
# or set COPILOTX_CONFIG env var to override

mkdir -p ~/.config/copilotx
cp copilotx/config/config.json ~/.config/copilotx/config.json
```

Edit the config and set your API key:

```json
{
  "hotkey": "CommandOrControl+Shift+Space",
  "model": "gpt-4o",
  "openaiApiKey": "sk-your-key-here",
  "anthropicApiKey": "",
  "profile": "interview",
  "overlayOpacity": 0.85,
  "overlayWidth": 320,
  "overlayPosition": "right"
}
```

Supported models: `gpt-4o`, `claude`, `claude-sonnet`.

Supported profiles: `interview`, `sales`, `meeting`, `presentation`, `negotiation`.

### 4. Run in development

```bash
cd copilotx
pnpm run dev
```

The overlay appears as a semi-transparent panel on the right edge of the screen.

## Usage

| Action | Shortcut |
|--------|----------|
| Capture screen + get answer | **Ctrl+Shift+Space** |
| Reposition overlay left | **Alt+Left** |
| Reposition overlay right | **Alt+Right** |
| Reposition overlay top | **Alt+Up** |
| Reposition overlay bottom | **Alt+Down** |
| Navigate previous answer | **◀** button in overlay |
| Navigate next answer | **▶** button in overlay |

The overlay is invisible in screen sharing (Zoom, Discord, OBS), absent from Alt+Tab, and hidden from the taskbar.

## Testing

### All tests

```bash
# Rust sidecar tests (21 tests)
cd copilotx/sidecar && cargo test

# Rust clippy
cd copilotx/sidecar && cargo clippy -- -D warnings

# Electron/React tests (20 tests)
cd copilotx && pnpm run test

# TypeScript typecheck
cd copilotx && pnpm run typecheck

# ESLint
cd copilotx && pnpm run lint
```

### Test sidecar ping/pong manually

```bash
echo '{"type":"ping"}' | copilotx/sidecar/target/release/system-helper
# Expected output: {"type":"pong"}
```

## Project Structure

```
copilotx/
├── package.json                    # Root package manifest
├── electron.vite.config.ts         # Build configuration
├── electron-builder.js             # Packaging config (platform-aware)
├── tsconfig.json                   # TypeScript project references
├── eslint.config.mjs               # ESLint flat config
├── .prettierrc.yaml                # Prettier config
├── .gitignore
├── config/
│   ├── config.json                 # Default config template
│   └── schemas/
│       └── config.schema.json      # JSON Schema for validation
├── scripts/
│   └── copy-sidecar.js             # Build script for sidecar binary
├── build/
│   └── icon.png                    # App icon
├── src/
│   ├── main/
│   │   ├── index.ts                # Electron main process entry
│   │   ├── ipc.ts                  # Sidecar spawn + NDJSON IPC
│   │   ├── config.ts               # Config loading and validation
│   │   ├── overlay.ts              # Overlay window creation
│   │   ├── stealth.ts              # Win32 anti-detection flags
│   │   ├── hotkey.ts               # Global hotkey with debounce
│   │   ├── position.ts             # Alt+Arrow repositioning
│   │   ├── error-handler.ts        # Error enum + display
│   │   └── __tests__/
│   │       ├── config.test.ts
│   │       ├── ipc.test.ts
│   │       └── hotkey.test.ts
│   ├── preload/
│   │   ├── index.ts                # Context bridge API
│   │   └── index.d.ts              # Type declarations
│   └── renderer/
│       ├── index.html
│       └── src/
│           ├── main.tsx            # React entry point
│           ├── App.tsx             # State machine + overlay
│           ├── TitleBar.tsx        # Drag handle + close
│           ├── AnswerPanel.tsx     # Answer display
│           ├── styles.css          # Overlay styles + animations
│           └── env.d.ts
└── sidecar/
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs                 # Async stdio pipeline
    │   ├── protocol.rs             # NDJSON message types
    │   ├── config.rs               # Config loading/validation
    │   ├── capture.rs              # Screen capture (xcap)
    │   ├── profiles.rs             # System prompt profiles
    │   └── llm.rs                  # OpenAI + Anthropic streaming
    └── tests/
        └── integration.rs
```

## Production Build

```bash
cd copilotx
pnpm run build:all      # Build sidecar + Electron
pnpm run build:linux    # Create AppImage + .deb  (Linux)
pnpm run build:win      # Create NSIS installer   (Windows)
```

Output:
- **Linux:** `copilotx/dist/CopilotX-0.1.0.AppImage` and `copilotx/dist/copilotx_0.1.0_amd64.deb`
- **Windows:** `copilotx/dist/CopilotX-0.1.0-setup.exe`

## Anti-Detection Features

- **Always on top** at screen-saver level
- **Frameless, transparent** window with backdrop blur
- **Content protection** prevents capture by screen-sharing apps
- **WS_EX_TOOLWINDOW** flag hides from Alt+Tab and taskbar
- **WS_EX_NOACTIVATE** prevents focus stealing
- No console window for sidecar process (`windows_subsystem = "windows"`)
- No open network ports

## Known Limitations

- **Stealth flags** (taskbar hide, Alt+Tab) are Windows-only via `ffi-napi`
- Linux screen capture requires a running PipeWire/Wayland session
- The default config stores API keys in plain text — set file permissions to `0600`
- No audio capture or speech-to-text in this MVP
- Requires an active internet connection for LLM inference

## License

Private — not for redistribution.


# AGENTS.md — CopilotX

This file provides instructions for AI coding agents working on the CopilotX
codebase. Read it in full before making any changes.

## Repository Layout
copilotx/
├── sidecar/ # Rust sidecar (system-helper.exe) — compiled separately
├── src/ # Electron main process + React renderer
│ ├── main/ # Electron main process (Node.js)
│ ├── preload/ # Electron preload scripts
│ └── renderer/ # React UI (Vite + HMR)
├── config/ # Default config template (config.json)
├── resources/ # Sidecar binary staging area (for electron-builder)
├── dist/ # Build output (electron-builder)
└── win-expanded/ # Test-ready unpacked app

The sidecar and the Electron app are **separate build units**. Never assume
changes to one automatically affect the other.

## Build System

### Full Clean Build (Windows — Release)

Run all steps in order from `copilotx/`. Do **not** skip `cargo clean` on
dependency changes. Do **not** copy only the binary.

```powershell
# 1. Kill any running instances
Get-Process -Name "CopilotX","electron","system-helper" -ErrorAction SilentlyContinue | Stop-Process -Force

# 2. Clean all artifacts
pushd sidecar; cargo clean; popd
Remove-Item -Recurse -Force dist, win-expanded -ErrorAction SilentlyContinue

# 3. Build Rust sidecar
pushd sidecar; cargo build --release; popd

# 4. Copy sidecar to resources/ (used by electron-builder)
pnpm run copy:sidecar

# 5. Build Electron app
pnpm exec electron-vite build

# 6. Package Windows app
pnpm exec electron-builder --win --dir

# 7. Copy to win-expanded for testing
New-Item -ItemType Directory -Path win-expanded -Force
robocopy dist\win-unpacked win-expanded /E /NFL /NDL
```

### Development (Fast Iteration)

Do **not** run the full build for UI or main-process changes.

```powershell
# Terminal 1 — Rust watcher (only when sidecar/ changes)
cd sidecar
cargo watch -x build

# Terminal 2 — Electron dev server with HMR
pnpm exec electron-vite dev
```

When Rust sidecar code changes, run `pnpm run copy:sidecar` after
`cargo build` before restarting the dev server.

## Branch Names

Use short branch names of at most three words, separated by hyphens.
Do not use slashes or type prefixes like `feat/` or `fix/`.

Examples: `realtime-stt`, `fix-sidecar-crash`, `update-config-schema`.

## Commits and PR Titles

Use conventional commit-style messages: `type(scope): summary`.

Valid types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`.
Scopes: `sidecar`, `main`, `renderer`, `preload`, `config`, `build`, `ipc`.

Examples:
- `feat(renderer): add voice activity indicator`
- `fix(sidecar): handle pcm16 buffer overflow`
- `chore(build): update electron-builder config`

## Style Guide

### General Principles

- Prefer `const` over `let`; avoid reassignment with ternaries or early returns
- Avoid `else`; use early returns
- Avoid `any` types in TypeScript
- Keep functions single-purpose; do not extract single-use helpers preemptively
- Inline values used only once to reduce variable count
- Use type inference; avoid explicit type annotations unless needed for exports

### Electron IPC

- All IPC channels must be declared in `preload/` with explicit types
- Never expose `ipcRenderer` directly to the renderer — use contextBridge
- Channel names use `kebab-case`: e.g. `sidecar:status`, `config:update`
- Avoid bidirectional IPC in a single call; use separate `invoke` + `on` pairs

### Rust (sidecar/)

- Use `thiserror` for error types; never `.unwrap()` in production paths
- Audio pipeline: `pcm16` format only — 24 kHz, 16-bit, mono, little-endian
- Keep capture (`cpal`) and transport (WebSocket) in separate async tasks
- All OpenAI Realtime API messages must include the model in both the URL
  query param and in `session.update` payload
- Do **not** add `OpenAI-Beta` header — it routes to the deprecated beta endpoint

### Config

- `config.json` is the single source of truth for runtime settings
- `openaiApiKey` is used for both LLM (gpt-4o) and STT (whisper-1)
- Never hardcode API keys or secrets anywhere in the codebase
- Config changes must remain backward-compatible; add fields, never rename

## OpenAI Realtime API

- **URL:** `wss://api.openai.com/v1/realtime?model=gpt-realtime-2`
- **Auth:** `Authorization: Bearer <openaiApiKey>` header only
- **Do NOT** add `OpenAI-Beta` header
- Model must appear in both the URL query param and `session.update`
- Session update event:
  ```json
  {
    "type": "session.update",
    "session": {
      "model": "gpt-realtime-2",
      "input_audio_transcription": { "model": "whisper-1" }
    }
  }
  ```
- Audio format: `pcm16` — 24 kHz, 16-bit, mono, little-endian (matches cpal)
- VAD: server-side VAD via `input_audio_transcription` config

## Testing

```powershell
# Rust tests
pushd sidecar; cargo test; popd

# JS/TS tests
pnpm test
```

- Test actual behaviour; do not duplicate logic into tests
- Avoid mocks where possible; test against real IPC and config paths
- Run Rust tests from `sidecar/`, not the repo root

## Type Checking

Run `pnpm typecheck` from the repo root, or per-package where applicable.
Never run `tsc` directly.

## Known Build Notes

- `electron-builder --win --dir` may fail on code signing (symlink privilege
  error on Windows) — the unpacked app in `dist/win-unpacked/` is still fully
  usable for testing
- Always produce a fresh `win-expanded\CopilotX.exe` via all 7 build steps
  before packaging a release; do not copy only the sidecar binary