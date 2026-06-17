import { spawn, ChildProcess } from 'child_process'
import { createInterface } from 'readline'
import * as path from 'path'
import { is } from '@electron-toolkit/utils'

export interface SidecarMessage {
  type: 'token' | 'done' | 'error' | 'pong' | 'key_event' | 'input_mode_state'
  content?: string
  message?: string
  key?: string
  shift?: boolean
  ctrl?: boolean
  state?: string
}

export type SidecarMessageHandler = (msg: SidecarMessage) => void

let sidecar: ChildProcess | null = null
let messageHandler: SidecarMessageHandler | null = null
let restartAttempts = 0
let restartTimer: ReturnType<typeof setTimeout> | null = null
const MAX_RESTART_ATTEMPTS = 3
let currentSidecarName: string = 'system-helper'

function handleSidecarExit(code: number | null, signal: string | null): void {
  console.error(`[sidecar] exited with code=${code} signal=${signal}`)
  sidecar = null

  if (restartAttempts < MAX_RESTART_ATTEMPTS) {
    restartAttempts++
    console.log(`[sidecar] Restarting (attempt ${restartAttempts}/${MAX_RESTART_ATTEMPTS})...`)
    restartTimer = setTimeout(() => startSidecar(currentSidecarName), 2000 * restartAttempts)
  }
}

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

export function stopSidecar(): Promise<void> {
  if (restartTimer) {
    clearTimeout(restartTimer)
    restartTimer = null
  }

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

    const onExit = () => {
      clearTimeout(timeout)
      resolve()
    }

    sidecar.once('exit', onExit)
  })
}

export function sendCapture(): void {
  writeSidecar({ type: 'capture' })
}

export function sendStartInputMode(): void {
  writeSidecar({ type: 'start_input_mode' })
}

export function sendStopInputMode(): void {
  writeSidecar({ type: 'stop_input_mode' })
}

export function sendCaptureWithText(content: string): void {
  writeSidecar({ type: 'capture_with_text', content })
}

export function onSidecarMessage(handler: SidecarMessageHandler): void {
  messageHandler = handler
}

function writeSidecar(msg: Record<string, unknown>): void {
  if (!sidecar?.stdin || sidecar.stdin.destroyed) return
  sidecar.stdin.write(JSON.stringify(msg) + '\n')
}
