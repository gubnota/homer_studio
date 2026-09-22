import { useEffect, useState } from 'react'
import type { JobRecord, Settings, SoundAsset, SoundRequest, Voice, WorkerHealth } from '../../shared/contracts'
import { errorMessage, isDesktop, productionApi, soundsApi, systemApi } from './native'
import { VoicePicker } from './VoicePicker'

export function isValidSoundRequest(request: SoundRequest): boolean {
  return request.category === 'speech' && !request.negativePrompt && request.prompt.trim().length > 0 && request.prompt.length <= 500 && request.durationSeconds >= 1 && request.durationSeconds <= 120 && (request.seed === null || (Number.isInteger(request.seed) && request.seed >= 0 && request.seed <= 2147483647))
}

const gestures = ['[clear throat]', '[sigh]', '[shush]', '[cough]', '[groan]', '[sniff]', '[gasp]', '[chuckle]', '[laugh]']

export function SoundStudioPage(): JSX.Element {
  const [prompt, setPrompt] = useState('Hello there. This is a short spoken sample.')
  const [duration, setDuration] = useState(5)
  const [seed, setSeed] = useState('')
  const [voices, setVoices] = useState<Voice[]>([])
  const [settings, setSettings] = useState<Settings | null>(null)
  const [pickerOpen, setPickerOpen] = useState(false)
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

  const selectedWorker = workers.find((worker) => worker.engine === 'chatterbox_turbo')
  const busy = job?.status === 'queued' || job?.status === 'running'
  useEffect(() => setSelectedIds((ids) => ids.filter((id) => assets.some((asset) => asset.id === id))), [assets])

  async function generate(request: SoundRequest): Promise<void> {
    setError(''); setMessage(''); setJob(null)
    try { setActiveJob(await soundsApi.generate(request)) } catch (cause) { setError(errorMessage(cause)) }
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

  const request: SoundRequest = { prompt, category: 'speech', durationSeconds: duration, seed: seed === '' ? null : Number(seed), negativePrompt: null }
  const valid = isValidSoundRequest(request) && (seed === '' || /^\d+$/.test(seed))

  return <div className="page sound-page">
    <header className="page-header"><div><p>Independent audio</p><h1>Sound Studio</h1><span>Make English speech with supported vocal gestures. No book or manuscript is needed.</span></div></header>
    {error && <div className="inline-error" role="alert">{error}</div>}
    {message && <div className="status-banner">{message}</div>}
    <div className="sound-grid">
      <section className="panel sound-compose">
        <h2>Create a clip</h2>
        <label>Words to speak<textarea value={prompt} maxLength={500} onChange={(event) => setPrompt(event.target.value)} rows={5} placeholder="Write the words to speak" /></label>
        <div className="voice-field"><span>Voice · {voices.find((voice) => voice.id === settings?.speech.voiceId)?.name ?? 'Loading'}</span><button onClick={() => setPickerOpen(true)} disabled={!settings}>Choose voice</button></div>
        <div className="gesture-panel"><p className="field-note">Add a documented Turbo gesture inline with speech. Punctuation and sentence breaks shape phrasing. Longer text is generated in short parts and joined. The maximum duration does not stretch a short sentence.</p><div className="gesture-list">{gestures.map((tag) => <button type="button" key={tag} onClick={() => setPrompt((value) => `${value.trimEnd()} ${tag} `)}>{tag}</button>)}</div></div>
        <div className="sound-controls"><label>Maximum duration · seconds<input type="number" min="1" max="120" value={duration} onChange={(event) => setDuration(Number(event.target.value))} /></label><label>Seed · optional<input type="number" min="0" max="2147483647" value={seed} onChange={(event) => setSeed(event.target.value)} placeholder="Random" /></label></div>
        <div className="sound-action"><span>Engine: Chatterbox Turbo</span><div className="actions"><button className="primary" disabled={!isDesktop() || !selectedWorker?.ready || !valid || busy} onClick={() => void generate(request)}>{busy ? 'Generating…' : 'Generate clip'}</button></div></div>
        {job && <div className="sound-job"><div className="sound-job-row"><strong>{job.status} · {job.progress}%</strong>{busy && <button onClick={() => void systemApi.controlJob(job.id, 'cancel')}>Cancel</button>}</div>{job.message && <p>{job.message}</p>}</div>}
      </section>
      <aside className="panel sound-workers"><h2>Local model</h2>{selectedWorker && <div className="sound-worker"><strong>Chatterbox Turbo</strong><span className={selectedWorker.ready ? 'ready' : 'unavailable'}>{selectedWorker.ready ? 'Ready' : 'Unavailable'}</span><small>{selectedWorker.message}</small></div>}<p className="field-note">Chatterbox makes English speech and documented gestures. Results vary with the prompt and voice.</p></aside>
    </div>
    <section className="panel sound-library"><div className="library-heading"><h2>Clip library <small>{assets.length}</small></h2><div className="actions"><button onClick={() => setSelectedIds(assets.map((asset) => asset.id))} disabled={!assets.length}>Select all</button><button onClick={() => setSelectedIds([])} disabled={!selectedIds.length}>Deselect all</button><button onClick={() => void exportSelected('wav')} disabled={!selectedIds.length}>Export selected WAV</button><button onClick={() => void exportSelected('m4a')} disabled={!selectedIds.length}>Export selected M4A</button><button onClick={() => void deleteClips(selectedIds)} disabled={!selectedIds.length}>Delete selected</button><button onClick={() => void deleteClips(assets.map((asset) => asset.id))} disabled={!assets.length}>Delete all</button></div></div><p className="field-note">{selectedIds.length} selected</p>{assets.length === 0 ? <p className="table-empty">Generated clips will appear here and remain available after restarting.</p> : <div className="sound-list">{[...assets].reverse().map((asset) => <div className="sound-asset" key={asset.id}>{selectionControl(asset)}<div><strong>{asset.prompt}</strong><small>{asset.category.replace('_', ' ')} · {asset.provider} · {(asset.durationMs / 1000).toFixed(1)}s · seed {asset.seed ?? 'random'} · {new Date(asset.createdAtMs).toLocaleString()}</small></div><div className="sound-asset-actions"><button onClick={() => void play(asset)}>Listen</button>{asset.category === 'speech' && <button onClick={() => { setPrompt(asset.prompt); setDuration(asset.requestedDurationSeconds); setSeed(asset.seed?.toString() ?? ''); void generate({ prompt: asset.prompt, category: 'speech', durationSeconds: asset.requestedDurationSeconds, seed: asset.seed, negativePrompt: null }) }} disabled={busy}>Retry</button>}<button onClick={() => void exportClip(asset, 'wav')}>Export WAV</button><button onClick={() => void exportClip(asset, 'm4a')}>Export M4A</button><button onClick={() => void deleteClips([asset.id])}>Delete</button></div>{playingId === asset.id && audioUrl && <audio controls autoPlay src={audioUrl} />}</div>)}</div>}</section>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={settings?.speech.voiceId ?? ''} onSelect={(voice) => void selectVoice(voice)} onClose={() => setPickerOpen(false)} />
  </div>
}
