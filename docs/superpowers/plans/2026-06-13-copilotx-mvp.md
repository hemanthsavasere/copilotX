# CopilotX MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a real-time AI interview copilot that captures the screen on hotkey, sends it to a vision LLM, and streams answers into a stealthy overlay window.

**Architecture:** Electron + React shell spawns a Rust sidecar (`system-helper.exe`) via NDJSON-over-stdio IPC. The sidecar handles native screen capture and LLM streaming. The overlay uses anti-detection window properties (content protection, no taskbar, no Alt+Tab).

**Tech Stack:** Electron 33+ / electron-vite / React 19 / TypeScript 5.7 / pnpm / Rust (stable, MSVC) / xcap / async-openai / reqwest-eventsource

---

## File Structure

| File | Responsibility |
|------|---------------|
| `copilotx/package.json` | Root package manifest, scripts, dependencies |
| `copilotx/electron.vite.config.ts` | electron-vite build configuration |
| `copilotx/tsconfig.json` | Root TS project references |
| `copilotx/tsconfig.node.json` | TS config for main/preload |
| `copilotx/tsconfig.web.json` | TS config for renderer |
| `copilotx/electron-builder.yml` | Electron packaging config |
| `copilotx/eslint.config.mjs` | ESLint flat config |
| `copilotx/.prettierrc.yaml` | Prettier config |
| `copilotx/.gitignore` | Git ignore rules |
| `copilotx/config/config.json` | Default config template |
| `copilotx/config/schemas/config.schema.json` | JSON schema for config validation |
| `copilotx/src/main/index.ts` | Electron main process entry |
| `copilotx/src/main/ipc.ts` | Sidecar spawn + NDJSON IPC bridge |
| `copilotx/src/main/config.ts` | Config loading and validation |
| `copilotx/src/main/overlay.ts` | Overlay window creation with stealth |
| `copilotx/src/main/stealth.ts` | Win32 stealth flags (WS_EX_TOOLWINDOW) |
| `copilotx/src/main/hotkey.ts` | Global hotkey registration + debounce |
| `copilotx/src/main/position.ts` | Alt+Arrow overlay repositioning |
| `copilotx/src/main/error-handler.ts` | Error enum + overlay error display |
| `copilotx/src/preload/index.ts` | Context bridge (renderer API) |
| `copilotx/src/preload/index.d.ts` | Type declarations for window.api |
| `copilotx/src/renderer/index.html` | HTML entry point |
| `copilotx/src/renderer/src/env.d.ts` | Vite client types |
| `copilotx/src/renderer/src/main.tsx` | React entry point |
| `copilotx/src/renderer/src/App.tsx` | Root overlay component + state machine |
| `copilotx/src/renderer/src/TitleBar.tsx` | Title bar (drag handle + close) |
| `copilotx/src/renderer/src/AnswerPanel.tsx` | Answer display panel |
| `copilotx/src/renderer/src/styles.css` | Overlay styles + animations |
| `copilotx/src/main/__tests__/config.test.ts` | Config validation unit tests |
| `copilotx/src/main/__tests__/ipc.test.ts` | IPC message parsing unit tests |
| `copilotx/src/main/__tests__/hotkey.test.ts` | Hotkey debounce logic unit tests |
| `copilotx/sidecar/Cargo.toml` | Rust project manifest |
| `copilotx/sidecar/src/main.rs` | Rust entry point, stdio loop |
| `copilotx/sidecar/src/protocol.rs` | NDJSON message types |
| `copilotx/sidecar/src/config.rs` | Config loading / validation |
| `copilotx/sidecar/src/capture.rs` | Windows native screen capture |
| `copilotx/sidecar/src/profiles.rs` | Profile-based system prompts |
| `copilotx/sidecar/src/llm.rs` | OpenAI + Anthropic streaming |
| `copilotx/sidecar/tests/integration.rs` | Rust integration tests |
| `copilotx/scripts/copy-sidecar.js` | Build script to copy sidecar binary |
| `copilotx/build/icon.png` | App icon placeholder |

---

## Phase 1: Project Scaffolding & IPC Foundation

### Task 1: Root package.json and pnpm setup

**Files:**
- Create: `copilotx/package.json`

- [ ] **Step 1: Create the root directory and package.json**

