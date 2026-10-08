import { isDesktop, invoke, uploadFile, open as chooseFile } from './platform'
import {getCurrentWebview} from '@tauri-apps/api/webview'
import {StudioModal} from './components/StudioModal'
import {notify} from './components/StudioToast'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { createContext, useContext, useEffect, useReducer, useRef, useState, type PropsWithChildren } from 'react'
import { placeVideo, videosOverlap, blankTimeline, reconcileVoiceProduction, sourceClip, timelineDuration, type WaveProject, type WaveSource, type WaveTimeline, type WaveView, type WaveVideo } from '../../shared/waveStudio'
import { recoverVoiceVersions, historyEdit, historyRedo, historyUndo, insertMain, type TimelineHistory } from '../../shared/waveStudioEdits'
import { errorMessage, hasBackend } from './native'
import { waveApi } from './waveStudioNative'
interface State { project: WaveProject | null; history: TimelineHistory; change: number }
type Action = {kind:'clear'} | {kind:'videos';videos:WaveVideo[]} | { kind: 'load'; project: WaveProject } | { kind: 'edit'; timeline: WaveTimeline; sources?: WaveSource[] } | { kind: 'undo' | 'redo' } | { kind: 'name'; name: string } | { kind: 'saved'; id: string; revision: number } | { kind: 'original'; timeline: WaveTimeline | null }
const initial: State = { project: null, history: { past: [], present: blankTimeline(), future: [] }, change: 0 }
function reducer(s: State, a: Action): State {
 if (a.kind === 'clear') return initial
 if (a.kind === 'load') return { project: a.project, history: { past: [], present: a.project.timeline, future: [] }, change: 0 }
 if (!s.project) return s
 if (a.kind === 'videos') return {...s,project:{...s.project,videos:a.videos},change:s.change+1}
 if (a.kind === 'saved' && a.id !== s.project.id) return s
 if (a.kind === 'saved') return { ...s, project: { ...s.project, revision: a.revision } }
 if (a.kind === 'original') return { ...s, project: { ...s.project, voiceOriginal: a.timeline }, change: s.change + 1 }
 if (a.kind === 'name') return { ...s, project: { ...s.project, name: a.name }, change: s.change + 1 }
 const h = a.kind === 'edit' ? historyEdit(s.history, a.timeline) : a.kind === 'undo' ? historyUndo(s.history) : historyRedo(s.history)
 if (h === s.history && !(a.kind === 'edit' && a.sources)) return s
 return { project: { ...s.project, timeline: h.present, sources: a.kind === 'edit' && a.sources ? a.sources : s.project.sources }, history: h, change: s.change + 1 }
}
const defaultView: WaveView = { offsetMs: 0, spanMs: 30000, playheadMs: 0, selection: null, selectedId: null, loop: false }
interface Context {
 operation:{id:string;label:string;progress:number}|null;cancelOperation:()=>void;project: WaveProject | null; history: TimelineHistory; view: WaveView; setView: React.Dispatch<React.SetStateAction<WaveView>>
 edit: (timeline: WaveTimeline, sources?: WaveSource[]) => void; undo: () => void; redo: () => void; rename: (name: string) => void
 busy: boolean; error: string; setError: (error: string) => void; saveStatus: string; flush: () => Promise<void>
 openBundle:(path:string|File)=>Promise<boolean>; deleteProject:(id:string)=>Promise<void>; setVideos:(videos:WaveVideo[])=>void; importVideo:(path:string,at:number)=>Promise<void>; importVideos:(paths:string[],at:number)=>Promise<void>; create: () => Promise<boolean>; open: (id: string) => Promise<boolean>; importAudio: (kind: 'file' | 'memo' | 'sound', value: string, mode?: 'insert' | 'append' | 'replace' | 'sfx', at?: number) => Promise<void>; addSource: (source: WaveSource, at?: number, lane?: 'main' | 'sfx') => void; rememberOriginal: () => void; restoreOriginal: () => void
}
const WaveContext = createContext<Context | null>(null)
export function useOptionalWaveStudio(): Context | null { return useContext(WaveContext) }
export function useWaveStudio(): Context { const c = useContext(WaveContext); if (!c) throw new Error('Wave Studio needs its provider'); return c }
function readView(id: string): WaveView { try { const v = JSON.parse(localStorage.getItem(`homer.wave.view.${id}`) || 'null') as WaveView | null; return v && Number.isFinite(v.spanMs) && v.spanMs >= 250 && v.spanMs <= 86400000 && Number.isFinite(v.offsetMs) && v.offsetMs >= 0 && Number.isFinite(v.playheadMs) && v.playheadMs >= 0 && (v.selection === null || (Array.isArray(v.selection) && v.selection.length === 2 && v.selection.every(Number.isFinite) && v.selection[0] >= 0 && v.selection[1] > v.selection[0])) && (v.selectedId === null || typeof v.selectedId === 'string') && typeof v.loop === 'boolean' ? { ...defaultView, ...v } : defaultView } catch { return defaultView } }
export function WaveStudioProvider({ children }: PropsWithChildren): JSX.Element {
 const [state, rawDispatch] = useReducer(reducer, initial), [view, rawSetView] = useState<WaveView>(defaultView)
 const [operation,setOperation]=useState<{id:string;label:string;progress:number}|null>(null)
 const restoring=useRef(true)
 const batchCancelled=useRef(false)
 const activeImport=useRef<string|null>(null)
 const busyRef=useRef(false), uploadAbort=useRef<AbortController|null>(null)
 const [openingBundle,setOpeningBundle]=useState(false)
 async function importing<T>(label:string,fn:(id:string)=>Promise<T>):Promise<T>{if(activeImport.current)throw new Error("Finish or cancel the active import first.");const id=crypto.randomUUID();activeImport.current=id;setOperation({id,label,progress:0});let polling=false;const timer=window.setInterval(()=>{if(polling)return;polling=true;void waveApi.operationStatus(id).then(s=>{if(s&&activeImport.current===id)setOperation({...s,label})}).catch(()=>{}).finally(()=>{polling=false})},500);try{return await fn(id)}finally{window.clearInterval(timer);if(activeImport.current===id){activeImport.current=null;setOperation(null)}}}
 function cancelOperation():void{batchCancelled.current=true;uploadAbort.current?.abort();if(activeImport.current)void waveApi.cancelOperation(activeImport.current).catch(e=>setError(errorMessage(e)))}
 const [busy, setBusy] = useState(false), [error, setError] = useState(''), [saveStatus, setSaveStatus] = useState('Saved')
 const current = useRef(state)
 function dispatch(action: Action): void { current.current = reducer(current.current, action); rawDispatch(action) }
 const viewRef = useRef(view)
 const setView: React.Dispatch<React.SetStateAction<WaveView>> = next => { const value = typeof next === 'function' ? next(viewRef.current) : next; viewRef.current = value; rawSetView(value) }
 const savedView = useRef(JSON.stringify(view))
 const revision = useRef(0), savedChange = useRef(0), saving = useRef<Promise<void> | null>(null)
 async function flush(): Promise<void> {
   while (saving.current) await saving.current
   const snapshot = current.current
   const viewSnapshot = JSON.stringify(viewRef.current)
   if (!snapshot.project || (savedChange.current === snapshot.change && savedView.current === viewSnapshot) || !hasBackend()) return
   setSaveStatus('Saving...')
   const work = waveApi.save({ ...snapshot.project, view: viewRef.current }, revision.current).then(saved => { revision.current = saved.revision; if (current.current.project?.id === saved.id) current.current = { ...current.current, project: { ...current.current.project, revision: saved.revision } }; dispatch({ kind: 'saved', id: saved.id, revision: saved.revision }); savedView.current = viewSnapshot; savedChange.current = snapshot.change; setSaveStatus(savedChange.current === current.current.change ? 'Saved' : 'Saving...') }).catch(cause => { setSaveStatus('Not saved'); setError(errorMessage(cause)); throw cause })
   saving.current = work
   try { await work } finally { if (saving.current === work) saving.current = null }
 }
 function load(p: WaveProject): void { p = { ...p, timeline: recoverVoiceVersions(reconcileVoiceProduction(p.timeline),p.voiceOriginal) }; const previous = p.view || readView(p.id); const restored = {...previous, selectedIds: previous.selectedIds || (previous.selectedId ? [previous.selectedId] : [])}; current.current = { project: p, history: { past: [], present: p.timeline, future: [] }, change: 0 }; viewRef.current = restored; savedView.current = JSON.stringify(restored); revision.current = p.revision; savedChange.current = 0; dispatch({ kind: 'load', project: p }); setView(restored); localStorage.setItem('homer.wave.lastProject', p.id); setSaveStatus('Saved') }
 useEffect(() => {
   if (!hasBackend()) { restoring.current=false; return }
   let alive = true
   const id = localStorage.getItem('homer.wave.lastProject')
   if (id) { setBusy(true); void waveApi.get(id).then(p => { if (alive) load(p) }).catch(cause => { if (alive) setError(errorMessage(cause)) }).finally(() => { restoring.current=false; if (alive) setBusy(false) }) } else { restoring.current=false }
   return () => { alive = false }
 }, [])
 useEffect(() => { const timer = window.setTimeout(() => { void flush().catch(() => {}) }, 450); return () => window.clearTimeout(timer) }, [state.change, view])
 useEffect(() => { if (!state.project) return; const timer = window.setTimeout(() => localStorage.setItem(`homer.wave.view.${state.project!.id}`, JSON.stringify(view)), 350); return () => window.clearTimeout(timer) }, [view, state.project?.id])
 useEffect(() => { const handler = (e: BeforeUnloadEvent) => { if (current.current.project && (savedChange.current !== current.current.change || savedView.current !== JSON.stringify(viewRef.current))) { e.preventDefault(); e.returnValue = ''; void flush().catch(() => {}) } }; window.addEventListener('beforeunload', handler); return () => window.removeEventListener('beforeunload', handler) }, [])
 useEffect(() => { const handler=(event:Event)=>setError(String((event as CustomEvent).detail));window.addEventListener('homer-download-error',handler);return()=>window.removeEventListener('homer-download-error',handler) }, [])
 useEffect(() => { if (!isDesktop()) return; let alive = true, unlisten: (()=>void) | undefined; const win=getCurrentWindow(); void win.onCloseRequested(async event => { event.preventDefault(); try { await drain(); if (current.current.project) localStorage.setItem(`homer.wave.view.${current.current.project.id}`,JSON.stringify(viewRef.current)); await win.destroy() } catch { setError('Save failed. Keep the window open and retry after resolving the error.') } }).then(fn=>{if(alive) unlisten=fn;else fn()}); return ()=>{alive=false;unlisten?.()} }, [])
 useEffect(() => { if(!isDesktop())return;let alive=true,checking=false;const timer=window.setInterval(()=>{if(checking||restoring.current||busyRef.current||activeImport.current||saving.current)return;checking=true;void (async()=>{const paths=await invoke<string[]>('take_open_wave_files');for(let path of paths){if(!alive)break;if(path===':open-dialog:'){const chosen=await chooseFile({title:'Open Wave project',multiple:false,filters:[{name:'Homer Studio project',extensions:['wavehs']}]});if(typeof chosen!=='string')continue;path=chosen}if(alive)await openBundle(path)}})().catch(e=>{if(alive)setError(errorMessage(e))}).finally(()=>{checking=false})},750);return()=>{alive=false;window.clearInterval(timer)} }, [])
 async function task(fn: () => Promise<void>): Promise<boolean> { if(busyRef.current||restoring.current){notify('Finish or cancel the current operation first.');return false}busyRef.current=true;setBusy(true); setError(''); try { await fn(); return true } catch (cause) { setError(errorMessage(cause)); return false } finally { busyRef.current=false;setBusy(false) } }
 async function drain(): Promise<void> { if(!hasBackend())return; do { await flush() } while (current.current.project && (savedChange.current !== current.current.change || savedView.current !== JSON.stringify(viewRef.current))) }
 async function openBundle(path:string|File):Promise<boolean> {
  return task(async()=>{
   await drain();setOpeningBundle(true)
   const controller=new AbortController();uploadAbort.current=controller
   try {
    const project=await importing(typeof path==='string'?'Opening project':'Uploading project',async id=>{
     const source=typeof path==='string'?path:await uploadFile(path,path.name,controller.signal)
     if(controller.signal.aborted)throw new Error('Project opening cancelled.')
     setOperation({id,label:'Opening project',progress:0})
     return waveApi.importBundle(source,id)
    })
    load(project);window.location.hash='wave-studio';notify('Project opened with its media and voice versions.')
   }catch(cause){notify(errorMessage(cause),'error');throw cause}
   finally{uploadAbort.current=null;setOpeningBundle(false)}
  })
 }
 useEffect(()=>{
  if(isDesktop()){
   let alive=true,unlisten:(()=>void)|undefined
   void getCurrentWebview().onDragDropEvent(event=>{if(alive&&event.payload.type==='drop'&&event.payload.paths.some(p=>/\.wavehs$/i.test(p))){if(event.payload.paths.length!==1)notify('Drop one project at a time.','error');else void openBundle(event.payload.paths[0]!)}}).then(fn=>{if(alive)unlisten=fn;else fn()}).catch(e=>{if(alive)notify(errorMessage(e),'error')})
   return()=>{alive=false;unlisten?.()}
  }
  const drag=(event:DragEvent)=>{if(event.dataTransfer?.types.includes('Files'))event.preventDefault()}
  const drop=(event:DragEvent)=>{const files=Array.from(event.dataTransfer?.files||[]);if(!files.some(f=>/\.wavehs$/i.test(f.name)))return;event.preventDefault();event.stopImmediatePropagation();if(files.length!==1)notify('Drop one project at a time.','error');else void openBundle(files[0]!)}
  window.addEventListener('dragover',drag);window.addEventListener('drop',drop,true)
  return()=>{window.removeEventListener('dragover',drag);window.removeEventListener('drop',drop,true)}
 },[])
 async function create(): Promise<boolean> { return task(async () => { await drain(); load(await waveApi.create('Untitled narration')) }) }
 async function open(id: string): Promise<boolean> { return task(async () => { await drain(); load(await waveApi.get(id)); window.location.hash = 'wave-studio' }) }
 async function deleteProject(id:string):Promise<void>{await drain();await waveApi.delete(id);if(current.current.project?.id===id){current.current=initial;dispatch({kind:'clear'});setView(defaultView);savedView.current=JSON.stringify(defaultView);savedChange.current=0;revision.current=0;localStorage.removeItem('homer.wave.lastProject')}}
 function setVideos(videos:WaveVideo[]):void{if(videosOverlap(videos)){setError('Video fragments cannot overlap. Arrange them sequentially first.');return}dispatch({kind:'videos',videos})}
 async function importVideos(paths:string[],at:number):Promise<void>{await task(async()=>{batchCancelled.current=false;let position=at;for(let i=0;i<paths.length;i++){if(batchCancelled.current)throw new Error(`Import cancelled. ${i} of ${paths.length} videos added; remaining files were not imported.`);try{const origin=current.current.project?.id,video=await importing(`Preparing video ${i+1} of ${paths.length}`,id=>waveApi.importVideo(paths[i]!,id));let p=current.current.project;if(p?.id!==origin)throw new Error('Project changed during video import. Try again.');if(!p){p=await waveApi.create(video.name);load(p)}if(videosOverlap(p.videos||[]))throw new Error('Arrange existing overlapping videos before adding more.');const start=placeVideo(p.videos||[],video.durationMs,position);setVideos([...(p.videos||[]),{...video,startMs:start}]);position=start+video.durationMs;setView(v=>({...v,selectedId:video.id,spanMs:Math.max(v.spanMs,position*1.04)}));if(start!==at&&i===0)setError(`Video placed at ${(start/1000).toFixed(3)}s to avoid overlap.`)}catch(e){throw new Error(`${errorMessage(e)} (${i} of ${paths.length} videos added; unfinished: ${paths.slice(i).map(p=>p.split('/').pop()).join(', ')})`)}}})}
 async function importVideo(path:string,at:number):Promise<void>{await importVideos([path],at)}
 function edit(timeline: WaveTimeline, sources?: WaveSource[]): void { dispatch({ kind: 'edit', timeline: reconcileVoiceProduction(timeline), sources }) }
 function addSource(source: WaveSource, at?: number, lane: 'main' | 'sfx' = 'sfx'): void {
   const p = current.current.project; if (!p) return
   const start = Math.max(0, at ?? viewRef.current.selection?.[0] ?? viewRef.current.playheadMs)
   const clip = sourceClip(source, start, lane === 'sfx')
   edit(lane === 'sfx' ? { ...p.timeline, sfx: [...p.timeline.sfx, clip] } : insertMain(p.timeline, clip, start), p.sources.some(s => s.id === source.id) ? p.sources : [...p.sources, source])
 }
 async function importAudio(kind: 'file' | 'memo' | 'sound', value: string, mode: 'insert' | 'append' | 'replace' | 'sfx' = 'insert', at?: number): Promise<void> {
   await task(async () => {
     const origin = current.current.project?.id
     const source = await importing('Preparing audio',id=>waveApi.import(kind, kind === 'file' ? value : null, kind === 'file' ? null : value,id))
     let p = current.current.project
     if (p?.id !== origin) throw new Error('Project changed during import. Add the audio again to this project.')
     if (!p) { p = await waveApi.create(source.name.replace(/\.[^.]+$/, '')); load(p) }
     const timeline = mode === 'replace' ? { ...p.timeline, clips: [], voices: [] } : p.timeline
     const position = mode === 'append' ? Math.max(0, ...timeline.clips.map(c => c.startMs + (c.sourceEndMs - c.sourceStartMs) / c.speed)) : mode === 'replace' ? 0 : Math.max(0, at ?? viewRef.current.selection?.[0] ?? viewRef.current.playheadMs)
     if(position+source.durationMs>86400000)throw new Error('Audio would exceed the 24-hour timeline limit. Choose an earlier position.')
     const next = mode === 'sfx' ? { ...timeline, sfx: [...timeline.sfx, sourceClip(source, position)] } : insertMain(timeline, sourceClip(source, position), position)
     edit(next, [...p.sources, source]); setView(v => ({ ...v, ...(timelineDuration(p!.timeline) === 0 || mode === 'replace' ? { offsetMs: 0, spanMs: Math.max(1000, timelineDuration(next) * 1.04) } : {}), playheadMs: position, selection: null, selectedId: null })); window.location.hash = 'wave-studio'
   })
 }
 function rememberOriginal(): void { if (current.current.project && !current.current.project.voiceOriginal) dispatch({ kind: 'original', timeline: current.current.project.timeline }) }
 function restoreOriginal(): void { const p = current.current.project; if (p?.voiceOriginal) { edit(p.voiceOriginal); dispatch({ kind: 'original', timeline: null }) } }
 const value: Context = { operation,cancelOperation,project: state.project, history: state.history, view, setView, edit, undo: () => dispatch({ kind: 'undo' }), redo: () => dispatch({ kind: 'redo' }), rename: name => dispatch({ kind: 'name', name }), busy, error, setError, saveStatus, flush: drain, deleteProject, setVideos, importVideo, importVideos, create, open, openBundle, importAudio, addSource, rememberOriginal, restoreOriginal }
 return <WaveContext.Provider value={value}>{children}{openingBundle&&<div className="wave-modal-backdrop"><StudioModal title="Opening project" onClose={cancelOperation}><h2>{operation?.label||'Opening project'}</h2><p>Restoring audio, video references and voice versions.</p><progress max={100} value={operation?.progress||0}/><p>{operation?.progress||0}%</p><button onClick={cancelOperation}>Cancel</button></StudioModal></div>}</WaveContext.Provider>
}
