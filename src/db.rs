/// SQLite-backed PKG storage.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::fsrs;
use crate::node::{Kind, Node};
use crate::obsidian;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS session_state (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS nodes (
    id               TEXT PRIMARY KEY,
    topic            TEXT NOT NULL,
    kind             TEXT NOT NULL DEFAULT 'concept',
    domain           TEXT NOT NULL DEFAULT '',
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
        // Migration: add domain column to existing databases
        let _ = conn.execute(
            "ALTER TABLE nodes ADD COLUMN domain TEXT NOT NULL DEFAULT ''",
            [],
        );
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
            domain: row.get::<_, String>("domain").unwrap_or_default(),
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
        domain: &str,
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
            // Only update domain if the stored one is empty and we have a real value
            let effective_domain = if !domain.is_empty() && node.domain.is_empty() {
                domain
            } else if !node.domain.is_empty() {
                &node.domain
            } else {
                domain
            };
            conn.execute(
                "UPDATE nodes SET
                    stability        = ?1,
                    difficulty       = ?2,
                    last_reviewed    = ?3,
                    last_encountered = ?4,
                    review_count     = review_count + 1,
                    kind             = ?5,
                    domain           = ?6,
                    description      = CASE WHEN description = '' THEN ?7 ELSE description END
                 WHERE id = ?8",
                params![new_s, new_d, reviewed, today, kind.as_str(), effective_domain, description, node_id],
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
                    (id, topic, kind, domain, description, difficulty, stability,
                     last_reviewed, last_encountered, review_count, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0.3, ?6, ?7, ?8, 1, ?9)",
                params![node_id, topic, kind.as_str(), domain, description, stability, reviewed, today, today],
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

    /// Set the domain on an undomained node (used by classify command).
    pub fn set_domain(&self, node_id: &str, domain: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("UPDATE nodes SET domain = ? WHERE id = ?", params![domain, node_id])?;
        Ok(())
    }

    /// Returns nodes that have no domain assigned yet.
    pub fn undomained_nodes(&self) -> Result<Vec<Node>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT * FROM nodes WHERE domain = '' ORDER BY topic")?;
        let nodes = stmt
            .query_map([], |row| Self::row_to_node(&conn, row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(nodes)
    }

    // ── JSON backup / restore ─────────────────────────────────────────────────

    pub fn export_pkg_json(&self, path: &std::path::Path) -> Result<()> {
        let nodes = self.all_nodes()?;
        let conn = self.connect()?;

        let mut state_map = std::collections::HashMap::new();
        let mut stmt = conn.prepare("SELECT key, value FROM session_state")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows.flatten() { state_map.insert(row.0, row.1); }

        let export = PkgExport {
            version: 1,
            exported_at: Local::now().to_rfc3339(),
            session_state: state_map,
            nodes: nodes.iter().map(NodeExport::from_node).collect(),
        };

        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(path, serde_json::to_string_pretty(&export)?)?;
        Ok(())
    }

    pub fn import_pkg_json(&self, path: &std::path::Path) -> Result<usize> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let export: PkgExport = serde_json::from_str(&text)?;
        let conn = self.connect()?;

        conn.execute_batch("DELETE FROM contexts; DELETE FROM nodes; DELETE FROM session_state;")?;

        for n in &export.nodes {
            conn.execute(
                "INSERT INTO nodes (id, topic, kind, domain, description, difficulty, stability,
                     last_reviewed, last_encountered, review_count, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![n.id, n.topic, n.kind, n.domain, n.description,
                    n.difficulty, n.stability, n.last_reviewed, n.last_encountered,
                    n.review_count, n.created_at],
            )?;
            for ctx in &n.contexts {
                conn.execute(
                    "INSERT OR IGNORE INTO contexts (node_id, context, added_at) VALUES (?,?,?)",
                    params![n.id, ctx, n.last_reviewed],
                )?;
            }
        }

        for (key, value) in &export.session_state {
            conn.execute(
                "INSERT INTO session_state (key, value, updated_at) VALUES (?1,?2,?3)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
                params![key, value, Local::now().naive_local().to_string()],
            )?;
        }

        Ok(export.nodes.len())
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

// ── JSON serialisation structs ────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct PkgExport {
    pub version: u32,
    pub exported_at: String,
    pub session_state: std::collections::HashMap<String, String>,
    pub nodes: Vec<NodeExport>,
}

#[derive(Serialize, Deserialize)]
pub struct NodeExport {
    pub id: String,
    pub topic: String,
    pub kind: String,
    pub domain: String,
    pub description: String,
    pub difficulty: f64,
    pub stability: f64,
    pub last_reviewed: String,
    pub last_encountered: String,
    pub review_count: i64,
    pub contexts: Vec<String>,
    pub created_at: String,
}

impl NodeExport {
    fn from_node(n: &Node) -> Self {
        Self {
            id: n.id.clone(),
            topic: n.topic.clone(),
            kind: n.kind.as_str().to_string(),
            domain: n.domain.clone(),
            description: n.description.clone(),
            difficulty: n.difficulty,
            stability: n.stability,
            last_reviewed: n.last_reviewed.to_string(),
            last_encountered: n.last_encountered.to_string(),
            review_count: n.review_count,
            contexts: n.contexts.clone(),
            created_at: n.created_at.to_string(),
        }
    }
}
