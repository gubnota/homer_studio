import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { JobRecord, ProjectSnapshot, Settings, TextCandidate, ToolDiagnostic, Voice } from '../../shared/contracts'

export function isDesktop(): boolean {
  return '__TAURI_INTERNALS__' in window
}

export async function chooseFolder(title: string): Promise<string | null> {
  if (!isDesktop()) return null
  const selected = await open({ directory: true, multiple: false, title })
  return typeof selected === 'string' ? selected : null
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
  return invoke('read_manuscript', { path: selected })
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
  generateAudio: (project: ProjectSnapshot, chapterId: string) =>
    invoke<string>('generate_chapter_audio', { rootPath: project.rootPath, expectedRevision: project.revision, chapterId }),
  importAudio: (project: ProjectSnapshot, chapterId: string, sourcePath: string) =>
    invoke<string>('import_chapter_audio', { rootPath: project.rootPath, expectedRevision: project.revision, chapterId, sourcePath }),
  review: (project: ProjectSnapshot, chapterId: string, status: 'approved' | 'changes_requested') =>
    invoke<ProjectSnapshot>('set_chapter_review', { rootPath: project.rootPath, expectedRevision: project.revision, chapterId, status }),
  audioUrl: (project: ProjectSnapshot, chapterId: string) =>
    invoke<string>('audio_url', { rootPath: project.rootPath, chapterId }),
  waveform: (project: ProjectSnapshot, chapterId: string) =>
    invoke<number[]>('audio_waveform', { rootPath: project.rootPath, chapterId })
}

export const systemApi = {
  settings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  diagnostics: () => invoke<ToolDiagnostic[]>('tool_diagnostics'),
  jobs: () => invoke<JobRecord[]>('list_jobs'),
  controlJob: (jobId: string, action: 'pause' | 'resume' | 'cancel') => invoke<void>('control_job', { jobId, action })
}

export function errorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return typeof error === 'string' ? error : 'The operation could not be completed.'
}
