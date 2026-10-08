import { describe, expect, it } from 'vitest'
import { placeVideo, videosOverlap, arrangeVideos, videoFrame, voiceAudioKey, voiceComplete, reconcileVoiceProduction, projectDuration, blankTimeline, clipDuration, clipEnd, formatWaveTime, sourceClip, timelineDuration } from '../src/shared/waveStudio'
import { rememberVoiceVersions, restoreVoiceOriginal, recoverVoiceVersions, packSfxRows, insertMain, transferClip, replaceRange, joinCandidateIds, assignVoice, changeSpeed, historyEdit, historyRedo, historyUndo, insertSilence, moveClip, removeRange, splitAt, splitSoundtrack, trimSoundtrack } from '../src/shared/waveStudioEdits'
const source = { id: crypto.randomUUID(), name: 'Narration', durationMs: 10000, channels: 2 }
const original = () => ({ ...blankTimeline(), clips: [sourceClip(source)] })
describe('Wave Studio reversible editing', () => {
 it('splits at the correct source sample without modifying the original', () => { const t = original(), result = splitAt(t, 2500); expect(t.clips).toHaveLength(1); expect(result.clips.map(c => [c.startMs,c.sourceStartMs,c.sourceEndMs])).toEqual([[0,0,2500],[2500,2500,10000]]) })
 it('distinguishes ripple deletion from replacing with silence', () => { const t = original(); expect(timelineDuration(removeRange(t,2000,4000))).toBe(8000); const silent=removeRange(t,2000,4000,true); expect(timelineDuration(silent)).toBe(10000); expect(silent.clips.find(c=>!c.sourceId)?.sourceEndMs).toBe(2000) })
 it('shifts sound effects and voice ranges when inserting silence', () => { const t=assignVoice(original(),3000,5000,{id:'voice',name:'Narrator'}); t.sfx=[sourceClip({...source,durationMs:500},4000,true)]; const next=insertSilence(t,2000,500); expect(next.sfx[0]?.startMs).toBe(4500); expect(next.voices[0]?.startMs).toBe(3500); expect(timelineDuration(next)).toBe(10500) })
 it('preserves source intervals and maps annotations when changing selected speed', () => { const t=assignVoice(original(),2000,4000,{id:'voice',name:'Narrator'}); const next=changeSpeed(t,2000,4000,1.08); expect(timelineDuration(next)).toBeCloseTo(8000+2000/1.08); expect(next.voices[0]?.endMs).toBeCloseTo(2000+2000/1.08); expect(next.clips[1]?.sourceEndMs).toBe(4000) })
 it('does not overwrite later fragments when inserting across a gap', () => { let t=splitAt(splitAt(original(),2000),4000); t.clips[1]!.startMs=3000; t.clips[2]!.startMs=5000; const moved=moveClip(t,t.clips[0]!.id,2500); const ordered=moved.clips; expect(ordered.every((c,i)=>i===0||clipEnd(ordered[i-1]!)<=c.startMs+.01)).toBe(true); expect(ordered.filter(c=>c.sourceId).reduce((sum,c)=>sum+clipDuration(c),0)).toBe(10000); expect(ordered.some(c=>!c.sourceId)).toBe(true) })
 it('swaps unequal fragments explicitly and follows voice annotations', () => { let t=splitAt(original(),2000); t=assignVoice(t,2000,10000,{id:'voice',name:'B'}); const swapped=moveClip(t,t.clips[0]!.id,0,t.clips[1]!.id); expect(swapped.clips[0]?.sourceStartMs).toBe(2000); expect(swapped.voices[0]?.startMs).toBe(0); expect(timelineDuration(swapped)).toBe(10000) })
 it('undoes and redoes exactly, discarding redo after a new edit', () => { const t=original(), h=historyEdit({past:[],present:t,future:[]},insertSilence(t,0,500)); expect(historyUndo(h).present).toEqual(t); expect(historyRedo(historyUndo(h)).present).toEqual(h.present); expect(historyEdit(historyUndo(h),splitAt(t,1000)).future).toEqual([]) })
 it('clamps inherited fades after splitting a short fragment', () => { const t=original(); t.clips[0]!.fadeInMs=5000; expect(splitAt(t,1000).clips[0]?.fadeInMs).toBe(1000) })
 it.each([300000,1800000,3600000,7200000])('edits a %i ms timeline without expanding into samples', length => { const t={...blankTimeline(),clips:[sourceClip({...source,durationMs:length})]}; const next=splitAt(t,length/2); expect(next.clips).toHaveLength(2); expect(timelineDuration(next)).toBe(length) })
 it('formats timestamps to milliseconds',()=>expect(formatWaveTime(3600123)).toBe('01:00:00.123'))
})

