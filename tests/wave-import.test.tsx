import {renderToStaticMarkup} from 'react-dom/server'
import {describe,it,expect} from 'vitest'
import {WaveAudioImportPanel} from '../src/renderer/src/components/WaveAudioImportPanel'
describe('audio import choices',()=>{
 it('defaults to an independent soundtrack and offers explicit narration changes',()=>{
  const html=renderToStaticMarkup(<WaveAudioImportPanel path="/audio/story.m4a" playheadMs={2000} endMs={10000} busy={false} onAdd={()=>{}} onClose={()=>{}}/>);
  expect(html).toMatch(/checked=""[^>]*\/>Separate soundtrack/);
  for(const text of ['Insert narration','Append narration','Replace narration','original level','Existing narration stays in place','Playhead','End of project','Custom time','Add audio'])expect(html).toContain(text);
  expect(html).toContain('story.m4a');
 })
})

import {vi,beforeEach} from 'vitest'
const storageMocks=vi.hoisted(()=>({desktop:vi.fn(()=>true),save:vi.fn(),create:vi.fn()}))
vi.mock('../src/renderer/src/platform',()=>({isDesktop:storageMocks.desktop,save:storageMocks.save,invoke:vi.fn(),uploadFile:vi.fn(),open:vi.fn()}))
vi.mock('../src/renderer/src/waveStudioNative',()=>({waveApi:{create:storageMocks.create}}))
import {createWavePackage} from '../src/renderer/src/WaveStudioProvider'
describe('required project destination',()=>{
 beforeEach(()=>{vi.clearAllMocks();storageMocks.desktop.mockReturnValue(true)})
 it('cancel creates no project',async()=>{
  storageMocks.save.mockResolvedValue(null)
  expect(await createWavePackage('Narration')).toBeNull()
  expect(storageMocks.create).not.toHaveBeenCalled()
 })
 it('chooses a .wavehs location before creating the project',async()=>{
  storageMocks.save.mockResolvedValue('/projects/Narration')
  storageMocks.create.mockResolvedValue({id:'fixture'})
  await createWavePackage('Narration')
  expect(storageMocks.create).toHaveBeenCalledWith('Narration','/projects/Narration.wavehs')
  expect(storageMocks.save.mock.invocationCallOrder[0]).toBeLessThan(storageMocks.create.mock.invocationCallOrder[0]!)
 })
 it('HTTP delegates package placement to server-owned storage',async()=>{
  storageMocks.desktop.mockReturnValue(false)
  storageMocks.create.mockResolvedValue({id:'fixture'})
  await createWavePackage('Narration')
  expect(storageMocks.save).not.toHaveBeenCalled()
  expect(storageMocks.create).toHaveBeenCalledWith('Narration',null)
 })
})
