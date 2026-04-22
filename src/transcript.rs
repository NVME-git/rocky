/// Reads Claude Code session transcripts from `~/.claude/projects/<encoded>/*.jsonl`.
///
/// Used by the Stop hook (`rocky session-end`) to enrich node generation with
/// Claude's reasoning, the docs it consulted, and any away_summary it emitted.
///
/// Privacy: this module deliberately strips file contents from tool results and
/// keeps only metadata (file paths, command names). Large code blocks in
/// assistant messages are also truncated.
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use serde_json::Value;

const MAX_ASSISTANT_CHARS: usize = 1500;
const MAX_PER_SESSION_FILE: u64 = 4 * 1024 * 1024;

#[derive(Debug, Default, Clone)]
pub struct TranscriptContext {
    pub user_prompts: Vec<String>,
    pub assistant_messages: Vec<String>,
    pub files_read: Vec<String>,
    pub commands_run: Vec<String>,
    pub away_summaries: Vec<String>,
    pub session_count: usize,
}

impl TranscriptContext {
    pub fn is_empty(&self) -> bool {
        self.user_prompts.is_empty()
            && self.assistant_messages.is_empty()
            && self.away_summaries.is_empty()
    }

    /// Format as a single context block suitable for inclusion in an LLM prompt.
    /// Capped to keep prompt size sane.
    pub fn to_prompt_block(&self) -> String {
        let mut out = String::new();

        if !self.away_summaries.is_empty() {
            out.push_str("Session summaries (written by the agent):\n");
            for s in &self.away_summaries {
                out.push_str("- ");
                out.push_str(s.trim());
                out.push('\n');
            }
            out.push('\n');
        }

        if !self.user_prompts.is_empty() {
            out.push_str("User prompts during the session:\n");
            for p in self.user_prompts.iter().take(20) {
                let line = p.trim().replace('\n', " ");
                out.push_str("- ");
                out.push_str(truncate(&line, 240));
                out.push('\n');
            }
            out.push('\n');
        }

        if !self.assistant_messages.is_empty() {
            out.push_str("Agent reasoning highlights:\n");
            for m in self.assistant_messages.iter().take(15) {
                let line = m.trim().replace('\n', " ");
                out.push_str("- ");
                out.push_str(truncate(&line, 320));
                out.push('\n');
            }
            out.push('\n');
        }

        if !self.files_read.is_empty() {
            out.push_str("Files the agent consulted:\n");
            let mut deduped: Vec<&String> = self.files_read.iter().collect();
            deduped.sort();
            deduped.dedup();
            for f in deduped.iter().take(40) {
                out.push_str("- ");
                out.push_str(f);
                out.push('\n');
            }
            out.push('\n');
        }

        if !self.commands_run.is_empty() {
            out.push_str("Commands the agent ran:\n");
            let mut deduped: Vec<&String> = self.commands_run.iter().collect();
            deduped.sort();
            deduped.dedup();
            for c in deduped.iter().take(20) {
                out.push_str("- ");
                out.push_str(truncate(c, 160));
                out.push('\n');
            }
            out.push('\n');
        }

        out
    }
}

/// Map a project working directory to its `~/.claude/projects/<encoded>` folder.
/// Claude Code encodes the path by replacing `/` with `-`.
fn encoded_project_dir(project_path: &Path) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let abs = project_path.canonicalize().ok()?;
    let s = abs.to_string_lossy();
    let encoded = s.replace('/', "-");
    Some(home.join(".claude").join("projects").join(encoded))
}

/// Find all session JSONL files for a given project path, sorted by mtime descending.
pub fn find_session_files(project_path: &Path) -> Result<Vec<PathBuf>> {
    let dir = match encoded_project_dir(project_path) {
        Some(d) => d,
        None => return Ok(vec![]),
    };
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = vec![];
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let mtime = entry.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        files.push((path, mtime));
    }
    files.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(files.into_iter().map(|(p, _)| p).collect())
}

/// Read the most recent session activity for a project within `hours` of now.
pub fn read_recent(project_path: &Path, hours: u32) -> Result<TranscriptContext> {
    let cutoff = Utc::now() - chrono::Duration::hours(hours as i64);
    let files = find_session_files(project_path)?;
    let mut ctx = TranscriptContext::default();

    for file in files {
        let meta = match std::fs::metadata(&file) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_PER_SESSION_FILE {
            // Cap individual file size to avoid pathological reads
            continue;
        }
        let text = match std::fs::read_to_string(&file) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mut produced_anything = false;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // Time gate
            if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                if let Ok(parsed) = DateTime::parse_from_rfc3339(ts) {
                    if parsed.with_timezone(&Utc) < cutoff {
                        continue;
                    }
                }
            }
            if extract_entry(&v, &mut ctx) {
                produced_anything = true;
            }
        }
        if produced_anything {
            ctx.session_count += 1;
        }
    }

    Ok(ctx)
}

