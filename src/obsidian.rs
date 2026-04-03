/// Obsidian vault exporter.
///
/// Writes one markdown file per PKG node. Files use YAML frontmatter so
/// Obsidian's Dataview plugin can query them:
///
///   TABLE rocky_retrievability, rocky_last_reviewed
///   FROM #rocky/node
///   WHERE rocky_retrievability < 0.7
///   SORT rocky_retrievability ASC
use std::path::Path;

use anyhow::Result;
use chrono::Local;

use crate::fsrs;
use crate::node::Node;

pub fn write_node(node: &Node, vault_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(vault_dir)?;

    let r = fsrs::retrievability(node.stability, node.last_reviewed);
    let today = Local::now().date_naive();
    let days_since = (today - node.last_reviewed).num_days();

    let frontmatter = format!(
        "---\nrocky_id: {}\nrocky_kind: {}\nrocky_difficulty: {}\nrocky_stability: {}\nrocky_retrievability: {}\nrocky_last_reviewed: {}\nrocky_last_encountered: {}\nrocky_review_count: {}\nrocky_days_since_review: {}\ntags: [rocky/node, rocky/kind/{}]\n---\n",
        node.id,
        node.kind.as_str(),
        node.difficulty,
        node.stability,
        r,
        node.last_reviewed,
        node.last_encountered,
        node.review_count,
        days_since,
        node.kind.as_str(),
    );

    let mut body = format!("# {}\n\n", node.topic);
    if !node.description.is_empty() {
        body.push_str(&node.description);
        body.push('\n');
    }
    if !node.contexts.is_empty() {
        body.push_str("\n## Contexts\n");
        for ctx in &node.contexts {
            body.push_str(&format!("- {ctx}\n"));
        }
    }

    let content = format!("{frontmatter}\n{body}");
    let file_path = vault_dir.join(format!("{}.md", node.id));
    std::fs::write(file_path, content)?;
    Ok(())
}

pub fn write_all(nodes: &[Node], vault_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(vault_dir)?;
    for node in nodes {
        write_node(node, vault_dir)?;
    }
    write_dashboard_pages(vault_dir)?;
    Ok(nodes.len())
}

/// Write the Rocky Dashboard and Review Queue pages to the vault.
/// Always overwrites so queries stay up-to-date as Rocky adds new fields.
pub fn write_dashboard_pages(vault_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(vault_dir)?;
    std::fs::write(vault_dir.join("Rocky Dashboard.md"), DASHBOARD)?;
    std::fs::write(vault_dir.join("Rocky Review Queue.md"), REVIEW_QUEUE)?;
    Ok(())
}

// ── Dashboard page ────────────────────────────────────────────────────────────

const DASHBOARD: &str = r#"---
tags: [rocky/meta]
---

# Rocky — Knowledge Dashboard

> This file is managed by Rocky. Run `rocky export` to refresh node files.
> Dashboard queries are live — they always reflect the current state of your vault.

---

## Overview

```dataviewjs
const pages = dv.pages('#rocky/node');
const known  = pages.filter(p => p.rocky_retrievability >= 0.9).length;
const fading = pages.filter(p => p.rocky_retrievability >= 0.7 && p.rocky_retrievability < 0.9).length;
const gaps   = pages.filter(p => p.rocky_retrievability < 0.7).length;
const total  = pages.length;
dv.paragraph(
  `**${total} topics total** — ` +
  `🟢 **${known}** known · ` +
  `🟡 **${fading}** fading · ` +
  `🔴 **${gaps}** gaps`
);
```

> [!tip] DataviewJS must be enabled in Obsidian Settings → Dataview for the overview block above.
> The query tables below work with standard Dataview.

---

## 🔴 Gaps — Review Now

Topics where recall has dropped below 70%. Run `rocky quiz` to work through these.

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_kind AS "Kind",
  rocky_days_since_review + "d ago" AS "Last Reviewed",
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability < 0.7
SORT rocky_retrievability ASC
```

---

## 🟡 Fading — Review Soon

Topics you know but are starting to slip. A quick reminder is all they need.

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_kind AS "Kind",
  rocky_last_reviewed AS "Last Reviewed",
  round(rocky_stability) + "d" AS "Stability"
FROM #rocky/node
WHERE rocky_retrievability >= 0.7 AND rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```

---

## 🟢 Known

Solid topics. Rocky will only surface these again if they start to fade.

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_stability) + "d" AS "Stability",
  rocky_kind AS "Kind",
  rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability >= 0.9
SORT rocky_stability DESC
```

---

## By Kind

### Concepts

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  rocky_review_count AS "Reviews",
  rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/kind/concept
SORT rocky_retrievability ASC
```

### Patterns

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  rocky_review_count AS "Reviews",
  rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/kind/pattern
SORT rocky_retrievability ASC
```

### Implementations

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  rocky_review_count AS "Reviews",
  rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/kind/implementation
SORT rocky_retrievability ASC
```

---

## Hardest Topics

Topics with the highest difficulty score — these have taken the most repetitions to stick.

```dataview
TABLE
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_review_count AS "Reviews",
  round(rocky_stability) + "d" AS "Stability"
FROM #rocky/node
WHERE rocky_difficulty > 0.4
SORT rocky_difficulty DESC
LIMIT 15
```

---

## Most Practiced

Topics you've come back to the most — either because they're hard or genuinely important.

```dataview
TABLE
  rocky_review_count AS "Reviews",
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_stability) + "d" AS "Stability",
  rocky_kind AS "Kind"
FROM #rocky/node
SORT rocky_review_count DESC
LIMIT 15
```

---

## Recently Encountered

Topics that appeared in your recent tasks or diffs, regardless of review status.

```dataview
TABLE
  rocky_last_encountered AS "Last Encountered",
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_kind AS "Kind"
FROM #rocky/node
SORT rocky_last_encountered DESC
LIMIT 20
```
"#;

// ── Review Queue page ─────────────────────────────────────────────────────────

const REVIEW_QUEUE: &str = r#"---
tags: [rocky/meta]
---

# Rocky — Review Queue

> Run `rocky quiz` to work through this list interactively.
> This page auto-updates whenever Rocky writes to the vault.

---

## Priority Order

All topics that need attention, sorted by urgency (lowest recall first).

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_kind AS "Kind",
  rocky_days_since_review + "d ago" AS "Last Reviewed",
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```

---

## Overdue by Kind

### Concepts to re-learn

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_days_since_review + "d ago" AS "Last Reviewed"
FROM #rocky/kind/concept
WHERE rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```

### Patterns to re-learn

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_days_since_review + "d ago" AS "Last Reviewed"
FROM #rocky/kind/pattern
WHERE rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```

### Implementations to re-learn

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_days_since_review + "d ago" AS "Last Reviewed"
FROM #rocky/kind/implementation
WHERE rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```
"#;
