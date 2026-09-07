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
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CommandError::io("Cannot start local process", error))?;
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CommandError::new(
                "JOB_CANCELLED",
                "The operation was cancelled.",
            ));
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CommandError::new(
                "PROCESS_TIMEOUT",
                "The local process timed out.",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|error| {
                    CommandError::io("Cannot collect local process output", error)
                })?;
                return Ok(ProcessResult {
                    success: status.success(),
                    stdout: bounded_utf8(output.stdout),
                    stderr: bounded_utf8(output.stderr),
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(CommandError::io("Cannot monitor local process", error)),
        }
    }
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
}
