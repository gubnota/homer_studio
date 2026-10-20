# Implementation Plan: Required .wavehs project audio storage

## Goal
Require a working `.wavehs` folder for Wave Studio projects and store their immutable audio in its `media/` directory. Stop duplicating new project audio in Application Support `wave-studio/assets`. Safely move existing project audio into packages without losing shared sources or inactive voice takes.

## Existing behavior
- Projects start as app-data JSON; importing audio can run before project creation.
- `wave_store::source_path` resolves every Wave source to global `assets/<UUID>.wav`. Rendering also directly resolves that layout.
- Save as folder copies media and binds incremental autosave, but playback, processing and imports continue using global assets. Portable imports also copy into global storage.
- Folder bindings and source metadata are already app-owned; exported manifests use relative media paths. Voice originals and inactive versions can retain sources absent from the active timeline. SFX can share sources across projects.
- Desktop and HTTP use the same core commands. HTTP filesystem access is scoped to server data/resources.

## Proposed approach
- On desktop, choose a `.wavehs` destination before creating a project or starting its first import/recording/generation. Cancel creates no project or project media. Existing packages open directly; compressed archives require a new destination folder for extraction. On HTTP, create the working package under the server's owned project directory.
- Make package `project.json` authoritative, with atomic revisioned saves. Keep only small source-location metadata, folder bindings, recents and disposable caches in app data. Preserve schema 1, legacy archives and current linked-video options.
- Extend `wave_create` with a destination, `wave_import` with a required project ID, and archive opening with an extraction destination. Validate bindings in the backend; unbound projects cannot create audio implicitly. Return clear destination/missing-folder errors rather than reverting to global assets.
- Resolve source IDs through app-owned metadata to validated package paths. Copy a shared/library source into the destination package before use; never depend on another project's folder for playback. Handle imported ID collisions through the existing remapping machinery. Do not trust absolute paths supplied by manifests.
- Write imported, joined, generated and converted Wave audio directly to package media using staging plus atomic publish. Keep temporary render/cache files separate from durable audio. Recording insertion copies the accepted memo into the package; standalone memo and voice libraries retain their current storage.
- Migrate legacy projects when opened or saved: choose a destination, include all source references (timeline, originals, inactive voice versions and outstanding results), copy and verify audio, publish the package, then switch the binding. Cancel/failure leaves the legacy project usable.
- Reclaim legacy `assets` files only after verified migration and a reference check across active/deleted projects, reusable SFX and processing results. Retain files still needed elsewhere; report migrated/reclaimed sizes. No blanket directory deletion. Removing a project from recents must not silently delete an external package.
- Missing/moved/read-only folders require locating the package or choosing a new destination. Save as copies to a new package and switches the active binding only after successful publication.

