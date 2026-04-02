#!/usr/bin/env python3
"""
Seed the demo knowledge graph with pre-existing topics in various states.

Run this before run.py to start from an interesting, realistic baseline
instead of an empty graph:

    python demo/seed.py

Topics are placed at different confidence levels and backdated to trigger
decay, so the demo immediately shows all three classifications:
  known  — recently reviewed, high confidence
  stale  — not touched in a while, confidence has faded
  new    — never seen before (not present in graph at all)
"""

import json
from datetime import date, timedelta
from pathlib import Path

DEMO_GRAPH = Path(__file__).parent / "demo_graph.json"

today = date.today()

nodes = {
    "rest-api-design": {
        "id": "rest-api-design",
        "topic": "REST API design",
        "kind": "pattern",
        "description": "Stateless HTTP API design using resources, verbs, and status codes.",
        "confidence": 0.88,
        "last_reviewed": (today - timedelta(days=5)).isoformat(),
        "last_encountered": (today - timedelta(days=5)).isoformat(),
        "review_count": 4,
        "contexts": ["build a user management API", "add CRUD endpoints for products"],
    },
    "sql-query-optimization": {
        "id": "sql-query-optimization",
        "topic": "SQL query optimization",
        "kind": "implementation",
        "description": "Techniques to improve query performance: indexes, explain plans, avoiding N+1.",
        "confidence": 0.75,
        # implementation half-life is 30 days — 45 days back drops this into stale territory
        "last_reviewed": (today - timedelta(days=45)).isoformat(),
        "last_encountered": (today - timedelta(days=45)).isoformat(),
        "review_count": 2,
        "contexts": ["optimize slow product listing queries"],
    },
    "http-caching": {
        "id": "http-caching",
        "topic": "HTTP caching",
        "kind": "concept",
        "description": "Browser and proxy caching via Cache-Control headers, ETags, and invalidation.",
        "confidence": 0.55,
        # concept half-life is 180 days — 120 days back drops this into stale territory
        "last_reviewed": (today - timedelta(days=120)).isoformat(),
        "last_encountered": (today - timedelta(days=120)).isoformat(),
        "review_count": 1,
        "contexts": ["add caching to API responses"],
    },
}

graph_data = {
    "nodes": nodes,
    "meta": {"created": (today - timedelta(days=90)).isoformat()},
}

with open(DEMO_GRAPH, "w") as f:
    json.dump(graph_data, f, indent=2)

print(f"Seeded {len(nodes)} topics → {DEMO_GRAPH.name}")
print()
print(f"  {'Topic':<35} {'Kind':<16} {'Confidence':<12} Last reviewed")
print("  " + "─" * 74)
for n in nodes.values():
    days_ago = (today - date.fromisoformat(n["last_reviewed"])).days
    print(f"  {n['topic']:<35} {n['kind']:<16} {n['confidence']:<12.0%} {days_ago}d ago")
