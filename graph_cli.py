#!/usr/bin/env python3
"""
Thin CLI wrapper around graph.py for use by the PKG Claude Code skill.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from graph import KnowledgeGraph

graph = KnowledgeGraph()


def cmd_classify(topic: str):
    result = {
        "classification": graph.classify(topic),
        "confidence": graph.effective_confidence(topic),
        "node": graph.get(topic),
    }
    print(json.dumps(result))


def cmd_update(topic: str, confidence: float, kind: str, description: str, context: str):
    graph.add_or_update(topic, confidence, kind=kind, description=description, context=context)
    print(json.dumps({"ok": True, "topic": topic}))


def cmd_encountered(topic: str):
    graph.mark_encountered(topic)
    print(json.dumps({"ok": True}))


def cmd_list():
    nodes = graph.all_topics()
    enriched = []
    for n in nodes:
        enriched.append({
            **n,
            "classification": graph.classify(n["topic"]),
            "effective_confidence": graph.effective_confidence(n["topic"]),
        })
    enriched.sort(key=lambda x: x["effective_confidence"], reverse=True)
    print(json.dumps(enriched))


def cmd_stats():
    print(json.dumps(graph.summary()))


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
