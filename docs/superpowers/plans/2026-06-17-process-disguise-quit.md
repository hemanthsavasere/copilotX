# Process Disguise & X Button Quit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make process names blend in with Windows system processes and make the X button fully quit the app instead of hiding it.

**Architecture:** Add `processName` and `sidecarName` string fields to `config.json` and `AppConfig`. The sidecar spawn path reads `sidecarName` from config at runtime. Build scripts (`copy-sidecar.js`, `electron-builder.js`) read these fields at build time. The X button quit is implemented by adding a `close` event handler on the BrowserWindow that calls `app.quit()`, and changing the `window-close` IPC handler from `hide()` to `close()`.

**Tech Stack:** TypeScript, Vitest, Node.js, Electron

---

## File Structure

| File | Change |
|---|---|
| `config/config.json` | Add `processName` and `sidecarName` defaults |
| `src/main/config.ts` | Add fields to `AppConfig`, add validation |
| `src/main/__tests__/config.test.ts` | Add tests for new fields |
| `src/main/ipc.ts` | Add `getSidecarPath`, accept `sidecarName` in `startSidecar` |
| `src/main/__tests__/ipc.test.ts` | Add tests for `getSidecarPath` |
| `src/main/overlay.ts` | Add `close` event handler calling `app.quit()` |
| `src/main/index.ts` | Pass `sidecarName` to `startSidecar`, change `window-close` handler |
| `scripts/copy-sidecar.js` | Read `sidecarName` from config, use as dest filename |
| `electron-builder.js` | Read `processName`/`sidecarName` from config |

---

### Task 1: Add processName and sidecarName to config

**Files:**
- Modify: `copilotx/config/config.json`
- Modify: `copilotx/src/main/config.ts`
- Modify: `copilotx/src/main/__tests__/config.test.ts`

- [ ] **Step 1: Update config.json with new fields**

Replace the contents of `copilotx/config/config.json` with:

```json
{
  "hotkey": "Ctrl+Shift+Space",
  "inputHotkey": "Ctrl+Shift+K",
  "model": "gpt-4o",
  "openaiApiKey": "",
  "anthropicApiKey": "",
  "profile": "interview",
  "overlayOpacity": 0.85,
  "overlayWidth": 320,
  "overlayHeight": 600,
  "overlayPosition": "right",
  "processName": "TaskHostW",
  "sidecarName": "svchost"
}
```

- [ ] **Step 2: Add fields to AppConfig interface and validation in config.ts**

Add `processName: string` and `sidecarName: string` to the `AppConfig` interface, after the `overlayPosition: string` line.

In `validateConfig`, add after the `overlayPosition` validation block (the `validPositions` check):

```typescript
if (!config.processName) {
  errors.push('processName is required')
}

if (/[/\\]/.test(config.processName)) {
  errors.push('processName must not contain path separators')
}

if (!config.sidecarName) {
  errors.push('sidecarName is required')
}

if (/[/\\]/.test(config.sidecarName)) {
  errors.push('sidecarName must not contain path separators')
}
```

- [ ] **Step 3: Write failing tests for new config fields**

Add `processName: 'TaskHostW'` and `sidecarName: 'svchost'` to the `validConfig` object in `config.test.ts`.

Add new test cases after the existing `it('returns error for empty inputHotkey', ...)`:

```typescript
it('returns error for empty processName', () => {
  const config = { ...validConfig, processName: '' }
  const errors = validateConfig(config)
  expect(errors).toContainEqual(expect.stringContaining('processName'))
})

it('returns error for processName with path separators', () => {
  const config = { ...validConfig, processName: 'path/to/exe' }
  const errors = validateConfig(config)
  expect(errors).toContainEqual(expect.stringContaining('processName'))
})

it('returns error for empty sidecarName', () => {
  const config = { ...validConfig, sidecarName: '' }
  const errors = validateConfig(config)
  expect(errors).toContainEqual(expect.stringContaining('sidecarName'))
})

it('returns error for sidecarName with path separators', () => {
  const config = { ...validConfig, sidecarName: 'path\\to\\exe' }
  const errors = validateConfig(config)
  expect(errors).toContainEqual(expect.stringContaining('sidecarName'))
})
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cd copilotx && pnpm run test -- src/main/__tests__/config.test.ts`
Expected: FAIL — `processName` and `sidecarName` not in `AppConfig` type, validation tests fail

