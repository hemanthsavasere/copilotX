import { BrowserWindow, screen } from 'electron'
import { join } from 'path'
import { is } from '@electron-toolkit/utils'
import type { AppConfig } from './config'

export function createOverlayWindow(config: AppConfig): BrowserWindow {
  const primaryDisplay = screen.getPrimaryDisplay()
  const { width: screenWidth, height: screenHeight } = primaryDisplay.workAreaSize
  const overlayWidth = config.overlayWidth || 320
  const overlayHeight = config.overlayHeight || 600

  const win = new BrowserWindow({
    width: overlayWidth,
    height: overlayHeight,
    x: screenWidth - overlayWidth,
    y: Math.floor((screenHeight - overlayHeight) / 2),
    alwaysOnTop: true,
    frame: false,
    transparent: true,
    backgroundColor: '#00000000',
    type: 'toolbar',
    skipTaskbar: true,
    resizable: false,
    hasShadow: false,
    focusable: false,
    show: true,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      backgroundThrottling: false
    }
  })

  win.setAlwaysOnTop(true, 'screen-saver')
  win.setContentProtection(true)

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    win.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }

  return win
}
