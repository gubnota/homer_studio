import { useEffect, useRef, useState } from 'react'
import type { SoundCategory } from '../../shared/contracts'

const kinds: { id: SoundCategory; label: string; description: string }[] = [
  { id: 'sound_effect', label: 'Sound effect', description: 'Fabric, footsteps, ambience, and other sounds' },
  { id: 'speech', label: 'Speech and vocal gestures', description: 'English words with supported Chatterbox Turbo tags' }
]

export function KindPicker({ open, selected, onSelect, onClose }: { open: boolean; selected: SoundCategory; onSelect: (kind: SoundCategory) => void; onClose: () => void }): JSX.Element {
  const dialog = useRef<HTMLDialogElement>(null)
  const search = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')
  useEffect(() => {
    const element = dialog.current
    if (!element) return
    if (open && !element.open) { element.showModal(); setQuery(''); requestAnimationFrame(() => search.current?.focus()) }
    if (!open && element.open) element.close()
  }, [open])
  const matches = kinds.filter((kind) => `${kind.label} ${kind.description}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
  return <dialog className="voice-dialog" ref={dialog} onCancel={onClose} onClose={onClose} aria-labelledby="kind-picker-title">
    <div className="voice-dialog-head"><div><h2 id="kind-picker-title">Choose a kind of audio</h2><p>Pick the local model task for this clip.</p></div><button type="button" onClick={onClose} aria-label="Close kind picker">×</button></div>
    <label>Filter kinds<input ref={search} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search kinds" /></label>
    <div className="voice-dialog-list">{matches.map((kind) => <div className={kind.id === selected ? 'voice-option selected' : 'voice-option'} key={kind.id}><button type="button" onClick={() => { onSelect(kind.id); onClose() }}><strong>{kind.label}</strong><small>{kind.description}</small></button></div>)}{matches.length === 0 && <p>No kinds match your search.</p>}</div>
  </dialog>
}
