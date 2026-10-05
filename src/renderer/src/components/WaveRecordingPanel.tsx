import { useEffect, useState } from 'react'
import type { CaptureDevice } from '../../../shared/audio'
import { useNativeRecording } from '../useNativeRecording'
import { audioApi } from '../audioNative'
import { errorMessage } from '../native'
import { AudioTransport, audioTime } from './AudioTransport'
export function WaveRecordingPanel({onRecorded,onClose}:{onRecorded:(memoId:string)=>Promise<void>;onClose:()=>void}):JSX.Element {
 const r=useNativeRecording(`Wave recording ${new Date().toLocaleString()}`,0), [devices,setDevices]=useState<CaptureDevice[]>([]),[device,setDevice]=useState(''),[error,setError]=useState(''),[busy,setBusy]=useState(false)
 useEffect(()=>{void audioApi.devices().then(setDevices).catch(e=>setError(errorMessage(e)))},[])
 async function run(fn:()=>Promise<void>):Promise<void>{setBusy(true);setError('');try{await fn()}catch(e){setError(errorMessage(e))}finally{setBusy(false)}}
 return <section className="wave-dialog" role="dialog" aria-modal="true" aria-label="Record audio"><h2>Record in Wave Studio</h2><p>Your recording is also saved in Voice Memos.</p>{(r.error||error)&&<p role="alert">{r.error||error}</p>}<select aria-label="Microphone" disabled={r.recording} value={device} onChange={e=>setDevice(e.target.value)}><option value="">Default microphone</option>{devices.map(d=><option key={d.id} value={d.id}>{d.name}</option>)}</select><p>{r.state?.state||'Ready'} · {audioTime(r.state?.elapsedMs||0)}</p><meter aria-label="Microphone level" min={0} max={100} value={r.level}/>{r.state?.clipping&&<p>Input is clipping. Reduce microphone level.</p>}<div className="wave-presets"><button disabled={busy||r.recording} onClick={()=>void run(()=>r.start(device||null))}>● Record</button><button disabled={busy||!r.recording} onClick={()=>void run(async()=>{await audioApi.control(r.state?.state==='paused'?'resume':'pause')})}>{r.state?.state==='paused'?'Resume':'Pause'}</button><button disabled={busy||!r.recording} onClick={()=>void run(r.stop)}>Stop & save</button><button disabled={busy||!r.recording} onClick={()=>void run(async()=>{await audioApi.control('discard');onClose()})}>Discard</button></div>{r.url&&<><AudioTransport url={r.url}/><button disabled={busy} onClick={()=>void run(async()=>{await onRecorded(r.memo!.id);onClose()})}>Insert recording</button></>}<button disabled={busy||r.recording} onClick={onClose}>Close</button></section>
}
