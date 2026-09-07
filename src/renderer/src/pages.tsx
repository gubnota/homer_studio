import { useEffect, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Chapter, JobRecord, ProjectSnapshot, Settings, ToolDiagnostic, Voice } from '../../shared/contracts'
import type { RouteId } from '../../shared/navigation'
import { chooseAudio, chooseFolder, chooseManuscript, errorMessage, productionApi, projectApi, systemApi } from './native'

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
  const [instruction, setInstruction] = useState('Improve narration flow while preserving meaning and factual details.')
  const [candidate, setCandidate] = useState(chapter.processedText ?? '')
  const [processing, setProcessing] = useState(false)
  const [state, setState] = useState<'idle' | 'saving' | 'saved'>('idle')
  const [error, setError] = useState('')
  async function save(): Promise<void> {
    setState('saving'); setError('')
    try { onProjectChange(await projectApi.updateChapter(project, chapter.id, title, text)); setState('saved') }
    catch (cause) { setState('idle'); setError(errorMessage(cause)) }
  }
  async function process(): Promise<void> {
    setProcessing(true); setError('')
    try { setCandidate((await productionApi.processText(instruction, text)).text) }
    catch (cause) { setError(errorMessage(cause)) }
    finally { setProcessing(false) }
  }
  async function accept(): Promise<void> {
    setProcessing(true); setError('')
    try { onProjectChange(await productionApi.acceptProcessed(project, chapter.id, candidate)) }
    catch (cause) { setError(errorMessage(cause)) }
    finally { setProcessing(false) }
  }
  return <section className="panel chapter-editor">
    {error && <div className="inline-error">{error}</div>}
    <label>Chapter title<input value={title} onChange={(event) => { setTitle(event.target.value); setState('idle') }} /></label>
    <label>Source text<textarea value={text} onChange={(event) => { setText(event.target.value); setState('idle') }} /></label>
    <div className="editor-footer"><span>{text.length.toLocaleString()} characters · {chapter.segments.length} segments</span><button className="primary" disabled={state === 'saving'} onClick={() => void save()}>{state === 'saving' ? 'Saving…' : state === 'saved' ? 'Saved' : 'Save chapter'}</button></div>
    <div className="processing-panel">
      <div><h2>Local text processing</h2><span>{chapter.processedText ? 'An accepted version is saved.' : 'Create a candidate without changing the source.'}</span></div>
      <label>Instruction<input value={instruction} onChange={(event) => setInstruction(event.target.value)} /></label>
      <button disabled={processing || !instruction.trim() || !text.trim()} onClick={() => void process()}>{processing ? 'Working locally…' : 'Create candidate'}</button>
      {candidate && <div className="candidate"><label>Candidate<textarea value={candidate} onChange={(event) => setCandidate(event.target.value)} /></label><div className="candidate-actions"><button onClick={() => setCandidate('')}>Discard</button><button className="primary" disabled={processing || !candidate.trim()} onClick={() => void accept()}>Accept candidate</button></div></div>}
    </div>
  </section>
}

const labels = {} as const

export function StudioPage({ route, project }: { route: 'editor'; project: ProjectSnapshot | null }): JSX.Element {
  const [title, copy] = ['Chapter editor', 'Edit and prepare each spoken segment.']
  return <div className="page"><Header eyebrow="Production" title={title} copy={copy} /><section className="panel placeholder"><span>{project ? project.title : 'Ready for a project'}</span><h2>{project ? 'This workflow connects in the next stage' : 'Import a manuscript to begin'}</h2><p>{project ? 'Your project is saved and ready for local audio operations.' : 'Create or open a project from the Projects screen.'}</p></section></div>
}