/// Returns true if this entry contributed any content to ctx.
fn extract_entry(v: &Value, ctx: &mut TranscriptContext) -> bool {
    let entry_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match entry_type {
        "user" => {
            if let Some(content) = v.pointer("/message/content") {
                if let Some(s) = content.as_str() {
                    let stripped = strip_code(s);
                    if !stripped.trim().is_empty() {
                        ctx.user_prompts.push(stripped);
                        return true;
                    }
                }
                // Tool result entries appear as user messages with array content; extract metadata
                if let Some(arr) = content.as_array() {
                    let mut produced = false;
                    for item in arr {
                        if let Some("tool_result") = item.get("type").and_then(|t| t.as_str()) {
                            // We don't extract tool result content here — handled via toolUseResult sibling
                            let _ = item;
                            produced |= false;
                        }
                    }
                    return produced;
                }
            }
            false
        }
        "assistant" => {
            if let Some(content) = v.pointer("/message/content").and_then(|c| c.as_array()) {
                let mut produced = false;
                for item in content {
                    let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if item_type == "text" {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            let cleaned = strip_code(text);
                            let cleaned = truncate(&cleaned, MAX_ASSISTANT_CHARS).to_string();
                            if !cleaned.trim().is_empty() {
                                ctx.assistant_messages.push(cleaned);
                                produced = true;
                            }
                        }
                    } else if item_type == "tool_use" {
                        let tool = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let input = item.get("input");
                        match tool {
                            "Read" => {
                                if let Some(path) = input.and_then(|i| i.get("file_path")).and_then(|p| p.as_str()) {
                                    ctx.files_read.push(path.to_string());
                                    produced = true;
                                }
                            }
                            "Bash" => {
                                if let Some(cmd) = input.and_then(|i| i.get("command")).and_then(|c| c.as_str()) {
                                    ctx.commands_run.push(cmd.to_string());
                                    produced = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                return produced;
            }
            false
        }
        "system" => {
            if v.get("subtype").and_then(|s| s.as_str()) == Some("away_summary") {
                if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                    ctx.away_summaries.push(content.to_string());
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Strip fenced code blocks and very long indented code from a text body.
/// Keeps prose and short inline code intact.
fn strip_code(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_fence = false;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            if !in_fence {
                out.push_str("[code omitted]\n");
            }
            continue;
        }
        if in_fence {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

#[allow(dead_code)]
pub fn now_local_string() -> String {
    Local::now().naive_local().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_removes_fenced_blocks() {
        let input = "Here is some prose.\n```rust\nfn main() {}\n```\nMore prose.";
        let out = strip_code(input);
        assert!(out.contains("Here is some prose."));
        assert!(out.contains("More prose."));
        assert!(out.contains("[code omitted]"));
        assert!(!out.contains("fn main()"));
    }

    #[test]
    fn truncate_respects_char_boundary() {
        let s = "héllo world";
        let t = truncate(s, 5);
        // Should not panic and should be valid utf8
        assert!(t.len() <= 5);
    }

    #[test]
    fn empty_context_is_empty() {
        let c = TranscriptContext::default();
        assert!(c.is_empty());
        assert!(c.to_prompt_block().is_empty());
    }

    #[test]
    fn assistant_message_extraction() {
        let line = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "I chose Axum because of its async ergonomics."}
                ]
            },
            "timestamp": "2099-01-01T00:00:00.000Z"
        });
        let mut ctx = TranscriptContext::default();
        let produced = extract_entry(&line, &mut ctx);
        assert!(produced);
        assert_eq!(ctx.assistant_messages.len(), 1);
        assert!(ctx.assistant_messages[0].contains("Axum"));
    }

    #[test]
    fn tool_use_read_extracts_path() {
        let line = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "tool_use", "name": "Read", "input": {"file_path": "/docs/decisions/0001.md"}}
                ]
            }
        });
        let mut ctx = TranscriptContext::default();
        extract_entry(&line, &mut ctx);
        assert_eq!(ctx.files_read, vec!["/docs/decisions/0001.md"]);
    }

    #[test]
    fn away_summary_extracted() {
        let line = serde_json::json!({
            "type": "system",
            "subtype": "away_summary",
            "content": "Refactored the auth module."
        });
        let mut ctx = TranscriptContext::default();
        extract_entry(&line, &mut ctx);
        assert_eq!(ctx.away_summaries, vec!["Refactored the auth module."]);
    }
}
