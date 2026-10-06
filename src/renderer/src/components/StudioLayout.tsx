import { useState, type PropsWithChildren } from 'react'
import {StudioIcon} from './StudioIcon'
import {StudioToastHost} from './StudioToast'
import type { RouteId, routes } from '../../../shared/navigation'
interface Props extends PropsWithChildren { route: RouteId; routes: typeof routes; onNavigate: (route: RouteId) => void }
export function StudioLayout({ route, routes: items, onNavigate, children }: Props): JSX.Element {
 const [collapsed, setCollapsed] = useState(() => localStorage.getItem('homer.sidebar.collapsed') === 'true')
 const groups = [...new Set(items.map(item => item.group))]
 return <div className={`studio-shell${collapsed ? ' sidebar-collapsed' : ''}${route === 'wave-studio' ? ' wave-shell' : ''}`}><aside className="sidebar"><div className="brand"><img className="brand-mark" src="/app-icon.png" alt="" />{!collapsed && <span>Homer Studio</span>}</div><button className="sidebar-toggle" title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'} aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'} aria-expanded={!collapsed} onClick={() => setCollapsed(v => { localStorage.setItem('homer.sidebar.collapsed', String(!v)); return !v })}><StudioIcon name={collapsed ? "chevron-right" : "chevron-left"}/>{!collapsed && <span>Collapse</span>}</button><nav aria-label="Studio navigation">{groups.map(group => <section className="nav-group" key={group}>{!collapsed && <p>{group}</p>}{items.filter(item => item.group === group).map(item => <button title={item.label} aria-label={item.label} aria-current={route === item.id ? 'page' : undefined} className={route === item.id ? 'active' : ''} key={item.id} onClick={() => onNavigate(item.id)}><span className="nav-icon" aria-hidden="true"><StudioIcon name={item.id}/></span>{!collapsed && item.label}</button>)}</section>)}</nav><div className="local-badge"><span />{!collapsed && 'Local workspace'}</div></aside><main className="workspace">{children}<StudioToastHost/></main></div>
}
