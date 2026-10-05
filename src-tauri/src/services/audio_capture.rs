use super::{audio_assets, memo_store, project_store::CommandError, settings};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureState {
    pub session_id: String,
    pub memo_id: String,
    pub state: String,
    pub elapsed_ms: u64,
    pub peak: f32,
    pub rms: f32,
    pub clipping: bool,
    pub peaks: Vec<f32>,
    pub message: Option<String>,
}
type Reply = mpsc::Sender<Result<CaptureState, CommandError>>;
struct Active {
    sender: mpsc::Sender<(String, Reply)>,
    thread: std::thread::JoinHandle<()>,
}
#[derive(Default)]
pub struct CaptureService {
    active: Mutex<Option<Active>>,
}
impl CaptureService {
    pub fn start(
        &self,
        app: AppHandle,
        memo_id: String,
        device_id: Option<String>,
    ) -> Result<CaptureState, CommandError> {
        if memo_store::load(&app, &memo_id)?.deleted {
            return Err(CommandError::new(
                "MEMO_DELETED",
                "Restore this recording before capturing audio.",
            ));
        }
        if permission(true)? != "granted" {
            return Err(CommandError::new(
                "MICROPHONE_DENIED",
                "Allow Homer Studio in System Settings → Privacy & Security → Microphone.",
            ));
        }
        let mut active = self
            .active
            .lock()
            .map_err(|_| CommandError::internal("Capture state unavailable"))?;
        if active.as_ref().is_some_and(|a| !a.thread.is_finished()) {
            return Err(CommandError::new(
                "RECORDING_ACTIVE",
                "Finish the current recording first.",
            ));
        }
        if let Some(previous) = active.take() {
            let _ = previous.thread.join();
        }
        let (tx, rx) = mpsc::channel();
        let (ready, reply) = mpsc::channel();
        let thread = std::thread::spawn(move || record(app, memo_id, device_id, rx, ready));
        *active = Some(Active { sender: tx, thread });
        match reply.recv_timeout(Duration::from_secs(15)) {
            Ok(result) => result,
            Err(_) => {
                if let Some(a) = active.as_ref() {
                    let (tx, _) = mpsc::channel();
                    let _ = a.sender.send(("discard".into(), tx));
                }
                Err(CommandError::new(
                    "CAPTURE_TIMEOUT",
                    "Microphone did not start.",
                ))
            }
        }
    }
    pub fn control(&self, action: &str) -> Result<CaptureState, CommandError> {
        if !["pause", "resume", "stop", "discard", "status"].contains(&action) {
            return Err(CommandError::new(
                "INVALID_CAPTURE_ACTION",
                "Unknown recording action.",
            ));
        }
        let active = self
            .active
            .lock()
            .map_err(|_| CommandError::internal("Capture unavailable"))?;
        let a = active
            .as_ref()
            .ok_or_else(|| CommandError::new("NO_RECORDING", "Start a recording first."))?;
        let (tx, rx) = mpsc::channel();
        a.sender
            .send((action.into(), tx))
            .map_err(|_| CommandError::new("NO_RECORDING", "Recording has stopped."))?;
        rx.recv_timeout(Duration::from_secs(120))
            .map_err(|_| CommandError::new("CAPTURE_TIMEOUT", "Recording action timed out."))?
    }
    pub fn shutdown(&self) {
        if let Ok(mut active) = self.active.lock() {
            if let Some(a) = active.take() {
                let (tx, _) = mpsc::channel();
                let _ = a.sender.send(("stop".into(), tx));
                let _ = a.thread.join();
            }
        }
    }
}
pub fn devices() -> Result<Vec<CaptureDevice>, CommandError> {
    let host = cpal::default_host();
    let default = host.default_input_device().and_then(|d| d.name().ok());
    let mut counts = std::collections::HashMap::new();
    host.input_devices()
        .map_err(|e| CommandError::new("CAPTURE_DEVICES", e.to_string()))?
        .map(|d| {
            let name = d
                .name()
                .map_err(|e| CommandError::new("CAPTURE_DEVICE", e.to_string()))?;
            let count = counts.entry(name.clone()).or_insert(0);
            let id = format!("{name}#{count}");
            *count += 1;
            Ok(CaptureDevice {
                id,
                is_default: default.as_ref() == Some(&name),
                name,
            })
        })
        .collect()
}
#[cfg(target_os = "macos")]
pub fn permission(request: bool) -> Result<String, CommandError> {
    use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};
    unsafe {
        let media = AVMediaTypeAudio
            .ok_or_else(|| CommandError::internal("Audio media type unavailable"))?;
        let status = AVCaptureDevice::authorizationStatusForMediaType(media).0;
        if status == 0 && request {
            let (tx, rx) = mpsc::channel();
            let handler = block2::RcBlock::new(move |_granted| {
                let _ = tx.send(());
            });
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media, &handler);
            rx.recv_timeout(Duration::from_secs(120)).map_err(|_| {
                CommandError::new(
                    "MICROPHONE_PERMISSION_TIMEOUT",
                    "Microphone permission is still pending.",
                )
            })?;
            return permission(false);
        }
        Ok(match status {
            0 => "notDetermined",
            1 => "restricted",
            2 => "denied",
            3 => "granted",
            _ => "restricted",
        }
        .into())
    }
}
#[cfg(not(target_os = "macos"))]
pub fn permission(_: bool) -> Result<String, CommandError> {
    Ok("granted".into())
}
fn stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: mpsc::SyncSender<Vec<f32>>,
    paused: Arc<AtomicBool>,
    failed: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, CommandError>
