# Desktop smoke test

The automated smoke check launches the packaged application executable with isolated configuration/data folders, requires it to stay alive for four seconds, and then closes it cleanly.

For a release candidate, also check these actions manually:

1. Open the app and create a project in a temporary folder.
2. Drag a short Markdown file with two headings onto the manuscript area and confirm its title and text appear.
3. Edit and save one chapter.
4. Open Settings and confirm FFmpeg and FFprobe are available.
5. Import or generate audio for both chapters, listen, and approve them.
6. Export the audiobook and play the result.
7. Confirm the timestamp file matches the chapter order.
