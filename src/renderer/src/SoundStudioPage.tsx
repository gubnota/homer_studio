import { useEffect, useState } from 'react'
import type { JobRecord, Settings, SoundAsset, SoundCategory, SoundRequest, Voice, WorkerHealth } from '../../shared/contracts'
import { errorMessage, isDesktop, productionApi, soundsApi, systemApi } from './native'
import { VoicePicker } from './VoicePicker'
import { KindPicker } from './KindPicker'

const categories: { id: SoundCategory; label: string; hint: string }[] = [
  { id: 'sound_effect', label: 'Sound effect', hint: 'Fabric, footsteps, rain, room tone, or an experimental panting sound.' },
  { id: 'speech', label: 'Speech and vocal gestures', hint: 'Type English words and add supported tags in the same prompt.' }
]
const examplePrompts: Record<SoundCategory, string> = {
  sound_effect: 'Soft cotton fabric rustling close to the microphone',
  speech: 'Hello there. This is a short spoken sample.',
  vocal_gesture: '[sigh]'
}

function engineFor(category: SoundCategory): string { return category === 'sound_effect' ? 'sound_effect' : 'chatterbox_turbo' }

export function isValidSoundRequest(request: SoundRequest): boolean {
  const validLegacyGesture = request.category !== 'vocal_gesture' || ['[clear throat]', '[sigh]', '[shush]', '[cough]', '[groan]', '[sniff]', '[gasp]', '[chuckle]', '[laugh]'].includes(request.prompt.trim().toLowerCase())
  return validLegacyGesture && request.prompt.trim().length > 0 && request.prompt.length <= 500 && (request.negativePrompt?.length ?? 0) <= 300 && request.durationSeconds >= 1 && request.durationSeconds <= 20 && (request.seed === null || (Number.isInteger(request.seed) && request.seed >= 0 && request.seed <= 2147483647))
}

const gestures = ['[clear throat]', '[sigh]', '[shush]', '[cough]', '[groan]', '[sniff]', '[gasp]', '[chuckle]', '[laugh]']

