import { useEffect, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Chapter, ProjectSnapshot } from '../../shared/contracts'
import type { RouteId } from '../../shared/navigation'
import { chooseFolder, chooseManuscript, errorMessage, projectApi } from './native'

interface DesktopInfo { platform: string; architecture: string; runtime: string }

function Header({ eyebrow, title, copy, action }: { eyebrow: string; title: string; copy: string; action?: ReactNode }): JSX.Element {
  return <header className="page-header"><div><p>{eyebrow}</p><h1>{title}</h1><span>{copy}</span></div>{action}</header>
}

interface ProjectsProps {
  project: ProjectSnapshot | null
  busy: boolean
  onImport: () => void
  onOpen: () => void
  onEdit: () => void
  onProjectChange: (project: ProjectSnapshot) => void
}

export function ProjectsPage({ project, busy, onImport, onOpen, onEdit, onProjectChange }: ProjectsProps): JSX.Element {
  if (!project) {
    return <div className="page"><Header eyebrow="Library" title="Your audiobooks" copy="Local projects stay on this Mac." action={<div className="actions"><button onClick={onOpen} disabled={busy}>Open project</button><button className="primary" onClick={onImport}>New project</button></div>} />
      <section className="empty-card"><div className="empty-icon">Aa</div><h2>Start your first audiobook</h2><p>Import a TXT or Markdown manuscript, review its chapters, then create audio with an installed macOS voice.</p><button className="primary" onClick={onImport}>Import manuscript</button></section>
    </div>
  }
  return <div className="page"><Header eyebrow="Active project" title={project.title} copy={`${project.chapters.length} chapters · Revision ${project.revision}`} action={<div className="actions"><button onClick={onOpen}>Open another</button><button className="primary" onClick={onEdit}>Edit chapters</button></div>} />
    <section className="chapter-table"><div className="table-head"><span>Chapter</span><span>Segments</span><span>Status</span><span>Order</span></div>{project.chapters.map((chapter, index) => <div className="chapter-row" key={chapter.id}><div><strong>{chapter.title}</strong><small>{chapter.sourceText.slice(0, 100) || 'Empty chapter'}</small></div><span>{chapter.segments.length}</span><span className={chapter.audioPath && !chapter.audioStale ? 'ready' : 'pending'}>{chapter.audioPath ? (chapter.audioStale ? 'Audio stale' : 'Audio ready') : 'Needs audio'}</span><div className="order-buttons"><button disabled={index === 0} onClick={() => void moveChapter(project, index, -1, onProjectChange)}>↑</button><button disabled={index === project.chapters.length - 1} onClick={() => void moveChapter(project, index, 1, onProjectChange)}>↓</button></div></div>)}</section>
    <p className="path-note">Stored at {project.rootPath}</p>
  </div>
}

async function moveChapter(project: ProjectSnapshot, index: number, direction: -1 | 1, onChange: (project: ProjectSnapshot) => void): Promise<void> {
  const ids = project.chapters.map((chapter) => chapter.id)
  const target = index + direction
  ;[ids[index], ids[target]] = [ids[target]!, ids[index]!]
  onChange(await projectApi.reorder(project, ids))
}

export function ImportPage({ onCreated }: { onCreated: (project: ProjectSnapshot) => void }): JSX.Element {
  const [title, setTitle] = useState('')
  const [manuscript, setManuscript] = useState('')
  const [sourceName, setSourceName] = useState('Pasted text')
  const [parentPath, setParentPath] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  async function pickManuscript(): Promise<void> {
    try {
      const selected = await chooseManuscript()
      if (!selected) return
      setSourceName(selected.name)
      setManuscript(selected.text)
      if (!title) setTitle(selected.name.replace(/\.(txt|md|markdown)$/i, ''))
    } catch (cause) { setError(errorMessage(cause)) }
  }

  async function pickLocation(): Promise<void> {
    const selected = await chooseFolder('Choose where to create the project')
    if (selected) setParentPath(selected)
  }

  async function create(): Promise<void> {
    setBusy(true)
    setError('')
    try { onCreated(await projectApi.create(parentPath, title, manuscript)) }
    catch (cause) { setError(errorMessage(cause)) }
    finally { setBusy(false) }
  }

  return <div className="page"><Header eyebrow="New project" title="Import a manuscript" copy="TXT and Markdown are supported." action={<button onClick={() => void pickManuscript()}>Choose file</button>} />
    {error && <div className="inline-error" role="alert">{error}</div>}
    <div className="split"><section className="panel"><label>Project title<input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="My audiobook" /></label><label>Manuscript <span className="field-note">{sourceName}</span><textarea value={manuscript} onChange={(event) => { setManuscript(event.target.value); setSourceName('Pasted text') }} placeholder="# Chapter One&#10;&#10;Paste your manuscript here…" /></label><label>Project location<div className="path-picker"><input value={parentPath} readOnly placeholder="Choose a parent folder" /><button onClick={() => void pickLocation()}>Choose</button></div></label><button className="primary" disabled={busy || !title.trim() || !manuscript.trim() || !parentPath} onClick={() => void create()}>{busy ? 'Creating…' : 'Create project'}</button></section><aside className="panel tip"><h3>Chapter detection</h3><p>Markdown headings and lines beginning with “Chapter” or “Part” become separate chapters. Text before the first heading becomes an introduction.</p><p className="status">Source text remains readable inside the project folder.</p></aside></div>
  </div>
}

