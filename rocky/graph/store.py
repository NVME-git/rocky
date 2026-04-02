"""
PKG — Personal Knowledge Graph.
SQLite-backed storage using the FSRS-inspired DSR model.
"""

import json
import sqlite3
from datetime import date
from pathlib import Path
from typing import Optional

from rocky.graph.node import Node
from rocky.graph import fsrs

ROCKY_DIR = Path.home() / ".rocky"
DB_FILE = ROCKY_DIR / "graph.db"
VAULT_DIR = ROCKY_DIR / "vault"
LEGACY_JSON = ROCKY_DIR / "knowledge_graph.json"

_SCHEMA = """
CREATE TABLE IF NOT EXISTS nodes (
    id              TEXT PRIMARY KEY,
    topic           TEXT NOT NULL,
    kind            TEXT NOT NULL DEFAULT 'concept',
    description     TEXT NOT NULL DEFAULT '',
    difficulty      REAL NOT NULL DEFAULT 0.3,
    stability       REAL NOT NULL DEFAULT 2.0,
    last_reviewed   TEXT NOT NULL,
    last_encountered TEXT NOT NULL,
    review_count    INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS contexts (
    node_id  TEXT NOT NULL,
    context  TEXT NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (node_id, context),
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
);
"""


class PKG:
    def __init__(self, path: Path = DB_FILE, vault_dir: Path = VAULT_DIR):
        self.path = path
        self.vault_dir = vault_dir
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._init_db()
        self._migrate_from_json()

    # ── internal ──────────────────────────────────────────────────────────────

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.path)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA foreign_keys = ON")
        return conn

    def _init_db(self):
        with self._connect() as conn:
            conn.executescript(_SCHEMA)

    def _migrate_from_json(self):
        """One-time migration from legacy knowledge_graph.json if DB is empty."""
        if not LEGACY_JSON.exists():
            return
        with self._connect() as conn:
            if conn.execute("SELECT COUNT(*) FROM nodes").fetchone()[0] > 0:
                return

        with open(LEGACY_JSON) as f:
            data = json.load(f)

        today = date.today().isoformat()
        with self._connect() as conn:
            for nd in data.get("nodes", {}).values():
                # Map old confidence → stability (rough: 100% conf ≈ 120 days)
                stability = max(1.0, nd.get("confidence", 0.3) * 120)
                conn.execute("""
                    INSERT OR IGNORE INTO nodes
                        (id, topic, kind, description, difficulty, stability,
                         last_reviewed, last_encountered, review_count, created_at)
                    VALUES (?, ?, ?, ?, 0.3, ?, ?, ?, ?, ?)
                """, (
                    nd["id"], nd["topic"], nd.get("kind", "concept"),
                    nd.get("description", ""), stability,
                    nd.get("last_reviewed", today), nd.get("last_encountered", today),
                    nd.get("review_count", 1), today,
                ))
                for ctx in nd.get("contexts", []):
                    conn.execute(
                        "INSERT OR IGNORE INTO contexts (node_id, context, added_at) VALUES (?,?,?)",
                        (nd["id"], ctx, today),
                    )

    def _node_id(self, topic: str) -> str:
        return topic.lower().strip().replace(" ", "-")

    def _row_to_node(self, row: sqlite3.Row, conn: sqlite3.Connection) -> Node:
        contexts = [r[0] for r in conn.execute(
            "SELECT context FROM contexts WHERE node_id = ? ORDER BY added_at",
            (row["id"],),
        ).fetchall()]
        return Node(
            id=row["id"],
            topic=row["topic"],
            kind=row["kind"],
            description=row["description"],
            difficulty=row["difficulty"],
            stability=row["stability"],
            last_reviewed=date.fromisoformat(row["last_reviewed"]),
            last_encountered=date.fromisoformat(row["last_encountered"]),
            review_count=row["review_count"],
            contexts=contexts,
            created_at=date.fromisoformat(row["created_at"]),
        )

    def _write_vault(self, node: Node):
        """Write an Obsidian-compatible markdown file for this node."""
        from rocky.export import obsidian
        obsidian.write_node(node, self.vault_dir)

    # ── public API ────────────────────────────────────────────────────────────

    def get(self, topic: str) -> Optional[Node]:
        node_id = self._node_id(topic)
        with self._connect() as conn:
            row = conn.execute("SELECT * FROM nodes WHERE id = ?", (node_id,)).fetchone()
            if row is None:
                return None
            return self._row_to_node(row, conn)

    def retrievability(self, topic: str) -> float:
        node = self.get(topic)
        if node is None:
            return 0.0
        return fsrs.retrievability(node.stability, node.last_reviewed)

    def classify(self, topic: str) -> str:
        node = self.get(topic)
        if node is None:
            return "new"
        r = fsrs.retrievability(node.stability, node.last_reviewed)
        return fsrs.classify(r)

    def add_or_update(
        self,
        topic: str,
        score: float,
        kind: str = "concept",
        description: str = "",
        context: str = "",
        last_reviewed: Optional[date] = None,
    ):
        """
        Add a new node or update an existing one after a review.
        score: 0.0 (skipped/failed) → 1.0 (perfect understanding)
        last_reviewed: override for seeding/testing (defaults to today)
        """
        node_id = self._node_id(topic)
        reviewed = (last_reviewed or date.today()).isoformat()
        today = date.today().isoformat()
        existing = self.get(topic)

        with self._connect() as conn:
            if existing is None:
                stability = fsrs.initial_stability(kind)
                if score >= 0.65:
                    pass
                elif score >= 0.4:
                    stability *= 0.6
                else:
                    stability *= 0.3

                conn.execute("""
                    INSERT INTO nodes
                        (id, topic, kind, description, difficulty, stability,
                         last_reviewed, last_encountered, review_count, created_at)
                    VALUES (?, ?, ?, ?, 0.3, ?, ?, ?, 1, ?)
                """, (node_id, topic, kind, description, stability, reviewed, today, today))
            else:
                r = fsrs.retrievability(existing.stability, existing.last_reviewed)
                new_stability, new_difficulty = fsrs.update_after_review(
                    existing.stability, existing.difficulty, r, score
                )
                conn.execute("""
                    UPDATE nodes SET
                        stability        = ?,
                        difficulty       = ?,
                        last_reviewed    = ?,
                        last_encountered = ?,
                        review_count     = review_count + 1,
                        kind             = ?,
                        description      = CASE WHEN description = '' THEN ? ELSE description END
                    WHERE id = ?
                """, (new_stability, new_difficulty, reviewed, today, kind, description, node_id))

            if context:
                conn.execute(
                    "INSERT OR IGNORE INTO contexts (node_id, context, added_at) VALUES (?,?,?)",
                    (node_id, context, today),
                )

        node = self.get(topic)
        if node:
            self._write_vault(node)

    def mark_encountered(self, topic: str):
        """Record that a known topic was encountered without a quiz."""
        node_id = self._node_id(topic)
        with self._connect() as conn:
            conn.execute(
                "UPDATE nodes SET last_encountered = ? WHERE id = ?",
                (date.today().isoformat(), node_id),
            )

    def all_topics(self) -> list[Node]:
        with self._connect() as conn:
            rows = conn.execute("SELECT * FROM nodes ORDER BY topic").fetchall()
            return [self._row_to_node(row, conn) for row in rows]

    def summary(self) -> dict:
        nodes = self.all_topics()
        total = len(nodes)
        known = sum(1 for n in nodes if fsrs.classify(fsrs.retrievability(n.stability, n.last_reviewed)) == "known")
        stale = sum(1 for n in nodes if fsrs.classify(fsrs.retrievability(n.stability, n.last_reviewed)) == "stale")
        return {"total": total, "known": known, "stale": stale, "gaps": total - known - stale}
