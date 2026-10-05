import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { VoiceMemosPage } from '../src/renderer/src/VoiceMemosPage'
import { AudioEditor } from '../src/renderer/src/components/AudioEditor'
import { compositionDuration, selectedTake, type RecordingSession, type AudioVariant } from '../src/shared/audio'
import { isRouteId } from '../src/shared/navigation'

const source: AudioVariant = {
  id: 'original', parentId: null, name: 'Original recording', path: 'assets/original.wav',
  durationMs: 10000, sampleRate: 48000, originalSampleRate: 44100, channels: 1,
  createdAtMs: 1, state: 'source', processing: null,
  composition: { clips: [{ assetId: 'original', startMs: 0, endMs: 10000, gain: 1, fadeInMs: 0, fadeOutMs: 0, crossfadeMs: 0 }] },
}
const memo: RecordingSession = {
  schemaVersion: 1, id: 'memo', name: 'Chapter practice', notes: '', favorite: false,
  deleted: false, revision: 1, createdAtMs: 1, updatedAtMs: 1,
  selectedTakeId: source.id, takes: [source, { ...source, id: 'preview', name: 'Retake preview', state: 'preview', parentId: source.id }], undo: [], redo: [],
}

describe('shared audio studio', () => {
  it('opens the memo workspace without a book project', () => {
    expect(isRouteId('voice-memos')).toBe(true)
    const html = renderToStaticMarkup(createElement(VoiceMemosPage))
    expect(html).toContain('Voice Memos')
    expect(html).toContain('New memo')
    expect(html).toContain('Search')
  })
  it('keeps the original selected while offering explicit preview acceptance', () => {
    expect(selectedTake(memo)?.id).toBe('original')
    const html = renderToStaticMarkup(createElement(AudioEditor, { memo, onChange: () => {}, onQueued: () => {} }))
    expect(html).toContain('Original recording')
    expect(html).toContain('Accept')
    expect(html).toContain('Reject')
    expect(html).toContain('Start (s)')
    expect(html).toContain('Save selection as new memo')
    expect(html).toContain('studio-select')
  })
  it('accounts for measured replacement length, silence and overlap', () => {
    const clip = source.composition.clips[0]!
    expect(compositionDuration({ clips: [
      { ...clip, endMs: 4020 },
      { ...clip, assetId: 'replacement', endMs: 3040, crossfadeMs: 40 },
      { ...clip, startMs: 5980, endMs: 10000, crossfadeMs: 40 },
      { ...clip, assetId: null, endMs: 500 },
    ] })).toBe(11500)
  })
})
