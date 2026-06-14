# CopilotX MVP Implementation Plan

## Overview

CopilotX is a real-time AI interview copilot running as an invisible background overlay on Windows. It captures the screen via a configurable global hotkey, sends the screenshot to a vision-capable LLM, and streams answers into an always-on-top, semi-transparent overlay window.

The MVP consists of three components: an Electron + React/TypeScript overlay UI, a Rust sidecar (`system-helper.exe`) for screen capture and LLM inference, and an NDJSON-over-stdio IPC bridge connecting them.

**Stack:** Electron 33+ / electron-vite / React 19 / TypeScript 5.7 / pnpm / Rust (stable, MSVC toolchain)

**Target platform:** Windows only (MVP)

## Current State Analysis

This is a **greenfield project**. The repository contains only documentation:
- `docs/superpowers/specs/2026-06-13-copilotx-design.md` — Approved MVP design specification
- `Architecture.md` — Architecture reference from the upstream "cheating-daddy" project

No source code, package manifests, build configuration, or tests exist yet.

**Key design constraints from the spec:**
- Overlay must be content-protected (`setContentProtection(true)`) and hidden from task switchers
- IPC is NDJSON over stdin/stdout — no open ports or named pipes (anti-detection)
- Sidecar binary named `system-helper.exe` (anti-detection)
- All screenshots stay in memory — never written to disk
- Config is read-once at startup — no runtime settings UI in MVP
- Only OpenAI GPT-4o and Anthropic Claude are supported (MVP)

## Desired End State

A working CopilotX application where:
1. User launches the app — an invisible overlay appears at the right edge of the screen
2. User presses `Ctrl+Shift+Space` (configurable) — screen is captured natively by the Rust sidecar
3. Screenshot is sent to the configured vision LLM (GPT-4o or Claude)
4. Answer tokens stream back into the overlay in real time
5. User can navigate between answers with prev/next buttons
6. Overlay is hidden from screen-share/capture APIs and not visible in Alt+Tab
7. The entire pipeline works end-to-end on Windows with no manual steps other than configuring `config.json`

### Key Discoveries:
- `screenshots` crate is deprecated — must use `xcap` (successor, uses Windows Graphics Capture API) at `docs/superpowers/specs/2026-06-13-copilotx-design.md:158`
- `async-openai` v0.41 is mature for OpenAI streaming; Anthropic should use raw `reqwest` + `reqwest-eventsource` (the `anthropic` crate is v0.0.8 and not production-ready) at `docs/superpowers/specs/2026-06-13-copilotx-design.md:184-188`
- Electron's `setAlwaysOnTop(true, 'screen')` + `setContentProtection(true)` + `skipTaskbar: true` covers most anti-detection; `WS_EX_TOOLWINDOW` + `WS_EX_NOACTIVATE` require a native module call via `ffi-napi` at `docs/superpowers/specs/2026-06-13-copilotx-design.md:99-108`
- The spec calls for `focusable: false` on the overlay window, which is supported in Electron 20+ at `docs/superpowers/specs/2026-06-13-copilotx-design.md:106`
- Debounce is state-based (boolean gate), not timer-based — block until `done` or `error` is received from the sidecar at `docs/superpowers/specs/2026-06-13-copilotx-design.md:159`

## What We're NOT Doing

- Audio capture / STT (post-MVP)
- Region-select capture (post-MVP)
- Runtime model switching or settings UI (post-MVP)
- Cross-platform support (Windows only for MVP)
- Continuous/always-on streaming (post-MVP)
- CI/CD pipeline (post-MVP)
- Linux/macOS development path — Windows-only throughout
- Any form of automated E2E testing that requires screen capture infrastructure (manual testing only for capture pipeline)

## Implementation Approach

We build in 5 phases, each producing a testable increment:

1. **Phase 1** proves the architecture works end-to-end with a ping/pong IPC message
2. **Phase 2** builds the Rust sidecar's core capability (capture + inference)
3. **Phase 3** builds the overlay UI with all visual states
4. **Phase 4** wires the full pipeline together with config and error handling
5. **Phase 5** polishes, packages, and does final smoke testing

Each phase has clear automated and manual verification steps. No phase proceeds until the previous phase's verification passes.

---

## Phase 1: Project Scaffolding & IPC Foundation

### Overview

Initialize the monorepo structure, set up electron-vite with React/TypeScript, scaffold the Rust sidecar, and prove that NDJSON IPC between Electron and the sidecar works with a ping/pong message. By the end of this phase, `pnpm dev` launches an Electron window that can send `{ "type": "ping" }` to the sidecar and receive `{ "type": "pong" }` back.

### Changes Required:

#### 1. Root package.json and pnpm workspace

**File**: `copilotx/package.json`

```json
{
  "name": "copilotx",
  "version": "0.1.0",
  "description": "Real-time AI interview copilot",
  "main": "./out/main/index.js",
  "scripts": {
    "dev": "electron-vite dev",
    "build": "pnpm run typecheck && electron-vite build",
    "preview": "electron-vite preview",
    "typecheck": "pnpm run typecheck:node && pnpm run typecheck:web",
    "typecheck:node": "tsc --noEmit -p tsconfig.node.json --composite false",
    "typecheck:web": "tsc --noEmit -p tsconfig.web.json --composite false",
    "lint": "eslint .",
    "lint:fix": "eslint . --fix",
    "format": "prettier --write .",
    "build:sidecar": "cd sidecar && cargo build --release",
    "build:all": "pnpm run build:sidecar && pnpm run build",
    "build:win": "pnpm run build:all && electron-builder --win"
  },
  "dependencies": {
    "@electron-toolkit/preload": "^3.0.2",
    "@electron-toolkit/utils": "^4.0.0"
  },
  "devDependencies": {
    "@electron-toolkit/tsconfig": "^2.0.0",
    "@types/node": "^22.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.4",
    "electron": "^33.0.0",
    "electron-builder": "^25.0.0",
    "electron-vite": "^2.3.0",
    "eslint": "^9.0.0",
    "prettier": "^3.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "typescript": "^5.7.0",
    "vitest": "^3.0.0"
  }
}
```

#### 2. electron-vite configuration

**File**: `copilotx/electron.vite.config.ts`

```typescript
import { resolve } from 'path'
import { defineConfig, externalizeDepsPlugin } from 'electron-vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()]
  },
  preload: {
    plugins: [externalizeDepsPlugin()]
  },
  renderer: {
    resolve: {
      alias: {
        '@renderer': resolve('src/renderer/src')
      }
    },
    plugins: [react()]
  }
})
```

#### 3. TypeScript configurations

**File**: `copilotx/tsconfig.json`

