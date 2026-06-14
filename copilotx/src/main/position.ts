import { BrowserWindow, screen, globalShortcut } from 'electron'

type Position = 'left' | 'right' | 'top' | 'bottom'

const POSITION_HOTKEYS: Record<Position, string> = {
  left: 'Alt+Left',
  right: 'Alt+Right',
  top: 'Alt+Up',
  bottom: 'Alt+Down'
}

export function registerPositionHotkeys(window: BrowserWindow, overlayWidth: number, overlayHeight: number): void {
  const moveTo = (position: Position) => {
    const { width: screenWidth, height: screenHeight } = screen.getPrimaryDisplay().workAreaSize
    const vy = Math.floor((screenHeight - overlayHeight) / 2)

    switch (position) {
      case 'right':
        window.setPosition(screenWidth - overlayWidth, vy)
        break
      case 'left':
        window.setPosition(0, vy)
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
          screenHeight - overlayHeight
        )
        break
    }
  }

  for (const [pos, accelerator] of Object.entries(POSITION_HOTKEYS)) {
    globalShortcut.register(accelerator, () => moveTo(pos as Position))
  }
}
