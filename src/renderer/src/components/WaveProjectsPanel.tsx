import {useEffect,useState} from 'react'
import {open,save,isDesktop} from '../platform'
import {useWaveStudio} from '../WaveStudioProvider'
import {waveApi} from '../waveStudioNative'
import {errorMessage} from '../native'
import {formatWaveTime,projectDuration,type WaveProject} from '../../../shared/waveStudio'
import {Checkbox} from './StudioControls'
import {StudioModal} from './StudioModal'
import {notify} from './StudioToast'
export function WaveProjectsPanel({onClose}:{onClose:()=>void}):JSX.Element {
 const [includeVideo,setIncludeVideo]=useState(false)
 const w=useWaveStudio(),[projects,setProjects]=useState<WaveProject[]>([]),[deleted,setDeleted]=useState<WaveProject[]>([]),[search,setSearch]=useState(''),[trash,setTrash]=useState(false),[busy,setBusy]=useState(false),[message,setMessage]=useState(''),[confirm,setConfirm]=useState<{id:string|null;name:string}|null>(null)
 async function refresh():Promise<void>{const [p,d]=await Promise.all([waveApi.list(),waveApi.deleted()]);setProjects(p);setDeleted(d)}
 async function task(fn:()=>Promise<void>):Promise<void>{setBusy(true);setMessage('');try{await fn();await refresh()}catch(e){setMessage(errorMessage(e))}finally{setBusy(false)}}
 useEffect(()=>{void task(async()=>{await w.flush()})},[])
 async function exportBundle(project:WaveProject,fromTrash=false):Promise<void>{await w.flush();const p=fromTrash?project:await waveApi.get(project.id);const path=await save({title:'Export project bundle',defaultPath:`${p.name.replace(/[/:]/g,'-')}.wavehs`,filters:[{name:'Homer Wave bundle',extensions:['wavehs']}]});if(path){await waveApi.exportBundle(p,path,includeVideo);notify('Project, media and voice references exported.')}}
 async function openProject(id:string):Promise<void>{if(await w.open(id))onClose();else throw new Error('Could not open this project. Your previous project remains available.')}
 async function openBundle():Promise<void>{const path=await open({title:'Open project bundle',multiple:false,filters:[{name:'Homer Wave bundle',extensions:['wavehs']}]});if(typeof path==='string'){if(await w.openBundle(path))onClose();else throw new Error('Could not open the project bundle.')}}
 const shown=(trash?deleted:projects).filter(p=>p.name.toLocaleLowerCase().includes(search.toLocaleLowerCase()))
 return <StudioModal title="Wave projects" onClose={onClose} className="wave-projects-panel">
  <header><div><small>YOUR WORKSPACE</small><h2>Wave projects</h2></div></header>
  <div className="wave-project-actions">
   <button disabled={busy} className="primary" onClick={()=>void task(async()=>{if(await w.create())onClose();else throw new Error('Could not create the project.')})}>New project</button>
   <button disabled={busy||!w.project} onClick={()=>void task(async()=>{await w.flush();notify('Project saved.')})}>Save current</button>
   {isDesktop()&&<><button disabled={busy||!w.project} onClick={()=>void task(async()=>{await w.flush();const p=await waveApi.get(w.project!.id),path=await save({title:'Save Wave project folder',defaultPath:`${p.name.replace(/[/:]/g,'-')}.wavehs`});if(path){await waveApi.saveCopy(p,path,includeVideo);notify('Project and media saved together.')}})}>Save as .wavehs folder...</button><button disabled={busy} onClick={()=>void task(async()=>{const folder=await open({title:'Open saved Wave project folder',directory:true,multiple:false});if(typeof folder==='string'){await w.flush();if(await w.openBundle(folder))onClose();else throw new Error('Could not open project folder.')}})}>Open folder...</button></>}
   <button disabled={busy||!w.project} onClick={()=>void task(()=>exportBundle(w.project!))}>Export portable archive...</button>
   <button disabled={busy} onClick={()=>void task(openBundle)}>Open .wavehs...</button>
  </div>
  <label><Checkbox checked={includeVideo} onChange={e=>setIncludeVideo(e.target.checked)}/>Include a video copy (larger project)</label>
  <input className="wave-project-search" type="search" aria-label="Search Wave projects" placeholder="Search projects..." value={search} onChange={e=>setSearch(e.target.value)}/>
  <div className="wave-project-tabs"><button aria-pressed={!trash} onClick={()=>setTrash(false)}>Projects ({projects.length})</button><button aria-pressed={trash} onClick={()=>setTrash(true)}>Recently deleted ({deleted.length})</button>{trash&&<button disabled={busy||!deleted.length} onClick={()=>setConfirm({id:null,name:'all recently deleted projects'})}>Empty recently deleted</button>}</div>
  {message&&<p role="alert">{message}</p>}
  <div className="wave-project-list">{!shown.length&&<p className="wave-project-no-results">{search?'No projects match your search.':trash?'No deleted projects.':'Create a project to start recording or importing audio.'}</p>}{shown.map(p=><article className={`wave-project-row ${p.id===w.project?.id?'current':''}`} key={p.id}>
   <div><strong>{p.name}</strong><small>{formatWaveTime(projectDuration(p))} / {new Date(p.updatedAtMs).toLocaleDateString()} {p.id===w.project?.id?' / Current':''}</small></div>
   <div className="wave-project-row-actions">
    {trash?<button disabled={busy} onClick={()=>void task(async()=>{await waveApi.delete(p.id,true);setTrash(false);await openProject(p.id)})}>Restore and open</button>:<button disabled={busy} onClick={()=>void task(()=>openProject(p.id))}>Open</button>}
    <button disabled={busy} onClick={()=>void task(()=>exportBundle(p,trash))}>Export archive</button>
    {isDesktop()&&<button disabled={busy} onClick={()=>void task(()=>waveApi.reveal(p.id,trash))}>Show in Finder</button>}
    <button disabled={busy} aria-label={`Delete ${p.name}${trash?' permanently':''}`} onClick={()=>trash?setConfirm({id:p.id,name:p.name}):void task(()=>w.deleteProject(p.id))}>{trash?'Delete permanently':'Delete'}</button>
   </div>
  </article>)}</div>
  <footer>Projects save automatically. Restore a deleted project to continue editing. Save as .wavehs folder to keep project files together. Portable archives are available for sharing.</footer>
  {confirm&&<div className="wave-modal-backdrop"><StudioModal title="Delete projects permanently" onClose={()=>setConfirm(null)}><h2>Delete {confirm.name} permanently?</h2><p>This removes the saved project state. Shared source media used by other projects remains available.</p><div className="audio-tools"><button onClick={()=>setConfirm(null)}>Cancel</button><button className="danger" disabled={busy} onClick={()=>void task(async()=>{await waveApi.purge(confirm.id);setConfirm(null);notify('Deleted projects removed.')})}>Delete permanently</button></div></StudioModal></div>}
 </StudioModal>
}
