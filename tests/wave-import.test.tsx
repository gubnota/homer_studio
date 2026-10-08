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
