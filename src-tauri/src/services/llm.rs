use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use super::{process_runner, project_store::CommandError, settings::LlmSettings};

const MAX_INPUT_CHARS: usize = 16_000;
const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextCandidate {
    pub text: String,
    pub provider: String,
    pub model: String,
}

pub fn process(
    config: &LlmSettings,
    instruction: &str,
    text: &str,
) -> Result<TextCandidate, CommandError> {
    validate_request(instruction, text)?;
    match config {
        LlmSettings::None => Err(CommandError::new(
            "LLM_DISABLED",
            "Choose a local text provider in Settings first.",
        )),
        LlmSettings::LlamaCpp {
            executable_path,
            model_path,
            context_size,
            max_tokens,
            gpu_layers,
        } => {
            let executable = process_runner::resolve_executable("llama-cli", Some(executable_path))
                .ok_or_else(|| {
                    CommandError::new(
                        "LLAMA_NOT_FOUND",
                        "The configured llama-cli executable was not found.",
                    )
                })?;
            if !Path::new(model_path).is_file() {
                return Err(CommandError::new(
                    "MODEL_NOT_FOUND",
                    "The configured GGUF model was not found.",
                ));
            }
            let prompt = prompt(instruction, text);
            let arguments = vec![
                "-m".into(),
                model_path.clone(),
                "-p".into(),
                prompt,
                "-c".into(),
                context_size.to_string(),
                "-n".into(),
                max_tokens.to_string(),
                "-ngl".into(),
                gpu_layers.to_string(),
                "--no-display-prompt".into(),
            ];
            let result = process_runner::run_bounded(
                &executable,
                &arguments,
                Duration::from_secs(600),
                Arc::new(AtomicBool::new(false)),
            )?;
            if !result.success {
                return Err(CommandError::new(
                    "LLM_PROCESS_FAILED",
                    concise(&result.stderr),
                ));
            }
            candidate(result.stdout, "llama_cpp", model_path)
        }
        LlmSettings::Ollama {
            base_url,
            model,
            context_size,
            max_tokens,
        } => {
            if model.trim().is_empty() {
                return Err(CommandError::new(
                    "MODEL_NOT_CONFIGURED",
                    "Enter an installed Ollama model in Settings.",
                ));
            }
            let body = json!({"model": model, "prompt": prompt(instruction, text), "stream": false, "options": {"num_ctx": context_size, "num_predict": max_tokens}}).to_string();
            let response = post_loopback(base_url, "/api/generate", &body)?;
            #[derive(Deserialize)]
            struct OllamaResponse {
                response: Option<String>,
                error: Option<String>,
            }
            let parsed: OllamaResponse = serde_json::from_str(&response).map_err(|error| {
                CommandError::new(
                    "INVALID_LLM_RESPONSE",
                    format!("Ollama returned invalid JSON: {error}"),
                )
            })?;
            if let Some(error) = parsed.error {
                return Err(CommandError::new("LLM_PROCESS_FAILED", error));
            }
            candidate(parsed.response.unwrap_or_default(), "ollama", model)
        }
    }
}

fn validate_request(instruction: &str, text: &str) -> Result<(), CommandError> {
    if instruction.trim().is_empty() {
        return Err(CommandError::new(
            "EMPTY_INSTRUCTION",
            "Enter an instruction for the text model.",
        ));
    }
    if text.trim().is_empty() {
        return Err(CommandError::new(
            "EMPTY_TEXT",
            "There is no text to process.",
        ));
    }
    if text.chars().count() > MAX_INPUT_CHARS {
        return Err(CommandError::new(
            "LLM_INPUT_TOO_LARGE",
            "This chapter is over 16,000 characters. Split it before local processing.",
        ));
    }
    Ok(())
}

fn prompt(instruction: &str, text: &str) -> String {
    format!(
        "Follow this editing instruction and return only the revised text.\nInstruction: {}\n\nText:\n{}",
        instruction.trim(),
        text
    )
}

