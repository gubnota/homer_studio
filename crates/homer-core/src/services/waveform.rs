use super::{audio_assets, project_store::CommandError, settings::Settings};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    hash::{Hash, Hasher},
    io::{BufReader, Read},
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
// One decoder at a time across all editors. Waiting callers re-read the cache.
static ANALYSIS: std::sync::Mutex<()> = std::sync::Mutex::new(());
const RATE: u64 = 8000;
const BLOCK: u64 = 80;
const MAX_PEAKS: usize = 2048;
#[derive(Serialize, Deserialize)]
struct Cache {
    version: u32,
    fingerprint: String,
    duration_ms: u64,
    block_ms: u64,
    levels: Vec<Vec<[f32; 2]>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeakWindow {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub peaks: Vec<[f32; 2]>,
}
fn fingerprint(path: &Path) -> Result<String, CommandError> {
    let metadata =
        fs::metadata(path).map_err(|e| CommandError::io("Cannot inspect waveform source", e))?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    metadata.len().hash(&mut h);
    metadata.modified().ok().hash(&mut h);
    Ok(format!("{:x}", h.finish()))
}
pub fn peaks(
    path: &Path,
    cache_root: &Path,
    settings: &Settings,
    start: u64,
    end: u64,
    max: usize,
    cancel: Arc<AtomicBool>,
) -> Result<PeakWindow, CommandError> {
    if end <= start || max == 0 || max > MAX_PEAKS {
        return Err(CommandError::new(
            "INVALID_WAVEFORM_RANGE",
            "Choose a valid window and at most 2048 peaks.",
        ));
    }
    let _analysis = loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(CommandError::new("JOB_CANCELLED", "Waveform cancelled."));
        }
        match ANALYSIS.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(25))
            }
            Err(_) => return Err(CommandError::internal("Waveform analysis lock unavailable")),
        }
    };
    let key = fingerprint(path)?;
    fs::create_dir_all(cache_root)
        .map_err(|e| CommandError::io("Cannot create waveform cache", e))?;
    let file = cache_root.join(format!("{key}.json"));
    let cached = fs::metadata(&file)
        .ok()
        .filter(|m| m.len() < 32 * 1024 * 1024)
        .and_then(|_| fs::read(&file).ok())
        .and_then(|b| serde_json::from_slice::<Cache>(&b).ok())
        .filter(|c| {
            c.version == 2
                && c.block_ms >= 10
                && c.fingerprint == key
                && !c.levels.is_empty()
                && c.levels.iter().all(|l| {
                    !l.is_empty() && l.iter().all(|p| p[0].is_finite() && p[1].is_finite())
                })
        });
    let cache = match cached {
        Some(c) => c,
        None => {
            let c = generate(path, settings, key, cancel)?;
            audio_assets::atomic_json(&file, &c)?;
            c
        }
    };
    let stop = end.min(cache.duration_ms);
    if start >= stop {
        return Err(CommandError::new(
            "INVALID_WAVEFORM_RANGE",
            "The waveform range exceeds the recording.",
        ));
    }
    let mut level = 0;
    while level + 1 < cache.levels.len()
        && (stop - start) / (cache.block_ms * (1_u64 << level)) > max as u64
    {
        level += 1;
    }
    let width = cache.block_ms * (1_u64 << level);
    let points = &cache.levels[level];
    let first = (start / width) as usize;
    let last = ((stop + width - 1) / width).min(points.len() as u64) as usize;
    let mut output = points[first..last].to_vec();
    if output.len() > max {
        let stride = (output.len() + max - 1) / max;
        output = output.chunks(stride).map(merge).collect();
    }
    Ok(PeakWindow {
        start_ms: start,
        end_ms: stop,
        duration_ms: cache.duration_ms,
        peaks: output,
    })
}
fn merge(points: &[[f32; 2]]) -> [f32; 2] {
    points
        .iter()
        .fold([0., 0.], |a, p| [a[0].min(p[0]), a[1].max(p[1])])
}
fn generate(
    path: &Path,
    settings: &Settings,
    key: String,
    cancel: Arc<AtomicBool>,
) -> Result<Cache, CommandError> {
    let tool = super::process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings."))?;
    let duration = super::speech::probe_duration(path, settings)?;
    let block = BLOCK.max(((duration * RATE / 1000 + 262143) / 262144 + 7) / 8 * 8);
    let mut child = Command::new(tool)
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args(["-ac", "1", "-ar", "8000", "-f", "f32le", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| CommandError::io("Cannot analyze waveform", e))?;
    let done = Arc::new(AtomicBool::new(false));
    let watch_done = done.clone();
    let pid = child.id();
    let watch = std::thread::spawn(move || {
        let start = Instant::now();
        while !watch_done.load(Ordering::SeqCst) {
            if cancel.load(Ordering::SeqCst) || start.elapsed() > Duration::from_secs(600) {
                let _ = Command::new("/bin/kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    let result = (|| {
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let mut b = [0; 4];
        let mut count = 0;
        let mut points = vec![];
        let mut current = [0_f32, 0_f32];
        loop {
            match reader.read_exact(&mut b) {
                Ok(()) => {
                    let x = f32::from_le_bytes(b);
                    if !x.is_finite() {
                        return Err(CommandError::new(
                            "INVALID_AUDIO",
                            "Audio contains non-finite samples.",
                        ));
                    }
                    current[0] = current[0].min(x);
                    current[1] = current[1].max(x);
                    count += 1;
                    if count % block == 0 {
                        points.push(current);
                        current = [0., 0.];
                    }
                    if count > RATE * 24 * 3600 {
                        return Err(CommandError::new(
                            "AUDIO_TOO_LONG",
                            "Waveform analysis supports recordings up to 24 hours.",
                        ));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(CommandError::io("Cannot read waveform", e)),
            }
        }
        if count % block != 0 {
            points.push(current);
        }
        if !child
            .wait()
            .map_err(|e| CommandError::io("Cannot finish waveform", e))?
            .success()
            || count == 0
        {
            return Err(CommandError::new(
                "WAVEFORM_FAILED",
                "Waveform decoding failed or was cancelled.",
            ));
        }
        let mut levels = vec![points];
        while levels.last().unwrap().len() > 1 {
            levels.push(levels.last().unwrap().chunks(2).map(merge).collect());
        }
        Ok(Cache {
            version: 2,
            fingerprint: key,
            duration_ms: count * 1000 / RATE,
            block_ms: block * 1000 / RATE,
            levels,
        })
    })();
    done.store(true, Ordering::SeqCst);
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = watch.join();
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peak_merge_preserves_quiet_signed_extrema() {
        assert_eq!(merge(&[[-0.2, 0.1], [-0.1, 0.3]]), [-0.2, 0.3]);
    }
    #[test]
    fn waveform_cache_serves_windows_and_invalidates_changed_source() {
        let base = std::env::temp_dir().join(format!("homer-peaks-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&base).unwrap();
        let source = base.join("input.wav");
        let write = |seconds: u32| {
            let mut writer = hound::WavWriter::create(
                &source,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 8000,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )
            .unwrap();
            for index in 0..8000 * seconds {
                writer
                    .write_sample(if index % 2 == 0 { 0.25_f32 } else { -0.25 })
                    .unwrap();
            }
            writer.finalize().unwrap();
        };
        write(1);
        let cache = base.join("cache");
        let settings = Settings::default();
        let first = peaks(&source, &cache, &settings, 0, 1000, 32, Default::default()).unwrap();
        assert_eq!(first.duration_ms, 1000);
        assert!(first.peaks.len() <= 32);
        assert!(first.peaks.iter().all(|p| p[0] < 0. && p[1] > 0.));
        let unavailable = Settings {
            ffmpeg_path: Some("/missing/ffmpeg".into()),
            ..settings.clone()
        };
        let window = peaks(
            &source,
            &cache,
            &unavailable,
            200,
            600,
            16,
            Default::default(),
        )
        .unwrap();
        assert_eq!((window.start_ms, window.end_ms), (200, 600));
        assert!(window.peaks.len() <= 16);
        assert!(
            peaks(
                &source,
                &cache,
                &settings,
                0,
                1000,
                2049,
                Default::default()
            )
            .is_err()
        );
        write(2);
        let changed = peaks(&source, &cache, &settings, 0, 2000, 32, Default::default()).unwrap();
        assert_eq!(changed.duration_ms, 2000);
        assert_eq!(fs::read_dir(cache).unwrap().count(), 2);
        fs::remove_dir_all(base).unwrap();
    }
}
