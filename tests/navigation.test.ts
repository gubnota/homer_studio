import { describe, expect, it } from 'vitest'
import { isRouteId, routes } from '../src/shared/navigation'

describe('studio navigation', () => {
  it('keeps route identifiers unique', () => {
    expect(new Set(routes.map((route) => route.id)).size).toBe(routes.length)
  })

  it('validates only known routes', () => {
    expect(isRouteId('projects')).toBe(true)
    expect(isRouteId('unknown')).toBe(false)
  })
})
