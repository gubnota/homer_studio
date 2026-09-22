import { describe, expect, it } from 'vitest'
import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import type { Chapter, ProjectSnapshot } from '../src/shared/contracts'
import { pendingBatchChapterIds, ReviewPage, selectedBatchChapterIds } from '../src/renderer/src/pages'

function chapter(id: string, audioPath: string | null, audioStale = false, audioOrigin: string | null = null, text = 'Read this.'): Chapter {
  return {
    id, title: id, order: Number(id.slice(1)), sourcePath: `chapters/${id}/source.md`, processedPath: null,
    sourceText: text, processedText: null, segments: [], audioPath, audioStale, audioDurationMs: audioPath ? 1000 : null,
    audioOrigin, reviewStatus: null, cues: []
  }
}

const project: ProjectSnapshot = {
  schemaVersion: 1, id: 'project', title: 'Batch book', revision: 1, createdAtMs: 0, updatedAtMs: 0,
  rootPath: '/tmp/batch-book', exports: [], chapters: [
    chapter('c0', null),
    chapter('c1', 'chapters/c1/audio.m4a', true, 'generated'),
    chapter('c2', 'chapters/c2/audio.m4a', false, 'generated'),
    chapter('c3', 'chapters/c3/imported.m4a', false, 'imported'),
    chapter('c4', null, false, null, '   ')
  ]
}

describe('Review batch narration', () => {
  it('fills missing and stale generated chapters without replacing current or imported audio', () => {
    expect(pendingBatchChapterIds(project)).toEqual(['c0', 'c1'])
  })

  it('regenerates every selected chapter with text, in project order', () => {
    expect(selectedBatchChapterIds(project, ['c3', 'c2', 'c0', 'c4'])).toEqual(['c0', 'c2', 'c3'])
  })

  it('shows both batch actions and selection for chapters without audio', () => {
    const html = renderToStaticMarkup(createElement(ReviewPage, { project, onProjectChange: () => {}, onOpenQueue: () => {} }))
    expect(html).toContain('Generate pending chapters · 2')
    expect(html).toContain('(Re)Generate selected · 0')
    expect(html).toContain('aria-label="Select c0"')
    expect(html).not.toContain('aria-label="Select c0" disabled')
  })
})
