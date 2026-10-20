import {beforeEach,afterEach,expect,it,vi} from 'vitest'
import type {Voice} from '../src/shared/contracts'
import {blankTimeline,sourceClip,type WaveProject} from '../src/shared/waveStudio'
import {assignVoice} from '../src/shared/waveStudioEdits'
const hooks=vi.hoisted(()=>({slots:[] as any[],index:0,effects:[] as (()=>void)[]}))
const state=vi.hoisted(()=>({current:null as any}))
const api=vi.hoisted(()=>({get:vi.fn(),convertRegions:vi.fn(),matchVoiceVolume:vi.fn(),processingResult:vi.fn(),url:vi.fn()}))
const system=vi.hoisted(()=>({jobs:vi.fn(),controlJob:vi.fn()}))
vi.mock('react',()=>({
 useRef:(v:unknown)=>{const i=hooks.index++;return hooks.slots[i]??=({current:v})},
 useState:(v:unknown)=>{const i=hooks.index++;if(!(i in hooks.slots))hooks.slots[i]=v;return [hooks.slots[i],(next:unknown)=>{hooks.slots[i]=next}]},
 useEffect:(effect:()=>void,deps:unknown[])=>{const i=hooks.index++,old=hooks.slots[i];if(!old||deps.some((v,j)=>v!==old[j])){hooks.slots[i]=deps;hooks.effects.push(effect)}}
}))
vi.mock('../src/renderer/src/WaveStudioProvider',()=>({useWaveStudio:()=>state.current}))
vi.mock('../src/renderer/src/waveStudioNative',()=>({waveApi:api}))
vi.mock('../src/renderer/src/native',()=>({systemApi:system,errorMessage:(e:Error)=>e.message}))
vi.mock('../src/renderer/src/components/StudioModal',()=>({StudioModal:()=>null}))
vi.mock('../src/renderer/src/components/AudioTransport',()=>({AudioTransport:()=>null}))
import {WaveProductionPanel} from '../src/renderer/src/components/WaveProductionPanel'
function elements(node:any):any[]{if(!node||typeof node!=='object')return [];if(Array.isArray(node))return node.flatMap(elements);return [node,...elements(node.props?.children)]}
const voice={id:'speaker',name:'Speaker',color:'#123456'} as Voice
const source={id:'original',name:'Original',durationMs:20000,channels:2 as const}
let close:ReturnType<typeof vi.fn<()=>void>>
function render(){hooks.index=0;const tree=WaveProductionPanel({kind:'voices',voices:[voice],onClose:close});for(const effect of hooks.effects.splice(0))effect();return elements(tree)}
function button(nodes:any[],text:string){return nodes.find(n=>n.type==='button'&&n.props.children===text)}
const settle=async()=>{for(let i=0;i<20;i++)await Promise.resolve()}
beforeEach(()=>{
 vi.clearAllMocks();vi.useFakeTimers();vi.stubGlobal('window',globalThis);hooks.slots=[];hooks.index=0;hooks.effects=[];close=vi.fn()
 const project:WaveProject={schemaVersion:1,id:'project',name:'Test',revision:1,updatedAtMs:0,sources:[source],timeline:assignVoice({...blankTimeline(),clips:[sourceClip(source)]},0,20000,voice)}
 state.current={project,view:{playheadMs:0},flush:vi.fn(async()=>{}),edit:vi.fn(),rememberOriginal:vi.fn()}
 api.get.mockResolvedValue(project);api.convertRegions.mockResolvedValue('job');api.url.mockImplementation(async id=>`audio:${id}`)
 system.jobs.mockResolvedValue([{id:'job',status:'completed',progress:100}])
 const replacement=(id:string)=>[{startMs:0,endMs:20000,sourceId:id,regionId:project.timeline.voices[0]!.id}]
 api.processingResult.mockResolvedValue({kind:'voices',projectId:'project',revision:1,sources:[{...source,id:'clean'}],replacements:replacement('clean'),warnings:[],withoutCleanup:{sources:[{...source,id:'raw'}],replacements:replacement('raw'),warnings:[]}})
})
afterEach(()=>{vi.clearAllTimers();vi.useRealTimers();vi.unstubAllGlobals()})
async function prepare(){await button(render(),'Generate voice preview').props.onClick();await settle();render();await settle();return render()}
it('switches staged cleanup previews without inference and applies only after explicit acceptance',async()=>{
 let nodes=await prepare();expect(state.current.edit).not.toHaveBeenCalled()
 expect(nodes.some(n=>n.props?.url==='audio:clean')).toBe(true)
 const cleanup=nodes.find(n=>n.props?.checked===true&&n.props?.onChange)
 cleanup.props.onChange({target:{checked:false}});nodes=render()
 expect(nodes.some(n=>n.props?.url==='audio:raw')).toBe(true)
 expect(api.convertRegions).toHaveBeenCalledTimes(1)
 button(nodes,'Accept converted voices').props.onClick()
 expect(state.current.edit.mock.calls[0][0].clips[0].sourceId).toBe('raw')
 expect(state.current.edit.mock.calls[0][0].voices[0].audio.original[0].sourceId).toBe('original')
 expect(close).toHaveBeenCalledOnce()
})
it('rejects staged acceptance after an intervening timeline edit',async()=>{
 await prepare()
 state.current={...state.current,project:{...state.current.project,timeline:{...state.current.project.timeline,clips:state.current.project.timeline.clips.map((c:any)=>({...c,gainDb:-6}))}}}
 button(render(),'Accept converted voices').props.onClick()
 expect(state.current.edit).not.toHaveBeenCalled();expect(close).not.toHaveBeenCalled()
 expect(render().some(n=>n.props?.role==='alert'&&n.props.children.includes('project changed'))).toBe(true)
})
