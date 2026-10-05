import { invoke } from '@tauri-apps/api/core'
import type { AudioEdit, CaptureDevice, CaptureState, PeakWindow, RecordingSession } from '../../shared/audio'
export const audioApi = {
  sourcePath: (memo: RecordingSession) => invoke<string>('memo_source_path', { memoId: memo.id, expectedRevision: memo.revision }),
  list: (includeDeleted = false) => invoke<RecordingSession[]>('list_memos', { includeDeleted }),
  create: (name: string, context: Partial<Pick<RecordingSession, 'contextType' | 'chapterId' | 'segmentId' | 'speakerId' | 'text'>> | null = null) => invoke<RecordingSession>('create_memo', { name, context }),
  get: (memoId: string) => invoke<RecordingSession>('get_memo', { memoId }),
  update: (memo: RecordingSession) => invoke<RecordingSession>('update_memo', { memoId: memo.id, expectedRevision: memo.revision, name: memo.name, notes: memo.notes, favorite: memo.favorite, deleted: memo.deleted }),
  choose: (memo: RecordingSession, takeId: string, action: string) => invoke<RecordingSession>('choose_memo_take', { memoId: memo.id, expectedRevision: memo.revision, takeId, action }),
  import: (memo: RecordingSession, sourcePath: string) => invoke<string>('import_memo_audio', { memoId: memo.id, expectedRevision: memo.revision, sourcePath, name: sourcePath.split('/').pop() || 'Imported audio' }),
  edit: (memo: RecordingSession, edit: AudioEdit) => invoke<string>('edit_memo_audio', { memoId: memo.id, expectedRevision: memo.revision, edit }),
  extract: (memo: RecordingSession, startMs: number, endMs: number) => invoke<string>('extract_memo_selection', {memoId:memo.id,expectedRevision:memo.revision,startMs,endMs}),
  updateTake: (memo: RecordingSession, takeId: string, name: string, notes: string, favorite: boolean, deleted = false) => invoke<RecordingSession>('update_memo_take',{memoId:memo.id,expectedRevision:memo.revision,takeId,name,notes,favorite,deleted}),
  replace: (memo: RecordingSession, replacementId: string, startMs: number, endMs: number, crossfadeMs = 20) => invoke<string>('replace_memo_range', {memoId: memo.id, expectedRevision: memo.revision, replacementId, startMs, endMs, crossfadeMs}),
  url: (memoId: string, takeId: string) => invoke<string>('memo_audio_url', { memoId, takeId }),
  waveform: (memoId: string, takeId: string, startMs: number, endMs: number, maxPeaks = 1000) => invoke<PeakWindow>('memo_waveform', { memoId, takeId, startMs, endMs, maxPeaks }),
  devices: () => invoke<CaptureDevice[]>('audio_capture_devices'),
  permission: (request = false) => invoke<string>('audio_capture_permission', { request }),
  start: (memoId: string, deviceId: string | null) => invoke<CaptureState>('audio_capture_start', { memoId, deviceId }),
  control: (action: string) => invoke<CaptureState>('audio_capture_control', { action }),
  export: (memo: RecordingSession, outputPath: string) => invoke<string>('export_memo', { memoId: memo.id, expectedRevision: memo.revision, outputPath })
}

import type { EngineConfig, EngineStatus, ProcessingRequest, VoiceProfile } from '../../shared/audio'
export const processorApi = {
 configs: () => invoke<EngineConfig[]>('audio_engine_configs'),
 saveConfig: (config: EngineConfig) => invoke<void>('save_audio_engine', {config}),
 status: (config: EngineConfig) => invoke<EngineStatus>('audio_engine_status', {config}),
 setup: (engine: string, pythonPath: string) => invoke<string>('setup_audio_engine', {engine, pythonPath}),
 profiles: () => invoke<VoiceProfile[]>('list_voice_profiles'),
 saveProfile: (profile: VoiceProfile) => invoke<VoiceProfile>('save_voice_profile', {profile}),
 process: (request: ProcessingRequest) => invoke<string>('process_memo_audio', {request}),
 publish: (memo: RecordingSession, target: string, extra: Record<string, unknown> = {}) => invoke<string>('publish_memo_audio', {memoId:memo.id,expectedRevision:memo.revision,target,rootPath:null,projectRevision:null,chapterId:null,segmentId:null,voiceId:null,...extra}),
 importLibrary: (memo: RecordingSession,target:string,extra:Record<string,unknown>) => invoke<string>('import_library_audio',{memoId:memo.id,expectedRevision:memo.revision,target,rootPath:null,chapterId:null,soundId:null,...extra})
}
