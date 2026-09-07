import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { RouteId } from '../../shared/navigation'

interface DesktopInfo { platform: string; architecture: string; runtime: string }

function Header({ eyebrow, title, copy, action }: { eyebrow: string; title: string; copy: string; action?: React.ReactNode }): JSX.Element {
  return <header className="page-header"><div><p>{eyebrow}</p><h1>{title}</h1><span>{copy}</span></div>{action}</header>
}

export function ProjectsPage({ onImport }: { onImport: () => void }): JSX.Element {
  return <div className="page"><Header eyebrow="Library" title="Your audiobooks" copy="Local projects stay on this Mac." action={<button className="primary" onClick={onImport}>New project</button>} />
    <section className="empty-card"><div className="empty-icon">Aa</div><h2>Start your first audiobook</h2><p>Import a TXT or Markdown manuscript, review its chapters, then create audio with an installed macOS voice.</p><button className="primary" onClick={onImport}>Import manuscript</button></section>
  </div>
}

export function ImportPage(): JSX.Element {
  return <div className="page"><Header eyebrow="New project" title="Import a manuscript" copy="TXT and Markdown are supported." />
    <div className="split"><section className="panel"><label>Project title<input placeholder="My audiobook" /></label><label>Manuscript<textarea placeholder="Paste text here, or choose a file once project storage is connected." /></label><button className="primary" disabled>Create project</button></section><aside className="panel tip"><h3>Chapter detection</h3><p>Markdown headings become chapters. Plain text can use “Chapter 1” style markers.</p><p className="status">Project storage connects in the next stage.</p></aside></div>
  </div>
}

const labels: Record<Exclude<RouteId, 'projects' | 'import' | 'queue' | 'exports' | 'settings'>, [string, string]> = {
  editor: ['Chapter editor', 'Edit and prepare each spoken segment.'],
  review: ['Review', 'Listen to selected takes and flag corrections.'],
  voices: ['Voices', 'Choose from voices installed on this Mac.']
}

export function StudioPage({ route }: { route: keyof typeof labels }): JSX.Element {
  const [title, copy] = labels[route]
  return <div className="page"><Header eyebrow="Production" title={title} copy={copy} /><section className="panel placeholder"><span>Ready for a project</span><h2>Import a manuscript to begin</h2><p>The interface is active; project and audio operations arrive in the next verified stages.</p></section></div>
}

export function QueuePage(): JSX.Element {
  return <div className="page"><Header eyebrow="Production" title="Render queue" copy="Track local jobs and cancel work safely." /><section className="panel table-empty">No jobs yet.</section></div>
}

export function ExportsPage(): JSX.Element {
  return <div className="page"><Header eyebrow="Output" title="Exports" copy="Combine approved chapters and create measured timestamps." /><section className="panel placeholder"><span>Final assembly</span><h2>No complete project</h2><p>Exports appear here after every chapter has a current audio take.</p></section></div>
}

export function SettingsPage(): JSX.Element {
  const [desktop, setDesktop] = useState<DesktopInfo>({ platform: 'macOS', architecture: 'arm64', runtime: 'Browser preview' })
  useEffect(() => {
    if ('__TAURI_INTERNALS__' in window) void invoke<DesktopInfo>('desktop_info').then(setDesktop)
  }, [])
  return <div className="page"><Header eyebrow="System" title="Settings" copy="Configure local tools and model providers." /><div className="settings-grid"><section className="panel"><h2>Desktop runtime</h2><dl><div><dt>Platform</dt><dd>{desktop.platform}</dd></div><div><dt>Architecture</dt><dd>{desktop.architecture}</dd></div><div><dt>Runtime</dt><dd>{desktop.runtime}</dd></div></dl></section><section className="panel"><h2>Local tools</h2><p>FFmpeg, llama.cpp, and Ollama diagnostics connect in a later stage.</p><span className="status">No network account required</span></section></div></div>
}