describe('Wave production editing',()=>{
 it('inserts audio within a fragment and ripples voice tags and effects',()=>{let t=assignVoice(original(),3000,5000,{id:'v',name:'V'});t.sfx=[sourceClip({...source,durationMs:500},4000,true)];const inserted=sourceClip({...source,durationMs:1000});const n=insertMain(t,inserted,2000);expect(n.clips.map(c=>[c.startMs,clipDuration(c)])).toEqual([[0,2000],[2000,1000],[3000,8000]]);expect(n.voices[0]?.startMs).toBe(4000);expect(n.sfx[0]?.startMs).toBe(5000);expect(timelineDuration(n)).toBe(11000)})
 it('transfers an effect to narration without duplication or overwriting',()=>{const t=original(),effect=sourceClip({...source,durationMs:500},2000,true);t.sfx=[effect];const n=transferClip(t,effect.id,'main',1000);expect(n.sfx).toEqual([]);expect(n.clips.filter(c=>c.id===effect.id)).toHaveLength(1);expect(timelineDuration(n)).toBe(10500);expect(t.clips).toHaveLength(1)})
 it('replaces a converted passage while preserving later timing, tags, and effects',()=>{let t=assignVoice(original(),2000,4000,{id:'v',name:'V'});t.sfx=[sourceClip({...source,durationMs:500},7000,true)];const n=replaceRange(t,2000,4000,sourceClip({...source,id:'converted',durationMs:2000}));expect(n.clips.map(c=>[c.startMs,clipEnd(c)])).toEqual([[0,2000],[2000,4000],[4000,10000]]);expect(n.voices).toEqual(t.voices);expect(n.sfx).toEqual(t.sfx);expect(n.clips[2]?.sourceStartMs).toBe(4000)})
 it('chooses fully selected clips for Join and honours a selected clip before the playhead',()=>{const t=splitAt(splitAt(original(),2000),4000);expect(joinCandidateIds(t,[0,4000],0,null)).toEqual(t.clips.slice(0,2).map(c=>c.id));expect(joinCandidateIds(t,null,0,t.clips[1]!.id)).toEqual(t.clips.slice(1).map(c=>c.id))})
})


describe('Voice production completion', () => {
 const completed = () => { const t=assignVoice(original(),0,10000,{id:'narrator',name:'Narrator'}); const v=t.voices[0]!;v.production={status:'converted',voiceId:v.voiceId,audioKey:voiceAudioKey(t,v)}; return t }
 it('keeps completion after splitting, changing gain or adding fades and effects', () => { const t=splitAt(completed(),5000); t.clips[0]!.gainDb=-12;t.clips[0]!.fadeInMs=250;t.sfx.push(sourceClip({...source,id:'effect'},0,true));expect(voiceComplete(t,t.voices[0]!)).toBe(true) })
 it('invalidates completion when source audio changes or a different voice is assigned', () => { const t=completed();t.clips[0]!.sourceId='new-recording';expect(reconcileVoiceProduction(t).voices[0]!.production).toBeUndefined();const next=completed();next.voices[0]!.voiceId='another-voice';expect(voiceComplete(next,next.voices[0]!)).toBe(false) })
 it('keeps completed regions after moving a whole passage without changing its content', () => { const t=completed();t.clips[0]!.startMs=1000;t.voices[0]!.startMs=1000;t.voices[0]!.endMs=11000;expect(voiceComplete(t,t.voices[0]!)).toBe(true) })
 it('includes reference video placement in playback duration', () => {expect(projectDuration({schemaVersion:1,id:'p',name:'P',revision:0,updatedAtMs:0,sources:[],timeline:blankTimeline(),videos:[{id:'v',name:'Video',startMs:1000,durationMs:2000}]})).toBe(3000)})
})


