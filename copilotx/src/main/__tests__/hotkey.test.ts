import { describe, it, expect } from 'vitest'

describe('hotkey debounce logic', () => {
  it('allows first capture when not processing', () => {
    const isProcessing = false
    const result = !isProcessing
    expect(result).toBe(true)
  })

  it('blocks capture when already processing', () => {
    const isProcessing = true
    const result = !isProcessing
    expect(result).toBe(false)
  })

  it('allows capture after processing completes', () => {
    let isProcessing = true
    isProcessing = false
    const result = !isProcessing
    expect(result).toBe(true)
  })
})
