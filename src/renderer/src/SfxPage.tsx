import { WaveStudioLibraryDrawer } from './components/WaveStudioLibraryDrawer'
export function SfxPage({ onBack }: { onBack: () => void }): JSX.Element { return <div className="page wave-sfx-page"><WaveStudioLibraryDrawer full onClose={onBack} /></div> }
