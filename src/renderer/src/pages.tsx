import { useEffect, useRef, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { Chapter, JobRecord, ProjectSnapshot, Settings, ToolDiagnostic, Voice } from '../../shared/contracts'
import type { RouteId } from '../../shared/navigation'
import { chooseAudio, chooseFolder, chooseManuscript, chooseTool, errorMessage, isDesktop, loadDroppedManuscript, productionApi, projectApi, systemApi } from './native'
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
    <div className="split"><section className="panel"><label>Project title<input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="My audiobook" /></label><label className={dragging ? 'manuscript-drop active' : 'manuscript-drop'}>Manuscript <span className="field-note">{dragging ? 'Drop the file to load it' : `${sourceName} · Drop a TXT or Markdown file here`}</span><textarea value={manuscript} onChange={(event) => { setManuscript(event.target.value); setSourceName('Pasted text') }} placeholder="# Chapter One&#10;&#10;Paste your manuscript here…" /></label><label>Project location<div className="path-picker"><input value={parentPath} readOnly placeholder="Choose a parent folder" /><button onClick={() => void pickLocation()}>Choose</button></div></label><button className="primary" disabled={busy || !title.trim() || !manuscript.trim() || !parentPath} onClick={() => void create()}>{busy ? 'Creating…' : 'Create project'}</button></section><aside className="panel tip"><h3>Chapter detection</h3><p>Markdown headings and lines beginning with “Chapter” or “Part” become separate chapters. Text before the first heading becomes an introduction.</p><p className="status">Source text remains readable inside the project folder.</p></aside></div>
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
  const [voices, setVoices] = useState<Voice[]>([])
  const [settings, setSettings] = useState<Settings | null>(null)
  const [pickerOpen, setPickerOpen] = useState(false)
  useEffect(() => { if (isDesktop()) void Promise.all([productionApi.voices(), systemApi.settings()]).then(([found, current]) => { setVoices(found); setSettings(current) }).catch((cause) => setError(errorMessage(cause))) }, [])
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
  const selectedVoice = voices.find((voice) => voice.id === settings?.speech.voiceId)
  async function selectVoice(voice: Voice): Promise<void> {
    if (!settings) return
    try { setSettings(await systemApi.saveSettings({ ...settings, speech: { ...settings.speech, voiceId: voice.id } })) } catch (cause) { setError(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow={project.title} title="Narrate and review" copy="Generate with a local Chatterbox voice or import existing chapter audio." />
    {error && <div className="inline-error">{error}</div>}
    <section className="panel review-voice"><div><strong>Narration voice</strong><span>{selectedVoice?.name ?? 'Loading voices…'}</span></div><button onClick={() => setPickerOpen(true)} disabled={!settings}>Choose voice</button></section>
    <section className="review-list">{project.chapters.map((chapter) => <article className="panel review-card" key={chapter.id}><div className="review-copy"><small>Chapter {chapter.order + 1}</small><h2>{chapter.title}</h2><span>{chapter.audioPath ? `${formatDuration(chapter.audioDurationMs)} · ${chapter.audioOrigin}${chapter.audioStale ? ' · stale' : ''}` : `${chapter.segments.length} text segments`}</span></div><div className="review-controls">{chapter.audioPath && !chapter.audioStale && <ChapterAudio project={project} chapter={chapter} />}<div className="actions"><button disabled={Boolean(busyId)} onClick={() => void importAudio(chapter.id)}>Import audio</button><button className="primary" disabled={Boolean(busyId)} onClick={() => void generate(chapter.id)}>{busyId === chapter.id ? 'Processing…' : chapter.audioPath ? 'Regenerate' : 'Generate audio'}</button></div>{chapter.audioPath && !chapter.audioStale && <div className="review-actions"><button className={chapter.reviewStatus === 'changes_requested' ? 'selected' : ''} onClick={() => void review(chapter.id, 'changes_requested')}>Needs changes</button><button className={chapter.reviewStatus === 'approved' ? 'selected approved' : ''} onClick={() => void review(chapter.id, 'approved')}>Approve</button></div>}</div></article>)}</section>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={selectedVoice?.id ?? ''} onSelect={(voice) => void selectVoice(voice)} onClose={() => setPickerOpen(false)} />
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
    try { const path = await chooseAudio(); if (!path) return; await productionApi.addVoiceSample(voice.id, sampleName || 'Imported sample', path); await refresh(); setMessage('Sample added. Select this voice to use it for narration.') }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function preview(voice: Voice): Promise<void> {
    setPreviewUrl(''); setMessage('Creating neural voice preview…')
    try { setPreviewUrl(await productionApi.previewVoice(voice.id, 180)); setMessage('Preview ready') }
    catch (cause) { setMessage(errorMessage(cause)) }
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
      media.start(); setRecorded(null); setRecordingVoiceId(voiceId); setRecording(true); setMessage('Recording. Speak naturally for about 10 seconds.')
      window.setTimeout(() => { if (media.state === 'recording') media.stop() }, 20_000)
    } catch (cause) { setMessage(`Microphone unavailable: ${errorMessage(cause)}`) }
  }
  async function saveRecording(voice: Voice): Promise<void> {
    if (!recorded || recordingVoiceId !== voice.id) return
    try { await productionApi.addRecordedVoiceSample(voice.id, sampleName || 'Recorded sample', Array.from(new Uint8Array(await recorded.arrayBuffer()))); setRecorded(null); setRecordingVoiceId(''); await refresh(); setMessage('Recorded sample added.') }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  async function remove(voice: Voice): Promise<void> {
    try { await productionApi.deleteVoice(voice.id); await refresh(); setMessage(`${voice.name} deleted`) }
    catch (cause) { setMessage(errorMessage(cause)) }
  }
  return <div className="page"><Header eyebrow="Production" title="Voice library" copy="Use Chatterbox's natural voice or create a voice from your own speech. Samples stay on this computer." action={<button onClick={() => setPickerOpen(true)}>Choose voice</button>} />
    {message && <div className="status-banner" role="status">{message}</div>}
    {previewUrl && <section className="voice-preview"><audio controls autoPlay src={previewUrl} /><button onClick={() => setPreviewUrl('')}>Close</button></section>}
    <section className="panel preset-editor"><h2>Create a voice</h2><p className="field-note">Record or import several samples of the same speaker. Choose one sample for generation; distinct speakers are not blended.</p><label>Voice name<input value={name} maxLength={80} onChange={(event) => setName(event.target.value)} placeholder="My narrator" /></label><button className="primary" disabled={!name.trim()} onClick={() => void create()}>Create voice</button></section>
    <section className="voice-section"><h2>Your voices</h2><div className="voice-grid">{voices.map((voice) => <article className={selected?.id === voice.id ? 'voice-card selected' : 'voice-card'} key={voice.id}><div><strong>{voice.name}</strong><small>{voice.builtIn ? 'Model voice' : `${voice.samples.length} sample${voice.samples.length === 1 ? '' : 's'}`}</small></div><div className="voice-actions"><button onClick={() => void select(voice)} disabled={!voice.builtIn && !voice.selectedSampleId}>Select</button><button onClick={() => void preview(voice)} disabled={!voice.builtIn && !voice.selectedSampleId}>Preview</button>{!voice.builtIn && <button onClick={() => void remove(voice)} disabled={selected?.id === voice.id}>Delete</button>}</div>{!voice.builtIn && <><label>Sample name<input value={sampleName} onChange={(event) => setSampleName(event.target.value)} /></label><div className="voice-actions"><button onClick={() => void addFile(voice)}>Import sample</button><button disabled={recording && recordingVoiceId !== voice.id} onClick={() => recording ? recorder.current?.stop() : void startRecording(voice.id)}>{recording && recordingVoiceId === voice.id ? 'Stop recording' : 'Record sample'}</button></div>{recording && recordingVoiceId === voice.id && <div className="recording-meter" role="meter" aria-label="Microphone level" aria-valuemin={0} aria-valuemax={100} aria-valuenow={level}><span style={{ width: `${level}%` }} /></div>}{recorded && recordingVoiceId === voice.id && <div className="recording-preview"><audio controls src={recordedUrl} /><button onClick={() => void saveRecording(voice)}>Save recording</button><button onClick={() => { setRecorded(null); setRecordingVoiceId('') }}>Retry</button></div>}{voice.samples.map((sample) => <div className="sample-row" key={sample.id}><span>{sample.name} · {(sample.durationMs / 1000).toFixed(1)}s</span><button onClick={() => void listenSample(voice, sample.id)}>Listen</button><button onClick={() => void productionApi.selectVoiceSample(voice.id, sample.id).then(refresh).catch((cause) => setMessage(errorMessage(cause)))} disabled={voice.selectedSampleId === sample.id}>{voice.selectedSampleId === sample.id ? 'Selected' : 'Use sample'}</button></div>)}{sampleUrl && <audio controls src={sampleUrl} />}</>}</article>)}</div></section>
    <p className="field-note">For expressive speech, use punctuation and sentence breaks. Chatterbox supports tags such as [sigh] and [laugh]. Exact word emphasis and SSML are not supported; preview a sentence and adjust its wording if needed.</p>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={selected?.id ?? ''} onSelect={(voice) => void select(voice)} onClose={() => setPickerOpen(false)} onPreview={(voice) => { setPickerOpen(false); void preview(voice) }} />
  </div>
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
    <Header eyebrow="System" title="Settings" copy="Configure local tools and model providers." action={<><button disabled={!settings} onClick={() => void systemApi.diagnostics().then(setTools).catch((cause) => setMessage(errorMessage(cause)))}>Check status</button><button className="primary" disabled={!settings} onClick={() => void save()}>Save settings</button></>} />
    {message && <div className="status-banner">{message}</div>}
    <div className="settings-grid">
      <section className="panel">
        <h2>Desktop runtime</h2>
        <dl><div><dt>Platform</dt><dd>{desktop.platform}</dd></div><div><dt>Architecture</dt><dd>{desktop.architecture}</dd></div><div><dt>Runtime</dt><dd>{desktop.runtime}</dd></div></dl>
        <h2>Local tools</h2>
        <div className="tool-list">{tools.map((tool) => <div key={tool.name}><span className={tool.available ? 'dot available' : 'dot'} /><strong>{tool.name}</strong><span className="tool-result"><small>{toolStatus(tool.status)}</small>{tool.path && <small title={tool.path}>{tool.path}</small>}</span></div>)}</div>
      </section>
      {settings && <SettingsForm settings={settings} tools={tools} onChange={setSettings} />}
    </div>
  </div>
}

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
    <div className="voice-field"><span>Narration voice</span><button onClick={() => setPickerOpen(true)}>{voices.find((voice) => voice.id === settings.speech.voiceId)?.name ?? 'Choose a voice'}</button></div>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={settings.speech.voiceId} onSelect={(voice) => onChange({ ...settings, speech: { ...settings.speech, voiceId: voice.id } })} onClose={() => setPickerOpen(false)} />
    <h2>Audio tools</h2>
    {pathField('FFmpeg path', settings.ffmpegPath ?? '', (path) => onChange({ ...settings, ffmpegPath: path || null }), 'ffmpeg')}
    {pathField('FFprobe path', settings.ffprobePath ?? '', (path) => onChange({ ...settings, ffprobePath: path || null }), 'ffprobe')}
    <h2>Sound workers</h2>
    <p className="field-note">Start each local worker separately. Sound Studio shows whether its model is ready.</p>
    <label>Chatterbox URL<input value={settings.sounds.chatterboxUrl} onChange={(event) => onChange({ ...settings, sounds: { ...settings.sounds, chatterboxUrl: event.target.value } })} /></label>
    <label>Sound effects URL<input value={settings.sounds.sfxUrl} onChange={(event) => onChange({ ...settings, sounds: { ...settings.sounds, sfxUrl: event.target.value } })} /></label>
    <h2>Text processing</h2>
    <label>Provider<select value={settings.llm.provider} onChange={(event) => onChange({ ...settings, llm: event.target.value === 'ollama' ? { provider: 'ollama', base_url: 'http://127.0.0.1:11434', model: '', context_size: 8192, max_tokens: 2048 } : event.target.value === 'llama_cpp' ? { provider: 'llama_cpp', executable_path: '', model_path: '', context_size: 8192, max_tokens: 2048, gpu_layers: 99 } : { provider: 'none' } })}><option value="none">Disabled</option><option value="llama_cpp">llama.cpp / GGUF</option><option value="ollama">Ollama</option></select></label>
    {ollama && <div><label>Loopback URL<input value={ollama.base_url} onChange={(event) => onChange({ ...settings, llm: { ...ollama, base_url: event.target.value } })} /></label><label>Model<input value={ollama.model} onChange={(event) => onChange({ ...settings, llm: { ...ollama, model: event.target.value } })} /></label>{pathField('Ollama CLI path', settings.ollamaPath ?? '', (path) => onChange({ ...settings, ollamaPath: path || null }), 'ollama')}</div>}
    {llama && <div>{pathField('llama-cli path', llama.executable_path, (path) => onChange({ ...settings, llm: { ...llama, executable_path: path } }), 'llama')}{pathField('GGUF model path', llama.model_path, (path) => onChange({ ...settings, llm: { ...llama, model_path: path } }), undefined, ['gguf'])}</div>}
  </section>
}
