import type { PropsWithChildren } from 'react'
import type { RouteId, routes } from '../../../shared/navigation'

interface Props extends PropsWithChildren {
  route: RouteId
  routes: typeof routes
  onNavigate: (route: RouteId) => void
}

export function StudioLayout({ route, routes: items, onNavigate, children }: Props): JSX.Element {
  const groups = [...new Set(items.map((item) => item.group))]
  return (
    <div className="studio-shell">
      <aside className="sidebar">
        <div className="brand"><span className="brand-mark">H</span><span>Homer Studio</span></div>
        <nav aria-label="Studio navigation">
          {groups.map((group) => (
            <section className="nav-group" key={group}>
              <p>{group}</p>
              {items.filter((item) => item.group === group).map((item) => (
                <button className={route === item.id ? 'active' : ''} key={item.id} onClick={() => onNavigate(item.id)}>
                  <span className="nav-dot" />{item.label}
                </button>
              ))}
            </section>
          ))}
        </nav>
        <div className="local-badge"><span />Local workspace</div>
      </aside>
      <main className="workspace">{children}</main>
    </div>
  )
}
