//! PromptIQ — global prompt log + heuristic scorer + agent-rescore plumbing.
//!
//! Stores prompts at `~/.rocky/prompts.db`. Each row carries a heuristic score
//! computed at log time (~1 ms, no network) and an optional `llm_score` written
//! later by `/rocky-promptiq`.
//!
//! See ADR 0007 for rationale.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{Local, Duration};
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::db::Db;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS prompts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ts              TEXT NOT NULL,
    source          TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    prompt          TEXT NOT NULL,
    heuristic_score INTEGER NOT NULL,
    llm_score       INTEGER,
    feedback        TEXT,
    dimensions      TEXT NOT NULL,
    evaluated_at    TEXT
);

CREATE INDEX IF NOT EXISTS prompts_ts ON prompts(ts);
CREATE INDEX IF NOT EXISTS prompts_unscored ON prompts(llm_score) WHERE llm_score IS NULL;

-- Set once after we've migrated per-project ./.rocky logs into this global table.
CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
";

fn db_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ROCKY_HOME") {
        if !custom.is_empty() {
            return PathBuf::from(custom).join("prompts.db");
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".rocky").join("prompts.db")
}

fn connect() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path).context("open prompts.db")?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

// ── Heuristic scorer ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct HeuristicScore {
    pub total: i64,
    pub specificity: i64,
    pub context: i64,
    pub actionability: i64,
    pub verification: i64,
    pub anti_pattern_penalty: i64,
}

pub fn heuristic_score(text: &str) -> Option<HeuristicScore> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let word_count = words.len();
    let char_count = trimmed.chars().count();

    // Specificity (max 30): file paths, symbol-like identifiers, framework/lib refs.
    let mut specificity: i64 = 0;
    let path_hits = trimmed.matches('/').count() + trimmed.matches('\\').count();
    let dot_ext_hits = ["src/", ".rs", ".ts", ".js", ".py", ".go", ".tsx", ".html", ".css", ".md", ".sql", ".json", ".toml", ".yaml", ".yml"]
        .iter().map(|p| lower.matches(p).count()).sum::<usize>();
    let backtick_spans = trimmed.matches('`').count() / 2;
    let camel_or_snake = trimmed.split_whitespace()
        .filter(|w| {
            let bytes = w.as_bytes();
            bytes.iter().any(|b| b.is_ascii_uppercase()) && bytes.iter().any(|b| b.is_ascii_lowercase())
                || w.contains('_') && w.chars().any(|c| c.is_ascii_alphabetic())
        })
        .count();
    let frameworks_hit = ["mern", "react", "next.js", "nextjs", "vue", "axum", "tokio", "django", "flask",
                          "fastapi", "express", "spring", "rails", "postgres", "mysql", "redis", "kafka",
                          "kubernetes", "docker", "terraform", "ollama", "claude", "anthropic", "openai",
                          "rust", "python", "typescript", "golang", "java"]
        .iter().filter(|k| lower.contains(*k)).count();
    if path_hits >= 1 { specificity += 8; }
    if dot_ext_hits >= 1 { specificity += 8; }
    if backtick_spans >= 1 { specificity += 6; }
    if camel_or_snake >= 1 { specificity += 4; }
    if frameworks_hit >= 1 { specificity += 4; }
    specificity = specificity.min(30);

    // Context (max 20): length above threshold + state-reference cues.
    let mut context: i64 = 0;
    if char_count >= 60 { context += 6; }
    if char_count >= 200 { context += 6; }
    let context_cues = ["currently", "right now", "today", "the current", "as of", "because", "since",
                        "the existing", "in production", "in our", "the system", "the app", "in main"]
        .iter().filter(|k| lower.contains(*k)).count();
    if context_cues >= 1 { context += 4; }
    if context_cues >= 2 { context += 4; }
    context = context.min(20);

    // Actionability (max 20): imperative verbs, explicit deliverables.
    let mut actionability: i64 = 0;
    let verbs = ["add", "remove", "delete", "fix", "refactor", "implement", "build", "create",
                 "update", "modify", "rename", "extract", "inline", "move", "split",
                 "wire", "expose", "document", "test", "ensure", "make sure", "convert", "migrate"];
    let verb_hits = verbs.iter().filter(|v| lower.contains(*v)).count();
    if verb_hits >= 1 { actionability += 8; }
    if verb_hits >= 2 { actionability += 4; }
    let deliverable_cues = ["should", "must", "needs to", "expected to", "so that", "in order to", "such that"];
    let deliv_hits = deliverable_cues.iter().filter(|k| lower.contains(*k)).count();
    if deliv_hits >= 1 { actionability += 8; }
    actionability = actionability.min(20);

    // Verification (max 15): tests, edge cases, error handling, acceptance criteria.
    let mut verification: i64 = 0;
    let verify_cues = ["test", "edge case", "error path", "error handling", "exception",
                       "acceptance criteria", "coverage", "regression", "validate", "assert"];
    let v_hits = verify_cues.iter().filter(|k| lower.contains(*k)).count();
    if v_hits >= 1 { verification += 8; }
    if v_hits >= 2 { verification += 7; }
    verification = verification.min(15);

    // Anti-patterns (penalty up to -20): vague phrasing without follow-up.
    let mut penalty: i64 = 0;
    let anti = ["build me an app", "build me a app", "build an app", "make it work", "fix it",
                "fix the bug", "do everything", "make this better", "improve it", "clean up",
                "do something", "anything", "handle it", "do the needful"];
    for phrase in anti {
        if lower.contains(phrase) { penalty -= 6; }
    }
    // Very short prompts (≤ 4 words) with no specificity get an extra hit.
    if word_count <= 4 && specificity == 0 { penalty -= 8; }
    penalty = penalty.max(-20);

    let mut total = specificity + context + actionability + verification + penalty;
    total = total.clamp(0, 100);

    Some(HeuristicScore {
        total,
        specificity,
        context,
        actionability,
        verification,
        anti_pattern_penalty: penalty,
    })
}