fn candidate(text: String, provider: &str, model: &str) -> Result<TextCandidate, CommandError> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(CommandError::new(
            "EMPTY_LLM_OUTPUT",
            "The text provider returned no candidate text.",
        ));
    }
    if text.len() > MAX_OUTPUT_BYTES {
        return Err(CommandError::new(
            "LLM_OUTPUT_TOO_LARGE",
            "The text provider returned more than 2 MB.",
        ));
    }
    Ok(TextCandidate {
        text,
        provider: provider.into(),
        model: model.into(),
    })
}

fn post_loopback(base_url: &str, path: &str, body: &str) -> Result<String, CommandError> {
    let (host, port) = loopback_address(base_url)?;
    let mut stream = connect_loopback(&host, port)?;
    stream.set_read_timeout(Some(Duration::from_secs(600))).ok();
    write!(stream, "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).map_err(|error| CommandError::io("Cannot send Ollama request", error))?;
    let mut response = Vec::new();
    stream
        .take((MAX_OUTPUT_BYTES + 64 * 1024) as u64)
        .read_to_end(&mut response)
        .map_err(|error| CommandError::io("Cannot read Ollama response", error))?;
    let response = String::from_utf8_lossy(&response);
    let (head, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
        CommandError::new(
            "INVALID_LLM_RESPONSE",
            "Ollama returned an invalid HTTP response.",
        )
    })?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return Err(CommandError::new(
            "LLM_PROCESS_FAILED",
            format!(
                "Ollama request failed: {}",
                head.lines().next().unwrap_or("HTTP error")
            ),
        ));
    }
    Ok(body.to_string())
}

pub fn ollama_reachable(base_url: &str) -> bool {
    loopback_address(base_url)
        .and_then(|(host, port)| connect_loopback(&host, port))
        .is_ok()
}

fn loopback_address(base_url: &str) -> Result<(String, u16), CommandError> {
    let address = base_url
        .strip_prefix("http://")
        .ok_or_else(|| CommandError::new("UNSAFE_OLLAMA_URL", "Ollama must use loopback HTTP."))?
        .trim_end_matches('/');
    let mut parts = address.split(':');
    let host = parts.next().unwrap_or_default();
    if !matches!(host, "127.0.0.1" | "localhost") {
        return Err(CommandError::new(
            "UNSAFE_OLLAMA_URL",
            "Ollama must use a loopback address.",
        ));
    }
    let port: u16 =
        parts.next().unwrap_or("11434").parse().map_err(|_| {
            CommandError::new("INVALID_OLLAMA_URL", "Ollama URL has an invalid port.")
        })?;
    if parts.next().is_some() {
        return Err(CommandError::new(
            "INVALID_OLLAMA_URL",
            "Ollama URL is invalid.",
        ));
    }
    Ok((host.to_string(), port))
}

fn connect_loopback(host: &str, port: u16) -> Result<TcpStream, CommandError> {
    TcpStream::connect_timeout(
        &format!("{host}:{port}")
            .parse()
            .map_err(|_| CommandError::new("INVALID_OLLAMA_URL", "Ollama URL is invalid."))?,
        Duration::from_secs(3),
    )
    .map_err(|error| {
        CommandError::new(
            "OLLAMA_UNAVAILABLE",
            format!("Cannot connect to Ollama: {error}"),
        )
    })
}

fn concise(message: &str) -> String {
    message
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Local text processing failed.")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_oversized_input() {
        assert_eq!(
            process(&LlmSettings::None, "Edit", &"a".repeat(MAX_INPUT_CHARS + 1))
                .unwrap_err()
                .code,
            "LLM_INPUT_TOO_LARGE"
        );
    }
    #[test]
    fn rejects_empty_candidate() {
        assert_eq!(
            candidate("  ".into(), "test", "test").unwrap_err().code,
            "EMPTY_LLM_OUTPUT"
        );
    }
}
