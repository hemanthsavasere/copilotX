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
let restartTimer: ReturnType<typeof setTimeout> | null = null
const MAX_RESTART_ATTEMPTS = 3

function handleSidecarExit(code: number | null, signal: string | null): void {
  console.error(`[sidecar] exited with code=${code} signal=${signal}`)
  sidecar = null

  if (restartAttempts < MAX_RESTART_ATTEMPTS) {
    restartAttempts++
    console.log(`[sidecar] Restarting (attempt ${restartAttempts}/${MAX_RESTART_ATTEMPTS})...`)
    restartTimer = setTimeout(() => startSidecar(), 2000 * restartAttempts)
  }
}

export function startSidecar(): void {
  if (restartTimer) {
    clearTimeout(restartTimer)
    restartTimer = null
  }

  if (sidecar?.pid && !sidecar.killed) {
    return
  }

  restartAttempts = 0

  const exeExt = process.platform === 'win32' ? '.exe' : ''
  const sidecarPath = is.dev
    ? path.join(__dirname, `../../sidecar/target/release/system-helper${exeExt}`)
    : path.join(process.resourcesPath, `system-helper${exeExt}`)

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

export function sendPing(): void {
  writeSidecar({ type: 'ping' })
}

export function onSidecarMessage(handler: SidecarMessageHandler): void {
  messageHandler = handler
}

function writeSidecar(msg: { type: string }): void {
  if (!sidecar?.stdin || sidecar.stdin.destroyed) return
  sidecar.stdin.write(JSON.stringify(msg) + '\n')
}
