import { invoke } from '@tauri-apps/api/core'
import type { PeakWindow } from '../../shared/audio'
import type { SfxAsset, WaveProject, WaveSource, WaveProcessingResult, WaveVideo } from '../../shared/waveStudio'
export const waveApi = {
 hasVideo: (path:string)=>invoke<boolean>('wave_has_video',{path}),
 importVideo: (path:string)=>invoke<WaveVideo>('wave_import_video',{path}),
 videoUrl:(id:string)=>invoke<string>('wave_video_url',{id}),
 deleted:()=>invoke<WaveProject[]>('wave_deleted'),
 delete:(id:string,restore=false)=>invoke<void>('wave_delete',{id,restore}),
 saveCopy:(project:WaveProject,path:string)=>invoke<void>('wave_save_copy',{project,path}),
 openCopy:(path:string)=>invoke<WaveProject>('wave_open_copy',{path}),
 normalize: (project: WaveProject, clipId: string) => invoke<number>('wave_normalize', { project, clipId }),
 join: (project: WaveProject, clipIds: string[]) => invoke<WaveSource>('wave_join', { project, clipIds }),
 generateSpeech: (text: string, voiceId: string, projectId: string, revision: number) => invoke<string>('wave_generate_speech', { text, voiceId, projectId, revision }),
 convertRegions: (project: WaveProject, regionIds: string[], regenerate = false) => invoke<string>('wave_convert_regions', { project, regionIds, regenerate }),
 processingResult: (jobId: string) => invoke<WaveProcessingResult>('wave_processing_result', { jobId }),
 list: () => invoke<WaveProject[]>('wave_list'),
 get: (id: string) => invoke<WaveProject>('wave_get', { id }),
 create: (name: string) => invoke<WaveProject>('wave_create', { name }),
 save: (project: WaveProject, expectedRevision: number) => invoke<WaveProject>('wave_save', { project, expectedRevision }),
 import: (kind: 'file' | 'memo' | 'sound', path: string | null = null, id: string | null = null) => invoke<WaveSource>('wave_import', { kind, path, id }),
 peaks: (sourceId: string, startMs: number, endMs: number, maxPeaks: number) => invoke<PeakWindow>('wave_peaks', { sourceId, startMs: Math.max(0, Math.floor(startMs)), endMs: Math.ceil(endMs), maxPeaks: Math.min(2048, Math.max(1, Math.floor(maxPeaks))) }),
 cancelPreview: () => invoke<void>('wave_cancel_preview'),
 preview: (project: WaveProject, startMs: number, endMs: number) => invoke<ArrayBuffer>('wave_preview', { project, startMs, endMs }),
 export: (project: WaveProject, outputPath: string) => invoke<string>('wave_export', { project, outputPath }),
 sfx: () => invoke<SfxAsset[]>('wave_sfx_list'),
 addSfx: (source: WaveSource, category: string) => invoke<SfxAsset[]>('wave_sfx_add', { source, category }),
 updateSfx: (id: string, name: string, category: string, remove = false) => invoke<SfxAsset[]>('wave_sfx_update', { id, name, category, delete: remove }),
 url: (sourceId: string) => invoke<string>('wave_source_url', { sourceId })
}