describe('overlapping effects and multi-fragment selection',()=>{
 it('keeps coincident effects visible and reuses a free row',()=>{const a=sourceClip({...source,durationMs:500},0,true),b=sourceClip({...source,durationMs:1000},0,true),c=sourceClip({...source,durationMs:200},500,true);expect(packSfxRows([c,b,a]).map(row=>row.map(clip=>clip.id))).toEqual([[b.id],[a.id,c.id]]);expect(packSfxRows([a,b,c]).flat()).toHaveLength(3)})
 it('joins only explicitly selected fragments rather than everything in the time range',()=>{const t=splitAt(splitAt(original(),2000),4000),ids=[t.clips[0]!.id,t.clips[2]!.id];expect(joinCandidateIds(t,[0,10000],0,null,ids)).toEqual(ids)})
})

describe('saved voice versions',()=>{
 const alex={id:'alex',name:'Alex'},john={id:'john',name:'John'}
 function accept(before:ReturnType<typeof original>,voiceSource:string) {
  const v=before.voices[0]!
  let after=replaceRange(before,v.startMs,v.endMs,sourceClip({...source,id:voiceSource,durationMs:v.endMs-v.startMs}))
  after={...after,voices:after.voices.map(r=>({...r,production:{status:'converted' as const,voiceId:r.voiceId,audioKey:voiceAudioKey(after,r)}}))}
  return rememberVoiceVersions(before,after,new Set([v.id]))
 }
 it('restores the first Alex result after John, including its original effect settings',()=>{
  const baseline=assignVoice(original(),0,10000,alex),a=accept(baseline,'alex-first')
  a.clips[0]!.gainDb=12 // Later changes do not modify the accepted snapshot.
  const pending=assignVoice(a,0,10000,john)
  expect(pending.clips[0]!.sourceId).toBe(source.id)
  const j=accept(pending,'john-first'),back=assignVoice(j,0,10000,alex)
  expect(back.clips[0]!.sourceId).toBe('alex-first');expect(back.clips[0]!.gainDb).toBe(0)
  expect(back.voices[0]!.audio!.original[0]!.sourceId).toBe(source.id)
  expect(voiceComplete(back,back.voices[0]!)).toBe(true)
  const restored=restoreVoiceOriginal(back,back.voices[0]!.id)
  expect(restored.clips[0]!.sourceId).toBe(source.id);expect(restored.voices[0]!.production).toBeUndefined()
  expect(assignVoice(restored,0,10000,alex).clips[0]!.sourceId).toBe('alex-first')
 })
 it('persists versions through serialization and Undo/Redo',()=>{
  const a=accept(assignVoice(original(),0,10000,alex),'alex-first'),j=accept(assignVoice(a,0,10000,john),'john-first')
  const reloaded=JSON.parse(JSON.stringify(j)),back=assignVoice(reloaded,0,10000,alex)
  const h=historyEdit({past:[],present:reloaded,future:[]},back)
  expect(historyUndo(h).present).toEqual(reloaded);expect(historyRedo(historyUndo(h)).present).toEqual(back)
  expect(back.clips[0]!.sourceId).toBe('alex-first')
 })
 it('starts a partial retag from original audio and preserves adjacent versions',()=>{
  const a=accept(assignVoice(original(),0,10000,alex),'alex-first'),j=assignVoice(a,2000,4000,john)
  expect(j.clips.find(c=>c.startMs===2000)!.sourceId).toBe(source.id)
  const back=assignVoice(j,2000,4000,alex)
  expect(back.clips.filter(c=>c.sourceId===source.id)).toHaveLength(0)
  expect(back.clips.find(c=>c.startMs===2000)!.sourceId).toBe('alex-first')
  expect(back.voices.every(v=>v.audio)).toBe(true)
 })
 it('invalidates snapshots after changing source content but preserves them after a split',()=>{
  const a=accept(assignVoice(original(),0,10000,alex),'alex-first')
  expect(reconcileVoiceProduction(splitAt(a,3000)).voices[0]!.audio).toBeDefined()
  a.clips[0]!.sourceId='replacement'
  expect(reconcileVoiceProduction(a).voices[0]!.audio).toBeUndefined()
 })
 it('recovers legacy converted projects from their saved original timeline',()=>{
  const baseline=assignVoice(original(),0,10000,alex),a=accept(baseline,'alex-first')
  delete a.voices[0]!.audio
  const recovered=recoverVoiceVersions(a,baseline)
  expect(assignVoice(recovered,0,10000,john).clips[0]!.sourceId).toBe(source.id)
 })
 it('retains the initial generated voice when reassigned',()=>{
  const speech=assignVoice(original(),0,10000,alex),v=speech.voices[0]!
  v.production={status:'generated',voiceId:v.voiceId,audioKey:voiceAudioKey(speech,v)}
  const saved=rememberVoiceVersions(speech,speech,new Set([v.id])),j=accept(assignVoice(saved,0,10000,john),'john')
  const back=assignVoice(j,0,10000,alex)
  expect(back.clips[0]!.sourceId).toBe(source.id);expect(back.voices[0]!.production!.status).toBe('generated')
 })
})

