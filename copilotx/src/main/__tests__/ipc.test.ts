import { describe, it, expect, vi } from 'vitest'
import * as path from 'path'

vi.mock('@electron-toolkit/utils', () => ({
  is: { dev: false }
}))

import { getSidecarPath } from '../ipc'
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

describe('getSidecarPath', () => {
  it('constructs dev path with sidecarName on Linux', () => {
    const result = getSidecarPath('svchost', true, '/resources', '/project/src/main', 'linux')
    expect(result).toBe(path.join('/project/src/main', '../../sidecar/target/release/svchost'))
  })

  it('constructs production path with sidecarName on Linux', () => {
    const result = getSidecarPath('svchost', false, '/app/resources', '', 'linux')
    expect(result).toBe(path.join('/app/resources', 'svchost'))
  })

  it('constructs dev path with sidecarName on Windows', () => {
    const result = getSidecarPath('svchost', true, 'C:\\resources', 'C:\\project\\src\\main', 'win32')
    expect(result).toBe(path.join('C:\\project\\src\\main', '../../sidecar/target/release/svchost.exe'))
  })

  it('constructs production path with sidecarName on Windows', () => {
    const result = getSidecarPath('svchost', false, 'C:\\app\\resources', '', 'win32')
    expect(result).toBe(path.join('C:\\app\\resources', 'svchost.exe'))
  })

  it('falls back to system-helper when sidecarName is empty', () => {
    const result = getSidecarPath('', false, '/resources', '', 'linux')
    expect(result).toBe(path.join('/resources', 'system-helper'))
  })
})
