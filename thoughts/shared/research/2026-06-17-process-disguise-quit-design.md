---
date: 2026-06-17T08:10:40+00:00
researcher: opencode
git_commit: 3d8d615c4283babb70bf6129061b49689645c57c
branch: ponytail-trim
repository: copilotX
topic: "Process Name Disguise & X Button Quit - Current Codebase Research"
tags: [research, codebase, config, sidecar, electron-builder, overlay, ipc, stealth, process-naming, quit-flow]
status: complete
last_updated: 2026-06-17
last_updated_by: opencode
---

# Research: Process Name Disguise & X Button Quit - Current Codebase State

**Date**: 2026-06-17T08:10:40+00:00
**Researcher**: opencode
**Git Commit**: 3d8d615c4283babb70bf6129061b49689645c57c
**Branch**: ponytail-trim
**Repository**: copilotX

## Research Question

What is the current state of the codebase as it relates to the spec at `docs/superpowers/specs/2026-06-17-process-disguise-quit-design.md`, which proposes: (1) configurable process name disguise for Windows builds, and (2) X button triggering full quit?

## Summary

The codebase currently hardcodes the sidecar binary name as `system-helper` across 6 files (~20 locations) and the Electron executable name as `CopilotX` in `electron-builder.js`. The `AppConfig` interface has no `processName` or `sidecarName` fields. The sidecar Rust code has no awareness of its own filename. The X button currently hides the overlay window via an IPC handler (`window-close` → `overlayWindow.hide()`), and there is no `close` event handler on the BrowserWindow itself. However, a `before-quit` handler already implements graceful shutdown (stops sidecar, unregisters hotkeys) using a `canQuit` flag pattern.

## Detailed Findings

### 1. Config Schema (`copilotx/config/config.json`)

The template config file has 11 fields. No `processName` or `sidecarName` fields exist.

| Field | Type | Default |
|---|---|---|
| `hotkey` | string | `"Ctrl+Shift+Space"` |
| `inputHotkey` | string | `"Ctrl+Shift+K"` |
| `model` | string | `"gpt-4o"` |
| `openaiApiKey` | string | `""` |
| `anthropicApiKey` | string | `""` |
| `profile` | string | `"interview"` |
| `overlayOpacity` | number | `0.85` |
| `overlayWidth` | number | `320` |
| `overlayHeight` | number | `600` |
| `overlayPosition` | string | `"right"` |

This file is bundled as an `extraResource` by electron-builder and copied to the user's `userData` directory on first launch by `loadConfig()`.

### 2. AppConfig Interface (`copilotx/src/main/config.ts`)

The `AppConfig` interface at lines 5-16 mirrors the config.json exactly with the same 11 fields. The `loadConfig()` function (lines 18-39) reads from `app.getPath('userData')/config.json`, falling back to copying the template from `process.resourcesPath/config.json`. The `validateConfig()` function (lines 41-93) validates all fields with specific rules.

No `processName` or `sidecarName` fields exist in the interface.

### 3. Sidecar Spawning (`copilotx/src/main/ipc.ts`)

The `startSidecar()` function (lines 35-79) constructs the sidecar path with hardcoded `system-helper`:

```typescript
// Line 47-50
const exeExt = process.platform === 'win32' ? '.exe' : ''
const sidecarPath = is.dev
  ? path.join(__dirname, `../../sidecar/target/release/system-helper${exeExt}`)
  : path.join(process.resourcesPath, `system-helper${exeExt}`)
```

The function spawns the binary at line 52-55 with `windowsHide: true` and communicates via NDJSON over stdin/stdout. Auto-restart logic exists (max 3 attempts with increasing delays).

The `stopSidecar()` function (lines 81-107) sends a `{ type: 'shutdown' }` command and waits up to 3 seconds before killing with SIGTERM.

### 4. Window Close Behavior (`copilotx/src/main/index.ts`)

The `window-close` IPC handler at lines 71-73:

```typescript
ipcMain.handle('window-close', () => {
  overlayWindow?.hide()
})
```

This hides the window without destroying it. The window can be re-shown later by the hotkey handler. There is no `close` event handler on the BrowserWindow itself.

