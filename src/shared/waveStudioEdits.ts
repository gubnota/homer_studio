import { clipDuration, clipEnd, newId, voiceColor, voiceAudioKey, type VoiceRegion, type WaveClip, type WaveTimeline } from './waveStudio'
const copy = (t: WaveTimeline): WaveTimeline => structuredClone(t)
export function splitClip(c: WaveClip, at: number): WaveClip[] {
  if (at <= c.startMs || at >= clipEnd(c)) return [c]
  const sourceAt = c.sourceStartMs + (at - c.startMs) * c.speed
  return [{ ...c, sourceEndMs: sourceAt, fadeInMs: Math.min(c.fadeInMs, at - c.startMs), fadeOutMs: 0 }, { ...c, id: newId(), startMs: at, sourceStartMs: sourceAt, fadeInMs: 0, fadeOutMs: Math.min(c.fadeOutMs, clipEnd(c) - at) }]
}
export function splitAt(t: WaveTimeline, at: number): WaveTimeline { return { ...t, clips: t.clips.flatMap(c => splitClip(c, at)) } }
/** Split only the selected independent soundtrack. Narration stays intact. */
export function splitSoundtrack(t: WaveTimeline, id: string, at: number): WaveTimeline {
 return {...t,sfx:t.sfx.flatMap(c=>c.id===id?splitClip(c,at):[c])}
}
export function soundtrackRange(c: WaveClip, sourceIn: number, sourceOut: number, sourceDuration: number): WaveClip {
 const minimum = c.speed
 const sourceStartMs=Math.max(0,Math.min(sourceIn,sourceDuration-minimum))
 const sourceEndMs=Math.max(sourceStartMs+minimum,Math.min(sourceOut,sourceDuration))
 const duration=(sourceEndMs-sourceStartMs)/c.speed
 return {...c,sourceStartMs,sourceEndMs,fadeInMs:Math.min(c.fadeInMs,duration),fadeOutMs:Math.min(c.fadeOutMs,duration)}
}
export function trimSoundtrack(t: WaveTimeline,id:string,edge:'start'|'end',at:number,sourceDuration:number): WaveTimeline {
 if(!Number.isFinite(at))return t
 return {...t,sfx:t.sfx.map(c=>{
  if(c.id!==id)return c
  if(edge==='start'){
   const start=Math.max(0,c.startMs-c.sourceStartMs/c.speed,Math.min(at,clipEnd(c)-1))
   return {...soundtrackRange(c,c.sourceStartMs+(start-c.startMs)*c.speed,c.sourceEndMs,sourceDuration),startMs:start}
  }
  const end=Math.max(c.startMs+1,Math.min(at,86400000,c.startMs+(sourceDuration-c.sourceStartMs)/c.speed))
  return soundtrackRange(c,c.sourceStartMs,c.sourceStartMs+(end-c.startMs)*c.speed,sourceDuration)
 })}
}
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
  const containing=t.voices.find(v=>v.startMs<=a && v.endMs>=b && v.audio && v.audio.activeAudioKey===voiceAudioKey(t,v))
  if(containing && (containing.startMs!==a || containing.endMs!==b)) {
    const selected=sliceVoice(t,containing,a,b)
    const remaining=t.voices.flatMap(v=>v.id!==containing.id?[v]:[
      ...(v.startMs<a?[sliceVoice(t,v,v.startMs,a)]:[]),
      ...(v.endMs>b?[sliceVoice(t,v,b,v.endMs)]:[])])
    return assignVoice({...t,voices:[...remaining,selected]},a,b,voice)
  }
  const previous=t.voices.find(v=>v.startMs===a && v.endMs===b)
  if(previous?.voiceId===voice.id && (!previous.audio || previous.production))return {...t,voices:t.voices.map(v=>v.id===previous.id?{...v,name:voice.name,color:voiceColor(voice)}:v)}
  if(previous?.audio && previous.audio.activeAudioKey===voiceAudioKey(t,previous)) {
    const version=previous.audio.versions.find(v=>v.voiceId===voice.id)
    let next=replaceClips(t,a,b,version?.clips || previous.audio.original)
    const region={...previous,voiceId:voice.id,name:voice.name,color:voiceColor(voice),production:version?.production}
    region.audio={...previous.audio,activeAudioKey:voiceAudioKey(next,region)}
    if(region.production)region.production={...region.production,audioKey:region.audio.activeAudioKey}
    next={...next,voices:next.voices.map(v=>v.id===previous.id?region:v)}
    return next
  }
  let baseline=t
  for(const v of t.voices)if(v.audio && v.endMs>a && v.startMs<b && v.audio.activeAudioKey===voiceAudioKey(t,v)) {
    const start=Math.max(a,v.startMs),end=Math.min(b,v.endMs)
    const original={...t,clips:v.audio.original.map(c=>({...c,startMs:c.startMs+v.startMs}))}
    baseline=replaceClips(baseline,start,end,passageClips(original,{startMs:start,endMs:end}))
  }
  const remaining = t.voices.flatMap(v => v.endMs <= a || v.startMs >= b ? [v] : [ ...(v.startMs < a ? [sliceVoice(t,v,v.startMs,a)] : []), ...(v.endMs > b ? [sliceVoice(t,v,b,v.endMs)] : []) ])
  return { ...baseline, voices: [...remaining, { id: newId(), voiceId: voice.id, name: voice.name, color: voiceColor(voice), startMs: a, endMs: b }] }
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
export function joinCandidateIds(t: WaveTimeline, selection: [number,number] | null, playhead: number, selectedId: string | null, selectedIds: string[] = []): string[] {
 const clips = [...t.clips].sort((a,b)=>a.startMs-b.startMs)
 if (selectedIds.length > 1) return clips.filter(c=>selectedIds.includes(c.id)).map(c=>c.id)
 if (selection) return clips.filter(c=>c.startMs>=selection[0]-.01 && clipEnd(c)<=selection[1]+.01).map(c=>c.id)
 const selected = clips.findIndex(c=>c.id===selectedId)
 const i = selected>=0 ? selected : clips.findIndex(c=>c.startMs<=playhead && clipEnd(c)>playhead)
 return i<0 ? [] : clips.slice(i,i+2).map(c=>c.id)
}