```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.node.json" },
    { "path": "./tsconfig.web.json" }
  ]
}
```

**File**: `copilotx/tsconfig.node.json`

```json
{
  "extends": "@electron-toolkit/tsconfig/tsconfig.node.json",
  "include": ["electron.vite.config.*", "src/main/**/*", "src/preload/**/*"],
  "compilerOptions": {
    "composite": true,
    "types": ["electron-vite/node"]
  }
}
```

**File**: `copilotx/tsconfig.web.json`

```json
{
  "extends": "@electron-toolkit/tsconfig/tsconfig.web.json",
  "include": [
    "src/renderer/src/env.d.ts",
    "src/renderer/src/**/*",
    "src/renderer/src/**/*.tsx",
    "src/preload/*.d.ts"
  ],
  "compilerOptions": {
    "composite": true,
    "jsx": "react-jsx",
    "baseUrl": ".",
    "paths": {
      "@renderer/*": ["src/renderer/src/*"]
    }
  }
}
```

#### 4. Electron main process (scaffold)

**File**: `copilotx/src/main/index.ts`

```typescript
import { app, BrowserWindow, globalShortcut } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import { startSidecar, stopSidecar, onSidecarMessage } from './ipc'

let mainWindow: BrowserWindow | null = null

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 320,
    height: 600,
    show: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow!.show()
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

app.whenReady().then(() => {
  electronApp.setAppUserModelId('com.copilotx')

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  startSidecar()
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('will-quit', () => {
  stopSidecar()
  globalShortcut.unregisterAll()
})
```

#### 5. IPC bridge (ping/pong only for Phase 1)

**File**: `copilotx/src/main/ipc.ts`

```typescript
import { spawn, ChildProcess } from 'child_process'
import { createInterface } from 'readline'
import * as path from 'path'
import { app } from 'electron'
import { is } from '@electron-toolkit/utils'

export interface SidecarMessage {
  type: 'token' | 'done' | 'error' | 'pong'
  content?: string
  message?: string
}

export type SidecarMessageHandler = (msg: SidecarMessage) => void

let sidecar: ChildProcess | null = null
let messageHandler: SidecarMessageHandler | null = null

export function startSidecar(): void {
  const sidecarPath = is.dev
    ? path.join(__dirname, '../../sidecar/target/release/system-helper.exe')
    : path.join(process.resourcesPath, 'system-helper.exe')

  sidecar = spawn(sidecarPath, [], {
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true
  })

  const rl = createInterface({
    input: sidecar.stdout!,
    terminal: false,
    crlfDelay: Infinity
  })

  rl.on('line', (line: string) => {
    const trimmed = line.trim()
    if (!trimmed) return
    try {
      const msg: SidecarMessage = JSON.parse(trimmed)
      messageHandler?.(msg)
    } catch {
      console.error('[sidecar] Invalid NDJSON:', trimmed)
    }
  })

  sidecar.stderr?.on('data', (d: Buffer) => {
    console.error('[sidecar stderr]', d.toString())
  })

  sidecar.on('exit', (code) => {
    console.log('[sidecar] exited with code:', code)
    sidecar = null
  })
}

export function stopSidecar(): void {
  if (!sidecar || sidecar.killed) return
  writeSidecar({ type: 'shutdown' })
  setTimeout(() => {
    sidecar?.kill('SIGTERM')
  }, 2000)
}

export function sendCapture(): void {
  writeSidecar({ type: 'capture' })
}

export function sendPing(): void {
  writeSidecar({ type: 'ping' })
}

export function onSidecarMessage(handler: SidecarMessageHandler): void {
  messageHandler = handler
}

function writeSidecar(msg: Record<string, string>): void {
  if (!sidecar?.stdin || sidecar.stdin.destroyed) return
  sidecar.stdin.write(JSON.stringify(msg) + '\n')
}
```

#### 6. Preload script

**File**: `copilotx/src/preload/index.ts`

```typescript
import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

const api = {
  onToken: (callback: (content: string) => void) =>
    ipcRenderer.on('token', (_event, content) => callback(content)),
  onCaptureState: (callback: (state: string, error?: string) => void) =>
    ipcRenderer.on('capture-state', (_event, state, error) => callback(state, error)),
  triggerCapture: () => ipcRenderer.invoke('trigger-capture')
}

if (process.contextIsolated) {
  contextBridge.exposeInMainWorld('electron', electronAPI)
  contextBridge.exposeInMainWorld('api', api)
} else {
  // @ts-ignore
  window.electron = electronAPI
  // @ts-ignore
  window.api = api
}
```

**File**: `copilotx/src/preload/index.d.ts`

```typescript
import { ElectronAPI } from '@electron-toolkit/preload'

declare global {
  interface Window {
    electron: ElectronAPI
    api: {
      onToken: (callback: (content: string) => void) => void
      onCaptureState: (callback: (state: string, error?: string) => void) => void
      triggerCapture: () => Promise<void>
    }
  }
}
```

#### 7. React renderer (scaffold)

**File**: `copilotx/src/renderer/index.html`

```html
<!doctype html>
<html>
  <head>
    <meta charset="UTF-8" />
    <title>CopilotX</title>
    <meta
      http-equiv="Content-Security-Policy"
      content="default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
    />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

**File**: `copilotx/src/renderer/src/main.tsx`

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
```

**File**: `copilotx/src/renderer/src/App.tsx`

```tsx
import { useState, useEffect } from 'react'

export default function App() {
  const [status, setStatus] = useState('idle')

  useEffect(() => {
    window.api.onCaptureState((state) => setStatus(state))
  }, [])

  return (
    <div style={{ padding: 16, fontFamily: 'monospace', color: '#e0e0e0', background: '#1a1a2e' }}>
      <h2>CopilotX</h2>
      <p>Status: {status}</p>
      <p>Press Ctrl+Shift+Space to capture</p>
    </div>
  )
}
```

#### 8. Rust sidecar (ping/pong only for Phase 1)

**File**: `copilotx/sidecar/Cargo.toml`

```toml
[package]
name = "system-helper"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "system-helper"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

**File**: `copilotx/sidecar/src/main.rs`

```rust
use serde::Deserialize;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Command {
    ping,
    capture,
    stop,
    shutdown,
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

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
                writeln!(writer, r#"{{"type":"error","message":"Parse error: {}"}}"#, e).ok();
                writer.flush().ok();
                continue;
            }
        };

        match cmd {
            Command::ping => {
                writeln!(writer, r#"{{"type":"pong"}}"#).ok();
                writer.flush().ok();
            }
            Command::capture => {
                writeln!(writer, r#"{{"type":"error","message":"Capture not implemented yet"}}"#).ok();
                writer.flush().ok();
            }
            Command::stop => {}
            Command::shutdown => break,
        }
    }
}
```

#### 9. electron-builder configuration

**File**: `copilotx/electron-builder.yml`

```yaml
appId: com.copilotx
productName: CopilotX
directories:
  buildResources: build
