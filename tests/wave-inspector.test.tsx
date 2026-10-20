import {expect,it,vi} from 'vitest'
import type {Voice} from '../src/shared/contracts'
import {blankTimeline,sourceClip,voiceAudioKey} from '../src/shared/waveStudio'
import {assignVoice,rememberVoiceVersions,replaceRange,restoreVoiceOriginal} from '../src/shared/waveStudioEdits'

const state=vi.hoisted(()=>({current:null as any}))
vi.mock('react',()=>({useState:(v:unknown)=>[v,vi.fn()],useRef:(v:unknown)=>({current:v}),useEffect:vi.fn()}))
vi.mock('../src/renderer/src/WaveStudioProvider',()=>({useWaveStudio:()=>state.current}))
vi.mock('../src/renderer/src/waveStudioNative',()=>({waveApi:{}}))
vi.mock('../src/renderer/src/native',()=>({errorMessage:(e:Error)=>e.message}))
import {WaveStudioInspector} from '../src/renderer/src/components/WaveStudioInspector'

function elements(node:any):any[]{
 if(!node||typeof node!=='object')return []
 if(Array.isArray(node))return node.flatMap(elements)
 return [node,...elements(node.props?.children)]
}
it('keeps the original through repeated changes in the actual Assigned voice control',()=>{
 const source={id:'original',name:'Recording',durationMs:20000,channels:2 as const}
 const voices=['a','b','c'].map(id=>({id,name:id,color:'#123456'} as Voice))
 let timeline=assignVoice({...blankTimeline(),clips:[sourceClip(source)]},0,20000,voices[0]!)
 const original=timeline.clips[0]!.sourceId
 const edit=vi.fn(next=>{timeline=next})
 for(const [i,voice] of voices.entries()){
  state.current={project:{timeline,sources:[source]},view:{selectedId:timeline.voices[0]!.id},edit}
  const tree=WaveStudioInspector({voices,onManageVoices:vi.fn(),onGenerateVoice:vi.fn(),onListenVoice:vi.fn(),onMatchVoice:vi.fn()})
  const select=elements(tree).find(e=>e.type==='select'&&e.props.value===timeline.voices[0]!.voiceId)
  expect(select).toBeDefined()
  select.props.onChange({target:{value:voice.id}})
  const before=timeline
  timeline=replaceRange(timeline,0,20000,sourceClip({...source,id:`converted-${i}`}))
  timeline={...timeline,voices:timeline.voices.map(v=>({...v,production:{status:'converted' as const,voiceId:v.voiceId,audioKey:voiceAudioKey(timeline,v)}}))}
  timeline=rememberVoiceVersions(before,timeline,new Set(timeline.voices.map(v=>v.id)))
 }
 expect(timeline.voices[0]!.audio!.versions).toHaveLength(3)
 expect(restoreVoiceOriginal(timeline,timeline.voices[0]!.id).clips[0]!.sourceId).toBe(original)
})
