import { useEffect, useMemo, useState } from 'react'
import { isRouteId, routes, type RouteId } from '../../shared/navigation'
import type { ProjectSnapshot } from '../../shared/contracts'
import { AudioPlayer } from './components/AudioPlayer'
import { StudioLayout } from './components/StudioLayout'
import { EditorPage, ExportsPage, ImportPage, ProjectsPage, QueuePage, ReviewPage, SettingsPage, StudioPage, VoicesPage } from './pages'
import { chooseFolder, errorMessage, isDesktop, projectApi } from './native'

function initialRoute(): RouteId {
  const value = window.location.hash.slice(1)
  return isRouteId(value) ? value : 'projects'
}

export function App(): JSX.Element {
  const [route, setRoute] = useState<RouteId>(initialRoute)
  const [project, setProject] = useState<ProjectSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    const recent = localStorage.getItem('homer.recentProject')
    if (!recent || !isDesktop()) return
    void projectApi.open(recent).then(setProject).catch(() => localStorage.removeItem('homer.recentProject'))
  }, [])

  function remember(next: ProjectSnapshot): void {
    setProject(next)
    localStorage.setItem('homer.recentProject', next.rootPath)
  }

  async function openExisting(): Promise<void> {
    const root = await chooseFolder('Open a Homer Studio project')
    if (!root) return
    setBusy(true)
    setError(null)
    try { remember(await projectApi.open(root)) } catch (cause) { setError(errorMessage(cause)) } finally { setBusy(false) }
  }

  const page = useMemo(() => {
    switch (route) {
      case 'projects': return <ProjectsPage project={project} busy={busy} onImport={() => navigate('import')} onOpen={openExisting} onEdit={() => navigate('editor')} onProjectChange={remember} />
      case 'import': return <ImportPage onCreated={(created) => { remember(created); navigate('projects') }} />
      case 'editor': return <EditorPage project={project} onProjectChange={remember} />
      case 'review': return <ReviewPage project={project} onProjectChange={remember} />
      case 'voices': return <VoicesPage />
      case 'queue': return <QueuePage />
      case 'exports': return <ExportsPage project={project} onProjectChange={remember} />
      case 'settings': return <SettingsPage />
      default: return <StudioPage route={route} project={project} />
    }
  }, [route, project, busy])

  function navigate(next: RouteId): void {
    window.location.hash = next
    setRoute(next)
  }

  return (
    <StudioLayout route={route} routes={routes} onNavigate={navigate}>
      {error && <div className="error-banner" role="alert">{error}<button onClick={() => setError(null)}>Dismiss</button></div>}
      {page}
      <AudioPlayer projectTitle={project?.title} />
    </StudioLayout>
  )
}
