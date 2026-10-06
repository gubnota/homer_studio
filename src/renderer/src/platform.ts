import { invoke as nativeInvoke } from '@tauri-apps/api/core'
import { listen as nativeListen } from '@tauri-apps/api/event'
import { open as nativeOpen, save as nativeSave } from '@tauri-apps/plugin-dialog'

export function isDesktop(): boolean { return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window }
export function hasBackend(): boolean { return isDesktop() || (typeof window !== 'undefined' && /^https?:$/.test(window.location?.protocol || '')) }
// Browser-owned dialogs share the same close control and keyboard behavior.
function browserDialog(panel:HTMLElement,onCancel:()=>void):()=>void {
 const previous=document.activeElement as HTMLElement|null
 panel.classList.add('studio-modal');panel.setAttribute('aria-modal','true');panel.tabIndex=-1
 const close=document.createElement('button');close.type='button';close.className='studio-modal-close';close.setAttribute('aria-label',`Close ${panel.getAttribute('aria-label')||'dialog'}`)
 close.innerHTML='<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 6 12 12M6 18 18 6"/></svg>'
 close.onclick=onCancel;panel.prepend(close)
 const key=(event:KeyboardEvent)=>{const dialogs=document.querySelectorAll('[role="dialog"][aria-modal="true"],dialog[open]');if(dialogs[dialogs.length-1]!==panel)return;if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();onCancel()}else if(event.key==='Tab'){const items=Array.from(panel.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),a[href]'));const first=items[0],last=items[items.length-1];if(event.shiftKey&&(document.activeElement===first||document.activeElement===panel)){event.preventDefault();last?.focus()}else if(!event.shiftKey&&(document.activeElement===last||document.activeElement===panel)){event.preventDefault();first?.focus()}}}
 window.addEventListener('keydown',key,true)
 return()=>{window.removeEventListener('keydown',key,true);if(previous?.isConnected)previous.focus()}
}
let login: Promise<void> | null = null
async function signIn(): Promise<void> {
 if (login) return login
 login = new Promise<void>((resolve, reject) => {
  const backdrop=document.createElement('div');backdrop.className='wave-dialog-backdrop'
  const form=document.createElement('form');form.className='wave-dialog';form.setAttribute('role','dialog');form.setAttribute('aria-label','Server sign in')
  const title=document.createElement('h2');title.textContent='Connect to Homer Studio'
  const label=document.createElement('label');label.textContent='Server access token'
  const input=document.createElement('input');input.type='password';input.autocomplete='current-password';input.required=true;label.append(input)
  const status=document.createElement('p');status.setAttribute('role','alert')
  const submit=document.createElement('button');submit.type='submit';submit.textContent='Connect';submit.className='primary'
  const cancel=document.createElement('button');cancel.type='button';cancel.textContent='Cancel';const dismiss=()=>{cleanup();backdrop.remove();reject(new Error('Server sign-in cancelled.'))};cancel.onclick=dismiss;const cleanup=browserDialog(form,dismiss)
  form.append(title,label,status,submit,cancel);backdrop.append(form);document.body.append(backdrop);input.focus()
  form.onsubmit=async event=>{event.preventDefault();submit.disabled=true;try{const response=await fetch('/api/session',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({token:input.value})});if(!response.ok)throw new Error('The access token was not accepted.');input.value='';cleanup();backdrop.remove();resolve()}catch(error){status.textContent=String(error instanceof Error?error.message:error);submit.disabled=false}}
 }).finally(()=>{login=null})
 return login
}
export async function serverRequest(url: string, options?: RequestInit): Promise<Response> {
 let response=await fetch(url,{...options,credentials:'same-origin'})
 if(response.status===401){await signIn();response=await fetch(url,{...options,credentials:'same-origin'})}
 if(!response.ok){let message=`Server operation failed (${response.status}).`;try{const body=await response.json();message=body.message||message}catch{}throw new Error(message)}
 return response
}
const pendingDownloads=new Map<string,string>()
export async function downloadFile(path:string):Promise<void>{
 const url=`/api/download?path=${encodeURIComponent(path)}`
 await serverRequest(url,{method:'HEAD'})
 const link=document.createElement('a');link.href=url;link.download=path.split('/').pop()||'export';link.click()
}
export async function invoke<T>(command:string,args:Record<string,unknown>={}):Promise<T>{
 if(isDesktop())return nativeInvoke<T>(command,args)
 if(command.startsWith('audio_capture_')) {const {captureCommand}=await import('./browserCapture');return captureCommand(command,args) as Promise<T>}
 const response=await serverRequest(`/api/command/${encodeURIComponent(command)}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(args)})
 const result=await response.json()
 if(command==='wave_preview')return new Uint8Array(result.binary).buffer as T
 if(['wave_export','export_memo'].includes(command)&&typeof result==='string'&&typeof args.outputPath==='string')pendingDownloads.set(result,args.outputPath)
 else if(['export_sounds','save_exports'].includes(command)&&Array.isArray(result)){for(const path of result)if(typeof path==='string')await downloadFile(path)}
 else if(['wave_export_bundle','export_sound','save_export_file'].includes(command)){const path=args.path||args.outputPath||args.destination;if(typeof path==='string')await downloadFile(path)}
 if(command==='list_jobs'&&Array.isArray(result))for(const job of result){if(['failed','cancelled'].includes(job.status))pendingDownloads.delete(job.id);if(job.status==='completed'){const path=pendingDownloads.get(job.id); if(path){pendingDownloads.delete(job.id);void downloadFile(path).catch(error=>window.dispatchEvent(new CustomEvent('homer-download-error',{detail:String(error instanceof Error?error.message:error)})))}}}
 return result as T
}
export async function uploadFile(file:File|Blob,name:string,signal?:AbortSignal):Promise<string>{const response=await serverRequest(`/api/upload?name=${encodeURIComponent(name)}`,{method:'POST',body:file,signal});return (await response.json()).path}
export async function open(options:Parameters<typeof nativeOpen>[0]={}):Promise<string|string[]|null>{
 if(isDesktop())return nativeOpen(options)
 if(options.directory){
  if(!String(options.title||'').startsWith('Open'))return invoke<string>('default_project_parent')
  const response=await serverRequest('/api/projects');const projects=await response.json() as {name:string;path:string}[]
  return new Promise<string|null>(resolve=>{const backdrop=document.createElement('div');backdrop.className='wave-dialog-backdrop';const panel=document.createElement('section');panel.className='wave-dialog';panel.setAttribute('role','dialog');panel.setAttribute('aria-label','Open server project');const title=document.createElement('h2');title.textContent='Open project';panel.append(title);if(!projects.length){const empty=document.createElement('p');empty.textContent='No manuscript projects saved on this server yet.';panel.append(empty)}for(const project of projects){const button=document.createElement('button');button.textContent=project.name;button.onclick=()=>{cleanup();backdrop.remove();resolve(project.path)};panel.append(button)}const cancel=document.createElement('button');cancel.textContent='Cancel';const dismiss=()=>{cleanup();backdrop.remove();resolve(null)};cancel.onclick=dismiss;const cleanup=browserDialog(panel,dismiss);panel.append(cancel);backdrop.append(panel);document.body.append(backdrop);panel.focus()})
 }
 const file=await new Promise<File|null>(resolve=>{const input=document.createElement('input');input.type='file';input.accept=(options.filters||[]).flatMap(f=>f.extensions.map(e=>'.'+e)).join(',');input.onchange=()=>resolve(input.files?.[0]||null);input.oncancel=()=>resolve(null);input.click()})
 return file?uploadFile(file,file.name):null
}
export async function save(options:Parameters<typeof nativeSave>[0]={}):Promise<string|null>{
 if(isDesktop())return nativeSave(options)
 const name=options.defaultPath?.split('/').pop()||'export.m4a';const response=await serverRequest('/api/destination',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({name})});return (await response.json()).path
}
export async function listen<T>(event:string,handler:(event:{payload:T})=>void):Promise<()=>void>{
 if(isDesktop())return nativeListen<T>(event,handler)
 const listener=(e:Event)=>handler({payload:(e as CustomEvent<T>).detail});window.addEventListener(event,listener);return()=>window.removeEventListener(event,listener)
}
