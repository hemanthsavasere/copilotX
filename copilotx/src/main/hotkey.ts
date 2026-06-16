import { globalShortcut, BrowserWindow } from 'electron'
import { sendCapture, sendStartInputMode } from './ipc'

let isProcessing = false
let isInputMode = false

export function registerHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isInputMode) return
    if (isProcessing) {
      window.webContents.send('capture-state', 'already-processing')
      return
    }

    isProcessing = true
    window.show()
    window.webContents.send('capture-state', 'processing')
    sendCapture()
  })

  if (!registered) {
    console.error(`Failed to register hotkey: ${accelerator}`)
  }

  return registered
}

export function registerInputHotkey(accelerator: string, window: BrowserWindow): boolean {
  const registered = globalShortcut.register(accelerator, () => {
    if (isProcessing || isInputMode) return
    isInputMode = true
    sendStartInputMode()
    window.webContents.send('input-mode-state', 'active')
  })

  if (!registered) {
    console.error(`Failed to register input hotkey: ${accelerator}`)
  }

  return registered
}

export function setProcessingComplete(): void {
  isProcessing = false
}

export function setInputModeActive(): void {
  isInputMode = true
}

export function setInputModeInactive(): void {
  isInputMode = false
}

export function isInInputMode(): boolean {
  return isInputMode
}

export function unregisterAll(): void {
  globalShortcut.unregisterAll()
}
