// Version 2 losslessly compresses the streamed container; version 1 remains readable.
use super::{project_store::CommandError, settings::Settings, wave_studio::Project};
use crate::runtime::AppHandle;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const MAGIC: &[u8; 8] = b"WAVEHS01";
const COMPRESSED_MAGIC: &[u8; 8] = b"WAVEHS02";
const MAX_TOTAL: u64 = 100 * 1024 * 1024 * 1024;
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn invalid() -> CommandError {
    CommandError::new(
        "INVALID_WAVE_BUNDLE",
        "The project bundle is incomplete, unsafe, or unsupported.",
    )
}
fn io(e: std::io::Error) -> CommandError {
    CommandError::io("Cannot read or write project bundle", e)
}
fn digest(path: &Path) -> Result<[u8; 32], CommandError> {
    let mut input = File::open(path).map_err(io)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = input.read(&mut b).map_err(io)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(h.finalize().into())
}
fn files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), CommandError> {
    for e in fs::read_dir(dir).map_err(io)? {
        let e = e.map_err(io)?;
        let kind = e.file_type().map_err(io)?;
        if kind.is_symlink() {
            return Err(invalid());
        }
        if kind.is_dir() {
            files(root, &e.path(), out)?
        } else if kind.is_file() {
            out.push(
                e.path()
                    .strip_prefix(root)
                    .map_err(|_| invalid())?
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
fn pack(root: &Path, destination: &Path) -> Result<(), CommandError> {
    pack_version(root, destination, true)
}
fn pack_version(root: &Path, destination: &Path, compressed: bool) -> Result<(), CommandError> {
    let mut names = vec![];
    files(root, root, &mut names)?;
    names.sort();
    if names.len() > 20000 {
        return Err(invalid());
    }
    let mut output = File::create(destination).map_err(io)?;
    output
        .write_all(if compressed { COMPRESSED_MAGIC } else { MAGIC })
        .map_err(io)?;
    if compressed {
        let mut encoder = flate2::write::GzEncoder::new(output, flate2::Compression::fast());
        pack_entries(root, names, &mut encoder)?;
        return encoder.finish().map_err(io)?.sync_all().map_err(io);
    }
    pack_entries(root, names, &mut output)?;
    output.sync_all().map_err(io)
}
fn pack_entries(
    root: &Path,
    names: Vec<String>,
    output: &mut dyn Write,
) -> Result<(), CommandError> {
    output
        .write_all(&(names.len() as u32).to_le_bytes())
        .map_err(io)?;
    let mut total: u64 = 0;
    for name in names {
        let path = root.join(&name);
        let size = fs::metadata(&path).map_err(io)?.len();
        total += size;
        if total > MAX_TOTAL {
            return Err(invalid());
        }
        let bytes = name.as_bytes();
        if bytes.len() > 1024 {
            return Err(invalid());
        }
        output
            .write_all(&(bytes.len() as u16).to_le_bytes())
            .map_err(io)?;
        output.write_all(bytes).map_err(io)?;
        output.write_all(&size.to_le_bytes()).map_err(io)?;
        output.write_all(&digest(&path)?).map_err(io)?;
        if std::io::copy(&mut File::open(path).map_err(io)?, &mut *output).map_err(io)? != size {
            return Err(invalid());
        }
    }
    Ok(())
}
#[cfg(test)]
fn unpack(source: &Path, root: &Path) -> Result<(), CommandError> {
    unpack_controlled(source, root, &AtomicBool::new(false), &|_| {})
}
struct ProgressReader<'a> {
    file: File,
    size: u64,
    read: u64,
    cancel: &'a AtomicBool,
    report: &'a dyn Fn(u8),
}
impl Read for ProgressReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("Project opening cancelled."));
        }
        let n = self.file.read(bytes)?;
        self.read += n as u64;
        (self.report)((self.read.saturating_mul(65) / self.size.max(1)).min(65) as u8);
        Ok(n)
    }
}
fn unpack_controlled(
    source: &Path,
    root: &Path,
    cancel: &AtomicBool,
    report: &dyn Fn(u8),
) -> Result<(), CommandError> {
    let file = File::open(source).map_err(io)?;
    let size = file.metadata().map_err(io)?.len();
    let mut input = ProgressReader {
        file,
        size,
        read: 0,
        cancel,
        report,
    };
    let mut magic = [0; 8];
    input.read_exact(&mut magic).map_err(io)?;
    if &magic == COMPRESSED_MAGIC {
        let mut decoder = flate2::bufread::GzDecoder::new(BufReader::new(input));
        unpack_entries(&mut decoder, root, cancel)?;
        let mut input = decoder.into_inner();
        let mut trailing = [0];
        if input.read(&mut trailing).map_err(io)? != 0 {
            return Err(invalid());
        }
        return Ok(());
    }
    if &magic != MAGIC {
        return Err(invalid());
    }
    unpack_entries(&mut input, root, cancel)
}
fn unpack_entries(
    input: &mut dyn Read,
    root: &Path,
    cancel: &AtomicBool,
) -> Result<(), CommandError> {
    let mut n = [0; 4];
    input.read_exact(&mut n).map_err(io)?;
    let count = u32::from_le_bytes(n);
    if count == 0 || count > 20000 {
        return Err(invalid());
    }
    let mut total: u64 = 0;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..count {
        let mut n = [0; 2];
        input.read_exact(&mut n).map_err(io)?;
        let length = u16::from_le_bytes(n) as usize;
        if length == 0 || length > 1024 {
            return Err(invalid());
        }
        let mut bytes = vec![0; length];
        input.read_exact(&mut bytes).map_err(io)?;
        let name = String::from_utf8(bytes).map_err(|_| invalid())?;
        if name.contains('\\')
            || name
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == ".." || s.contains(':'))
            || !seen.insert(name.clone())
        {
            return Err(invalid());
        }
        let mut b = [0; 8];
        input.read_exact(&mut b).map_err(io)?;
        let size = u64::from_le_bytes(b);
        total = total.checked_add(size).ok_or_else(invalid)?;
        if total > MAX_TOTAL {
            return Err(invalid());
        }
        if name.ends_with(".json") && size > 16 * 1024 * 1024 {
            return Err(invalid());
        }
        let mut hash = [0; 32];
        input.read_exact(&mut hash).map_err(io)?;
        let path = root.join(name);
        fs::create_dir_all(path.parent().ok_or_else(invalid)?).map_err(io)?;
        let mut out = File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(io)?;
        let mut remaining = size;
        let mut actual = Sha256::new();
        let mut buffer = [0u8; 65536];
        while remaining > 0 {
            if cancel.load(Ordering::SeqCst) {
                return Err(CommandError::new(
                    "JOB_CANCELLED",
                    "Project opening cancelled.",
                ));
            }
            let length = remaining.min(buffer.len() as u64) as usize;
            let n = input.read(&mut buffer[..length]).map_err(io)?;
            if n == 0 {
                return Err(invalid());
            }
            out.write_all(&buffer[..n]).map_err(io)?;
            actual.update(&buffer[..n]);
            remaining -= n as u64;
        }
        out.sync_all().map_err(io)?;
        if actual.finalize().as_slice() != hash {
            return Err(invalid());
        }
    }
    let mut trailing = [0];
    if input.read(&mut trailing).map_err(io)? != 0 {
        return Err(invalid());
    }
    Ok(())
}
pub fn export(app: &AppHandle, p: &Project, destination: &Path) -> Result<(), CommandError> {
    export_options(app, p, destination, false)
}
pub fn export_options(
    app: &AppHandle,
    p: &Project,
    destination: &Path,
    include_video: bool,
) -> Result<(), CommandError> {
    if destination.extension().and_then(|s| s.to_str()) != Some("wavehs") {
        return Err(CommandError::new(
            "INVALID_DESTINATION",
            "Choose a .wavehs filename.",
        ));
    }
    if destination.exists() {
        return Err(CommandError::new(
            "DESTINATION_EXISTS",
            "Choose a new bundle filename.",
        ));
    }
    let parent = destination.parent().ok_or_else(invalid)?;
    let scratch = Scratch(parent.join(format!(".wavehs-{}", uuid::Uuid::new_v4())));
    fs::create_dir(&scratch.0).map_err(io)?;
    let media = scratch.0.join("project");
    super::wave_store::save_copy_options(app, p, &media, include_video)?;
    super::voice_store::export_portable(app, p, &media)?;
    let temporary = scratch.0.join("bundle");
    pack(&media, &temporary)?;
    fs::rename(temporary, destination).map_err(io)
}
pub fn import(
    app: &AppHandle,
    path: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    report: Arc<dyn Fn(u8) + Send + Sync>,
) -> Result<Project, CommandError> {
    let scratch = Scratch(std::env::temp_dir().join(format!("wavehs-{}", uuid::Uuid::new_v4())));
    fs::create_dir(&scratch.0).map_err(io)?;
    let folder = if path.is_dir() {
        path
    } else {
        unpack_controlled(path, &scratch.0, &cancel, &*report)?;
        &scratch.0
    };
    if cancel.load(Ordering::SeqCst) {
        return Err(CommandError::new(
            "JOB_CANCELLED",
            "Project opening cancelled.",
        ));
    }
    let bytes = fs::read(folder.join("project.json")).map_err(io)?;
    let p: Project = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    super::wave_studio::validate(&p)?;
    if path.is_dir() {
        if let Ok(existing) = super::wave_store::load(app, &p.id) {
            if super::wave_store::bound_folder(app, &p.id)?.as_deref()
                == Some(path.canonicalize().map_err(io)?.as_path())
                && serde_json::to_value(&existing).map_err(|_| invalid())?
                    == serde_json::to_value(&p).map_err(|_| invalid())?
            {
                super::wave_store::validate_sources(app, &existing)?;
                return Ok(existing);
            }
        }
    }
    let old_videos = p.videos.clone();
    let mapping = super::voice_store::import_portable(app, folder, settings)?;
    let imported = super::wave_store::open_bundle_copy(app, folder, settings, cancel, report);
    let mut p = match imported {
        Ok(p) => p,
        Err(error) => {
            for id in mapping.values() {
                let _ = super::voice_store::delete(app, id, super::voice_store::DEFAULT_VOICE);
            }
            return Err(error);
        }
    };
    for timeline in std::iter::once(&mut p.timeline).chain(p.voice_original.iter_mut()) {
        for r in &mut timeline.voices {
            if let Some(audio) = &mut r.audio {
                for version in &mut audio.versions {
                    if let Some(id) = mapping.get(&version.voice_id) {
                        version.production.audio_key =
                            version.production.audio_key.replace(&version.voice_id, id);
                        version.voice_id = id.clone();
                        version.production.voice_id = id.clone();
                    }
                }
            }
            if let Some(id) = mapping.get(&r.voice_id) {
                if let Some(production) = &mut r.production {
                    production.audio_key = production.audio_key.replace(&r.voice_id, id);
                }
                if let Some(production) = &mut r.production {
                    production.voice_id = id.clone();
                }
                r.voice_id = id.clone();
            }
        }
    }
    let result = super::wave_store::save(app, p.clone(), 0);
    if result.is_err() {
        for id in mapping.values() {
            let _ = super::voice_store::delete(app, id, super::voice_store::DEFAULT_VOICE);
        }
        for source in &p.sources {
            if let Ok(path) = super::wave_store::source_path(app, &source.id) {
                let _ = fs::remove_file(path);
            }
        }
        for video in &p.videos {
            if let Ok(path) = super::wave_video::path(app, video.asset_id()) {
                let _ = fs::remove_file(path);
            }
        }
        let _ = fs::remove_file(
            super::wave_store::root(app)?
                .join("projects")
                .join(format!("{}.json", p.id)),
        );
    }
    let result = result?;
    if path.is_dir() {
        for source in &result.sources {
            super::wave_store::copy_media(
                path,
                &super::wave_store::source_path(app, &source.id)?,
                &PathBuf::from(format!("media/{}.wav", source.id)),
            )?;
        }
        // Linked video stays external; only preserve an explicit bundled copy.
        for (old, video) in old_videos.iter().zip(result.videos.iter()) {
            if path.join("media").join(format!("{}.mp4", old.id)).is_file() {
                super::wave_store::copy_media(
                    path,
                    &super::wave_video::original_path(app, video.asset_id())?,
                    &PathBuf::from(format!("media/{}.mp4", video.id)),
                )?;
            }
        }
        super::voice_store::export_portable(app, &result, path)?;
        super::audio_assets::atomic_json(&path.join("project.json"), &result)?;
        super::wave_store::bind_folder(app, &result, path)?;
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_integrity() {
        let work =
            Scratch(std::env::temp_dir().join(format!("wavehs-test-{}", uuid::Uuid::new_v4())));
        fs::create_dir(&work.0).unwrap();
        let source = work.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("project.json"), b"{}").unwrap();
        fs::write(source.join("audio.wav"), vec![7; 200000]).unwrap();
        let bundle = work.0.join("test.wavehs");
        pack(&source, &bundle).unwrap();
        let target = work.0.join("target");
        fs::create_dir(&target).unwrap();
        unpack(&bundle, &target).unwrap();
        assert_eq!(fs::read(target.join("audio.wav")).unwrap(), vec![7; 200000]);
        assert!(fs::metadata(&bundle).unwrap().len() < 5000);
        assert_eq!(&fs::read(&bundle).unwrap()[..8], COMPRESSED_MAGIC);
        let legacy = work.0.join("legacy.wavehs");
        pack_version(&source, &legacy, false).unwrap();
        let old_target = work.0.join("legacy-restored");
        fs::create_dir(&old_target).unwrap();
        unpack(&legacy, &old_target).unwrap();
        assert_eq!(
            fs::read(old_target.join("audio.wav")).unwrap(),
            vec![7; 200000]
        );
        let cancelled = work.0.join("cancelled");
        fs::create_dir(&cancelled).unwrap();
        assert!(unpack_controlled(&bundle, &cancelled, &AtomicBool::new(true), &|_| {}).is_err());
        assert_eq!(fs::read_dir(&cancelled).unwrap().count(), 0);
        let trailing = work.0.join("trailing.wavehs");
        let mut bytes = fs::read(&bundle).unwrap();
        bytes.push(0);
        fs::write(&trailing, bytes).unwrap();
        let extra = work.0.join("extra");
        fs::create_dir(&extra).unwrap();
        assert!(unpack(&trailing, &extra).is_err());
        let mut data = fs::read(&bundle).unwrap();
        let last = data.len() - 1;
        data[last] ^= 1;
        fs::write(&bundle, data).unwrap();
        let bad = work.0.join("bad");
        fs::create_dir(&bad).unwrap();
        assert!(unpack(&bundle, &bad).is_err());
    }
    #[test]
    fn refuses_traversal() {
        let work =
            Scratch(std::env::temp_dir().join(format!("wavehs-test-{}", uuid::Uuid::new_v4())));
        fs::create_dir(&work.0).unwrap();
        let path = work.0.join("bad");
        let mut f = File::create(&path).unwrap();
        f.write_all(MAGIC).unwrap();
        f.write_all(&1u32.to_le_bytes()).unwrap();
        f.write_all(&4u16.to_le_bytes()).unwrap();
        f.write_all(b"../x").unwrap();
        assert!(unpack(&path, &work.0).is_err());
    }
}
