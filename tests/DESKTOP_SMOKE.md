# Desktop smoke test

The automated smoke check launches the packaged application executable with isolated configuration/data folders, requires it to stay alive for four seconds, and then closes it cleanly.

For a release candidate, also check these actions manually:

1. Open the app and create a project in a temporary folder.
2. Drag a short Markdown file with two headings onto the manuscript area and confirm its title and text appear.
3. Edit and save one chapter.
4. Open Settings and confirm FFmpeg and FFprobe show their detected paths; exercise **Choose** or **Use detected path** once.
5. Confirm llama.cpp and Ollama CLI status is separate from the Ollama server status.
6. Start Chatterbox, create a custom voice, import a clear 6–20 second spoken sample, record another with the microphone, preview and select a sample, and verify microphone denial is handled. Search for the voice in the modal picker. Confirm the built-in model voice remains available.
7. Generate one chapter with the selected neural voice, then import or generate audio for the other chapter; listen and approve both.
8. Export the audiobook and play the result.
9. Confirm the timestamp file matches the chapter order.
10. With no project open, visit Sound Studio. Confirm both workers show unavailable when stopped and generation is disabled.
11. After configuring local model checkpoints and starting each worker, generate a short speech clip and a fabric clip. Set a negative prompt and compare three effect variations. Listen, retry, export WAV/M4A, and reopen the app to confirm the clip library persists. Treat panting as experimental and assess the result by listening.

12. In Review, zoom and pan the waveform, seek by clicking, select a range, and use Play/Pause and Stop. Save and delete selected generated chapter audio; confirm imported audio is not removed.
13. In Exports and Sound Studio, select/deselect all, save selected files, delete selected/all, and confirm exported copies remain available.
14. In Voice Lab, record or import a longer clip, convert it with Original Chatterbox, play the result, and save/delete it. Cancel a conversion mid-job and check that no incomplete clip appears.
15. Preview the built-in voice with the bundled recording. For a custom voice, verify the copied selected sample plays when a generated preview is unavailable.
16. Generate an effect longer than 20 seconds, verify its final duration, and listen at each segment boundary.
