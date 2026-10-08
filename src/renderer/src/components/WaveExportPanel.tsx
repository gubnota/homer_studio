import {arrangeVideos, timelineDuration, formatWaveTime, videosOverlap} from '../../../shared/waveStudio'
import {StudioModal} from './StudioModal'
import {useState} from 'react'
import {save} from '../platform'
import {useWaveStudio} from '../WaveStudioProvider'
import {waveApi} from '../waveStudioNative'
import {errorMessage} from '../native'
import {Select} from './StudioControls'

export function WaveExportPanel({onClose,onJob}:{onClose:()=>void;onJob:(id:string)=>void}):JSX.Element {
 const w=useWaveStudio()
 const [videoId,setVideoId]=useState('timeline'),[lengthMode,setLengthMode]=useState<'longest'|'shortest'>('longest'),[format,setFormat]=useState('m4a')
 const [stage,setStage]=useState(''),[error,setError]=useState('')
 const busy=!!stage,project=w.project,overlap=videosOverlap(project?.videos||[])
 const selected=project?.videos?.find(v=>v.id===videoId),audio=Math.max(0,(project?timelineDuration(project.timeline):0)-(selected?.startMs||0))
 const video=selected?.durationMs||Math.max(0,...(project?.videos||[]).map(v=>v.startMs+v.durationMs)),duration=lengthMode==='longest'?Math.max(audio,video):Math.min(audio,video)
 async function exportMix():Promise<void>{
  if(!w.project||busy||overlap)return
  setError('');w.setError('');setStage('Saving project changes…')
  try{
   await w.flush()
   const project=await waveApi.get(w.project.id)
   setStage('Choose where to save your export…')
   const path=await save({title:`Export ${format.toUpperCase()} ${format==='mp4'?'video':'audio'}`,defaultPath:`${project.name}.${format}`,filters:[{name:format==='mp4'?'MP4 video with edited audio':format==='m4a'?'M4A  /  AAC audio':'WAV  /  lossless audio',extensions:[format]}]})
   if(path){
    setStage('Starting export…')
    const output=path.replace(/\.(wav|m4a|aac|mp3|flac|mp4)$/i,'')+`.${format}`
    onJob(await waveApi.export(project,output,format==='mp4'&&videoId!=='timeline'?videoId:null,format==='mp4'&&videoId==='timeline',format==='mp4'?lengthMode:null))
    onClose()
   }
  }catch(e){const message=errorMessage(e);setError(message);w.setError(message)}finally{setStage('')}
 }
 return <StudioModal title="Export mix" onClose={onClose}>
  <header><h2>Export mix</h2></header>
  {overlap&&<div role="alert"><p>These video fragments overlap. Arrange them sequentially before exporting.</p><button disabled={busy} onClick={()=>{w.setVideos(arrangeVideos(project!.videos||[]));setError('');w.setError('')}}>Arrange videos sequentially</button></div>}
  {error&&<p role="alert">{error}</p>}
  <label>Format<Select value={format} disabled={busy} onChange={e=>setFormat(e.target.value)}><option value="m4a">M4A  /  AAC  /  smaller file</option><option value="wav">WAV  /  lossless</option>{!!project?.videos?.length&&<option value="mp4">MP4 video with edited audio</option>}</Select></label>
  {format==='mp4'?<>
   <label>Video<Select value={videoId} disabled={busy} onChange={e=>setVideoId(e.target.value)}><option value="timeline">Entire video timeline</option>{project?.videos?.map(v=><option key={v.id} value={v.id}>{v.name}</option>)}</Select></label>
   <label>Length<Select value={lengthMode} disabled={busy} onChange={e=>setLengthMode(e.target.value as 'longest'|'shortest')}><option value="longest">Keep full length</option><option value="shortest">Trim to shorter track</option></Select></label>
   <p>{lengthMode==='longest'?'Keeps all audio. Holds the final video frame through remaining audio; pads audio with silence when video is longer.':'Stops when the shorter audio or video track ends.'} {videoId==='timeline'&&'Holds frames across video gaps; black before the first video.'}</p>
   <p>Export duration: {formatWaveTime(duration)}</p>
  </>:<p>Exports narration and sound effects with their levels and fades.</p>}
  {busy&&<div role="status">{stage}<progress aria-label="Preparing export"/></div>}
  <button className="primary" disabled={busy||overlap||(format==='mp4'&&duration<=0)} onClick={()=>void exportMix()}>{busy?'Preparing…':`Save ${format.toUpperCase()}`}</button>
 </StudioModal>
}
