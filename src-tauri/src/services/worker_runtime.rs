use std::{
    fs,
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::{jobs::JobControl, process_runner, project_store::CommandError};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub path: String,
    pub installed: bool,
    pub python_path: Option<String>,
    pub message: String,
}

pub fn data_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_data_dir()
        .map_err(|error| CommandError::new("APP_DATA_UNAVAILABLE", error.to_string()))
}

pub fn bundled_workers(app: &AppHandle) -> Result<PathBuf, CommandError> {
    if let Some(configured) = std::env::var_os("HOMER_WORKER_LAUNCHER") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return path.parent().map(Path::to_path_buf).ok_or_else(|| {
                CommandError::new(
                    "WORKER_LAUNCHER_NOT_FOUND",
                    "Invalid worker launcher override.",
                )
            });
        }
        return Err(CommandError::new(
            "WORKER_LAUNCHER_NOT_FOUND",
            "Worker launcher override does not exist.",
        ));
    }
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|error| CommandError::new("WORKER_LAUNCHER_NOT_FOUND", error.to_string()))?
        .join("workers");
    if bundled.join("start_local.py").is_file() {
        return Ok(bundled);
    }
    #[cfg(debug_assertions)]
    {
        let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("workers");
        if checkout.join("start_local.py").is_file() {
            return Ok(checkout);
        }
    }
    Err(CommandError::new(
        "WORKER_LAUNCHER_NOT_FOUND",
        "Bundled Chatterbox worker files are missing. Reinstall Homer Studio.",
    ))
}

pub fn runtime_python(app: &AppHandle) -> Result<PathBuf, CommandError> {
    Ok(data_dir(app)?.join("python/chatterbox/bin/python"))
}

fn requirements(app: &AppHandle) -> Result<(PathBuf, Vec<u8>), CommandError> {
    let path = bundled_workers(app)?.join("chatterbox/requirements.txt");
    let content = fs::read(&path)
        .map_err(|error| CommandError::io("Cannot read bundled Chatterbox requirements", error))?;
    Ok((path, content))
}

fn detected_python(configured: Option<&str>) -> Option<PathBuf> {
    process_runner::resolve_executable("python3.10", configured)
}

pub fn status(app: &AppHandle, configured: Option<&str>) -> Result<RuntimeStatus, CommandError> {
    let python = runtime_python(app)?;
    let (_, content) = requirements(app)?;
    let installed = fs::read(data_dir(app)?.join("python/chatterbox/.requirements-installed"))
        .ok()
        .as_deref()
        == Some(content.as_slice())
        && python.is_file();
    let selected = detected_python(configured);
    let message = if installed {
        "Chatterbox Python packages are ready in Homer Studio's Application Support folder.".into()
    } else if configured.is_some_and(|path| !path.trim().is_empty()) && selected.is_none() {
        "The selected Python 3.10 executable was not found. Choose a valid Python 3.10 executable."
            .into()
    } else if selected.is_none() {
        "Python 3.10 is required. Install it, then return here to set up Chatterbox.".into()
    } else {
        "Chatterbox Python packages are not installed. Select Install runtime to set them up in Application Support.".into()
    };
    Ok(RuntimeStatus {
        path: python
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        installed,
        python_path: selected.map(|path| path.to_string_lossy().into_owned()),
        message,
    })
}

fn run_step(
    executable: &Path,
    args: &[&str],
    control: &JobControl,
    timeout: Duration,
    log: Option<&Path>,
) -> Result<(), CommandError> {
    let mut command = Command::new(executable);
    command.args(args).stdin(Stdio::null());
    if let Some(path) = log {
        let output = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| CommandError::io("Cannot open Python setup log", error))?;
        command.stdout(Stdio::from(output.try_clone().map_err(|error| {
            CommandError::io("Cannot clone Python setup log", error)
        })?));
        command.stderr(Stdio::from(output));
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let mut child = command
        .spawn()
        .map_err(|error| CommandError::io("Cannot start Python setup", error))?;
    let started = Instant::now();
    loop {
        if let Err(error) = control.boundary() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CommandError::new(
                "PYTHON_SETUP_TIMEOUT",
                "Python setup timed out. Check the setup log in Application Support.",
            ));
        }
        match child.try_wait() {
            Ok(Some(result)) if result.success() => return Ok(()),
            Ok(Some(_)) => {
                return Err(CommandError::new(
                    "PYTHON_SETUP_FAILED",
                    format!(
                        "Python setup failed: {}",
                        log.map(last_log_line)
                            .unwrap_or_else(|| "Check the Python executable.".into())
                    ),
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(150)),
            Err(error) => return Err(CommandError::io("Cannot monitor Python setup", error)),
        }
    }
}

fn last_log_line(path: &Path) -> String {
    let Ok(mut file) = fs::File::open(path) else {
        return "See the Python setup log in Application Support.".into();
    };
    let length = file.metadata().map(|value| value.len()).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(4096)));
    let mut bytes = Vec::new();
    let _ = file.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes)
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("See the Python setup log in Application Support.")
        .chars()
        .take(300)
        .collect()
}

pub fn install(
    app: &AppHandle,
    configured: Option<&str>,
    control: &JobControl,
    progress: &dyn Fn(u8),
    report: &dyn Fn(String),
) -> Result<(), CommandError> {
    let base = detected_python(configured).ok_or_else(|| {
        CommandError::new(
            "PYTHON_NOT_FOUND",
            "Install Python 3.10 or choose its executable in Settings.",
        )
    })?;
    let version = Command::new(&base)
        .arg("--version")
        .output()
        .map_err(|error| CommandError::io("Cannot check Python version", error))?;
    let version_text = String::from_utf8_lossy(&version.stdout);
    if !version.status.success() || !version_text.starts_with("Python 3.10.") {
        return Err(CommandError::new(
            "PYTHON_VERSION",
            "Chatterbox requires Python 3.10. Choose a Python 3.10 executable.",
        ));
    }
    let (requirements, content) = requirements(app)?;
    let python = runtime_python(app)?;
    let venv = python.parent().unwrap().parent().unwrap();
    fs::create_dir_all(venv.parent().unwrap())
        .map_err(|error| CommandError::io("Cannot create application data folder", error))?;
    let log_dir = data_dir(app)?.join("logs");
    fs::create_dir_all(&log_dir)
        .map_err(|error| CommandError::io("Cannot create application log folder", error))?;
    let log = log_dir.join("chatterbox-setup.log");
    fs::write(&log, b"")
        .map_err(|error| CommandError::io("Cannot reset Python setup log", error))?;
    report("Creating Chatterbox Python environment in Application Support".into());
    progress(10);
    run_step(
        &base,
        &["-m", "venv", venv.to_str().unwrap()],
        control,
        Duration::from_secs(120),
        Some(&log),
    )?;
    progress(25);
    report("Installing Chatterbox packages; this may take several minutes".into());
    run_step(
        &python,
        &[
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "-r",
            requirements.to_str().unwrap(),
        ],
        control,
        Duration::from_secs(3600),
        Some(&log),
    )?;
    progress(90);
    run_step(
        &python,
        &["-c", "import chatterbox, torch, torchaudio"],
        control,
        Duration::from_secs(30),
        Some(&log),
    )?;
    fs::write(venv.join(".requirements-installed"), content)
        .map_err(|error| CommandError::io("Cannot mark Python setup complete", error))?;
    report("Chatterbox Python runtime is ready".into());
    progress(100);
    Ok(())
}
