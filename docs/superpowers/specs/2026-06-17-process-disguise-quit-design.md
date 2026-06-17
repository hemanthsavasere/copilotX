# Process Name Disguise & X Button Quit

**Date:** 2026-06-17
**Status:** Draft

## Problem

1. **Process names are conspicuous** — "CopilotX.exe" and "system-helper.exe" stand out in Task Manager during interviews/meetings, defeating the app's stealth purpose.
2. **X button doesn't quit** — clicking the close button hides the window but leaves both the Electron process and the sidecar running, requiring manual Task Manager termination.

## Solution

### 1. Configurable Process Name Disguise

Add `processName` and `sidecarName` fields to `config.json`. These control the executable filenames at build time, so the processes blend in with Windows system processes in Task Manager.

**Config schema additions:**
- `processName` (string, default: `"TaskHostW"`) — Electron exe name on Windows
- `sidecarName` (string, default: `"svchost"`) — Sidecar exe name on Windows

These settings are **Windows-only**. Linux builds use existing names (`copilotx`, `system-helper`).

**Default result in Task Manager:**
| Process | Before | After |
|---|---|---|
| Electron app | CopilotX.exe | TaskHostW.exe |
| Sidecar | system-helper.exe | svchost.exe |

#### Build Pipeline Changes

**`copy-sidecar.js`:**
- Read `sidecarName` from `config/config.json`
- Copy the compiled binary as `<sidecarName>.exe` instead of `system-helper.exe`
- Example: copies `system-helper.exe` → `resources/svchost.exe`

**`electron-builder.js`:**
- Read `processName` from `config/config.json`
- Set `win.executableName` to `processName` value instead of hardcoded `'CopilotX'`
- Update `win.extraResources` to reference `<sidecarName>.exe` instead of `system-helper.exe`

#### Runtime Changes

**`src/main/config.ts`:**
- Add `processName` and `sidecarName` to `AppConfig` interface

**`src/main/ipc.ts`:**
- Accept `sidecarName` as a parameter in `startSidecar()` (passed from `index.ts` which has the config)
- Use it when constructing the sidecar spawn path instead of hardcoded `system-helper`

**No changes to:**
- Sidecar Rust code (it doesn't know its own filename)
- Preload/bridge layer
- Renderer

### 2. X Button = Full Quit

**Current behavior:** The `window-close` IPC handler calls `overlayWindow?.hide()`, leaving both processes running.

**New behavior:** Clicking X triggers full app shutdown — stops sidecar, unregisters hotkeys, and exits.

**`src/main/overlay.ts`:**
- Add `close` event handler on the BrowserWindow that calls `app.quit()`

**`src/main/index.ts`:**
- Remove the `window-close` IPC handler (no longer needed)

**Quit flow:**
1. User clicks X
2. Window `close` event fires
3. `app.quit()` is called
4. `before-quit` handler fires → stops sidecar, unregisters hotkeys
5. App exits cleanly

## Files Changed

| File | Change |
|---|---|
| `config/config.json` | Add `processName`, `sidecarName` fields |
| `src/main/config.ts` | Add fields to `AppConfig` interface |
| `src/main/ipc.ts` | Read `sidecarName` for spawn path |
| `src/main/overlay.ts` | Add `close` → `app.quit()` handler |
| `src/main/index.ts` | Remove `window-close` IPC handler |
| `scripts/copy-sidecar.js` | Read `sidecarName` from config, use as filename |
| `electron-builder.js` | Read `processName` from config, use as `executableName` and in `extraResources` |

## Files NOT Changed

- Sidecar Rust code (`sidecar/src/`)
- Preload bridge (`src/preload/`)
- Renderer (`src/renderer/`)
- Linux build configuration (aside from config schema additions which are inert)