files:
  - '!**/.vscode/*'
  - '!src/*'
  - '!electron.vite.config.{js,ts,mjs,cjs}'
  - '!{.eslintcache,.prettierignore,.prettierrc.yaml,dev-app-update.yml,CHANGELOG.md,README.md}'
  - '!{.env,.env.*,.npmrc,pnpm-lock.yaml}'
  - '!{tsconfig.json,tsconfig.node.json,tsconfig.web.json}'
asarUnpack:
  - resources/**
win:
  executableName: system-helper
  target:
    - nsis
nsis:
  artifactName: ${name}-${version}-setup.${ext}
  shortcutName: ${productName}
  uninstallDisplayName: ${productName}
  createDesktopShortcut: always
```

#### 10. Config template

**File**: `copilotx/config/config.json`

```json
{
  "hotkey": "CommandOrControl+Shift+Space",
  "model": "gpt-4o",
  "openaiApiKey": "",
  "anthropicApiKey": "",
  "profile": "interview",
  "overlayOpacity": 0.85,
  "overlayWidth": 320,
  "overlayPosition": "right"
}
```

#### 11. ESLint and Prettier config

**File**: `copilotx/eslint.config.mjs`

```javascript
import js from '@eslint/js'
import tseslint from 'typescript-eslint'

export default tseslint.config(
  { ignores: ['dist', 'out', 'sidecar'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['src/renderer/src/**/*.{ts,tsx}'],
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn'
    }
  },
  {
    files: ['src/main/**/*.{ts}', 'src/preload/**/*.{ts}'],
    rules: {
      'no-console': 'warn'
    }
  }
)
```

**File**: `copilotx/.prettierrc.yaml`

```yaml
semi: false
singleQuote: true
trailingComma: all
printWidth: 100
```

#### 12. .gitignore

**File**: `copilotx/.gitignore`

```
node_modules/
dist/
out/
sidecar/target/
.env
.env.*
*.log
.DS_Store
Thumbs.db
resources/system-helper.exe
```

### Success Criteria:

#### Automated Verification:
- [ ] `pnpm install` completes without errors
- [ ] `pnpm run typecheck` passes
- [ ] `pnpm run lint` passes with no errors
- [ ] `cd sidecar && cargo build --release` compiles successfully
- [ ] `pnpm run dev` launches Electron window without crashes
- [ ] Sending `{ "type": "ping" }` to the sidecar via stdin returns `{ "type": "pong" }` on stdout

#### Manual Verification:
- [ ] Electron app window appears when running `pnpm dev`
- [ ] Sidecar process starts alongside the Electron app (visible in stderr logs)
- [ ] No console errors in DevTools

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the app launches correctly and the sidecar ping/pong works before proceeding to Phase 2.

---

## Phase 2: Rust Sidecar Core — Capture, Vision & LLM Streaming

### Overview

Implement the full Rust sidecar pipeline: Windows native screen capture, vision API calls, and streaming LLM inference for both OpenAI and Anthropic. The sidecar should be testable independently via CLI mode before integrating with Electron.

### Changes Required:

#### 1. Protocol types (NDJSON message definitions)

**File**: `copilotx/sidecar/src/protocol.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    ping,
    capture,
    stop,
    shutdown,
}

#[derive(Serialize)]
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
```

#### 2. Configuration loading

**File**: `copilotx/sidecar/src/config.rs`

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub hotkey: String,
    pub model: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: String,
    pub profile: String,
    pub overlay_opacity: f64,
    pub overlay_width: u32,
    pub overlay_position: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("COPILOTX_CONFIG")
            .unwrap_or_else(|_| {
                let mut p = dirs::data_local_dir()
                    .unwrap_or_else(|| PathBuf::from("."));
                p.push("copilotx");
                p.push("config.json");
                p.to_string_lossy().to_string()
            });

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from {}", config_path))?;
        let config: Config = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config.json")?;
        Ok(config)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !matches!(self.model.as_str(), "gpt-4o" | "claude" | "claude-sonnet") {
            errors.push(format!("Unknown model: {}. Supported: gpt-4o, claude, claude-sonnet", self.model));
        }

        if self.model == "gpt-4o" && self.openai_api_key.is_empty() {
            errors.push("openaiApiKey is required when model is gpt-4o".to_string());
        }

        if matches!(self.model.as_str(), "claude" | "claude-sonnet") && self.anthropic_api_key.is_empty() {
            errors.push("anthropicApiKey is required when model is claude/claude-sonnet".to_string());
        }

        if self.hotkey.is_empty() {
            errors.push("hotkey is required".to_string());
        }

        if self.overlay_opacity < 0.1 || self.overlay_opacity > 1.0 {
            errors.push("overlayOpacity must be between 0.1 and 1.0".to_string());
        }

        errors
    }
}
```

Add `dirs` and `serde_json` to `Cargo.toml` dependencies.

#### 3. Windows native screen capture

**File**: `copilotx/sidecar/src/capture.rs`

```rust
use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use xcap::Monitor;

pub fn capture_primary_monitor() -> Result<String> {
    let monitors = Monitor::all().context("Failed to enumerate monitors")?;
    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .context("No primary monitor found")?;

    let image = primary.capture_image().context("Failed to capture screen")?;

    let mut png_buf = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png_buf), image::ImageFormat::Png)
        .context("Failed to encode screenshot to PNG")?;

    let b64 = BASE64.encode(&png_buf);
    Ok(b64)
}
```

Add to `Cargo.toml`:
```toml
xcap = "0.9"
image = "0.25"
base64 = "0.22"
```

#### 4. Profile-based system prompts

**File**: `copilotx/sidecar/src/profiles.rs`

```rust
use std::collections::HashMap;

pub fn get_system_prompt(profile: &str) -> Option<String> {
    let profiles: HashMap<&str, &str> = HashMap::from([
        ("interview", "You are an expert interview assistant. When shown a screenshot of a coding problem, MCQ, or technical question, provide a concise, correct answer. For coding problems, give working code with brief explanation. For MCQs, give the answer with one-line reasoning."),
        ("sales", "You are a sales assistant. Help respond to objections and suggest talking points."),
        ("meeting", "You are a meeting assistant. Summarize discussions and suggest action items."),
        ("presentation", "You are a presentation assistant. Help with talking points and Q&A responses."),
        ("negotiation", "You are a negotiation assistant. Suggest strategies and counterarguments."),
    ]);
    profiles.get(profile).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interview_profile() {
        let prompt = get_system_prompt("interview").unwrap();
        assert!(prompt.contains("interview assistant"));
    }

    #[test]
    fn test_unknown_profile() {
        assert!(get_system_prompt("unknown").is_none());
    }
}
```

