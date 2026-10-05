import { Checkbox } from './StudioControls'
import { useEffect, useRef, useState } from 'react'
import type { JobRecord, Voice } from '../../../shared/contracts'
import type { WaveProcessingResult } from '../../../shared/waveStudio'
import { sourceClip, formatWaveTime, timelineDuration, voiceAudioKey, voiceComplete } from '../../../shared/waveStudio'
import { assignVoice, insertMain, replaceRange } from '../../../shared/waveStudioEdits'
import { useWaveStudio } from '../WaveStudioProvider'
import { waveApi } from '../waveStudioNative'
import { errorMessage, systemApi } from '../native'
import { AudioTransport } from './AudioTransport'
export function WaveProductionPanel({kind,voices,onClose,regionId}:{kind:'speech'|'voices';voices:Voice[];onClose:()=>void;regionId?:string}):JSX.Element {
 const w=useWaveStudio(), current=useRef(w);current.current=w
 const [text,setText]=useState(''),[voiceId,setVoiceId]=useState(voices[0]?.id||''),[jobId,setJobId]=useState(''),[job,setJob]=useState<JobRecord|null>(null),[result,setResult]=useState<WaveProcessingResult|null>(null),[urls,setUrls]=useState<Record<string,string>>({}),[error,setError]=useState(''),[busy,setBusy]=useState(false),[lane]=useState<'main'|'sfx'>('main'),[placement,setPlacement]=useState<'cursor'|'end'>('cursor'),[regenerate,setRegenerate]=useState(false)
 const origin=useRef<{id:string;timeline:string;at:number;voiceId:string;lane:'main'|'sfx'}|null>(null)
 const regions=w.project?.timeline.voices.filter(r=>(!regionId||r.id===regionId)&&(regenerate||!voiceComplete(w.project!.timeline,r)))||[]
 useEffect(()=>{if(!voiceId&&voices[0])setVoiceId(voices[0].id)},[voices,voiceId])
 useEffect(()=>{if(!jobId)return;let alive=true,loading=false;const poll=async()=>{if(loading)return;loading=true;try{const found=(await systemApi.jobs()).find(j=>j.id===jobId);if(!alive)return;if(found)setJob(found);if(found?.status==='completed'){const r=await waveApi.processingResult(jobId);const pairs=await Promise.all(r.sources.map(async s=>[s.id,await waveApi.url(s.id)] as const));if(alive){setResult(r);setUrls(Object.fromEntries(pairs));window.clearInterval(timer)}}else if(found&&['failed','cancelled'].includes(found.status)){setError(found.message||`Job ${found.status}`);window.clearInterval(timer)}}catch(e){if(alive)setError(errorMessage(e))}finally{loading=false}};const timer=window.setInterval(()=>void poll(),700);void poll();return()=>{alive=false;window.clearInterval(timer)}},[jobId])
 async function start():Promise<void>{setBusy(true);setError('');try{await w.flush();const latest=current.current,p=latest.project;if(!p)throw new Error('Open a project first.');const saved=await waveApi.get(p.id);origin.current={id:p.id,timeline:JSON.stringify(p.timeline),at:placement==='end'?timelineDuration(p.timeline):latest.view.selection?.[0]??latest.view.playheadMs,voiceId,lane};setResult(null);setJob(null);setJobId(kind==='speech'?await waveApi.generateSpeech(text,voiceId,p.id,saved.revision):await waveApi.convertRegions({...p,revision:saved.revision},regions.map(r=>r.id),regenerate))}catch(e){setError(errorMessage(e))}finally{setBusy(false)}}
 function accept():void {
  const c=current.current,p=c.project,o=origin.current
  if(!p||!o||!result||p.id!==o.id||result.projectId!==p.id||JSON.stringify(p.timeline)!==o.timeline){setError('The project changed. Generate again before accepting this audio.');return}
  let timeline=p.timeline
  if(kind==='speech') {
   const source=result.sources[0],voice=voices.find(v=>v.id===o.voiceId)
   if(!source||!voice){setError('The selected voice is unavailable. Generate again.');return}
   const clip=sourceClip(source,o.at,o.lane==='sfx')
   timeline=o.lane==='main'?insertMain(timeline,clip,o.at):{...timeline,sfx:[...timeline.sfx,clip]}
   timeline=assignVoice(timeline,o.at,o.at+source.durationMs,voice)
   timeline={...timeline,voices:timeline.voices.map(v=>v.startMs===o.at&&v.voiceId===voice.id?{...v,production:{status:'generated',voiceId:v.voiceId,audioKey:voiceAudioKey(timeline,v)}}:v)}
  } else {
   for(const r of result.replacements) {
    const source=result.sources.find(s=>s.id===r.sourceId)
    if(!source){setError('Converted audio is missing. Retry this job.');return}
    const clip=sourceClip(source,r.startMs);clip.sourceEndMs=r.endMs-r.startMs
    timeline=replaceRange(timeline,r.startMs,r.endMs,clip)
   }
   const completed=new Set(result.replacements.map(r=>r.regionId))
   timeline={...timeline,voices:timeline.voices.map(v=>completed.has(v.id)?{...v,production:{status:'converted',voiceId:v.voiceId,audioKey:voiceAudioKey(timeline,v)}}:v)}
   c.rememberOriginal()
  }
  c.edit(timeline,[...p.sources,...result.sources.filter(s=>!p.sources.some(old=>old.id===s.id))]);onClose()
 }
 const running=!!jobId&&!result&&!['failed','cancelled','completed'].includes(job?.status||'')
 return <section className="wave-dialog" role="dialog" aria-modal="true" aria-label={kind==='speech'?'Generate speech':'Apply tagged voices'}>
  <h2>{kind==='speech'?'Generate speech':regionId?'Generate selected voice':'Apply tagged voices'}</h2>
  {kind==='speech'?<><label>Voice<select value={voiceId} disabled={busy||running||!!result} onChange={e=>setVoiceId(e.target.value)}>{voices.map(v=><option key={v.id} value={v.id}>{v.name}</option>)}</select></label>{!voices.length&&<p>Create a voice in Voice Lab first.</p>}<label>Text<textarea rows={5} value={text} disabled={busy||running} onChange={e=>setText(e.target.value)}/></label><label>Position<select value={placement} disabled={busy||running||!!result} onChange={e=>setPlacement(e.target.value as 'cursor'|'end')}><option value="cursor">Selection start / playhead</option><option value="end">End of timeline</option></select></label></>:<><p>Convert {regions.length} {regionId?'selected':'pending'} passage(s) with Original Chatterbox. Listen below before accepting.</p><label className="wave-check"><Checkbox checked={regenerate} disabled={busy||running} onChange={e=>setRegenerate(e.target.checked)}/>Regenerate completed passages</label>{!regions.length&&<p>All passages are complete, or no voice is assigned.</p>}</>}
  {error&&<p role="alert">{error}</p>}{job&&<p className="wave-production-status" role="status">{job.status} · {job.progress}% {job.message}</p>}
  {running&&<button onClick={()=>void systemApi.controlJob(jobId,'cancel').catch(e=>setError(errorMessage(e)))}>Cancel job</button>}
  {result&&<><p>Ready to review</p>{result.sources.map((s,i)=><div key={s.id}><small>{s.name}{result.replacements[i]&&` · ${formatWaveTime(result.replacements[i]!.startMs)}–${formatWaveTime(result.replacements[i]!.endMs)}`}</small>{urls[s.id]&&<AudioTransport url={urls[s.id]!}/>}</div>)}{result.warnings.map((warning,i)=><p key={i}>{warning}</p>)}<button className="primary" onClick={accept}>{kind==='speech'?'Insert speech':'Accept converted voices'}</button></>}
  <button disabled={busy||running||(kind==='speech'?(!text.trim()||!voiceId):!regions.length)} onClick={()=>void start()}>{result?'Generate again':kind==='speech'?'Generate':'Generate voice preview'}</button><button disabled={busy} onClick={onClose}>Close</button>
 </section>
}
