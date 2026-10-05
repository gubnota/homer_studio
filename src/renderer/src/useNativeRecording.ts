import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import type { CaptureState, RecordingSession } from '../../shared/audio'
import { audioApi } from './audioNative'
import { errorMessage } from './native'

// Every recording entry point preserves its native capture in Voice Memos.
export function useNativeRecording(name: string, limitMs: number, context: Partial<Pick<RecordingSession, 'contextType' | 'chapterId' | 'segmentId' | 'speakerId' | 'text'>> | null = null) {
 const [memo,setMemo]=useState<RecordingSession|null>(null),[state,setState]=useState<CaptureState|null>(null),[url,setUrl]=useState(''),[path,setPath]=useState(''),[error,setError]=useState('')
 const id=useRef(''), mounted=useRef(true), active=useRef(false), timer=useRef<ReturnType<typeof setTimeout>|null>(null)
 async function finish():Promise<void>{const next=await audioApi.get(id.current);if(!mounted.current)return;setMemo(next);if(next.selectedTakeId){const [u,p]=await Promise.all([audioApi.url(next.id,next.selectedTakeId),audioApi.sourcePath(next)]);if(mounted.current){setUrl(u);setPath(p)}}}
 useEffect(()=>{mounted.current=true;let cleanup:(()=>void)|undefined;void listen<CaptureState>('audio-capture',event=>{if(event.payload.memoId!==id.current||!mounted.current)return;setState(event.payload);if(['stopped','failed'].includes(event.payload.state)){active.current=false;if(timer.current)clearTimeout(timer.current);void finish().catch(e=>{if(mounted.current)setError(errorMessage(e))});if(event.payload.message)setError(event.payload.message)}}).then(fn=>{if(mounted.current)cleanup=fn;else fn()});return()=>{mounted.current=false;cleanup?.();if(timer.current)clearTimeout(timer.current);if(active.current)void audioApi.control('status').then(s=>{if(s.memoId===id.current&&['recording','paused'].includes(s.state))return audioApi.control('stop')}).catch(()=>{})}},[])
 async function start():Promise<void>{if(active.current)return;active.current=true;setError('');setPath('');setUrl('');try{const next=await audioApi.create(name, context);id.current=next.id;const capture=await audioApi.start(next.id,null);if(!mounted.current){await audioApi.control('stop');active.current=false;return}setMemo(next);setState(capture);timer.current=setTimeout(()=>void stop(),limitMs)}catch(e){active.current=false;if(mounted.current)setError(errorMessage(e))}}
 async function stop():Promise<void>{if(timer.current)clearTimeout(timer.current);try{const capture=await audioApi.control('stop');active.current=false;if(mounted.current){setState(capture);await finish()}}catch(e){if(mounted.current)setError(errorMessage(e))}}
 return {memo,state,url,path,error,start,stop,recording:state?.state==='recording'||state?.state==='paused',level:Math.round((state?.rms||0)*100)}
}
