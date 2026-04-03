/// SQLite-backed PKG storage.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, params};

use crate::fsrs;
use crate::node::{Kind, Node};
use crate::obsidian;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS session_state (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_log (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    task      TEXT NOT NULL,
    mode      TEXT NOT NULL DEFAULT 'manual',
    logged_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS nodes (
    id               TEXT PRIMARY KEY,
    topic            TEXT NOT NULL,
    kind             TEXT NOT NULL DEFAULT 'concept',
    description      TEXT NOT NULL DEFAULT '',
    difficulty       REAL NOT NULL DEFAULT 0.3,
    stability        REAL NOT NULL DEFAULT 2.0,
    last_reviewed    TEXT NOT NULL,
    last_encountered TEXT NOT NULL,
    review_count     INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS contexts (
    node_id  TEXT NOT NULL,
    context  TEXT NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (node_id, context),
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
);
";

pub struct Db {
    path: PathBuf,
    pub vault_dir: PathBuf,
}

impl Db {
    pub fn open(path: &Path, vault_dir: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Self { path: path.to_path_buf(), vault_dir: vault_dir.to_path_buf() };
        db.init()?;
        Ok(db)
    }

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)
            .with_context(|| format!("opening {:?}", self.path))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(conn)
    }

    fn init(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute_batch(SCHEMA)?;
        Ok(())
    }

    fn node_id(topic: &str) -> String {
        topic.to_lowercase().trim().replace(' ', "-")
    }

    fn today() -> String {
        Local::now().date_naive().to_string()
    }

    fn now() -> String {
        Local::now().naive_local().to_string()
    }

    fn parse_date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or_else(|_| Local::now().date_naive())
    }

    fn row_to_node(conn: &Connection, row: &rusqlite::Row) -> rusqlite::Result<Node> {
        let id: String = row.get("id")?;
        let mut stmt = conn.prepare(
            "SELECT context FROM contexts WHERE node_id = ? ORDER BY added_at",
        )?;
        let contexts: Vec<String> = stmt
            .query_map(params![id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(Node {
            id: id.clone(),
            topic: row.get("topic")?,
            kind: Kind::from_str(&row.get::<_, String>("kind")?),
            description: row.get("description")?,
            difficulty: row.get("difficulty")?,
            stability: row.get("stability")?,
            last_reviewed: Self::parse_date(&row.get::<_, String>("last_reviewed")?),
            last_encountered: Self::parse_date(&row.get::<_, String>("last_encountered")?),
            review_count: row.get("review_count")?,
            contexts,
            created_at: Self::parse_date(&row.get::<_, String>("created_at")?),
        })
    }

    // ── nodes ─────────────────────────────────────────────────────────────────

    pub fn get_node(&self, topic: &str) -> Result<Option<Node>> {
        let conn = self.connect()?;
        let node_id = Self::node_id(topic);
        let mut stmt = conn.prepare("SELECT * FROM nodes WHERE id = ?")?;
        let mut rows = stmt.query(params![node_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_node(&conn, row)?))
        } else {
            Ok(None)
        }
    }

    pub fn all_nodes(&self) -> Result<Vec<Node>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT * FROM nodes ORDER BY topic")?;
        let nodes = stmt
            .query_map([], |row| Self::row_to_node(&conn, row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(nodes)
    }

    pub fn add_or_update(
        &self,
        topic: &str,
        score: f64,
        kind: &Kind,
        description: &str,
        context: &str,
        last_reviewed_override: Option<NaiveDate>,
    ) -> Result<()> {
        let node_id = Self::node_id(topic);
        let reviewed = last_reviewed_override
            .unwrap_or_else(|| Local::now().date_naive())
            .to_string();
        let today = Self::today();

        let conn = self.connect()?;

        let existing = {
            let mut stmt = conn.prepare("SELECT * FROM nodes WHERE id = ?")?;
            let mut rows = stmt.query(params![node_id])?;
            if let Some(row) = rows.next()? {
                Some(Self::row_to_node(&conn, row)?)
            } else {
                None
            }
        };

        if let Some(node) = existing {
            let r = fsrs::retrievability(node.stability, node.last_reviewed);
            let (new_s, new_d) = fsrs::update_after_review(node.stability, node.difficulty, r, score);
            conn.execute(
                "UPDATE nodes SET
                    stability        = ?1,
                    difficulty       = ?2,
                    last_reviewed    = ?3,
                    last_encountered = ?4,
                    review_count     = review_count + 1,
                    kind             = ?5,
                    description      = CASE WHEN description = '' THEN ?6 ELSE description END
                 WHERE id = ?7",
                params![new_s, new_d, reviewed, today, kind.as_str(), description, node_id],
            )?;
        } else {
            let mut stability = fsrs::initial_stability(kind);
            if score < 0.4 {
                stability *= 0.3;
            } else if score < 0.65 {
                stability *= 0.6;
            }
            conn.execute(
                "INSERT INTO nodes
                    (id, topic, kind, description, difficulty, stability,
                     last_reviewed, last_encountered, review_count, created_at)
                 VALUES (?1, ?2, ?3, ?4, 0.3, ?5, ?6, ?7, 1, ?8)",
                params![node_id, topic, kind.as_str(), description, stability, reviewed, today, today],
            )?;
        }

        if !context.is_empty() {
            conn.execute(
                "INSERT OR IGNORE INTO contexts (node_id, context, added_at) VALUES (?, ?, ?)",
                params![node_id, context, today],
            )?;
        }

        // Write Obsidian vault file (best-effort — never block on failure)
        if let Ok(Some(node)) = self.get_node(topic) {
            obsidian::write_node(&node, &self.vault_dir).ok();
        }

        Ok(())
    }

    pub fn mark_encountered(&self, topic: &str) -> Result<()> {
        let node_id = Self::node_id(topic);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE nodes SET last_encountered = ? WHERE id = ?",
            params![Self::today(), node_id],
        )?;
        Ok(())
    }

    // ── session state ─────────────────────────────────────────────────────────

    pub fn session_get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT value FROM session_state WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn session_set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO session_state (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, Self::now()],
        )?;
        Ok(())
    }

    // ── task log ──────────────────────────────────────────────────────────────

    pub fn log_task(&self, task: &str, mode: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO task_log (task, mode, logged_at) VALUES (?, ?, ?)",
            params![task, mode, Self::now()],
        )?;
        Ok(())
    }

    pub fn recent_tasks(&self, hours: u32) -> Result<Vec<String>> {
        let cutoff = (Local::now() - chrono::Duration::hours(hours as i64))
            .naive_local()
            .to_string();
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT task FROM task_log WHERE logged_at >= ? AND mode = 'claude-code' ORDER BY logged_at DESC",
        )?;
        let tasks = stmt
            .query_map(params![cutoff], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tasks)
    }

    // ── search / delete ───────────────────────────────────────────────────────

    pub fn search_nodes(&self, query: &str) -> Result<Vec<Node>> {
        let conn = self.connect()?;
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE LOWER(topic) LIKE ? OR LOWER(description) LIKE ? ORDER BY topic",
        )?;
        let nodes = stmt
            .query_map(params![pattern, pattern], |row| Self::row_to_node(&conn, row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(nodes)
    }

    pub fn delete_node(&self, node_id: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM nodes WHERE id = ?", params![node_id])?;
        Ok(())
    }

    // ── summary ───────────────────────────────────────────────────────────────

    pub fn summary(&self) -> Result<(usize, usize, usize, usize)> {
        let nodes = self.all_nodes()?;
        let total = nodes.len();
        let known = nodes
            .iter()
            .filter(|n| fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) == "known")
            .count();
        let stale = nodes
            .iter()
            .filter(|n| fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) == "stale")
            .count();
        let gaps = total - known - stale;
        Ok((total, known, stale, gaps))
    }
}
