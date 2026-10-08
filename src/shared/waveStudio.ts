export interface WaveSource { id: string; name: string; durationMs: number; channels: number }
export interface WaveClip { id: string; sourceId: string | null; name: string; startMs: number; sourceStartMs: number; sourceEndMs: number; speed: number; gainDb: number; fadeInMs: number; fadeOutMs: number }
export interface VoiceProduction { status: 'generated' | 'converted'; voiceId: string; audioKey: string }
export interface VoiceVersion { voiceId: string; clips: WaveClip[]; production: VoiceProduction }
export interface VoiceAudio { original: WaveClip[]; versions: VoiceVersion[]; activeAudioKey: string }
export interface VoiceRegion { id: string; voiceId: string; name: string; color: string; startMs: number; endMs: number; production?: VoiceProduction; audio?: VoiceAudio }
export interface WaveTimeline { clips: WaveClip[]; sfx: WaveClip[]; voices: VoiceRegion[] }
export interface WaveVideo { id:string; name:string; durationMs:number; startMs:number; assetId?:string; sourceStartMs?:number; sourceDurationMs?:number }
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
export function reconcileVoiceProduction(t: WaveTimeline): WaveTimeline {
 return {...t,voices:t.voices.map(v=>({...v,
  production: voiceComplete(t,v)?v.production:undefined,
  audio: v.audio?.activeAudioKey===voiceAudioKey(t,v)?v.audio:undefined
 }))}
}

export const projectDuration = (p: WaveProject) => Math.max(timelineDuration(p.timeline),...(p.videos || []).map(v=>v.startMs+v.durationMs))

/** Find a complete free interval, keeping existing references in place. */
export function placeVideo(videos: WaveVideo[], durationMs: number, requested: number, excludeId?: string): number {
 if (!Number.isFinite(requested) || !Number.isFinite(durationMs) || durationMs <= 0) throw new Error('Invalid video position or duration.')
 let start = Math.max(0, requested)
 for (const video of videos.filter(v => v.id !== excludeId).sort((a,b) => a.startMs-b.startMs)) {
  if (start + durationMs <= video.startMs) break
  if (start < video.startMs + video.durationMs) start = video.startMs + video.durationMs
 }
 if (start + durationMs > 86400000) throw new Error('Video would exceed the 24-hour timeline limit.')
 return start
}
export function videosOverlap(videos: WaveVideo[]): boolean {
 const sorted = [...videos].sort((a,b) => a.startMs-b.startMs)
 return sorted.some((v,i) => i > 0 && v.startMs < sorted[i-1]!.startMs + sorted[i-1]!.durationMs)
}
export function arrangeVideos(videos: WaveVideo[]): WaveVideo[] {
 const placed: WaveVideo[] = []
 for (const video of [...videos].sort((a,b) => a.startMs-b.startMs)) placed.push({...video,startMs:placeVideo(placed,video.durationMs,video.startMs)})
 return placed
}
export function videoFrame(videos: WaveVideo[], playheadMs: number): {video: WaveVideo; timeMs: number; held: boolean} | null {
 const video = [...videos].sort((a,b) => b.startMs-a.startMs).find(v => v.startMs <= playheadMs)
 if (!video) return null
 const held = playheadMs >= video.startMs + video.durationMs
 return {video, held, timeMs: (video.sourceStartMs || 0) + Math.max(0, Math.min(playheadMs-video.startMs, video.durationMs-1))}
}

/** Navigation uses video fragments when the narration lane is empty. */
export function navigateFragment(timeline: WaveTimeline, videos: WaveVideo[], view: WaveView, right: boolean, boundary: boolean): {id:string;at:number}|null {
 const fragments=(timeline.clips.length?timeline.clips.map(c=>({id:c.id,start:c.startMs,end:clipEnd(c)})):videos.map(v=>({id:v.id,start:v.startMs,end:v.startMs+v.durationMs}))).sort((a,b)=>a.start-b.start)
 if(!fragments.length)return null
 const selected=fragments.find(c=>c.id===view.selectedId),inside=fragments.find(c=>view.playheadMs>=c.start&&view.playheadMs<c.end)
 let target=selected||inside
 if(boundary){target=target||(right?fragments.find(c=>c.start>=view.playheadMs):[...fragments].reverse().find(c=>c.end<=view.playheadMs))||(right?fragments.at(-1):fragments[0]);return target?{id:target.id,at:right?target.end:target.start}:null}
 if(target){const index=fragments.indexOf(target);if(right&&index===fragments.length-1)return {id:target.id,at:target.end};target=fragments[Math.max(0,Math.min(fragments.length-1,index+(right?1:-1)))]}
 else target=(right?fragments.find(c=>c.start>=view.playheadMs):[...fragments].reverse().find(c=>c.start<view.playheadMs))||(right?fragments.at(-1):fragments[0])
 return target?{id:target.id,at:target.start}:null
}
/** Display held-frame coverage without changing the source video duration. */
export function videoHolds(videos: WaveVideo[], endMs: number): {video:WaveVideo;startMs:number;endMs:number}[] {
 const sorted=[...videos].sort((a,b)=>a.startMs-b.startMs)
 return sorted.flatMap((video,i)=>{const startMs=video.startMs+video.durationMs,end=sorted[i+1]?.startMs??endMs;return end>startMs?[{video,startMs,endMs:end}]:[]})
}
