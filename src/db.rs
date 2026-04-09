/// SQLite-backed PKG storage.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::fsrs;
use crate::node::{Kind, Node};
use crate::obsidian;

// ── Edge types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    Implies,
    DependsOn,
    ConflictsWith,
    PartOf,
}

impl EdgeKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "implies"        => Self::Implies,
            "depends_on"     => Self::DependsOn,
            "conflicts_with" => Self::ConflictsWith,
            "part_of"        => Self::PartOf,
            _                => Self::Implies,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Implies        => "implies",
            Self::DependsOn      => "depends_on",
            Self::ConflictsWith  => "conflicts_with",
            Self::PartOf         => "part_of",
        }
    }
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Edge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub kind: EdgeKind,
    pub description: String,
    pub strength: f64,
    pub created_at: String,
    pub last_fired: Option<String>,
    pub last_fired_session: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Review {
    #[allow(dead_code)]
    pub node_id: String,
    pub reviewed_at: String,
    pub question: String,
    pub answer: String,
    pub feedback: String,
    pub score: f64,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS session_state (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS nodes (
    id                   TEXT PRIMARY KEY,
    topic                TEXT NOT NULL,
    kind                 TEXT NOT NULL DEFAULT 'concept',
    domain               TEXT NOT NULL DEFAULT '',
    description          TEXT NOT NULL DEFAULT '',
    difficulty           REAL NOT NULL DEFAULT 0.3,
    stability            REAL NOT NULL DEFAULT 2.0,
    last_reviewed        TEXT NOT NULL,
    last_encountered     TEXT NOT NULL,
    review_count         INTEGER NOT NULL DEFAULT 0,
    created_at           TEXT NOT NULL,
    canonical_question   TEXT NOT NULL DEFAULT '',
    canonical_answer     TEXT NOT NULL DEFAULT '',
    canonical_clue       TEXT NOT NULL DEFAULT '',
    repo                 TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS contexts (
    node_id  TEXT NOT NULL,
    context  TEXT NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (node_id, context),
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS edges (
    id          TEXT PRIMARY KEY,
    source_id   TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    target_id   TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    description TEXT NOT NULL,
    strength    REAL NOT NULL DEFAULT 0.5,
    created_at  TEXT NOT NULL,
    last_fired  TEXT
);

CREATE INDEX IF NOT EXISTS edges_source ON edges(source_id);
CREATE INDEX IF NOT EXISTS edges_target ON edges(target_id);

CREATE TABLE IF NOT EXISTS reviews (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    reviewed_at TEXT NOT NULL,
    question    TEXT NOT NULL DEFAULT '',
    answer      TEXT NOT NULL DEFAULT '',
    feedback    TEXT NOT NULL DEFAULT '',
    score       REAL NOT NULL DEFAULT 0.0
);
CREATE INDEX IF NOT EXISTS reviews_node ON reviews(node_id);
";

pub struct Db {
    path: PathBuf,
    pub pkg_dir: PathBuf,
}

impl Db {
    pub fn open(path: &Path, pkg_dir: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Self { path: path.to_path_buf(), pkg_dir: pkg_dir.to_path_buf() };
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
        // Migrations for existing databases (all idempotent — ignored if column already exists)
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN domain TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN canonical_question TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN canonical_answer TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN canonical_clue TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE nodes ADD COLUMN repo TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE edges ADD COLUMN last_fired_session INTEGER", []);
        Ok(())
    }

    fn node_id(topic: &str) -> String {
        topic.to_lowercase().trim().replace(' ', "-")
    }

    pub fn node_id_static(topic: &str) -> String {
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
            canonical_question: row.get::<_, String>("canonical_question").unwrap_or_default(),
            canonical_answer: row.get::<_, String>("canonical_answer").unwrap_or_default(),
            canonical_clue: row.get::<_, String>("canonical_clue").unwrap_or_default(),
            repo: row.get::<_, String>("repo").unwrap_or_default(),
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
        repo: &str,
        created_at_override: Option<NaiveDate>,
    ) -> Result<()> {
        let node_id = Self::node_id(topic);
        let reviewed = last_reviewed_override
            .unwrap_or_else(|| Local::now().date_naive())
            .to_string();
        let today = Self::today();
        let created_at = created_at_override
            .map(|d| d.to_string())
            .unwrap_or_else(|| today.clone());

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
            let effective_domain = if !domain.is_empty() && node.domain.is_empty() {
                domain
            } else if !node.domain.is_empty() {
                &node.domain
            } else {
                domain
            };
            // repo only backfills if not already set
            let effective_repo = if !repo.is_empty() && node.repo.is_empty() { repo } else { &node.repo };
            conn.execute(
                "UPDATE nodes SET
                    stability        = ?1,
                    difficulty       = ?2,
                    last_reviewed    = ?3,
                    last_encountered = ?4,
                    review_count     = review_count + 1,
                    kind             = ?5,
                    domain           = ?6,
                    description      = CASE WHEN description = '' THEN ?7 ELSE description END,
                    repo             = ?8
                 WHERE id = ?9",
                params![new_s, new_d, reviewed, today, kind.as_str(), effective_domain, description, effective_repo, node_id],
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
                     last_reviewed, last_encountered, review_count, created_at, repo)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0.3, ?6, ?7, ?8, 1, ?9, ?10)",
                params![node_id, topic, kind.as_str(), domain, description, stability, reviewed, today, created_at, repo],
            )?;
        }

        if !context.is_empty() {
            conn.execute(
                "INSERT OR IGNORE INTO contexts (node_id, context, added_at) VALUES (?, ?, ?)",
                params![node_id, context, today],
            )?;
        }

        // Write Obsidian PKG file (best-effort — never block on failure)
        if let Ok(Some(node)) = self.get_node(topic) {
            obsidian::write_node(&node, &self.pkg_dir).ok();
        }

        Ok(())
    }

    pub fn set_canonical_qa(&self, topic: &str, question: &str, answer: &str, clue: &str) -> Result<()> {
        let node_id = Self::node_id(topic);
        let conn = self.connect()?;
        conn.execute(
            "UPDATE nodes SET canonical_question = ?1, canonical_answer = ?2, canonical_clue = ?3 WHERE id = ?4",
            params![question, answer, clue, node_id],
        )?;
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

    pub fn get_nodes_by_date(&self, since: Option<&str>, before: Option<&str>) -> Result<Vec<Node>> {
        let conn = self.connect()?;
        // Use sentinel values so we can always bind two params
        let since_val  = since.unwrap_or("0000-00-00");
        let before_val = before.unwrap_or("9999-99-99");
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE kind != 'domain'
             AND created_at >= ?1 AND created_at <= ?2
             ORDER BY created_at DESC, topic",
        )?;
        let nodes: Vec<Node> = stmt
            .query_map(params![since_val, before_val], |row| Self::row_to_node(&conn, row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(nodes)
    }

    pub fn search_nodes(&self, query: &str) -> Result<Vec<Node>> {
        let conn = self.connect()?;
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE kind != 'domain' AND (LOWER(topic) LIKE ? OR LOWER(description) LIKE ?) ORDER BY topic",
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

    // ── edges ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT OR IGNORE INTO edges
             (id, source_id, target_id, kind, description, strength, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                edge.id, edge.source_id, edge.target_id, edge.kind.as_str(),
                edge.description, edge.strength,
                Local::now().naive_local().to_string()
            ],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_edges_for_node(&self, node_id: &str) -> Result<Vec<Edge>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, source_id, target_id, kind, description, strength, created_at, last_fired, last_fired_session
             FROM edges WHERE source_id = ?1 OR target_id = ?1
             ORDER BY strength DESC",
        )?;
        let edges = stmt
            .query_map(params![node_id], |row| {
                Ok(Edge {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    target_id: row.get(2)?,
                    kind: EdgeKind::from_str(&row.get::<_, String>(3)?),
                    description: row.get(4)?,
                    strength: row.get(5)?,
                    created_at: row.get(6)?,
                    last_fired: row.get(7)?,
                    last_fired_session: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(edges)
    }

    pub fn get_all_edges(&self) -> Result<Vec<Edge>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, source_id, target_id, kind, description, strength, created_at, last_fired, last_fired_session
             FROM edges ORDER BY strength DESC",
        )?;
        let edges = stmt
            .query_map([], |row| {
                Ok(Edge {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    target_id: row.get(2)?,
                    kind: EdgeKind::from_str(&row.get::<_, String>(3)?),
                    description: row.get(4)?,
                    strength: row.get(5)?,
                    created_at: row.get(6)?,
                    last_fired: row.get(7)?,
                    last_fired_session: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(edges)
    }

    #[allow(dead_code)]
    pub fn fire_edge(&self, edge_id: &str) -> Result<()> {
        let session_count = self.total_quizzes()?;
        let conn = self.connect()?;
        conn.execute(
            "UPDATE edges SET last_fired = ?1, last_fired_session = ?2 WHERE id = ?3",
            params![Local::now().naive_local().to_string(), session_count, edge_id],
        )?;
        Ok(())
    }

    pub fn total_quizzes(&self) -> Result<i64> {
        let v = self.session_get("total_quizzes")?;
        Ok(v.unwrap_or_default().parse().unwrap_or(0))
    }

    pub fn increment_total_quizzes(&self) -> Result<()> {
        let count = self.total_quizzes()?;
        self.session_set("total_quizzes", &(count + 1).to_string())
    }

    #[allow(dead_code)]
    pub fn edge_exists(&self, source_id: &str, target_id: &str, kind: &EdgeKind) -> Result<bool> {
        let conn = self.connect()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM edges
             WHERE ((source_id = ?1 AND target_id = ?2) OR (source_id = ?2 AND target_id = ?1))
             AND kind = ?3",
            params![source_id, target_id, kind.as_str()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Returns (total, most_connected_topic, most_connected_count, avg_strength).
    pub fn edge_stats(&self) -> Result<(usize, String, usize, f64)> {
        let edges = self.get_all_edges()?;
        let total = edges.len();
        if total == 0 {
            return Ok((0, String::new(), 0, 0.0));
        }
        let avg_strength = edges.iter().map(|e| e.strength).sum::<f64>() / total as f64;
        // Find node with most connections
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for e in &edges {
            *counts.entry(&e.source_id).or_insert(0) += 1;
            *counts.entry(&e.target_id).or_insert(0) += 1;
        }
        let (most_connected_id, most_count) = counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(id, c)| (id, c))
            .unwrap_or(("", 0));
        let topic = if let Ok(Some(node)) = self.get_node(most_connected_id) {
            node.topic
        } else {
            most_connected_id.to_string()
        };
        Ok((total, topic, most_count, avg_strength))
    }

    /// Set the domain on an undomained node (used by classify command).
    pub fn set_domain(&self, node_id: &str, domain: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("UPDATE nodes SET domain = ? WHERE id = ?", params![domain, node_id])?;
        Ok(())
    }

    /// Returns nodes that have no domain assigned yet.
    /// Nodes that have a canonical question but no clue yet — targets for retroactive clue generation.
    pub fn nodes_missing_clue(&self) -> Result<Vec<Node>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT * FROM nodes WHERE canonical_question != '' AND canonical_clue = '' ORDER BY topic"
        )?;
        let nodes = stmt
            .query_map([], |row| Self::row_to_node(&conn, row))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(nodes)
    }

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
        // Exclude taxonomy skeleton nodes from all user-facing counts
        let nodes: Vec<_> = nodes.into_iter().filter(|n| !n.kind.is_domain()).collect();
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

    /// Seed the 13 taxonomy domain nodes if they don't already exist.
    /// These act as the structural skeleton — every topic gets a free `part_of` edge to its domain.
    pub fn ensure_taxonomy_skeleton(&self) -> Result<()> {
        const DOMAINS: &[(&str, &str)] = &[
            ("Language",     "Programming languages, syntax, type systems, and language-specific patterns and idioms."),
            ("Database",     "Databases, query languages, schema design, indexing, transactions, and data persistence."),
            ("Auth",         "Authentication, authorisation, identity management, tokens, sessions, and access control."),
            ("API",          "API design, REST, GraphQL, WebSockets, RPC, and service interfaces."),
            ("Frontend",     "UI frameworks, DOM, CSS, browser APIs, rendering, and client-side patterns."),
            ("DevOps",       "CI/CD pipelines, containers, infrastructure, deployment strategies, and operations."),
            ("Architecture", "System design, scalability patterns, distributed systems, and architectural trade-offs."),
            ("Performance",  "Optimisation, profiling, caching strategies, latency, and throughput."),
            ("Security",     "Vulnerabilities, encryption, input validation, threat modelling, and secure coding practices."),
            ("Testing",      "Unit tests, integration tests, mocking, test strategy, and quality assurance."),
            ("Tooling",      "Build systems, package managers, editors, linters, and developer productivity tools."),
            ("Data",         "Algorithms, data structures, machine learning, and data processing pipelines."),
            ("Other",        "Topics that span multiple domains or don't fit a single category."),
        ];

        for (domain, description) in DOMAINS {
            let node_id = Self::node_id(domain);
            let conn = self.connect()?;
            let exists: bool = conn
                .query_row("SELECT 1 FROM nodes WHERE id = ?", params![node_id], |_| Ok(()))
                .is_ok();
            if !exists {
                let today = Self::today();
                conn.execute(
                    "INSERT INTO nodes
                        (id, topic, kind, domain, description, difficulty, stability,
                         last_reviewed, last_encountered, review_count, created_at)
                     VALUES (?1, ?2, 'domain', ?3, ?4, 0.0, 999.0, ?5, ?6, 0, ?7)",
                    params![node_id, domain, domain, description, today, today, today],
                )?;
            }
        }
        Ok(())
    }

    pub fn add_review(
        &self,
        node_id: &str,
        question: &str,
        answer: &str,
        feedback: &str,
        score: f64,
    ) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO reviews (node_id, reviewed_at, question, answer, feedback, score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![node_id, Self::today(), question, answer, feedback, score],
        )?;
        Ok(())
    }

    pub fn get_reviews(&self, node_id: &str) -> Result<Vec<Review>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT node_id, reviewed_at, question, answer, feedback, score
             FROM reviews WHERE node_id = ?1 ORDER BY id ASC",
        )?;
        let reviews = stmt
            .query_map(params![node_id], |row| {
                Ok(Review {
                    node_id:     row.get(0)?,
                    reviewed_at: row.get(1)?,
                    question:    row.get(2)?,
                    answer:      row.get(3)?,
                    feedback:    row.get(4)?,
                    score:       row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(reviews)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn open_temp_db() -> (Db, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db = Db::open(&dir.path().join("test.db"), dir.path()).unwrap();
        (db, dir)
    }

    #[test]
    fn add_and_retrieve_node() {
        let (db, _dir) = open_temp_db();
        db.add_or_update("JWT authentication", 0.8, &Kind::Pattern, "Auth",
            "Stateless token auth", "test task", None, "", None).unwrap();

        let node = db.get_node("JWT authentication").unwrap().unwrap();
        assert_eq!(node.topic, "JWT authentication");
        assert_eq!(node.kind, Kind::Pattern);
        assert_eq!(node.domain, "Auth");
        assert_eq!(node.review_count, 1);
    }

    #[test]
    fn node_id_is_slugified() {
        let (db, _dir) = open_temp_db();
        db.add_or_update("Redis TTL expiry", 0.8, &Kind::Implementation, "Database",
            "desc", "ctx", None, "", None).unwrap();

        // Lookup by original topic name should find it
        let node = db.get_node("Redis TTL expiry").unwrap().unwrap();
        assert_eq!(node.id, "redis-ttl-expiry");
    }

    #[test]
    fn repeated_review_increments_count() {
        let (db, _dir) = open_temp_db();
        for _ in 0..3 {
            db.add_or_update("SQL indexes", 0.9, &Kind::Concept, "Database",
                "desc", "ctx", None, "", None).unwrap();
        }
        let node = db.get_node("SQL indexes").unwrap().unwrap();
        assert_eq!(node.review_count, 3);
    }

    #[test]
    fn search_finds_substring_match() {
        let (db, _dir) = open_temp_db();
        db.add_or_update("Redis TTL expiry", 0.8, &Kind::Concept, "Database",
            "desc", "ctx", None, "", None).unwrap();
        db.add_or_update("Redis pub/sub", 0.8, &Kind::Concept, "Database",
            "desc", "ctx", None, "", None).unwrap();
        db.add_or_update("JWT authentication", 0.8, &Kind::Pattern, "Auth",
            "desc", "ctx", None, "", None).unwrap();

        let results = db.search_nodes("redis").unwrap();
        assert_eq!(results.len(), 2);

        let results = db.search_nodes("jwt").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn delete_removes_node() {
        let (db, _dir) = open_temp_db();
        db.add_or_update("JWT authentication", 0.8, &Kind::Pattern, "Auth",
            "desc", "ctx", None, "", None).unwrap();

        db.delete_node("jwt-authentication").unwrap();
        assert!(db.get_node("JWT authentication").unwrap().is_none());
    }

    #[test]
    fn summary_counts_correctly() {
        let (db, _dir) = open_temp_db();
        // Add one topic reviewed today — will be "known"
        db.add_or_update("fresh topic", 0.9, &Kind::Concept, "Other",
            "desc", "ctx", None, "", None).unwrap();

        let (total, known, _stale, _gaps) = db.summary().unwrap();
        assert_eq!(total, 1);
        assert_eq!(known, 1);
    }

    #[test]
    fn session_get_set_round_trips() {
        let (db, _dir) = open_temp_db();
        assert!(db.session_get("missing_key").unwrap().is_none());

        db.session_set("quiz_streak", "5").unwrap();
        assert_eq!(db.session_get("quiz_streak").unwrap().as_deref(), Some("5"));

        db.session_set("quiz_streak", "6").unwrap();
        assert_eq!(db.session_get("quiz_streak").unwrap().as_deref(), Some("6"));
    }

    #[test]
    fn pkg_json_export_import_round_trip() {
        let (db, dir) = open_temp_db();
        db.add_or_update("JWT authentication", 0.8, &Kind::Pattern, "Auth",
            "Token auth", "test task", None, "", None).unwrap();
        db.add_or_update("SQL indexes", 0.7, &Kind::Concept, "Database",
            "Index desc", "other task", None, "", None).unwrap();

        let json_path = dir.path().join("pkg.json");
        db.export_pkg_json(&json_path).unwrap();
        assert!(json_path.exists());

        // Import into a fresh DB
        let (db2, _dir2) = open_temp_db();
        let count = db2.import_pkg_json(&json_path).unwrap();
        assert_eq!(count, 2);

        let node = db2.get_node("JWT authentication").unwrap().unwrap();
        assert_eq!(node.domain, "Auth");
    }

    #[test]
    fn set_domain_updates_existing_node() {
        let (db, _dir) = open_temp_db();
        db.add_or_update("some topic", 0.8, &Kind::Concept, "",
            "desc", "ctx", None, "", None).unwrap();

        db.set_domain("some-topic", "Language").unwrap();
        let node = db.get_node("some topic").unwrap().unwrap();
        assert_eq!(node.domain, "Language");
    }
}
