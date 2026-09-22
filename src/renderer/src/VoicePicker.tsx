import { useEffect, useRef, useState } from 'react'
import type { Voice } from '../../shared/contracts'

export function VoicePicker({ open, voices, selectedId, onSelect, onClose, onPreview }: {
  open: boolean; voices: Voice[]; selectedId: string; onSelect: (voice: Voice) => void; onClose: () => void; onPreview?: (voice: Voice) => void
}): JSX.Element {
  const dialog = useRef<HTMLDialogElement>(null)
  const search = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')
  useEffect(() => {
    const element = dialog.current
    if (!element) return
    if (open && !element.open) { element.showModal(); setQuery(''); requestAnimationFrame(() => search.current?.focus()) }
    if (!open && element.open) element.close()
  }, [open])
  const matches = voices.filter((voice) => voice.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
  return <dialog className="voice-dialog" ref={dialog} onCancel={onClose} onClose={onClose} aria-labelledby="voice-picker-title">
    <div className="voice-dialog-head"><div><h2 id="voice-picker-title">Choose a voice</h2><p>Chatterbox runs locally. Custom voices use your chosen spoken sample.</p></div><button type="button" onClick={onClose} aria-label="Close voice picker">×</button></div>
    <label>Find a voice<input ref={search} value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === 'ArrowDown') { event.preventDefault(); dialog.current?.querySelector<HTMLButtonElement>('.voice-dialog-list button')?.focus() } }} placeholder="Search voices" /></label>
    <div className="voice-dialog-list">{matches.map((voice) => <div className={voice.id === selectedId ? 'voice-option selected' : 'voice-option'} key={voice.id}><button type="button" disabled={!voice.builtIn && !voice.selectedSampleId} onClick={() => { onSelect(voice); onClose() }}><strong>{voice.name}</strong><small>{voice.builtIn ? 'Model voice' : voice.selectedSampleId ? `${voice.samples.length} sample${voice.samples.length === 1 ? '' : 's'}` : 'Add a sample to use this voice'}</small></button>{onPreview && <button type="button" disabled={!voice.builtIn && !voice.selectedSampleId} onClick={() => onPreview(voice)}>Preview</button>}</div>)}{matches.length === 0 && <p>No voices match your search.</p>}</div>
  </dialog>
}