## File changes
- `crates/homer-core/src/services/wave_store.rs` — modify: required package creation, source ownership/resolution, package saves, migration, safe reclamation and recents/trash behavior; colocated persistence tests.
- `crates/homer-core/src/services/wave_bundle.rs` — modify: extract archives to selected packages; open folders without global audio duplication; preserve verified streaming, rollback and ID remapping.
- `crates/homer-core/src/services/wave_render.rs` — modify: separate source resolution from cache storage; remove direct global-assets assumptions; colocated render tests.
- `crates/homer-core/src/services/wave_processing.rs` — modify: direct joined/generated/converted audio into the originating package and retain cancellation/revision checks.
- `crates/homer-core/src/services/sfx_store.rs` — modify: preserve library media independently and copy effects into their destination project; include library references in reclamation checks.
- `crates/homer-core/src/services/wave_video.rs` — modify: pass package-aware audio resolution to video export while preserving current linked/embedded video behavior.
- `crates/homer-core/src/commands/wave_studio.rs` — modify: enforce destination/project ownership at command boundaries, expose migration status and resolve audio for preview, URLs, peaks and export.
- `server/src/api.rs` — modify: adapt shared command arguments, choose server-owned package destinations and retain filesystem scope/authentication checks.
- `src/renderer/src/waveStudioNative.ts` — modify: typed destination/project arguments and migration/location operations.
- `src/renderer/src/WaveStudioProvider.tsx` — modify: destination-first creation and imports, legacy migration, archive extraction and missing-folder recovery with cancellation preserving the current project.
- `src/renderer/src/components/WaveProjectsPanel.tsx` — modify: mandatory package creation/save messaging, location recovery and migration/reclaimed-size feedback; distinguish recents deletion from package deletion.
- `tests/wave-import.test.tsx` — modify: destination cancellation, creation before media work and scoped imports.
- `scripts/test-server.py` — modify: package-backed create/import fixtures and real-media persistence, migration, shared-reference and portable round-trip checks.
- `scripts/smoke-app.mjs` — modify: assert document audio resides in package media and global assets do not grow.
- `tests/DESKTOP_SMOKE.md` — modify: package destination, cancellation, recording, moved-folder and migration checks.
- `docs/PROJECT_CONTEXT.md`, `docs/ARCHITECTURE_INDEX.md`, `docs/MODULE_OWNERSHIP.md`, `docs/API_CONTRACTS.md`, `docs/DECISIONS.md`, `docs/TASK_LOG.md` — modify: final storage ownership, command contracts, migration guarantees and verification evidence.

## Implementation steps
1. Implement validated package bindings and source-location metadata in `wave_store.rs`; add filesystem fixtures proving atomic save, traversal rejection and missing-folder behavior.
2. Add required package creation and legacy migration, including complete voice/source reference collection and conservative global-media reclamation. Test shared sources, trash, cancellation, failed copy and restart.
3. Update command, processing, SFX, render and video callers to resolve package media and publish durable outputs there. Test mixing/tempo/export after moving package location and ensure processing cancellation leaves no published partial output.
4. Update folder/archive imports and save-copy publication. Test legacy formats, corrupt input, duplicate/colliding IDs, inactive voice versions and media checksums without writing global Wave assets.
5. Update the typed bridge/provider/project dialog and HTTP adapter. Desktop cancellation must precede imports; HTTP packages stay within server-owned storage. Existing projects migrate before new durable audio is created.
6. Update regression/integration/smoke fixtures, then update the six compact memory documents with final behavior and measured results.

## Verification
- `bun run build` (typecheck, renderer tests, production build).
- `cargo test -p homer-core --features server` and `cargo test -p homer-server`.
- `cargo test --manifest-path src-tauri/Cargo.toml`.
- `cargo build -p homer-server --release`, then `python3 scripts/test-server.py` with isolated data; compare audio checksums/durations before and after migration and portable import.
- `bun run pack:mac`, `node scripts/smoke-app.mjs` and `node scripts/smoke-app.mjs --documents` using isolated packages.
- Desktop: New/first import Cancel; folder and archive open; save/restart; accepted recording, join and voice processing; moved/read-only folder recovery. If local voice models are unavailable, verify storage plumbing with fixtures and report the inference limitation.
- Measure global `assets` before/after repeated project imports/processing: no new Wave project WAVs there. Migration must reduce eligible global bytes while preserving all referenced audio.

## Risks / edge cases
- Shared SFX and inactive voice versions make deletion unsafe without a full reference check. Unknown/unreadable metadata prevents reclamation.
- Package folders can move, lose permissions or run out of disk space; publish/binding changes must remain transactional.
- Identical IDs in independently copied packages require collision handling; an external folder must not overwrite another registered project silently.
- Durable media can still grow inside packages as editing retains original/inactive takes; this change relocates ownership, not automatic loss of edit history.

## Out of scope
- Bulk deletion of existing user data before verified migration; changing standalone memo/model/voice library storage; changing video inclusion defaults; version bump, release/tag/push or replacement of the installed application.

Status: Approved and implemented. Renderer/core/server/desktop checks, real-media integration and isolated packaged startup/document tests passed. Native microphone, live model quality and read-only destination UX remain manual follow-up.
