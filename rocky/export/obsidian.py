"""
Obsidian vault exporter.

Writes one markdown file per PKG node into a configurable vault directory.
Files use YAML frontmatter so Obsidian's Dataview plugin can query them directly:

    TABLE rocky_retrievability, rocky_last_reviewed
    FROM #rocky/node
    WHERE rocky_retrievability < 0.7
    SORT rocky_retrievability ASC
"""

from datetime import date
from pathlib import Path

from rocky.graph.node import Node
from rocky.graph import fsrs


def write_node(node: Node, vault_dir: Path):
    vault_dir.mkdir(parents=True, exist_ok=True)

    r = fsrs.retrievability(node.stability, node.last_reviewed)
    days_since = (date.today() - node.last_reviewed).days

    frontmatter = f"""---
rocky_id: {node.id}
rocky_kind: {node.kind}
rocky_difficulty: {node.difficulty}
rocky_stability: {node.stability}
rocky_retrievability: {r}
rocky_last_reviewed: {node.last_reviewed}
rocky_last_encountered: {node.last_encountered}
rocky_review_count: {node.review_count}
rocky_days_since_review: {days_since}
tags: [rocky/node, rocky/kind/{node.kind}]
---
"""

    body = f"# {node.topic}\n\n"
    if node.description:
        body += f"{node.description}\n"

    if node.contexts:
        body += "\n## Contexts\n"
        for ctx in node.contexts:
            body += f"- {ctx}\n"

    (vault_dir / f"{node.id}.md").write_text(frontmatter + "\n" + body)
