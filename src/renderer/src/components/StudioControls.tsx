import type { CSSProperties, InputHTMLAttributes, SelectHTMLAttributes } from 'react'
export function Checkbox({className = '', ...props}: Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>): JSX.Element {
 return <input {...props} type="checkbox" role={props.role || (props['aria-label']?.startsWith('Select ') ? undefined : 'switch')} className={`${props['aria-label']?.startsWith('Select ') ? 'studio-checkbox' : 'studio-switch'} ${className}`} />
}
export function Select({className = '', ...props}: SelectHTMLAttributes<HTMLSelectElement>): JSX.Element {
 return <select {...props} className={`studio-select ${className}`} />
}

export function StudioSlider({className='', min=0, max=100, value, defaultValue, ...props}: Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>): JSX.Element {
 const n=Number(value ?? defaultValue ?? min), fill=Math.max(0,Math.min(100,(n-Number(min))/Math.max(1e-9,Number(max)-Number(min))*100))
 return <input {...props} type="range" min={min} max={max} value={value} defaultValue={defaultValue} className={`studio-slider ${className}`} style={{'--slider-fill':`${fill}%`,...props.style} as CSSProperties}/>
}
