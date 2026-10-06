import { StudioSlider } from './StudioControls'
/** Map the neutral value to the center even when its two ranges are unequal. */
export function NeutralSlider({label,value,min,max,neutral=0,step=1,onChange}:{label:string;value:number;min:number;max:number;neutral?:number;step?:number;onChange:(n:number)=>void}):JSX.Element {
 const position=value<=neutral?(value-neutral)/(neutral-min)*100:(value-neutral)/(max-neutral)*100
 return <StudioSlider aria-label={label} min={-100} max={100} step={1} value={position} onChange={e=>{const p=Number(e.target.value);const raw=Math.abs(p)<=3?neutral:neutral+p/100*(p<0?neutral-min:max-neutral);onChange(Math.max(min,Math.min(max,Math.round(raw/step)*step)))}}/>
}
