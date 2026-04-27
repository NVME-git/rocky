/// Obsidian PKG exporter.
///
/// File layout:
///   pkg/{Domain}/topic-id.md   — one file per PKG node
///   pkg/pkg.json               — full PKG backup for cross-machine restore
///   pkg/Rocky Dashboard.md     — Dataview dashboard
///   pkg/Rocky Review Queue.md  — review queue
///
/// Nodes with no domain (empty string) are written to pkg root for
/// backwards compatibility; run `rocky classify` then `rocky export` to migrate them.
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;

use crate::db::{Db, Edge};
use crate::fsrs;
use crate::node::Node;

// ── File path helpers ─────────────────────────────────────────────────────────

fn node_path(node: &Node, pkg_dir: &Path) -> PathBuf {
    if node.domain.is_empty() {
        pkg_dir.join(format!("{}.md", node.id))
    } else {
        pkg_dir.join(&node.domain).join(format!("{}.md", node.id))
    }
}

// ── Write a single node ───────────────────────────────────────────────────────

/// Write a node with edges resolved from the full edge list.
/// `mastery` is the mean of the node's last-3 review scores (0.5 default when none).
pub fn write_node_with_edges(node: &Node, pkg_dir: &Path, all_edges: &[Edge], mastery: f64) -> Result<()> {
    let edges: Vec<&Edge> = all_edges.iter()
        .filter(|e| e.source_id == node.id || e.target_id == node.id)
        .collect();
    write_node_inner(node, pkg_dir, &edges, mastery)
}

/// Write a node with no edge context (used when edges are unavailable).
pub fn write_node(node: &Node, pkg_dir: &Path, mastery: f64) -> Result<()> {
    write_node_inner(node, pkg_dir, &[], mastery)
}

fn recall_status(recall_now: f64) -> &'static str {
    if recall_now >= fsrs::KNOWN_RECALL { "known" }
    else if recall_now >= fsrs::STALE_RECALL { "fading" }
    else { "gap" }
}

