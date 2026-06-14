import { ElectronAPI } from '@electron-toolkit/preload'

declare global {
  interface Window {
    electron: ElectronAPI
    api: {
      onToken: (callback: (content: string) => void) => void
      onCaptureState: (callback: (state: string, error?: string) => void) => void
      triggerCapture: () => Promise<void>
      close: () => Promise<void>
    }
  }
}