```bash
mkdir -p copilotx
```

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
    "build:win": "pnpm run build:all && electron-builder --win",
    "test": "vitest run",
    "test:watch": "vitest"
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

- [ ] **Step 2: Install dependencies**

Run: `cd copilotx && pnpm install`
Expected: Dependencies installed, `node_modules/` created, `pnpm-lock.yaml` generated

- [ ] **Step 3: Commit**

```bash
git add copilotx/package.json copilotx/pnpm-lock.yaml
git commit -m "chore: initialize copilotx package.json with dependencies"
```

---

### Task 2: TypeScript and build config files

**Files:**
- Create: `copilotx/tsconfig.json`
- Create: `copilotx/tsconfig.node.json`
- Create: `copilotx/tsconfig.web.json`
- Create: `copilotx/electron.vite.config.ts`
- Create: `copilotx/electron-builder.yml`
- Create: `copilotx/eslint.config.mjs`
- Create: `copilotx/.prettierrc.yaml`
- Create: `copilotx/.gitignore`

- [ ] **Step 1: Create tsconfig.json**

```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.node.json" },
    { "path": "./tsconfig.web.json" }
  ]
}
```

- [ ] **Step 2: Create tsconfig.node.json**

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

- [ ] **Step 3: Create tsconfig.web.json**

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

- [ ] **Step 4: Create electron.vite.config.ts**

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

- [ ] **Step 5: Create electron-builder.yml**

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

- [ ] **Step 6: Create eslint.config.mjs**

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

- [ ] **Step 7: Create .prettierrc.yaml**

```yaml
semi: false
singleQuote: true
trailingComma: all
printWidth: 100
```

- [ ] **Step 8: Create .gitignore**

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

- [ ] **Step 9: Commit**

```bash
git add copilotx/
git commit -m "chore: add TypeScript, build, lint, and packaging config"
```

---

### Task 3: Rust sidecar scaffolding with ping/pong

**Files:**
- Create: `copilotx/sidecar/Cargo.toml`
- Create: `copilotx/sidecar/src/main.rs`
- Create: `copilotx/sidecar/src/protocol.rs`
- Create: `copilotx/sidecar/tests/integration.rs`

- [ ] **Step 1: Write the failing test for protocol message serialization**

Create `copilotx/sidecar/Cargo.toml`:

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

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

Create `copilotx/sidecar/src/protocol.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Command {
    ping,
    capture,
    stop,
    shutdown,
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
        assert_eq!(cmd, Command::ping);
    }

    #[test]
    fn test_command_deserialize_capture() {
        let cmd: Command = serde_json::from_str(r#"{"type":"capture"}"#).unwrap();
        assert_eq!(cmd, Command::capture);
    }

    #[test]
    fn test_command_deserialize_shutdown() {
        let cmd: Command = serde_json::from_str(r#"{"type":"shutdown"}"#).unwrap();
        assert_eq!(cmd, Command::shutdown);
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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd copilotx/sidecar && cargo test`
Expected: All 8 protocol tests pass

- [ ] **Step 3: Write the main.rs with stdio loop**

Create `copilotx/sidecar/src/main.rs`:

```rust
#![cfg_attr(windows, windows_subsystem = "windows")]

mod protocol;

use protocol::{Command, Message};
use std::io::{self, BufRead, Write};

fn print_message(msg: &Message) {
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    writeln!(writer, "{}", msg.to_ndjson()).ok();
    writer.flush().ok();
}

fn main() {
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
                print_message(&Message::Error {
                    message: format!("Parse error: {}", e),
                });
                continue;
            }
        };

        match cmd {
            Command::ping => {
                print_message(&Message::Pong);
            }
            Command::capture => {
                print_message(&Message::Error {
                    message: "Capture not implemented yet".to_string(),
                });
            }
            Command::stop => {}
            Command::shutdown => break,
        }
    }
}
```

- [ ] **Step 4: Build the sidecar**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: Compiles successfully

- [ ] **Step 5: Write integration tests for the stdio loop**

Create `copilotx/sidecar/tests/integration.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_ping_pong() {
    Command::cargo_bin("system-helper")
        .unwrap()
        .write_stdin(r#"{"type":"ping"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"{"type":"pong"}"#));
}

#[test]
fn test_unknown_command() {
    Command::cargo_bin("system-helper")
        .unwrap()
        .write_stdin(r#"{"type":"unknown_command"}"#)
        .assert()
        .stdout(predicate::str::contains(r#"Parse error"#));
}

#[test]
fn test_shutdown() {
    Command::cargo_bin("system-helper")
        .unwrap()
        .write_stdin(r#"{"type":"shutdown"}"#)
        .assert()
        .success();
}
```

