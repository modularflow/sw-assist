use crate::llm::{ChatMessage, Usage};
use crate::util;
use anyhow::{Context, Result};
use dirs::data_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write as _};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const APP_DIR_NAME: &str = "sw-assistant";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub timestamp_ms: i64,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub name: String,
    pub path: PathBuf,
    pub last_used_ms: Option<i64>,
    pub num_lines: usize,
    pub file_size: u64,
}

pub fn data_base_dir() -> Result<PathBuf> {
    let base = data_dir().context("unable to resolve OS data directory")?;
    Ok(base.join(APP_DIR_NAME))
}

pub fn sessions_dir() -> Result<PathBuf> {
    Ok(data_base_dir()?.join("sessions"))
}

pub fn active_session_path() -> Result<PathBuf> {
    Ok(data_base_dir()?.join("active_session"))
}

pub fn ensure_sessions_dir() -> Result<()> {
    let dir = sessions_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("creating sessions dir: {}", dir.display()))?;
    Ok(())
}

pub fn session_file_path(name: &str) -> Result<PathBuf> {
    Ok(sessions_dir()?.join(format!("{}.jsonl", name)))
}

pub fn set_active_session(name: &str) -> Result<()> {
    let path = active_session_path()?;
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    fs::write(&path, name).with_context(|| format!("writing active session: {}", path.display()))?;
    Ok(())
}

pub fn get_active_session() -> Result<Option<String>> {
    let path = active_session_path()?;
    if !path.exists() { return Ok(None); }
    let name = fs::read_to_string(&path).with_context(|| format!("reading active session: {}", path.display()))?;
    Ok(Some(name.trim().to_string()))
}

pub fn create_session_if_missing(name: &str) -> Result<PathBuf> {
    ensure_sessions_dir()?;
    let path = session_file_path(name)?;
    if !path.exists() {
        OpenOptions::new().create(true).append(true).open(&path)
            .with_context(|| format!("creating session file: {}", path.display()))?;
    }
    Ok(path)
}

pub fn append_record(name: &str, record: &SessionRecord) -> Result<()> {
    let path = create_session_if_missing(name)?;
    let mut f = OpenOptions::new().create(true).append(true).open(&path)
        .with_context(|| format!("opening session for append: {}", path.display()))?;
    let line = serde_json::to_string(record)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

pub fn now_ms() -> i64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    (now.as_millis() as i128).min(i64::MAX as i128) as i64
}

pub fn list_sessions_metadata() -> Result<Vec<SessionMeta>> {
    let dir = sessions_dir()?;
    ensure_sessions_dir()?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("listing sessions: {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") { continue; }
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let meta_fs = entry.metadata()?;
        let file_size = meta_fs.len();
        let (num_lines, last_used_ms) = read_tail_meta(&path)?;
        out.push(SessionMeta { name, path, last_used_ms, num_lines, file_size });
    }
    out.sort_by(|a, b| b.last_used_ms.cmp(&a.last_used_ms));
    Ok(out)
}

fn read_tail_meta(path: &Path) -> Result<(usize, Option<i64>)> {
    let content = fs::read_to_string(path)?;
    let mut last_ts: Option<i64> = None;
    let mut num = 0usize;
    for line in content.lines() {
        num += 1;
        if let Ok(rec) = serde_json::from_str::<SessionRecord>(line) {
            last_ts = Some(rec.timestamp_ms);
        }
    }
    Ok((num, last_ts))
}

pub fn load_session_history(name: &str) -> Result<Vec<SessionRecord>> {
    let path = session_file_path(name)?;
    if !path.exists() { return Ok(vec![]); }
    let content = fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        match serde_json::from_str::<SessionRecord>(line) {
            Ok(r) => out.push(r),
            Err(_) => continue,
        }
    }
    Ok(out)
}

pub fn search_session(name: &str, needle: &str) -> Result<Vec<SessionRecord>> {
    let hist = load_session_history(name)?;
    let needle_lower = needle.to_lowercase();
    Ok(hist
        .into_iter()
        .filter(|r| r.content.to_lowercase().contains(&needle_lower))
        .collect())
}

pub fn build_messages_with_truncation(
    history: &[SessionRecord],
    new_user_message: &str,
    max_tokens: usize,
) -> Vec<ChatMessage> {
    let mut messages: Vec<ChatMessage> = Vec::new();
    for rec in history.iter() {
        messages.push(ChatMessage { role: rec.role.clone(), content: rec.content.clone() });
    }
    messages.push(ChatMessage { role: "user".to_string(), content: new_user_message.to_string() });
    // Truncate from the start by token budget
    let mut total = 0usize;
    let mut kept: Vec<ChatMessage> = Vec::new();
    for msg in messages.iter().rev() {
        let t = util::estimate_tokens_for_text(&msg.content);
        if total + t > max_tokens && !kept.is_empty() {
            break;
        }
        kept.push(ChatMessage { role: msg.role.clone(), content: msg.content.clone() });
        total += t;
    }
    kept.reverse();
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_smoke() {
        let mut hist = Vec::new();
        for i in 0..100 {
            hist.push(SessionRecord { timestamp_ms: now_ms(), role: "user".into(), content: format!("line {}", i), model: None, usage: None });
            hist.push(SessionRecord { timestamp_ms: now_ms(), role: "assistant".into(), content: format!("resp {}", i), model: None, usage: None });
        }
        let msgs = build_messages_with_truncation(&hist, "final question", 200);
        assert!(msgs.len() < hist.len() + 1);
        assert_eq!(msgs.last().unwrap().role, "user");
        assert!(msgs.last().unwrap().content.contains("final question"));
    }
}
