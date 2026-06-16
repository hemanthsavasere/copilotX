import { ElectronAPI } from '@electron-toolkit/preload'

declare global {
  interface Window {
    electron: ElectronAPI
    api: {
      onToken: (callback: (content: string) => void) => void
      onCaptureState: (callback: (state: string, error?: string) => void) => void
      onKeyEvent: (callback: (key: string, shift: boolean, ctrl: boolean) => void) => void
      onInputModeState: (callback: (state: string) => void) => void
      triggerCapture: () => Promise<void>
      sendTextInput: (text: string) => Promise<void>
      stopInputMode: () => Promise<void>
      close: () => Promise<void>
    }
  }
}
