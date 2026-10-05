use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use super::project_store::CommandError;

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub struct ProcessResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn resolve_executable(name: &str, override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = override_path.filter(|value| !value.trim().is_empty()) {
        let candidate = PathBuf::from(path);
        return candidate.is_file().then_some(candidate);
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    resolve_in_directories(name, common_tool_directories())
}

fn resolve_in_directories(
    name: &str,
    directories: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    directories
        .into_iter()
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn common_tool_directories() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        directories.extend([
            home.join(".local/bin"),
            home.join("homebrew/bin"),
            home.join("local/homebrew/bin"),
        ]);
    }
    directories
}

pub fn run_bounded(
    executable: &Path,
    arguments: &[String],
    timeout: Duration,
    cancelled: Arc<AtomicBool>,
) -> Result<ProcessResult, CommandError> {
    use std::{io::Read, os::unix::process::CommandExt};
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|e| CommandError::io("Cannot start local process", e))?;
    let pid = child.id();
    fn drain(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
        thread::spawn(move || {
            let mut tail = Vec::new();
            let mut chunk = [0; 8192];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&chunk[..n]);
                if tail.len() > MAX_OUTPUT_BYTES {
                    tail.drain(..tail.len() - MAX_OUTPUT_BYTES);
                }
            }
            bounded_utf8(tail)
        })
    }
    let stdout = drain(child.stdout.take().unwrap());
    let stderr = drain(child.stderr.take().unwrap());
    let started = Instant::now();
    let result = (|| loop {
        if cancelled.load(Ordering::SeqCst) {
            break Err(CommandError::new(
                "JOB_CANCELLED",
                "The operation was cancelled.",
            ));
        }
        if started.elapsed() >= timeout {
            break Err(CommandError::new(
                "PROCESS_TIMEOUT",
                "The local process timed out.",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status.success()),
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(e) => break Err(CommandError::io("Cannot monitor local process", e)),
        }
    })();
    let _ = Command::new("/bin/kill")
        .args(["-KILL", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
    let stdout = stdout.join().unwrap_or_default();
    let stderr = stderr.join().unwrap_or_default();
    result.map(|success| ProcessResult {
        success,
        stdout,
        stderr,
    })
}

fn bounded_utf8(mut bytes: Vec<u8>) -> String {
    if bytes.len() > MAX_OUTPUT_BYTES {
        bytes.drain(..bytes.len() - MAX_OUTPUT_BYTES);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_absolute_tool() {
        assert_eq!(
            resolve_executable("ignored", Some("/usr/bin/true")),
            Some(PathBuf::from("/usr/bin/true"))
        );
        assert!(resolve_executable("ignored", Some("/definitely/missing")).is_none());
    }

    #[test]
    fn resolves_tool_from_a_custom_directory() {
        let directory =
            std::env::temp_dir().join(format!("homer-tool-discovery-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("example-tool");
        std::fs::write(&executable, b"fixture").unwrap();
        assert_eq!(
            resolve_in_directories("example-tool", [directory.clone()]),
            Some(executable)
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn cancels_a_real_child_process() {
        let cancelled = Arc::new(AtomicBool::new(true));
        let error = run_bounded(
            Path::new("/bin/sleep"),
            &["2".into()],
            Duration::from_secs(3),
            cancelled,
        )
        .expect_err("cancel child");
        assert_eq!(error.code, "JOB_CANCELLED");
    }
    #[test]
    fn drains_verbose_output_without_blocking() {
        let result=run_bounded(Path::new("/bin/sh"), &["-c".into(), "i=0; while [ $i -lt 12000 ]; do echo verbose-output; echo verbose-error >&2; i=$((i+1)); done".into()], Duration::from_secs(10),Default::default()).unwrap();
        assert!(result.success);
        assert!(result.stdout.len() <= MAX_OUTPUT_BYTES);
        assert!(result.stderr.len() <= MAX_OUTPUT_BYTES);
    }
    #[test]
    fn kills_descendants_holding_output_pipes() {
        let start = Instant::now();
        let result = run_bounded(
            Path::new("/bin/sh"),
            &["-c".into(), "sleep 30 & exit 0".into()],
            Duration::from_secs(2),
            Default::default(),
        )
        .unwrap();
        assert!(result.success);
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
