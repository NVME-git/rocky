#!/usr/bin/env python3
"""Generate demo graph HTML files from the demo SQLite database.

Reads docs/demo/graph.db, builds the same JSON structure that `rocky view`
produces, and injects it into the src/view.html template for each walkthrough
stage.  The output goes to docs/web/graphs/stage{1..7}.html.

Each stage shows a progressive slice of the knowledge graph matching the
Demo Usecase walkthrough in the docs.
"""

import json
import math
import sqlite3
import os
import sys
from datetime import date, datetime

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEMO_DB = os.path.join(REPO, "docs", "demo", "graph.db")
TEMPLATE = os.path.join(REPO, "src", "view.html")
OUT_DIR = os.path.join(REPO, "docs", "web", "graphs")
USER_NAME = "alex"

# Project summaries loaded into the view (mirrors ~/.rocky/summaries/*.txt)
SUMMARIES = {
    "taskify": "Rust REST API — task management backend with JWT auth, PostgreSQL, Redis caching, and Docker deployment",
    "home-bank": "Python data pipeline — personal finance tracker that imports bank CSV exports, categorises transactions with a rule-based engine, and models accounts with double-entry bookkeeping",
}

# ── Stage definitions ─────────────────────────────────────────────────────────
# Each stage lists which topic IDs are visible at that point in the walkthrough.

STAGE_TOPICS = {
    1: [
        "rust-async/await", "tokio-runtime", "axum-framework",
    ],
    2: [
        "rust-async/await", "tokio-runtime", "axum-framework",
        "sqlx-connection-pooling", "database-migrations",
    ],
    3: [
        "rust-async/await", "tokio-runtime", "axum-framework",
        "sqlx-connection-pooling", "database-migrations",
        "jwt-authentication", "httponly-cookie-security", "token-expiry-handling",
    ],
    4: [
        "rust-async/await", "tokio-runtime", "axum-framework",
        "sqlx-connection-pooling", "database-migrations",
        "jwt-authentication", "httponly-cookie-security", "token-expiry-handling",
        "redis-ttl-expiry", "cache-invalidation",
    ],
    5: [
        "rust-async/await", "tokio-runtime", "axum-framework",
        "sqlx-connection-pooling", "database-migrations",
        "jwt-authentication", "httponly-cookie-security", "token-expiry-handling",
        "redis-ttl-expiry", "cache-invalidation",
        "redis-sorted-sets", "lua-scripting-in-redis",
    ],
    # Stage 6 = same topics as 5 but a week later (decay applied)
    6: [
        "rust-async/await", "tokio-runtime", "axum-framework",
        "sqlx-connection-pooling", "database-migrations",
        "jwt-authentication", "httponly-cookie-security", "token-expiry-handling",
        "redis-ttl-expiry", "cache-invalidation",
        "redis-sorted-sets", "lua-scripting-in-redis",
    ],
    7: [
        "rust-async/await", "tokio-runtime", "axum-framework",
        "sqlx-connection-pooling", "database-migrations",
        "jwt-authentication", "httponly-cookie-security", "token-expiry-handling",
        "redis-ttl-expiry", "cache-invalidation",
        "redis-sorted-sets", "lua-scripting-in-redis",
        "docker-multi-stage-builds", "container-image-optimization",
        "github-actions-workflow-syntax", "ci/cd-pipeline-design",
        "openapi-specification",
        # home-bank project — second repo in the PKG, shows multi-repo filter
        "csv-parsing", "transaction-categorisation",
        "pandas-dataframe-operations", "double-entry-bookkeeping",
    ],
}

# The 13 taxonomy domains Rocky uses
DOMAINS = [
    "Language", "Database", "Auth", "API", "Frontend",
    "DevOps", "Architecture", "Performance", "Security",
    "Testing", "Tooling", "Data", "Other",
]


def retrievability(stability: float, last_reviewed: str) -> float:
    """FSRS-style retrievability: R = (1 + t/9s)^-1, same as src/fsrs.rs."""
    if stability <= 0 or stability >= 999:
        return 1.0
    try:
        last = datetime.strptime(last_reviewed, "%Y-%m-%d").date()
    except (ValueError, TypeError):
        return 1.0
    today = date(2026, 4, 8)  # Fixed demo date
    days = max(0, (today - last).days)
    return (1 + days / (9 * stability)) ** -1


def classify(r: float) -> str:
    if r >= 0.9:
        return "known"
    elif r >= 0.7:
        return "stale"
    else:
        return "gap"


def load_demo_data():
    """Load all nodes, edges, and reviews from the demo database."""
    conn = sqlite3.connect(DEMO_DB)
    conn.row_factory = sqlite3.Row

    nodes = {}
    for row in conn.execute("SELECT * FROM nodes ORDER BY created_at"):
        node_id = row["id"]
        # Load contexts
        contexts = [r[0] for r in conn.execute(
            "SELECT context FROM contexts WHERE node_id = ? ORDER BY added_at",
            (node_id,)
        )]
        # Load reviews
        reviews = []
        for rev in conn.execute(
            "SELECT * FROM reviews WHERE node_id = ? ORDER BY reviewed_at",
            (node_id,)
        ):
            reviews.append({
                "date": rev["reviewed_at"],
                "question": rev["question"],
                "answer": rev["answer"],
                "feedback": rev["feedback"],
                "score": rev["score"],
            })

        nodes[node_id] = {
            "id": node_id,
            "topic": row["topic"],
            "kind": row["kind"],
            "domain": row["domain"],
            "description": row["description"],
            "stability": row["stability"],
            "difficulty": row["difficulty"],
            "last_reviewed": row["last_reviewed"],
            "review_count": row["review_count"],
            "created_at": row["created_at"],
            "canonical_question": row["canonical_question"],
            "canonical_answer": row["canonical_answer"],
            "canonical_clue": row["canonical_clue"] if "canonical_clue" in row.keys() else "",
            "repo": row["repo"],
            "contexts": contexts,
            "reviews": reviews,
        }

    edges = []
    for row in conn.execute("SELECT * FROM edges ORDER BY created_at"):
        edges.append({
            "id": row["id"],
            "source": row["source_id"],
            "target": row["target_id"],
            "kind": row["kind"],
            "description": row["description"],
            "strength": row["strength"],
        })

    conn.close()
    return nodes, edges


