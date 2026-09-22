import { describe, expect, it } from 'vitest'
import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { isRouteId } from '../src/shared/navigation'
import { isValidSoundRequest, SoundStudioPage } from '../src/renderer/src/SoundStudioPage'

describe('standalone sound studio', () => {
  it('is reachable and renders without an audiobook project', () => {
    Object.assign(globalThis, { window: {} })
    expect(isRouteId('sounds')).toBe(true)
    const html = renderToStaticMarkup(createElement(SoundStudioPage))
    expect(html).toContain('No book or manuscript is needed')
    expect(html).toContain('Clip library')
  })

  it('checks prompt, duration, and seed before generation', () => {
    const good = { prompt: 'Cotton fabric rustling', category: 'sound_effect' as const, durationSeconds: 5, seed: null }
    expect(isValidSoundRequest(good)).toBe(true)
    expect(isValidSoundRequest({ ...good, prompt: ' ' })).toBe(false)
    expect(isValidSoundRequest({ ...good, durationSeconds: 21 })).toBe(false)
    expect(isValidSoundRequest({ ...good, negativePrompt: 'music, voices' })).toBe(true)
    expect(isValidSoundRequest({ ...good, negativePrompt: 'x'.repeat(301) })).toBe(false)
    expect(isValidSoundRequest({ ...good, seed: -1 })).toBe(false)
    expect(isValidSoundRequest({ ...good, category: 'vocal_gesture', prompt: 'heavy breathing' })).toBe(false)
    expect(isValidSoundRequest({ ...good, category: 'vocal_gesture', prompt: '[sigh]' })).toBe(true)
  })
})
