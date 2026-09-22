use serde::{Deserialize, Serialize};
use std::{io::{Read, Write}, net::{TcpStream, SocketAddrV4, Ipv4Addr}, sync::atomic::Ordering, time::{Duration, Instant}};

use super::{jobs::JobControl, project_store::CommandError, settings::validate_worker_url};

const MAX_JSON: usize = 64 * 1024;
const MAX_WAV: usize = 32 * 1024 * 1024;
const MAX_REFERENCE: usize = 2 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerHealth {
    pub protocol_version: u32,
    pub engine: String,
    pub model: String,
    pub ready: bool,
    pub categories: Vec<String>,
    pub max_duration_seconds: u8,
    pub message: String,
}

#[derive(Deserialize)]
struct Created { id: String }
#[derive(Deserialize)]
struct RemoteJob { status: String, error: Option<String>, format: String }
#[derive(Deserialize)]
struct RemoteError { error: String }

pub fn health(url: &str, expected_engine: &str) -> WorkerHealth {
    let fallback = |message: String| WorkerHealth {
        protocol_version: 2,
        engine: expected_engine.into(),
        model: "not loaded".into(),
        ready: false,
        categories: match expected_engine {
            "chatterbox_turbo" => vec!["speech".into(), "vocal_gesture".into()],
            "chatterbox_original" => vec!["speech".into(), "voice_conversion".into()],
            _ => vec!["sound_effect".into()],
        },
        max_duration_seconds: 20,
        message,
    };
    match request(url, "GET", "/v2/health", None, MAX_JSON)
        .and_then(|body| serde_json::from_slice::<WorkerHealth>(&body).map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned invalid health data."))) {
        Ok(value) if value.protocol_version == 2 && (value.engine == expected_engine || (expected_engine == "sound_effect" && value.engine == "stable_audio_open")) => value,
        Ok(_) => fallback("Worker version is out of date. Restart the local workers from this repository.".into()),
        Err(error) => fallback(error.message),
    }
}

