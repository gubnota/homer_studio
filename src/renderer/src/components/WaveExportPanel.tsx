import {StudioModal} from './StudioModal'
import { useState } from 'react'
import { save } from '../platform'
import { useWaveStudio } from '../WaveStudioProvider'
import { waveApi } from '../waveStudioNative'
import { errorMessage } from '../native'
import { Select } from './StudioControls'
export function WaveExportPanel({onClose,onJob}:{onClose:()=>void;onJob:(id:string)=>void}):JSX.Element {
 const w=useWaveStudio(),[videoId,setVideoId]=useState(w.project?.videos?.[0]?.id||''),[format,setFormat]=useState('m4a'),[busy,setBusy]=useState(false)
 async function exportMix():Promise<void>{if(!w.project)return;setBusy(true);try{await w.flush();const project=await waveApi.get(w.project.id);const path=await save({title:`Export ${format.toUpperCase()} ${format==='mp4'?'video':'audio'}`,defaultPath:`${project.name}.${format}`,filters:[{name:format==='mp4'?'MP4 video with edited audio':format==='m4a'?'M4A  /  AAC audio': 'WAV  /  lossless audio',extensions:[format]}]});if(path){const output=path.replace(/\.(wav|m4a|aac|mp3|flac|mp4)$/i,'')+`.${format}`;onJob(await waveApi.export(project,output,format==='mp4'?videoId:null));onClose()}}catch(e){w.setError(errorMessage(e))}finally{setBusy(false)}}
 return <StudioModal title="Export mix" onClose={onClose}><header><h2>Export mix</h2></header><label>Format<Select value={format} disabled={busy} onChange={e=>setFormat(e.target.value)}><option value="m4a">M4A  /  AAC  /  smaller file</option><option value="wav">WAV  /  lossless</option>{!!w.project?.videos?.length&&<option value="mp4">MP4 video with edited audio</option>}</Select></label>{format==='mp4'?<><label>Video<Select value={videoId} onChange={e=>setVideoId(e.target.value)}>{w.project?.videos?.map(v=><option key={v.id} value={v.id}>{v.name}</option>)}</Select></label><p>Uses this video's duration and your edited audio at its timeline position. Gaps are silent; audio beyond the video end is trimmed.</p></>:<p>Exports narration and sound effects with their levels and fades.</p>}<button className="primary" disabled={busy} onClick={()=>void exportMix()}>{busy?'Preparing...':`Save ${format.toUpperCase()}`}</button></StudioModal>
}
