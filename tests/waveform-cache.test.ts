import { describe, expect, it, vi } from 'vitest'
import { WaveformCache } from '../src/renderer/src/waveformCache'

describe('waveform window reuse', () => {
  it('reuses completed windows and evicts least recently viewed windows', async () => {
    const cache = new WaveformCache<number[]>(2), request = vi.fn(async () => [1, 2])
    await cache.load('take:0:1000', request)
    await cache.load('take:0:1000', request)
    expect(request).toHaveBeenCalledTimes(1)
    cache.set('other:0:1000', [3]); cache.get('take:0:1000'); cache.set('new:0:1000', [4])
    expect(cache.get('other:0:1000')).toBeUndefined()
    expect(cache.get('take:0:1000')).toEqual([1, 2])
  })
  it('does not cache failures or reuse a different take/window', async () => {
    const cache = new WaveformCache<number[]>(), failed = vi.fn(async (): Promise<number[]> => { throw Error('cancelled') })
    await expect(cache.load('take:0:1000', failed)).rejects.toThrow('cancelled')
    const retry = vi.fn(async () => [5])
    await cache.load('take:0:1000', retry)
    expect(cache.get('take:1000:2000')).toBeUndefined()
    expect(cache.get('replacement:0:1000')).toBeUndefined()
    expect(retry).toHaveBeenCalledTimes(1)
  })
})