The `before-quit` handler at lines 98-108 already implements graceful shutdown:

```typescript
let canQuit = false
app.on('before-quit', async (event) => {
  if (!canQuit) {
    event.preventDefault()
    await stopSidecar()
    unregisterAll()
    canQuit = true
    app.quit()
  }
})
```

This stops the sidecar and unregisters all global shortcuts before exiting.

### 5. Overlay Window (`copilotx/src/main/overlay.ts`)

The `createOverlayWindow()` function (lines 6-43) creates a frameless, transparent, always-on-top BrowserWindow. Notable configuration:

- `frame: false` (no title bar)
- `skipTaskbar: true`
- `focusable: false`
- `setAlwaysOnTop(true, 'screen-saver')` (highest z-order)
- `setContentProtection(true)` (prevents screen capture)

**There is NO `close` event handler** registered on the window. The function simply creates and returns the BrowserWindow instance.

### 6. Sidecar Copy Script (`copilotx/scripts/copy-sidecar.js`)

This build-time script (31 lines) copies the compiled Rust binary from `sidecar/target/<triple>/release/` to `resources/`. Both source and destination use hardcoded `system-helper`:

```javascript
// Lines 22-23
const srcPath = path.join(__dirname, '..', targetDir, 'system-helper' + exeExt)
const sidecarDest = path.join(resourcesDir, 'system-helper' + exeExt)
```

The `.exe` extension is determined by checking `TARGET_OS` env var, `TARGET_TRIPLE` containing "windows", or `process.platform === 'win32'`.

npm scripts reference it: `copy:sidecar` and `copy:sidecar:win` (with `TARGET_TRIPLE=x86_64-pc-windows-gnu`).

### 7. Electron Builder Config (`copilotx/electron-builder.js`)

Windows config (lines 18-31):
- `executableName: "CopilotX"` (line 19) — the name of the .exe file
- `extraResources` includes `resources/system-helper.exe` → `system-helper.exe` (lines 22-25)
- `extraResources` includes `config/config.json` → `config.json` (lines 26-29)

Linux config (lines 33-48):
- `executableName: "copilotx"` (line 34) — lowercase convention
- `extraResources` includes `resources/system-helper` → `system-helper` (lines 39-42)
- `extraResources` includes `config/config.json` → `config.json` (lines 43-46)

### 8. Sidecar Rust Code (`copilotx/sidecar/`)

The sidecar has **NO awareness of its own filename or process name**:
- No `std::env::current_exe()` calls
- No `std::env::args()` calls
- No `build.rs` script
- Binary name `system-helper` is defined in `Cargo.toml` lines 2 and 7

The `Cargo.toml` package and binary name:
```toml
name = "system-helper"
[[bin]]
name = "system-helper"
path = "src/main.rs"
```

Integration tests at `sidecar/tests/integration.rs` reference `system-helper` in 6 locations via `Command::cargo_bin("system-helper")`.

### 9. Preload Bridge (`copilotx/src/preload/index.ts`)

The preload script exposes `window.api.close()` (line 16) which invokes the `window-close` IPC handler. If the `window-close` IPC handler is removed per the spec, this method would become a dead reference in the preload layer.

## Code References

- `copilotx/config/config.json:1-12` - Template config with 11 fields, no processName/sidecarName
- `copilotx/src/main/config.ts:5-16` - AppConfig interface matching config.json
- `copilotx/src/main/config.ts:18-39` - loadConfig() function
- `copilotx/src/main/config.ts:41-93` - validateConfig() function
- `copilotx/src/main/ipc.ts:47-50` - Hardcoded `system-helper` in sidecar path construction
- `copilotx/src/main/ipc.ts:35-79` - startSidecar() function
- `copilotx/src/main/ipc.ts:81-107` - stopSidecar() function with shutdown + SIGTERM
- `copilotx/src/main/index.ts:71-73` - `window-close` IPC handler → `overlayWindow.hide()`
- `copilotx/src/main/index.ts:98-108` - `before-quit` handler with canQuit pattern
- `copilotx/src/main/overlay.ts:6-43` - createOverlayWindow() with no close event handler
- `copilotx/scripts/copy-sidecar.js:22-23` - Hardcoded `system-helper` in copy paths
- `copilotx/electron-builder.js:19` - `executableName: "CopilotX"`
- `copilotx/electron-builder.js:22-25` - Windows extraResources for system-helper.exe
- `copilotx/electron-builder.js:39-42` - Linux extraResources for system-helper
- `copilotx/sidecar/Cargo.toml:2` - `name = "system-helper"`
- `copilotx/sidecar/Cargo.toml:7` - `[[bin]] name = "system-helper"`
- `copilotx/sidecar/tests/integration.rs:20,32,44,61,73,85` - `Command::cargo_bin("system-helper")`
- `copilotx/src/preload/index.ts:16` - `close()` method invoking `window-close` IPC

