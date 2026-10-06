import { useEffect, useMemo, useState } from 'react'
import { isRouteId, routes, type RouteId } from '../../shared/navigation'
import type { ProjectSnapshot } from '../../shared/contracts'
import { StudioLayout } from './components/StudioLayout'
import { EditorPage, ExportsPage, ImportPage, ProjectsPage, QueuePage, ReviewPage, SettingsPage, StudioPage, VoicesPage } from './pages'
import { chooseFolder, errorMessage, hasBackend, projectApi } from './native'
import { SoundStudioPage } from './SoundStudioPage'
import { VoiceLabPage } from './VoiceLabPage'
import { WaveStudioPage } from './WaveStudioPage'
import { SfxPage } from './SfxPage'
import { WaveStudioProvider } from './WaveStudioProvider'
import { VoiceMemosPage } from './VoiceMemosPage'

function initialRoute(): RouteId {
  const value = window.location.hash.slice(1)
  return isRouteId(value) ? value : 'wave-studio'
}

function StudioApp(): JSX.Element {
  const [route, setRoute] = useState<RouteId>(initialRoute)
  const [project, setProject] = useState<ProjectSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    const recent = localStorage.getItem('homer.recentProject')
    if (!recent || !hasBackend()) return
    void projectApi.open(recent).then(setProject).catch(() => localStorage.removeItem('homer.recentProject'))
  }, [])

  useEffect(() => { const update = () => setRoute(initialRoute()); window.addEventListener('hashchange', update); return () => window.removeEventListener('hashchange', update) }, [])

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
      case 'wave-studio': return <WaveStudioPage onNavigate={navigate} />
      case 'sfx': return <SfxPage onBack={() => navigate('wave-studio')} />
      case 'projects': return <ProjectsPage project={project} busy={busy} onImport={() => navigate('import')} onOpen={openExisting} onEdit={() => navigate('editor')} onProjectChange={remember} />
      case 'import': return <ImportPage onCreated={(created) => { remember(created); navigate('projects') }} />
      case 'editor': return <EditorPage project={project} onProjectChange={remember} />
      case 'review': return <ReviewPage project={project} onProjectChange={remember} onOpenQueue={() => navigate('queue')} />
      case 'voices': return <VoicesPage />
      case 'sounds': return <SoundStudioPage />
      case 'voice-memos': return <div className="page"><VoiceMemosPage /></div>
      case 'voice-lab': return <VoiceLabPage />
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
    </StudioLayout>
  )
}

export function App(): JSX.Element { return <WaveStudioProvider><StudioApp /></WaveStudioProvider> }