export function SoundStudioPage(): JSX.Element {
  const [category, setCategory] = useState<SoundCategory>('sound_effect')
  const [prompt, setPrompt] = useState(examplePrompts.sound_effect)
  const [duration, setDuration] = useState(5)
  const [seed, setSeed] = useState('')
  const [negativePrompt, setNegativePrompt] = useState('')
  const [voices, setVoices] = useState<Voice[]>([])
  const [settings, setSettings] = useState<Settings | null>(null)
  const [pickerOpen, setPickerOpen] = useState(false)
  const [kindOpen, setKindOpen] = useState(false)
  const [candidateIds, setCandidateIds] = useState<string[]>([])
  const [candidateJobs, setCandidateJobs] = useState<string[]>([])
  const [candidateBaseline, setCandidateBaseline] = useState<string[]>([])
  const [workers, setWorkers] = useState<WorkerHealth[]>([])
  const [assets, setAssets] = useState<SoundAsset[]>([])
  const [activeJob, setActiveJob] = useState('')
  const [job, setJob] = useState<JobRecord | null>(null)
  const [playingId, setPlayingId] = useState('')
  const [audioUrl, setAudioUrl] = useState('')
  const [error, setError] = useState('')
  const [message, setMessage] = useState('')
  const [selectedIds, setSelectedIds] = useState<string[]>([])

  useEffect(() => {
    if (!isDesktop()) return
    const refresh = (): void => {
      void soundsApi.workers().then(setWorkers).catch((cause) => setError(errorMessage(cause)))
      void soundsApi.list().then(setAssets).catch((cause) => setError(errorMessage(cause)))
    }
    refresh()
    void Promise.all([productionApi.voices(), systemApi.settings()]).then(([found, current]) => { setVoices(found); setSettings(current) }).catch((cause) => setError(errorMessage(cause)))
    const timer = window.setInterval(refresh, 3000)
    return () => window.clearInterval(timer)
  }, [])

  useEffect(() => {
    if (!activeJob || job?.status === 'completed' || job?.status === 'failed' || job?.status === 'cancelled') return
    const refresh = (): void => {
      void systemApi.jobs().then((jobs) => {
        const current = jobs.find((item) => item.id === activeJob) ?? null
        setJob(current)
        if (current?.status === 'completed') void soundsApi.list().then(setAssets)
      }).catch((cause) => setError(errorMessage(cause)))
    }
    refresh()
    const timer = window.setInterval(refresh, 750)
    return () => window.clearInterval(timer)
  }, [activeJob, job?.status])

  useEffect(() => {
    if (candidateJobs.length === 0) return
    const refresh = (): void => { void systemApi.jobs().then((jobs) => {
      if (candidateJobs.every((id) => jobs.some((item) => item.id === id && ['completed', 'failed', 'cancelled'].includes(item.status)))) {
        const failures = candidateJobs.map((id) => jobs.find((item) => item.id === id)).filter((item) => item?.status === 'failed')
        if (failures.length) setError(failures.map((item) => item?.message ?? 'Generation failed').join(' · '))
        void soundsApi.list().then((found) => { setAssets(found); setCandidateIds(found.filter((item) => !candidateBaseline.includes(item.id)).map((item) => item.id)) })
        setCandidateJobs([])
      }
    }).catch((cause) => setError(errorMessage(cause))) }
    refresh(); const timer = window.setInterval(refresh, 750); return () => window.clearInterval(timer)
  }, [candidateJobs, candidateBaseline])

  const selectedWorker = category === 'sound_effect' ? workers.find((worker) => worker.categories.includes('sound_effect')) : workers.find((worker) => worker.engine === 'chatterbox_turbo')
  const busy = job?.status === 'queued' || job?.status === 'running' || candidateJobs.length > 0
  useEffect(() => setSelectedIds((ids) => ids.filter((id) => assets.some((asset) => asset.id === id))), [assets])

  async function generate(request: SoundRequest): Promise<void> {
    setError(''); setMessage(''); setJob(null)
    try { setActiveJob(await soundsApi.generate(request)) } catch (cause) { setError(errorMessage(cause)) }
  }

  async function generateCandidates(): Promise<void> {
    setError(''); setCandidateIds([])
    const ids: string[] = []
    try {
      const existing = await soundsApi.list()
      setCandidateBaseline(existing.map((item) => item.id))
      const base = seed === '' ? Math.floor(Math.random() * 2147483644) : Number(seed)
      for (let index = 0; index < 3; index++) ids.push(await soundsApi.generate({ ...request, seed: (base + index) % 2147483647 }))
      setCandidateJobs(ids); setMessage('Three variations queued. Listen and compare them below.')
    } catch (cause) { if (ids.length) setCandidateJobs(ids); setError(errorMessage(cause)) }
  }

  async function selectVoice(voice: Voice): Promise<void> {
    if (!settings) return
    try { setSettings(await systemApi.saveSettings({ ...settings, speech: { ...settings.speech, voiceId: voice.id } })) } catch (cause) { setError(errorMessage(cause)) }
  }

  async function play(asset: SoundAsset): Promise<void> {
    try { setAudioUrl(await soundsApi.audioUrl(asset.id)); setPlayingId(asset.id); setError('') }
    catch (cause) { setError(errorMessage(cause)) }
  }

  async function exportClip(asset: SoundAsset, format: 'wav' | 'm4a'): Promise<void> {
    try { const path = await soundsApi.export(asset, format); if (path) setMessage(`Saved to ${path}`) }
    catch (cause) { setError(errorMessage(cause)) }
  }
  async function exportSelected(format: 'wav' | 'm4a'): Promise<void> {
    try { const paths = await soundsApi.exportMany(selectedIds, format); if (paths.length) setMessage(`Exported ${paths.length} clips to the chosen folder.`) }
    catch (cause) { setError(errorMessage(cause)) }
  }
  async function deleteClips(ids: string[]): Promise<void> {
    if (!ids.length || !window.confirm(`Delete ${ids.length} clip${ids.length === 1 ? '' : 's'} and their audio files?`)) return
    setError('')
    try {
      setAssets(await soundsApi.deleteMany(ids))
      setSelectedIds((current) => current.filter((id) => !ids.includes(id)))
      if (ids.includes(playingId)) { setPlayingId(''); setAudioUrl('') }
      setMessage(`${ids.length} clip${ids.length === 1 ? '' : 's'} deleted.`)
    } catch (cause) { setError(errorMessage(cause)) }
  }
  const selectionControl = (asset: SoundAsset): JSX.Element => <label className="sound-select"><input type="checkbox" checked={selectedIds.includes(asset.id)} onChange={(event) => setSelectedIds((ids) => event.target.checked ? [...new Set([...ids, asset.id])] : ids.filter((id) => id !== asset.id))} /> Select</label>

  const request: SoundRequest = { prompt, category, durationSeconds: duration, seed: seed === '' ? null : Number(seed), negativePrompt: category === 'sound_effect' ? negativePrompt.trim() || null : null }
  const valid = isValidSoundRequest(request) && (seed === '' || /^\d+$/.test(seed))

  return <div className="page sound-page">
    <header className="page-header"><div><p>Independent audio</p><h1>Sound Studio</h1><span>Make a short clip from a prompt. No book or manuscript is needed.</span></div></header>
    {error && <div className="inline-error" role="alert">{error}</div>}
    {message && <div className="status-banner">{message}</div>}
    <div className="sound-grid">
      <section className="panel sound-compose">
        <h2>Create a clip</h2>
        <div className="kind-field"><span>Kind of audio</span><button type="button" onClick={() => setKindOpen(true)}>{categories.find((item) => item.id === category)?.label ?? 'Speech and vocal gestures'} <span aria-hidden="true">⌄</span></button></div>
        <p className="field-note">{categories.find((item) => item.id === category)?.hint}</p>
        <label>{category === 'speech' ? 'Words to speak' : 'Prompt'}<textarea value={prompt} maxLength={500} onChange={(event) => setPrompt(event.target.value)} rows={5} placeholder="Describe or write the audio you want" /></label>
        {category !== 'sound_effect' && <div className="voice-field"><span>Voice · {voices.find((voice) => voice.id === settings?.speech.voiceId)?.name ?? 'Loading'}</span><button onClick={() => setPickerOpen(true)} disabled={!settings}>Choose voice</button></div>}
        {category === 'speech' && <div className="gesture-panel"><p className="field-note">Add a documented Turbo gesture inline with speech. Punctuation and sentence breaks shape phrasing. For a soft exhale or panting, record a performance or experiment with wording; those phrases are not supported tags.</p><div className="gesture-list">{gestures.map((tag) => <button type="button" key={tag} onClick={() => setPrompt((value) => `${value.trimEnd()} ${tag} `)}>{tag}</button>)}</div></div>}
        {category === 'sound_effect' && <><label>Sounds to avoid · optional<input value={negativePrompt} maxLength={300} onChange={(event) => setNegativePrompt(event.target.value)} placeholder="music, voices, hiss" /></label><p className="field-note">Describe the source, movement, and microphone distance. For example: “Dry cotton sleeve brushing a wooden table, close recording, quiet room.” Generate variations and keep the best take.</p></>}
        <div className="sound-controls"><label>Maximum duration · seconds<input type="number" min="1" max="20" value={duration} onChange={(event) => setDuration(Number(event.target.value))} /></label><label>Seed · optional<input type="number" min="0" max="2147483647" value={seed} onChange={(event) => setSeed(event.target.value)} placeholder="Random" /></label></div>
        <div className="sound-action"><span>Engine: {selectedWorker?.engine ?? engineFor(category)}</span><div className="actions">{category === 'sound_effect' && <button disabled={!isDesktop() || !selectedWorker?.ready || !valid || busy} onClick={() => void generateCandidates()}>Generate 3 variations</button>}<button className="primary" disabled={!isDesktop() || !selectedWorker?.ready || !valid || busy} onClick={() => void generate(request)}>{busy ? 'Generating…' : 'Generate clip'}</button></div></div>
        {job && <div className="sound-job"><div className="sound-job-row"><strong>{job.status} · {job.progress}%</strong>{busy && <button onClick={() => void systemApi.controlJob(job.id, 'cancel')}>Cancel</button>}</div>{job.message && <p>{job.message}</p>}</div>}
      </section>
      <aside className="panel sound-workers"><h2>Local models</h2>{workers.filter((worker) => worker.engine !== 'audioldm2').map((worker) => <div key={worker.engine} className="sound-worker"><strong>{worker.engine === 'chatterbox_turbo' ? 'Chatterbox Turbo' : worker.engine === 'stable_audio_open' ? 'Stable Audio Open' : 'Sound effects worker'}</strong><span className={worker.ready ? 'ready' : 'unavailable'}>{worker.ready ? 'Ready' : 'Unavailable'}</span><small>{worker.message}</small></div>)}<p className="field-note">Chatterbox makes English speech and documented gestures. For fabric and ambience prompts, start Stable Audio Open. It requires separate local model access. See workers/README.md for setup. Results vary with the prompt and seed.</p></aside>
    </div>
    {candidateIds.length > 0 && <section className="panel sound-library"><h2>Compare variations</h2><div className="sound-list">{candidateIds.map((id, index) => { const asset = assets.find((item) => item.id === id); return asset && <div className="sound-asset" key={id}>{selectionControl(asset)}<strong>Take {index + 1} · seed {asset.seed}</strong><div className="sound-asset-actions"><button onClick={() => void play(asset)}>Listen</button><button onClick={() => void exportClip(asset, 'wav')}>Save WAV</button><button onClick={() => void exportClip(asset, 'm4a')}>Save M4A</button></div>{playingId === id && audioUrl && <audio controls autoPlay src={audioUrl} />}</div> })}</div></section>}
    <section className="panel sound-library"><div className="library-heading"><h2>Clip library <small>{assets.length}</small></h2><div className="actions"><button onClick={() => setSelectedIds(assets.map((asset) => asset.id))} disabled={!assets.length}>Select all</button><button onClick={() => setSelectedIds([])} disabled={!selectedIds.length}>Deselect all</button><button onClick={() => void exportSelected('wav')} disabled={!selectedIds.length}>Export selected WAV</button><button onClick={() => void exportSelected('m4a')} disabled={!selectedIds.length}>Export selected M4A</button><button onClick={() => void deleteClips(selectedIds)} disabled={!selectedIds.length}>Delete selected</button><button onClick={() => void deleteClips(assets.map((asset) => asset.id))} disabled={!assets.length}>Delete all</button></div></div><p className="field-note">{selectedIds.length} selected · selection also applies to variations above</p>{assets.length === 0 ? <p className="table-empty">Generated clips will appear here and remain available after restarting.</p> : <div className="sound-list">{[...assets].reverse().map((asset) => <div className="sound-asset" key={asset.id}>{selectionControl(asset)}<div><strong>{asset.prompt}</strong><small>{asset.category.replace('_', ' ')} · {asset.provider} · {(asset.durationMs / 1000).toFixed(1)}s · seed {asset.seed ?? 'random'} · {new Date(asset.createdAtMs).toLocaleString()}</small></div><div className="sound-asset-actions"><button onClick={() => void play(asset)}>Listen</button><button onClick={() => { const next = asset.category === 'vocal_gesture' ? 'speech' : asset.category; setCategory(next); setPrompt(asset.prompt); setDuration(asset.requestedDurationSeconds); setSeed(asset.seed?.toString() ?? ''); setNegativePrompt(asset.negativePrompt ?? ''); void generate({ prompt: asset.prompt, category: next, durationSeconds: asset.requestedDurationSeconds, seed: asset.seed, negativePrompt: asset.negativePrompt }) }} disabled={busy}>Retry</button><button onClick={() => void exportClip(asset, 'wav')}>Export WAV</button><button onClick={() => void exportClip(asset, 'm4a')}>Export M4A</button><button onClick={() => void deleteClips([asset.id])}>Delete</button></div>{playingId === asset.id && audioUrl && <audio controls autoPlay src={audioUrl} />}</div>)}</div>}</section>
    <KindPicker open={kindOpen} selected={category} onSelect={(next) => { if (!prompt.trim() || prompt === examplePrompts[category]) setPrompt(examplePrompts[next]); setCategory(next) }} onClose={() => setKindOpen(false)} />
    <VoicePicker open={pickerOpen} voices={voices} selectedId={settings?.speech.voiceId ?? ''} onSelect={(voice) => void selectVoice(voice)} onClose={() => setPickerOpen(false)} />
  </div>
}