## Architecture Documentation

### Config Flow
```
config/config.json (template)
    → electron-builder.js extraResources
    → Built app's resources/config.json
    → loadConfig() copies to userData/config.json on first launch
    → Reads and parses as AppConfig
    → validateConfig() checks constraints
```

### Sidecar Binary Flow
```
sidecar/Cargo.toml defines binary name "system-helper"
    → cargo build --release produces target/release/system-helper[.exe]
    → scripts/copy-sidecar.js copies to resources/system-helper[.exe]
    → electron-builder.js extraResources packages into built app
    → ipc.ts startSidecar() spawns from process.resourcesPath/system-helper[.exe]
```

### Quit Flow (Current)
```
User clicks X in overlay
    → Renderer calls window.api.close()
    → Preload invokes 'window-close' IPC
    → index.ts handler calls overlayWindow.hide()
    → Window is hidden, both processes remain running

App quit (e.g., via app.quit()):
    → before-quit event fires
    → event.preventDefault() on first call
    → stopSidecar() sends shutdown, waits 3s, SIGTERM if needed
    → unregisterAll() releases global shortcuts
    → canQuit = true, app.quit() called again
    → App exits
```

### Hardcoded Process Names Summary

| Location | Hardcoded Value | Context |
|---|---|---|
| `copilotx/electron-builder.js:19` | `"CopilotX"` | Windows executable name |
| `copilotx/electron-builder.js:34` | `"copilotx"` | Linux executable name |
| `copilotx/src/main/ipc.ts:49-50` | `"system-helper"` | Sidecar binary name in spawn path |
| `copilotx/scripts/copy-sidecar.js:22-23` | `"system-helper"` | Sidecar copy source/dest |
| `copilotx/electron-builder.js:23,40` | `"system-helper"` | extraResources from path |
| `copilotx/sidecar/Cargo.toml:2,7` | `"system-helper"` | Rust package & binary name |

## Historical Context (from thoughts/)

- `thoughts/shared/plans/2026-06-13-copilotx-mvp.md` - MVP plan covering stealth window properties, sidecar architecture, process naming strategy (sidecar named `system-helper.exe` for anti-detection), and quit flow (shutdown via `will-quit` event)
- `thoughts/shared/plans/2026-06-16-stealth-text-input.md` - Stealth text input plan with keyboard hook details, sidecar protocol extensions, and Windows-specific build configuration

## Related Research

No prior research documents found in `thoughts/shared/research/`.

## Open Questions

- The preload bridge exposes `window.api.close()` which invokes the `window-close` IPC handler. The spec says "no changes to preload/bridge layer," but removing the `window-close` IPC handler from index.ts would leave `close()` as a dead method in the preload layer. Should this be cleaned up or left as-is?
- The sidecar integration tests reference `Command::cargo_bin("system-helper")` in 6 places. If the Cargo.toml binary name changes, these would need updating — but the spec says no Rust code changes, and the Cargo.toml binary name would remain `system-helper` (the rename only happens at the copy step in `copy-sidecar.js`).
- The `window-all-closed` handler at `copilotx/src/main/index.ts:94-96` calls `app.quit()` on non-macOS platforms. If the X button triggers `app.quit()` via the close event, this handler would also fire — need to verify the quit flow doesn't double-exit.