export function ReviewPage({ project, onProjectChange }: { project: ProjectSnapshot | null; onProjectChange: (project: ProjectSnapshot) => void }): JSX.Element {
  const [busyId, setBusyId] = useState('')
  const [error, setError] = useState('')
  if (!project) return <div className="page"><Header eyebrow="Production" title="Review" copy="Listen to chapter audio and approve it." /><section className="panel placeholder"><h2>Open a project first</h2></section></div>
  const activeProject = project
  async function waitForJob(jobId: string): Promise<void> {
    for (;;) {
      await new Promise((resolve) => window.setTimeout(resolve, 400))
      const job = (await systemApi.jobs()).find((item) => item.id === jobId)
      if (!job || ['completed', 'failed', 'cancelled'].includes(job.status)) {
        if (job?.status === 'failed') throw new Error(job.message ?? 'Audio processing failed.')
        if (job?.status === 'cancelled') throw new Error('Audio processing was cancelled.')
        onProjectChange(await projectApi.open(activeProject.rootPath))
        return
      }
    }
  }
  async function generate(chapterId: string): Promise<void> {
    setBusyId(chapterId); setError('')
    try { await waitForJob(await productionApi.generateAudio(activeProject, chapterId)) } catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
  async function importAudio(chapterId: string): Promise<void> {
    const source = await chooseAudio(); if (!source) return
    setBusyId(chapterId); setError('')
    try { await waitForJob(await productionApi.importAudio(activeProject, chapterId, source)) } catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
  async function review(chapterId: string, status: 'approved' | 'changes_requested'): Promise<void> {
    setError('')
    try { onProjectChange(await productionApi.review(activeProject, chapterId, status)) } catch (cause) { setError(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow={project.title} title="Narrate and review" copy="Generate with the selected macOS voice or import existing chapter audio." />
    {error && <div className="inline-error">{error}</div>}
    <section className="review-list">{project.chapters.map((chapter) => <article className="panel review-card" key={chapter.id}><div className="review-copy"><small>Chapter {chapter.order + 1}</small><h2>{chapter.title}</h2><span>{chapter.audioPath ? `${formatDuration(chapter.audioDurationMs)} · ${chapter.audioOrigin}${chapter.audioStale ? ' · stale' : ''}` : `${chapter.segments.length} text segments`}</span></div><div className="review-controls">{chapter.audioPath && !chapter.audioStale && <ChapterAudio project={project} chapter={chapter} />}<div className="actions"><button disabled={Boolean(busyId)} onClick={() => void importAudio(chapter.id)}>Import audio</button><button className="primary" disabled={Boolean(busyId)} onClick={() => void generate(chapter.id)}>{busyId === chapter.id ? 'Processing…' : chapter.audioPath ? 'Regenerate' : 'Generate audio'}</button></div>{chapter.audioPath && !chapter.audioStale && <div className="review-actions"><button className={chapter.reviewStatus === 'changes_requested' ? 'selected' : ''} onClick={() => void review(chapter.id, 'changes_requested')}>Needs changes</button><button className={chapter.reviewStatus === 'approved' ? 'selected approved' : ''} onClick={() => void review(chapter.id, 'approved')}>Approve</button></div>}</div></article>)}</section>
  </div>
}

function ChapterAudio({ project, chapter }: { project: ProjectSnapshot; chapter: Chapter }): JSX.Element {
  const [url, setUrl] = useState('')
  const [waveform, setWaveform] = useState<number[]>([])
  useEffect(() => { void productionApi.audioUrl(project, chapter.id).then(setUrl).catch(() => setUrl('')) }, [project.rootPath, chapter.id, chapter.audioPath])
  useEffect(() => { void productionApi.waveform(project, chapter.id).then(setWaveform).catch(() => setWaveform([])) }, [project.rootPath, chapter.id, chapter.audioPath])
  return url ? <div className="chapter-player">{waveform.length > 0 && <div className="waveform" aria-hidden="true">{waveform.map((peak, index) => <i key={index} style={{ height: `${Math.max(8, peak * 100)}%` }} />)}</div>}<audio controls preload="metadata" src={url} /></div> : <span>Preparing player…</span>
}

function formatDuration(value: number | null): string {
  if (!value) return 'Duration unavailable'
  const seconds = Math.round(value / 1000)
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}

export function VoicesPage(): JSX.Element {
  const [voices, setVoices] = useState<Voice[]>([])
  const [settings, setSettings] = useState<Settings | null>(null)
  const [message, setMessage] = useState('Loading installed voices…')
  useEffect(() => { void Promise.all([productionApi.voices(), systemApi.settings()]).then(([found, current]) => { setVoices(found); setSettings(current); setMessage('') }).catch((cause) => setMessage(errorMessage(cause))) }, [])
  async function select(voiceId: string): Promise<void> {
    if (!settings) return
    const next = { ...settings, speech: { ...settings.speech, voiceId } }
    setSettings(next); setMessage('Saving…')
    try { setSettings(await systemApi.saveSettings(next)); setMessage('Voice saved') } catch (cause) { setMessage(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow="Production" title="Installed voices" copy="These voices are provided by macOS and work fully offline." />{message && <div className="status-banner">{message}</div>}<section className="voice-grid">{voices.map((voice) => <button className={settings?.speech.voiceId === voice.id ? 'voice-card selected' : 'voice-card'} key={`${voice.id}-${voice.language}`} onClick={() => void select(voice.id)}><strong>{voice.id}</strong><span>{voice.language}</span><small>{voice.sample || 'Installed macOS voice'}</small></button>)}</section></div>
}

export function QueuePage(): JSX.Element {
  const [jobs, setJobs] = useState<JobRecord[]>([])
  const [error, setError] = useState('')
  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return
    const refresh = (): void => { void systemApi.jobs().then(setJobs).catch((cause) => setError(errorMessage(cause))) }
    refresh()
    const timer = window.setInterval(refresh, 750)
    return () => window.clearInterval(timer)
  }, [])
  async function control(job: JobRecord, action: 'pause' | 'resume' | 'cancel'): Promise<void> {
    try { await systemApi.controlJob(job.id, action); setJobs(await systemApi.jobs()) } catch (cause) { setError(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow="Production" title="Render queue" copy="Heavy local work runs one job at a time." />{error && <div className="inline-error">{error}</div>}<section className="panel">{jobs.length === 0 ? <div className="table-empty">No jobs yet.</div> : <div className="job-list">{jobs.map((job) => <article className="job-row" key={job.id}><div><strong>{job.label}</strong><small>{job.kind} · {job.status}{job.message ? ` · ${job.message}` : ''}</small></div><progress value={job.progress} max="100" /><span>{job.progress}%</span>{['queued', 'running'].includes(job.status) && <div className="actions"><button onClick={() => void control(job, 'pause')}>Pause</button><button onClick={() => void control(job, 'resume')}>Resume</button><button onClick={() => void control(job, 'cancel')}>Cancel</button></div>}</article>)}</div>}</section></div>
}

export function ExportsPage({ project, onProjectChange }: { project: ProjectSnapshot | null; onProjectChange: (project: ProjectSnapshot) => void }): JSX.Element {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  if (!project) return <div className="page"><Header eyebrow="Output" title="Exports" copy="Combine approved chapters and create measured timestamps." /><section className="panel placeholder"><h2>Open a project first</h2></section></div>
  const activeProject = project
  const ready = activeProject.chapters.filter((chapter) => chapter.audioPath && !chapter.audioStale && chapter.reviewStatus === 'approved').length
  const youtubeNote = youtubeEligibility(activeProject)
  async function createExport(): Promise<void> {
    setBusy(true); setError('')
    try {
      const jobId = await productionApi.exportProject(activeProject)
      for (;;) {
        await new Promise((resolve) => window.setTimeout(resolve, 500))
        const job = (await systemApi.jobs()).find((item) => item.id === jobId)
        if (!job || ['completed', 'failed', 'cancelled'].includes(job.status)) {
          if (job?.status === 'failed') throw new Error(job.message ?? 'Export failed.')
          if (job?.status === 'cancelled') throw new Error('Export was cancelled.')
          onProjectChange(await projectApi.open(activeProject.rootPath)); break
        }
      }
    } catch (cause) { setError(errorMessage(cause)) } finally { setBusy(false) }
  }
  return <div className="page"><Header eyebrow={project.title} title="Exports" copy="Combine approved chapters and create measured timestamps." action={<button className="primary" disabled={busy || ready !== project.chapters.length} onClick={() => void createExport()}>{busy ? 'Exporting…' : 'Export audiobook'}</button>} />
    {error && <div className="inline-error">{error}</div>}
    {ready !== project.chapters.length && <div className="status-banner">{ready} of {project.chapters.length} chapters have current, approved audio.</div>}
    {ready === project.chapters.length && youtubeNote && <div className="status-banner">The audiobook can be exported. YouTube may not activate chapter marks because {youtubeNote}.</div>}
    <section className="panel export-history"><h2>Export history</h2>{project.exports.length === 0 ? <div className="table-empty">No exports yet.</div> : [...project.exports].reverse().map((item) => <ExportItem key={item.id} project={project} item={item} stale={item.sourceUpdatedAtMs !== project.updatedAtMs} />)}</section>
  </div>
}

function youtubeEligibility(project: ProjectSnapshot): string {
  if (project.chapters.length < 3) return 'it has fewer than three chapters'
  if (project.chapters.some((chapter) => (chapter.audioDurationMs ?? 0) < 10_000)) return 'one or more chapters are shorter than ten seconds'
  let elapsed = 0
  const starts = project.chapters.map((chapter) => { const value = Math.floor(elapsed / 1000); elapsed += chapter.audioDurationMs ?? 0; return value })
  if (new Set(starts).size !== starts.length) return 'two chapter marks have the same whole-second start'
  return ''
}

function ExportItem({ project, item, stale }: { project: ProjectSnapshot; item: ProjectSnapshot['exports'][number]; stale: boolean }): JSX.Element {
  const [url, setUrl] = useState('')
  const [timestamps, setTimestamps] = useState('')
  const [copied, setCopied] = useState(false)
  useEffect(() => { void Promise.all([productionApi.exportAudioUrl(project, item.id), productionApi.exportTimestamps(project, item.id)]).then(([audio, text]) => { setUrl(audio); setTimestamps(text) }).catch(() => {}) }, [project.rootPath, item.id])
  async function copy(): Promise<void> { await navigator.clipboard.writeText(timestamps); setCopied(true); window.setTimeout(() => setCopied(false), 1500) }
  return <article className="export-row"><div><strong>{new Date(item.createdAtMs).toLocaleString()}</strong><small>{formatDuration(item.durationMs)} · {stale ? 'Project changed since export' : 'Current project content'}</small></div>{url && <audio controls preload="metadata" src={url} />}<textarea readOnly value={timestamps} aria-label="YouTube chapter timestamps" /><button disabled={!timestamps} onClick={() => void copy()}>{copied ? 'Copied' : 'Copy timestamps'}</button></article>
}

export function SettingsPage(): JSX.Element {
  const [desktop, setDesktop] = useState<DesktopInfo>({ platform: 'macOS', architecture: 'arm64', runtime: 'Browser preview' })
  const [settings, setSettings] = useState<Settings | null>(null)
  const [tools, setTools] = useState<ToolDiagnostic[]>([])
  const [message, setMessage] = useState('')
  useEffect(() => { if ('__TAURI_INTERNALS__' in window) { void invoke<DesktopInfo>('desktop_info').then(setDesktop); void Promise.all([systemApi.settings(), systemApi.diagnostics()]).then(([value, found]) => { setSettings(value); setTools(found) }).catch((cause) => setMessage(errorMessage(cause))) } }, [])
  async function save(): Promise<void> {
    if (!settings) return
    setMessage('Saving…')
    try { setSettings(await systemApi.saveSettings(settings)); setTools(await systemApi.diagnostics()); setMessage('Settings saved') } catch (cause) { setMessage(errorMessage(cause)) }
  }
  return <div className="page">
    <Header eyebrow="System" title="Settings" copy="Configure local tools and model providers." action={<button className="primary" disabled={!settings} onClick={() => void save()}>Save settings</button>} />
    {message && <div className="status-banner">{message}</div>}
    <div className="settings-grid">
      <section className="panel">
        <h2>Desktop runtime</h2>
        <dl><div><dt>Platform</dt><dd>{desktop.platform}</dd></div><div><dt>Architecture</dt><dd>{desktop.architecture}</dd></div><div><dt>Runtime</dt><dd>{desktop.runtime}</dd></div></dl>
        <h2>Local tools</h2>
        <div className="tool-list">{tools.map((tool) => <div key={tool.name}><span className={tool.available ? 'dot available' : 'dot'} /><strong>{tool.name}</strong><small>{tool.path ?? 'Not found'}</small></div>)}</div>
      </section>
      {settings && <SettingsForm settings={settings} onChange={setSettings} />}
    </div>
  </div>
}

function SettingsForm({ settings, onChange }: { settings: Settings; onChange: (value: Settings) => void }): JSX.Element {
  const ollama = settings.llm.provider === 'ollama' ? settings.llm : null
  const llama = settings.llm.provider === 'llama_cpp' ? settings.llm : null
  return <section className="panel settings-form">
    <h2>Speech</h2>
    <label>Installed voice<input value={settings.speech.voiceId} onChange={(event) => onChange({ ...settings, speech: { ...settings.speech, voiceId: event.target.value } })} /></label>
    <label>Words per minute<input type="number" min="80" max="500" value={settings.speech.rate} onChange={(event) => onChange({ ...settings, speech: { ...settings.speech, rate: Number(event.target.value) } })} /></label>
    <h2>Text processing</h2>
    <label>Provider<select value={settings.llm.provider} onChange={(event) => onChange({ ...settings, llm: event.target.value === 'ollama' ? { provider: 'ollama', base_url: 'http://127.0.0.1:11434', model: '', context_size: 8192, max_tokens: 2048 } : event.target.value === 'llama_cpp' ? { provider: 'llama_cpp', executable_path: '', model_path: '', context_size: 8192, max_tokens: 2048, gpu_layers: 99 } : { provider: 'none' } })}><option value="none">Disabled</option><option value="llama_cpp">llama.cpp / GGUF</option><option value="ollama">Ollama</option></select></label>
    {ollama && <div><label>Loopback URL<input value={ollama.base_url} onChange={(event) => onChange({ ...settings, llm: { ...ollama, base_url: event.target.value } })} /></label><label>Model<input value={ollama.model} onChange={(event) => onChange({ ...settings, llm: { ...ollama, model: event.target.value } })} /></label></div>}
    {llama && <div><label>llama-cli path<input value={llama.executable_path} onChange={(event) => onChange({ ...settings, llm: { ...llama, executable_path: event.target.value } })} /></label><label>GGUF model path<input value={llama.model_path} onChange={(event) => onChange({ ...settings, llm: { ...llama, model_path: event.target.value } })} /></label></div>}
  </section>
}