#### 5. LLM streaming (OpenAI + Anthropic)

**File**: `copilotx/sidecar/src/llm.rs`

```rust
use anyhow::{Context, Result};
use std::io::Write;

use crate::protocol::Message;

fn print_message(msg: &Message) {
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    writeln!(writer, "{}", msg.to_ndjson()).ok();
    writer.flush().ok();
}

// ── OpenAI GPT-4o ──

pub async fn stream_openai(
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
) -> Result<()> {
    use async_openai::{
        Client,
        config::OpenAIConfig,
        types::{
            ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
            ContentPart, ImageUrlContentPartTypeArgs, ImageUrlArgs, TextContentPartTypeArgs,
            CreateChatCompletionRequestArgs,
        },
    };
    use futures::StreamExt;

    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .stream(true)
        .messages(vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()?
                    .into(),
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::TextAndImage(vec![
                        ContentPart::Text(
                            TextContentPartTypeArgs::default()
                                .text("Analyze this screenshot and provide the answer.")
                                .build()?
                                .into(),
                        ),
                        ContentPart::ImageUrl(
                            ImageUrlContentPartTypeArgs::default()
                                .image_url(
                                    ImageUrlArgs::default()
                                        .url(format!("data:image/png;base64,{}", image_base64))
                                        .detail(async_openai::types::Image_URLDetail::High)
                                        .build()?
                                        .into(),
                                )
                                .build()?
                                .into(),
                        ),
                    ]))
                    .build()?
                    .into(),
            ),
        ])
        .build()?;

    let mut stream = client.chat().create_stream(request).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                for choice in response.choices {
                    if let Some(content) = choice.delta.content {
                        print_message(&Message::Token { content });
                    }
                }
            }
            Err(e) => {
                print_message(&Message::Error {
                    message: e.to_string(),
                });
                return Err(e.into());
            }
        }
    }

    print_message(&Message::Done);
    Ok(())
}

// ── Anthropic Claude ──

pub async fn stream_anthropic(
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
) -> Result<()> {
    use reqwest::Client as HttpClient;
    use reqwest_eventsource::{Event, EventSource};
    use futures::StreamExt;

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
                        "text": "Analyze this screenshot and provide the answer."
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
                            print_message(&Message::Token {
                                content: text.to_string(),
                            });
                        }
                    }
                    "message_stop" => {
                        print_message(&Message::Done);
                        es.close();
                        return Ok(());
                    }
                    "error" => {
                        let err_msg = parsed["error"]["message"]
                            .as_str()
                            .unwrap_or("Unknown Anthropic error");
                        print_message(&Message::Error {
                            message: err_msg.to_string(),
                        });
                        anyhow::bail!("Anthropic API error: {}", err_msg);
                    }
                    _ => {}
                }
            }
            Event::Closed => break,
        }
    }

    print_message(&Message::Done);
    Ok(())
}
```

#### 6. Full pipeline orchestration (updated main.rs)

**File**: `copilotx/sidecar/src/main.rs`

```rust
#![cfg_attr(windows, windows_subsystem = "windows")]

mod capture;
mod config;
mod llm;
mod profiles;
mod protocol;

use protocol::{Command, Message};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn print_message(msg: &Message) {
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    writeln!(writer, "{}", msg.to_ndjson()).ok();
    writer.flush().ok();
}

fn print_error(message: &str) {
    print_message(&Message::Error {
        message: message.to_string(),
    });
}

#[tokio::main]
async fn main() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            print_error(&format!("Config error: {}", e));
            std::process::exit(1);
        }
    };

    let validation_errors = config.validate();
    if !validation_errors.is_empty() {
        print_error(&format!("Config validation: {}", validation_errors.join("; ")));
        std::process::exit(1);
    }

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
                print_error(&format!("Parse error: {}", e));
                continue;
            }
        };

        match cmd {
            Command::ping => {
                print_message(&Message::Pong);
            }
            Command::capture => {
                if is_processing.load(Ordering::SeqCst) {
                    print_error("Already processing");
                    continue;
                }
                is_processing.store(true, Ordering::SeqCst);

                let system_prompt = match profiles::get_system_prompt(&config.profile) {
                    Some(p) => p,
                    None => {
                        print_error(&format!("Unknown profile: {}", config.profile));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let image_b64 = match capture::capture_primary_monitor() {
                    Ok(img) => img,
                    Err(e) => {
                        print_error(&format!("Capture failed: {}", e));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let result = match config.model.as_str() {
                    "gpt-4o" => {
                        llm::stream_openai(&config.openai_api_key, &system_prompt, &image_b64).await
                    }
                    "claude" | "claude-sonnet" => {
                        llm::stream_anthropic(&config.anthropic_api_key, &system_prompt, &image_b64).await
                    }
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    print_error(&format!("LLM error: {}", e));
                }

                is_processing.store(false, Ordering::SeqCst);
            }
            Command::stop => {
                is_processing.store(false, Ordering::SeqCst);
            }
            Command::shutdown => break,
        }
    }
}
```

#### 7. Updated Cargo.toml with all dependencies

**File**: `copilotx/sidecar/Cargo.toml` (full version)

```toml
[package]
name = "system-helper"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "system-helper"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "2"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
xcap = "0.9"
image = "0.25"
base64 = "0.22"
async-openai = "0.41"
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
reqwest-eventsource = "0.6"
dirs = "6"

[profile.release]
opt-level = "s"
lto = true
strip = true
codegen-units = 1
```

### Success Criteria:

#### Automated Verification:
- [ ] `cd sidecar && cargo build --release` compiles without errors
- [ ] `cd sidecar && cargo test` passes (unit tests for protocol, profiles, config)
- [ ] Sidecar responds to `{ "type": "ping" }` with `{ "type": "pong" }` via CLI: `echo '{"type":"ping"}' | cargo run --release`
- [ ] Sidecar loads config from `COPILOTX_CONFIG` env var or default path
- [ ] `cd sidecar && cargo clippy -- -D warnings` passes

#### Manual Verification:
- [ ] Sidecar captures primary monitor screenshot when sent `{ "type": "capture" }` (requires valid API key and Windows desktop)
- [ ] Tokens stream back in real time (test with both GPT-4o and Claude)
- [ ] Error messages appear correctly for invalid API keys, network failures
- [ ] No screenshot files written to disk (verify via file system monitoring)
- [ ] Memory usage is reasonable during and after capture

