import { describe, expect, it } from 'vitest'
import type { Settings, VoicePreset } from '../src/shared/contracts'
import { activateVoicePreset, deleteVoicePreset, resetBuiltInPreset, upsertVoicePreset } from '../src/renderer/src/voice-presets'

const base: Settings = {
  schemaVersion: 1,
  llm: { provider: 'none' },
  speech: { provider: 'macos_say', voiceId: 'Samantha', rate: 180 },
  ffmpegPath: null,
  ffprobePath: null,
  ollamaPath: null,
  voicePresets: [],
  selectedVoicePresetId: null
}

const custom: VoicePreset = { id: 'custom-one', name: 'Quiet', voiceId: 'Daniel', rate: 150, builtIn: false }

describe('voice preset settings', () => {
  it('creates and activates a custom preset', () => {
    const next = upsertVoicePreset(base, custom)
    expect(next.voicePresets).toEqual([custom])
    expect(next.speech).toMatchObject({ voiceId: 'Daniel', rate: 150 })
    expect(next.selectedVoicePresetId).toBe('custom-one')
  })

  it('activates, edits, and deletes custom presets', () => {
    const created = upsertVoicePreset(base, custom)
    const edited = upsertVoicePreset(created, { ...custom, rate: 165 })
    expect(activateVoicePreset(edited, edited.voicePresets[0]!).speech.rate).toBe(165)
    expect(deleteVoicePreset(edited, custom.id).voicePresets).toEqual([])
  })

  it('protects and resets built-in presets', () => {
    const builtIn: VoicePreset = { id: 'built-in-samantha', name: 'Changed', voiceId: 'Samantha', rate: 300, builtIn: true }
    const settings = { ...base, voicePresets: [builtIn] }
    expect(() => deleteVoicePreset(settings, builtIn.id)).toThrow('cannot be deleted')
    expect(resetBuiltInPreset(settings, builtIn.id).voicePresets[0]).toMatchObject({ name: 'Warm narrator', rate: 180 })
  })
})
