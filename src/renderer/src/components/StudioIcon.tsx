import type { SVGProps } from 'react'
const paths: Record<string, string> = {
 'wave-studio':'M3 10v4m3-7v10m3-13v16m3-12v8m3-10v12m3-9v6m3-5v4',
 'voice-memos':'M4 10v4m4-7v10m4-13v16m4-13v10m4-7v4',
 voices:'M16 8a4 4 0 1 1-8 0 4 4 0 0 1 8 0M4 21v-2a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v2',
 sfx:'M9 18V5l11-2v13M9 18a3 3 0 1 1-3-3h3m11 1a3 3 0 1 1-3-3h3M9 9l11-2',
 sounds:'M9 18V5l11-2v13M9 18a3 3 0 1 1-3-3h3m11 1a3 3 0 1 1-3-3h3',
 projects:'M3 7V4h7l2 3h9v13H3z', import:'M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5',
 exports:'M12 16V3m-5 5 5-5 5 5M4 16v5h16v-5',
 settings:'M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1zM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0',
 editor:'m4 16 12-12 4 4L8 20H4zM13 7l4 4', review:'m4 12 5 5L20 6',
 queue:'M8 6h12M8 12h12M8 18h12M3 6h1M3 12h1M3 18h1',
 'voice-lab':'M9 3h6M10 3v7L4 20h16l-6-10V3M7 15h10',
 close:'m6 6 12 12M6 18 18 6', 'chevron-left':'m15 5-7 7 7 7', 'chevron-right':'m9 5 7 7-7 7',
 microphone:'M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0zM5 10v2a7 7 0 0 0 14 0v-2M12 19v3m-4 0h8',
 play:'m8 4 12 8-12 8z', document:'M6 3h8l4 4v14H6zM14 3v5h4M9 12h6m-6 4h6'
}
export function StudioIcon({name,...props}: SVGProps<SVGSVGElement> & {name:string}):JSX.Element {
 return <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}><path d={paths[name] || paths.document}/></svg>
}
