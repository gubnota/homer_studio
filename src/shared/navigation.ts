export const routes = [
  { id: 'projects', label: 'Projects', group: 'Library' },
  { id: 'import', label: 'Import manuscript', group: 'Library' },
  { id: 'editor', label: 'Chapter editor', group: 'Production' },
  { id: 'review', label: 'Review', group: 'Production' },
  { id: 'voices', label: 'Voices', group: 'Production' },
  { id: 'sounds', label: 'Sound Studio', group: 'Production' },
  { id: 'queue', label: 'Render queue', group: 'Production' },
  { id: 'exports', label: 'Exports', group: 'Output' },
  { id: 'settings', label: 'Settings', group: 'System' }
] as const

export type RouteId = (typeof routes)[number]['id']

export function isRouteId(value: string): value is RouteId {
  return routes.some((route) => route.id === value)
}
