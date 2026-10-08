import {renderToStaticMarkup} from 'react-dom/server'
import {beforeEach,describe,expect,it,vi} from 'vitest'
import type {WaveProject} from '../src/shared/waveStudio'
const studio=vi.hoisted(()=>({project:null as WaveProject|null}))
vi.mock('../src/renderer/src/WaveStudioProvider',()=>({useWaveStudio:()=>studio}))
import {WaveExportPanel} from '../src/renderer/src/components/WaveExportPanel'
const render=()=>renderToStaticMarkup(<WaveExportPanel onClose={()=>{}} onJob={()=>{}}/>)
beforeEach(()=>{studio.project={schemaVersion:1,id:'p',name:'Export fixture',revision:0,updatedAtMs:0,sources:[],timeline:{clips:[],sfx:[],voices:[]},videos:[{id:'a',name:'a.mp4',startMs:0,durationMs:1000},{id:'b',name:'b.mp4',startMs:500,durationMs:1000}]}})
describe('export overlap feedback',()=>{
 it('shows the blocking problem and recovery inside the dialog before Save',()=>{
  const html=render()
  expect(html).toContain('role="alert"')
  expect(html).toContain('These video fragments overlap')
  expect(html).toContain('Arrange videos sequentially')
  expect(html).toMatch(/<button class="primary" disabled="">Save M4A/)
 })
 it('allows export after videos no longer overlap',()=>{
  studio.project!.videos![1]!.startMs=1000
  const html=render()
  expect(html).not.toContain('These video fragments overlap')
  expect(html).toContain('<button class="primary">Save M4A')
 })
})
