import { useEffect, useRef, useState } from 'react'
import { projectDuration, type WaveProject } from '../../shared/waveStudio'
import { errorMessage } from './native'
import { waveApi } from './waveStudioNative'
interface Run { token: number; nodes: AudioBufferSourceNode[]; origin: number; start: number; end: number }
export function useWaveStudioPlayback(project: WaveProject | null, onTime: (ms: number) => void, onError: (message: string) => void) {
 const [playing, setPlaying] = useState(false), [loading, setLoading] = useState(false)
 const context = useRef<AudioContext | null>(null), run = useRef<Run | null>(null), token = useRef(0), cache = useRef(new Map<string, AudioBuffer>())
 const callbacks = useRef({ onTime, onError }); callbacks.current = { onTime, onError }
 function stop(update = true): void { token.current++; void waveApi.cancelPreview().catch(() => {}); const r = run.current; if (r) { if (update && context.current) callbacks.current.onTime(Math.max(r.start, Math.min(r.end, r.start + (context.current.currentTime - r.origin) * 1000))); for (const n of r.nodes) { try { n.stop(); n.disconnect() } catch {} } }; run.current = null; setPlaying(false); setLoading(false) }
 async function play(startMs: number, endMs = project ? projectDuration(project) : 0, loop = false): Promise<void> {
   stop(false); if (!project || endMs <= startMs) return
   // AudioContext must be created/resumed within the user gesture.
   const audio = context.current ??= new AudioContext({ sampleRate: 48000 }); try { await audio.resume() } catch(cause) { callbacks.current.onError(errorMessage(cause)); return }
   document.querySelectorAll('audio').forEach(a => a.pause())
   const mine = ++token.current, snapshot = project; setLoading(true)
   const chunk = async (start: number, end: number) => {
     const key = `${JSON.stringify(snapshot.timeline)}:${snapshot.id}:${start}:${end}`
     const existing = cache.current.get(key); if (existing) return existing
     let timer:number|undefined
     const data = await Promise.race([waveApi.preview(snapshot, start, end), new Promise<never>((_,reject)=>{timer=window.setTimeout(()=>reject(new Error('Audio preparation timed out. Cancel and retry, or re-import this source.')),45000)})]).finally(()=>window.clearTimeout(timer))
     if (mine !== token.current) throw new Error('superseded')
     const buffer = await audio.decodeAudioData(data instanceof ArrayBuffer ? data : new Uint8Array(data as unknown as number[]).buffer)
     cache.current.set(key, buffer); while (cache.current.size > 4) cache.current.delete(cache.current.keys().next().value!)
     return buffer
   }
   try {
     const firstEnd = Math.min(endMs, startMs + 10000), first = await chunk(startMs, firstEnd)
     if (mine !== token.current) return
     const r: Run = { token: mine, nodes: [], origin: audio.currentTime + .1, start: startMs, end: endMs }; run.current = r
     const schedule = (buffer: AudioBuffer, start: number) => { const node = audio.createBufferSource(); node.buffer = buffer; node.connect(audio.destination); node.start(r.origin + (start - startMs) / 1000); r.nodes.push(node); node.onended = () => { node.disconnect(); r.nodes = r.nodes.filter(n => n !== node) } }
     schedule(first, startMs); setPlaying(true); setLoading(false)
     const tick = () => {
       if (mine !== token.current || run.current !== r) return
       const ms = Math.min(endMs, Math.max(startMs, startMs + (audio.currentTime - r.origin) * 1000)); callbacks.current.onTime(ms)
       if (ms >= endMs) { stop(false); if (loop) void play(startMs, endMs, true) } else window.setTimeout(tick, 40)
     }; tick()
     let next = firstEnd
     while (next < endMs && mine === token.current) {
       const end = Math.min(endMs, next + 10000), buffer = await chunk(next, end)
       if (mine !== token.current) return
       const scheduled = r.origin + (next - startMs) / 1000
       if (audio.currentTime > scheduled - .02) throw new Error('Preview could not keep up. Pause and play again to use the cached audio.')
       schedule(buffer, next); next = end
       // Keep only two future chunks scheduled. Wake regularly so seeking stays immediate.
       while (next - (r.start + (audio.currentTime - r.origin) * 1000) > 20000 && mine === token.current) await new Promise(resolve => window.setTimeout(resolve, 150))
     }

   } catch (cause) { if (mine === token.current) { stop(false); callbacks.current.onError(errorMessage(cause)) } }
 }
 useEffect(() => { stop(); cache.current.clear() }, [project?.id, project?.timeline])
 useEffect(() => { const preview = (event: Event) => { if (event.target instanceof HTMLAudioElement) stop() }; document.addEventListener('play', preview, true); return () => { document.removeEventListener('play', preview, true); stop(); void context.current?.close() } }, [])
 return { playing, loading, play, stop }
}
