import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

const api = {
  onToken: (callback: (content: string) => void) =>
    ipcRenderer.on('token', (_event, content) => callback(content)),
  onCaptureState: (callback: (state: string, error?: string) => void) =>
    ipcRenderer.on('capture-state', (_event, state, error) => callback(state, error)),
  triggerCapture: () => ipcRenderer.invoke('trigger-capture'),
  close: () => ipcRenderer.invoke('window-close')
}

if (process.contextIsolated) {
  contextBridge.exposeInMainWorld('electron', electronAPI)
  contextBridge.exposeInMainWorld('api', api)
} else {
  // @ts-expect-error fallback for non-isolated context
  window.electron = electronAPI
  // @ts-expect-error fallback for non-isolated context
  window.api = api
}
