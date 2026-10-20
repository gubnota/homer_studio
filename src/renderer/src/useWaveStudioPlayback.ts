import { useEffect, useRef, useState } from 'react'
import { projectDuration, type WaveProject } from '../../shared/waveStudio'
import { errorMessage } from './native'
import { waveApi } from './waveStudioNative'
import { decodePreview } from './wavePreviewAudio'
interface Run { nodes: AudioBufferSourceNode[]; origin: number; start: number; end: number; until:number }
export function useWaveStudioPlayback(project: WaveProject | null, onTime: (ms: number) => void, onError: (message: string) => void) {
 const [playing,setPlaying]=useState(false),[loading,setLoading]=useState(false)
 const context=useRef<AudioContext|null>(null),run=useRef<Run|null>(null),token=useRef(0),pending=useRef(new Set<string>()),cache=useRef(new Map<string,AudioBuffer>())
 const callbacks=useRef({onTime,onError});callbacks.current={onTime,onError}
 const position=(r:Run)=>Math.max(r.start,Math.min(r.until,r.start+((context.current?.currentTime??r.origin)-r.origin)*1000))
 function stop(update=true):void {
  token.current++
  for(const id of pending.current)void waveApi.cancelPreview(id).catch(()=>{});pending.current.clear()
  const r=run.current;if(r){if(update)callbacks.current.onTime(position(r));for(const n of r.nodes){try{n.stop();n.disconnect()}catch{}}}
  run.current=null;setPlaying(false);setLoading(false)
 }
 async function play(startMs:number,endMs=project?projectDuration(project):0,loop=false):Promise<void>{
  stop(false);if(!project||endMs<=startMs)return
  const mine=++token.current,snapshot=project;setLoading(true)
  try {
   // Establish ownership BEFORE resume: Stop during resume must not restart stale playback.
   const audio=context.current??=new AudioContext({sampleRate:48000});await audio.resume()
   if(mine!==token.current)return
   if(audio.state!=='running')throw new Error('Audio is paused by the system. Click Play again or check your audio output device.')
   document.querySelectorAll('audio').forEach(a=>a.pause())
   const chunk=async(start:number,end:number)=>{
    const key=`${snapshot.id}:${JSON.stringify(snapshot.timeline)}:${start}:${end}`,existing=cache.current.get(key);if(existing)return existing
    const id=crypto.randomUUID();pending.current.add(id);let timer:number|undefined
    try {
     const data=await Promise.race([waveApi.preview(snapshot,start,end,id),new Promise<never>((_,reject)=>{timer=window.setTimeout(()=>{void waveApi.cancelPreview(id).catch(()=>{});reject(new Error('Audio preparation timed out. Retry playback or check that the project audio is available.'))},45000)})])
     if(mine!==token.current)throw new Error('superseded')
     const buffer=decodePreview(audio,data,end-start);cache.current.set(key,buffer);while(cache.current.size>4)cache.current.delete(cache.current.keys().next().value!)
     return buffer
    }finally{window.clearTimeout(timer);pending.current.delete(id)}
   }
   const firstEnd=Math.min(endMs,startMs+10000),secondEnd=Math.min(endMs,firstEnd+10000)
   const first=await chunk(startMs,firstEnd),second=firstEnd<endMs?await chunk(firstEnd,secondEnd):null
   if(mine!==token.current)return
   const r:Run={nodes:[],origin:audio.currentTime+.1,start:startMs,end:endMs,until:startMs};run.current=r
   const schedule=(buffer:AudioBuffer,start:number,end:number)=>{
    let when=r.origin+(start-startMs)/1000
    // Render fell behind: freeze the playhead at the last audible boundary, then resume.
    if(when<audio.currentTime+.02){r.origin=audio.currentTime+.05-(start-startMs)/1000;when=r.origin+(start-startMs)/1000}
    const node=audio.createBufferSource();node.buffer=buffer;node.connect(audio.destination);node.start(when);r.nodes.push(node);r.until=end
    node.onended=()=>{node.disconnect();r.nodes=r.nodes.filter(n=>n!==node)}
   }
   schedule(first,startMs,firstEnd);if(second)schedule(second,firstEnd,secondEnd)
   setPlaying(true);setLoading(false)
   const tick=()=>{
    if(mine!==token.current||run.current!==r)return
    if(audio.state!=='running'){stop();callbacks.current.onError('Audio output was suspended. Check your output device and click Play again.');return}
    const ms=position(r);callbacks.current.onTime(ms);setLoading(ms>=r.until&&r.until<endMs)
    if(ms>=endMs){stop(false);if(loop)void play(startMs,endMs,true)}else window.setTimeout(tick,40)
   };tick()
   let next=second?secondEnd:firstEnd
   while(next<endMs&&mine===token.current){
    while(next-position(r)>10000&&mine===token.current)await new Promise(resolve=>window.setTimeout(resolve,100))
    if(mine!==token.current)return
    const end=Math.min(endMs,next+10000),buffer=await chunk(next,end)
    if(mine!==token.current)return
    schedule(buffer,next,end);next=end
   }
  }catch(cause){if(mine===token.current){stop(false);callbacks.current.onError(errorMessage(cause))}}
 }
 useEffect(()=>{stop();cache.current.clear()},[project?.id,project?.timeline,project?.sources])
 useEffect(()=>{const preview=(event:Event)=>{if(event.target instanceof HTMLAudioElement)stop()};document.addEventListener('play',preview,true);return()=>{document.removeEventListener('play',preview,true);stop();void context.current?.close()}},[])
 return {playing,loading,play,stop}
}
