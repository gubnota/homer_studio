import {useEffect,useState,type ReactNode} from 'react'
import {StudioIcon} from './StudioIcon'
type Toast={id:number;message:ReactNode;kind:'status'|'error'}
let sequence=0
export function notify(message:ReactNode,kind:Toast['kind']='status'):void {if(message)window.dispatchEvent(new CustomEvent('homer-toast',{detail:{id:++sequence,message,kind}}))}
export function useToast(message:string,kind:Toast['kind']='status'):void {useEffect(()=>{if(message)notify(message,kind)},[message,kind])}
export function StudioToastHost():JSX.Element {
 const [items,setItems]=useState<Toast[]>([])
 useEffect(()=>{const timers=new Set<ReturnType<typeof setTimeout>>();const receive=(e:Event)=>{const item=(e as CustomEvent<Toast>).detail;setItems(v=>[...v.filter(t=>t.message!==item.message),item].slice(-4));const timer=setTimeout(()=>{setItems(v=>v.filter(t=>t.id!==item.id));timers.delete(timer)},item.kind==='error'?10000:5500);timers.add(timer)};window.addEventListener('homer-toast',receive);return()=>{window.removeEventListener('homer-toast',receive);timers.forEach(clearTimeout)}},[])
 return <div className="studio-toasts" aria-live="polite">{items.map(t=><div key={t.id} className={`studio-toast ${t.kind}`}><span>{t.message}</span><button aria-label="Dismiss notification" onClick={()=>setItems(v=>v.filter(x=>x.id!==t.id))}><StudioIcon name="close"/></button></div>)}</div>
}

export function StudioNotice({children,kind='status'}:{children:ReactNode;kind?:Toast['kind']}):null {useEffect(()=>{notify(children,kind)},[]);return null}
