/// Per-project prompt log.
///
/// Stored in `.rocky` (SQLite) in the current working directory.
/// Only created when the project has Rocky hooks installed.
/// Prompts older than 24 hours are automatically deleted on every write.
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;
use rusqlite::{Connection, params};

const LOG_FILE: &str = ".rocky";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS prompts (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    prompt    TEXT NOT NULL,
    logged_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS queued_topics (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    topic       TEXT NOT NULL,
    kind        TEXT NOT NULL,
    description TEXT NOT NULL,
    context     TEXT NOT NULL,
    queued_at   TEXT NOT NULL
);
";

pub struct LocalLog {
    path: PathBuf,
}

impl LocalLog {
    /// Returns a LocalLog for the current directory if Rocky hooks are installed here.
    /// Returns None if this project hasn't opted in.
    pub fn open_if_configured() -> Option<Self> {
        if !is_hook_installed() {
            return None;
        }
        Some(Self { path: PathBuf::from(LOG_FILE) })
    }

    /// Opens an existing `.rocky` log regardless of hook status (for `rocky logs`).
    pub fn open_existing() -> Option<Self> {
        let path = PathBuf::from(LOG_FILE);
        if path.exists() { Some(Self { path }) } else { None }
    }

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(conn)
    }

    /// Log a prompt and delete entries older than 24 hours.
    pub fn log_prompt(&self, prompt: &str) -> Result<()> {
        let conn = self.connect()?;
        let now = Local::now().naive_local().to_string();
        conn.execute(
            "INSERT INTO prompts (prompt, logged_at) VALUES (?, ?)",
            params![prompt, now],
        )?;
        // Auto-cleanup: delete anything older than 24 hours
        let cutoff = (Local::now() - chrono::Duration::hours(24))
            .naive_local()
            .to_string();
        conn.execute("DELETE FROM prompts WHERE logged_at < ?", params![cutoff])?;
        Ok(())
    }

    pub fn queue_topic(&self, topic: &str, kind: &str, description: &str, context: &str) -> Result<()> {
        let conn = self.connect()?;
        let now = Local::now().naive_local().to_string();
        // Upsert — don't duplicate if already queued
        conn.execute(
            "INSERT OR IGNORE INTO queued_topics (topic, kind, description, context, queued_at)
             VALUES (?, ?, ?, ?, ?)",
            params![topic, kind, description, context, now],
        )?;
        Ok(())
    }

    pub fn get_queued_topics(&self) -> Result<Vec<(String, String, String, String)>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT topic, kind, description, context FROM queued_topics ORDER BY queued_at ASC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn remove_queued_topic(&self, topic: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM queued_topics WHERE topic = ?", params![topic])?;
        Ok(())
    }

    /// Fetch prompts from the last N hours.
    pub fn recent_prompts(&self, hours: u32) -> Result<Vec<(String, String)>> {
        let conn = self.connect()?;
        let cutoff = (Local::now() - chrono::Duration::hours(hours as i64))
            .naive_local()
            .to_string();
        let mut stmt = conn.prepare(
            "SELECT prompt, logged_at FROM prompts WHERE logged_at >= ? ORDER BY logged_at DESC",
        )?;
        let rows = stmt
            .query_map(params![cutoff], |r| Ok((r.get(0)?, r.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}

const PROMPT_MARKER: &str = ".rocky-project";

/// Returns true if this project has opted in to Rocky prompt logging.
/// Either via the git post-commit hook or via the lightweight `.rocky-project` marker.
pub fn is_hook_installed() -> bool {
    if Path::new(PROMPT_MARKER).exists() {
        return true;
    }
    let hook = Path::new(".git/hooks/post-commit");
    if !hook.exists() {
        return false;
    }
    std::fs::read_to_string(hook)
        .map(|s| s.contains("rocky"))
        .unwrap_or(false)
}

/// Create `.rocky-project` marker to opt this project in to prompt logging.
pub fn install_prompt_marker() -> anyhow::Result<(bool, String)> {
    if Path::new(PROMPT_MARKER).exists() {
        return Ok((false, "prompt logging already enabled for this project".into()));
    }
    std::fs::write(PROMPT_MARKER, "")?;
    ensure_gitignored()?;
    ensure_marker_gitignored()?;
    Ok((true, "prompt logging enabled for this project".into()))
}

/// Remove `.rocky-project` marker.
pub fn uninstall_prompt_marker() -> anyhow::Result<(bool, String)> {
    let path = Path::new(PROMPT_MARKER);
    if !path.exists() {
        return Ok((false, "prompt logging not enabled for this project".into()));
    }
    std::fs::remove_file(path)?;
    Ok((true, "prompt logging disabled for this project".into()))
}

fn ensure_marker_gitignored() -> anyhow::Result<()> {
    let gitignore = Path::new(".gitignore");
    if gitignore.exists() {
        let content = std::fs::read_to_string(gitignore)?;
        if content.lines().any(|l| l.trim() == PROMPT_MARKER) {
            return Ok(());
        }
        let updated = format!("{}\n{PROMPT_MARKER}\n", content.trim_end());
        std::fs::write(gitignore, updated)?;
    } else {
        std::fs::write(gitignore, format!("{PROMPT_MARKER}\n"))?;
    }
    Ok(())
}

/// Ensures `.rocky` is listed in `.gitignore`. Creates `.gitignore` if absent.
pub fn ensure_gitignored() -> Result<()> {
    let gitignore = Path::new(".gitignore");
    if gitignore.exists() {
        let content = std::fs::read_to_string(gitignore)?;
        if content.lines().any(|l| l.trim() == ".rocky") {
            return Ok(());
        }
        let updated = format!("{}\n.rocky\n", content.trim_end());
        std::fs::write(gitignore, updated)?;
    } else {
        std::fs::write(gitignore, ".rocky\n")?;
    }
    Ok(())
}