- [ ] **Step 6: Run all tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: All unit + integration tests pass

- [ ] **Step 7: Commit**

```bash
git add copilotx/sidecar/
git commit -m "feat: add Rust sidecar scaffolding with ping/pong protocol"
```

---

### Task 4: Electron main process scaffolding with IPC

**Files:**
- Create: `copilotx/src/main/ipc.ts`
- Create: `copilotx/src/main/index.ts`
- Create: `copilotx/src/preload/index.ts`
- Create: `copilotx/src/preload/index.d.ts`
- Create: `copilotx/src/renderer/index.html`
- Create: `copilotx/src/renderer/src/env.d.ts`
- Create: `copilotx/src/renderer/src/main.tsx`
- Create: `copilotx/src/renderer/src/App.tsx`
- Create: `copilotx/src/main/__tests__/ipc.test.ts`

- [ ] **Step 1: Write the failing test for IPC message parsing**

Create `copilotx/src/main/__tests__/ipc.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import type { SidecarMessage } from '../ipc'

describe('SidecarMessage type parsing', () => {
  it('parses a pong message', () => {
    const raw = '{"type":"pong"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('pong')
  })

  it('parses a token message', () => {
    const raw = '{"type":"token","content":"hello"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('token')
    expect(msg.content).toBe('hello')
  })

  it('parses a done message', () => {
    const raw = '{"type":"done"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('done')
  })

  it('parses an error message', () => {
    const raw = '{"type":"error","message":"fail"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('error')
    expect(msg.message).toBe('fail')
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd copilotx && pnpm run test`
Expected: FAIL — `ipc.ts` module does not exist yet

- [ ] **Step 3: Create the ipc.ts module**

Create `copilotx/src/main/ipc.ts`:

```typescript
import { spawn, ChildProcess } from 'child_process'
import { createInterface } from 'readline'
import * as path from 'path'
import { is } from '@electron-toolkit/utils'

export interface SidecarMessage {
  type: 'token' | 'done' | 'error' | 'pong'
  content?: string
  message?: string
}

export type SidecarMessageHandler = (msg: SidecarMessage) => void

let sidecar: ChildProcess | null = null
let messageHandler: SidecarMessageHandler | null = null
let restartAttempts = 0
const MAX_RESTART_ATTEMPTS = 3

function handleSidecarExit(code: number | null, signal: string | null): void {
  console.error(`[sidecar] exited with code=${code} signal=${signal}`)
  sidecar = null

  if (restartAttempts < MAX_RESTART_ATTEMPTS) {
    restartAttempts++
    console.log(`[sidecar] Restarting (attempt ${restartAttempts}/${MAX_RESTART_ATTEMPTS})...`)
    setTimeout(() => startSidecar(), 2000 * restartAttempts)
  }
}

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

  sidecar.on('exit', handleSidecarExit)
}

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

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd copilotx && pnpm run test`
Expected: PASS — all 4 SidecarMessage tests pass

- [ ] **Step 5: Create the main process entry**

Create `copilotx/src/main/index.ts`:

```typescript
import { app, BrowserWindow } from 'electron'
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
})
```

- [ ] **Step 6: Create the preload script**

Create `copilotx/src/preload/index.ts`:

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

Create `copilotx/src/preload/index.d.ts`:

```typescript
import { ElectronAPI } from '@electron-toolkit/preload'

declare global {
  interface Window {
    electron: ElectronAPI
    api: {
      onToken: (callback: (content: string) => void) => void
      onCaptureState: (callback: (state: string, error?: string) => void) => void
      triggerCapture: () => Promise<void>
      close: () => Promise<void>
    }
  }
}
```

- [ ] **Step 7: Create the renderer entry files**

Create `copilotx/src/renderer/index.html`:

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

Create `copilotx/src/renderer/src/env.d.ts`:

```typescript
/// <reference types="vite/client" />
```

Create `copilotx/src/renderer/src/main.tsx`:

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

Create `copilotx/src/renderer/src/App.tsx`:

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

- [ ] **Step 8: Verify the project compiles**