where
    T: cpal::SizedSample + cpal::Sample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let errors = failed.clone();
    device.build_input_stream(config,move|data:&[T],_|{if paused.load(Ordering::SeqCst){return;}if data.len()>131072{if let Ok(mut e)=errors.lock(){*e=Some("The device audio buffer exceeded the recording limit.".into());}return;}let mono=data.chunks_exact(channels).map(|frame|frame.iter().map(|s|s.to_sample::<f32>()).sum::<f32>()/channels as f32).collect();if tx.try_send(mono).is_err(){if let Ok(mut e)=errors.lock(){*e=Some("Recording stopped because disk writing could not keep up. The captured audio is recoverable.".into());}}},move|e|{if let Ok(mut error)=failed.lock(){*error=Some(format!("The input device stopped: {e}"));}},None).map_err(|e|CommandError::new("CAPTURE_START_FAILED",e.to_string()))
}
fn record(
    app: AppHandle,
    memo_id: String,
    device_id: Option<String>,
    commands: mpsc::Receiver<(String, Reply)>,
    ready: Reply,
) {
    let initial = CaptureState {
        session_id: uuid::Uuid::new_v4().to_string(),
        memo_id: memo_id.clone(),
        state: "recording".into(),
        elapsed_ms: 0,
        peak: 0.,
        rms: 0.,
        clipping: false,
        peaks: vec![],
        message: None,
    };
    let mut state = initial;
    let mut stop_reply: Option<Reply> = None;
    let mut recovery_path = None;
    let result: Result<(), CommandError> = (|| {
        let host = cpal::default_host();
        let device = if let Some(id) = device_id.filter(|s| !s.is_empty()) {
            let mut counts = std::collections::HashMap::new();
            host.input_devices()
                .map_err(|e| CommandError::new("CAPTURE_DEVICES", e.to_string()))?
                .find(|d| {
                    let name = d.name().unwrap_or_default();
                    let count = counts.entry(name.clone()).or_insert(0);
                    let current = format!("{name}#{count}");
                    *count += 1;
                    current == id
                })
        } else {
            host.default_input_device()
        }
        .ok_or_else(|| CommandError::new("NO_INPUT_DEVICE", "Select an available microphone."))?;
        let supported = device
            .default_input_config()
            .map_err(|e| CommandError::new("CAPTURE_FORMAT", e.to_string()))?;
        let config = supported.config();
        let rate = config.sample_rate.0;
        let staging = audio_assets::root(&app)?.join("recordings");
        fs::create_dir_all(&staging)
            .map_err(|e| CommandError::io("Cannot create recording folder", e))?;
        let path = staging.join(format!("{}.wav", state.session_id));
        recovery_path = Some(path.clone());
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .map_err(|e| CommandError::new("RECORDING_WRITE_FAILED", e.to_string()))?;
        let (tx, rx) = mpsc::sync_channel(64);
        let paused = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(Mutex::new(None));
        let capture = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                stream::<f32>(&device, &config, tx, paused.clone(), failed.clone())
            }
            cpal::SampleFormat::I16 => {
                stream::<i16>(&device, &config, tx, paused.clone(), failed.clone())
            }
            cpal::SampleFormat::U16 => {
                stream::<u16>(&device, &config, tx, paused.clone(), failed.clone())
            }
            _ => Err(CommandError::new(
                "CAPTURE_FORMAT",
                "This microphone has an unsupported sample format.",
            )),
        }?;
        capture
            .play()
            .map_err(|e| CommandError::new("CAPTURE_START_FAILED", e.to_string()))?;
        let _ = ready.send(Ok(state.clone()));
        let mut samples = 0_u64;
        let mut emitted = Instant::now();
        let mut discard = false;
        loop {
            if let Ok((action, reply)) = commands.try_recv() {
                match action.as_str() {
                    "pause" => {
                        paused.store(true, Ordering::SeqCst);
                        state.state = "paused".into();
                    }
                    "resume" => {
                        paused.store(false, Ordering::SeqCst);
                        state.state = "recording".into();
                    }
                    "stop" | "discard" => {
                        discard = action == "discard";
                        stop_reply = Some(reply);
                        break;
                    }
                    _ => {}
                }
                let _ = reply.send(Ok(state.clone()));
            }
            if let Some(error) = failed.lock().ok().and_then(|e| e.clone()) {
                state.message = Some(error);
                state.state = "failed".into();
                break;
            }
            if let Ok(buffer) = rx.recv_timeout(Duration::from_millis(10)) {
                for x in &buffer {
                    writer
                        .write_sample(*x)
                        .map_err(|e| CommandError::new("RECORDING_WRITE_FAILED", e.to_string()))?;
                }
                samples += buffer.len() as u64;
                state.elapsed_ms = samples * 1000 / rate as u64;
                state.peak = buffer.iter().fold(0_f32, |p, x| p.max(x.abs()));
                state.rms =
                    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len().max(1) as f32).sqrt();
                state.clipping = state.peak >= 0.99;
                state.peaks = buffer
                    .chunks((buffer.len() / 32).max(1))
                    .take(32)
                    .map(|p| p.iter().fold(0_f32, |a, x| a.max(x.abs())))
                    .collect();
            }
            if emitted.elapsed() >= Duration::from_millis(50) {
                let _ = app.emit("audio-capture", &state);
                emitted = Instant::now();
            }
            if state.elapsed_ms >= 3_600_000 {
                state.message = Some("The one-hour recording limit was reached.".into());
                break;
            }
        }
        drop(capture);
        if !discard {
            for buffer in rx.try_iter() {
                for x in buffer {
                    writer
                        .write_sample(x)
                        .map_err(|e| CommandError::new("RECORDING_WRITE_FAILED", e.to_string()))?;
                    samples += 1;
                }
            }
        }
        writer
            .finalize()
            .map_err(|e| CommandError::new("RECORDING_WRITE_FAILED", e.to_string()))?;
        state.elapsed_ms = samples * 1000 / rate as u64;
        if discard {
            let _ = fs::remove_file(&path);
            state.state = "stopped".into();
        } else if samples > 0 {
            let settings = settings::load(&app)?;
            let lock = app.state::<crate::AppState>();
            let _write = lock
                .project_write_lock
                .lock()
                .map_err(|_| CommandError::internal("Audio library lock unavailable"))?;
            let name = format!("Recording {}", super::sound_store::now_ms());
            memo_store::import_file(
                &app,
                &memo_id,
                &path,
                &name,
                Some(rate),
                &settings,
                Default::default(),
            )?;
            let _ = fs::remove_file(path);
            if state.state != "failed" {
                state.state = "stopped".into();
            }
        } else {
            let _ = fs::remove_file(&path);
            state.state = "stopped".into();
        }
        Ok(())
    })();
    if let Err(mut error) = result {
        if let Some(path) = recovery_path.filter(|path| path.is_file()) {
            error.message.push_str(&format!(
                " Recover captured audio by importing {}.",
                path.display()
            ));
        }
        state.state = "failed".into();
        state.message = Some(error.message.clone());
        if let Some(reply) = stop_reply.take() {
            let _ = reply.send(Err(CommandError::new(
                "CAPTURE_FAILED",
                error.message.clone(),
            )));
        }
        let _ = ready.send(Err(error));
    }
    if let Some(reply) = stop_reply {
        let _ = reply.send(Ok(state.clone()));
    }
    let _ = app.emit("audio-capture", state);
}
