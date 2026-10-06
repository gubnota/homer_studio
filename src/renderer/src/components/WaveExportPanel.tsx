import {StudioModal} from './StudioModal'
import { useState } from 'react'
import { save } from '../platform'
import { useWaveStudio } from '../WaveStudioProvider'
import { waveApi } from '../waveStudioNative'
import { errorMessage } from '../native'
import { Select } from './StudioControls'
export function WaveExportPanel({onClose,onJob}:{onClose:()=>void;onJob:(id:string)=>void}):JSX.Element {
 const w=useWaveStudio(),[format,setFormat]=useState('m4a'),[busy,setBusy]=useState(false)
 async function exportMix():Promise<void>{if(!w.project)return;setBusy(true);try{await w.flush();const project=await waveApi.get(w.project.id);const path=await save({title:`Export ${format.toUpperCase()} audio`,defaultPath:`${project.name}.${format}`,filters:[{name:format==='m4a'?'M4A  /  AAC audio': 'WAV  /  lossless audio',extensions:[format]}]});if(path){const output=path.replace(/\.(wav|m4a|aac|mp3|flac)$/i,'')+`.${format}`;onJob(await waveApi.export(project,output));onClose()}}catch(e){w.setError(errorMessage(e))}finally{setBusy(false)}}
 return <StudioModal title="Export audio" onClose={onClose}><header><h2>Export audio</h2></header><label>Format<Select value={format} disabled={busy} onChange={e=>setFormat(e.target.value)}><option value="m4a">M4A  /  AAC  /  smaller file</option><option value="wav">WAV  /  lossless</option></Select></label><p>Exports narration and sound effects with their levels and fades. Video references stay muted and are excluded.</p><button className="primary" disabled={busy} onClick={()=>void exportMix()}>{busy?'Preparing...':`Save ${format.toUpperCase()}`}</button></StudioModal>
}
