# Desktop smoke test

The automated smoke check launches the packaged application executable with isolated configuration/data folders, requires it to stay alive for four seconds, and then closes it cleanly.

For a release candidate, also check these actions manually:

1. Open the app and create a project in a temporary folder.
2. Drag a short Markdown file with two headings onto the manuscript area and confirm its title and text appear.
3. Edit and save one chapter.
4. Open Settings and confirm FFmpeg and FFprobe show their detected paths; exercise **Choose** or **Use detected path** once.
5. Confirm llama.cpp and Ollama CLI status is separate from the Ollama server status.
6. Create a custom voice preset, preview it, select it, and confirm the built-in presets can be reset but not deleted.
7. Generate one chapter with the selected preset, then import or generate audio for the other chapter; listen and approve both.
8. Export the audiobook and play the result.
9. Confirm the timestamp file matches the chapter order.
