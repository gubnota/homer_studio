use std::{
    collections::HashMap,
    fs,
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
    let Ok(data) = fs::read(path) else {
        return response(StatusCode::NOT_FOUND, Vec::new(), None);
    };
    let total = data.len();
    if let Some(range) = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        if let Some((start, end)) = parse_range(range, total) {
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, "audio/mp4")
                .header(header::ACCEPT_RANGES, "bytes")
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{total}"),
                )
                .header(header::CONTENT_LENGTH, (end - start + 1).to_string())
                .body(data[start..=end].to_vec())
                .unwrap();
        }
        return response(StatusCode::RANGE_NOT_SATISFIABLE, Vec::new(), Some(total));
    }
    response(StatusCode::OK, data, Some(total))
}

fn response(status: StatusCode, body: Vec<u8>, total: Option<usize>) -> Response<Vec<u8>> {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "audio/mp4")
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(total) = total {
        builder = builder.header(header::CONTENT_LENGTH, total.to_string());
    }
    builder.body(body).unwrap()
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
}
