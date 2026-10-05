import { clipDuration, clipEnd, newId, voiceColor, type WaveClip, type WaveTimeline } from './waveStudio'
const copy = (t: WaveTimeline): WaveTimeline => structuredClone(t)
export function splitClip(c: WaveClip, at: number): WaveClip[] {
  if (at <= c.startMs || at >= clipEnd(c)) return [c]
  const sourceAt = c.sourceStartMs + (at - c.startMs) * c.speed
  return [{ ...c, sourceEndMs: sourceAt, fadeInMs: Math.min(c.fadeInMs, at - c.startMs), fadeOutMs: 0 }, { ...c, id: newId(), startMs: at, sourceStartMs: sourceAt, fadeInMs: 0, fadeOutMs: Math.min(c.fadeOutMs, clipEnd(c) - at) }]
}
export function splitAt(t: WaveTimeline, at: number): WaveTimeline { return { ...t, clips: t.clips.flatMap(c => splitClip(c, at)) } }
function isolate(t: WaveTimeline, a: number, b: number): WaveTimeline { return splitAt(splitAt(copy(t), a), b) }
export function removeRange(t: WaveTimeline, a: number, b: number, silence = false): WaveTimeline {
  if (b <= a) return t
  const next = isolate(t, a, b), d = b - a
  next.clips = next.clips.filter(c => c.startMs < a || c.startMs >= b).map(c => !silence && c.startMs >= b ? { ...c, startMs: c.startMs - d } : c)
  if (silence) next.clips.push({ id: newId(), sourceId: null, name: 'Silence', startMs: a, sourceStartMs: 0, sourceEndMs: d, speed: 1, gainDb: 0, fadeInMs: 0, fadeOutMs: 0 })
  else {
    next.sfx = next.sfx.map(c => ({ ...c, startMs: mapDeleted(c.startMs, a, b) }))
    next.voices = next.voices.map(v => ({ ...v, startMs: mapDeleted(v.startMs, a, b), endMs: mapDeleted(v.endMs, a, b) })).filter(v => v.endMs > v.startMs)
  }
  return { ...next, clips: next.clips.sort((x, y) => x.startMs - y.startMs) }
}
function mapDeleted(n: number, a: number, b: number) { return n < a ? n : n < b ? a : n - (b - a) }
export function insertSilence(t: WaveTimeline, at: number, duration: number): WaveTimeline {
  const next = splitAt(copy(t), at)
  next.clips = next.clips.map(c => c.startMs >= at ? { ...c, startMs: c.startMs + duration } : c)
  next.sfx = next.sfx.map(c => c.startMs >= at ? { ...c, startMs: c.startMs + duration } : c)
  next.voices = next.voices.map(v => ({ ...v, startMs: v.startMs >= at ? v.startMs + duration : v.startMs, endMs: v.endMs > at ? v.endMs + duration : v.endMs }))
  next.clips.push({ id: newId(), sourceId: null, name: 'Silence', startMs: at, sourceStartMs: 0, sourceEndMs: duration, speed: 1, gainDb: 0, fadeInMs: 0, fadeOutMs: 0 })
  return { ...next, clips: next.clips.sort((x, y) => x.startMs - y.startMs) }
}
export function changeSpeed(t: WaveTimeline, a: number, b: number, speed: number): WaveTimeline {
  const next = isolate(t, a, b)
  const selected = next.clips.filter(c => c.startMs >= a && clipEnd(c) <= b + .01)
  const map = (time: number) => time + selected.reduce((d, c) => d + Math.max(0, Math.min(clipDuration(c), time - c.startMs)) * (c.speed / speed - 1), 0)
  next.clips = next.clips.map(c => ({ ...c, startMs: map(c.startMs), ...(selected.includes(c) ? { speed, fadeInMs: Math.min(c.fadeInMs, (c.sourceEndMs - c.sourceStartMs) / speed), fadeOutMs: Math.min(c.fadeOutMs, (c.sourceEndMs - c.sourceStartMs) / speed) } : {}) }))
  next.sfx = next.sfx.map(c => ({ ...c, startMs: map(c.startMs) }))
  next.voices = next.voices.map(v => ({ ...v, startMs: map(v.startMs), endMs: map(v.endMs) }))
  return next
}
export function assignVoice(t: WaveTimeline, a: number, b: number, voice: { id: string; name: string; color?: string }): WaveTimeline {
  const remaining = t.voices.flatMap(v => v.endMs <= a || v.startMs >= b ? [v] : [ ...(v.startMs < a ? [{ ...v, endMs: a }] : []), ...(v.endMs > b ? [{ ...v, id: newId(), startMs: b }] : []) ])
  return { ...t, voices: [...remaining, { id: newId(), voiceId: voice.id, name: voice.name, color: voiceColor(voice), startMs: a, endMs: b }] }
}
export function moveClip(t: WaveTimeline, id: string, startMs: number, swapId?: string): WaveTimeline {
  const next = copy(t), moving = next.clips.find(c => c.id === id)
  if (!moving) return t
  const target = next.clips.find(c => c.id === swapId && c.id !== id)
  if (target) {
    // Swap in the ordered sequence; variable durations preserve the intervening material.
    const sorted = next.clips.sort((a, b) => a.startMs - b.startMs), i = sorted.indexOf(moving), j = sorted.indexOf(target)
    const lo = Math.min(i, j), hi = Math.max(i, j), origin = sorted[lo]!.startMs
    ;[sorted[i], sorted[j]] = [sorted[j]!, sorted[i]!]
    let at = origin
    for (let k = lo; k <= hi; k++) { sorted[k] = { ...sorted[k]!, startMs: at }; at += clipDuration(sorted[k]!) }
    next.clips = sorted
  } else {
    const desired = Math.max(0, startMs), others = next.clips.filter(c => c.id !== id).sort((a, b) => a.startMs - b.startMs)
    const collision = others.find(c => desired >= c.startMs && desired < clipEnd(c))
    if (collision) {
      // An insertion drops before the highlighted clip, never over its audio.
      const at = collision.startMs
      next.clips = others.map(c => c.startMs >= at ? { ...c, startMs: c.startMs + clipDuration(moving) } : c)
      next.clips.push({ ...moving, startMs: at })
    } else {
      const end = desired + clipDuration(moving)
      const first = others.find(c => c.startMs >= desired && c.startMs < end)
      next.clips = others.map(c => first && c.startMs >= first.startMs ? { ...c, startMs: c.startMs + clipDuration(moving) } : c)
      next.clips.push({ ...moving, startMs: desired })
    }
  }
  // Map annotations with every affected fragment, including insertion shifts.
  next.voices = t.voices.flatMap(v => t.clips.flatMap(old => {
    const updated = next.clips.find(c => c.id === old.id)
    const a = Math.max(v.startMs, old.startMs), b = Math.min(v.endMs, clipEnd(old))
    return updated && b > a ? [{ ...v, id: newId(), startMs: updated.startMs + a - old.startMs, endMs: updated.startMs + b - old.startMs }] : []
  }))
  // Make uncovered narration gaps selectable and resizable.
  const ordered = next.clips.sort((a, b) => a.startMs - b.startMs), filled: WaveClip[] = []
  let cursor = 0
  for (const c of ordered) { if (c.startMs > cursor + .01) filled.push({ id: newId(), sourceId: null, name: 'Silence', startMs: cursor, sourceStartMs: 0, sourceEndMs: c.startMs - cursor, speed: 1, gainDb: 0, fadeInMs: 0, fadeOutMs: 0 }); filled.push(c); cursor = clipEnd(c) }
  return { ...next, clips: filled }
}
export interface TimelineHistory { past: WaveTimeline[]; present: WaveTimeline; future: WaveTimeline[] }
export function historyEdit(h: TimelineHistory, next: WaveTimeline): TimelineHistory { return JSON.stringify(next) === JSON.stringify(h.present) ? h : { past: [...h.past, h.present].slice(-100), present: next, future: [] } }
export function historyUndo(h: TimelineHistory): TimelineHistory { const last = h.past.at(-1); return last ? { past: h.past.slice(0, -1), present: last, future: [h.present, ...h.future].slice(0, 100) } : h }
export function historyRedo(h: TimelineHistory): TimelineHistory { const first = h.future[0]; return first ? { past: [...h.past, h.present].slice(-100), present: first, future: h.future.slice(1) } : h }