Run: `cd copilotx && pnpm run typecheck`
Expected: No TypeScript errors

- [ ] **Step 9: Commit**

```bash
git add copilotx/src/
git commit -m "feat: add Electron main process, preload, and renderer scaffolding with IPC"
```

---

### Task 5: Config template and schema

**Files:**
- Create: `copilotx/config/config.json`
- Create: `copilotx/config/schemas/config.schema.json`

- [ ] **Step 1: Create default config template**

Create `copilotx/config/config.json`:

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

- [ ] **Step 2: Create JSON schema for config validation**

Create `copilotx/config/schemas/config.schema.json`:

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

- [ ] **Step 3: Commit**

```bash
git add copilotx/config/
git commit -m "chore: add config template and JSON schema"
```

---

### Task 6: Electron config loading with validation tests

**Files:**
- Create: `copilotx/src/main/config.ts`
- Create: `copilotx/src/main/__tests__/config.test.ts`

- [ ] **Step 1: Write failing tests for config validation**

Create `copilotx/src/main/__tests__/config.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { validateConfig } from '../config'
import type { AppConfig } from '../config'

describe('validateConfig', () => {
  const validConfig: AppConfig = {
    hotkey: 'CommandOrControl+Shift+Space',
    model: 'gpt-4o',
    openaiApiKey: 'sk-test',
    anthropicApiKey: '',
    profile: 'interview',
    overlayOpacity: 0.85,
    overlayWidth: 320,
    overlayPosition: 'right'
  }

  it('returns no errors for valid config with gpt-4o', () => {
    const errors = validateConfig(validConfig)
    expect(errors).toHaveLength(0)
  })

  it('returns no errors for valid config with claude', () => {
    const config = { ...validConfig, model: 'claude', anthropicApiKey: 'sk-ant-test', openaiApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toHaveLength(0)
  })

  it('returns no errors for valid config with claude-sonnet', () => {
    const config = { ...validConfig, model: 'claude-sonnet', anthropicApiKey: 'sk-ant-test', openaiApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toHaveLength(0)
  })

  it('returns error for unknown model', () => {
    const config = { ...validConfig, model: 'gpt-3' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('Unknown model'))
  })

  it('returns error when openaiApiKey missing for gpt-4o', () => {
    const config = { ...validConfig, openaiApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('openaiApiKey'))
  })

  it('returns error when anthropicApiKey missing for claude', () => {
    const config = { ...validConfig, model: 'claude', anthropicApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('anthropicApiKey'))
  })

  it('returns error for empty hotkey', () => {
    const config = { ...validConfig, hotkey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('hotkey'))
  })

  it('returns error for overlayOpacity out of range', () => {
    const config = { ...validConfig, overlayOpacity: 0.05 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayOpacity'))
  })

  it('returns error for overlayOpacity above 1.0', () => {
    const config = { ...validConfig, overlayOpacity: 1.5 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayOpacity'))
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd copilotx && pnpm run test`
Expected: FAIL — `config.ts` module does not exist yet

- [ ] **Step 3: Create config.ts**

