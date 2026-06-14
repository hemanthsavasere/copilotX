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
