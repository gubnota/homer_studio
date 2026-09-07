import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { JobRecord, ProjectSnapshot, Settings, ToolDiagnostic } from '../../shared/contracts'

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