fn write_node_inner(node: &Node, pkg_dir: &Path, edges: &[&Edge], mastery: f64) -> Result<()> {
    let path = node_path(node, pkg_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // If the node was previously written at a different location (e.g. root
    // before domain was assigned), remove the stale file.
    let root_path = pkg_dir.join(format!("{}.md", node.id));
    if root_path != path && root_path.exists() {
        std::fs::remove_file(&root_path).ok();
    }

    let r = fsrs::retrievability(node.stability, node.last_reviewed);
    let recall_now = fsrs::recall_now(r, mastery);
    let today = Local::now().date_naive();
    let days_since = (today - node.last_reviewed).num_days();
    let status = recall_status(recall_now);

    let domain_tag = if node.domain.is_empty() {
        String::new()
    } else {
        format!("\nrocky_domain: {}", node.domain)
    };

    let repo_tag = if node.repo.is_empty() {
        String::new()
    } else {
        format!("\nrocky_repo: {}", node.repo)
    };

    let kind_tag = node.kind.as_str();

    let domain_folder_tag = if node.domain.is_empty() {
        String::new()
    } else {
        format!(", rocky/domain/{}", node.domain.to_lowercase())
    };

    let repo_folder_tag = if node.repo.is_empty() {
        String::new()
    } else {
        format!(", rocky/repo/{}", node.repo.to_lowercase().replace(' ', "-"))
    };

    let frontmatter = format!(
        "---\nrocky_id: {}\nrocky_kind: {kind_tag}{domain_tag}{repo_tag}\nrocky_status: {status}\nrocky_difficulty: {:.3}\nrocky_stability: {:.2}\nrocky_retrievability: {:.4}\nrocky_mastery: {:.4}\nrocky_recall: {:.4}\nrocky_last_reviewed: {}\nrocky_last_encountered: {}\nrocky_review_count: {}\nrocky_days_since_review: {}\ntags: [rocky/node, rocky/kind/{kind_tag}, rocky/status/{status}{domain_folder_tag}{repo_folder_tag}]\n---\n",
        node.id,
        node.difficulty,
        node.stability,
        r,
        mastery,
        recall_now,
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

    // Canonical Q&A
    if !node.canonical_question.is_empty() {
        body.push_str("\n## Question\n\n");
        body.push_str(&node.canonical_question);
        body.push('\n');
        if !node.canonical_answer.is_empty() {
            body.push_str("\n## Ideal Answer\n\n");
            body.push_str(&node.canonical_answer);
            body.push('\n');
        }
    }

    // Contexts
    if !node.contexts.is_empty() {
        body.push_str("\n## Contexts\n");
        for ctx in &node.contexts {
            body.push_str(&format!("- {ctx}\n"));
        }
    }

    // Edge-based related links
    if !edges.is_empty() {
        body.push_str("\n## Related\n");
        for edge in edges {
            let (other_id, arrow, kind) = if edge.source_id == node.id {
                (&edge.target_id, "→", edge.kind.as_str())
            } else {
                (&edge.source_id, "←", edge.kind.as_str())
            };
            let desc = if edge.description.is_empty() {
                String::new()
            } else {
                format!(" — {}", edge.description)
            };
            body.push_str(&format!("- [[{other_id}]] {arrow} `{kind}`{desc}\n"));
        }
    }

    std::fs::write(path, format!("{frontmatter}\n{body}"))?;
    Ok(())
}

// ── Delete a node's PKG file ─────────────────────────────────────────────────

pub fn delete_node(node_id: &str, domain: &str, pkg_dir: &Path) {
    // Try both domain subfolder and root (covers migration edge cases)
    if !domain.is_empty() {
        std::fs::remove_file(pkg_dir.join(domain).join(format!("{node_id}.md"))).ok();
    }
    std::fs::remove_file(pkg_dir.join(format!("{node_id}.md"))).ok();
}

// ── Export all nodes ──────────────────────────────────────────────────────────

pub fn write_all(db: &Db, nodes: &[Node], all_edges: &[Edge], pkg_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(pkg_dir)?;

    for node in nodes {
        let (_, mastery, _) = db.node_recall(node);
        write_node_with_edges(node, pkg_dir, all_edges, mastery)?;
    }

    write_dashboard_pages(pkg_dir)?;
    Ok(nodes.len())
}

// ── Dashboard pages ───────────────────────────────────────────────────────────

pub fn write_dashboard_pages(pkg_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(pkg_dir)?;
    std::fs::write(pkg_dir.join("Rocky Dashboard.md"), DASHBOARD)?;
    std::fs::write(pkg_dir.join("Rocky Review Queue.md"), REVIEW_QUEUE)?;
    Ok(())
}

// ── Dashboard page ────────────────────────────────────────────────────────────

const DASHBOARD: &str = r#"---
tags: [rocky/meta]
---

# Rocky — Knowledge Dashboard

> This file is managed by Rocky. Run `rocky export` to refresh node files.
> Dashboard queries are live — they always reflect the current state of your PKG.

---

## Overview

```dataviewjs
const pages = dv.pages('#rocky/node');
const known  = pages.filter(p => p.rocky_recall >= 0.6).length;
const fading = pages.filter(p => p.rocky_recall >= 0.3 && p.rocky_recall < 0.6).length;
const gaps   = pages.filter(p => p.rocky_recall < 0.3).length;
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
  round(rocky_recall * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_days_since_review + "d ago" AS "Last Reviewed",
  round(rocky_difficulty * 100) + "%" AS "Difficulty"
FROM #rocky/node
WHERE rocky_recall < 0.3
SORT rocky_recall ASC
```

---

## 🟡 Fading — Review Soon

```dataview
TABLE
  round(rocky_recall * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_last_reviewed AS "Last Reviewed",
  round(rocky_stability) + "d" AS "Stability"
FROM #rocky/node
WHERE rocky_recall >= 0.3 AND rocky_recall < 0.6
SORT rocky_recall ASC
```

---

## 🟢 Known

```dataview
TABLE
  round(rocky_recall * 100) + "%" AS "Recall",
  round(rocky_stability) + "d" AS "Stability",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_recall >= 0.6
SORT rocky_stability DESC
```

---

## By Domain

```dataviewjs
const domains = [...new Set(dv.pages('#rocky/node').map(p => p.rocky_domain).filter(d => d))];
for (const domain of domains.sort()) {
  const pages = dv.pages('#rocky/node').filter(p => p.rocky_domain === domain);
  const known = pages.filter(p => p.rocky_recall >= 0.6).length;
  dv.header(3, `${domain} (${pages.length} topics · ${known} known)`);
  dv.table(
    ["Topic", "Recall", "Kind", "Reviews"],
    pages.sort(p => p.rocky_recall).map(p => [
      p.file.link,
      Math.round(p.rocky_recall * 100) + "%",
      p.rocky_kind,
      p.rocky_review_count
    ])
  );
}
```

---

## By Project

```dataviewjs
const repos = [...new Set(dv.pages('#rocky/node').map(p => p.rocky_repo).filter(r => r))];
if (repos.length === 0) {
  dv.paragraph("_No repo tags found. Run `rocky backfill` in your projects to tag topics._");
} else {
  for (const repo of repos.sort()) {
    const pages = dv.pages('#rocky/node').filter(p => p.rocky_repo === repo);
    const known = pages.filter(p => p.rocky_recall >= 0.6).length;
    const gaps  = pages.filter(p => p.rocky_recall < 0.3).length;
    dv.header(3, `${repo} (${pages.length} topics · ${known} known · ${gaps} gaps)`);
    dv.table(
      ["Topic", "Recall", "Domain", "Kind"],
      pages.sort(p => p.rocky_recall).map(p => [
        p.file.link,
        Math.round(p.rocky_recall * 100) + "%",
        p.rocky_domain,
        p.rocky_kind
      ])
    );
  }
}
```

---

## Hardest Topics

```dataview
TABLE
  round(rocky_difficulty * 100) + "%" AS "Difficulty",
  round(rocky_recall * 100) + "%" AS "Recall",
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
  round(rocky_recall * 100) + "%" AS "Recall",
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
  round(rocky_recall * 100) + "%" AS "Recall",
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
> This page auto-updates whenever Rocky writes to the PKG.

---

## Priority Order

```dataview
TABLE
  round(rocky_recall * 100) + "%" AS "Recall",
  rocky_domain AS "Domain",
  rocky_kind AS "Kind",
  rocky_days_since_review + "d ago" AS "Last Reviewed",
  round(rocky_difficulty * 100) + "%" AS "Difficulty"
FROM #rocky/node
WHERE rocky_recall < 0.6
SORT rocky_recall ASC
```

---

## By Domain

```dataviewjs
const domains = [...new Set(dv.pages('#rocky/node').filter(p => p.rocky_recall < 0.6).map(p => p.rocky_domain).filter(d => d))];
for (const domain of domains.sort()) {
  const pages = dv.pages('#rocky/node').filter(p => p.rocky_domain === domain && p.rocky_recall < 0.6);
  if (pages.length === 0) continue;
  dv.header(3, domain);
  dv.table(
    ["Topic", "Recall", "Days Since Review"],
    pages.sort(p => p.rocky_recall).map(p => [
      p.file.link,
      Math.round(p.rocky_recall * 100) + "%",
      p.rocky_days_since_review + "d ago"
    ])
  );
}
```

---

## By Project

```dataviewjs
const repos = [...new Set(dv.pages('#rocky/node').filter(p => p.rocky_recall < 0.6).map(p => p.rocky_repo).filter(r => r))];
for (const repo of repos.sort()) {
  const pages = dv.pages('#rocky/node').filter(p => p.rocky_repo === repo && p.rocky_recall < 0.6);
  if (pages.length === 0) continue;
  dv.header(3, repo);
  dv.table(
    ["Topic", "Recall", "Domain", "Days Since Review"],
    pages.sort(p => p.rocky_recall).map(p => [
      p.file.link,
      Math.round(p.rocky_recall * 100) + "%",
      p.rocky_domain,
      p.rocky_days_since_review + "d ago"
    ])
  );
}
```
"#;
