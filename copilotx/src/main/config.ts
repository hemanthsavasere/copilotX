import { readFileSync, existsSync, mkdirSync, copyFileSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'

export interface AppConfig {
  hotkey: string
  inputHotkey: string
  model: string
  openaiApiKey: string
  anthropicApiKey: string
  profile: string
  overlayOpacity: number
  overlayWidth: number
  overlayHeight: number
  overlayPosition: string
}

export function loadConfig(): AppConfig {
  const configPath = join(app.getPath('userData'), 'config.json')

  if (!existsSync(configPath)) {
    const templatePath = join(process.resourcesPath, 'config.json')
    if (existsSync(templatePath)) {
      const userDir = app.getPath('userData')
      if (!existsSync(userDir)) mkdirSync(userDir, { recursive: true })
      copyFileSync(templatePath, configPath)
    }
  }

  if (!existsSync(configPath)) {
    throw new Error(`Config file not found: ${configPath}`)
  }
  const content = readFileSync(configPath, 'utf-8')
  try {
    return JSON.parse(content) as AppConfig
  } catch {
    throw new Error(`Failed to parse config.json at ${configPath}`)
  }
}

export function validateConfig(config: AppConfig): string[] {
  const errors: string[] = []

  if (!config.model) {
    errors.push('model is required')
  } else if (!['gpt-4o', 'claude', 'claude-sonnet'].includes(config.model)) {
    errors.push(`Unknown model: ${config.model}. Supported: gpt-4o, claude, claude-sonnet`)
  }

  if (config.model === 'gpt-4o' && !config.openaiApiKey) {
    errors.push('openaiApiKey is required when model is gpt-4o')
  }

  if ((config.model === 'claude' || config.model === 'claude-sonnet') && !config.anthropicApiKey) {
    errors.push('anthropicApiKey is required when model is claude/claude-sonnet')
  }

  if (!config.hotkey) {
    errors.push('hotkey is required')
  }

  if (!config.inputHotkey) {
    errors.push('inputHotkey is required')
  }

  if (config.overlayOpacity < 0.1 || config.overlayOpacity > 1.0) {
    errors.push('overlayOpacity must be between 0.1 and 1.0')
  }

  if (config.overlayWidth < 200 || config.overlayWidth > 800) {
    errors.push('overlayWidth must be between 200 and 800')
  }

  if (config.overlayHeight < 200 || config.overlayHeight > 2000) {
    errors.push('overlayHeight must be between 200 and 2000')
  }

  const validPositions = ['left', 'right', 'top', 'bottom']
  if (!validPositions.includes(config.overlayPosition)) {
    errors.push(
      `Unknown overlayPosition: ${config.overlayPosition}. Supported: ${validPositions.join(', ')}`
    )
  }

  const validProfiles = ['interview', 'sales', 'meeting', 'presentation', 'negotiation']
  if (!validProfiles.includes(config.profile)) {
    errors.push(
      `Unknown profile: ${config.profile}. Supported: ${validProfiles.join(', ')}`
    )
  }

  return errors
}