/** Insert without overwriting; annotations and effects follow later narration. */
export function insertMain(t: WaveTimeline, clip: WaveClip, at: number): WaveTimeline {
 const shifted = insertSilence(t, Math.max(0, at), clipDuration(clip))
 const placeholder = shifted.clips.find(c => c.sourceId === null && c.startMs === Math.max(0, at) && c.sourceEndMs === clipDuration(clip))
 return { ...shifted, clips: [...shifted.clips.filter(c => c !== placeholder), { ...clip, startMs: Math.max(0, at) }].sort((a,b)=>a.startMs-b.startMs) }
}
export function transferClip(t: WaveTimeline, id: string, lane: 'main' | 'sfx', at: number): WaveTimeline {
 const source = t.clips.find(c=>c.id===id) || t.sfx.find(c=>c.id===id)
 if (!source) return t
 const wasMain=t.clips.some(c=>c.id===id)
 const base = { ...t, clips: wasMain&&lane==='sfx' ? t.clips.map(c=>c.id===id?{...c,id:newId(),sourceId:null,name:'Silence',sourceStartMs:0,sourceEndMs:clipDuration(c),speed:1,gainDb:0,fadeInMs:0,fadeOutMs:0}:c) : t.clips.filter(c=>c.id!==id), sfx: t.sfx.filter(c=>c.id!==id) }
 return lane === 'main' ? insertMain(base, source, at) : { ...base, sfx: [...base.sfx,{...source,startMs:Math.max(0,at)}] }
}
export function replaceRange(t: WaveTimeline, a: number, b: number, clip: WaveClip): WaveTimeline {
 const next = isolate(t,a,b)
 return { ...next, clips: [...next.clips.filter(c=>c.startMs<a || c.startMs>=b), {...clip,startMs:a}].sort((x,y)=>x.startMs-y.startMs) }
}
export function joinCandidateIds(t: WaveTimeline, selection: [number,number] | null, playhead: number, selectedId: string | null): string[] {
 const clips = [...t.clips].sort((a,b)=>a.startMs-b.startMs)
 if (selection) return clips.filter(c=>c.startMs>=selection[0]-.01 && clipEnd(c)<=selection[1]+.01).map(c=>c.id)
 const selected = clips.findIndex(c=>c.id===selectedId)
 const i = selected>=0 ? selected : clips.findIndex(c=>c.startMs<=playhead && clipEnd(c)>playhead)
 return i<0 ? [] : clips.slice(i,i+2).map(c=>c.id)
}
