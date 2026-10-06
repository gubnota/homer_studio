export interface WaveSource { id: string; name: string; durationMs: number; channels: number }
export interface WaveClip { id: string; sourceId: string | null; name: string; startMs: number; sourceStartMs: number; sourceEndMs: number; speed: number; gainDb: number; fadeInMs: number; fadeOutMs: number }
export interface VoiceRegion { id: string; voiceId: string; name: string; color: string; startMs: number; endMs: number; production?: { status: 'generated' | 'converted'; voiceId: string; audioKey: string } }
export interface WaveTimeline { clips: WaveClip[]; sfx: WaveClip[]; voices: VoiceRegion[] }
export interface WaveVideo { id:string; name:string; durationMs:number; startMs:number }
export interface WaveProject { schemaVersion: 1; id: string; name: string; revision: number; updatedAtMs: number; sources: WaveSource[]; timeline: WaveTimeline; view?: WaveView; voiceOriginal?: WaveTimeline | null; videos?: WaveVideo[] }
export interface SfxAsset extends WaveSource { category: string; builtIn: boolean }
export interface WaveView { offsetMs: number; spanMs: number; playheadMs: number; selection: [number, number] | null; selectedId: string | null; selectedIds?: string[]; loop: boolean }
export const sfxCategories = ['Audience', 'Transitions', 'Atmosphere', 'UI', 'Intro', 'Outro', 'Custom'] as const
export const speedPresets = [.85, .9, .95, 1, 1.05, 1.1, 1.15, 1.2]
export const blankTimeline = (): WaveTimeline => ({ clips: [], sfx: [], voices: [] })
export const newId = () => crypto.randomUUID()
export const clipDuration = (c: WaveClip) => (c.sourceEndMs - c.sourceStartMs) / c.speed
export const clipEnd = (c: WaveClip) => c.startMs + clipDuration(c)
export const timelineDuration = (t: WaveTimeline) => Math.max(0, ...t.clips.map(clipEnd), ...t.sfx.map(clipEnd))
export function formatWaveTime(ms: number): string {
  const n = Math.max(0, Math.round(ms)), h = Math.floor(n / 3600000), m = Math.floor(n / 60000) % 60, s = Math.floor(n / 1000) % 60
  return `${h ? `${String(h).padStart(2, '0')}:` : ''}${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}.${String(n % 1000).padStart(3, '0')}`
}
export function sourceClip(source: WaveSource, startMs = 0, sfx = false): WaveClip {
  return { id: newId(), sourceId: source.id, name: source.name, startMs, sourceStartMs: 0, sourceEndMs: source.durationMs, speed: 1, gainDb: sfx ? -10 : 0, fadeInMs: 0, fadeOutMs: 0 }
}

export interface WaveProcessingResult { kind: string; projectId: string; revision: number; sources: WaveSource[]; replacements: { startMs: number; endMs: number; sourceId: string; regionId?: string }[]; warnings: string[] }

export function voiceColor(voice: {id: string; color?: string}): string {
 if (voice.color && !['#718392', '#6c7f91'].includes(voice.color.toLowerCase())) return voice.color
 const colors=['#8d79b8','#619f97','#b38b61','#6d96bf','#b87591','#9ca863']
 return colors[Array.from(voice.id).reduce((h,c)=>(h*31+c.charCodeAt(0))>>>0,0)%colors.length]!
}
/** Audio provenance ignores level/fades and IDs, but detects replacement, retiming and range edits. */
export function voiceAudioKey(t: WaveTimeline, v: VoiceRegion): string {
 const parts: (string | number | null)[][]=[]
 for (const [lane,clips] of [['main',t.clips],['sfx',t.sfx]] as const) {
  for(const c of [...clips].sort((a,b)=>a.startMs-b.startMs)) {
   const a=Math.max(v.startMs,c.startMs), b=Math.min(v.endMs,clipEnd(c)); if(b<=a)continue
   const n=(x:number)=>Math.round(x*1000)/1000
   const item=[lane,c.sourceId,n(a-v.startMs),n(b-v.startMs),n(c.sourceStartMs+(a-c.startMs)*c.speed),n(c.sourceStartMs+(b-c.startMs)*c.speed),c.speed]
   // Sound effects do not invalidate conversion of narration.
   if(lane==='sfx' && t.clips.some(c=>c.sourceId && c.startMs<v.endMs && clipEnd(c)>v.startMs))continue
   const last=parts.at(-1)
   if(last && last[0]===lane && last[1]===c.sourceId && last[3]===item[2] && last[5]===item[4] && last[6]===c.speed){last[3]=item[3]!;last[5]=item[5]!}else parts.push(item)
  }
 }
 return JSON.stringify(parts)
}
export function voiceComplete(t: WaveTimeline,v: VoiceRegion): boolean { return !!v.production && v.production.voiceId===v.voiceId && v.production.audioKey===voiceAudioKey(t,v) }
export function reconcileVoiceProduction(t: WaveTimeline): WaveTimeline { return {...t,voices:t.voices.map(v=>v.production&&!voiceComplete(t,v)?{...v,production:undefined}:v)} }

export const projectDuration = (p: WaveProject) => Math.max(timelineDuration(p.timeline),...(p.videos || []).map(v=>v.startMs+v.durationMs))