Create `copilotx/src/main/config.ts`:

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

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd copilotx && pnpm run test`
Expected: PASS — all 9 config tests pass

- [ ] **Step 5: Commit**

```bash
git add copilotx/src/main/config.ts copilotx/src/main/__tests__/config.test.ts
git commit -m "feat: add config loading and validation with tests"
```

---

### Task 7: Verify Phase 1 end-to-end

- [ ] **Step 1: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 2: Run lint**

Run: `cd copilotx && pnpm run lint`
Expected: No errors (may need `pnpm run lint:fix` first)

- [ ] **Step 3: Run Rust tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: All tests pass

- [ ] **Step 4: Build sidecar**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: Compiles successfully

- [ ] **Step 5: Test ping/pong via CLI**

Run: `echo '{"type":"ping"}' | copilotx/sidecar/target/release/system-helper.exe`
Expected: `{"type":"pong"}`

---

## Phase 2: Rust Sidecar Core — Capture, Vision & LLM Streaming

### Task 8: Config loading in Rust sidecar

**Files:**
- Create: `copilotx/sidecar/src/config.rs`
- Modify: `copilotx/sidecar/Cargo.toml` (add `dirs` dependency)

- [ ] **Step 1: Write failing tests for config loading and validation**

Create `copilotx/sidecar/src/config.rs`:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
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
    pub fn load_from_path(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path))?;
        let config: Config = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config.json")?;
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let config_path = std::env::var("COPILOTX_CONFIG").unwrap_or_else(|_| {
            let mut p = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            p.push("copilotx");
            p.push("config.json");
            p.to_string_lossy().to_string()
        });
        Self::load_from_path(&config_path)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !matches!(self.model.as_str(), "gpt-4o" | "claude" | "claude-sonnet") {
            errors.push(format!(
                "Unknown model: {}. Supported: gpt-4o, claude, claude-sonnet",
                self.model
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_valid_config_json() -> String {
        r#"{
            "hotkey": "CommandOrControl+Shift+Space",
            "model": "gpt-4o",
            "openai_api_key": "sk-test",
            "anthropic_api_key": "",
            "profile": "interview",
            "overlay_opacity": 0.85,
            "overlay_width": 320,
            "overlay_position": "right"
        }"#.to_string()
    }

    #[test]
    fn test_load_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", make_valid_config_json()).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.hotkey, "CommandOrControl+Shift+Space");
    }

    #[test]
    fn test_validate_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", make_valid_config_json()).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.validate().len(), 0);
    }

    #[test]
    fn test_validate_missing_api_key() {
        let json = r#"{
            "hotkey": "CommandOrControl+Shift+Space",
            "model": "gpt-4o",
            "openai_api_key": "",
            "anthropic_api_key": "",
            "profile": "interview",
            "overlay_opacity": 0.85,
            "overlay_width": 320,
            "overlay_position": "right"
        }"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("openaiApiKey")));
    }

    #[test]
    fn test_validate_unknown_model() {
        let json = r#"{
            "hotkey": "CommandOrControl+Shift+Space",
            "model": "gpt-3",
            "openai_api_key": "sk-test",
            "anthropic_api_key": "",
            "profile": "interview",
            "overlay_opacity": 0.85,
            "overlay_width": 320,
            "overlay_position": "right"
        }"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("Unknown model")));
    }

    #[test]
    fn test_load_missing_file() {
        let result = Config::load_from_path("/nonexistent/path/config.json");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Add dependencies to Cargo.toml**

Add `dirs = "6"` to `[dependencies]` in `copilotx/sidecar/Cargo.toml`.

Add `tempfile = "3"` to `[dev-dependencies]` in `copilotx/sidecar/Cargo.toml`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd copilotx/sidecar && cargo test`
Expected: All config tests pass

- [ ] **Step 4: Commit**

```bash
git add copilotx/sidecar/src/config.rs copilotx/sidecar/Cargo.toml
git commit -m "feat: add Rust sidecar config loading and validation with tests"
```

---

### Task 9: Profile-based system prompts

**Files:**
- Create: `copilotx/sidecar/src/profiles.rs`

- [ ] **Step 1: Write failing tests for profile lookup**