**Implementation Note**: After completing this phase, pause for manual confirmation that the sidecar can capture and stream before proceeding to Phase 3.

---

## Phase 3: Electron Overlay UI

### Overview

Build the overlay window with all visual states (idle, processing, streaming, error), answer navigation (prev/next), Alt+Arrow repositioning, and all anti-detection window properties. The overlay communicates with the main process via IPC and renders answer tokens as they arrive.

### Changes Required:

#### 1. Overlay window creation with anti-detection properties

**File**: `copilotx/src/main/overlay.ts`

```typescript
import { BrowserWindow, screen } from 'electron'
import { join } from 'path'
import { is } from '@electron-toolkit/utils'
import type { AppConfig } from './config'
import { applyStealthFlags } from './stealth'

export function createOverlayWindow(config: AppConfig): BrowserWindow {
  const primaryDisplay = screen.getPrimaryDisplay()
  const { width: screenWidth, height: screenHeight } = primaryDisplay.workAreaSize
  const overlayWidth = config.overlayWidth || 320

  const win = new BrowserWindow({
    width: overlayWidth,
    height: screenHeight,
    x: screenWidth - overlayWidth,
    y: 0,
    alwaysOnTop: true,
    frame: false,
    transparent: true,
    backgroundColor: '#00000000',
    skipTaskbar: true,
    resizable: false,
    hasShadow: false,
    focusable: false,
    show: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      backgroundThrottling: false
    }
  })

  win.setAlwaysOnTop(true, 'screen')
  win.setContentProtection(true)
  applyStealthFlags(win)

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    win.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }

  return win
}
```

#### 2. Stealth flags module (Win32 API)

**File**: `copilotx/src/main/stealth.ts`

```typescript
import { BrowserWindow } from 'electron'

export function applyStealthFlags(win: BrowserWindow): void {
  if (process.platform !== 'win32') return

  try {
    const ffi = require('ffi-napi')
    const ref = require('ref-napi')

    const user32 = ffi.Library('user32', {
      GetWindowLongPtrW: ['long', ['pointer', 'int']],
      SetWindowLongPtrW: ['long', ['pointer', 'int', 'long']],
      SetWindowPos: ['bool', ['pointer', 'pointer', 'int', 'int', 'int', 'int', 'uint']]
    })

    const GWL_EXSTYLE = -20
    const WS_EX_TOOLWINDOW = 0x00000080
    const WS_EX_NOACTIVATE = 0x08000000
    const SWP_NOMOVE = 0x0002
    const SWP_NOSIZE = 0x0001
    const SWP_NOZORDER = 0x0004
    const SWP_FRAMECHANGED = 0x0020

    const hwnd = win.getNativeWindowHandle()
    const exStyle = user32.GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
    const newStyle = exStyle | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
    user32.SetWindowLongPtrW(hwnd, GWL_EXSTYLE, newStyle)
    user32.SetWindowPos(hwnd, null, 0, 0, 0, 0,
      SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED
    )
  } catch (err) {
    console.error('[stealth] Failed to apply Win32 flags (ffi-napi may not be installed):', err)
  }
}
```

Add `ffi-napi` and `ref-napi` to `package.json` dependencies (Windows-only, optional).

#### 3. Config loading

**File**: `copilotx/src/main/config.ts`

```typescript
import { readFileSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'

export interface AppConfig {
  hotkey: string
  model: string
  openaiApiKey: string
  anthropicApiKey: string
  profile: string
  overlayOpacity: number
  overlayWidth: number
  overlayPosition: string
}

export function loadConfig(): AppConfig {
  const configPath = join(app.getPath('userData'), 'config.json')
  const content = readFileSync(configPath, 'utf-8')
  return JSON.parse(content) as AppConfig
}

export function validateConfig(config: AppConfig): string[] {
  const errors: string[] = []

  if (!config.model) {
    errors.push('model is required')
  } else if (!['gpt-4o', 'claude', 'claude-sonnet'].includes(config.model)) {
    errors.push(`Unknown model: ${config.model}. Supported: gpt-4o, claude, claude-sonnet`)
  }

  if (config.model === 'gpt-4o' && !config.openaiApiKey) {
    errors.push('openaiApiKey is required when model is gpt-4o')
  }

  if ((config.model === 'claude' || config.model === 'claude-sonnet') && !config.anthropicApiKey) {
    errors.push('anthropicApiKey is required when model is claude/claude-sonnet')
  }

  if (!config.hotkey) {
    errors.push('hotkey is required')
  }

  if (config.overlayOpacity < 0.1 || config.overlayOpacity > 1.0) {
    errors.push('overlayOpacity must be between 0.1 and 1.0')
  }

  return errors
}
```

#### 4. Global hotkey and debounce

**File**: `copilotx/src/main/hotkey.ts`

```typescript
import { globalShortcut, BrowserWindow } from 'electron'
import { sendCapture } from './ipc'

let isProcessing = false

export function registerHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isProcessing) {
      window.webContents.send('capture-state', 'already-processing')
      return
    }

    isProcessing = true
    window.webContents.send('capture-state', 'processing')
    sendCapture()
  })

  if (!registered) {
    console.error(`Failed to register hotkey: ${accelerator}`)
  }

  return registered
}

export function setProcessingComplete(): void {
  isProcessing = false
}

export function unregisterAll(): void {
  globalShortcut.unregisterAll()
}
```

#### 5. Overlay position management (Alt+Arrow)

**File**: `copilotx/src/main/position.ts`

```typescript
import { BrowserWindow, screen, globalShortcut } from 'electron'

type Position = 'left' | 'right' | 'top' | 'bottom'

const POSITION_HOTKEYS: Record<Position, string> = {
  left: 'Alt+Left',
  right: 'Alt+Right',
  top: 'Alt+Up',
  bottom: 'Alt+Down'
}

export function registerPositionHotkeys(window: BrowserWindow, overlayWidth: number): void {
  const moveTo = (position: Position) => {
    const { width: screenWidth, height: screenHeight } = screen.getPrimaryDisplay().workAreaSize

    switch (position) {
      case 'right':
        window.setPosition(screenWidth - overlayWidth, 0)
        break
      case 'left':
        window.setPosition(0, 0)
        break
      case 'top':
        window.setPosition(
          Math.floor((screenWidth - overlayWidth) / 2),
          0
        )
        break
      case 'bottom':
        window.setPosition(
          Math.floor((screenWidth - overlayWidth) / 2),
          screenHeight - 400
        )
        break
    }
  }

  for (const [pos, accelerator] of Object.entries(POSITION_HOTKEYS)) {
    globalShortcut.register(accelerator, () => moveTo(pos as Position))
  }
}
```

#### 6. Wire up main process

