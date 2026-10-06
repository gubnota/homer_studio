use super::{audio_assets, process_runner, project_store::CommandError, worker_runtime};
use crate::runtime::AppHandle;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    pub engine: String,
    pub python_path: Option<String>,
    pub model_dir: Option<String>,
    pub backend: String,
    pub options: Value,
}
pub const ENGINES: [&str; 5] = [
    "passthrough",
    "seed_vc",
    "rvc",
    "deepfilternet",
    "resemble_enhance",
];
pub fn validate(config: &EngineConfig) -> Result<(), CommandError> {
    if !ENGINES.contains(&config.engine.as_str())
        || !["auto", "cpu", "mps"].contains(&config.backend.as_str())
    {
        return Err(CommandError::new(
            "INVALID_ENGINE",
            "Choose a supported engine and backend.",
        ));
    }
    if !config.options.is_object()
        || serde_json::to_vec(&config.options)
            .map_err(|e| CommandError::internal(e.to_string()))?
            .len()
            > 8192
    {
        return Err(CommandError::new(
            "INVALID_ENGINE_CONFIG",
            "Engine options must be a JSON object smaller than 8 KB.",
        ));
    }
    Ok(())
}
pub fn configurations(app: &AppHandle) -> Result<Vec<EngineConfig>, CommandError> {
    let file = audio_assets::root(app)?.join("engines.json");
    if file.exists() {
        let b = fs::read(file).map_err(|e| CommandError::io("Cannot load engine settings", e))?;
        if b.len() > 65536 {
            return Err(CommandError::new(
                "INVALID_ENGINE_CONFIG",
                "Engine settings are too large.",
            ));
        }
        let configs: Vec<EngineConfig> =
            serde_json::from_slice(&b).map_err(|e| CommandError::internal(e.to_string()))?;
        for c in &configs {
            validate(c)?;
        }
        Ok(configs)
    } else {
        Ok(ENGINES
            .iter()
            .map(|e| EngineConfig {
                engine: (*e).into(),
                python_path: None,
                model_dir: None,
                backend: "auto".into(),
                options: json!({}),
            })
            .collect())
    }
}
pub fn save(app: &AppHandle, config: EngineConfig) -> Result<(), CommandError> {
    validate(&config)?;
    let mut configs = configurations(app)?;
    configs.retain(|c| c.engine != config.engine);
    configs.push(config);
    audio_assets::atomic_json(&audio_assets::root(app)?.join("engines.json"), &configs)
}
pub fn python(app: &AppHandle, config: &EngineConfig) -> Result<PathBuf, CommandError> {
    if let Some(path) = &config.python_path {
        if Path::new(path).is_file() {
            return Ok(path.into());
        }
        return Err(CommandError::new(
            "ENGINE_PYTHON_MISSING",
            "The selected Python environment is unavailable.",
        ));
    }
    let isolated = audio_assets::root(app)?
        .join("runtimes")
        .join(&config.engine)
        .join("bin/python");
    if isolated.is_file() {
        return Ok(isolated);
    }
    if config.engine == "passthrough" {
        return process_runner::resolve_executable("python3", None).ok_or_else(|| {
            CommandError::new("PYTHON_NOT_FOUND", "Select Python in engine settings.")
        });
    }
    Err(CommandError::new(
        "ENGINE_NOT_INSTALLED",
        "Set up this engine's isolated Python environment first.",
    ))
}
pub fn invoke(
    app: &AppHandle,
    config: &EngineConfig,
    request: Value,
    cancel: Arc<AtomicBool>,
    report: &dyn Fn(String),
) -> Result<Value, CommandError> {
    let script = worker_runtime::bundled_workers(app)?.join("audio/processor.py");
    supervised(
        &python(app, config)?,
        &script,
        request,
        cancel,
        report,
        Duration::from_secs(3600),
    )
}
// A new process group belongs only to this request. Kill the group, drain bounded
// frames/logs, and reap on every exit so adapters cannot orphan inference children.
pub fn supervised(
    python: &Path,
    script: &Path,
    request: Value,
    cancel: Arc<AtomicBool>,
    report: &dyn Fn(String),
    timeout: Duration,
) -> Result<Value, CommandError> {
    use std::os::unix::process::CommandExt;
    let mut command = Command::new(python);
    command
        .arg("-u")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|e| CommandError::io("Cannot start audio processor", e))?;
    let pid = child.id();
    let encoded =
        serde_json::to_vec(&request).map_err(|e| CommandError::internal(e.to_string()))?;
    if encoded.len() > 1024 * 1024 {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CommandError::new(
            "PROCESSOR_PROTOCOL",
            "Request too large.",
        ));
    }
    if let Err(e) = child
        .stdin
        .take()
        .unwrap()
        .write_all(&[encoded, b"\n".to_vec()].concat())
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(CommandError::io("Cannot send processing request", e));
    }
    let (tx, rx) = mpsc::sync_channel(64);
    let stdout = child.stdout.take().unwrap();
    let frames = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut bytes = vec![];
            let length = reader.by_ref().take(65537).read_until(b'\n', &mut bytes);
            match length {
                Ok(0) => break,
                Ok(_) if bytes.len() <= 65536 && bytes.last() == Some(&b'\n') => {
                    if tx
                        .send(serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string()))
                        .is_err()
                    {
                        break;
                    }
                }
                _ => {
                    let _ = tx.send(Err("Malformed or oversized processing frame".into()));
                    break;
                }
            }
        }
    });
    let stderr = child.stderr.take().unwrap();
    let logs = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut tail = vec![];
        let mut chunk = [0; 4096];
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&chunk[..n]);
            if tail.len() > 8192 {
                tail.drain(..tail.len() - 8192);
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let start = Instant::now();
    let mut output = None;
    let result = (|| loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(CommandError::new("JOB_CANCELLED", "Processing cancelled."));
        }
        if start.elapsed() > timeout {
            return Err(CommandError::new(
                "PROCESSOR_TIMEOUT",
                "Audio processing timed out.",
            ));
        }
        for _ in 0..64 {
            match rx.try_recv() {
                Ok(frame) => accept_frame(frame, &mut output, report)?,
                Err(_) => break,
            }
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| CommandError::io("Cannot monitor processor", e))?
        {
            if frames.is_finished() {
                for frame in rx.try_iter() {
                    accept_frame(frame, &mut output, report)?;
                }
                if !status.success() {
                    return Err(CommandError::new(
                        "PROCESSOR_FAILED",
                        "Processor exited unsuccessfully. Check its configured runtime and models.",
                    ));
                }
                return output.take().ok_or_else(|| {
                    CommandError::new("PROCESSOR_PROTOCOL", "Processor exited without a result.")
                });
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    })();
    // Terminate descendants even if the parent already exited.
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
    drop(rx);
    let _ = frames.join();
    let tail = logs.join().unwrap_or_default();
    result.map_err(|mut e| {
        if e.code == "PROCESSOR_FAILED" && !tail.is_empty() {
            e.message = format!(
                "{}\n{}",
                e.message,
                tail.chars()
                    .rev()
                    .take(1200)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
            );
        }
        e
    })
}
fn accept_frame(
    frame: Result<Value, String>,
    output: &mut Option<Value>,
    report: &dyn Fn(String),
) -> Result<(), CommandError> {
    let frame = frame.map_err(|e| CommandError::new("PROCESSOR_PROTOCOL", e))?;
    if frame["protocol"] != 1 {
        return Err(CommandError::new(
            "PROCESSOR_PROTOCOL",
            "Unsupported processing protocol.",
        ));
    }
    match frame["type"].as_str() {
        Some("stage") if output.is_none() => report(
            frame["message"]
                .as_str()
                .unwrap_or("Processing audio")
                .into(),
        ),
        Some("result") if output.is_none() && frame.get("result").is_some() => {
            *output = Some(frame["result"].clone())
        }
        Some("error") => {
            return Err(CommandError::new(
                "PROCESSOR_FAILED",
                frame["message"]
                    .as_str()
                    .unwrap_or("Processing failed")
                    .chars()
                    .take(2000)
                    .collect::<String>(),
            ));
        }
        _ => {
            return Err(CommandError::new(
                "PROCESSOR_PROTOCOL",
                "Unknown, duplicate or out-of-order processing frame.",
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_and_trailing_frames() {
        let mut output = None;
        accept_frame(
            Ok(json!({"protocol":1,"type":"result","result":{}})),
            &mut output,
            &|_| {},
        )
        .unwrap();
        assert!(
            accept_frame(
                Ok(json!({"protocol":1,"type":"result","result":{}})),
                &mut output,
                &|_| {}
            )
            .is_err()
        );
        assert!(
            accept_frame(
                Ok(json!({"protocol":1,"type":"stage"})),
                &mut output,
                &|_| {}
            )
            .is_err()
        );
    }
    #[test]
    fn rejects_unknown_and_invalid_protocol() {
        for frame in [
            json!({"protocol":2,"type":"result"}),
            json!({"protocol":1,"type":"unknown"}),
        ] {
            assert!(accept_frame(Ok(frame), &mut None, &|_| {}).is_err());
        }
    }
}
pub fn status(app: &AppHandle, config: &EngineConfig) -> Value {
    match invoke(
        app,
        config,
        json!({"protocol":1,"action":"status","config":config}),
        Default::default(),
        &|_| {},
    ) {
        Ok(v) => v,
        Err(e) => {
            json!({"engine":config.engine,"installed":false,"ready":false,"backend":"cpu","message":e.message,"missingFiles":[],"capabilities":[]})
        }
    }
}
pub fn setup(
    app: &AppHandle,
    engine: &str,
    python_path: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(), CommandError> {
    if !ENGINES.contains(&engine) || engine == "passthrough" {
        return Err(CommandError::new(
            "INVALID_ENGINE",
            "Choose an optional audio engine.",
        ));
    }
    let source = Path::new(python_path);
    if !source.is_file() {
        return Err(CommandError::new(
            "PYTHON_NOT_FOUND",
            "Select an existing Python 3.10 or 3.11 executable.",
        ));
    }
    let version = process_runner::run_bounded(source, &["-c".into(), "import sys; assert sys.version_info[:2] in ((3,10),(3,11)), 'Select Python 3.10 or 3.11'".into()], Duration::from_secs(10), cancel.clone())?;
    if !version.success {
        return Err(CommandError::new(
            "PYTHON_VERSION",
            "Select Python 3.10 or 3.11 for the isolated audio runtime.",
        ));
    }
    let folder = audio_assets::root(app)?.join("runtimes").join(engine);
    fs::create_dir_all(folder.parent().unwrap())
        .map_err(|e| CommandError::io("Cannot create isolated runtime", e))?;
    let result = process_runner::run_bounded(
        source,
        &["-m".into(), "venv".into(), folder.to_string_lossy().into()],
        Duration::from_secs(120),
        cancel.clone(),
    )?;
    if !result.success {
        return Err(CommandError::new("ENGINE_SETUP_FAILED", result.stderr));
    }
    let requirements = worker_runtime::bundled_workers(app)?.join(format!("audio/{engine}.txt"));
    let result = process_runner::run_bounded(
        &folder.join("bin/python"),
        &[
            "-m".into(),
            "pip".into(),
            "install".into(),
            "--quiet".into(),
            "-r".into(),
            requirements.to_string_lossy().into(),
        ],
        Duration::from_secs(1800),
        cancel,
    )?;
    if !result.success {
        return Err(CommandError::new(
            "ENGINE_SETUP_FAILED",
            result.stderr.chars().take(2000).collect::<String>(),
        ));
    }
    Ok(())
}