- [ ] **Step 5: Implement config changes (apply Step 2 code)**

Apply the interface and validation code from Step 2.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd copilotx && pnpm run test`
Expected: All tests pass (including new ones)

- [ ] **Step 7: Commit**

```bash
cd copilotx
git add config/config.json src/main/config.ts src/main/__tests__/config.test.ts
git commit -m "feat: add processName and sidecarName to config"
```

---

### Task 2: Accept sidecarName in startSidecar

**Files:**
- Modify: `copilotx/src/main/ipc.ts`
- Modify: `copilotx/src/main/__tests__/ipc.test.ts`

- [ ] **Step 1: Write failing test for getSidecarPath**

Add to `copilotx/src/main/__tests__/ipc.test.ts` (add import at top, add describe block after existing tests):

Update the import line:
```typescript
import { getSidecarPath } from '../ipc'
```

Add new describe block:

```typescript
describe('getSidecarPath', () => {
  it('constructs dev path with sidecarName on Linux', () => {
    const result = getSidecarPath('svchost', true, '/resources', '/project/src/main', 'linux')
    expect(result).toBe('/project/sidecar/target/release/svchost')
  })

  it('constructs production path with sidecarName on Linux', () => {
    const result = getSidecarPath('svchost', false, '/app/resources', '', 'linux')
    expect(result).toBe('/app/resources/Svchost')
  })

  it('constructs dev path with sidecarName on Windows', () => {
    const result = getSidecarPath('svchost', true, 'C:\\resources', 'C:\\project\\src\\main', 'win32')
    expect(result).toBe('C:\\project\\sidecar\\target\\release\\svchost.exe')
  })

  it('constructs production path with sidecarName on Windows', () => {
    const result = getSidecarPath('svchost', false, 'C:\\app\\resources', '', 'win32')
    expect(result).toBe('C:\\app\\resources\\svchost.exe')
  })

  it('falls back to system-helper when sidecarName is empty', () => {
    const result = getSidecarPath('', false, '/resources', '', 'linux')
    expect(result).toBe('/resources/system-helper')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd copilotx && pnpm run test -- src/main/__tests__/ipc.test.ts`
Expected: FAIL — `getSidecarPath` is not exported from `ipc.ts`

- [ ] **Step 3: Implement getSidecarPath and update startSidecar in ipc.ts**

Add a `currentSidecarName` module-level variable after the existing module-level variables:

```typescript
let currentSidecarName: string = 'system-helper'
```

Add the `getSidecarPath` exported function (before `startSidecar`):

```typescript
export function getSidecarPath(
  sidecarName: string,
  isDev: boolean,
  resourcesPath: string,
  dirname: string,
  platform: string
): string {
  const exeExt = platform === 'win32' ? '.exe' : ''
  const name = sidecarName || 'system-helper'
  return isDev
    ? path.join(dirname, `../../sidecar/target/release/${name}${exeExt}`)
    : path.join(resourcesPath, `${name}${exeExt}`)
}
```

Update `startSidecar` signature to accept `sidecarName` and use `getSidecarPath`. Replace the entire `startSidecar` function:

```typescript
export function startSidecar(sidecarName?: string): void {
  if (sidecarName) {
    currentSidecarName = sidecarName
  }

  if (restartTimer) {
    clearTimeout(restartTimer)
    restartTimer = null
  }

  if (sidecar?.pid && !sidecar.killed) {
    return
  }

  restartAttempts = 0

  const sidecarPath = getSidecarPath(
    currentSidecarName,
    is.dev,
    process.resourcesPath,
    __dirname,
    process.platform
  )

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
```

Also update the `handleSidecarExit` function to pass `currentSidecarName` when calling `startSidecar`:

```typescript
function handleSidecarExit(code: number | null, signal: string | null): void {
  console.error(`[sidecar] exited with code=${code} signal=${signal}`)
  sidecar = null

  if (restartAttempts < MAX_RESTART_ATTEMPTS) {
    restartAttempts++
    console.log(`[sidecar] Restarting (attempt ${restartAttempts}/${MAX_RESTART_ATTEMPTS})...`)
    restartTimer = setTimeout(() => startSidecar(currentSidecarName), 2000 * restartAttempts)
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd copilotx && pnpm run test -- src/main/__tests__/ipc.test.ts`
Expected: All `getSidecarPath` tests pass

- [ ] **Step 5: Run full test suite**

Run: `cd copilotx && pnpm run test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
cd copilotx
git add src/main/ipc.ts src/main/__tests__/ipc.test.ts
git commit -m "feat: make startSidecar accept configurable sidecarName"
```

---

### Task 3: X button triggers full quit

**Files:**
- Modify: `copilotx/src/main/overlay.ts`
- Modify: `copilotx/src/main/index.ts`

**Note:** The spec says to remove the `window-close` IPC handler and add a `close` event on the BrowserWindow. However, the renderer close button uses `window.api.close()` → `ipcRenderer.invoke('window-close')` (in `src/preload/index.ts`). Removing the IPC handler would break the close button, and the spec says NOT to modify the preload. So we keep the IPC channel but change its behavior from `hide()` to `close()`, which triggers the BrowserWindow `close` event handler.

- [ ] **Step 1: Add close event handler in overlay.ts**

Change the import from:
```typescript
import { BrowserWindow, screen } from 'electron'
```
to:
```typescript
import { app, BrowserWindow, screen } from 'electron'
```

Add after `win.setContentProtection(true)` (after line 35):

```typescript
win.on('close', () => {
  app.quit()
})
```

- [ ] **Step 2: Change window-close IPC handler and pass sidecarName in index.ts**

Change the `window-close` handler from:
```typescript
ipcMain.handle('window-close', () => {
  overlayWindow?.hide()
})
```
to:
```typescript
ipcMain.handle('window-close', () => {
  overlayWindow?.close()
})
```

Change the `startSidecar()` call from:
```typescript
startSidecar()
```
to:
```typescript
startSidecar(config.sidecarName)
```

- [ ] **Step 3: Run typecheck to verify changes compile**

Run: `cd copilotx && pnpm run typecheck`
Expected: No type errors

- [ ] **Step 4: Run full test suite**

Run: `cd copilotx && pnpm run test`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
cd copilotx
git add src/main/overlay.ts src/main/index.ts
git commit -m "feat: X button triggers full quit instead of hiding window"
```

---

### Task 4: Build pipeline reads config names

**Files:**
- Modify: `copilotx/scripts/copy-sidecar.js`
- Modify: `copilotx/electron-builder.js`

- [ ] **Step 1: Update copy-sidecar.js to read sidecarName from config**

Replace the contents of `copilotx/scripts/copy-sidecar.js` with:

```javascript
const fs = require('fs')
const path = require('path')

const config = require('../config/config.json')
const sidecarName = config.sidecarName || 'system-helper'

const targetTriple = process.env.TARGET_TRIPLE || ''
const targetDir = targetTriple
  ? path.join('sidecar', 'target', targetTriple, 'release')
  : path.join('sidecar', 'target', 'release')

const resourcesDir = path.join(__dirname, '..', 'resources')
if (!fs.existsSync(resourcesDir)) {
  fs.mkdirSync(resourcesDir, { recursive: true })
}

const exeExt = process.env.TARGET_OS
  ? process.env.TARGET_OS === 'windows'
  : targetTriple.includes('windows')
    ? '.exe'
    : process.platform === 'win32'
      ? '.exe'
      : ''

const srcPath = path.join(__dirname, '..', targetDir, 'system-helper' + exeExt)
const sidecarDest = path.join(resourcesDir, sidecarName + exeExt)

if (fs.existsSync(srcPath)) {
  fs.copyFileSync(srcPath, sidecarDest)
  console.log(`Copied ${srcPath} -> ${sidecarDest}`)
} else {
  console.error(`Sidecar binary not found at ${srcPath}. Build it first.`)
  process.exit(1)
}
```

- [ ] **Step 2: Update electron-builder.js to read names from config**

The source filename stays `system-helper` (it's the Cargo binary name). Only the **destination** filename changes to use `sidecarName` from config:

```javascript
const config = require('./config/config.json')

module.exports = {
  appId: 'com.copilotx',
  productName: 'CopilotX',
  directories: {
    buildResources: 'build',
    output: 'dist'
  },
  files: [
    'out/**/*',
    'resources/**/*',
    '!**/.vscode/*',
    '!src/*',
    '!sidecar/*',
    '!{.eslintcache,.prettierrc.yaml,dev-app-update.yml}',
    '!{.env,.env.*,.npmrc,pnpm-lock.yaml}',
    '!{tsconfig*.json,electron.vite.config.*}'
  ],
  win: {
    executableName: config.processName || 'CopilotX',
    target: ['zip'],
    extraResources: [
      {
        from: `resources/${config.sidecarName || 'svchost'}.exe`,
        to: `${config.sidecarName || 'svchost'}.exe`
      },
      {
        from: 'config/config.json',
        to: 'config.json'
      }
    ]
  },
  npmRebuild: false,
  linux: {
    executableName: 'copilotx',
    target: ['AppImage', 'deb'],
    category: 'Utility',
    maintainer: 'CopilotX Team',
    extraResources: [
      {
        from: 'resources/system-helper',
        to: 'system-helper'
      },
      {
        from: 'config/config.json',
        to: 'config.json'
      }
    ]
  }
}
```

- [ ] **Step 3: Verify scripts can read config**

Run: `cd copilotx && node -e "const c = require('./config/config.json'); console.log('processName:', c.processName, 'sidecarName:', c.sidecarName)"`
Expected: `processName: TaskHostW sidecarName: svchost`

- [ ] **Step 4: Commit**

```bash
cd copilotx
git add scripts/copy-sidecar.js electron-builder.js
git commit -m "feat: build pipeline reads processName and sidecarName from config"
```

---

## Self-Review

### Spec Coverage

| Spec Requirement | Task |
|---|---|
| `processName` / `sidecarName` fields in config.json | Task 1 |
| `processName` default `"TaskHostW"`, `sidecarName` default `"svchost"` | Task 1 |
| `AppConfig` interface updated | Task 1 |
| `copy-sidecar.js` reads `sidecarName`, copies as `<sidecarName>.exe` | Task 4 |
| `electron-builder.js` reads `processName` for `win.executableName` | Task 4 |
| `electron-builder.js` uses `sidecarName` in `win.extraResources` | Task 4 |
| `ipc.ts` accepts `sidecarName` for spawn path | Task 2 |
| `overlay.ts` adds `close` event → `app.quit()` | Task 3 |
| `index.ts` removes/replaces `window-close` IPC handler | Task 3 |
| Linux build config unchanged (aside from config schema) | Task 4 |
| No changes to sidecar Rust, preload, renderer | ✓ |

### Placeholder Scan

No TBDs, TODOs, "implement later", or "similar to Task N" patterns found. ✓

### Type Consistency

- `sidecarName` is `string` throughout all tasks
- `startSidecar(sidecarName?: string)` matches `config.sidecarName` (string)
- `getSidecarPath` returns string, consumed by `spawn()`
- `processName` string used as `win.executableName`
- Config validation matches interface fields ✓

### Deviation from Spec

The spec says "Remove the `window-close` IPC handler" and also says "No changes to preload bridge." These are contradictory — removing the IPC handler breaks the renderer close button since `preload/index.ts` still calls `ipcRenderer.invoke('window-close')`. Instead, I kept the IPC handler but changed it from `overlayWindow?.hide()` to `overlayWindow?.close()`, which triggers the BrowserWindow `close` event → `app.quit()`. Same end behavior, no preload changes needed.