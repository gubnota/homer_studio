import { describe, expect, it } from 'vitest'
import { validateDroppedManuscript } from '../src/renderer/src/native'

describe('manuscript drop validation', () => {
  it('accepts one supported manuscript', () => {
    expect(validateDroppedManuscript(['/tmp/My Book.markdown'])).toBe('/tmp/My Book.markdown')
  })

  it('rejects unsupported and multiple files', () => {
    expect(() => validateDroppedManuscript(['/tmp/book.pdf'])).toThrow('TXT, MD, or Markdown')
    expect(() => validateDroppedManuscript(['/tmp/one.md', '/tmp/two.md'])).toThrow('only one')
  })
})
