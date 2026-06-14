import { BrowserWindow } from 'electron'

export enum AppError {
  SIDECAR_SPAWN_FAILED = 'sidecar_spawn_failed',
  SIDECAR_CRASHED = 'sidecar_crashed',
  HOTKEY_REGISTRATION_FAILED = 'hotkey_registration_failed',
  CONFIG_LOAD_FAILED = 'config_load_failed',
  API_KEY_MISSING = 'api_key_missing',
  NETWORK_ERROR = 'network_error',
}

export function showError(window: BrowserWindow | null, error: AppError, detail?: string): void {
  if (!window) return
  window.webContents.send('capture-state', 'error', `${error}: ${detail || ''}`)
}
