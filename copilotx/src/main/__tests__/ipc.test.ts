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
})
