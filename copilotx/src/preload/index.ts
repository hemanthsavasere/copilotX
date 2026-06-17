import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

const api = {
  onToken: (callback: (content: string) => void) =>
    ipcRenderer.on('token', (_event, content) => callback(content)),
  onCaptureState: (callback: (state: string, error?: string) => void) =>
    ipcRenderer.on('capture-state', (_event, state, error) => callback(state, error)),
  onKeyEvent: (callback: (key: string, shift: boolean, ctrl: boolean) => void) =>
    ipcRenderer.on('key-event', (_event, key, shift, ctrl) => callback(key, shift, ctrl)),
  onInputModeState: (callback: (state: string) => void) =>
    ipcRenderer.on('input-mode-state', (_event, state) => callback(state)),
  triggerCapture: () => ipcRenderer.invoke('trigger-capture'),
  sendTextInput: (text: string) => ipcRenderer.invoke('send-text-input', text),
  stopInputMode: () => ipcRenderer.invoke('stop-input-mode'),
  close: () => ipcRenderer.invoke('window-close')
}

contextBridge.exposeInMainWorld('electron', electronAPI)
contextBridge.exposeInMainWorld('api', api)
