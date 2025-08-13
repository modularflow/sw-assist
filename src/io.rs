use anyhow::{Context, Result};
use std::path::Path;
pub async fn read_file_to_string_async(path: &Path) -> Result<String> {
    let data = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("reading file: {}", path.display()))?;
    Ok(data)
}

pub async fn read_file_segment_range_async(path: &Path, start: usize, end: usize) -> Result<String> {
    // start/end are 1-based inclusive line numbers
    let text = read_file_to_string_async(path).await?;
    let mut result = String::new();
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        if line_no >= start && line_no <= end {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

/// Chunk text by approximate token limit. Returns Vec of (chunk_index, text)
pub fn chunk_text_for_token_limit(text: &str, max_tokens_per_chunk: usize) -> Vec<(usize, String)> {
    if text.is_empty() {
        return vec![];
    }
    // Heuristic: 1 token ~= 4 chars
    let approx_chars_per_token = 4usize;
    let max_chars = max_tokens_per_chunk.saturating_mul(approx_chars_per_token);
    if max_chars == 0 {
        return vec![(0, String::new())];
    }
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    let mut last_break = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'\n' || bytes[idx] == b' ' { last_break = idx; }
        if idx - start >= max_chars {
            let split = if last_break > start { last_break } else { idx };
            let piece = &text[start..split];
            chunks.push(piece.to_string());
            start = split + 1; // skip break char
            last_break = start;
        }
        idx += 1;
    }
    if start < text.len() {
        chunks.push(text[start..].to_string());
    }
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, s)| (i, s))
        .collect()
}

pub fn filename_only(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

pub async fn read_diff_file_async(path: &Path) -> Result<String> {
    let data = read_file_to_string_async(path).await?;
    // Basic validation: look for diff headers
    if !(data.contains("--- ") && data.contains("+++ ")) {
        // still return content; caller can decide
        return Ok(data);
    }
    Ok(data)
}

pub fn scan_todos(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let ln = i + 1;
        let upper = line.to_uppercase();
        if upper.contains("TODO") || upper.contains("FIXME") || upper.contains("NOTE:") || upper.starts_with("NOTE") {
            out.push((ln, line.trim().to_string()));
        }
    }
    out
}
