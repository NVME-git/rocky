#!/usr/bin/env python3
"""
Seed the demo PKG with pre-existing topics in various states.

Run this before run.py to start from an interesting, realistic baseline
instead of an empty PKG:

    python demo/seed.py

Topics are seeded with backdated last_reviewed dates so the DSR model
immediately classifies them into all three states:
  known  — reviewed 5 days ago, high stability
  stale  — not touched in 45 days (implementation decays fast)
  stale  — not touched in 120 days (concept, but low stability)
"""

import sys
from datetime import date, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from rocky.graph.store import PKG
from rocky.graph import fsrs

DEMO_DB = Path(__file__).parent / "demo_graph.db"

today = date.today()

SEED_TOPICS = [
    {
        "topic": "REST API design",
        "kind": "pattern",
        "description": "Stateless HTTP API design using resources, verbs, and status codes.",
        "stability": 28.0,   # strong — recently reviewed, many passes
        "difficulty": 0.15,
        "last_reviewed": today - timedelta(days=5),
        "contexts": ["build a user management API", "add CRUD endpoints for products"],
    },
    {
        "topic": "SQL query optimization",
        "kind": "implementation",
        "description": "Techniques to improve query performance: indexes, explain plans, avoiding N+1.",
        "stability": 15.0,   # R = (1 + 45/(9*15))^-1 ≈ 0.75 → stale
        "difficulty": 0.35,
        "last_reviewed": today - timedelta(days=45),
        "contexts": ["optimize slow product listing queries"],
    },
    {
        "topic": "HTTP caching",
        "kind": "concept",
        "description": "Browser and proxy caching via Cache-Control headers, ETags, and invalidation.",
        "stability": 40.0,   # R = (1 + 120/(9*40))^-1 ≈ 0.75 → stale
        "difficulty": 0.45,
        "last_reviewed": today - timedelta(days=120),
        "contexts": ["add caching to API responses"],
    },
]

# Wipe and re-create the demo DB
if DEMO_DB.exists():
    DEMO_DB.unlink()

pkg = PKG(path=DEMO_DB, vault_dir=Path(__file__).parent / "demo_vault")

for t in SEED_TOPICS:
    node_id = t["topic"].lower().strip().replace(" ", "-")
    today_str = today.isoformat()
    reviewed_str = t["last_reviewed"].isoformat()

    import sqlite3
    with sqlite3.connect(DEMO_DB) as conn:
        conn.execute("""
            INSERT OR REPLACE INTO nodes
                (id, topic, kind, description, difficulty, stability,
                 last_reviewed, last_encountered, review_count, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            node_id, t["topic"], t["kind"], t["description"],
            t["difficulty"], t["stability"],
            reviewed_str, reviewed_str, 3, reviewed_str,
        ))
        for ctx in t["contexts"]:
            conn.execute(
                "INSERT OR IGNORE INTO contexts (node_id, context, added_at) VALUES (?,?,?)",
                (node_id, ctx, today_str),
            )

print(f"Seeded {len(SEED_TOPICS)} topics → {DEMO_DB.name}")
print()
print(f"  {'Topic':<35} {'Kind':<16} {'Recall':<10} Status")
print("  " + "─" * 72)
for t in SEED_TOPICS:
    r = fsrs.retrievability(t["stability"], t["last_reviewed"])
    classification = fsrs.classify(r)
    days_ago = (today - t["last_reviewed"]).days
    print(f"  {t['topic']:<35} {t['kind']:<16} {r:<10.0%} {classification} ({days_ago}d ago)")