/// One-line human readable critique of the dominant weakness — used in
/// "immediate" feedback mode and inside the dashboard tooltip.
pub fn heuristic_top_issue(_text: &str, s: &HeuristicScore) -> String {
    if s.anti_pattern_penalty <= -10 { return "weak: vague phrasing, no follow-up".into(); }
    if s.specificity < 6      { return "weak: no file/function/framework references".into(); }
    if s.actionability < 6    { return "weak: no clear deliverable".into(); }
    if s.context < 4          { return "weak: no context — what's the state?".into(); }
    if s.verification < 4     { return "could add: how would you verify the change?".into(); }
    String::new()
}

// ── Logging + reading ────────────────────────────────────────────────────────

pub fn log_prompt(text: &str, source: &str, project_path: &str) -> Result<i64> {
    let now = Local::now().naive_local().to_string();
    let h = heuristic_score(text);
    let total = h.as_ref().map(|x| x.total).unwrap_or(0);
    let dimensions_json = match &h {
        Some(x) => serde_json::to_string(x).unwrap_or("{}".into()),
        None => "{}".into(),
    };
    let conn = connect()?;
    conn.execute(
        "INSERT INTO prompts (ts, source, project_path, prompt, heuristic_score, dimensions)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![now, source, project_path, text, total, dimensions_json],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn set_llm_score(id: i64, score: i64, feedback: Option<&str>) -> Result<()> {
    let conn = connect()?;
    let now = Local::now().naive_local().to_string();
    conn.execute(
        "UPDATE prompts SET llm_score = ?, feedback = ?, evaluated_at = ? WHERE id = ?",
        params![score, feedback.unwrap_or(""), now, id],
    )?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct PromptRecord {
    pub id: i64,
    pub ts: String,
    pub source: String,
    pub project_path: String,
    pub prompt: String,
    pub heuristic_score: i64,
    pub llm_score: Option<i64>,
    pub feedback: Option<String>,
}

pub fn list_prompts(unscored_only: bool, since_days: i64) -> Result<Vec<PromptRecord>> {
    let conn = connect()?;
    let cutoff = (Local::now() - Duration::days(since_days)).naive_local().to_string();
    let mut sql = String::from(
        "SELECT id, ts, source, project_path, prompt, heuristic_score, llm_score, feedback
         FROM prompts WHERE ts >= ?"
    );
    if unscored_only {
        sql.push_str(" AND llm_score IS NULL");
    }
    sql.push_str(" ORDER BY ts DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![cutoff], |r| {
        Ok(PromptRecord {
            id: r.get(0)?,
            ts: r.get(1)?,
            source: r.get(2)?,
            project_path: r.get(3)?,
            prompt: r.get(4)?,
            heuristic_score: r.get(5)?,
            llm_score: r.get::<_, Option<i64>>(6)?,
            feedback: r.get::<_, Option<String>>(7)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ── Score computation (the headline KPI) ─────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PromptIqReport {
    pub current: i64,            // mean of past 7 days (best of llm/heuristic per prompt)
    pub previous: i64,           // mean of the 7 days before that
    pub delta: i64,              // current - previous
    pub sample_size: i64,
    pub by_source: Vec<SourceBreakdown>,
    pub recent_low_score: Option<PromptRecord>,
    pub recent_high_score: Option<PromptRecord>,
    /// Last ~20 prompts (most recent first) for the dashboard tile's modal.
    pub recent: Vec<PromptRecord>,
    /// MAX(evaluated_at) within the last-7-days window — when an LLM rescore
    /// last touched the score floor. None if no rescore yet.
    pub last_rescored_at: Option<String>,
}

/// True if the prompt represents real user input (something they typed to
/// direct the agent). False for slash-command-only invocations and for
/// auto-injected harness blocks (task-notification wrappers) that get logged
/// as "user prompts" by the hook even though the user never typed them.
fn is_real_prompt(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() { return false; }
    // Single-line slash-command invocations: `/rocky-checkpoint`, `/loop`, etc.
    if t.starts_with('/') && !t.contains('\n') { return false; }
    // Entire prompt is a harness-injected notification block.
    if t.starts_with("<task-notification>") && t.ends_with("</task-notification>") {
        return false;
    }
    true
}

#[derive(Debug, Serialize)]
pub struct SourceBreakdown {
    pub source: String,
    pub mean_score: f64,
    pub count: i64,
}

fn effective_score(heuristic: i64, llm: Option<i64>) -> i64 {
    llm.unwrap_or(heuristic)
}

pub fn current_iq() -> Result<PromptIqReport> {
    let conn = connect()?;
    let now = Local::now();
    let last7_start = (now - Duration::days(7)).naive_local().to_string();
    let prev7_start = (now - Duration::days(14)).naive_local().to_string();

    let mut q = conn.prepare(
        "SELECT heuristic_score, llm_score, source, ts, id, project_path, prompt, feedback, evaluated_at
         FROM prompts WHERE ts >= ?"
    )?;
    let mut prev_q = conn.prepare(
        "SELECT heuristic_score, llm_score, prompt FROM prompts WHERE ts >= ? AND ts < ?"
    )?;

    // Last-7 window. `is_real_prompt` filters out skill invocations and
    // harness-injected notification blocks so the IQ tracks user intent only.
    let last7_all: Vec<(i64, Option<i64>, String, String, i64, String, String, Option<String>, Option<String>)> =
        q.query_map(params![last7_start.clone()], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?))
        })?.filter_map(|r| r.ok()).collect();
    let last7: Vec<_> = last7_all.into_iter().filter(|r| is_real_prompt(&r.6)).collect();

    let prev: Vec<(i64, Option<i64>)> = prev_q
        .query_map(params![prev7_start, last7_start], |r| -> rusqlite::Result<(i64, Option<i64>, String)> {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .filter(|r| is_real_prompt(&r.2))
        .map(|(h, l, _)| (h, l))
        .collect();

    let current = if last7.is_empty() {
        0
    } else {
        let sum: i64 = last7.iter().map(|(h, l, ..)| effective_score(*h, *l)).sum();
        sum / last7.len() as i64
    };
    let previous = if prev.is_empty() {
        0
    } else {
        let sum: i64 = prev.iter().map(|(h, l)| effective_score(*h, *l)).sum();
        sum / prev.len() as i64
    };

    // Per-source breakdown over the last 7 days.
    use std::collections::HashMap;
    let mut by_src: HashMap<String, (i64, i64)> = HashMap::new();
    for (h, l, src, ..) in &last7 {
        let entry = by_src.entry(src.clone()).or_insert((0, 0));
        entry.0 += effective_score(*h, *l);
        entry.1 += 1;
    }
    let mut by_source: Vec<SourceBreakdown> = by_src
        .into_iter()
        .map(|(source, (sum, count))| SourceBreakdown {
            source,
            mean_score: if count == 0 { 0.0 } else { sum as f64 / count as f64 },
            count,
        })
        .collect();
    by_source.sort_by(|a, b| b.count.cmp(&a.count));

    // Best/worst prompts in the last 7 days for surfacing.
    let to_record = |r: &(i64, Option<i64>, String, String, i64, String, String, Option<String>, Option<String>)| -> PromptRecord {
        PromptRecord {
            id: r.4, ts: r.3.clone(), source: r.2.clone(), project_path: r.5.clone(),
            prompt: r.6.clone(), heuristic_score: r.0, llm_score: r.1, feedback: r.7.clone(),
        }
    };
    let recent_low = last7.iter().min_by_key(|r| effective_score(r.0, r.1)).map(to_record);
    let recent_high = last7.iter().max_by_key(|r| effective_score(r.0, r.1)).map(to_record);

    // Most-recent slice for the dashboard tile. SELECT had no ORDER BY, so
    // sort descending by ts here and cap at 20.
    let mut recent_sorted: Vec<&_> = last7.iter().collect();
    recent_sorted.sort_by(|a, b| b.3.cmp(&a.3));
    let recent: Vec<PromptRecord> = recent_sorted.into_iter().take(20).map(to_record).collect();

    // Most-recent rescore timestamp anywhere in the last-7 window.
    let last_rescored_at = last7
        .iter()
        .filter_map(|r| r.8.clone())
        .max();

    Ok(PromptIqReport {
        current,
        previous,
        delta: current - previous,
        sample_size: last7.len() as i64,
        by_source,
        recent_low_score: recent_low,
        recent_high_score: recent_high,
        recent,
        last_rescored_at,
    })
}

pub fn print_iq_human(r: &PromptIqReport) {
    use colored::Colorize;
    let arrow = if r.delta > 0 { format!("↑ {}", r.delta).green() }
                else if r.delta < 0 { format!("↓ {}", -r.delta).red() }
                else { "—".dimmed() };
    println!();
    println!("  {}: {}  ·  {}  this week (n={})",
        "PromptIQ".bold(),
        format!("{}", r.current).bold(),
        arrow,
        r.sample_size);
    if !r.by_source.is_empty() {
        println!("  {}", "by source:".dimmed());
        for s in &r.by_source {
            println!("    {:<12}  {:>5.1}  (n={})", s.source, s.mean_score, s.count);
        }
    }
}

pub fn print_recent_human(n: usize) -> Result<()> {
    use colored::Colorize;
    let prompts = list_prompts(false, 7)?;
    let take = prompts.iter().take(n);
    println!();
    println!("  {} ({})", "recent prompts".bold(), n.min(prompts.len()));
    for p in take {
        let score = effective_score(p.heuristic_score, p.llm_score);
        let trunc = if p.prompt.chars().count() > 90 {
            let s: String = p.prompt.chars().take(89).collect();
            format!("{s}…")
        } else {
            p.prompt.clone()
        };
        let llm_marker = if p.llm_score.is_some() { "✓" } else { "·" };
        println!("    [{score:>3}] {llm_marker} {} — {}",
            format!("{}", p.source).dimmed(),
            trunc);
        if let Some(f) = &p.feedback {
            if !f.is_empty() {
                println!("          {}", f.dimmed());
            }
        }
    }
    Ok(())
}

// ── Migration from per-project .rocky → global prompts.db ────────────────────

const MIGRATED_KEY: &str = "migrated_per_project_v1";

pub fn ensure_migrated(db: &Db) -> Result<()> {
    let conn = connect()?;
    let already: bool = conn
        .query_row("SELECT 1 FROM meta WHERE k = ?", params![MIGRATED_KEY], |_| Ok(true))
        .unwrap_or(false);
    if already { return Ok(()); }

    // Walk every repo Rocky knows about (from PKG nodes' repo field).
    // For each, look for a sibling .rocky SQLite file at the repo root and
    // import its prompts table if present.
    let nodes = db.all_nodes()?;
    let mut repos: std::collections::HashSet<String> = nodes.iter()
        .map(|n| n.repo.clone())
        .filter(|r| !r.is_empty())
        .collect();
    // The current dir's .rocky is a likely candidate too even if not in the PKG.
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(s) = cwd.to_str() { repos.insert(s.to_string()); }
    }

    let mut imported = 0i64;
    for repo in repos {
        // Find a path that holds a .rocky file. We have a name (repo) but not
        // necessarily a path. Best-effort: look in $HOME/Work/<repo> and a few
        // common locations. If user has a different layout, the migration just
        // skips — they can re-import later.
        let candidates = candidate_repo_paths(&repo);
        for candidate in candidates {
            let log = candidate.join(".rocky");
            if !log.exists() { continue; }
            imported += import_legacy_log(&log, candidate.to_string_lossy().as_ref()).unwrap_or(0);
            break;
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO meta (k, v) VALUES (?, ?)",
        params![MIGRATED_KEY, format!("imported {imported} prompts on first PromptIQ read")],
    )?;
    Ok(())
}

fn candidate_repo_paths(repo: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = dirs::home_dir() {
        out.push(home.join("Work").join(repo));
        out.push(home.join("work").join(repo));
        out.push(home.join("src").join(repo));
        out.push(home.join("dev").join(repo));
        out.push(home.join("Projects").join(repo));
        out.push(home.join("projects").join(repo));
    }
    out.push(PathBuf::from(repo));
    out
}

fn import_legacy_log(legacy_db_path: &std::path::Path, project_path: &str) -> Result<i64> {
    let src = Connection::open(legacy_db_path)?;
    // Old schema: prompts(id, prompt, logged_at)
    let mut stmt = match src.prepare("SELECT prompt, logged_at FROM prompts ORDER BY id") {
        Ok(s) => s,
        Err(_) => return Ok(0),  // table missing — nothing to do
    };
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let dst = connect()?;
    let mut count = 0i64;
    for (prompt, ts) in rows {
        let h = heuristic_score(&prompt);
        let total = h.as_ref().map(|x| x.total).unwrap_or(0);
        let dimensions_json = match &h {
            Some(x) => serde_json::to_string(x).unwrap_or("{}".into()),
            None => "{}".into(),
        };
        // Source = "claude" for legacy logs (the only thing that wrote to .rocky).
        // Best-effort dedupe: skip if (ts, prompt) already exists.
        let exists: bool = dst.query_row(
            "SELECT 1 FROM prompts WHERE ts = ? AND prompt = ? LIMIT 1",
            params![ts, prompt],
            |_| Ok(true),
        ).unwrap_or(false);
        if exists { continue; }
        dst.execute(
            "INSERT INTO prompts (ts, source, project_path, prompt, heuristic_score, dimensions)
             VALUES (?, 'claude', ?, ?, ?, ?)",
            params![ts, project_path, prompt, total, dimensions_json],
        )?;
        count += 1;
    }
    Ok(count)
}
