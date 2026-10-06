import {StudioNotice} from './components/StudioToast'
import { useOptionalWaveStudio } from './WaveStudioProvider'
import { AudioTransport } from './components/AudioTransport'
import { Checkbox } from './components/StudioControls'
import { useNativeRecording } from './useNativeRecording'
import { VoiceMemosPage } from './VoiceMemosPage'
import { useEffect, useState } from 'react'
import type { JobRecord, SoundAsset, Voice } from '../../shared/contracts'
import { chooseAudio, errorMessage, hasBackend, productionApi, soundsApi, systemApi } from './native'
import { VoicePicker } from './VoicePicker'

export function VoiceLabPage(): JSX.Element {
 const wave = useOptionalWaveStudio()
  const [voices, setVoices] = useState<Voice[]>([])
  const [voiceId, setVoiceId] = useState('')
  const [pickerOpen, setPickerOpen] = useState(false)
  const [sourcePath, setSourcePath] = useState('')
  const capture = useNativeRecording('Voice Lab delivery',120_000)
  const recording = capture.recording
  const clip = capture.memo?.selectedTakeId
  const [jobId, setJobId] = useState('')
  const [job, setJob] = useState<JobRecord | null>(null)
  const [assets, setAssets] = useState<SoundAsset[]>([])
  const [selected, setSelected] = useState<string[]>([])
  const [playing, setPlaying] = useState('')
  const [audioUrl, setAudioUrl] = useState('')
  const previewUrl = capture.url
  const [error, setError] = useState('')

  useEffect(() => {
    if (!hasBackend()) return
    void Promise.all([productionApi.voices(), soundsApi.list()]).then(([found, library]) => {
      setVoices(found); setVoiceId((id) => id || found[0]?.id || ''); setAssets(library)
    }).catch((cause) => setError(errorMessage(cause)))
  }, [])
  useEffect(() => {
    if (!jobId || ['completed', 'failed', 'cancelled'].includes(job?.status ?? '')) return
    const poll = (): void => { void systemApi.jobs().then((jobs) => {
      const current = jobs.find((item) => item.id === jobId) ?? null
      setJob(current)
      if (current?.status === 'completed') void soundsApi.list().then(setAssets)
      if (current?.status === 'failed') setError(current.message ?? 'Voice conversion failed.')
    }).catch((cause) => setError(errorMessage(cause))) }
    poll(); const interval = window.setInterval(poll, 700); return () => window.clearInterval(interval)
  }, [jobId, job?.status])
  useEffect(() => setSelected((ids) => ids.filter((id) => assets.some((asset) => asset.id === id))), [assets])

  async function start(): Promise<void> { setSourcePath(''); await capture.start() }
  function stop(): void { void capture.stop() }
  async function choose(): Promise<void> { const path = await chooseAudio(); if(path) {setSourcePath(path);setError('')} }
  async function convert(): Promise<void> {
    if (!voiceId || (!clip && !sourcePath)) return
    setError(''); setJob(null)
    try {
      setJobId(await soundsApi.convertVoiceClip(voiceId, sourcePath || capture.path))
    } catch (cause) { setError(errorMessage(cause)) }
  }
  async function play(asset: SoundAsset): Promise<void> {
    try { setAudioUrl(await soundsApi.audioUrl(asset.id)); setPlaying(asset.id) } catch (cause) { setError(errorMessage(cause)) }
  }
  async function remove(ids: string[]): Promise<void> {
    if (!ids.length || !window.confirm(`Delete ${ids.length} converted clip${ids.length === 1 ? '' : 's'}?`)) return
    try { setAssets(await soundsApi.deleteMany(ids)); setSelected([]); if (ids.includes(playing)) setPlaying('') } catch (cause) { setError(errorMessage(cause)) }
  }
  const converted = assets.filter((asset) => asset.category === 'voice_conversion')
  const busy = job?.status === 'queued' || job?.status === 'running'
  return <div className="page voice-lab-page"><header className="page-header"><div><p>Independent audio</p><h1>Voice Lab</h1><span>Record a short delivery or import one, then hear it in your narrator's voice. No book is needed.</span></div></header>
    {(error || capture.error) && <StudioNotice key={error || capture.error} kind="error">{error || capture.error}</StudioNotice>}
    <section className="panel"><h2>Source recording</h2><p className="field-note">Record up to two minutes. Your pauses and emphasis guide Original Chatterbox. Conversion needs a narrator sample and the local Original Chatterbox worker.</p><div className="actions"><button onClick={() => recording ? stop() : void start()} disabled={busy}>{recording ? 'Stop recording' : 'Record my voice'}</button><button onClick={() => void choose()} disabled={busy || recording}>Import audio</button></div>{recording && <p>Recording...</p>}{sourcePath && <p>{sourcePath}</p>}{previewUrl && <AudioTransport url={previewUrl} />}</section>
    <section className="panel"><h2>Narrator</h2><div className="voice-field"><span>{voices.find((voice) => voice.id === voiceId)?.name ?? 'Choose a voice'}</span><button onClick={() => setPickerOpen(true)}>Choose voice</button></div><button className="primary" disabled={!hasBackend() || !voiceId || (!clip && !sourcePath) || busy || recording} onClick={() => void convert()}>Convert to narrator voice</button>{job && <div className="sound-job"><strong>{job.status}  /  {job.progress}%</strong><p>{job.message}</p>{busy && <button onClick={() => void systemApi.controlJob(job.id, 'cancel')}>Cancel</button>}</div>}</section>
    <section className="panel sound-library"><div className="library-heading"><h2>Converted clips <small>{converted.length}</small></h2><div className="actions"><button disabled={!converted.length} onClick={() => setSelected(converted.map((asset) => asset.id))}>Select all</button><button disabled={!selected.length} onClick={() => setSelected([])}>Deselect all</button><button disabled={!selected.length} onClick={() => void soundsApi.exportMany(selected, 'wav')}>Save selected WAV</button><button disabled={!selected.length} onClick={() => void remove(selected)}>Delete selected</button></div></div>{converted.length === 0 ? <p className="table-empty">Converted clips will appear here.</p> : <div className="sound-list">{[...converted].reverse().map((asset) => <div className="sound-asset" key={asset.id}><label className="sound-select"><Checkbox checked={selected.includes(asset.id)} onChange={(event) => setSelected((ids) => event.target.checked ? [...ids, asset.id] : ids.filter((id) => id !== asset.id))} /> Select</label><div><strong>{voices.find((voice) => voice.id === asset.voiceId)?.name ?? 'Narrator voice'}</strong><small>{(asset.durationMs / 1000).toFixed(1)}s  /  {new Date(asset.createdAtMs).toLocaleString()}</small></div><div className="sound-asset-actions"><button onClick={() => void play(asset)}>Listen</button>{wave && <button disabled={wave.busy} onClick={() => void wave.importAudio('sound', asset.id)}>Open in Wave Studio</button>}<button onClick={() => void soundsApi.export(asset, 'wav')}>Save WAV</button><button onClick={() => void soundsApi.export(asset, 'm4a')}>Save M4A</button><button onClick={() => void remove([asset.id])}>Delete</button></div>{playing === asset.id && audioUrl && <AudioTransport url={audioUrl} />}</div>)}</div>}</section>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={voiceId} onSelect={(voice) => setVoiceId(voice.id)} onClose={() => setPickerOpen(false)} />
    <details className="panel"><summary>Native recording, editing & optional voice engines</summary><VoiceMemosPage title="Audio workspace" /></details>
  </div>
}