**File**: `copilotx/src/main/index.ts` (final version)

```typescript
import { app, BrowserWindow, ipcMain } from 'electron'
import { electronApp } from '@electron-toolkit/utils'
import { createOverlayWindow } from './overlay'
import { startSidecar, stopSidecar, onSidecarMessage } from './ipc'
import { loadConfig, validateConfig } from './config'
import { registerHotkey, setProcessingComplete, unregisterAll } from './hotkey'
import { registerPositionHotkeys } from './position'

let overlayWindow: BrowserWindow | null = null

app.whenReady().then(() => {
  electronApp.setAppUserModelId('com.copilotx')

  const config = loadConfig()
  const errors = validateConfig(config)
  if (errors.length > 0) {
    console.error('Config errors:', errors.join('; '))
    app.quit()
    return
  }

  startSidecar()
  overlayWindow = createOverlayWindow(config)

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
    }
  })

  registerHotkey(config.hotkey, overlayWindow)
  registerPositionHotkeys(overlayWindow, config.overlayWidth)

  ipcMain.handle('trigger-capture', () => {
    if (!overlayWindow) return
    overlayWindow.webContents.send('capture-state', 'processing')
    sendCapture()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('will-quit', () => {
  stopSidecar()
  unregisterAll()
})

import { sendCapture } from './ipc'
```

#### 7. React overlay components

**File**: `copilotx/src/renderer/src/App.tsx`

```tsx
import { useState, useEffect, useRef } from 'react'
import { TitleBar } from './TitleBar'
import { AnswerPanel } from './AnswerPanel'
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
  const streamingRef = useRef(streamingContent)
  streamingRef.current = streamingContent

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
        setCurrentIndex(answers.length)
        setStreamingContent('')
        setState('idle')
      } else if (newState === 'error') {
        setState('error')
        setErrorMessage(error || 'Unknown error')
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
    <div className={`overlay ${state === 'error' ? 'error' : ''}`}>
      <TitleBar state={state} onClose={() => window.close()} />
      <AnswerPanel
        content={displayContent}
        state={state}
        errorMessage={errorMessage}
      />
      {answers.length > 1 && state === 'idle' && (
        <div className="navigation">
          <button onClick={handlePrev} disabled={currentIndex === 0}>
            ◀
          </button>
          <span className="counter">
            {currentIndex + 1} / {answers.length}
          </span>
          <button onClick={handleNext} disabled={currentIndex === answers.length - 1}>
            ▶
          </button>
        </div>
      )}
    </div>
  )
}
```

**File**: `copilotx/src/renderer/src/TitleBar.tsx`

```tsx
import type { OverlayState } from './App'

interface TitleBarProps {
  state: OverlayState
  onClose: () => void
}

export function TitleBar({ state, onClose }: TitleBarProps) {
  return (
    <div className="title-bar">
      <span className={`status-dot ${state}`} />
      <span className="title-text">CopilotX</span>
      <button className="close-btn" onClick={onClose}>✕</button>
    </div>
  )
}
```

**File**: `copilotx/src/renderer/src/AnswerPanel.tsx`

```tsx
import type { OverlayState } from './App'

interface AnswerPanelProps {
  content: string
  state: OverlayState
  errorMessage: string
}

export function AnswerPanel({ content, state, errorMessage }: AnswerPanelProps) {
  if (state === 'idle' && !content) {
    return (
      <div className="answer-panel idle">
        Press [hotkey] to capture
      </div>
    )
  }

  if (state === 'processing') {
    return (
      <div className="answer-panel processing">
        <div className="pulse-border" />
        Capturing screen...
      </div>
    )
  }

  if (state === 'error') {
    return (
      <div className="answer-panel error">
        Error: {errorMessage}
      </div>
    )
  }

  return (
    <div className="answer-panel streaming">
      <pre className="answer-text">{content}</pre>
    </div>
  )
}
```

**File**: `copilotx/src/renderer/src/styles.css`

```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #root {
  height: 100%;
  background-color: rgba(0, 0, 0, 0);
  font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
  color: #e0e0e0;
  overflow: hidden;
  user-select: none;
}

.overlay {
  height: 100%;
  background-color: rgba(20, 20, 30, 0.85);
  backdrop-filter: blur(24px);
  display: flex;
  flex-direction: column;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: border-color 0.3s ease;
}

.overlay.error {
  border-color: rgba(255, 80, 80, 0.6);
  animation: error-pulse 1.5s ease-in-out infinite;
}

.overlay.streaming {
  border-color: rgba(40, 167, 69, 0.4);
}

.title-bar {
  -webkit-app-region: drag;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  height: 32px;
}

.title-text {
  font-size: 12px;
  font-weight: 600;
  opacity: 0.7;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #555;
}

.status-dot.idle { background-color: #555; }
.status-dot.processing { background-color: #f0ad4e; animation: pulse 1s infinite; }
.status-dot.streaming { background-color: #28a745; }
.status-dot.error { background-color: #dc3545; }

.close-btn {
  -webkit-app-region: no-drag;
  background: none;
  border: none;
  color: #999;
  cursor: pointer;
  font-size: 14px;
  padding: 2px 6px;
  border-radius: 4px;
}

.close-btn:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.answer-panel {
  flex: 1;
  padding: 12px;
  overflow-y: auto;
  font-size: 13px;
  line-height: 1.6;
}

.answer-panel.idle {
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.3);
  font-size: 14px;
}

.answer-panel.processing {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.5);
}

.answer-panel.error {
  color: #ff6b6b;
}

.answer-text {
  white-space: pre-wrap;
  word-wrap: break-word;
  font-family: 'Cascadia Code', 'JetBrains Mono', 'Consolas', monospace;
  font-size: 12px;
  animation: fadeIn 0.2s ease-in;
}

.processing .pulse-border {
  position: absolute;
  inset: 0;
  border: 2px solid rgba(240, 173, 78, 0.6);
  border-radius: 8px;
  animation: pulse-border 1.5s ease-in-out infinite;
  pointer-events: none;
}

.navigation {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  gap: 12px;
}

.navigation button {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #e0e0e0;
  padding: 4px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.navigation button:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.navigation button:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.15);
}

.counter {
  font-size: 12px;
  opacity: 0.6;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

@keyframes pulse-border {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 1; }
}

@keyframes error-pulse {
  0%, 100% { border-color: rgba(220, 53, 69, 0.6); }
  50% { border-color: rgba(220, 53, 69, 0.2); }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

::-webkit-scrollbar {
  width: 4px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}
```

#### 8. Updated preload with full API

**File**: `copilotx/src/preload/index.ts` (updated)

