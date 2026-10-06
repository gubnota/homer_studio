# Wave Studio

Wave Studio opens by default. Import or drop WAV, MP3, M4A, AAC, FLAC or WebM audio. Later imports offer insertion at a chosen time, append, sound-effect overlay or narration replacement. Projects save automatically in app data independently of chapter projects and Voice Memos.

## Editing
- Drag across narration to select; drag selection handles to refine. Select all uses Cmd+A.
- Record opens native microphone capture; stop, listen and insert into narration. Captures are also retained in Voice Memos. Generate speech queues local TTS, with preview before insertion. Empty projects support both workflows.
- Join combines selected adjacent narration fragments, or the selected fragment and its next neighbour, into an immutable lossless bounce; unrelated SFX are excluded.
- Delete closes the gap; Replace with silence retains timing. S splits at the playhead.
- Insert silence offers 0.25/0.5/1/2 seconds or a custom duration. Select silence to change its length, or drag its right edge.
- Drag a fragment title (or Option-drag) to move it. The insertion preview preserves other narration; Shift explicitly swaps with a fragment.
- Speed supports 0.85–1.20×, including custom values such as 1.08×. Pitch is preserved; duration and annotations follow the edit.
- The inspector adjusts clip gain/fades and SFX trim, placement, gain, mute and duplication. Normalize measures the isolated fragment peak and targets −1 dB, with gain capped at +6 dB; silence reports an error.
- Drag fragment titles vertically between narration and SFX. Moving SFX into narration shifts later audio and annotations; moving narration to SFX leaves silence to preserve timing.

## Voices and sound effects
Voice assigns the selected passage a colored annotation. Edit canonical voice names, colors, notes and model provider in Voices. Assignment itself performs no conversion. Apply tagged voices queues Original Chatterbox conversion, previews each output and replaces narration only after acceptance. Nonoverlapping passages are split into at most 15-second chunks; small duration differences are fitted to the original range, larger differences fail explicitly. Accepted audio is heard in playback and included in export. Restore original voices restores the full timeline saved before the first acceptance. Optional model setup remains an explicit Settings action.

Add SFX opens categories, search and recent assets. The bundled Audience `sitcom_laugh01` defaults to -10 dB on placement; its original remains intact. Intro, outro, rewind, shoosh, riser, suspense, thud, surprised reaction, gong and closing door are also bundled and available in Sound Studio. Choose selection start, center or end/punchline alignment. Transitions snap to nearby clip boundaries. SFX can be imported or copied from saved Sound Studio/Voice Lab clips; the full SFX page manages custom names and categories.

## Playback and navigation
Space plays/pauses; Shift+Space plays the selection; Loop repeats it. Transport buttons jump five seconds. Cmd/Ctrl-wheel zooms at the pointer; ordinary trackpad scrolling pans. +/− zoom, 0 fits the project, and Fit selection focuses a passage. Cmd+Z / Shift+Cmd+Z undo/redo. Sidebar collapse preserves icon tooltips.

Voice Memos, Voice Lab and Sound Studio can open their audio in Wave Studio. Navigation retains current history; restart restores the project, viewport, playhead, selection and Loop state. Switching projects and normal Quit drain pending saves. Export writes a verified WAV, MP3, M4A, AAC or FLAC mix.

## Storage and performance
Project metadata and immutable 48 kHz stereo sources live under app data `wave-studio/`. History retains up to 100 edits in the current session. The renderer draws bounded viewport peaks and decodes ten-second previews, rather than an entire audiobook. Native exports stream through FFmpeg.

Changed-speed clips first prepare a native disk tempo variant; preparation can take time for long recordings and supports cancellation. Subsequent playback and export reuse the same samples, including gain/fade envelopes. Unreferenced tempo variants are evicted when cache exceeds 4 GB; variants needed by the active render can exceed that budget. Sources are retained for safe reuse.

## Verification checklist
- Create empty projects; record in Wave Studio and Voice Memos, stop, preview and insert.
- Import at a selection or entered time; drag SFX into narration; split and Join; normalize and listen.
- Generate a short speech take, preview and accept; tag real speech, convert and accept, then export and compare playback.
- Change project/view/Loop, immediately switch or quit/restart, and verify restoration. Original model inference quality requires listening with real speech; synthetic or silent fixtures do not establish it.

Export opens a format selector: M4A (AAC) or WAV. The selected format controls the file extension and encoder; M4A encodes stereo AAC at 48 kHz. Native queue exports additionally support AAC, MP3 and FLAC.

## Version 0.2.5 workflows
- Select a voice tag and choose Generate for that passage, then preview and accept. Listen plays its current timeline selection. Apply tagged voices processes pending passages; check Regenerate completed only when desired. Voice tints distinguish speakers and tags show Assigned, Generated or Converted. Generated text fragments carry the chosen voice.
- Gain, fade in/out, speed and preview position use styled sliders; number fields retain precise entry. Join is beside Split and also on the main toolbar.
- Projects opens a vertical list with search, explicit Save current, portable Save copy/Open copy and recoverable Delete/Restore. Copies include immutable audio and video, so they can reopen independently.
- Import/drop a video to add a separate reference lane and current-frame preview. Drag it or change Timeline start to align it with narration. References are muted, their audio is removed during import, and audio export contains only narration/SFX.

## 0.2.6 editing and portability
- Double-click selects a fragment; Shift-click adds fragments and Cmd-click toggles them. Join prompts for one resulting voice if tags differ.
- Left/Right select adjacent fragments; Cmd/Ctrl+Left/Right seek fragment bounds; Shift+Left/Right seek five seconds; Shift+Space plays selection. Shortcut help is available in the editor.
- Gain supports −96 to +20 dB with neutral zero at the center. Fade sliders snap to zero; playback speed snaps to 1. Peak normalization can attenuate loud peaks and boost quiet audio within the gain bounds; it does not measure perceived loudness.
- Coincident SFX occupy visible rows. Preview starts on one click. Bottom timeline control adjusts zoom.
- Cancel is available for generation and media preparation. Cancelled managed inference may require starting its worker again.
- Save/import `.wavehs` to transfer media, video references and custom voice samples. Bundles open by drag, File menu or desktop argument.
- Optional browser operation is documented in `LINUX_SERVER.md`. Server projects and models reside on the server.