def build_stage_json(all_nodes, all_edges, topic_ids):
    """Build the PKG JSON for a specific stage."""
    visible_ids = set(topic_ids)
    nodes_json = []

    # Add topic nodes
    for tid in topic_ids:
        if tid not in all_nodes:
            print(f"  WARNING: topic '{tid}' not found in database", file=sys.stderr)
            continue
        n = all_nodes[tid]
        r = retrievability(n["stability"], n["last_reviewed"])
        nodes_json.append({
            "id": n["id"],
            "topic": n["topic"],
            "kind": n["kind"],
            "domain": n["domain"],
            "description": n["description"],
            "stability": n["stability"],
            "difficulty": n["difficulty"],
            "retrievability": round(r, 3),
            "classification": classify(r),
            "last_reviewed": n["last_reviewed"],
            "review_count": n["review_count"],
            "created_at": n["created_at"],
            "repo": n["repo"],
            "canonical_question": n["canonical_question"],
            "canonical_answer": n["canonical_answer"],
            "canonical_clue": n.get("canonical_clue", ""),
            "reviews": n["reviews"],
        })

    # Determine which domains are active (have at least one visible topic)
    active_domains = set()
    for tid in topic_ids:
        if tid in all_nodes:
            domain = all_nodes[tid]["domain"]
            if domain:
                active_domains.add(domain)

    # Add domain nodes (only for active domains)
    for domain in DOMAINS:
        if domain not in active_domains:
            continue
        did = domain.lower()
        visible_ids.add(did)
        nodes_json.append({
            "id": did,
            "topic": domain,
            "kind": "domain",
            "domain": domain,
            "description": f"{domain} domain",
            "stability": 999,
            "difficulty": 0,
            "retrievability": 1,
            "classification": "known",
            "last_reviewed": "",
            "review_count": 0,
            "created_at": "2026-03-01",
            "reviews": [],
        })

    # Add user node
    user_id = "__user__"
    visible_ids.add(user_id)
    nodes_json.append({
        "id": user_id,
        "topic": USER_NAME,
        "kind": "user",
        "domain": "",
        "description": "Your personal knowledge graph",
        "stability": 999,
        "difficulty": 0,
        "retrievability": 1,
        "classification": "known",
        "last_reviewed": "",
        "review_count": 0,
        "created_at": "",
        "reviews": [],
    })

    # Add edges (only between visible nodes)
    edges_json = []
    for e in all_edges:
        src = e["source"]
        tgt = e["target"]
        if src in visible_ids and tgt in visible_ids:
            edges_json.append(e)

    # Add synthetic topic→domain edges (mirrors DB edges created by rocky when topics are added)
    for tid in topic_ids:
        if tid not in all_nodes:
            continue
        domain = all_nodes[tid]["domain"]
        if not domain:
            continue
        did = domain.lower()
        if did not in visible_ids:
            continue
        edges_json.append({
            "id": f"{tid}-{did}-part_of",
            "source": tid,
            "target": did,
            "kind": "part_of",
            "description": f"{all_nodes[tid]['topic']} is a topic within the {domain} domain.",
            "strength": 1.0,
        })

    # Add synthetic domain→user edges
    for domain in active_domains:
        did = domain.lower()
        edges_json.append({
            "id": f"{did}-user",
            "source": did,
            "target": user_id,
            "kind": "part_of",
            "description": "",
            "strength": 1.0,
        })

    return {"nodes": nodes_json, "edges": edges_json, "userName": USER_NAME, "summaries": SUMMARIES}


def main():
    if not os.path.exists(DEMO_DB):
        print(f"ERROR: Demo database not found at {DEMO_DB}", file=sys.stderr)
        sys.exit(1)
    if not os.path.exists(TEMPLATE):
        print(f"ERROR: view.html template not found at {TEMPLATE}", file=sys.stderr)
        sys.exit(1)

    with open(TEMPLATE) as f:
        template = f.read()

    all_nodes, all_edges = load_demo_data()
    os.makedirs(OUT_DIR, exist_ok=True)

    for stage, topic_ids in sorted(STAGE_TOPICS.items()):
        pkg_data = build_stage_json(all_nodes, all_edges, topic_ids)
        data_str = json.dumps(pkg_data, separators=(",", ":"))
        html = template.replace("__DATA_JSON__", data_str)
        out_path = os.path.join(OUT_DIR, f"stage{stage}.html")
        with open(out_path, "w") as f:
            f.write(html)

        topic_count = len([n for n in pkg_data["nodes"] if n["kind"] not in ("domain", "user")])
        edge_count = len([e for e in pkg_data["edges"] if not e["id"].endswith("-user")])
        print(f"  stage{stage}.html: {topic_count} topics, {edge_count} edges")

    print(f"\nGenerated {len(STAGE_TOPICS)} graph files in {OUT_DIR}")


if __name__ == "__main__":
    main()
