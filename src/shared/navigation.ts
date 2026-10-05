export const routes = [
  { id: 'wave-studio', label: 'Wave Studio', group: 'Studio' },
  { id: 'voices', label: 'Voices', group: 'Studio' },
  { id: 'sfx', label: 'SFX', group: 'Studio' },
  { id: 'projects', label: 'Projects', group: 'Studio' },
  { id: 'settings', label: 'Settings', group: 'Studio' },
  { id: 'import', label: 'Import manuscript', group: 'Library' },
  { id: 'editor', label: 'Chapter editor', group: 'Production' },
  { id: 'review', label: 'Review', group: 'Production' },
  { id: 'sounds', label: 'Sound Studio', group: 'Production' },
  { id: 'voice-memos', label: 'Voice Memos', group: 'Production' },
  { id: 'voice-lab', label: 'Voice Lab', group: 'Production' },
  { id: 'queue', label: 'Render queue', group: 'Production' },
  { id: 'exports', label: 'Exports', group: 'Output' },
] as const

export type RouteId = (typeof routes)[number]['id']

export function isRouteId(value: string): value is RouteId {
  return routes.some((route) => route.id === value)
}
