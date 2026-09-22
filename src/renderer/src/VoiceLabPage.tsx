import { useEffect, useRef, useState } from 'react'
import type { JobRecord, SoundAsset, Voice } from '../../shared/contracts'
import { chooseAudio, errorMessage, isDesktop, productionApi, soundsApi, systemApi } from './native'
import { VoicePicker } from './VoicePicker'

export function VoiceLabPage(): JSX.Element {
  const [voices, setVoices] = useState<Voice[]>([])
  const [voiceId, setVoiceId] = useState('')
  const [pickerOpen, setPickerOpen] = useState(false)
  const [sourcePath, setSourcePath] = useState('')
  const [clip, setClip] = useState<Blob | null>(null)
  const [recording, setRecording] = useState(false)
  const [jobId, setJobId] = useState('')
  const [job, setJob] = useState<JobRecord | null>(null)
  const [assets, setAssets] = useState<SoundAsset[]>([])
  const [selected, setSelected] = useState<string[]>([])
  const [playing, setPlaying] = useState('')
  const [audioUrl, setAudioUrl] = useState('')
  const [previewUrl, setPreviewUrl] = useState('')
  const [error, setError] = useState('')
  const recorder = useRef<MediaRecorder | null>(null)
  const stream = useRef<MediaStream | null>(null)
  const chunks = useRef<Blob[]>([])
  const timer = useRef<number | null>(null)

  useEffect(() => {
    if (!isDesktop()) return
    void Promise.all([productionApi.voices(), soundsApi.list()]).then(([found, library]) => {
      setVoices(found); setVoiceId((id) => id || found[0]?.id || ''); setAssets(library)
    }).catch((cause) => setError(errorMessage(cause)))
  }, [])
  useEffect(() => () => {
    if (timer.current !== null) window.clearTimeout(timer.current)
    if (recorder.current?.state === 'recording') recorder.current.stop()
    stream.current?.getTracks().forEach((track) => track.stop())
  }, [])
  useEffect(() => {
    if (!clip) { setPreviewUrl(''); return }
    const url = URL.createObjectURL(clip)
    setPreviewUrl(url)
    return () => URL.revokeObjectURL(url)
  }, [clip])
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

  async function start(): Promise<void> {
    setError(''); setClip(null); setSourcePath('')
    try {
      const mic = await navigator.mediaDevices.getUserMedia({ audio: true })
      stream.current = mic
      const mimeType = ['audio/mp4', 'audio/webm;codecs=opus', 'audio/webm'].find((value) => MediaRecorder.isTypeSupported(value))
      const capture = new MediaRecorder(mic, mimeType ? { mimeType } : undefined)
      chunks.current = []
      capture.ondataavailable = (event) => { if (event.data.size) chunks.current.push(event.data) }
      capture.onstop = () => { setRecording(false); mic.getTracks().forEach((track) => track.stop()); stream.current = null; setClip(new Blob(chunks.current, { type: capture.mimeType })) }
      recorder.current = capture; capture.start(); setRecording(true)
      timer.current = window.setTimeout(stop, 120_000)
    } catch (cause) { setError(errorMessage(cause)) }
  }
  function stop(): void {
    if (timer.current !== null) window.clearTimeout(timer.current)
    timer.current = null
    if (recorder.current?.state === 'recording') recorder.current.stop()
  }
  async function choose(): Promise<void> { const path = await chooseAudio(); if (path) { setSourcePath(path); setClip(null); setError('') } }
  async function convert(): Promise<void> {
    if (!voiceId || (!clip && !sourcePath)) return
    setError(''); setJob(null)
    try {
      const bytes = clip ? Array.from(new Uint8Array(await clip.arrayBuffer())) : undefined
      setJobId(await soundsApi.convertVoiceClip(voiceId, sourcePath || undefined, bytes))
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
    {error && <div className="inline-error" role="alert">{error}</div>}
    <section className="panel"><h2>Source recording</h2><p className="field-note">Record up to two minutes. Your pauses and emphasis guide Original Chatterbox. Conversion needs a narrator sample and the local Original Chatterbox worker.</p><div className="actions"><button onClick={() => recording ? stop() : void start()} disabled={busy}>{recording ? 'Stop recording' : 'Record my voice'}</button><button onClick={() => void choose()} disabled={busy || recording}>Import audio</button></div>{recording && <p>Recording…</p>}{sourcePath && <p>{sourcePath}</p>}{previewUrl && <audio controls src={previewUrl} />}</section>
    <section className="panel"><h2>Narrator</h2><div className="voice-field"><span>{voices.find((voice) => voice.id === voiceId)?.name ?? 'Choose a voice'}</span><button onClick={() => setPickerOpen(true)}>Choose voice</button></div><button className="primary" disabled={!isDesktop() || !voiceId || (!clip && !sourcePath) || busy || recording} onClick={() => void convert()}>Convert to narrator voice</button>{job && <div className="sound-job"><strong>{job.status} · {job.progress}%</strong><p>{job.message}</p>{busy && <button onClick={() => void systemApi.controlJob(job.id, 'cancel')}>Cancel</button>}</div>}</section>
    <section className="panel sound-library"><div className="library-heading"><h2>Converted clips <small>{converted.length}</small></h2><div className="actions"><button disabled={!converted.length} onClick={() => setSelected(converted.map((asset) => asset.id))}>Select all</button><button disabled={!selected.length} onClick={() => setSelected([])}>Deselect all</button><button disabled={!selected.length} onClick={() => void soundsApi.exportMany(selected, 'wav')}>Save selected WAV</button><button disabled={!selected.length} onClick={() => void remove(selected)}>Delete selected</button></div></div>{converted.length === 0 ? <p className="table-empty">Converted clips will appear here.</p> : <div className="sound-list">{[...converted].reverse().map((asset) => <div className="sound-asset" key={asset.id}><label className="sound-select"><input type="checkbox" checked={selected.includes(asset.id)} onChange={(event) => setSelected((ids) => event.target.checked ? [...ids, asset.id] : ids.filter((id) => id !== asset.id))} /> Select</label><div><strong>{voices.find((voice) => voice.id === asset.voiceId)?.name ?? 'Narrator voice'}</strong><small>{(asset.durationMs / 1000).toFixed(1)}s · {new Date(asset.createdAtMs).toLocaleString()}</small></div><div className="sound-asset-actions"><button onClick={() => void play(asset)}>Listen</button><button onClick={() => void soundsApi.export(asset, 'wav')}>Save WAV</button><button onClick={() => void soundsApi.export(asset, 'm4a')}>Save M4A</button><button onClick={() => void remove([asset.id])}>Delete</button></div>{playing === asset.id && audioUrl && <audio controls autoPlay src={audioUrl} />}</div>)}</div>}</section>
    <VoicePicker open={pickerOpen} voices={voices} selectedId={voiceId} onSelect={(voice) => setVoiceId(voice.id)} onClose={() => setPickerOpen(false)} />
  </div>
}
