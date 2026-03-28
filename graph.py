"""
PKG - Personal Knowledge Graph
Knowledge graph storage and classification logic.
"""

import json
import math
from datetime import datetime, date
from pathlib import Path
from typing import Optional

GRAPH_FILE = Path(__file__).parent / "knowledge_graph.json"

# Confidence thresholds
KNOWN_THRESHOLD = 0.70
STALE_THRESHOLD = 0.35

# Decay rates (half-life in days)
DECAY_HALFLIFE = {
    "implementation": 30,
    "pattern": 90,
    "concept": 180,
}
DEFAULT_HALFLIFE = 60


def _decay(confidence: float, last_reviewed: str, kind: str = "concept") -> float:
    """Apply time-based confidence decay."""
    halflife = DECAY_HALFLIFE.get(kind, DEFAULT_HALFLIFE)
    last = date.fromisoformat(last_reviewed)
    days_elapsed = (date.today() - last).days
    if days_elapsed <= 0:
        return confidence
    decayed = confidence * math.pow(0.5, days_elapsed / halflife)
    return round(decayed, 4)


class KnowledgeGraph:
    def __init__(self, path: Path = GRAPH_FILE):
        self.path = path
        self.data: dict = self._load()

    def _load(self) -> dict:
        if self.path.exists():
            with open(self.path) as f:
                return json.load(f)
        return {"nodes": {}, "meta": {"created": date.today().isoformat()}}

    def save(self):
        with open(self.path, "w") as f:
            json.dump(self.data, f, indent=2)

    def _node_id(self, topic: str) -> str:
        return topic.lower().strip().replace(" ", "-")

    def get(self, topic: str) -> Optional[dict]:
        node_id = self._node_id(topic)
        return self.data["nodes"].get(node_id)

    def classify(self, topic: str) -> str:
        """Return 'known', 'stale', or 'new' for a topic."""
        node = self.get(topic)
        if node is None:
            return "new"
        effective = _decay(node["confidence"], node["last_reviewed"], node.get("kind", "concept"))
        if effective >= KNOWN_THRESHOLD:
            return "known"
        if effective >= STALE_THRESHOLD:
            return "stale"
        return "new"

    def effective_confidence(self, topic: str) -> float:
        node = self.get(topic)
        if node is None:
            return 0.0
        return _decay(node["confidence"], node["last_reviewed"], node.get("kind", "concept"))

    def add_or_update(self, topic: str, confidence_delta: float, kind: str = "concept",
                      description: str = "", context: str = ""):
        """Add a new node or update an existing one."""
        node_id = self._node_id(topic)
        today = date.today().isoformat()

        if node_id in self.data["nodes"]:
            node = self.data["nodes"][node_id]
            # Apply decay first, then add delta
            current = _decay(node["confidence"], node["last_reviewed"], node.get("kind", kind))
            new_conf = max(0.0, min(1.0, current + confidence_delta))
            node["confidence"] = round(new_conf, 4)
            node["last_reviewed"] = today
            node["review_count"] = node.get("review_count", 0) + 1
            if context:
                node.setdefault("contexts", [])
                if context not in node["contexts"]:
                    node["contexts"].append(context)
        else:
            self.data["nodes"][node_id] = {
                "id": node_id,
                "topic": topic,
                "kind": kind,
                "description": description,
                "confidence": max(0.0, min(1.0, confidence_delta)),
                "last_reviewed": today,
                "last_encountered": today,
                "review_count": 1,
                "contexts": [context] if context else [],
            }
        self.save()

    def mark_encountered(self, topic: str):
        """Record that the topic was encountered (without a Q&A session)."""
        node_id = self._node_id(topic)
        if node_id in self.data["nodes"]:
            self.data["nodes"][node_id]["last_encountered"] = date.today().isoformat()
            self.save()

    def summary(self) -> dict:
        nodes = self.data["nodes"]
        total = len(nodes)
        known = sum(1 for t in nodes if self.classify(nodes[t]["topic"]) == "known")
        stale = sum(1 for t in nodes if self.classify(nodes[t]["topic"]) == "stale")
        new_topics = total - known - stale
        return {"total": total, "known": known, "stale": stale, "gaps": new_topics}

    def all_topics(self) -> list[dict]:
        return list(self.data["nodes"].values())
