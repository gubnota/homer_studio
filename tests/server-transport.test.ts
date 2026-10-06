import { afterEach, describe, expect, it, vi } from 'vitest'
import { hasBackend, invoke, isDesktop, uploadFile } from '../src/renderer/src/platform'

afterEach(() => vi.unstubAllGlobals())
describe('browser backend transport', () => {
 it('uses the HTTP backend and restores binary preview data', async () => {
  vi.stubGlobal('window', {location:{protocol:'http:'}})
  const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify({binary:[82,73,70,70]})))
  vi.stubGlobal('fetch',fetch)
  expect(isDesktop()).toBe(false); expect(hasBackend()).toBe(true)
  const result=await invoke<ArrayBuffer>('wave_preview',{startMs:0,endMs:1000})
  expect([...new Uint8Array(result)]).toEqual([82,73,70,70])
  expect(fetch.mock.calls[0]?.[0]).toBe('/api/command/wave_preview')
  expect(JSON.parse(fetch.mock.calls[0]?.[1].body)).toEqual({startMs:0,endMs:1000})
 })
 it('uploads a file as a body without expanding it into JSON', async () => {
  const fetch=vi.fn().mockResolvedValue(new Response(JSON.stringify({path:'/data/uploads/audio.mp3'})))
  vi.stubGlobal('fetch',fetch)
  const audio=new Blob(['audio'])
  expect(await uploadFile(audio,'a b.mp3')).toBe('/data/uploads/audio.mp3')
  expect(fetch.mock.calls[0]?.[0]).toBe('/api/upload?name=a%20b.mp3')
  expect(fetch.mock.calls[0]?.[1].body).toBe(audio)
 })
 it('reports backend errors and downloads completed exports only once', async () => {
  vi.stubGlobal('window',{location:{protocol:'http:'},dispatchEvent:vi.fn()})
  const click=vi.fn();vi.stubGlobal('document',{createElement:()=>({click})})
  const fetch=vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({message:'Invalid project'}),{status:400}))
   .mockResolvedValueOnce(new Response(JSON.stringify('job-id')))
   .mockResolvedValueOnce(new Response(JSON.stringify([{id:'job-id',status:'completed'}])))
   .mockResolvedValueOnce(new Response(''))
   .mockResolvedValueOnce(new Response(JSON.stringify([{id:'job-id',status:'completed'}])))
  vi.stubGlobal('fetch',fetch)
  await expect(invoke('wave_get')).rejects.toThrow('Invalid project')
  await invoke('wave_export',{outputPath:'/data/downloads/mix.m4a'})
  await invoke('list_jobs');await vi.waitFor(()=>expect(click).toHaveBeenCalledTimes(1))
  await invoke('list_jobs');expect(click).toHaveBeenCalledTimes(1)
 })
})
