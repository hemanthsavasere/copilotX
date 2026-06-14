import { describe, it, expect } from 'vitest'
import { validateConfig } from '../config'
import type { AppConfig } from '../config'

describe('validateConfig', () => {
  const validConfig: AppConfig = {
    hotkey: 'CommandOrControl+Shift+Space',
    model: 'gpt-4o',
    openaiApiKey: 'sk-test',
    anthropicApiKey: '',
    profile: 'interview',
    overlayOpacity: 0.85,
    overlayWidth: 320,
    overlayHeight: 600,
    overlayPosition: 'right'
  }

  it('returns no errors for valid config with gpt-4o', () => {
    const errors = validateConfig(validConfig)
    expect(errors).toHaveLength(0)
  })

  it('returns no errors for valid config with claude', () => {
    const config = { ...validConfig, model: 'claude', anthropicApiKey: 'sk-ant-test', openaiApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toHaveLength(0)
  })

  it('returns no errors for valid config with claude-sonnet', () => {
    const config = { ...validConfig, model: 'claude-sonnet', anthropicApiKey: 'sk-ant-test', openaiApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toHaveLength(0)
  })

  it('returns error for unknown model', () => {
    const config = { ...validConfig, model: 'gpt-3' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('Unknown model'))
  })

  it('returns error when openaiApiKey missing for gpt-4o', () => {
    const config = { ...validConfig, openaiApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('openaiApiKey'))
  })

  it('returns error when anthropicApiKey missing for claude', () => {
    const config = { ...validConfig, model: 'claude', anthropicApiKey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('anthropicApiKey'))
  })

  it('returns error for empty hotkey', () => {
    const config = { ...validConfig, hotkey: '' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('hotkey'))
  })

  it('returns error for overlayOpacity out of range', () => {
    const config = { ...validConfig, overlayOpacity: 0.05 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayOpacity'))
  })

  it('returns error for overlayOpacity above 1.0', () => {
    const config = { ...validConfig, overlayOpacity: 1.5 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayOpacity'))
  })

  it('returns error for overlayWidth below 200', () => {
    const config = { ...validConfig, overlayWidth: 100 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayWidth'))
  })

  it('returns error for overlayWidth above 800', () => {
    const config = { ...validConfig, overlayWidth: 1000 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayWidth'))
  })

  it('returns error for overlayHeight below 200', () => {
    const config = { ...validConfig, overlayHeight: 100 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayHeight'))
  })

  it('returns error for overlayHeight above 2000', () => {
    const config = { ...validConfig, overlayHeight: 2500 }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayHeight'))
  })

  it('returns error for unknown overlayPosition', () => {
    const config = { ...validConfig, overlayPosition: 'center' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('overlayPosition'))
  })

  it('returns error for unknown profile', () => {
    const config = { ...validConfig, profile: 'gaming' }
    const errors = validateConfig(config)
    expect(errors).toContainEqual(expect.stringContaining('profile'))
  })
})
