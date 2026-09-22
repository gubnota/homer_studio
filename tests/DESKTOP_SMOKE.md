# Desktop smoke test

The automated smoke check launches the packaged application executable with isolated configuration/data folders, requires it to stay alive for four seconds, and then closes it cleanly.

For a release candidate, also check these actions manually:

1. Open the app and create a project in a temporary folder.
2. Drag a short Markdown file with two headings onto the manuscript area and confirm its title and text appear.
3. Edit and save one chapter.
4. Open Settings and confirm FFmpeg and FFprobe show their detected paths; exercise **Choose** or **Use detected path** once.
5. Confirm llama.cpp and Ollama CLI status is separate from the Ollama server status.
6. Start Chatterbox, create a custom voice, import a clear 6–20 second spoken sample, record another with the microphone, preview and select a sample, and verify microphone denial is handled. Search for the voice in the modal picker. Confirm the built-in model voice remains available.
7. In Review, use **Generate pending chapters** on both chapters. Open Render queue and check aggregate progress, chapter names in the log, and Cancel. Reopen Review while the job is running and confirm it refreshes after completion. Completed chapters should remain playable after cancellation; rerun pending chapters to finish the book. Select a ready chapter and use **(Re)Generate selected**, confirming replacement before it starts. Import audio into a chapter and confirm the pending action leaves it alone while selected regeneration can replace it.
8. Export the audiobook and play the result.
9. Confirm the timestamp file matches the chapter order.
10. With no project open, visit Sound Studio. Confirm the Chatterbox worker shows unavailable when stopped and generation is disabled.
11. After configuring the local Chatterbox checkpoint and starting its worker, generate a short English speech clip with a supported inline gesture. Listen, retry, export WAV/M4A, and reopen the app to confirm the clip library persists.

12. In Review, zoom and pan the waveform, seek by clicking, select a range, and use Play/Pause and Stop. Save and delete selected generated chapter audio; confirm imported audio is not removed.
13. In Exports and Sound Studio, select/deselect all, save selected files, delete selected/all, and confirm exported copies remain available.
14. In Voice Lab, record or import a longer clip, convert it with Original Chatterbox, play the result, and save/delete it. Cancel a conversion mid-job and check that no incomplete clip appears.
15. Preview the built-in voice with the bundled recording. For a custom voice, verify the copied selected sample plays when a generated preview is unavailable.
16. Generate longer speech, verify its final duration, and listen at each joined segment boundary.
