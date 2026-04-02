"""PKG node — a single topic in the Personal Knowledge Graph."""

from dataclasses import dataclass, field
from datetime import date
from typing import Literal

Kind = Literal["concept", "pattern", "implementation"]


@dataclass
class Node:
    id: str
    topic: str
    kind: Kind
    description: str
    difficulty: float       # 0–1: how hard this topic has been for this user
    stability: float        # days until retrievability drops to 90%
    last_reviewed: date
    last_encountered: date
    review_count: int
    contexts: list[str] = field(default_factory=list)
    created_at: date = field(default_factory=date.today)

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "topic": self.topic,
            "kind": self.kind,
            "description": self.description,
            "difficulty": self.difficulty,
            "stability": self.stability,
            "last_reviewed": self.last_reviewed.isoformat(),
            "last_encountered": self.last_encountered.isoformat(),
            "review_count": self.review_count,
            "contexts": self.contexts,
            "created_at": self.created_at.isoformat(),
        }
