import { useEffect, useState } from 'react'
import type { JobRecord, SoundAsset, SoundCategory, SoundRequest, WorkerHealth } from '../../shared/contracts'
import { errorMessage, isDesktop, soundsApi, systemApi } from './native'

const categories: { id: SoundCategory; label: string; hint: string }[] = [
  { id: 'sound_effect', label: 'Sound effect', hint: 'Fabric, footsteps, rain, room tone, or an experimental panting sound.' },
  { id: 'speech', label: 'Speech', hint: 'Type the exact English words you want spoken.' },
  { id: 'vocal_gesture', label: 'Vocal gesture', hint: 'Use one tag: [sigh], [gasp], [cough], [laugh], [chuckle], or [groan].' }
]
const examplePrompts: Record<SoundCategory, string> = {
  sound_effect: 'Soft cotton fabric rustling close to the microphone',
  speech: 'Hello there. This is a short spoken sample.',
  vocal_gesture: '[sigh]'
}

function engineFor(category: SoundCategory): string { return category === 'sound_effect' ? 'sound_effect' : 'chatterbox_turbo' }

export function isValidSoundRequest(request: SoundRequest): boolean {
  const validGesture = request.category !== 'vocal_gesture' || ['[sigh]', '[gasp]', '[cough]', '[laugh]', '[chuckle]', '[groan]'].includes(request.prompt.trim().toLowerCase())
  return validGesture && request.prompt.trim().length > 0 && request.prompt.length <= 500 && request.durationSeconds >= 1 && request.durationSeconds <= 20 && (request.seed === null || (Number.isInteger(request.seed) && request.seed >= 0 && request.seed <= 2147483647))
}