export function packSfxRows(clips: WaveClip[]): WaveClip[][] {
 const rows: WaveClip[][] = []
 for (const clip of [...clips].sort((a,b)=>a.startMs-b.startMs)) { let row=rows.find(r=>clipEnd(r[r.length-1]!)<=clip.startMs); if(!row){row=[];rows.push(row)} row.push(clip) }
 return rows.length ? rows : [[]]
}

/** Snapshots use local passage time so they remain valid when the passage moves. */
export function passageClips(t:WaveTimeline,v:Pick<VoiceRegion,'startMs'|'endMs'>):WaveClip[] {
 return t.clips.flatMap(c=>splitClip(c,v.startMs).flatMap(c=>splitClip(c,v.endMs)))
  .filter(c=>c.startMs>=v.startMs && clipEnd(c)<=v.endMs+.01)
  .map(c=>({...c,startMs:c.startMs-v.startMs}))
}
function replaceClips(t:WaveTimeline,a:number,b:number,clips:WaveClip[]):WaveTimeline {
 const next=isolate(t,a,b)
 return {...next,clips:[...next.clips.filter(c=>c.startMs<a || c.startMs>=b),...clips.map(c=>({...c,id:newId(),startMs:a+c.startMs}))].sort((x,y)=>x.startMs-y.startMs)}
}
export function rememberVoiceVersions(before:WaveTimeline,after:WaveTimeline,completed:Set<string>):WaveTimeline {
 return {...after,voices:after.voices.map(v=>{
  if(!completed.has(v.id) || !v.production)return v
  const old=before.voices.find(r=>r.id===v.id)
  const previous=old?.audio && old.audio.activeAudioKey===voiceAudioKey(before,old)?old.audio:undefined
  const retained=previous?.versions || (old?.production?[{voiceId:old.voiceId,clips:passageClips(before,old),production:{...old.production}}]:[])
  const version={voiceId:v.voiceId,clips:passageClips(after,v),production:{...v.production}}
  return {...v,audio:{original:previous?.original || passageClips(before,v),
   versions:[...retained.filter(r=>r.voiceId!==v.voiceId),version],activeAudioKey:voiceAudioKey(after,v)}}
 })}
}
export function restoreVoiceOriginal(t:WaveTimeline,id:string):WaveTimeline {
 const v=t.voices.find(r=>r.id===id)
 if(!v?.audio || v.audio.activeAudioKey!==voiceAudioKey(t,v))return t
 const next=replaceClips(t,v.startMs,v.endMs,v.audio.original)
 return {...next,voices:next.voices.map(r=>r.id===id?{...r,production:undefined,audio:{...v.audio!,activeAudioKey:voiceAudioKey(next,r)}}:r)}
}

function sliceVoice(t:WaveTimeline,v:VoiceRegion,a:number,b:number):VoiceRegion {
 const region={...v,id:newId(),startMs:a,endMs:b}
 const slice=(clips:WaveClip[])=>passageClips({...t,clips}, {startMs:a-v.startMs,endMs:b-v.startMs})
 if(v.audio && v.audio.activeAudioKey===voiceAudioKey(t,v))region.audio={
  original:slice(v.audio.original),versions:v.audio.versions.map(version=>({...version,clips:slice(version.clips)})),activeAudioKey:voiceAudioKey(t,region)}
 else region.audio=undefined
 if(region.production)region.production={...region.production,audioKey:voiceAudioKey(t,region)}
 return region
}

/** Recover originals saved by older releases before per-passage versions existed. */
export function recoverVoiceVersions(t:WaveTimeline,original:WaveTimeline|null|undefined):WaveTimeline {
 if(!original)return t
 return {...t,voices:t.voices.map(v=>{
  if(v.audio || !original.voices.some(r=>r.startMs===v.startMs && r.endMs===v.endMs))return v
  return {...v,audio:{original:passageClips(original,v),versions:v.production?[{voiceId:v.voiceId,clips:passageClips(t,v),production:{...v.production}}]:[],activeAudioKey:voiceAudioKey(t,v)}}
 })}
}
