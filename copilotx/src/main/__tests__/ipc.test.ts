import { describe, it, expect } from 'vitest'
import type { SidecarMessage } from '../ipc'

describe('SidecarMessage type parsing', () => {
  it('parses a pong message', () => {
    const raw = '{"type":"pong"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('pong')
  })

  it('parses a token message', () => {
    const raw = '{"type":"token","content":"hello"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('token')
    expect(msg.content).toBe('hello')
  })

  it('parses a done message', () => {
    const raw = '{"type":"done"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('done')
  })

  it('parses an error message', () => {
    const raw = '{"type":"error","message":"fail"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('error')
    expect(msg.message).toBe('fail')
  })

  it('parses a key_event message', () => {
    const raw = '{"type":"key_event","key":"a","shift":false,"ctrl":false}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('key_event')
    expect(msg.key).toBe('a')
    expect(msg.shift).toBe(false)
    expect(msg.ctrl).toBe(false)
  })

  it('parses an input_mode_state message', () => {
    const raw = '{"type":"input_mode_state","state":"active"}'
    const msg: SidecarMessage = JSON.parse(raw)
    expect(msg.type).toBe('input_mode_state')
    expect(msg.state).toBe('active')
  })
})
