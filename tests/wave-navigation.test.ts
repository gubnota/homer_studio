import {describe,it,expect} from 'vitest'
import {blankTimeline,navigateFragment,sourceClip,videoHolds,type WaveView} from '../src/shared/waveStudio'
const videos=[{id:'v2',name:'Second',startMs:4000,durationMs:2000},{id:'v1',name:'First',startMs:500,durationMs:1000}]
const view=(playheadMs:number,selectedId:string|null=null):WaveView=>({playheadMs,selectedId,offsetMs:0,spanMs:10000,selection:null,loop:false})
describe('fragment navigation',()=>{
 it('uses video boundaries with empty narration, even when soundtracks exist',()=>{
  const timeline=blankTimeline();timeline.sfx=[sourceClip({id:'sfx',name:'Music',durationMs:8000,channels:2})]
  expect(navigateFragment(timeline,videos,view(700),false,true)).toEqual({id:'v1',at:500})
  expect(navigateFragment(timeline,videos,view(700),true,true)).toEqual({id:'v1',at:1500})
  expect(navigateFragment(timeline,videos,view(2000),true,true)).toEqual({id:'v2',at:6000})
  expect(navigateFragment(timeline,videos,view(2000),false,true)).toEqual({id:'v1',at:500})
 })
 it('moves through video fragments and reaches the final end',()=>{
  expect(navigateFragment(blankTimeline(),videos,view(700),true,false)).toEqual({id:'v2',at:4000})
  expect(navigateFragment(blankTimeline(),videos,view(4500,'v2'),true,false)).toEqual({id:'v2',at:6000})
  expect(navigateFragment(blankTimeline(),videos,view(9000),false,false)).toEqual({id:'v2',at:4000})
  expect(navigateFragment(blankTimeline(),[],view(0),true,true)).toBeNull()
 })
 it('preserves narration precedence and selected fragment boundaries',()=>{
  const timeline=blankTimeline();const clip=sourceClip({id:'s',name:'Narration',durationMs:3000,channels:2});timeline.clips=[{...clip,startMs:1000}]
  expect(navigateFragment(timeline,videos,view(500),true,true)).toEqual({id:clip.id,at:4000})
  expect(navigateFragment(timeline,videos,view(8000,clip.id),false,true)).toEqual({id:clip.id,at:1000})
 })
})
describe('held frame coverage',()=>{
 it('covers only gaps and the audio tail, preserving original video intervals',()=>{
  expect(videoHolds(videos,8000).map(h=>[h.video.id,h.startMs,h.endMs])).toEqual([['v1',1500,4000],['v2',6000,8000]])
  expect(videos[1]?.durationMs).toBe(1000)
  expect(videoHolds([{...videos[1]!,startMs:0}],500)).toEqual([])
  expect(videoHolds([],8000)).toEqual([])
 })
})