describe('video timeline placement and frames',()=>{
 const videos=[{id:'a',name:'First',startMs:1000,durationMs:2000},{id:'b',name:'Second',startMs:5000,durationMs:1000}]
 it('uses a whole free interval and permits touching boundaries',()=>{expect(placeVideo(videos,1000,0)).toBe(0);expect(placeVideo(videos,2500,0)).toBe(6000);expect(placeVideo(videos,2000,3000)).toBe(3000)})
 it('excludes the moved clip and rejects overflow or invalid positions',()=>{expect(placeVideo(videos,2000,1500,'a')).toBe(1500);expect(()=>placeVideo(videos,1000,86400000)).toThrow();expect(()=>placeVideo(videos,1000,NaN)).toThrow()})
 it('resolves black, playing, held and backward-seek frames independent of selection',()=>{expect(videoFrame(videos,0)).toBeNull();expect(videoFrame(videos,1500)).toMatchObject({video:{id:'a'},timeMs:500,held:false});expect(videoFrame(videos,4000)).toMatchObject({video:{id:'a'},timeMs:1999,held:true});expect(videoFrame(videos,5000)).toMatchObject({video:{id:'b'},timeMs:0,held:false});expect(videoFrame(videos,9000)).toMatchObject({video:{id:'b'},timeMs:999,held:true});expect(videoFrame(videos,1000)?.video.id).toBe('a')})
 it('arranges legacy overlaps explicitly while preserving input',()=>{const old=[videos[0]!,{...videos[1]!,startMs:2000}];expect(videosOverlap(old)).toBe(true);const next=arrangeVideos(old);expect(next[1]?.startMs).toBe(3000);expect(videosOverlap(next)).toBe(false);expect(old[1]?.startMs).toBe(2000)})
})

describe('independent soundtrack edits',()=>{
 const fixture=()=>({...original(),sfx:[{...sourceClip(source,1000),speed:2,fadeInMs:2000,fadeOutMs:2000},sourceClip(source,2000)]})
 it('splits only the selected soundtrack, with source offsets at its speed',()=>{const t=fixture(),c=t.sfx[0]!,n=splitSoundtrack(t,c.id,2000);expect(n.clips).toEqual(t.clips);expect(n.sfx.slice(0,2).map(c=>[c.startMs,c.sourceStartMs,c.sourceEndMs])).toEqual([[1000,0,2000],[2000,2000,10000]]);expect(n.sfx[2]).toEqual(t.sfx[1]);expect(n.sfx[0]?.fadeInMs).toBe(1000);expect(splitSoundtrack(t,c.id,1000).sfx).toEqual(t.sfx);const h=historyEdit({past:[],present:t,future:[]},n);expect(historyUndo(h).present).toEqual(t);expect(historyRedo(historyUndo(h)).present).toEqual(n)})
 it('trims and restores source ranges without moving other fragments',()=>{const t=fixture(),id=t.sfx[0]!.id,n=trimSoundtrack(t,id,'start',2000,10000);expect(n.sfx[0]).toMatchObject({startMs:2000,sourceStartMs:2000,sourceEndMs:10000});expect(clipEnd(n.sfx[0]!)).toBe(6000);expect(trimSoundtrack(n,id,'start',0,10000).sfx[0]?.startMs).toBe(1000);expect(n.sfx[1]).toEqual(t.sfx[1]);expect(n.clips).toEqual(t.clips)})
 it('clamps trim edges to source bounds and positive duration and fades',()=>{const t=fixture(),id=t.sfx[0]!.id;expect(trimSoundtrack(t,id,'end',20000,10000).sfx[0]?.sourceEndMs).toBe(10000);const c=trimSoundtrack(t,id,'end',0,10000).sfx[0]!;expect(clipDuration(c)).toBe(1);expect(c.fadeInMs).toBe(1);expect(c.fadeOutMs).toBe(1);expect(clipDuration(trimSoundtrack(t,id,'start',20000,10000).sfx[0]!)).toBe(1)})
})
