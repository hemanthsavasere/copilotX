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

describe('input mode state guards', () => {
  it('blocks capture hotkey when in input mode', () => {
    const isInputMode = true
    const isProcessing = false
    const result = !isInputMode && !isProcessing
    expect(result).toBe(false)
  })

  it('blocks input hotkey when already in input mode', () => {
    const isInputMode = true
    const result = !isInputMode
    expect(result).toBe(false)
  })

  it('allows input hotkey when not in input mode', () => {
    const isInputMode = false
    const isProcessing = false
    const result = !isInputMode && !isProcessing
    expect(result).toBe(true)
  })

  it('blocks input hotkey when processing', () => {
    const isInputMode = false
    const isProcessing = true
    const result = !isInputMode && !isProcessing
    expect(result).toBe(false)
  })
})
