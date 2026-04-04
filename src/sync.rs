/// PKG version control and push reminder.
///
/// Rocky tracks the PKG directory as a git repo. The DB is excluded via
/// .gitignore. pkg.json (inside the PKG dir) is the machine-readable backup.
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::config::SyncConfig;
use crate::db::Db;

// ── Git helpers ───────────────────────────────────────────────────────────────

fn git(dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|_| anyhow!("git not found — install git to use sync features"))
}

fn git_ok(dir: &Path, args: &[&str]) -> Result<String> {
    let out = git(dir, args)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

// ── Setup ─────────────────────────────────────────────────────────────────────

/// Initialise a git repo in the rocky dir (~/.rocky).
/// Writes .gitignore to exclude graph.db. Returns true if newly created.
pub fn ensure_repo(rocky_dir: &Path) -> Result<bool> {
    let new = !is_git_repo(rocky_dir);
    if new {
        git_ok(rocky_dir, &["init"])?;
    }
    // Always ensure .gitignore is correct
    let gitignore = rocky_dir.join(".gitignore");
    let existing = std::fs::read_to_string(&gitignore).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().collect();
    for entry in &["graph.db", "*.db-shm", "*.db-wal"] {
        if !lines.contains(entry) {
            lines.push(entry);
        }
    }
    std::fs::write(&gitignore, lines.join("\n") + "\n")?;
    Ok(new)
}

/// Configure a remote. Replaces origin if it already exists.
pub fn set_remote(rocky_dir: &Path, name: &str, url: &str) -> Result<()> {
    // Remove if exists, then add
    let _ = git(rocky_dir, &["remote", "remove", name]);
    git_ok(rocky_dir, &["remote", "add", name, url])?;
    Ok(())
}

// ── Commit ────────────────────────────────────────────────────────────────────

/// Stage all changes in the rocky dir and commit. Returns true if there was
/// anything to commit.
pub fn commit(rocky_dir: &Path, message: &str) -> Result<bool> {
    git_ok(rocky_dir, &["add", "-A"])?;

    // Check if there's actually anything staged
    let status = git_ok(rocky_dir, &["status", "--porcelain"])?;
    if status.is_empty() {
        return Ok(false); // nothing to commit
    }

    git_ok(rocky_dir, &["commit", "--no-gpg-sign", "-m", message])?;
    Ok(true)
}

/// Push to the configured remote and branch.
pub fn push(rocky_dir: &Path, remote: &str, branch: &str) -> Result<()> {
    git_ok(rocky_dir, &["push", remote, branch])?;
    Ok(())
}

/// Show a short status summary (staged/unstaged file counts).
pub fn status_summary(rocky_dir: &Path) -> String {
    if !is_git_repo(rocky_dir) {
        return "not a git repo — run `rocky sync --init` to set up".to_string();
    }
    let status = git_ok(rocky_dir, &["status", "--porcelain"])
        .unwrap_or_default();
    if status.is_empty() {
        return "PKG up to date".to_string();
    }
    let changed = status.lines().count();
    format!("{changed} file{} changed since last commit", if changed == 1 { "" } else { "s" })
}

/// Returns the number of commits ahead of the remote (unpushed).
pub fn commits_ahead(rocky_dir: &Path, remote: &str, branch: &str) -> u32 {
    let spec = format!("{remote}/{branch}..HEAD");
    git_ok(rocky_dir, &["rev-list", "--count", &spec])
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

// ── Push reminder ─────────────────────────────────────────────────────────────

/// Increment the session counter in the DB. Call after each auto-commit.
pub fn increment_sessions_since_push(db: &Db) -> Result<()> {
    let current: u32 = db.session_get("sessions_since_push")?
        .unwrap_or_default().parse().unwrap_or(0);
    db.session_set("sessions_since_push", &(current + 1).to_string())?;
    if db.session_get("first_commit_date")?.is_none() {
        let today = chrono::Local::now().date_naive().to_string();
        db.session_set("first_commit_date", &today)?;
    }
    Ok(())
}

/// Reset the push counter. Call after a successful push.
pub fn reset_push_counter(db: &Db) -> Result<()> {
    db.session_set("sessions_since_push", "0")?;
    let today = chrono::Local::now().date_naive().to_string();
    db.session_set("last_push_date", &today)?;
    Ok(())
}

/// Returns Some(reminder_message) if the user should be reminded to push.
pub fn push_reminder(db: &Db, cfg: &SyncConfig) -> Option<String> {
    if cfg.remind_push_sessions == 0 && cfg.remind_push_days == 0 {
        return None;
    }

    // Session-based reminder
    if cfg.remind_push_sessions > 0 {
        let sessions: u32 = db.session_get("sessions_since_push")
            .ok().flatten().unwrap_or_default().parse().unwrap_or(0);
        if sessions >= cfg.remind_push_sessions {
            return Some(format!(
                "♫ Rocky: {sessions} sessions unsynced — consider `rocky sync --push` to back up, question?"
            ));
        }
    }

    // Day-based reminder
    if cfg.remind_push_days > 0 {
        if let Ok(Some(last)) = db.session_get("last_push_date") {
            if let Ok(last_date) = chrono::NaiveDate::parse_from_str(&last, "%Y-%m-%d") {
                let today = chrono::Local::now().date_naive();
                let days = (today - last_date).num_days() as u32;
                if days >= cfg.remind_push_days {
                    return Some(format!(
                        "♫ Rocky: {days} days since last push — consider `rocky sync --push`, question?"
                    ));
                }
            }
        } else {
            // Never pushed — use first_commit_date as reference
            if let Ok(Some(first)) = db.session_get("first_commit_date") {
                if let Ok(first_date) = chrono::NaiveDate::parse_from_str(&first, "%Y-%m-%d") {
                    let today = chrono::Local::now().date_naive();
                    let days = (today - first_date).num_days() as u32;
                    if days >= cfg.remind_push_days {
                        return Some(format!(
                            "♫ Rocky: {days} days of commits never pushed — run `rocky sync --push`, question?"
                        ));
                    }
                }
            }
        }
    }

    None
}
