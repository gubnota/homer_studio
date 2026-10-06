import {useEffect, useRef, useState, type PropsWithChildren} from 'react'
import {StudioIcon} from './StudioIcon'
import {errorMessage} from '../native'
export function StudioModal({title,onClose,className='',children}:PropsWithChildren<{title:string;onClose:()=>void|Promise<void>;className?:string}>):JSX.Element {
 const root=useRef<HTMLElement>(null),closeRef=useRef(onClose),[closing,setClosing]=useState(false),[error,setError]=useState(''),pending=useRef(false)
 closeRef.current=onClose
 async function close():Promise<void>{if(pending.current)return;pending.current=true;setClosing(true);try{await closeRef.current()}catch(e){setError(errorMessage(e))}finally{pending.current=false;setClosing(false)}}
 useEffect(()=>{
  const previous=document.activeElement as HTMLElement|null,element=root.current!
  element.focus()
  const key=(e:KeyboardEvent)=>{
   const dialogs=Array.from(document.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"],dialog[open]'))
   if(dialogs.at(-1)!==element)return
   if(e.key==='Escape'){e.preventDefault();e.stopImmediatePropagation();void close()}
   if(e.key==='Tab'){
    const items=Array.from(element.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),a[href],[tabindex="0"]')).filter(el=>el.getClientRects().length>0)
    const first=items[0],last=items.at(-1)
    if(!first){e.preventDefault();element.focus()}else if(e.shiftKey&&(document.activeElement===first||document.activeElement===element)){e.preventDefault();last?.focus()}else if(!e.shiftKey&&(document.activeElement===last||document.activeElement===element)){e.preventDefault();first.focus()}
   }
  }
  window.addEventListener('keydown',key,true)
  return()=>{window.removeEventListener('keydown',key,true);if(previous?.isConnected)previous.focus()}
 },[])
 return <section ref={root} tabIndex={-1} className={`wave-dialog studio-modal ${className}`} role="dialog" aria-modal="true" aria-label={title}><button className="studio-modal-close" aria-label={`Close ${title}`} disabled={closing} onClick={()=>void close()}><StudioIcon name="close"/></button>{error&&<p role="alert">{error}</p>}{children}</section>
}
