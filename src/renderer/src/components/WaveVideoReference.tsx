import { useEffect, useRef, useState } from 'react'
import type { WaveVideo, WaveView } from '../../../shared/waveStudio'
import { formatWaveTime } from '../../../shared/waveStudio'
import { waveApi } from '../waveStudioNative'
import { errorMessage } from '../native'
export function WaveVideoReference({videos,view,playing,setVideos,setView,onError}:{videos:WaveVideo[];view:WaveView;playing:boolean;setVideos:(v:WaveVideo[])=>void;setView:React.Dispatch<React.SetStateAction<WaveView>>;onError:(m:string)=>void}):JSX.Element {
 const active=videos.find(v=>v.id===view.selectedId)||videos.find(v=>view.playheadMs>=v.startMs&&view.playheadMs<v.startMs+v.durationMs)||videos[0], ref=useRef<HTMLVideoElement>(null),[url,setUrl]=useState('')
 useEffect(()=>{let alive=true;setUrl('');if(active)void waveApi.videoUrl(active.id).then(u=>{if(alive)setUrl(u)}).catch(e=>onError(errorMessage(e)));return()=>{alive=false}},[active?.id])
 function sync():void{const video=ref.current;if(!video||!active)return;video.muted=true;const time=Math.max(0,Math.min(active.durationMs/1000,(view.playheadMs-active.startMs)/1000));if(Math.abs(video.currentTime-time)>.15||!playing)video.currentTime=time;const inside=view.playheadMs>=active.startMs&&view.playheadMs<active.startMs+active.durationMs;if(playing&&inside){if(video.paused)void video.play().catch(()=>{})}else video.pause()}
 useEffect(sync,[view.playheadMs,playing,active?.id,url,active?.startMs])
 if(!active)return <></>
 return <section className="wave-video-reference"><video ref={ref} src={url||undefined} muted playsInline preload="metadata" onLoadedMetadata={sync} onError={()=>onError('This video reference could not be displayed.')} /><div><small>VIDEO REFERENCE · MUTED</small><strong>{active.name}</strong><p>Current frame · {formatWaveTime(Math.max(0,view.playheadMs-active.startMs))}</p><label>Timeline start (seconds)<input key={`${active.id}:${active.startMs}`} type="number" min={0} max={(86400000-active.durationMs)/1000} step={.001} defaultValue={active.startMs/1000} onBlur={e=>{const n=Number(e.target.value)*1000;if(Number.isFinite(n)&&n>=0&&n+active.durationMs<=86400000)setVideos(videos.map(v=>v.id===active.id?{...v,startMs:n}:v))}}/></label><button onClick={()=>setView(v=>({...v,playheadMs:active.startMs,selectedId:active.id}))}>Go to video start</button><button onClick={()=>{ref.current?.pause();setVideos(videos.filter(v=>v.id!==active.id))}}>Remove reference</button></div></section>
}
