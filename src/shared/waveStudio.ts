export interface WaveSource { id: string; name: string; durationMs: number; channels: number }
export interface WaveClip { id: string; sourceId: string | null; name: string; startMs: number; sourceStartMs: number; sourceEndMs: number; speed: number; gainDb: number; fadeInMs: number; fadeOutMs: number }
export interface VoiceRegion { id: string; voiceId: string; name: string; color: string; startMs: number; endMs: number }
export interface WaveTimeline { clips: WaveClip[]; sfx: WaveClip[]; voices: VoiceRegion[] }
export interface WaveProject { schemaVersion: 1; id: string; name: string; revision: number; updatedAtMs: number; sources: WaveSource[]; timeline: WaveTimeline }
export interface SfxAsset extends WaveSource { category: string; builtIn: boolean }
export interface WaveView { offsetMs: number; spanMs: number; playheadMs: number; selection: [number, number] | null; selectedId: string | null; loop: boolean }
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
