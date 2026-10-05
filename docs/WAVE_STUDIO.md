# Wave Studio

Wave Studio opens by default. Import or drop WAV, MP3, M4A, AAC or FLAC narration. Later imports offer append, sound-effect placement or narration replacement. Projects save automatically in app data independently of chapter projects and Voice Memos.

## Editing
- Drag across narration to select; drag selection handles to refine. Select all uses Cmd+A.
- Delete closes the gap; Replace with silence retains timing. S splits at the playhead.
- Insert silence offers 0.25/0.5/1/2 seconds or a custom duration. Select silence to change its length, or drag its right edge.
- Drag a fragment title (or Option-drag) to move it. The insertion preview preserves other narration; Shift explicitly swaps with a fragment.
- Speed supports 0.85–1.20×, including custom values such as 1.08×. Pitch is preserved; duration and annotations follow the edit.
- The inspector adjusts clip gain/fades and SFX trim, placement, gain, mute and duplication.

## Voices and sound effects
Voice assigns the selected passage a colored annotation. Edit canonical voice names, colors, notes and model provider in Voices. Assignment itself performs no conversion; use Voice Lab for actual narrator conversion. Optional model setup remains an explicit Settings action.

Add SFX opens categories, search and recent assets. The bundled Audience `sitcom_laugh01` defaults to -10 dB on placement; its original remains intact. Choose selection start, center or end/punchline alignment. Transitions snap to nearby clip boundaries. SFX can be imported or copied from saved Sound Studio/Voice Lab clips; the full SFX page manages custom names and categories.

## Playback and navigation
Space plays/pauses; Shift+Space plays the selection; Loop repeats it. Transport buttons jump five seconds. Cmd/Ctrl-wheel zooms at the pointer; ordinary trackpad scrolling pans. +/− zoom, 0 fits the project, and Fit selection focuses a passage. Cmd+Z / Shift+Cmd+Z undo/redo. Sidebar collapse preserves icon tooltips.

Voice Memos, Voice Lab and Sound Studio can open their audio in Wave Studio. Navigation retains current history; restart restores the project and viewport. Export writes a verified WAV, MP3, M4A, AAC or FLAC mix.

## Storage and performance
Project metadata and immutable 48 kHz stereo sources live under app data `wave-studio/`. History retains up to 100 edits in the current session. The renderer draws bounded viewport peaks and decodes ten-second previews, rather than an entire audiobook. Native exports stream through FFmpeg.

Changed-speed clips first prepare a native disk tempo variant; preparation can take time for long recordings and supports cancellation. Subsequent playback and export reuse the same samples, including gain/fade envelopes. Unreferenced tempo variants are evicted when cache exceeds 4 GB; variants needed by the active render can exceed that budget. Sources are retained for safe reuse.
