#!/usr/bin/env python3
"""
Thin CLI wrapper around the PKG for use by the Rocky Claude Code skill.
Communicates via JSON over stdout.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from rocky.graph.store import PKG
from rocky.graph import fsrs

pkg = PKG()


def cmd_classify(topic: str):
    node = pkg.get(topic)
    r = pkg.retrievability(topic)
    print(json.dumps({
        "classification": pkg.classify(topic),
        "retrievability": r,
        "node": node.to_dict() if node else None,
    }))


def cmd_update(topic: str, score: float, kind: str, description: str, context: str):
    pkg.add_or_update(topic, score, kind=kind, description=description, context=context)
    print(json.dumps({"ok": True, "topic": topic}))


def cmd_encountered(topic: str):
    pkg.mark_encountered(topic)
    print(json.dumps({"ok": True}))


def cmd_list():
    nodes = pkg.all_topics()
    enriched = []
    for n in nodes:
        r = fsrs.retrievability(n.stability, n.last_reviewed)
        d = n.to_dict()
        d["classification"] = fsrs.classify(r)
        d["retrievability"] = r
        enriched.append(d)
    enriched.sort(key=lambda x: x["retrievability"], reverse=True)
    print(json.dumps(enriched))


def cmd_stats():
    print(json.dumps(pkg.summary()))


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: graph_cli.py <command> [args...]")
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "classify" and len(sys.argv) == 3:
        cmd_classify(sys.argv[2])
    elif cmd == "update" and len(sys.argv) == 7:
        cmd_update(sys.argv[2], float(sys.argv[3]), sys.argv[4], sys.argv[5], sys.argv[6])
    elif cmd == "encountered" and len(sys.argv) == 3:
        cmd_encountered(sys.argv[2])
    elif cmd == "list":
        cmd_list()
    elif cmd == "stats":
        cmd_stats()
    else:
        print(f"Unknown command: {cmd}", file=sys.stderr)
        sys.exit(1)
