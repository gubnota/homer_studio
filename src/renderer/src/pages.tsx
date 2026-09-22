import { useEffect, useRef, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { Chapter, JobRecord, ModelStatus, ProjectSnapshot, Settings, ToolDiagnostic, Voice } from '../../shared/contracts'
import type { RouteId } from '../../shared/navigation'
import { chooseAudio, chooseFolder, chooseManuscript, chooseTool, defaultProjectParent, errorMessage, isDesktop, loadDroppedManuscript, productionApi, projectApi, systemApi } from './native'
import { VoicePicker } from './VoicePicker'

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
      <section className="empty-card"><div className="empty-icon">Aa</div><h2>Start your first audiobook</h2><p>Import a TXT or Markdown manuscript, review its chapters, then create audio with a local Chatterbox voice.</p><button className="primary" onClick={onImport}>Import manuscript</button></section>
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
  const [dragging, setDragging] = useState(false)

  useEffect(() => { if (isDesktop()) void defaultProjectParent().then(setParentPath).catch((cause) => setError(errorMessage(cause))) }, [])

  function applyManuscript(selected: { name: string; text: string }): void {
    setSourceName(selected.name)
    setManuscript(selected.text)
    setTitle((current) => current || selected.name.replace(/\.(txt|md|markdown)$/i, ''))
    setError('')
  }

  useEffect(() => {
    if (!isDesktop()) return
    let disposed = false
    let stop: (() => void) | undefined
    void getCurrentWebview().onDragDropEvent((event) => {
      if (disposed) return
      if (event.payload.type === 'enter' || event.payload.type === 'over') setDragging(true)
      if (event.payload.type === 'leave') setDragging(false)
      if (event.payload.type === 'drop') {
        setDragging(false)
        void loadDroppedManuscript(event.payload.paths).then(applyManuscript).catch((cause) => setError(errorMessage(cause)))
      }
    }).then((unlisten) => { if (disposed) unlisten(); else stop = unlisten }).catch((cause) => setError(errorMessage(cause)))
    return () => { disposed = true; stop?.() }
  }, [])

  async function pickManuscript(): Promise<void> {
    try {
      const selected = await chooseManuscript()
      if (!selected) return
      applyManuscript(selected)
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
    <div className="split"><section className="panel"><label>Project title<input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="My audiobook" /></label><label className={dragging ? 'manuscript-drop active' : 'manuscript-drop'}>Manuscript <span className="field-note">{dragging ? 'Drop the file to load it' : `${sourceName} · Drop a TXT or Markdown file here`}</span><textarea value={manuscript} onChange={(event) => { setManuscript(event.target.value); setSourceName('Pasted text') }} placeholder="# Chapter One&#10;&#10;Paste your manuscript here…" /></label><label>Project location<div className="path-picker"><input value={parentPath} readOnly placeholder="Loading Documents folder…" /><button onClick={() => void pickLocation()}>Choose</button></div></label><button className="primary" disabled={busy || !title.trim() || !manuscript.trim() || !parentPath} onClick={() => void create()}>{busy ? 'Creating…' : 'Create project'}</button>{(!parentPath || !title.trim() || !manuscript.trim()) && <span className="field-note">{!parentPath ? 'Choose a project location.' : !title.trim() ? 'Add a project title.' : 'Add or paste manuscript text.'}</span>}</section><aside className="panel tip"><h3>Chapter detection</h3><p>Markdown headings and lines beginning with “Chapter” or “Part” become separate chapters. Text before the first heading becomes an introduction.</p><p className="status">Source text remains readable inside the project folder.</p></aside></div>
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
  const [candidatePieces, setCandidatePieces] = useState<string[]>([])
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
    try { const result = await productionApi.processText(instruction, text); setCandidate(result.text); setCandidatePieces(result.pieces) }
    catch (cause) { setError(errorMessage(cause)) }
    finally { setProcessing(false) }
  }
  async function accept(): Promise<void> {
    setProcessing(true); setError('')
    try { onProjectChange(await productionApi.acceptProcessed(project, chapter.id, candidate)) }
    catch (cause) { setError(errorMessage(cause)) }
    finally { setProcessing(false) }
  }
  function updatePiece(index: number, value: string): void { const next = candidatePieces.map((item, itemIndex) => itemIndex === index ? value : item); setCandidatePieces(next); setCandidate(next.join('\n\n')) }
  function removePiece(index: number): void { const next = candidatePieces.filter((_, itemIndex) => itemIndex !== index); setCandidatePieces(next); setCandidate(next.join('\n\n')) }
  return <section className="panel chapter-editor">
    {error && <div className="inline-error">{error}</div>}
    <label>Chapter title<input value={title} onChange={(event) => { setTitle(event.target.value); setState('idle') }} /></label>
    <label>Source text<textarea value={text} onChange={(event) => { setText(event.target.value); setState('idle') }} /></label>
    <div className="editor-footer"><span>{text.length.toLocaleString()} characters · {chapter.segments.length} segments</span><button className="primary" disabled={state === 'saving'} onClick={() => void save()}>{state === 'saving' ? 'Saving…' : state === 'saved' ? 'Saved' : 'Save chapter'}</button></div>
    <div className="processing-panel">
      <div><h2>Local text processing</h2><span>{chapter.processedText ? 'An accepted version is saved.' : 'Create a candidate without changing the source.'}</span></div>
      <label>Instruction<input value={instruction} onChange={(event) => setInstruction(event.target.value)} /></label>
      <button disabled={processing || !instruction.trim() || !text.trim()} onClick={() => void process()}>{processing ? 'Working locally…' : 'Create candidate'}</button>
      {candidate && <div className="candidate"><h3>Review {candidatePieces.length || 1} editable piece{candidatePieces.length === 1 ? '' : 's'}</h3>{candidatePieces.length > 0 ? candidatePieces.map((piece, index) => <div className="candidate-piece" key={index}><label>Piece {index + 1}<textarea value={piece} onChange={(event) => updatePiece(index, event.target.value)} /></label><button onClick={() => removePiece(index)}>Remove piece</button></div>) : null}<label>Combined preview<textarea value={candidate} onChange={(event) => setCandidate(event.target.value)} /></label><div className="candidate-actions"><button onClick={() => { setCandidate(''); setCandidatePieces([]) }}>Discard</button><button className="primary" disabled={processing || !candidate.trim()} onClick={() => void accept()}>Accept candidate</button></div></div>}
    </div>
  </section>
}

const labels = {} as const

export function StudioPage({ route, project }: { route: 'editor'; project: ProjectSnapshot | null }): JSX.Element {
  const [title, copy] = ['Chapter editor', 'Edit and prepare each spoken segment.']
  return <div className="page"><Header eyebrow="Production" title={title} copy={copy} /><section className="panel placeholder"><span>{project ? project.title : 'Ready for a project'}</span><h2>{project ? 'This workflow connects in the next stage' : 'Import a manuscript to begin'}</h2><p>{project ? 'Your project is saved and ready for local audio operations.' : 'Create or open a project from the Projects screen.'}</p></section></div>
}

export function pendingBatchChapterIds(project: ProjectSnapshot): string[] {
  return project.chapters.filter((chapter) => (!chapter.audioPath || (chapter.audioStale && chapter.audioOrigin === 'generated')) && Boolean((chapter.processedText ?? chapter.sourceText).trim())).map((chapter) => chapter.id)
}

export function selectedBatchChapterIds(project: ProjectSnapshot, selectedIds: string[]): string[] {
  return project.chapters.filter((chapter) => selectedIds.includes(chapter.id) && Boolean((chapter.processedText ?? chapter.sourceText).trim())).map((chapter) => chapter.id)
}

export function ReviewPage({ project, onProjectChange, onOpenQueue }: { project: ProjectSnapshot | null; onProjectChange: (project: ProjectSnapshot) => void; onOpenQueue: () => void }): JSX.Element {
  const [busyId, setBusyId] = useState('')
  const [error, setError] = useState('')
  const [voices, setVoices] = useState<Voice[]>([])
  const [settings, setSettings] = useState<Settings | null>(null)
  const [pickerOpen, setPickerOpen] = useState(false)
  const [recordingSegmentId, setRecordingSegmentId] = useState('')
  const [recordingRange, setRecordingRange] = useState<{ startMs: number; endMs: number } | undefined>()
  const [selectedChapters, setSelectedChapters] = useState<string[]>([])
  const [batchJobId, setBatchJobId] = useState('')
  const [batchMessage, setBatchMessage] = useState('')
  useEffect(() => setSelectedChapters((ids) => ids.filter((id) => project?.chapters.some((chapter) => chapter.id === id) ?? false)), [project])
  useEffect(() => { if (isDesktop()) void Promise.all([productionApi.voices(), systemApi.settings()]).then(([found, current]) => { setVoices(found); setSettings(current) }).catch((cause) => setError(errorMessage(cause))) }, [])
  useEffect(() => {
    if (!isDesktop() || !project?.rootPath) return
    setBatchJobId(window.sessionStorage.getItem(`review-batch:${project.rootPath}`) ?? '')
    void projectApi.open(project.rootPath).then(onProjectChange).catch((cause) => setError(errorMessage(cause)))
  }, [project?.rootPath])
  useEffect(() => {
    if (!batchJobId) return
    const refresh = (): void => { void systemApi.jobs().then((jobs) => {
      const current = jobs.find((job) => job.id === batchJobId)
      if (!current || !['completed', 'failed', 'cancelled'].includes(current.status)) return
      setBatchJobId('')
      if (project?.rootPath) window.sessionStorage.removeItem(`review-batch:${project.rootPath}`)
      setBatchMessage(current.status === 'completed' ? 'Batch narration finished.' : '')
      if (current.status !== 'completed') setError(current.message ?? `Batch narration ${current.status}.`)
      if (project?.rootPath) void projectApi.open(project.rootPath).then(onProjectChange).catch((cause) => setError(errorMessage(cause)))
    }).catch((cause) => setError(errorMessage(cause))) }
    refresh()
    const timer = window.setInterval(refresh, 1000)
    return () => window.clearInterval(timer)
  }, [batchJobId, project?.rootPath])
  if (!project) return <div className="page"><Header eyebrow="Production" title="Review" copy="Listen to chapter audio and approve it." /><section className="panel placeholder"><h2>Open a project first</h2></section></div>
  const activeProject = project
  const pendingIds = pendingBatchChapterIds(activeProject)
  const selectedRenderIds = selectedBatchChapterIds(activeProject, selectedChapters)
  const busy = Boolean(busyId || batchJobId)
  async function startBatch(ids: string[], replaceExisting: boolean): Promise<void> {
    if (!ids.length || busy) return
    const replacements = ids.filter((id) => activeProject.chapters.some((chapter) => chapter.id === id && Boolean(chapter.audioPath))).length
    if (replaceExisting && replacements && !window.confirm(`(Re)Generate ${ids.length} selected chapters? This will replace the current audio for ${replacements} chapter${replacements === 1 ? '' : 's'}.`)) return
    setBusyId('batch'); setError(''); setBatchMessage('')
    try { const id = await productionApi.generateChapters(activeProject, ids); window.sessionStorage.setItem(`review-batch:${activeProject.rootPath}`, id); setBatchJobId(id); setBatchMessage(`Queued ${ids.length} chapter${ids.length === 1 ? '' : 's'} for narration.`) }
    catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
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
  async function generateSegment(chapterId: string, segmentId: string): Promise<void> {
    setBusyId(segmentId); setError('')
    try { await waitForJob(await productionApi.generateSegment(activeProject, chapterId, segmentId)) } catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
  async function convertRecording(chapterId: string, segmentId: string, bytes: number[]): Promise<void> {
    setBusyId(segmentId); setError('')
    try {
      await waitForJob(await productionApi.convertSegmentRecording(activeProject, chapterId, segmentId, bytes, recordingRange))
      setRecordingSegmentId('')
      setRecordingRange(undefined)
    } catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
  async function generateRemaining(chapter: Chapter): Promise<void> {
    setBusyId(chapter.id); setError('')
    try {
      for (const segment of chapter.segments.filter((item) => !item.selectedTake)) {
        const fresh = await projectApi.open(activeProject.rootPath)
        await waitForJob(await productionApi.generateSegment(fresh, chapter.id, segment.id))
      }
    } catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
  async function assemble(chapterId: string): Promise<void> {
    setBusyId(chapterId); setError('')
    try { await waitForJob(await productionApi.assembleTakes(activeProject, chapterId)) } catch (cause) { setError(errorMessage(cause)) } finally { setBusyId('') }
  }
  async function selectTake(chapterId: string, segmentId: string, takeId: string): Promise<void> {
    setError('')
    try { onProjectChange(await productionApi.selectTake(activeProject, chapterId, segmentId, takeId)) } catch (cause) { setError(errorMessage(cause)) }
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
  async function exportAudio(chapter: Chapter): Promise<void> {
    setError('')
    try { await productionApi.exportChapterAudio(activeProject, chapter.id, chapter.title) } catch (cause) { setError(errorMessage(cause)) }
  }
  async function deleteAudio(chapter: Chapter): Promise<void> {
    if (!window.confirm(`Delete the current generated audio for “${chapter.title}”?`)) return
    setError('')
    try { onProjectChange(await productionApi.deleteGeneratedAudio(activeProject, chapter.id)) } catch (cause) { setError(errorMessage(cause)) }
  }
  async function saveSelectedChapters(): Promise<void> {
    setError('')
    try { await productionApi.saveChapters(activeProject, selectedChapters.filter((id) => activeProject.chapters.some((chapter) => chapter.id === id && chapter.audioPath))) } catch (cause) { setError(errorMessage(cause)) }
  }
  async function deleteSelectedChapters(): Promise<void> {
    const ids = selectedChapters.filter((id) => activeProject.chapters.some((chapter) => chapter.id === id && chapter.audioOrigin === 'generated'))
    if (!ids.length || !window.confirm(`Delete ${ids.length} selected generated chapter audio file${ids.length === 1 ? '' : 's'}?`)) return
    setError('')
    try { onProjectChange(await productionApi.deleteGeneratedAudioMany(activeProject, ids)); setSelectedChapters([]) } catch (cause) { setError(errorMessage(cause)) }
  }
  const selectedVoice = voices.find((voice) => voice.id === settings?.speech.voiceId)
  async function selectVoice(voice: Voice): Promise<void> {
    if (!settings) return
    try { setSettings(await systemApi.saveSettings({ ...settings, speech: { ...settings.speech, voiceId: voice.id } })) } catch (cause) { setError(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow={project.title} title="Narrate and review" copy="Generate with a local Chatterbox voice or import existing chapter audio." />
    {error && <div className="inline-error">{error}</div>}
    <section className="panel review-voice"><div><strong>Narration voice</strong><span>{selectedVoice?.name ?? 'Loading voices…'}</span></div><button onClick={() => setPickerOpen(true)} disabled={!settings}>Choose voice</button></section>
    {batchMessage && <div className="status-banner">{batchMessage} <button onClick={onOpenQueue}>View render queue</button></div>}
    <section className="panel"><div className="actions">
      <button disabled={!project.chapters.length} onClick={() => setSelectedChapters(project.chapters.map((chapter) => chapter.id))}>Select all</button>
      <button disabled={!selectedChapters.length} onClick={() => setSelectedChapters([])}>Deselect all</button>
      <button disabled={busy || !pendingIds.length} onClick={() => void startBatch(pendingIds, false)}>Generate pending chapters · {pendingIds.length}</button>
      <button className="primary" disabled={busy || !selectedRenderIds.length} onClick={() => void startBatch(selectedRenderIds, true)}>(Re)Generate selected · {selectedRenderIds.length}</button>
      <button disabled={!selectedChapters.some((id) => project.chapters.some((chapter) => chapter.id === id && chapter.audioPath))} onClick={() => void saveSelectedChapters()}>Save selected</button>
      <button disabled={!selectedChapters.some((id) => project.chapters.some((chapter) => chapter.id === id && chapter.audioOrigin === 'generated'))} onClick={() => void deleteSelectedChapters()}>Delete selected generations</button>
    </div></section><section className="review-list">{project.chapters.map((chapter) => <article className="panel review-card" key={chapter.id}><div className="review-copy"><label className="sound-select"><input type="checkbox" aria-label={`Select ${chapter.title}`} checked={selectedChapters.includes(chapter.id)} onChange={(event) => setSelectedChapters((ids) => event.target.checked ? [...ids, chapter.id] : ids.filter((id) => id !== chapter.id))} /> Select</label><small>Chapter {chapter.order + 1}</small><h2>{chapter.title}</h2><span>{chapter.audioPath ? `${formatDuration(chapter.audioDurationMs)} · ${chapter.audioOrigin}${chapter.audioStale ? ' · stale' : ''}` : `${chapter.segments.length} text segments`}</span></div><div className="review-controls">{chapter.audioPath && !chapter.audioStale && <ChapterAudio project={project} chapter={chapter} onEditCue={(order, range) => { setRecordingSegmentId(chapter.segments[order]?.id ?? ''); setRecordingRange(range) }} />}<div className="actions"><button disabled={busy} onClick={() => void importAudio(chapter.id)}>Import audio</button><button className="primary" disabled={busy} onClick={() => void generate(chapter.id)}>{busyId === chapter.id ? 'Processing…' : chapter.audioPath ? 'Regenerate' : 'Generate audio'}</button>{chapter.audioPath && <button disabled={busy} onClick={() => void exportAudio(chapter)}>Export audio</button>}{chapter.audioOrigin === 'generated' && <button disabled={busy} onClick={() => void deleteAudio(chapter)}>Delete generation</button>}</div><details className="segment-editor" open={chapter.segments.some((item) => item.id === recordingSegmentId) ? true : undefined}><summary>Manual sections · {chapter.segments.filter((item) => item.selectedTake).length}/{chapter.segments.length} narrated</summary><div className="actions"><button disabled={busy || chapter.segments.every((item) => item.selectedTake)} onClick={() => void generateRemaining(chapter)}>Generate remaining</button><button disabled={busy || !chapter.segments.length || chapter.segments.some((item) => !item.selectedTake)} onClick={() => void assemble(chapter.id)}>Assemble chapter</button></div>{chapter.segments.map((segment) => <div className="segment-row" key={segment.id}><p>{segment.text}</p><span>{segment.selectedTake ? `Narrated · ${segment.takes.length} take${segment.takes.length === 1 ? '' : 's'}` : 'Needs narration'}</span><button disabled={busy} onClick={() => void generateSegment(chapter.id, segment.id)}>{busyId === segment.id ? 'Working…' : 'Narrate section'}</button><button disabled={busy} onClick={() => { setRecordingSegmentId(recordingSegmentId === segment.id ? '' : segment.id); setRecordingRange(undefined) }}>{recordingSegmentId === segment.id ? 'Close recorder' : 'Record delivery'}</button>{recordingSegmentId === segment.id && <SectionRecorder disabled={busy} range={recordingRange} onConvert={(bytes) => void convertRecording(chapter.id, segment.id, bytes)} />}{segment.takes.length > 0 && <div className="segment-takes">{segment.takes.map((take, index) => <SegmentTakePlayer key={take.id} project={project} chapterId={chapter.id} segmentId={segment.id} takeId={take.id} label={`Take ${index + 1}${segment.selectedTake === take.id ? ' · selected' : ''}`} selected={segment.selectedTake === take.id} onSelect={() => void selectTake(chapter.id, segment.id, take.id)} />)}</div>}</div>)}</details>{chapter.audioPath && !chapter.audioStale && <div className="review-actions"><button className={chapter.reviewStatus === 'changes_requested' ? 'selected' : ''} onClick={() => void review(chapter.id, 'changes_requested')}>Needs changes</button><button className={chapter.reviewStatus === 'approved' ? 'selected approved' : ''} onClick={() => void review(chapter.id, 'approved')}>Approve</button></div>}</div></article>)}</section>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={selectedVoice?.id ?? ''} onSelect={(voice) => void selectVoice(voice)} onClose={() => setPickerOpen(false)} />
  </div>
}

function SegmentTakePlayer({ project, chapterId, segmentId, takeId, label, selected, onSelect }: { project: ProjectSnapshot; chapterId: string; segmentId: string; takeId: string; label: string; selected: boolean; onSelect: () => void }): JSX.Element {
  const [url, setUrl] = useState('')
  useEffect(() => { void productionApi.takeUrl(project, chapterId, segmentId, takeId).then(setUrl).catch(() => setUrl('')) }, [project.rootPath, chapterId, segmentId, takeId])
  return <div className="segment-take"><span>{label}</span>{url && <audio controls preload="none" src={url} />}<button disabled={selected} onClick={onSelect}>{selected ? 'Using' : 'Use take'}</button></div>
}

function SectionRecorder({ disabled, range, onConvert }: { disabled: boolean; range?: { startMs: number; endMs: number }; onConvert: (bytes: number[]) => void }): JSX.Element {
  const recorder = useRef<MediaRecorder | null>(null)
  const stream = useRef<MediaStream | null>(null)
  const timer = useRef<number | null>(null)
  const chunks = useRef<Blob[]>([])
  const [recording, setRecording] = useState(false)
  const [clip, setClip] = useState<Blob | null>(null)
  const [previewUrl, setPreviewUrl] = useState('')
  const [error, setError] = useState('')
  useEffect(() => {
    if (!clip) { setPreviewUrl(''); return }
    const url = URL.createObjectURL(clip)
    setPreviewUrl(url)
    return () => URL.revokeObjectURL(url)
  }, [clip])
  useEffect(() => () => {
    if (timer.current !== null) window.clearTimeout(timer.current)
    if (recorder.current?.state === 'recording') recorder.current.stop()
    stream.current?.getTracks().forEach((track) => track.stop())
  }, [])
  async function start(): Promise<void> {
    setError(''); setClip(null)
    try {
      const mic = await navigator.mediaDevices.getUserMedia({ audio: true })
      stream.current = mic
      const mimeType = ['audio/mp4', 'audio/webm;codecs=opus', 'audio/webm'].find((type) => MediaRecorder.isTypeSupported(type))
      const capture = new MediaRecorder(mic, mimeType ? { mimeType } : undefined)
      chunks.current = []
      capture.ondataavailable = (event) => { if (event.data.size) chunks.current.push(event.data) }
      capture.onstop = () => {
        setRecording(false)
        stream.current?.getTracks().forEach((track) => track.stop())
        stream.current = null
        setClip(new Blob(chunks.current, { type: capture.mimeType }))
      }
      capture.start()
      recorder.current = capture
      setRecording(true)
      timer.current = window.setTimeout(stop, 20000)
    } catch (cause) { setError(errorMessage(cause)); stream.current?.getTracks().forEach((track) => track.stop()) }
  }
  function stop(): void {
    if (timer.current !== null) window.clearTimeout(timer.current)
    timer.current = null
    if (recorder.current?.state === 'recording') recorder.current.stop()
  }
  async function convert(): Promise<void> {
    if (!clip?.size) { setError('Record a passage first.'); return }
    onConvert(Array.from(new Uint8Array(await clip.arrayBuffer())))
  }
  return <div className="section-recorder"><p>{range ? `Read the selected ${formatDuration(range.endMs - range.startMs)} passage with your preferred delivery. It will replace only that part of the selected section take.` : 'Read this section with your preferred timing and emphasis.'} Original Chatterbox will convert your delivery to the chosen narrator voice. Listen to the new take before choosing it.</p><div className="actions"><button disabled={disabled} onClick={() => recording ? stop() : void start()}>{recording ? 'Stop recording' : 'Start recording'}</button>{clip && <button className="primary" disabled={disabled} onClick={() => void convert()}>Convert to narrator voice</button>}</div>{recording && <span>Recording · stops after 20 seconds</span>}{previewUrl && <audio controls src={previewUrl} />}{error && <div className="inline-error">{error}</div>}</div>
}

function ChapterAudio({ project, chapter, onEditCue }: { project: ProjectSnapshot; chapter: Chapter; onEditCue: (order: number, range?: { startMs: number; endMs: number }) => void }): JSX.Element {
  const [url, setUrl] = useState('')
  const [waveform, setWaveform] = useState<number[]>([])
  const [zoom, setZoom] = useState(1)
  const [pan, setPan] = useState(0)
  const [currentMs, setCurrentMs] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [activeCue, setActiveCue] = useState(-1)
  const [selectedCue, setSelectedCue] = useState(-1)
  const [selectedEditRange, setSelectedEditRange] = useState<{ startMs: number; endMs: number } | null>(null)
  const dragStart = useRef<number | null>(null)
  const canEditCues = chapter.cues.length === chapter.segments.length && chapter.cues.length > 0 && chapter.segments.every((item) => Boolean(item.selectedTake))
  const cue = chapter.cues[selectedCue]
  const highlightedRange = selectedEditRange ?? cue
  const audio = useRef<HTMLAudioElement>(null)
  const totalMs = chapter.audioDurationMs ?? 0
  const visibleMs = Math.min(totalMs, 3_600_000, Math.max(1000, totalMs / zoom))
  const windowStart = Math.round(Math.max(0, totalMs - visibleMs) * pan / 1000)
  const windowEnd = Math.min(totalMs, Math.ceil(windowStart + visibleMs))
  useEffect(() => { void productionApi.audioUrl(project, chapter.id).then(setUrl).catch(() => setUrl('')) }, [project.rootPath, chapter.id, chapter.audioPath])
  useEffect(() => {
    if (windowEnd <= windowStart) { setWaveform([]); return }
    let active = true
    const timer = window.setTimeout(() => {
      void productionApi.waveformWindow(project, chapter.id, windowStart, windowEnd).then((peaks) => { if (active) setWaveform(peaks) }).catch(() => { if (active) setWaveform([]) })
    }, 120)
    return () => { active = false; window.clearTimeout(timer) }
  }, [project.rootPath, chapter.id, chapter.audioPath, windowStart, windowEnd])
  function timeAt(clientX: number, element: HTMLElement): number {
    const ratio = Math.max(0, Math.min(1, (clientX - element.getBoundingClientRect().left) / element.clientWidth))
    return windowStart + ratio * visibleMs
  }
  function chooseRange(from: number, to: number): void {
    const start = Math.min(from, to)
    const end = Math.max(from, to)
    if (audio.current) audio.current.currentTime = start / 1000
    const index = chapter.cues.findIndex((item) => start >= item.startMs && start < item.endMs)
    if (index < 0) return
    const selected = chapter.cues[index]
    if (!selected) return
    setSelectedCue(index)
    const clippedEnd = Math.min(end, selected.endMs, start + 20_000)
    setSelectedEditRange(clippedEnd - start >= 150 ? { startMs: start, endMs: clippedEnd } : null)
    if (audio.current) audio.current.currentTime = start / 1000
  }
  return url ? <div className="chapter-player">
    <div className="review-transport"><button onClick={() => { if (!audio.current) return; if (audio.current.paused) void audio.current.play(); else audio.current.pause() }}>{playing ? 'Pause' : 'Play'}</button><button onClick={() => { if (!audio.current) return; audio.current.pause(); audio.current.currentTime = 0; setCurrentMs(0) }}>Stop</button><span>{formatDuration(currentMs)} / {formatDuration(totalMs)}</span><button disabled={zoom <= 1} onClick={() => setZoom((value) => Math.max(1, value / 2))}>Zoom out</button><button disabled={zoom >= 16} onClick={() => setZoom((value) => Math.min(16, value * 2))}>Zoom in</button><span>{zoom}×</span></div>
    {totalMs > visibleMs && <label className="waveform-pan">Scroll through chapter <input type="range" min="0" max="1000" value={pan} onChange={(event) => setPan(Number(event.target.value))} /><span>{formatDuration(windowStart)}–{formatDuration(windowEnd)}</span></label>}
    {waveform.length > 0 && <div className="waveform" role={canEditCues ? 'button' : undefined} tabIndex={canEditCues ? 0 : undefined} aria-label={canEditCues ? 'Drag to select up to 20 seconds within a spoken section' : 'Audio waveform'}
      onWheel={(event) => { if (totalMs > visibleMs && (event.shiftKey || Math.abs(event.deltaX) > Math.abs(event.deltaY))) { event.preventDefault(); setPan((value) => Math.max(0, Math.min(1000, value + Math.sign(event.deltaX || event.deltaY) * 40))) } }}
      onPointerDown={(event) => { dragStart.current = timeAt(event.clientX, event.currentTarget); event.currentTarget.setPointerCapture(event.pointerId) }}
      onPointerUp={(event) => { if (dragStart.current === null) return; chooseRange(dragStart.current, timeAt(event.clientX, event.currentTarget)); dragStart.current = null; event.currentTarget.releasePointerCapture(event.pointerId) }}
      onKeyDown={(event) => { if (canEditCues && event.key === 'Enter') { event.preventDefault(); setSelectedCue(activeCue >= 0 ? activeCue : 0); setSelectedEditRange(null) } }}>
      {waveform.map((peak, index) => { const at = windowStart + index / waveform.length * visibleMs; return <i key={index} className={highlightedRange && at >= highlightedRange.startMs && at <= highlightedRange.endMs ? 'selected' : currentMs >= at && currentMs < at + visibleMs / waveform.length ? 'playing' : ''} style={{ height: `${Math.max(8, peak * 100)}%` }} /> })}
    </div>}
    {cue && canEditCues && <div className="waveform-selection"><span>{selectedEditRange ? 'Selected passage' : 'Selected section'}: {formatDuration(highlightedRange?.startMs ?? cue.startMs)}–{formatDuration(highlightedRange?.endMs ?? cue.endMs)}</span><button onClick={() => onEditCue(selectedCue, selectedEditRange ? { startMs: Math.round(selectedEditRange.startMs - cue.startMs), endMs: Math.round(selectedEditRange.endMs - cue.startMs) } : undefined)}>Record replacement</button></div>}
    <audio ref={audio} preload="metadata" src={url} onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onEnded={() => setPlaying(false)} onTimeUpdate={(event) => { const ms = event.currentTarget.currentTime * 1000; setCurrentMs(ms); setActiveCue(chapter.cues.findIndex((item) => ms >= item.startMs && ms < item.endMs)) }} />
    {chapter.cues.length > 0 ? <div className="spoken-cues" aria-label="Spoken lines">{chapter.cues.map((item) => <button key={item.order} className={activeCue === item.order ? 'active' : ''} onClick={() => { if (audio.current) audio.current.currentTime = item.startMs / 1000 }}>{item.text}</button>)}</div> : <span className="field-note">Line timing is available for newly generated narration.</span>}
  </div> : <span>Preparing player…</span>
}

function formatDuration(value: number | null): string {
  if (!value) return 'Duration unavailable'
  const seconds = Math.round(value / 1000)
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}

export function VoicesPage(): JSX.Element {
  const [voices, setVoices] = useState<Voice[]>([])
  const [settings, setSettings] = useState<Settings | null>(null)
  const [message, setMessage] = useState('Loading voices…')
  const [name, setName] = useState('')
  const [sampleName, setSampleName] = useState('Reference sample')
  const [pickerOpen, setPickerOpen] = useState(false)
  const [previewUrl, setPreviewUrl] = useState('')
  const [sampleUrl, setSampleUrl] = useState('')
  const [recording, setRecording] = useState(false)
  const [recordingVoiceId, setRecordingVoiceId] = useState('')
  const [level, setLevel] = useState(0)
  const [recorded, setRecorded] = useState<Blob | null>(null)
  const [recordedUrl, setRecordedUrl] = useState('')
  const recorder = useRef<MediaRecorder | null>(null)
  const recordingStream = useRef<MediaStream | null>(null)
  const meter = useRef<number | null>(null)
  const recordingContext = useRef<AudioContext | null>(null)
  useEffect(() => { void Promise.all([productionApi.voices(), systemApi.settings()]).then(([found, current]) => { setVoices(found); setSettings(current); setMessage('') }).catch((cause) => setMessage(errorMessage(cause))) }, [])
  useEffect(() => () => { recorder.current?.stop(); recordingStream.current?.getTracks().forEach((track) => track.stop()); if (meter.current !== null) window.clearInterval(meter.current); void recordingContext.current?.close() }, [])
  useEffect(() => { if (!recorded) { setRecordedUrl(''); return }; const url = URL.createObjectURL(recorded); setRecordedUrl(url); return () => URL.revokeObjectURL(url) }, [recorded])
  const selected = voices.find((voice) => voice.id === settings?.speech.voiceId) ?? voices[0]
  async function refresh(): Promise<void> { setVoices(await productionApi.voices()) }
  async function select(voice: Voice): Promise<void> {
    if (!settings) return
    try { const saved = await systemApi.saveSettings({ ...settings, speech: { ...settings.speech, provider: 'chatterbox_turbo', voiceId: voice.id } }); setSettings(saved); setMessage(`${voice.name} selected`) }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function create(): Promise<void> {
    if (!name.trim()) return
    try { const voice = await productionApi.createVoice(name); await refresh(); setName(''); setMessage(`Created ${voice.name}. Add a spoken sample to use it.`) }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function addFile(voice: Voice): Promise<void> {
    try { const path = await chooseAudio(); if (!path) return; await productionApi.addVoiceSample(voice.id, sampleName || 'Imported sample', path); await refresh(); setMessage('Sample copied into Homer Studio. You can move the original file. Preparing a voice preview in the background.') }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function preview(voice: Voice): Promise<void> {
    setPreviewUrl(''); setMessage('Loading voice preview…')
    if (voice.id === 'chatterbox-default') { setPreviewUrl('/default-voice-preview.m4a'); setMessage('Preview ready'); return }
    try { setPreviewUrl(await productionApi.previewVoice(voice.id, 180)); setMessage('Preview ready') }
    catch (cause) {
      const detail = errorMessage(cause)
      if (voice.selectedSampleId) {
        try { setPreviewUrl(await productionApi.voiceSampleUrl(voice.id, voice.selectedSampleId)); setMessage(`${detail} Playing your saved reference recording.`); return }
        catch { /* Show the original preview error. */ }
      }
      setMessage(detail)
    }
  }
  async function listenSample(voice: Voice, sampleId: string): Promise<void> {
    try { setSampleUrl(await productionApi.voiceSampleUrl(voice.id, sampleId)) } catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function startRecording(voiceId: string): Promise<void> {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      recordingStream.current = stream
      const context = new AudioContext()
      recordingContext.current = context
      const analyser = context.createAnalyser()
      context.createMediaStreamSource(stream).connect(analyser)
      const levels = new Uint8Array(analyser.frequencyBinCount)
      meter.current = window.setInterval(() => { analyser.getByteTimeDomainData(levels); setLevel(Math.min(100, Math.round(Math.sqrt(levels.reduce((sum, value) => sum + (value - 128) ** 2, 0) / levels.length) * 3))) }, 100)
      const chunks: Blob[] = []
      const mimeType = ['audio/webm', 'audio/mp4'].find((type) => MediaRecorder.isTypeSupported(type))
      const media = new MediaRecorder(stream, mimeType ? { mimeType } : undefined)
      recorder.current = media
      media.ondataavailable = (event) => { if (event.data.size) chunks.push(event.data) }
      media.onstop = () => {
        setRecorded(new Blob(chunks, { type: media.mimeType }))
        stream.getTracks().forEach((track) => track.stop())
        if (meter.current !== null) window.clearInterval(meter.current)
        void context.close(); setRecording(false); setLevel(0)
      }
      media.start(); setRecorded(null); setRecordingVoiceId(voiceId); setRecording(true); setMessage('Recording. Speak naturally for 6–20 seconds.')
      window.setTimeout(() => { if (media.state === 'recording') media.stop() }, 20_000)
    } catch (cause) { setMessage(`Microphone unavailable: ${errorMessage(cause)}`) }
  }
  async function saveRecording(voice: Voice): Promise<void> {
    if (!recorded || recordingVoiceId !== voice.id) return
    try { await productionApi.addRecordedVoiceSample(voice.id, sampleName || 'Recorded sample', Array.from(new Uint8Array(await recorded.arrayBuffer()))); setRecorded(null); setRecordingVoiceId(''); await refresh(); setMessage('Recording saved in Homer Studio. Preparing a voice preview in the background.') }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function remove(voice: Voice): Promise<void> {
    try { await productionApi.deleteVoice(voice.id); await refresh(); setMessage(`${voice.name} deleted`) }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow="Production" title="Voice library" copy="Use Chatterbox's natural voice or create a voice from your own speech. Samples stay on this computer." action={<button onClick={() => setPickerOpen(true)}>Choose voice</button>} />
    {message && <div className="status-banner" role="status">{message}</div>}
    {previewUrl && <section className="voice-preview"><audio controls autoPlay src={previewUrl} /><button onClick={() => setPreviewUrl('')}>Close</button></section>}
    <section className="panel preset-editor"><h2>Create a voice</h2><p className="field-note">Record or import clear 6–20 second samples of the same speaker. Choose one sample for generation; distinct speakers are not blended.</p><label>Voice name<input value={name} maxLength={80} onChange={(event) => setName(event.target.value)} placeholder="My narrator" /></label><button className="primary" disabled={!name.trim()} onClick={() => void create()}>Create voice</button></section>
    <section className="voice-section"><h2>Your voices</h2><div className="voice-grid">{voices.map((voice) => <article className={selected?.id === voice.id ? 'voice-card selected' : 'voice-card'} key={voice.id}><div className="voice-heading"><strong>{voice.name}</strong><small>{voice.builtIn ? 'Model voice' : `${voice.samples.length} sample${voice.samples.length === 1 ? '' : 's'}`}</small></div><div className="voice-actions"><button onClick={() => void select(voice)} disabled={!voice.builtIn && !voice.selectedSampleId}>Select</button><button onClick={() => void preview(voice)} disabled={!voice.builtIn && !voice.selectedSampleId}>Preview</button>{!voice.builtIn && <button onClick={() => void remove(voice)} disabled={selected?.id === voice.id}>Delete</button>}</div>{!voice.builtIn && <><label>Sample name<input value={sampleName} onChange={(event) => setSampleName(event.target.value)} /></label><div className="voice-actions"><button onClick={() => void addFile(voice)}>Import sample</button><button disabled={recording && recordingVoiceId !== voice.id} onClick={() => recording ? recorder.current?.stop() : void startRecording(voice.id)}>{recording && recordingVoiceId === voice.id ? 'Stop recording' : 'Record sample'}</button></div>{recording && recordingVoiceId === voice.id && <div className="recording-meter" role="meter" aria-label="Microphone level" aria-valuemin={0} aria-valuemax={100} aria-valuenow={level}><span style={{ width: `${level}%` }} /></div>}{recorded && recordingVoiceId === voice.id && <div className="recording-preview"><audio controls src={recordedUrl} /><button onClick={() => void saveRecording(voice)}>Save recording</button><button onClick={() => { setRecorded(null); setRecordingVoiceId('') }}>Retry</button></div>}{voice.samples.map((sample) => <div className="sample-row" key={sample.id}><span>{sample.name} · {(sample.durationMs / 1000).toFixed(1)}s</span><button onClick={() => void listenSample(voice, sample.id)}>Listen</button><button onClick={() => void productionApi.selectVoiceSample(voice.id, sample.id).then(refresh).catch((cause) => setMessage(errorMessage(cause)))} disabled={voice.selectedSampleId === sample.id}>{voice.selectedSampleId === sample.id ? 'Selected' : 'Use sample'}</button></div>)}{sampleUrl && <audio controls src={sampleUrl} />}</>}</article>)}</div></section>
    <p className="field-note">Shape delivery with punctuation, line breaks, and cues: [pause:800], [sigh], [gasp], [cough], [laugh], [chuckle], or [groan]. Example: “Tomorrow, and tomorrow, and tomorrow… [pause:800] creeps in this petty pace…” Put emphasis on a word with punctuation and sentence structure; exact word-level controls are not available yet. Cues are removed from the displayed spoken-line text.</p>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={selected?.id ?? ''} onSelect={(voice) => void select(voice)} onClose={() => setPickerOpen(false)} onPreview={(voice) => { setPickerOpen(false); void preview(voice) }} />
  </div>
}

export function QueuePage(): JSX.Element {
  const [jobs, setJobs] = useState<JobRecord[]>([])
  const [error, setError] = useState('')
  const [selected, setSelected] = useState<string[]>([])
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
  const terminal = jobs.filter((job) => ['completed', 'failed', 'cancelled'].includes(job.status))
  async function clean(): Promise<void> {
    try { await systemApi.dismissJobs(selected); setSelected([]); setJobs(await systemApi.jobs()) } catch (cause) { setError(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow="Production" title="Render queue" copy="Heavy local work runs one job at a time." />{error && <div className="inline-error" role="alert">{error}</div>}<section className="panel"><div className="queue-toolbar"><span>{selected.length} finished jobs selected</span><div className="actions"><button onClick={() => setSelected(terminal.map((job) => job.id))} disabled={!terminal.length}>Select all</button><button onClick={() => setSelected([])} disabled={!selected.length}>Deselect all</button><button onClick={() => void clean()} disabled={!selected.length}>Clean selected</button></div></div>{jobs.length === 0 ? <div className="table-empty">No jobs yet.</div> : <div className="job-list">{jobs.map((job) => <article className="queue-job" key={job.id}><div className="queue-job-top"><label className="queue-select"><input type="checkbox" aria-label={`Select ${job.label}`} disabled={!['completed', 'failed', 'cancelled'].includes(job.status)} checked={selected.includes(job.id)} onChange={(event) => setSelected((ids) => event.target.checked ? [...ids, job.id] : ids.filter((id) => id !== job.id))} /></label><div className="queue-job-detail"><strong>{job.label}</strong><small>{job.kind} · {job.status} · {job.progress}%</small>{job.message && <p className={job.status === 'failed' ? 'queue-error' : ''}>{job.message}</p>}</div><div className="queue-job-actions">{['queued', 'running'].includes(job.status) && <><button onClick={() => void control(job, 'pause')}>Pause</button><button onClick={() => void control(job, 'resume')}>Resume</button><button onClick={() => void control(job, 'cancel')}>Cancel</button></>}</div></div><progress value={job.progress} max="100" /><details><summary>Activity log</summary><ol className="queue-log">{(job.events ?? []).map((event, index) => <li key={index}><time>{new Date(event.atMs).toLocaleTimeString()}</time><span>{event.status} · {event.progress}% · {event.message}</span></li>)}</ol></details></article>)}</div>}</section></div>
}

export function ExportsPage({ project, onProjectChange }: { project: ProjectSnapshot | null; onProjectChange: (project: ProjectSnapshot) => void }): JSX.Element {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState<string[]>([])
  useEffect(() => setSelected((ids) => ids.filter((id) => project?.exports.some((item) => item.id === id) ?? false)), [project])
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
  async function saveSelected(): Promise<void> {
    setError('')
    try { await productionApi.saveExports(activeProject, selected) } catch (cause) { setError(errorMessage(cause)) }
  }
  async function deleteOne(id: string): Promise<void> {
    if (!window.confirm("Delete this export and its files?")) return
    setError("")
    try { onProjectChange(await productionApi.deleteExports(activeProject, [id])) } catch (cause) { setError(errorMessage(cause)) }
  }
  async function deleteSelected(): Promise<void> {
    if (!selected.length || !window.confirm(`Delete ${selected.length} selected export${selected.length === 1 ? '' : 's'}?`)) return
    setError('')
    try { onProjectChange(await productionApi.deleteExports(activeProject, selected)); setSelected([]) } catch (cause) { setError(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow={project.title} title="Exports" copy="Combine approved chapters and create measured timestamps." action={<button className="primary" disabled={busy || ready !== project.chapters.length} onClick={() => void createExport()}>{busy ? 'Exporting…' : 'Export audiobook'}</button>} />
    {error && <div className="inline-error">{error}</div>}
    {ready !== project.chapters.length && <div className="status-banner">{ready} of {project.chapters.length} chapters have current, approved audio.</div>}
    {ready === project.chapters.length && youtubeNote && <div className="status-banner">The audiobook can be exported. YouTube may not activate chapter marks because {youtubeNote}.</div>}
    <section className="panel export-history"><div className="library-heading"><h2>Export history</h2><div className="actions"><button disabled={!project.exports.length} onClick={() => setSelected(project.exports.map((item) => item.id))}>Select all</button><button disabled={!selected.length} onClick={() => setSelected([])}>Deselect all</button><button disabled={!selected.length} onClick={() => void saveSelected()}>Save selected</button><button disabled={!selected.length} onClick={() => void deleteSelected()}>Delete selected</button></div></div>{project.exports.length === 0 ? <div className="table-empty">No exports yet.</div> : [...project.exports].reverse().map((item) => <ExportItem key={item.id} project={project} item={item} stale={item.sourceUpdatedAtMs !== project.updatedAtMs} selected={selected.includes(item.id)} onSelect={(checked) => setSelected((ids) => checked ? [...ids, item.id] : ids.filter((id) => id !== item.id))} onDelete={() => void deleteOne(item.id)} />)}</section>
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

function ExportItem({ project, item, stale, selected, onSelect, onDelete }: { project: ProjectSnapshot; item: ProjectSnapshot['exports'][number]; stale: boolean; selected: boolean; onSelect: (checked: boolean) => void; onDelete: () => void }): JSX.Element {
  const [url, setUrl] = useState('')
  const [timestamps, setTimestamps] = useState('')
  const [copied, setCopied] = useState(false)
  useEffect(() => { void Promise.all([productionApi.exportAudioUrl(project, item.id), productionApi.exportTimestamps(project, item.id)]).then(([audio, text]) => { setUrl(audio); setTimestamps(text) }).catch(() => {}) }, [project.rootPath, item.id])
  async function copy(): Promise<void> { await navigator.clipboard.writeText(timestamps); setCopied(true); window.setTimeout(() => setCopied(false), 1500) }
  return <article className="export-row"><label className="sound-select"><input type="checkbox" checked={selected} onChange={(event) => onSelect(event.target.checked)} aria-label={`Select export ${new Date(item.createdAtMs).toLocaleString()}`} /> Select</label><div><strong>{new Date(item.createdAtMs).toLocaleString()}</strong><small>{formatDuration(item.durationMs)} · {stale ? 'Project changed since export' : 'Current project content'}</small></div>{url && <audio controls preload="metadata" src={url} />}<textarea readOnly value={timestamps} aria-label="YouTube chapter timestamps" /><div className="actions"><button onClick={() => void productionApi.saveExportFile(project, item.id, 'audio')}>Save audio</button><button onClick={() => void productionApi.saveExportFile(project, item.id, 'timestamps')}>Save timestamps</button><button disabled={!timestamps} onClick={() => void copy()}>{copied ? 'Copied' : 'Copy timestamps'}</button><button onClick={onDelete}>Delete</button></div></article>
}

export function SettingsPage(): JSX.Element {
  const [desktop, setDesktop] = useState<DesktopInfo>({ platform: 'macOS', architecture: 'arm64', runtime: 'Browser preview' })
  const [settings, setSettings] = useState<Settings | null>(null)
  const [tools, setTools] = useState<ToolDiagnostic[]>([])
  const [message, setMessage] = useState('')
  const [models, setModels] = useState<ModelStatus[]>([])
  const [installJobs, setInstallJobs] = useState<Partial<Record<'turbo' | 'original', string>>>({})
  const [jobs, setJobs] = useState<JobRecord[]>([])
  const refreshModels = async (): Promise<void> => { setModels(await Promise.all(['turbo', 'original'].map((model) => systemApi.modelStatus(model as 'turbo' | 'original')))) }
  useEffect(() => { if ('__TAURI_INTERNALS__' in window) { void invoke<DesktopInfo>('desktop_info').then(setDesktop); void Promise.all([systemApi.settings(), systemApi.diagnostics()]).then(([value, found]) => { setSettings(value); setTools(found) }).catch((cause) => setMessage(errorMessage(cause))) } }, [])
  useEffect(() => { if (!isDesktop()) return; void refreshModels().catch((cause) => setMessage(errorMessage(cause))); const timer = window.setInterval(() => { void refreshModels().catch(() => {}); void systemApi.jobs().then(setJobs).catch(() => {}) }, 1000); return () => window.clearInterval(timer) }, [])
  async function install(model: 'turbo' | 'original'): Promise<void> {
    try { const id = await systemApi.installModel(model); setInstallJobs((previous) => ({ ...previous, [model]: id })); setJobs(await systemApi.jobs()) } catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function save(): Promise<void> {
    if (!settings) return
    setMessage('Saving…')
    try { setSettings(await systemApi.saveSettings(settings)); setTools(await systemApi.diagnostics()); setMessage('Settings saved') } catch (cause) { setMessage(errorMessage(cause)) }
  }
  return <div className="page">
    <Header eyebrow="System" title="Settings" copy="Configure local tools and model providers." action={<><button disabled={!settings} onClick={() => void systemApi.diagnostics().then(setTools).catch((cause) => setMessage(errorMessage(cause)))}>Check status</button><button className="primary" disabled={!settings} onClick={() => void save()}>Save settings</button></>} />
    {message && <div className="status-banner">{message}</div>}
    <div className="settings-grid">
      <section className="panel">
        <h2>Desktop runtime</h2>
        <dl><div><dt>Platform</dt><dd>{desktop.platform}</dd></div><div><dt>Architecture</dt><dd>{desktop.architecture}</dd></div><div><dt>Runtime</dt><dd>{desktop.runtime}</dd></div></dl>
        <h2>Local tools</h2>
        <div className="tool-list">{tools.map((tool) => <div key={tool.name}><span className={tool.available ? 'dot available' : 'dot'} /><strong>{tool.name}</strong><span className="tool-result"><small>{toolStatus(tool.status)}</small>{tool.path && <small title={tool.path}>{tool.path}</small>}</span></div>)}</div>
        <h2>Chatterbox checkpoints</h2>
        <p className="field-note">Install only the models you use. Downloads stay on this Mac and can resume after cancellation. Start the local Python worker separately.</p>
        {models.map((model) => { const job = jobs.find((item) => item.id === installJobs[model.model]); const active = job?.status === 'running' || job?.status === 'queued'; return <div className="model-install" key={model.model}>
          <div className="model-install-heading"><strong>{model.model === 'turbo' ? 'Chatterbox Turbo' : 'Original Chatterbox'}</strong><span>{model.installed ? 'Installed' : active ? `${job?.progress ?? 0}%` : 'Not installed'}</span></div>
          <small title={model.path}>{model.path}</small>
          <small>{formatBytes(model.bytesOnDisk)} on disk{model.totalBytes ? ` · ${formatBytes(model.downloadedBytes)} of ${formatBytes(model.totalBytes)} downloaded` : ''}</small>
          {active && <progress max={100} value={job?.progress ?? 0} />}
          {job?.status === 'failed' && <small className="error-text">{job.message}</small>}
          <div className="model-install-actions"><button type="button" disabled={active || model.installed} onClick={() => void install(model.model)}>{job?.status === 'failed' || job?.status === 'cancelled' ? 'Retry' : 'Install'}</button>{active && <button type="button" onClick={() => void systemApi.controlJob(job!.id, 'cancel').catch((cause) => setMessage(errorMessage(cause)))}>Cancel</button>}</div>
        </div> })}
      </section>
      {settings && <SettingsForm settings={settings} tools={tools} onChange={setSettings} />}
    </div>
  </div>
}

function formatBytes(bytes: number): string { return bytes >= 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(2)} GB` : bytes >= 1024 ** 2 ? `${(bytes / 1024 ** 2).toFixed(1)} MB` : `${Math.round(bytes / 1024)} KB` }

function toolStatus(status: ToolDiagnostic['status']): string {
  return ({ invalid_configuration: 'Configured path is invalid', not_found: 'Not found', service_unavailable: 'Server API is unavailable', service_reachable: 'Server is reachable', service_ready: 'Ready', no_models: 'Running, but no models installed', model_not_installed: 'Selected model is not installed', configured: 'Configured', found_automatically: 'Found automatically' })[status]
}

function SettingsForm({ settings, tools, onChange }: { settings: Settings; tools: ToolDiagnostic[]; onChange: (value: Settings) => void }): JSX.Element {
  const [voices, setVoices] = useState<Voice[]>([])
  const [pickerOpen, setPickerOpen] = useState(false)
  useEffect(() => { if (isDesktop()) void productionApi.voices().then(setVoices) }, [])
  const ollama = settings.llm.provider === 'ollama' ? settings.llm : null
  const llama = settings.llm.provider === 'llama_cpp' ? settings.llm : null
  const detected = (key: ToolDiagnostic['key']): string => tools.find((tool) => tool.key === key)?.detectedPath ?? ''
  async function pick(title: string, apply: (path: string) => void, extensions?: string[]): Promise<void> {
    const path = await chooseTool(title, extensions)
    if (path) apply(path)
  }
  const pathField = (label: string, value: string, apply: (path: string) => void, key?: ToolDiagnostic['key'], extensions?: string[]): JSX.Element => {
    const detectedPath = key ? detected(key) : ''
    return <label>{label}<div className="path-picker"><input value={value} onChange={(event) => apply(event.target.value)} placeholder={key ? 'Automatically detected when empty' : 'Choose a file'} /><button type="button" onClick={() => void pick(`Choose ${label}`, apply, extensions)}>Choose</button>{detectedPath && value !== detectedPath && <button type="button" onClick={() => apply(detectedPath)}>Use detected</button>}</div></label>
  }
  return <section className="panel settings-form">
    <h2>Speech</h2>
    <label>Narration model<select value={settings.speech.provider} onChange={(event) => onChange({ ...settings, speech: { ...settings.speech, provider: event.target.value as Settings['speech']['provider'] } })}><option value="chatterbox_turbo">Chatterbox Turbo · speech and vocal gestures</option><option value="chatterbox_original">Original Chatterbox · expression controls</option></select></label>
    {settings.speech.provider === 'chatterbox_original' && <div className="expression-controls"><label>Expression · {settings.speech.exaggeration.toFixed(2)}<input type="range" min="0.25" max="2" step="0.05" value={settings.speech.exaggeration} onChange={(event) => onChange({ ...settings, speech: { ...settings.speech, exaggeration: Number(event.target.value) } })} /></label><label>CFG / pace · {settings.speech.cfgWeight.toFixed(2)}<input type="range" min="0" max="1" step="0.05" value={settings.speech.cfgWeight} onChange={(event) => onChange({ ...settings, speech: { ...settings.speech, cfgWeight: Number(event.target.value) } })} /></label></div>}
    <div className="voice-field"><span>Narration voice</span><button onClick={() => setPickerOpen(true)}>{voices.find((voice) => voice.id === settings.speech.voiceId)?.name ?? 'Choose a voice'}</button></div>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={settings.speech.voiceId} onSelect={(voice) => onChange({ ...settings, speech: { ...settings.speech, voiceId: voice.id } })} onClose={() => setPickerOpen(false)} />
    <h2>Audio tools</h2>
    {pathField('FFmpeg path', settings.ffmpegPath ?? '', (path) => onChange({ ...settings, ffmpegPath: path || null }), 'ffmpeg')}
    {pathField('FFprobe path', settings.ffprobePath ?? '', (path) => onChange({ ...settings, ffprobePath: path || null }), 'ffprobe')}
    <h2>Sound workers</h2>
    <p className="field-note">Start each local worker separately. Sound Studio shows whether its model is ready.</p>
    <label>Chatterbox URL<input value={settings.sounds.chatterboxUrl} onChange={(event) => onChange({ ...settings, sounds: { ...settings.sounds, chatterboxUrl: event.target.value } })} /></label>
    <label>Original Chatterbox URL<input value={settings.sounds.originalUrl} onChange={(event) => onChange({ ...settings, sounds: { ...settings.sounds, originalUrl: event.target.value } })} /></label>
    <h2>Text processing</h2>
    <label>Provider<select value={settings.llm.provider} onChange={(event) => onChange({ ...settings, llm: event.target.value === 'ollama' ? { provider: 'ollama', base_url: 'http://127.0.0.1:11434', model: '', context_size: 8192, max_tokens: 2048 } : event.target.value === 'llama_cpp' ? { provider: 'llama_cpp', executable_path: '', model_path: '', context_size: 8192, max_tokens: 2048, gpu_layers: 99 } : { provider: 'none' } })}><option value="none">Disabled</option><option value="llama_cpp">llama.cpp / GGUF</option><option value="ollama">Ollama</option></select></label>
    {ollama && <div><label>Loopback URL<input value={ollama.base_url} onChange={(event) => onChange({ ...settings, llm: { ...ollama, base_url: event.target.value } })} /></label><label>Model<input value={ollama.model} onChange={(event) => onChange({ ...settings, llm: { ...ollama, model: event.target.value } })} /></label>{pathField('Ollama CLI path', settings.ollamaPath ?? '', (path) => onChange({ ...settings, ollamaPath: path || null }), 'ollama')}</div>}
    {llama && <div>{pathField('llama-cli path', llama.executable_path, (path) => onChange({ ...settings, llm: { ...llama, executable_path: path } }), 'llama')}{pathField('GGUF model path', llama.model_path, (path) => onChange({ ...settings, llm: { ...llama, model_path: path } }), undefined, ['gguf'])}</div>}
  </section>
}
