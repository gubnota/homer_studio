use std::{
    collections::HashMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tauri::http::{Request, Response, StatusCode, header};

pub fn respond(
    registry: &Arc<RwLock<HashMap<String, PathBuf>>>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let id = request.uri().path().trim_start_matches('/');
    let path = registry
        .read()
        .ok()
        .and_then(|assets| assets.get(id).cloned());
    let Some(path) = path else {
        return response(StatusCode::NOT_FOUND, Vec::new(), None);
    };
    let mime = if path.extension().and_then(|s| s.to_str()) == Some("wav") {
        "audio/wav"
    } else {
        "audio/mp4"
    };
    let Ok(mut file) = fs::File::open(path) else {
        return response(StatusCode::NOT_FOUND, Vec::new(), None);
    };
    let Ok(metadata) = file.metadata() else {
        return response(StatusCode::NOT_FOUND, vec![], None);
    };
    let total = metadata.len() as usize;
    if let Some(range) = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        if let Some((start, requested_end)) = parse_range(range, total) {
            let end = requested_end.min(start.saturating_add(4 * 1024 * 1024 - 1));
            let mut data = vec![0; end - start + 1];
            if file
                .seek(SeekFrom::Start(start as u64))
                .and_then(|_| file.read_exact(&mut data))
                .is_err()
            {
                return response(StatusCode::NOT_FOUND, vec![], None);
            }
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, mime)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{total}"),
                )
                .header(header::CONTENT_LENGTH, (end - start + 1).to_string())
                .body(data)
                .unwrap();
        }
        return response(StatusCode::RANGE_NOT_SATISFIABLE, Vec::new(), Some(total));
    }
    if total > 4 * 1024 * 1024 {
        let end = (4 * 1024 * 1024).min(total) - 1;
        let mut data = vec![0; end + 1];
        if file.read_exact(&mut data).is_err() {
            return response(StatusCode::NOT_FOUND, vec![], None);
        }
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, mime)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_RANGE, format!("bytes 0-{end}/{total}"))
            .header(header::CONTENT_LENGTH, data.len())
            .body(data)
            .unwrap();
    }
    let mut data = vec![0; total];
    if file.read_exact(&mut data).is_err() {
        return response(StatusCode::NOT_FOUND, vec![], None);
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, total)
        .body(data)
        .unwrap()
}

fn response(status: StatusCode, body: Vec<u8>, total: Option<usize>) -> Response<Vec<u8>> {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "audio/mp4")
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(total) = total {
        builder = builder.header(header::CONTENT_RANGE, format!("bytes */{total}"));
    }
    builder
        .header(header::CONTENT_LENGTH, body.len())
        .body(body)
        .unwrap()
}

fn parse_range(value: &str, total: usize) -> Option<(usize, usize)> {
    if total == 0 {
        return None;
    }
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix: usize = end.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        return Some((total.saturating_sub(suffix), total - 1));
    }
    let start: usize = start.parse().ok()?;
    if start >= total {
        return None;
    }
    let end = if end.is_empty() {
        total - 1
    } else {
        end.parse::<usize>().ok()?.min(total - 1)
    };
    (start <= end).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_single_and_suffix_ranges() {
        assert_eq!(parse_range("bytes=2-4", 10), Some((2, 4)));
        assert_eq!(parse_range("bytes=7-", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=10-", 10), None);
    }
    #[test]
    fn unsatisfied_range_reports_total_without_a_phantom_body() {
        let reply = response(StatusCode::RANGE_NOT_SATISFIABLE, vec![], Some(512));
        assert_eq!(reply.headers()[header::CONTENT_RANGE], "bytes */512");
        assert_eq!(reply.headers()[header::CONTENT_LENGTH], "0");
        assert!(reply.body().is_empty());
    }
}
