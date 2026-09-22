import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type { JobRecord, ProjectSnapshot, Settings, SoundAsset, SoundRequest, TextCandidate, ToolDiagnostic, Voice, WorkerHealth } from '../../shared/contracts'

export function isDesktop(): boolean {
  return '__TAURI_INTERNALS__' in window
}

export async function chooseFolder(title: string): Promise<string | null> {
  if (!isDesktop()) return null
  const selected = await open({ directory: true, multiple: false, title })
  return typeof selected === 'string' ? selected : null
}

export async function defaultProjectParent(): Promise<string> {
  return invoke<string>('default_project_parent')
}

export async function chooseManuscript(): Promise<{ name: string; text: string } | null> {
  if (!isDesktop()) return null
  const selected = await open({
    directory: false,
    multiple: false,
    title: 'Choose a manuscript',
    filters: [{ name: 'Manuscript', extensions: ['txt', 'md', 'markdown'] }]
  })
  if (typeof selected !== 'string') return null
  return loadDroppedManuscript([selected])
}

export function validateDroppedManuscript(paths: string[]): string {
  if (paths.length !== 1) throw new Error(paths.length === 0 ? 'Drop one manuscript file.' : 'Drop only one manuscript at a time.')
  const path = paths[0]!
  if (!/\.(txt|md|markdown)$/i.test(path)) throw new Error('Choose a TXT, MD, or Markdown manuscript.')
  return path
}

export function loadDroppedManuscript(paths: string[]): Promise<{ name: string; text: string }> {
  return invoke('read_manuscript', { path: validateDroppedManuscript(paths) })
}

export async function chooseAudio(): Promise<string | null> {
  if (!isDesktop()) return null
  const selected = await open({
    directory: false,
    multiple: false,
    title: 'Choose chapter audio',
    filters: [{ name: 'Audio', extensions: ['m4a', 'mp3', 'wav', 'aiff', 'aif', 'flac', 'ogg'] }]
  })
  return typeof selected === 'string' ? selected : null
}

export async function chooseTool(title: string, extensions?: string[]): Promise<string | null> {
  if (!isDesktop()) return null
  const selected = await open({
    directory: false,
    multiple: false,
    title,
    ...(extensions ? { filters: [{ name: title, extensions }] } : {})
  })
  return typeof selected === 'string' ? selected : null
}

export const projectApi = {
  create: (parentPath: string, title: string, manuscript: string) =>
    invoke<ProjectSnapshot>('create_project', { parentPath, title, manuscript }),
  open: (rootPath: string) => invoke<ProjectSnapshot>('open_project', { rootPath }),
  updateChapter: (project: ProjectSnapshot, chapterId: string, title: string, sourceText: string) =>
    invoke<ProjectSnapshot>('update_chapter', {
      rootPath: project.rootPath,
      expectedRevision: project.revision,
      chapterId,
      title,
      sourceText
    }),
  reorder: (project: ProjectSnapshot, chapterIds: string[]) =>
    invoke<ProjectSnapshot>('reorder_chapters', {
      rootPath: project.rootPath,
      expectedRevision: project.revision,
      chapterIds
    })
}

export const productionApi = {
  processText: (instruction: string, text: string) =>
    invoke<TextCandidate>('process_text', { instruction, text }),
  acceptProcessed: (project: ProjectSnapshot, chapterId: string, text: string) =>
    invoke<ProjectSnapshot>('accept_processed_text', {
      rootPath: project.rootPath,
      expectedRevision: project.revision,
      chapterId,
      text
    }),
  voices: () => invoke<Voice[]>('list_voices'),
  createVoice: (name: string) => invoke<Voice>('create_voice', { name }),
  addVoiceSample: (voiceId: string, name: string, sourcePath: string) => invoke<Voice>('add_voice_sample', { voiceId, name, sourcePath }),
  addRecordedVoiceSample: (voiceId: string, name: string, bytes: number[]) => invoke<Voice>('add_recorded_voice_sample', { voiceId, name, bytes }),
  selectVoiceSample: (voiceId: string, sampleId: string) => invoke<Voice>('select_voice_sample', { voiceId, sampleId }),
  voiceSampleUrl: (voiceId: string, sampleId: string) => invoke<string>('voice_sample_url', { voiceId, sampleId }),
  deleteVoice: (voiceId: string) => invoke<void>('delete_voice', { voiceId }),
  previewVoice: (voiceId: string, rate: number) => invoke<string>('preview_voice', { voiceId, rate }),
  generateAudio: (project: ProjectSnapshot, chapterId: string) =>
    invoke<string>('generate_chapter_audio', { rootPath: project.rootPath, expectedRevision: project.revision, chapterId }),
  importAudio: (project: ProjectSnapshot, chapterId: string, sourcePath: string) =>
    invoke<string>('import_chapter_audio', { rootPath: project.rootPath, expectedRevision: project.revision, chapterId, sourcePath }),
  review: (project: ProjectSnapshot, chapterId: string, status: 'approved' | 'changes_requested') =>
    invoke<ProjectSnapshot>('set_chapter_review', { rootPath: project.rootPath, expectedRevision: project.revision, chapterId, status }),
  audioUrl: (project: ProjectSnapshot, chapterId: string) =>
    invoke<string>('audio_url', { rootPath: project.rootPath, chapterId }),
  waveform: (project: ProjectSnapshot, chapterId: string) =>
    invoke<number[]>('audio_waveform', { rootPath: project.rootPath, chapterId }),
  exportProject: (project: ProjectSnapshot) =>
    invoke<string>('export_project', { rootPath: project.rootPath, expectedRevision: project.revision }),
  exportAudioUrl: (project: ProjectSnapshot, exportId: string) =>
    invoke<string>('export_audio_url', { rootPath: project.rootPath, exportId }),
  exportTimestamps: (project: ProjectSnapshot, exportId: string) =>
    invoke<string>('read_export_timestamps', { rootPath: project.rootPath, exportId })
}

export const systemApi = {
  settings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  diagnostics: () => invoke<ToolDiagnostic[]>('tool_diagnostics'),
  jobs: () => invoke<JobRecord[]>('list_jobs'),
  controlJob: (jobId: string, action: 'pause' | 'resume' | 'cancel') => invoke<void>('control_job', { jobId, action })
}

export const soundsApi = {
  workers: () => invoke<WorkerHealth[]>('sound_workers'),
  list: () => invoke<SoundAsset[]>('list_sounds'),
  generate: (request: SoundRequest) => invoke<string>('generate_sound', { request }),
  audioUrl: (id: string) => invoke<string>('sound_audio_url', { id }),
  export: async (asset: SoundAsset, format: 'wav' | 'm4a') => {
    const destination = await save({ title: 'Export sound clip', defaultPath: `sound-${asset.id.slice(0, 8)}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] })
    if (destination) await invoke<void>('export_sound', { id: asset.id, destination })
    return destination
  }
}

export function errorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return typeof error === 'string' ? error : 'The operation could not be completed.'
}
