// Converts manuscript formatting to lines that can be sent to the speech worker.
// Narration markers are deliberately retained; speech::parse_markup validates them.
pub fn lines(source: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut fenced = false;
    for raw in source.lines() {
        let mut line = raw.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.is_empty() || line.starts_with("<!--") || line.starts_with("<http") {
            continue;
        }
        while let Some(stripped) = line.strip_prefix('>') { line = stripped.trim_start(); }
        if line.starts_with('#') {
            line = line.trim_start_matches('#').trim_start();
        }
        if let Some((prefix, rest)) = line.split_once(' ') {
            if matches!(prefix, "-" | "*" | "+") || (prefix.ends_with('.') && prefix[..prefix.len()-1].chars().all(|c| c.is_ascii_digit())) {
                line = rest;
            }
        }
        if line.chars().all(|c| matches!(c, '-' | '*' | '_' | '=' | ' ')) { continue; }
        let spoken = inline(line);
        if !spoken.is_empty() { result.push(spoken); }
    }
    result
}

fn inline(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while !rest.is_empty() {
        if let Some(image) = rest.strip_prefix("![") {
            if let Some(end) = image.find("](") {
                if let Some(close) = image[end+2..].find(')') {
                    rest = &image[end+3+close..];
                    continue;
                }
            }
        }
        if let Some(link) = rest.strip_prefix('[') {
            if let Some(end) = link.find("](") {
                if let Some(close) = link[end+2..].find(')') {
                    out.push_str(&inline(&link[..end]));
                    rest = &link[end+3+close..];
                    continue;
                }
            }
        }
        let ch = rest.chars().next().unwrap();
        rest = &rest[ch.len_utf8()..];
        if ch == '\\' {
            if let Some(next) = rest.chars().next() {
                out.push(next);
                rest = &rest[next.len_utf8()..];
            }
        } else if !matches!(ch, '*' | '_' | '`' | '~') {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::lines;

    #[test]
    fn removes_markdown_without_losing_speech_markers() {
        let source = "# Tomorrow, **and tomorrow**\n- [Macbeth](https://example.com) [sigh] speaks.\n![cover](cover.png)\n```rust\nnot spoken\n```\n> *Out*, damned spot! [pause:500]";
        assert_eq!(lines(source), vec!["Tomorrow, and tomorrow", "Macbeth [sigh] speaks.", "Out, damned spot! [pause:500]"]);
    }
}