export function EditorPage({ project, onProjectChange }: { project: ProjectSnapshot | null; onProjectChange: (project: ProjectSnapshot) => void }): JSX.Element {
  const [selectedId, setSelectedId] = useState(project?.chapters[0]?.id ?? '')
  const chapter = project?.chapters.find((item) => item.id === selectedId) ?? project?.chapters[0]
  if (!project || !chapter) return <StudioPage route="editor" project={project} />
  return <div className="page editor-page"><Header eyebrow={project.title} title="Chapter editor" copy="Edit source text and keep changes on disk." />
    <div className="editor-grid"><aside className="chapter-nav">{project.chapters.map((item) => <button className={item.id === chapter.id ? 'active' : ''} key={item.id} onClick={() => setSelectedId(item.id)}><small>{String(item.order + 1).padStart(2, '0')}</small><span>{item.title}</span></button>)}</aside><ChapterEditor key={`${chapter.id}-${project.revision}`} project={project} chapter={chapter} onProjectChange={onProjectChange} /></div>
  </div>
}

function ChapterEditor({ project, chapter, onProjectChange }: { project: ProjectSnapshot; chapter: Chapter; onProjectChange: (project: ProjectSnapshot) => void }): JSX.Element {
  const [title, setTitle] = useState(chapter.title)
  const [text, setText] = useState(chapter.sourceText)
  const [state, setState] = useState<'idle' | 'saving' | 'saved'>('idle')
  const [error, setError] = useState('')
  async function save(): Promise<void> {
    setState('saving'); setError('')
    try { onProjectChange(await projectApi.updateChapter(project, chapter.id, title, text)); setState('saved') }
    catch (cause) { setState('idle'); setError(errorMessage(cause)) }
  }
  return <section className="panel chapter-editor">{error && <div className="inline-error">{error}</div>}<label>Chapter title<input value={title} onChange={(event) => { setTitle(event.target.value); setState('idle') }} /></label><label>Source text<textarea value={text} onChange={(event) => { setText(event.target.value); setState('idle') }} /></label><div className="editor-footer"><span>{text.length.toLocaleString()} characters · {chapter.segments.length} segments</span><button className="primary" disabled={state === 'saving'} onClick={() => void save()}>{state === 'saving' ? 'Saving…' : state === 'saved' ? 'Saved' : 'Save chapter'}</button></div></section>
}

const labels = {
  review: ['Review', 'Listen to selected takes and flag corrections.'],
  voices: ['Voices', 'Choose from voices installed on this Mac.']
} as const

export function StudioPage({ route, project }: { route: 'editor' | keyof typeof labels; project: ProjectSnapshot | null }): JSX.Element {
  const [title, copy] = route === 'editor' ? ['Chapter editor', 'Edit and prepare each spoken segment.'] : labels[route]
  return <div className="page"><Header eyebrow="Production" title={title} copy={copy} /><section className="panel placeholder"><span>{project ? project.title : 'Ready for a project'}</span><h2>{project ? 'This workflow connects in the next stage' : 'Import a manuscript to begin'}</h2><p>{project ? 'Your project is saved and ready for local audio operations.' : 'Create or open a project from the Projects screen.'}</p></section></div>
}

export function QueuePage(): JSX.Element {
  return <div className="page"><Header eyebrow="Production" title="Render queue" copy="Track local jobs and cancel work safely." /><section className="panel table-empty">No jobs yet.</section></div>
}

export function ExportsPage(): JSX.Element {
  return <div className="page"><Header eyebrow="Output" title="Exports" copy="Combine approved chapters and create measured timestamps." /><section className="panel placeholder"><span>Final assembly</span><h2>No complete project</h2><p>Exports appear here after every chapter has a current audio take.</p></section></div>
}

export function SettingsPage(): JSX.Element {
  const [desktop, setDesktop] = useState<DesktopInfo>({ platform: 'macOS', architecture: 'arm64', runtime: 'Browser preview' })
  useEffect(() => { if ('__TAURI_INTERNALS__' in window) void invoke<DesktopInfo>('desktop_info').then(setDesktop) }, [])
  return <div className="page"><Header eyebrow="System" title="Settings" copy="Configure local tools and model providers." /><div className="settings-grid"><section className="panel"><h2>Desktop runtime</h2><dl><div><dt>Platform</dt><dd>{desktop.platform}</dd></div><div><dt>Architecture</dt><dd>{desktop.architecture}</dd></div><div><dt>Runtime</dt><dd>{desktop.runtime}</dd></div></dl></section><section className="panel"><h2>Local tools</h2><p>FFmpeg, llama.cpp, and Ollama diagnostics connect in a later stage.</p><span className="status">No network account required</span></section></div></div>
}
