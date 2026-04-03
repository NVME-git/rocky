/// Obsidian vault exporter.
///
/// File layout:
///   vault/{Domain}/topic-id.md   — one file per PKG node
///   vault/pkg.json               — full PKG backup for cross-machine restore
///   vault/Rocky Dashboard.md     — Dataview dashboard
///   vault/Rocky Review Queue.md  — review queue
///
/// Nodes with no domain (empty string) are written to vault root for
/// backwards compatibility; run `rocky export --classify` to migrate them.
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;

use crate::fsrs;
use crate::node::Node;

// ── File path helpers ─────────────────────────────────────────────────────────

fn node_path(node: &Node, vault_dir: &Path) -> PathBuf {
    if node.domain.is_empty() {
        vault_dir.join(format!("{}.md", node.id))
    } else {
        vault_dir.join(&node.domain).join(format!("{}.md", node.id))
    }
}

// ── Write a single node ───────────────────────────────────────────────────────

pub fn write_node(node: &Node, vault_dir: &Path) -> Result<()> {
    write_node_inner(node, vault_dir, &[])
}

pub fn write_node_with_links(node: &Node, vault_dir: &Path, related_ids: &[String]) -> Result<()> {
    write_node_inner(node, vault_dir, related_ids)
}

fn write_node_inner(node: &Node, vault_dir: &Path, related_ids: &[String]) -> Result<()> {
    let path = node_path(node, vault_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // If the node was previously written at a different location (e.g. root
    // before domain was assigned), remove the stale file.
    let root_path = vault_dir.join(format!("{}.md", node.id));
    if root_path != path && root_path.exists() {
        std::fs::remove_file(&root_path).ok();
    }

    let r = fsrs::retrievability(node.stability, node.last_reviewed);
    let today = Local::now().date_naive();
    let days_since = (today - node.last_reviewed).num_days();

    let domain_tag = if node.domain.is_empty() {
        String::new()
    } else {
        format!("\nrocky_domain: {}", node.domain)
    };

    let kind_tag = node.kind.as_str();
    let domain_folder_tag = if node.domain.is_empty() {
        String::new()
    } else {
        format!(", rocky/domain/{}", node.domain.to_lowercase())
    };

    let frontmatter = format!(
        "---\nrocky_id: {}\nrocky_kind: {kind_tag}{domain_tag}\nrocky_difficulty: {:.3}\nrocky_stability: {:.2}\nrocky_retrievability: {:.4}\nrocky_last_reviewed: {}\nrocky_last_encountered: {}\nrocky_review_count: {}\nrocky_days_since_review: {}\ntags: [rocky/node, rocky/kind/{kind_tag}{domain_folder_tag}]\n---\n",
        node.id,
        node.difficulty,
        node.stability,
        r,
        node.last_reviewed,
        node.last_encountered,
        node.review_count,
        days_since,
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
    if !related_ids.is_empty() {
        body.push_str("\n## Related\n");
        for id in related_ids {
            body.push_str(&format!("- [[{id}]]\n"));
        }
    }

    std::fs::write(path, format!("{frontmatter}\n{body}"))?;
    Ok(())
}

// ── Delete a node's vault file ────────────────────────────────────────────────

pub fn delete_node(node_id: &str, domain: &str, vault_dir: &Path) {
    // Try both domain subfolder and root (covers migration edge cases)
    if !domain.is_empty() {
        std::fs::remove_file(vault_dir.join(domain).join(format!("{node_id}.md"))).ok();
    }
    std::fs::remove_file(vault_dir.join(format!("{node_id}.md"))).ok();
}

// ── Export all nodes ──────────────────────────────────────────────────────────

pub fn write_all(nodes: &[Node], vault_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(vault_dir)?;

    // Pre-compute wikilinks: for each node, find others whose topic ID shares
    // a meaningful keyword (word >3 chars from the ID slug)
    let all_ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    for node in nodes {
        let related = find_related_ids(node, &all_ids);
        write_node_with_links(node, vault_dir, &related)?;
    }

    write_dashboard_pages(vault_dir)?;
    Ok(nodes.len())
}

fn find_related_ids(node: &Node, all_ids: &[&str]) -> Vec<String> {
    // Split this node's ID on '-', keep words >3 chars, match against other IDs
    let keywords: Vec<&str> = node.id.split('-').filter(|w| w.len() > 3).collect();
    if keywords.is_empty() {
        return vec![];
    }
    all_ids.iter()
        .filter(|&&id| id != node.id)
        .filter(|&&id| keywords.iter().any(|kw| id.contains(kw)))
        .map(|&id| id.to_string())
        .take(5) // cap at 5 related links per node
        .collect()
}

// ── Dashboard pages ───────────────────────────────────────────────────────────

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

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_days_since_review + "d ago" AS "Last Reviewed",
  round(rocky_difficulty * 100) + "%" AS "Difficulty"
FROM #rocky/node
WHERE rocky_retrievability < 0.7
SORT rocky_retrievability ASC
```

---

## 🟡 Fading — Review Soon

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_last_reviewed AS "Last Reviewed",
  round(rocky_stability) + "d" AS "Stability"
FROM #rocky/node
WHERE rocky_retrievability >= 0.7 AND rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```

---

## 🟢 Known

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_stability) + "d" AS "Stability",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability >= 0.9
SORT rocky_stability DESC
```

---

## By Domain

```dataviewjs
const domains = [...new Set(dv.pages('#rocky/node').map(p => p.rocky_domain).filter(d => d))];
for (const domain of domains.sort()) {
  const pages = dv.pages('#rocky/node').filter(p => p.rocky_domain === domain);
  const known = pages.filter(p => p.rocky_retrievability >= 0.9).length;
  dv.header(3, `${domain} (${pages.length} topics · ${known} known)`);
  dv.table(
    ["Topic", "Recall", "Kind", "Reviews"],
    pages.sort(p => p.rocky_retrievability).map(p => [
      p.file.link,
      Math.round(p.rocky_retrievability * 100) + "%",
      p.rocky_kind,
      p.rocky_review_count
    ])
  );
}
```

---

## Hardest Topics

```dataview
TABLE
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_difficulty > 0.4
SORT rocky_difficulty DESC
LIMIT 15
```

---

## Most Practiced

```dataview
TABLE
  rocky_review_count AS "Reviews",
  round(rocky_retrievability * 100) + "%" AS "Recall",
  round(rocky_stability) + "d" AS "Stability",
  rocky_domain AS "Domain"
FROM #rocky/node
SORT rocky_review_count DESC
LIMIT 15
```

---

## Recently Encountered

```dataview
TABLE
  rocky_last_encountered AS "Last Encountered",
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
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

```dataview
TABLE
  round(rocky_retrievability * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_days_since_review + "d ago" AS "Last Reviewed",
  round(rocky_difficulty * 100) + "%" AS "Difficulty"
FROM #rocky/node
WHERE rocky_retrievability < 0.9
SORT rocky_retrievability ASC
```

---

## By Domain

```dataviewjs
const domains = [...new Set(dv.pages('#rocky/node').filter(p => p.rocky_retrievability < 0.9).map(p => p.rocky_domain).filter(d => d))];
for (const domain of domains.sort()) {
  const pages = dv.pages('#rocky/node').filter(p => p.rocky_domain === domain && p.rocky_retrievability < 0.9);
  if (pages.length === 0) continue;
  dv.header(3, domain);
  dv.table(
    ["Topic", "Recall", "Days Since Review"],
    pages.sort(p => p.rocky_retrievability).map(p => [
      p.file.link,
      Math.round(p.rocky_retrievability * 100) + "%",
      p.rocky_days_since_review + "d ago"
    ])
  );
}
```
"#;
