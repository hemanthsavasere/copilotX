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
    window.show()
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
