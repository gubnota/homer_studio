import { getCurrentWindow } from '@tauri-apps/api/window'
import { createContext, useContext, useEffect, useReducer, useRef, useState, type PropsWithChildren } from 'react'
import { blankTimeline, sourceClip, timelineDuration, type WaveProject, type WaveSource, type WaveTimeline, type WaveView } from '../../shared/waveStudio'
import { historyEdit, historyRedo, historyUndo, type TimelineHistory } from '../../shared/waveStudioEdits'
import { errorMessage, isDesktop } from './native'
import { waveApi } from './waveStudioNative'
interface State { project: WaveProject | null; history: TimelineHistory; change: number }
type Action = { kind: 'load'; project: WaveProject } | { kind: 'edit'; timeline: WaveTimeline; sources?: WaveSource[] } | { kind: 'undo' | 'redo' } | { kind: 'name'; name: string }
const initial: State = { project: null, history: { past: [], present: blankTimeline(), future: [] }, change: 0 }
function reducer(s: State, a: Action): State {
 if (a.kind === 'load') return { project: a.project, history: { past: [], present: a.project.timeline, future: [] }, change: 0 }
 if (!s.project) return s
 if (a.kind === 'name') return { ...s, project: { ...s.project, name: a.name }, change: s.change + 1 }
 const h = a.kind === 'edit' ? historyEdit(s.history, a.timeline) : a.kind === 'undo' ? historyUndo(s.history) : historyRedo(s.history)
 if (h === s.history && !(a.kind === 'edit' && a.sources)) return s
 return { project: { ...s.project, timeline: h.present, sources: a.kind === 'edit' && a.sources ? a.sources : s.project.sources }, history: h, change: s.change + 1 }
}
const defaultView: WaveView = { offsetMs: 0, spanMs: 30000, playheadMs: 0, selection: null, selectedId: null, loop: false }
interface Context {
 project: WaveProject | null; history: TimelineHistory; view: WaveView; setView: React.Dispatch<React.SetStateAction<WaveView>>
 edit: (timeline: WaveTimeline, sources?: WaveSource[]) => void; undo: () => void; redo: () => void; rename: (name: string) => void
 busy: boolean; error: string; setError: (error: string) => void; saveStatus: string; flush: () => Promise<void>
 create: () => Promise<void>; open: (id: string) => Promise<void>; importAudio: (kind: 'file' | 'memo' | 'sound', value: string, mode?: 'append' | 'replace' | 'sfx') => Promise<void>; addSource: (source: WaveSource, at?: number) => void
}
const WaveContext = createContext<Context | null>(null)
export function useOptionalWaveStudio(): Context | null { return useContext(WaveContext) }
export function useWaveStudio(): Context { const c = useContext(WaveContext); if (!c) throw new Error('Wave Studio needs its provider'); return c }
function readView(id: string): WaveView { try { const v = JSON.parse(localStorage.getItem(`homer.wave.view.${id}`) || 'null') as WaveView | null; return v && Number.isFinite(v.spanMs) && v.spanMs >= 250 && v.spanMs <= 86400000 && Number.isFinite(v.offsetMs) && v.offsetMs >= 0 && Number.isFinite(v.playheadMs) && v.playheadMs >= 0 && (v.selection === null || (Array.isArray(v.selection) && v.selection.length === 2 && v.selection.every(Number.isFinite) && v.selection[0] >= 0 && v.selection[1] > v.selection[0])) && (v.selectedId === null || typeof v.selectedId === 'string') && typeof v.loop === 'boolean' ? { ...defaultView, ...v } : defaultView } catch { return defaultView } }
export function WaveStudioProvider({ children }: PropsWithChildren): JSX.Element {
 const [state, dispatch] = useReducer(reducer, initial), [view, setView] = useState<WaveView>(defaultView)
 const [busy, setBusy] = useState(false), [error, setError] = useState(''), [saveStatus, setSaveStatus] = useState('Saved')
 const current = useRef(state); current.current = state
 const revision = useRef(0), savedChange = useRef(0), saving = useRef<Promise<void> | null>(null)
 async function flush(): Promise<void> {
   while (saving.current) await saving.current
   const snapshot = current.current
   if (!snapshot.project || savedChange.current === snapshot.change || !isDesktop()) return
   setSaveStatus('Saving…')
   const work = waveApi.save(snapshot.project, revision.current).then(saved => { revision.current = saved.revision; savedChange.current = snapshot.change; setSaveStatus(savedChange.current === current.current.change ? 'Saved' : 'Saving…') }).catch(cause => { setSaveStatus('Not saved'); setError(errorMessage(cause)); throw cause })
   saving.current = work
   try { await work } finally { if (saving.current === work) saving.current = null }
 }
 function load(p: WaveProject): void { revision.current = p.revision; savedChange.current = 0; dispatch({ kind: 'load', project: p }); setView(readView(p.id)); localStorage.setItem('homer.wave.lastProject', p.id); setSaveStatus('Saved') }
 useEffect(() => {
   if (!isDesktop()) return
   let alive = true
   const id = localStorage.getItem('homer.wave.lastProject')
   if (id) { setBusy(true); void waveApi.get(id).then(p => { if (alive) load(p) }).catch(cause => { if (alive) setError(errorMessage(cause)) }).finally(() => { if (alive) setBusy(false) }) }
   return () => { alive = false }
 }, [])
 useEffect(() => { const timer = window.setTimeout(() => { void flush().catch(() => {}) }, 450); return () => window.clearTimeout(timer) }, [state.change])
 useEffect(() => { if (!state.project) return; const timer = window.setTimeout(() => localStorage.setItem(`homer.wave.view.${state.project!.id}`, JSON.stringify(view)), 350); return () => window.clearTimeout(timer) }, [view, state.project?.id])
 useEffect(() => { const handler = (e: BeforeUnloadEvent) => { if (current.current.project && savedChange.current !== current.current.change) { e.preventDefault(); e.returnValue = ''; void flush().catch(() => {}) } }; window.addEventListener('beforeunload', handler); return () => window.removeEventListener('beforeunload', handler) }, [])
 useEffect(() => { if (!isDesktop()) return; let alive = true, unlisten: (()=>void) | undefined; const win=getCurrentWindow(); void win.onCloseRequested(async event => { event.preventDefault(); try { while (savedChange.current !== current.current.change) await flush(); if (current.current.project) localStorage.setItem(`homer.wave.view.${current.current.project.id}`,JSON.stringify(viewRef.current)); await win.destroy() } catch { setError('Save failed. Keep the window open and retry after resolving the error.') } }).then(fn=>{if(alive) unlisten=fn;else fn()}); return ()=>{alive=false;unlisten?.()} }, [])
 const viewRef=useRef(view);viewRef.current=view
 async function task(fn: () => Promise<void>): Promise<void> { setBusy(true); setError(''); try { await fn() } catch (cause) { setError(errorMessage(cause)) } finally { setBusy(false) } }
 async function create(): Promise<void> { await task(async () => { await flush(); load(await waveApi.create('Untitled narration')) }) }
 async function open(id: string): Promise<void> { await task(async () => { await flush(); load(await waveApi.get(id)); window.location.hash = 'wave-studio' }) }
 function edit(timeline: WaveTimeline, sources?: WaveSource[]): void { dispatch({ kind: 'edit', timeline, sources }) }
 function addSource(source: WaveSource, at?: number): void {
   const p = current.current.project; if (!p) return
   const start = at ?? view.selection?.[1] ?? view.playheadMs
   edit({ ...p.timeline, sfx: [...p.timeline.sfx, sourceClip(source, start, true)] }, p.sources.some(s => s.id === source.id) ? p.sources : [...p.sources, source])
 }
 async function importAudio(kind: 'file' | 'memo' | 'sound', value: string, mode: 'append' | 'replace' | 'sfx' = 'append'): Promise<void> {
   await task(async () => {
     const source = await waveApi.import(kind, kind === 'file' ? value : null, kind === 'file' ? null : value)
     let p = current.current.project
     if (!p) { p = await waveApi.create(source.name.replace(/\.[^.]+$/, '')); revision.current = p.revision; savedChange.current = 0; dispatch({ kind: 'load', project: p }); localStorage.setItem('homer.wave.lastProject', p.id) }
     const timeline = mode === 'replace' ? { ...p.timeline, clips: [], voices: [] } : p.timeline
     const next = mode === 'sfx' ? { ...timeline, sfx: [...timeline.sfx, sourceClip(source, view.playheadMs, true)] } : { ...timeline, clips: [...timeline.clips, sourceClip(source, Math.max(0, ...timeline.clips.map(c => c.startMs + (c.sourceEndMs - c.sourceStartMs) / c.speed)))] }
     edit(next, [...p.sources, source]); setView(v => ({ ...v, ...(mode !== 'sfx' ? { offsetMs: 0, spanMs: Math.max(1000, timelineDuration(next) * 1.04), playheadMs: mode === 'replace' ? 0 : v.playheadMs, selection: null, selectedId: null } : {}) })); window.location.hash = 'wave-studio'
   })
 }
 const value: Context = { project: state.project, history: state.history, view, setView, edit, undo: () => dispatch({ kind: 'undo' }), redo: () => dispatch({ kind: 'redo' }), rename: name => dispatch({ kind: 'name', name }), busy, error, setError, saveStatus, flush, create, open, importAudio, addSource }
 return <WaveContext.Provider value={value}>{children}</WaveContext.Provider>
}