```typescript
import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

const api = {
  onToken: (callback: (content: string) => void) =>
    ipcRenderer.on('token', (_event, content) => callback(content)),
  onCaptureState: (callback: (state: string, error?: string) => void) =>
    ipcRenderer.on('capture-state', (_event, state, error) => callback(state, error)),
  triggerCapture: () => ipcRenderer.invoke('trigger-capture'),
  close: () => ipcRenderer.invoke('window-close')
}

if (process.contextIsolated) {
  contextBridge.exposeInMainWorld('electron', electronAPI)
  contextBridge.exposeInMainWorld('api', api)
} else {
  // @ts-ignore
  window.electron = electronAPI
  // @ts-ignore
  window.api = api
}
```

### Success Criteria:

#### Automated Verification:
- [ ] `pnpm run typecheck` passes with no errors
- [ ] `pnpm run lint` passes with no errors
- [ ] `pnpm run dev` launches the overlay window
- [ ] Overlay window appears at the right edge of the screen
- [ ] Overlay is frameless, semi-transparent, always-on-top

#### Manual Verification:
- [ ] Overlay is not visible in Alt+Tab (with stealth flags applied)
- [ ] Overlay is not visible in taskbar
- [ ] `setContentProtection(true)` works — overlay is black/invisible in screen-share
- [ ] Alt+Left/Right/Up/Down moves the overlay to different screen edges
- [ ] Ctrl+Shift+Space triggers the processing state (pulsing indicator)
- [ ] Idle state shows "Press [hotkey] to capture"
- [ ] Error state shows red border and error message
- [ ] Prev/next navigation works when multiple answers exist

**Implementation Note**: After completing this phase, pause for manual confirmation that the overlay UI works correctly with all visual states before proceeding to Phase 4.

---

## Phase 4: Full Pipeline Integration

### Overview

Wire the complete end-to-end pipeline: hotkey press → Electron sends capture command to sidecar → sidecar captures screen → vision API call → LLM streaming → tokens relayed back to overlay. Add config loading, error handling, and overlay positioning defaults.

### Changes Required:

#### 1. Sidecar spawn path resolution (dev vs production)

**File**: `copilotx/src/main/ipc.ts` (updated)

```typescript
import { spawn, ChildProcess } from 'child_process'
import { createInterface } from 'readline'
import * as path from 'path'
import { is } from '@electron-toolkit/utils'

export function getSidecarPath(): string {
  if (is.dev) {
    return path.join(__dirname, '../../sidecar/target/release/system-helper.exe')
  }
  return path.join(process.resourcesPath, 'system-helper.exe')
}
```

#### 2. Sidecar crash recovery with auto-restart

Add to `copilotx/src/main/ipc.ts`:

```typescript
let sidecar: ChildProcess | null = null
let restartAttempts = 0
const MAX_RESTART_ATTEMPTS = 3

function handleSidecarExit(code: number | null, signal: string | null) {
  console.error(`[sidecar] exited with code=${code} signal=${signal}`)
  sidecar = null

  if (restartAttempts < MAX_RESTART_ATTEMPTS) {
    restartAttempts++
    console.log(`[sidecar] Restarting (attempt ${restartAttempts}/${MAX_RESTART_ATTEMPTS})...`)
    setTimeout(() => startSidecar(), 2000 * restartAttempts)
  }
}
```

#### 3. Error handling module

**File**: `copilotx/src/main/error-handler.ts`

```typescript
import { BrowserWindow } from 'electron'

export enum AppError {
  SIDECAR_SPAWN_FAILED = 'sidecar_spawn_failed',
  SIDECAR_CRASHED = 'sidecar_crashed',
  HOTKEY_REGISTRATION_FAILED = 'hotkey_registration_failed',
  CONFIG_LOAD_FAILED = 'config_load_failed',
  API_KEY_MISSING = 'api_key_missing',
  NETWORK_ERROR = 'network_error',
}

export function showError(window: BrowserWindow | null, error: AppError, detail?: string): void {
  if (!window) return
  window.webContents.send('capture-state', 'error', `${error}: ${detail || ''}`)
}
```

#### 4. Graceful shutdown with promise

Update `stopSidecar()` in `copilotx/src/main/ipc.ts`:

```typescript
export function stopSidecar(): Promise<void> {
  return new Promise((resolve) => {
    if (!sidecar || sidecar.killed) {
      resolve()
      return
    }

    writeSidecar({ type: 'shutdown' })

    const timeout = setTimeout(() => {
      sidecar?.kill('SIGTERM')
      resolve()
    }, 3000)

    sidecar.on('exit', () => {
      clearTimeout(timeout)
      resolve()
    })
  })
}
```

#### 5. Config JSON schema

**File**: `copilotx/config/schemas/config.schema.json`

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["hotkey", "model"],
  "properties": {
    "hotkey": {
      "type": "string",
      "description": "Electron accelerator string for the global hotkey",
      "default": "CommandOrControl+Shift+Space"
    },
    "model": {
      "type": "string",
      "enum": ["gpt-4o", "claude", "claude-sonnet"],
      "description": "LLM model to use"
    },
    "openaiApiKey": {
      "type": "string",
      "description": "OpenAI API key (required if model is gpt-4o)"
    },
    "anthropicApiKey": {
      "type": "string",
      "description": "Anthropic API key (required if model is claude/claude-sonnet)"
    },
    "profile": {
      "type": "string",
      "enum": ["interview", "sales", "meeting", "presentation", "negotiation"],
      "default": "interview"
    },
    "overlayOpacity": {
      "type": "number",
      "minimum": 0.1,
      "maximum": 1.0,
      "default": 0.85
    },
    "overlayWidth": {
      "type": "integer",
      "minimum": 200,
      "maximum": 800,
      "default": 320
    },
    "overlayPosition": {
      "type": "string",
      "enum": ["left", "right", "top", "bottom"],
      "default": "right"
    }
  }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Config validation catches missing API keys and invalid models
- [ ] `pnpm run typecheck` passes
- [ ] `pnpm run lint` passes
- [ ] `cd sidecar && cargo test` passes
- [ ] `cd sidecar && cargo clippy -- -D warnings` passes

#### Manual Verification:
- [ ] Full end-to-end pipeline works: press hotkey → screen captured → LLM inference → tokens stream into overlay
- [ ] Changing `model` to `"gpt-4o"` in config uses OpenAI, changing to `"claude"` uses Anthropic
- [ ] Invalid API key shows a clear error message in the overlay
- [ ] Network failures show error with retry suggestion
- [ ] Model timeout (30s) shows error message
- [ ] Sidecar crash shows error in overlay and auto-restarts (up to 3 attempts)
- [ ] Rapid hotkey presses are properly debounced (second press during processing is ignored)
- [ ] Config errors prevent app startup with clear error messages