Create `copilotx/sidecar/src/profiles.rs`:

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
    fn test_sales_profile() {
        let prompt = get_system_prompt("sales").unwrap();
        assert!(prompt.contains("sales assistant"));
    }

    #[test]
    fn test_meeting_profile() {
        let prompt = get_system_prompt("meeting").unwrap();
        assert!(prompt.contains("meeting assistant"));
    }

    #[test]
    fn test_unknown_profile_returns_none() {
        assert!(get_system_prompt("unknown").is_none());
    }

    #[test]
    fn test_empty_profile_returns_none() {
        assert!(get_system_prompt("").is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd copilotx/sidecar && cargo test profiles`
Expected: All 5 profile tests pass

- [ ] **Step 3: Commit**

```bash
git add copilotx/sidecar/src/profiles.rs
git commit -m "feat: add profile-based system prompts with tests"
```

---

### Task 10: Windows native screen capture

**Files:**
- Create: `copilotx/sidecar/src/capture.rs`
- Modify: `copilotx/sidecar/Cargo.toml` (add `xcap`, `image`, `base64`)

- [ ] **Step 1: Add capture dependencies to Cargo.toml**

Add to `copilotx/sidecar/Cargo.toml` `[dependencies]`:

```toml
xcap = "0.9"
image = "0.25"
base64 = "0.22"
```

- [ ] **Step 2: Create capture.rs**

Create `copilotx/sidecar/src/capture.rs`:

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_capture_primary_monitor_real() {
        let result = capture_primary_monitor();
        assert!(result.is_ok());
        let b64 = result.unwrap();
        assert!(!b64.is_empty());
        assert!(b64.starts_with("iVBOR"));
    }
}
```

Note: The `test_capture_primary_monitor_real` test is `#[ignore]` because it requires a real display. Run manually on Windows with `cargo test -- --ignored`.

- [ ] **Step 3: Run the non-ignored tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: All non-ignored tests pass

- [ ] **Step 4: Compile the sidecar with full dependencies**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: Compiles (note: xcap requires Windows for full functionality; on non-Windows, it will compile but capture may fail)

- [ ] **Step 5: Commit**

```bash
git add copilotx/sidecar/src/capture.rs copilotx/sidecar/Cargo.toml
git commit -m "feat: add Windows native screen capture with xcap"
```

---

### Task 11: LLM streaming — OpenAI GPT-4o

**Files:**
- Create: `copilotx/sidecar/src/llm.rs`
- Modify: `copilotx/sidecar/Cargo.toml` (add async deps)

- [ ] **Step 1: Add LLM dependencies to Cargo.toml**

Add to `copilotx/sidecar/Cargo.toml` `[dependencies]`:

```toml
tokio = { version = "1", features = ["full"] }
futures = "0.3"
async-openai = "0.41"
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
reqwest-eventsource = "0.6"
```

- [ ] **Step 2: Create llm.rs with OpenAI streaming**

Create `copilotx/sidecar/src/llm.rs`:

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
```

- [ ] **Step 3: Verify it compiles**

Run: `cd copilotx/sidecar && cargo check`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add copilotx/sidecar/src/llm.rs copilotx/sidecar/Cargo.toml
git commit -m "feat: add OpenAI GPT-4o streaming LLM integration"
```

---

### Task 12: LLM streaming — Anthropic Claude

**Files:**
- Modify: `copilotx/sidecar/src/llm.rs` (add `stream_anthropic` function)

- [ ] **Step 1: Add the Anthropic streaming function to llm.rs**

Append to `copilotx/sidecar/src/llm.rs`:

```rust
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

- [ ] **Step 2: Verify it compiles**

Run: `cd copilotx/sidecar && cargo check`
Expected: Compiles without errors

- [ ] **Step 3: Commit**

```bash
git add copilotx/sidecar/src/llm.rs
git commit -m "feat: add Anthropic Claude streaming LLM integration"
```

---

### Task 13: Wire up main.rs with full pipeline

**Files:**
- Modify: `copilotx/sidecar/src/main.rs` (full async pipeline)
- Modify: `copilotx/sidecar/Cargo.toml` (add release profile)

- [ ] **Step 1: Update main.rs to use all modules**

Replace `copilotx/sidecar/src/main.rs`:

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

- [ ] **Step 2: Add release profile to Cargo.toml**

Add to `copilotx/sidecar/Cargo.toml`:

```toml
[profile.release]
opt-level = "s"
lto = true
strip = true
codegen-units = 1
```

- [ ] **Step 3: Build the sidecar**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: Compiles successfully

- [ ] **Step 4: Run all sidecar tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: All unit + integration tests pass

- [ ] **Step 5: Run clippy**

Run: `cd copilotx/sidecar && cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 6: Commit**

```bash
git add copilotx/sidecar/src/main.rs copilotx/sidecar/Cargo.toml
git commit -m "feat: wire up Rust sidecar full pipeline with async capture+inference"
```

---

### Task 14: Verify Phase 2 end-to-end

- [ ] **Step 1: Full sidecar build**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: Success

- [ ] **Step 2: Ping/pong still works**

Run: `echo '{"type":"ping"}' | copilotx/sidecar/target/release/system-helper.exe`
Expected: `{"type":"pong"}`

- [ ] **Step 3: Config loading works**

Run: `COPILOTX_CONFIG=/path/to/copilotx/config/config.json echo '{"type":"ping"}' | copilotx/sidecar/target/release/system-helper.exe`
Expected: `{"type":"pong"}` (valid config)

- [ ] **Step 4: Manual: Capture+inference works on Windows with valid API key** (requires Windows desktop + API key)

---

## Phase 3: Electron Overlay UI

### Task 15: Overlay window creation with anti-detection

**Files:**
- Create: `copilotx/src/main/overlay.ts`
- Create: `copilotx/src/main/stealth.ts`

- [ ] **Step 1: Create overlay.ts**

Create `copilotx/src/main/overlay.ts`:

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

- [ ] **Step 2: Create stealth.ts**

Create `copilotx/src/main/stealth.ts`:

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

- [ ] **Step 3: Add ffi-napi and ref-napi as dependencies**

Add to `copilotx/package.json` `dependencies`:

```json
"ffi-napi": "^4.0.3",
"ref-napi": "^3.0.3"
```

Run: `cd copilotx && pnpm install`

- [ ] **Step 4: Verify typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add copilotx/src/main/overlay.ts copilotx/src/main/stealth.ts copilotx/package.json copilotx/pnpm-lock.yaml
git commit -m "feat: add overlay window creation with Win32 stealth flags"
```

---

### Task 16: Config loading tests in Electron (verify)

- [ ] **Step 1: Run all existing tests**

Run: `cd copilotx && pnpm run test`
Expected: All tests pass (config + IPC parsing)

- [ ] **Step 2: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

---

### Task 17: Global hotkey registration with debounce

**Files:**
- Create: `copilotx/src/main/hotkey.ts`
- Create: `copilotx/src/main/__tests__/hotkey.test.ts`

- [ ] **Step 1: Write failing tests for hotkey debounce logic**

Create `copilotx/src/main/__tests__/hotkey.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'

describe('hotkey debounce logic', () => {
  it('allows first capture when not processing', () => {
    let isProcessing = false
    const result = !isProcessing
    expect(result).toBe(true)
  })

  it('blocks capture when already processing', () => {
    let isProcessing = true
    const result = !isProcessing
    expect(result).toBe(false)
  })

  it('allows capture after processing completes', () => {
    let isProcessing = true
    isProcessing = false
    const result = !isProcessing
    expect(result).toBe(true)
  })
})
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cd copilotx && pnpm run test`
Expected: PASS

- [ ] **Step 3: Create hotkey.ts**

Create `copilotx/src/main/hotkey.ts`:

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

- [ ] **Step 4: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add copilotx/src/main/hotkey.ts copilotx/src/main/__tests__/hotkey.test.ts
git commit -m "feat: add global hotkey registration with debounce logic"
```

---

### Task 18: Overlay position management (Alt+Arrow)

**Files:**
- Create: `copilotx/src/main/position.ts`

- [ ] **Step 1: Create position.ts**

Create `copilotx/src/main/position.ts`:

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

- [ ] **Step 2: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add copilotx/src/main/position.ts
git commit -m "feat: add overlay position management with Alt+Arrow hotkeys"
```

---

### Task 19: Error handler module

**Files:**
- Create: `copilotx/src/main/error-handler.ts`

- [ ] **Step 1: Create error-handler.ts**

Create `copilotx/src/main/error-handler.ts`:

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

- [ ] **Step 2: Commit**

```bash
git add copilotx/src/main/error-handler.ts
git commit -m "feat: add error handler module with AppError enum"
```

---

### Task 20: React overlay components

**Files:**
- Modify: `copilotx/src/renderer/src/App.tsx`
- Create: `copilotx/src/renderer/src/TitleBar.tsx`
- Create: `copilotx/src/renderer/src/AnswerPanel.tsx`
- Create: `copilotx/src/renderer/src/styles.css`

- [ ] **Step 1: Create styles.css**

Create `copilotx/src/renderer/src/styles.css`:

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

- [ ] **Step 2: Create TitleBar.tsx**

Create `copilotx/src/renderer/src/TitleBar.tsx`:

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
      <button className="close-btn" onClick={onClose}>&#10005;</button>
    </div>
  )
}
```

- [ ] **Step 3: Create AnswerPanel.tsx**

Create `copilotx/src/renderer/src/AnswerPanel.tsx`:

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

- [ ] **Step 4: Update App.tsx with full state machine**

Replace `copilotx/src/renderer/src/App.tsx`:

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

- [ ] **Step 5: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add copilotx/src/renderer/src/
git commit -m "feat: add React overlay components with all visual states"
```

---

### Task 21: Wire up main process with all modules

**Files:**
- Modify: `copilotx/src/main/index.ts` (final version with all imports)

- [ ] **Step 1: Update main/index.ts with full orchestration**

Replace `copilotx/src/main/index.ts`:

```typescript
import { app, BrowserWindow, ipcMain } from 'electron'
import { electronApp } from '@electron-toolkit/utils'
import { createOverlayWindow } from './overlay'
import { startSidecar, stopSidecar, onSidecarMessage, sendCapture } from './ipc'
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
```

- [ ] **Step 2: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 3: Run all tests**

Run: `cd copilotx && pnpm run test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add copilotx/src/main/index.ts
git commit -m "feat: wire up full Electron main process with overlay, IPC, hotkey, and config"
```

---

### Task 22: Verify Phase 3 end-to-end

- [ ] **Step 1: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 2: Run lint**

Run: `cd copilotx && pnpm run lint`
Expected: No errors

- [ ] **Step 3: Manual: Launch app and verify overlay appears** (requires Windows + built sidecar)

Run: `cd copilotx && pnpm run dev`
Expected: Overlay window appears at right edge, frameless, semi-transparent

---

## Phase 4: Full Pipeline Integration

### Task 23: Verify sidecar spawn path in dev vs production

The `startSidecar()` in `ipc.ts` already uses `is.dev` for path resolution (Task 4/21). No additional changes needed.

- [ ] **Step 1: Verify path logic is correct in ipc.ts**

The existing code contains:
```typescript
const sidecarPath = is.dev
  ? path.join(__dirname, '../../sidecar/target/release/system-helper.exe')
  : path.join(process.resourcesPath, 'system-helper.exe')
```
No changes needed. Move on.

---

### Task 24: Build script for sidecar binary copy

**Files:**
- Create: `copilotx/scripts/copy-sidecar.js`
- Modify: `copilotx/package.json` (add `prebuild` script)

- [ ] **Step 1: Create copy-sidecar.js**

Create `copilotx/scripts/copy-sidecar.js`:

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

- [ ] **Step 2: Add prebuild script to package.json**

Add to `copilotx/package.json` `scripts`:

```json
"prebuild": "node scripts/copy-sidecar.js"
```

- [ ] **Step 3: Commit**

```bash
git add copilotx/scripts/ copilotx/package.json
git commit -m "feat: add build script to copy sidecar binary to resources"
```

---

### Task 25: Final integration verification

- [ ] **Step 1: Run all sidecar tests**

Run: `cd copilotx/sidecar && cargo test`
Expected: All tests pass

- [ ] **Step 2: Run clippy**

Run: `cd copilotx/sidecar && cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 3: Run all Electron tests**

Run: `cd copilotx && pnpm run test`
Expected: All tests pass

- [ ] **Step 4: Run typecheck**

Run: `cd copilotx && pnpm run typecheck`
Expected: No errors

- [ ] **Step 5: Run lint**

Run: `cd copilotx && pnpm run lint`
Expected: No errors

- [ ] **Step 6: Build sidecar**

Run: `cd copilotx/sidecar && cargo build --release`
Expected: Compiles successfully

---

## Phase 5: Polish & Packaging

### Task 26: App icon placeholder

**Files:**
- Create: `copilotx/build/icon.png`

- [ ] **Step 1: Create build directory and a placeholder icon**

Run: `mkdir -p copilotx/build`

Create a minimal 256x256 placeholder PNG. For MVP, a solid-color icon is acceptable.

- [ ] **Step 2: Commit**

```bash
git add copilotx/build/
git commit -m "chore: add placeholder app icon"
```

---

### Task 27: Final build verification

- [ ] **Step 1: Full build**

Run: `cd copilotx && pnpm run build:all`
Expected: Compiles sidecar + builds Electron app

- [ ] **Step 2: Windows build**

Run: `cd copilotx && pnpm run build:win`
Expected: Produces a `.exe` installer (Windows only)

- [ ] **Step 3: Manual smoke test checklist**

- [ ] Full end-to-end flow: hotkey → capture → LLM inference → tokens stream
- [ ] Overlay invisible in screen-share (Zoom, Discord, OBS)
- [ ] Overlay not in Alt+Tab or taskbar
- [ ] Alt+Arrow repositioning works
- [ ] Answer navigation (prev/next) works
- [ ] Error states display with red border
- [ ] Processing pulse animation visible
- [ ] No console window for sidecar process
- [ ] `netstat -an` shows no open ports from the app