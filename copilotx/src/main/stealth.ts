import { BrowserWindow } from 'electron'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let ffiLib: Record<string, any> | null = null

export function applyStealthFlags(win: BrowserWindow): void {
  if (process.platform !== 'win32') return

  try {
    if (!ffiLib) {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      ffiLib = require('ffi-napi')
    }
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    require('ref-napi')
    const ffi = ffiLib

    // @ts-expect-error dynamic ffi
    const user32 = ffi.Library('user32', {
      GetWindowLongPtrW: ['long', ['pointer', 'int']],
      SetWindowLongPtrW: ['long', ['pointer', 'int', 'long']],
      SetWindowPos: ['bool', ['pointer', 'pointer', 'int', 'int', 'int', 'int', 'uint']]
    })

    const GWL_EXSTYLE = -20
    const WS_EX_TOOLWINDOW = 0x00000080
    const WS_EX_NOACTIVATE = 0x08000000
    const SWP_NOMOVE = 0x0002
    const SWP_NOSIZE = 0x0001
    const SWP_NOZORDER = 0x0004
    const SWP_FRAMECHANGED = 0x0020

    const hwnd = win.getNativeWindowHandle()
    const exStyle = user32.GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
    const newStyle = exStyle | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
    user32.SetWindowLongPtrW(hwnd, GWL_EXSTYLE, newStyle)
    user32.SetWindowPos(
      hwnd,
      null,
      0,
      0,
      0,
      0,
      SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED
    )
  } catch {
    console.error('[stealth] Failed to apply Win32 flags (ffi-napi may not be installed)')
  }
}
