import type { Settings, VoicePreset } from '../../shared/contracts'

const builtInDefaults: Record<string, Omit<VoicePreset, 'id'>> = {
  'built-in-samantha': { name: 'Warm narrator', voiceId: 'Samantha', rate: 180, builtIn: true },
  'built-in-daniel': { name: 'Measured storyteller', voiceId: 'Daniel', rate: 170, builtIn: true },
  'built-in-karen': { name: 'Clear narrator', voiceId: 'Karen', rate: 180, builtIn: true }
}

export function activateVoicePreset(settings: Settings, preset: VoicePreset): Settings {
  return { ...settings, speech: { ...settings.speech, voiceId: preset.voiceId, rate: preset.rate }, selectedVoicePresetId: preset.id }
}

export function upsertVoicePreset(settings: Settings, preset: VoicePreset): Settings {
  const index = settings.voicePresets.findIndex((item) => item.id === preset.id)
  const voicePresets = [...settings.voicePresets]
  if (index === -1) voicePresets.push(preset)
  else voicePresets[index] = preset
  return activateVoicePreset({ ...settings, voicePresets }, preset)
}

export function deleteVoicePreset(settings: Settings, presetId: string): Settings {
  const preset = settings.voicePresets.find((item) => item.id === presetId)
  if (preset?.builtIn) throw new Error('Built-in voice presets cannot be deleted.')
  return {
    ...settings,
    voicePresets: settings.voicePresets.filter((item) => item.id !== presetId),
    selectedVoicePresetId: settings.selectedVoicePresetId === presetId ? null : settings.selectedVoicePresetId
  }
}

export function resetBuiltInPreset(settings: Settings, presetId: string): Settings {
  const defaults = builtInDefaults[presetId]
  if (!defaults) return settings
  return upsertVoicePreset(settings, { id: presetId, ...defaults })
}