pub fn generate(url: &str, request_body: &serde_json::Value, control: &JobControl) -> Result<Vec<u8>, CommandError> {
    let body = serde_json::to_vec(request_body).map_err(|_| CommandError::internal("Cannot serialize sound request"))?;
    let created: Created = serde_json::from_slice(&request(url, "POST", "/v2/jobs", Some(&body), MAX_JSON)?)
        .map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned an invalid job ID."))?;
    uuid::Uuid::parse_str(&created.id).map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned an invalid job ID."))?;
    let path = format!("/v2/jobs/{}", created.id);
    let start = Instant::now();
    loop {
        if control.cancelled.load(Ordering::SeqCst) {
            let _ = request(url, "DELETE", &path, None, MAX_JSON);
            return Err(CommandError::new("JOB_CANCELLED", "The operation was cancelled."));
        }
        if let Err(error) = control.boundary() {
            let _ = request(url, "DELETE", &path, None, MAX_JSON);
            return Err(error);
        }
        if start.elapsed() > Duration::from_secs(1800) {
            let _ = request(url, "DELETE", &path, None, MAX_JSON);
            return Err(CommandError::new("WORKER_TIMEOUT", "Sound generation timed out."));
        }
        let status: RemoteJob = serde_json::from_slice(&request(url, "GET", &path, None, MAX_JSON)?)
            .map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned an invalid job status."))?;
        match status.status.as_str() {
            "completed" if status.format == "wav" => {
                let wav = request(url, "GET", &format!("{path}/audio"), None, MAX_WAV)?;
                if !wav.starts_with(b"RIFF") { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Worker did not return WAV audio.")); }
                return Ok(wav);
            }
            "completed" => return Err(CommandError::new("WORKER_PROTOCOL", "Worker returned an unsupported audio format.")),
            "failed" => return Err(CommandError::new("WORKER_FAILED", status.error.unwrap_or_else(|| "The model failed to generate audio.".into()))),
            "cancelled" => return Err(CommandError::new("JOB_CANCELLED", "The worker cancelled generation.")),
            "queued" | "running" => std::thread::sleep(Duration::from_millis(300)),
            _ => return Err(CommandError::new("WORKER_PROTOCOL", "Worker returned an unknown job state.")),
        }
    }
}

pub fn upload_reference(url: &str, wav: &[u8]) -> Result<String, CommandError> {
    if wav.len() > MAX_REFERENCE || wav.len() < 44 || !wav.starts_with(b"RIFF") || wav.get(8..12) != Some(b"WAVE") {
        return Err(CommandError::new("INVALID_VOICE_SAMPLE", "Voice reference must be a WAV file smaller than 2 MB."));
    }
    let response = request(url, "POST", "/v2/references", Some(wav), MAX_JSON)?;
    let created: Created = serde_json::from_slice(&response).map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned an invalid reference ID."))?;
    uuid::Uuid::parse_str(&created.id).map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned an invalid reference ID."))?;
    Ok(created.id)
}

fn request(url: &str, method: &str, path: &str, body: Option<&[u8]>, max_body: usize) -> Result<Vec<u8>, CommandError> {
    validate_worker_url(url)?;
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let mut stream = TcpStream::connect_timeout(&address.into(), Duration::from_secs(2))
        .map_err(|_| CommandError::new("WORKER_UNAVAILABLE", format!("Cannot reach the local sound worker at {url}. From the repository folder, run python3 workers/start_local.py; then check Settings if it still cannot connect.")))?;
    stream.set_read_timeout(Some(Duration::from_secs(15))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(15))).ok();
    let payload = body.unwrap_or_default();
    let content_type = if path == "/v2/references" { "audio/wav" } else { "application/json" };
    let headers = format!("{method} {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", payload.len());
    stream.write_all(headers.as_bytes()).and_then(|_| stream.write_all(payload))
        .map_err(|error| CommandError::io("Cannot send sound worker request", error))?;
    let mut bytes = Vec::new();
    stream.take((max_body + 8193) as u64).read_to_end(&mut bytes)
        .map_err(|error| CommandError::io("Cannot read sound worker response", error))?;
    if bytes.len() > max_body + 8192 { return Err(CommandError::new("WORKER_RESPONSE_TOO_LARGE", "Sound worker response is too large.")); }
    let split = bytes.windows(4).position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| CommandError::new("WORKER_PROTOCOL", "Worker returned an invalid HTTP response."))?;
    if split > 8192 || bytes.len() - split - 4 > max_body { return Err(CommandError::new("WORKER_RESPONSE_TOO_LARGE", "Sound worker response is too large.")); }
    let header = std::str::from_utf8(&bytes[..split]).map_err(|_| CommandError::new("WORKER_PROTOCOL", "Worker returned invalid headers."))?;
    let status = header.lines().next().and_then(|line| line.split_whitespace().nth(1)).and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| CommandError::new("WORKER_PROTOCOL", "Worker returned no HTTP status."))?;
    let result = bytes[split + 4..].to_vec();
    if !(200..300).contains(&status) {
        let message = serde_json::from_slice::<RemoteError>(&result).map(|error| error.error).unwrap_or_else(|_| format!("Worker returned HTTP {status}."));
        return Err(CommandError::new("WORKER_ERROR", message));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    #[test]
    fn worker_url_rejects_non_loopback() {
        for value in ["http://evil.test:8765", "http://127.0.0.1:8765/other", "http://127.0.0.1:8765@evil.test", "https://127.0.0.1:8765"] {
            assert!(validate_worker_url(value).is_err());
        }
    }

    #[test]
    fn rejects_malformed_worker_reply() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut input = [0u8; 512];
            let _ = stream.read(&mut input);
            stream.write_all(b"not an HTTP response").unwrap();
        });
        let error = request(&format!("http://127.0.0.1:{port}"), "GET", "/v1/health", None, MAX_JSON).unwrap_err();
        assert_eq!(error.code, "WORKER_PROTOCOL");
        server.join().unwrap();
    }
}
