// Streaming single-file project container. Media never enters a whole-file buffer.
use super::{project_store::CommandError, settings::Settings, wave_studio::Project};
use crate::runtime::AppHandle;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAGIC: &[u8; 8] = b"WAVEHS01";
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
    let mut names = vec![];
    files(root, root, &mut names)?;
    names.sort();
    if names.len() > 20000 {
        return Err(invalid());
    }
    let mut output = File::create(destination).map_err(io)?;
    output.write_all(MAGIC).map_err(io)?;
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
        if std::io::copy(&mut File::open(path).map_err(io)?, &mut output).map_err(io)? != size {
            return Err(invalid());
        }
    }
    output.sync_all().map_err(io)
}
fn unpack(source: &Path, root: &Path) -> Result<(), CommandError> {
    let mut input = File::open(source).map_err(io)?;
    let mut magic = [0; 8];
    input.read_exact(&mut magic).map_err(io)?;
    if &magic != MAGIC {
        return Err(invalid());
    }
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
        if std::io::copy(&mut (&mut input).take(size), &mut out).map_err(io)? != size {
            return Err(invalid());
        }
        out.sync_all().map_err(io)?;
        if digest(&path)? != hash {
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
    super::wave_store::save_copy(app, p, &media)?;
    super::voice_store::export_portable(app, p, &media)?;
    let temporary = scratch.0.join("bundle");
    pack(&media, &temporary)?;
    fs::rename(temporary, destination).map_err(io)
}
pub fn import(app: &AppHandle, path: &Path, settings: &Settings) -> Result<Project, CommandError> {
    let scratch = Scratch(std::env::temp_dir().join(format!("wavehs-{}", uuid::Uuid::new_v4())));
    fs::create_dir(&scratch.0).map_err(io)?;
    unpack(path, &scratch.0)?;
    let bytes = fs::read(scratch.0.join("project.json")).map_err(io)?;
    let p: Project = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    super::wave_studio::validate(&p)?;
    let mapping = super::voice_store::import_portable(app, &scratch.0, settings)?;
    let imported = super::wave_store::open_copy(app, &scratch.0, settings);
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
            if let Some(id) = mapping.get(&r.voice_id) {
                if let Some(production) = &mut r.production {
                    production.audio_key = production.audio_key.replace(&r.voice_id, id);
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
            if let Ok(path) = super::wave_video::path(app, &video.id) {
                let _ = fs::remove_file(path);
            }
        }
        let _ = fs::remove_file(
            super::wave_store::root(app)?
                .join("projects")
                .join(format!("{}.json", p.id)),
        );
    }
    result
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
        assert_eq!(fs::read(target.join("audio.wav")).unwrap().len(), 200000);
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
