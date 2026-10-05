import { useEffect, useRef, useState } from 'react'
import { clipDuration, clipEnd, type WaveClip, type WaveView } from '../../../shared/waveStudio'
import { waveApi } from '../waveStudioNative'
interface Props { clips: WaveClip[]; view: WaveView; height: number; selectedId: string | null; onError: (message: string) => void }
const peakCache = new Map<string, [number, number][]>()
export function WaveformCanvas({ clips, view, height: preferredHeight, selectedId, onError }: Props): JSX.Element {
 const ref = useRef<HTMLCanvasElement>(null), [width, setWidth] = useState(800), [height, setHeight] = useState(preferredHeight), [version, refresh] = useState(0)
 const callbacks = useRef(onError); callbacks.current = onError
 useEffect(() => { const element = ref.current; if (!element) return; const observer = new ResizeObserver(() => { setWidth(element.clientWidth); setHeight(element.clientHeight) }); observer.observe(element); return () => observer.disconnect() }, [])
 useEffect(() => {
   let alive = true, cursor = 0
   const visible = clips.filter(c => c.sourceId && c.startMs < view.offsetMs + view.spanMs && clipEnd(c) > view.offsetMs).slice(0, 200)
   const work = async () => { while (alive && cursor < visible.length) {
     const c = visible[cursor++]!, a = Math.max(c.startMs, view.offsetMs), b = Math.min(clipEnd(c), view.offsetMs + view.spanMs)
     const start = Math.max(0, Math.floor(c.sourceStartMs + (a - c.startMs) * c.speed)), end = Math.ceil(Math.min(c.sourceEndMs, c.sourceStartMs + (b - c.startMs) * c.speed))
     if (end <= start) continue
     const count = Math.max(1, Math.min(2048, Math.ceil((b - a) / view.spanMs * width / 3))), key = `${c.sourceId}:${start}:${end}:${count}`
     if (peakCache.has(key)) continue
     try { const peaks = await waveApi.peaks(c.sourceId!, start, end, count); peakCache.set(key, peaks.peaks); while (peakCache.size > 120) peakCache.delete(peakCache.keys().next().value!); if (alive) refresh(n => n + 1) } catch { if (alive) callbacks.current('Some waveform peaks could not be read. Your audio remains available for playback.') }
   } }
   void Promise.all([work(), work(), work(), work()]); return () => { alive = false }
 }, [clips, view.offsetMs, view.spanMs, width])
 useEffect(() => {
   const canvas = ref.current; if (!canvas || width <= 0) return
   const ratio = Math.min(2, window.devicePixelRatio || 1); canvas.width = width * ratio; canvas.height = height * ratio
   const ctx = canvas.getContext('2d'); if (!ctx) return; ctx.scale(ratio, ratio); ctx.clearRect(0, 0, width, height)
   const x = (ms: number) => (ms - view.offsetMs) / view.spanMs * width
   ctx.strokeStyle = '#e2e3df'; ctx.beginPath(); ctx.moveTo(0, height / 2); ctx.lineTo(width, height / 2); ctx.stroke()
   for (const c of clips) {
     if (c.startMs >= view.offsetMs + view.spanMs || clipEnd(c) <= view.offsetMs) continue
     const a = Math.max(c.startMs, view.offsetMs), b = Math.min(clipEnd(c), view.offsetMs + view.spanMs), left = x(a), right = x(b)
     ctx.fillStyle = c.id === selectedId ? '#edf1f4' : c.sourceId ? '#f3f4f1' : '#fafaf8'; ctx.fillRect(left, 1, Math.max(1, right - left), height - 2)
     ctx.strokeStyle = c.id === selectedId ? '#758d9c' : '#d6d8d1'; ctx.strokeRect(x(c.startMs) + .5, .5, Math.max(1, x(clipEnd(c)) - x(c.startMs) - 1), height - 1)
     if (!c.sourceId) { ctx.fillStyle = '#9da097'; ctx.font = '11px -apple-system, sans-serif'; if (right - left > 48) ctx.fillText('Silence', left + 10, height / 2 - 8); continue }
     const start = Math.max(0, Math.floor(c.sourceStartMs + (a - c.startMs) * c.speed)), end = Math.ceil(Math.min(c.sourceEndMs, c.sourceStartMs + (b - c.startMs) * c.speed)), count = Math.max(1, Math.min(2048, Math.ceil((b - a) / view.spanMs * width / 3)))
     const peaks = peakCache.get(`${c.sourceId}:${start}:${end}:${count}`), gain = c.gainDb <= -96 ? 0 : 10 ** (c.gainDb / 20)
     ctx.fillStyle = '#535b5d'
     peaks?.forEach(([lo, hi], i) => { const px = left + i / peaks.length * (right - left), local = a - c.startMs + i / peaks.length * (b - a), envelope = Math.min(1, c.fadeInMs ? local / c.fadeInMs : 1, c.fadeOutMs ? (clipDuration(c) - local) / c.fadeOutMs : 1), low = Math.max(-1, lo * gain * envelope), high = Math.min(1, hi * gain * envelope); ctx.fillRect(px, height / 2 - high * (height / 2 - 28), Math.max(1, (right - left) / peaks.length - 1), Math.max(1, (high - low) * (height / 2 - 28))) })
     ctx.save(); ctx.beginPath(); ctx.rect(left + 5, 0, Math.max(0, right - left - 10), 24); ctx.clip(); ctx.fillStyle = '#737971'; ctx.font = '10px -apple-system, sans-serif'; ctx.fillText(`${c.name}${c.speed !== 1 ? ` · ${c.speed.toFixed(2)}×` : ''}`, left + 8, 16); ctx.restore()
     if (c.fadeInMs || c.fadeOutMs) { ctx.strokeStyle = '#9b7956'; ctx.beginPath(); ctx.moveTo(x(c.startMs), height - 5); ctx.lineTo(x(c.startMs + c.fadeInMs), 22); ctx.lineTo(x(clipEnd(c) - c.fadeOutMs), 22); ctx.lineTo(x(clipEnd(c)), height - 5); ctx.stroke() }
   }
 }, [clips, view.offsetMs, view.spanMs, width, height, selectedId, version])
 return <canvas ref={ref} className="wave-canvas" style={{ height: preferredHeight }} aria-label="Audio waveform" />
}
