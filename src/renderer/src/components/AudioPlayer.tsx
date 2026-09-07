export function AudioPlayer({ projectTitle }: { projectTitle?: string }): JSX.Element {
  return (
    <footer className="player">
      <button aria-label="Play" disabled>▶</button>
      <div className="player-copy"><strong>No audio selected</strong><span>{projectTitle ? `${projectTitle} has no selected take.` : 'Open a project and generate or import a take.'}</span></div>
      <div className="timeline"><span /><small>00:00 / 00:00</small></div>
      <button className="quiet" disabled>1×</button>
    </footer>
  )
}
