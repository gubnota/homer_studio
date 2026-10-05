import type { InputHTMLAttributes, SelectHTMLAttributes } from 'react'
export function Checkbox({className = '', ...props}: Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>): JSX.Element {
 return <input {...props} type="checkbox" className={`studio-checkbox ${className}`} />
}
export function Select({className = '', ...props}: SelectHTMLAttributes<HTMLSelectElement>): JSX.Element {
 return <select {...props} className={`studio-select ${className}`} />
}
