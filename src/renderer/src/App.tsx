import { useMemo, useState } from 'react'
import { isRouteId, routes, type RouteId } from '../../shared/navigation'
import { AudioPlayer } from './components/AudioPlayer'
import { StudioLayout } from './components/StudioLayout'
import { ExportsPage, ImportPage, ProjectsPage, QueuePage, SettingsPage, StudioPage } from './pages'

function initialRoute(): RouteId {
  const value = window.location.hash.slice(1)
  return isRouteId(value) ? value : 'projects'
}

export function App(): JSX.Element {
  const [route, setRoute] = useState<RouteId>(initialRoute)
  const page = useMemo(() => {
    switch (route) {
      case 'projects': return <ProjectsPage onImport={() => navigate('import')} />
      case 'import': return <ImportPage />
      case 'queue': return <QueuePage />
      case 'exports': return <ExportsPage />
      case 'settings': return <SettingsPage />
      default: return <StudioPage route={route} />
    }
  }, [route])

  function navigate(next: RouteId): void {
    window.location.hash = next
    setRoute(next)
  }

  return (
    <StudioLayout route={route} routes={routes} onNavigate={navigate}>
      {page}
      <AudioPlayer />
    </StudioLayout>
  )
}
