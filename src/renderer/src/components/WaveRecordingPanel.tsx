import {StudioModal} from './StudioModal'
import { useEffect, useRef, useState } from 'react'
import type { CaptureDevice } from '../../../shared/audio'
import { useNativeRecording } from '../useNativeRecording'
import { audioApi } from '../audioNative'
import { errorMessage } from '../native'
import { AudioTransport, audioTime } from './AudioTransport'
export function WaveRecordingPanel({projectId,onRecorded,onClose}:{projectId:string;onRecorded:(memoId:string)=>Promise<void>;onClose:()=>void}):JSX.Element {
 const r=useNativeRecording(`Wave recording ${new Date().toLocaleString()}`,0,{projectId,contextType:'wave-project'}), [devices,setDevices]=useState<CaptureDevice[]>([]),[device,setDevice]=useState(''),[error,setError]=useState(''),[busy,setBusy]=useState(false)
 useEffect(()=>{void audioApi.devices().then(setDevices).catch(e=>setError(errorMessage(e)))},[])
 const inserted=useRef(false)
 async function keepRecording():Promise<void>{if(!r.memo||inserted.current)return;const memo=await audioApi.get(r.memo.id);if(memo.selectedTakeId){await onRecorded(memo.id);inserted.current=true}}
 async function run(fn:()=>Promise<void>):Promise<void>{setBusy(true);setError('');try{await fn()}catch(e){setError(errorMessage(e))}finally{setBusy(false)}}
 return <StudioModal title="Record audio" onClose={async()=>{if(r.recording){const ended=await audioApi.control('stop');if(ended.state!=='stopped')throw new Error('Stop and save the recording before closing.')}await keepRecording();onClose()}}><h2>Record in Wave Studio</h2><p>This recording belongs to this Wave project.</p>{(r.error||error)&&<p role="alert">{r.error||error}</p>}<select aria-label="Microphone" disabled={r.recording} value={device} onChange={e=>setDevice(e.target.value)}><option value="">Default microphone</option>{devices.map(d=><option key={d.id} value={d.id}>{d.name}</option>)}</select><p>{r.state?.state||'Ready'}  /  {audioTime(r.state?.elapsedMs||0)}</p><meter aria-label="Microphone level" min={0} max={100} value={r.level}/>{r.state?.clipping&&<p>Input is clipping. Reduce microphone level.</p>}<div className="wave-presets"><button disabled={busy||r.recording} onClick={()=>void run(async()=>{inserted.current=false;await r.start(device||null)})}> Record</button><button disabled={busy||!r.recording} onClick={()=>void run(async()=>{await audioApi.control(r.state?.state==='paused'?'resume':'pause')})}>{r.state?.state==='paused'?'Resume':'Pause'}</button><button disabled={busy||!r.recording} onClick={()=>void run(async()=>{await r.stop();await keepRecording()})}>Stop & save</button><button disabled={busy||!r.recording} onClick={()=>void run(async()=>{await audioApi.control('discard');onClose()})}>Discard</button></div>{r.url&&<><AudioTransport url={r.url}/><button disabled={busy} onClick={()=>void run(async()=>{await keepRecording();onClose()})}>{inserted.current?'Done':'Insert recording'}</button></>}</StudioModal>
}
