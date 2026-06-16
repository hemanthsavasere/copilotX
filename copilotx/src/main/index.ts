import { app, BrowserWindow, ipcMain } from 'electron'
import { electronApp } from '@electron-toolkit/utils'
import { createOverlayWindow } from './overlay'
import { startSidecar, stopSidecar, onSidecarMessage, sendCapture, sendStopInputMode, sendCaptureWithText } from './ipc'
import { loadConfig, validateConfig } from './config'
import { registerHotkey, setProcessingComplete, unregisterAll, registerInputHotkey, isInInputMode, setInputModeInactive } from './hotkey'
import { registerPositionHotkeys } from './position'

let overlayWindow: BrowserWindow | null = null

app.whenReady().then(() => {
  electronApp.setAppUserModelId('com.copilotx')

  let config
  try {
    config = loadConfig()
  } catch (e) {
    console.error('Failed to load config:', e)
    app.quit()
    return
  }

  const errors = validateConfig(config)
  if (errors.length > 0) {
    console.warn('Config warnings:', errors.join('; '))
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
      case 'key_event':
        if (isInInputMode()) {
          overlayWindow.webContents.send('key-event', msg.key, msg.shift, msg.ctrl)
        }
        break
      case 'input_mode_state':
        if (msg.state === 'inactive' || msg.state === 'error') {
          setInputModeInactive()
          overlayWindow.webContents.send('input-mode-state', msg.state)
        }
        break
    }
  })

  registerHotkey(config.hotkey, overlayWindow)
  registerInputHotkey(config.inputHotkey, overlayWindow)
  registerPositionHotkeys(overlayWindow, config.overlayWidth, config.overlayHeight)

  ipcMain.handle('trigger-capture', () => {
    if (!overlayWindow) return
    overlayWindow.webContents.send('capture-state', 'processing')
    sendCapture()
  })

  ipcMain.handle('window-close', () => {
    overlayWindow?.hide()
  })

  ipcMain.handle('send-text-input', (_event, text: string) => {
    if (!overlayWindow) return
    overlayWindow.webContents.send('capture-state', 'processing')
    sendCaptureWithText(text)
    setInputModeInactive()
  })

  ipcMain.handle('stop-input-mode', () => {
    sendStopInputMode()
    setInputModeInactive()
  })

  overlayWindow.webContents.on('dom-ready', () => {
    overlayWindow?.webContents.executeJavaScript(
      `document.documentElement.style.setProperty('--overlay-bg', 'rgba(20, 20, 30, ${config.overlayOpacity})')`
    )
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

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