export function SoundStudioPage(): JSX.Element {
  const [category, setCategory] = useState<SoundCategory>('sound_effect')
  const [prompt, setPrompt] = useState(examplePrompts.sound_effect)
  const [duration, setDuration] = useState(5)
  const [seed, setSeed] = useState('')
  const [workers, setWorkers] = useState<WorkerHealth[]>([])
  const [assets, setAssets] = useState<SoundAsset[]>([])
  const [activeJob, setActiveJob] = useState('')
  const [job, setJob] = useState<JobRecord | null>(null)
  const [playingId, setPlayingId] = useState('')
  const [audioUrl, setAudioUrl] = useState('')
  const [error, setError] = useState('')
  const [message, setMessage] = useState('')

  useEffect(() => {
    if (!isDesktop()) return
    const refresh = (): void => {
      void soundsApi.workers().then(setWorkers).catch((cause) => setError(errorMessage(cause)))
      void soundsApi.list().then(setAssets).catch((cause) => setError(errorMessage(cause)))
    }
    refresh()
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

  const selectedWorker = category === 'sound_effect' ? workers.find((worker) => worker.categories.includes('sound_effect')) : workers.find((worker) => worker.engine === 'chatterbox_turbo')
  const busy = job?.status === 'queued' || job?.status === 'running'

  async function generate(request: SoundRequest): Promise<void> {
    setError(''); setMessage(''); setJob(null)
    try { setActiveJob(await soundsApi.generate(request)) } catch (cause) { setError(errorMessage(cause)) }
  }

  async function play(asset: SoundAsset): Promise<void> {
    try { setAudioUrl(await soundsApi.audioUrl(asset.id)); setPlayingId(asset.id); setError('') }
    catch (cause) { setError(errorMessage(cause)) }
  }

  async function exportClip(asset: SoundAsset, format: 'wav' | 'm4a'): Promise<void> {
    try { const path = await soundsApi.export(asset, format); if (path) setMessage(`Saved to ${path}`) }
    catch (cause) { setError(errorMessage(cause)) }
  }

  const request: SoundRequest = { prompt, category, durationSeconds: duration, seed: seed === '' ? null : Number(seed) }
  const valid = isValidSoundRequest(request) && (seed === '' || /^\d+$/.test(seed))

  return <div className="page sound-page">
    <header className="page-header"><div><p>Independent audio</p><h1>Sound Studio</h1><span>Make a short clip from a prompt. No book or manuscript is needed.</span></div></header>
    {error && <div className="inline-error" role="alert">{error}</div>}
    {message && <div className="status-banner">{message}</div>}
    <div className="sound-grid">
      <section className="panel sound-compose">
        <h2>Create a clip</h2>
        <label>Kind of audio<select value={category} onChange={(event) => { const next = event.target.value as SoundCategory; if (!prompt.trim() || prompt === examplePrompts[category]) setPrompt(examplePrompts[next]); setCategory(next) }}>{categories.map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}</select></label>
        <p className="field-note">{categories.find((item) => item.id === category)?.hint}</p>
        <label>{category === 'speech' ? 'Words to speak' : 'Prompt'}<textarea value={prompt} maxLength={500} onChange={(event) => setPrompt(event.target.value)} rows={5} placeholder={category === 'vocal_gesture' ? '[sigh]' : 'Describe the sound you want'} /></label>
        <div className="sound-controls"><label>Maximum duration · seconds<input type="number" min="1" max="20" value={duration} onChange={(event) => setDuration(Number(event.target.value))} /></label><label>Seed · optional<input type="number" min="0" max="2147483647" value={seed} onChange={(event) => setSeed(event.target.value)} placeholder="Random" /></label></div>
        <div className="sound-action"><span>Engine: {selectedWorker?.engine ?? engineFor(category)}</span><button className="primary" disabled={!isDesktop() || !selectedWorker?.ready || !valid || busy} onClick={() => void generate(request)}>{busy ? 'Generating…' : 'Generate clip'}</button></div>
        {job && <div className="sound-job"><strong>{job.status} · {job.progress}%</strong>{job.message && <p>{job.message}</p>}{busy && <button onClick={() => void systemApi.controlJob(job.id, 'cancel')}>Cancel</button>}</div>}
      </section>
      <aside className="panel sound-workers"><h2>Local models</h2>{workers.map((worker) => <div key={worker.engine} className="sound-worker"><strong>{worker.engine === 'chatterbox_turbo' ? 'Chatterbox Turbo' : worker.engine === 'audioldm2' ? 'AudioLDM 2' : worker.engine === 'stable_audio_open' ? 'Stable Audio Open' : 'Sound effects worker'}</strong><span className={worker.ready ? 'ready' : 'unavailable'}>{worker.ready ? 'Ready' : 'Unavailable'}</span><small>{worker.message}</small></div>)}<p className="field-note">Chatterbox makes exact speech and listed gestures. For fabric, ambience, or panting prompts, start the sound effects worker. AudioLDM 2 is a public noncommercial checkpoint; Stable Audio Open requires separate access. Both need local model files and Python packages. See workers/README.md for setup. Results vary with the prompt and seed.</p></aside>
    </div>
    <section className="panel sound-library"><h2>Clip library <small>{assets.length}</small></h2>{assets.length === 0 ? <p className="table-empty">Generated clips will appear here and remain available after restarting.</p> : <div className="sound-list">{[...assets].reverse().map((asset) => <div className="sound-asset" key={asset.id}><div><strong>{asset.prompt}</strong><small>{asset.category.replace('_', ' ')} · {asset.provider} · {(asset.durationMs / 1000).toFixed(1)}s · {new Date(asset.createdAtMs).toLocaleString()}</small></div><div className="sound-asset-actions"><button onClick={() => void play(asset)}>Listen</button><button onClick={() => { setCategory(asset.category); setPrompt(asset.prompt); setDuration(asset.requestedDurationSeconds); setSeed(asset.seed?.toString() ?? ''); void generate({ prompt: asset.prompt, category: asset.category, durationSeconds: asset.requestedDurationSeconds, seed: asset.seed }) }} disabled={busy}>Retry</button><button onClick={() => void exportClip(asset, 'wav')}>Export WAV</button><button onClick={() => void exportClip(asset, 'm4a')}>Export M4A</button></div>{playingId === asset.id && audioUrl && <audio controls autoPlay src={audioUrl} />}</div>)}</div>}</section>
  </div>
}