**Implementation Note**: After completing this phase, pause for manual end-to-end testing before proceeding to Phase 5.

---

## Phase 5: Polish & Packaging

### Overview

Visual polish (pulse animation, smooth transitions), Electron Forge packaging, sidecar binary bundling, and final smoke testing. This phase makes the app distribution-ready.

### Changes Required:

#### 1. Copy sidecar binary to resources during build

**File**: `copilotx/scripts/copy-sidecar.js`

```javascript
const fs = require('fs')
const path = require('path')

const sidecarSrc = path.join(__dirname, '../sidecar/target/release/system-helper.exe')
const resourcesDir = path.join(__dirname, '../resources')

if (!fs.existsSync(resourcesDir)) {
  fs.mkdirSync(resourcesDir, { recursive: true })
}

const sidecarDest = path.join(resourcesDir, 'system-helper.exe')

if (fs.existsSync(sidecarSrc)) {
  fs.copyFileSync(sidecarSrc, sidecarDest)
  console.log(`Copied ${sidecarSrc} -> ${sidecarDest}`)
} else {
  console.error(`Sidecar binary not found at ${sidecarSrc}. Run 'cd sidecar && cargo build --release' first.`)
  process.exit(1)
}
```

Add to `package.json` scripts:

```json
{
  "scripts": {
    "prebuild": "node scripts/copy-sidecar.js"
  }
}
```

#### 2. Updated electron-builder.yml for production packaging

**File**: `copilotx/electron-builder.yml` (updated)

```yaml
appId: com.copilotx
productName: CopilotX
directories:
  buildResources: build
  output: dist
files:
  - out/**/*
  - resources/**/*
  - '!**/.vscode/*'
  - '!src/*'
  - '!sidecar/*'
  - '!{.eslintcache,.prettierrc.yaml,dev-app-update.yml}'
  - '!{.env,.env.*,.npmrc,pnpm-lock.yaml}'
  - '!{tsconfig*.json,electron.vite.config.*}'
extraResources:
  - from: resources/system-helper.exe
    to: system-helper.exe
win:
  executableName: system-helper
  target:
    - nsis
nsis:
  artifactName: ${name}-${version}-setup.${ext}
  shortcutName: CopilotX
  uninstallDisplayName: CopilotX
  createDesktopShortcut: always
  oneClick: true
```

#### 3. App icon placeholder

Create a `copilotx/build/icon.png` — a simple 256x256 app icon (can be a placeholder for MVP).

### Success Criteria:

#### Automated Verification:
- [ ] `pnpm run build:all` completes without errors (sidecar + Electron)
- [ ] `pnpm run build:win` produces a `.exe` installer
- [ ] `pnpm run typecheck` passes
- [ ] `pnpm run lint` passes
- [ ] `cd sidecar && cargo test` passes
- [ ] `cd sidecar && cargo clippy -- -D warnings` passes
- [ ] Built `.exe` launches without console window

#### Manual Verification:
- [ ] Full end-to-end flow works from packaged app (not just dev mode)
- [ ] Overlay is invisible in screen-share (Zoom, Discord, OBS)
- [ ] Overlay is not in Alt+Tab or taskbar
- [ ] Alt+Arrow repositioning works
- [ ] Answer navigation (prev/next) works correctly
- [ ] Error states display properly with red border
- [ ] Processing pulse animation is visible but subtle
- [ ] No console window appears for the sidecar process
- [ ] App icon appears correctly in the installer and desktop shortcut
- [ ] Uninstaller cleans up properly

---

## Testing Strategy

### Unit Tests:

**Rust sidecar** (`sidecar/src/`):
- Protocol message serialization/deserialization (`protocol.rs`)
- Config loading and validation (`config.rs`)
- Profile prompt lookup (`profiles.rs`)
- Error handling paths

**Electron/TypeScript** (`src/`):
- Config validation logic (`config.ts`)
- IPC message parsing (`ipc.ts`)
- Hotkey debounce logic (`hotkey.ts`)
- Overlay state machine (App component)

### Integration Tests:

- NDJSON round-trip: send `{ "type": "ping" }` via stdin, expect `{ "type": "pong" }` on stdout
- Config loading: verify config is read from `COPILOTX_CONFIG` env var
- Sidecar spawn: verify Electron can spawn the sidecar and receive messages
- Error propagation: verify sidecar errors appear in the overlay

### Manual Testing Steps:

1. Launch app with valid OpenAI API key and `model: "gpt-4o"` — press hotkey — verify GPT-4o streams tokens
2. Change config to `model: "claude"` with Anthropic key — press hotkey — verify Claude streams tokens
3. Set invalid API key — verify error message appears in overlay
4. Disconnect network — press hotkey — verify network error appears
5. Press hotkey rapidly 5 times — verify only one capture is processed
6. Capture 5 screens — use prev/next to navigate — verify counter shows correct position
7. Try Alt+Left, Alt+Right, Alt+Up, Alt+Down — verify overlay repositions
8. Start a Zoom/Meet screen share — verify overlay content is not visible to participants
9. Open Task Manager — verify `system-helper.exe` is running, but not `copilotx.exe`
10. Open `netstat -an` — verify no open ports from the app

## Performance Considerations

- **Screenshot size**: A 1080p PNG screenshot is ~1-3MB. Base64 encoding adds ~33% overhead. Vision APIs accept up to 20MB images. No compression needed for MVP.
- **Token streaming latency**: The sidecar must flush stdout after each `writeln!` or tokens will buffer and the overlay will stutter. Using `BufWriter<Stdout>` with explicit `flush()` after each message is critical.
- **Memory**: Screenshot stays in memory in the Rust sidecar only — it never reaches the Electron process. The Rust process should use ~50-100MB during capture, dropping back to ~10-15MB idle.
- **Overlay rendering**: React re-renders on each token. For very fast token streams (~100 tokens/sec), consider using `requestAnimationFrame` batching in post-MVP if performance becomes an issue.

## Migration Notes

This is a greenfield project — no migration needed. The first production config file should be placed at `%LOCALAPPDATA%/copilotx/config.json` on Windows.

## References

- Original design spec: `docs/superpowers/specs/2026-06-13-copilotx-design.md`
- Upstream architecture reference: `Architecture.md`
- electron-vite documentation: https://electron-vite.org/
- xcap crate (Windows screen capture): https://crates.io/crates/xcap
- async-openai crate (OpenAI streaming): https://crates.io/crates/async-openai
- reqwest-eventsource (Anthropic SSE): https://crates.io/crates/reqwest-eventsource
- ffi-napi (Win32 API access): https://www.npmjs.com/package/ffi-napi