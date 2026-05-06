---
title: "Obsidian Integration"
order: 8
---
Rocky writes your entire knowledge graph as Markdown files into a PKG directory. Each topic becomes a note with metadata that Obsidian's Dataview plugin can query and visualise.

## Setup

### 1. Tell Rocky where your PKG directory is

In `~/.config/rocky/config.toml`:

```toml
[export]
pkg_dir = "~/Documents/Obsidian/MyVault/rocky"
```

Rocky will write all files directly into the path you set. If pointing to an Obsidian vault, set it to a subfolder like `~/Documents/Obsidian/MyVault/rocky`.

### 2. Export your topics

```bash
rocky export
```

This writes one `.md` file per topic, grouped by domain into subfolders. Rocky also exports automatically every time a topic is updated, so your PKG stays in sync.

### 3. Install the Dataview plugin in Obsidian

1. Open Obsidian → Settings → Community plugins
2. Search for "Dataview" and install it
3. Enable it

---

## Vault structure

Topics are grouped by domain into subfolders:

```
pkg/
  Rocky Dashboard.md       ← auto-created overview dashboard
  Rocky Review Queue.md    ← auto-created review queue
  pkg.json                 ← full PKG backup (for sync/restore)
  Language/
    rust-ownership.md
    python-decorators.md
  Database/
    redis-ttl-expiry.md
    sql-indexes.md
  Auth/
    jwt-authentication.md
  ...
```

The 13 domains: Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other.

---

## What the notes look like

Each topic becomes a file like `Auth/jwt-authentication.md`:

```markdown
---
rocky_id: jwt-authentication
rocky_kind: pattern
rocky_domain: Auth
rocky_repo: taskify
rocky_status: known
rocky_difficulty: 0.300
rocky_stability: 8.50
rocky_retrievability: 0.9400
rocky_mastery: 0.9000
rocky_recall: 0.8460
rocky_last_reviewed: 2026-04-03
rocky_last_encountered: 2026-04-03
rocky_review_count: 3
rocky_days_since_review: 0
tags: [rocky/node, rocky/kind/pattern, rocky/status/known, rocky/domain/auth, rocky/repo/taskify]
---

# JWT authentication

Stateless token-based auth where the server signs a payload the client stores and sends back.

## Contexts
- add user login with JWT tokens to my Express API

## See also
- [[token-expiry-handling]]
- [[httponly-cookie-security]]
```

The three frontmatter fields that drive Dataview queries:

- `rocky_retrievability` — FSRS freshness alone (R)
- `rocky_mastery` — mean of last-3 review scores (M)
- `rocky_recall` — `R × M`, the field the dashboard pages classify on

`rocky_status` is one of `known | fading | gap`, derived from `rocky_recall` against the same 0.6/0.3 thresholds Rocky uses elsewhere. Use whichever is most ergonomic for your queries.

Related topics are linked via `See also:` wikilinks, so Obsidian's graph view shows the connections between your topics.

---

## Auto-created dashboard pages

Rocky automatically creates two dashboard pages when you export:

### Rocky Dashboard.md

An overview of your entire PKG, including a by-domain breakdown and a table of all topics sorted by recall.

### Rocky Review Queue.md

Topics that need attention — your gaps and fading topics, sorted by most urgent.

Both pages use Dataview queries and update automatically as your PKG changes.

---

## Custom Dataview queries

You can add your own queries to any Obsidian note.

### Topics that need attention

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      round(rocky_mastery * 100) + "%" AS "Mastery",
      rocky_last_reviewed AS "Last Reviewed",
      rocky_kind AS "Kind"
FROM #rocky/node
WHERE rocky_recall < 0.6
SORT rocky_recall ASC
```

### Your strongest topics

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_recall >= 0.6
SORT rocky_review_count DESC
```

### Topics by domain

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/domain/auth
SORT rocky_recall ASC
```

Replace `auth` with any domain name (lowercased).

### Topics by kind

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/kind/pattern
SORT rocky_recall ASC
```

Replace `pattern` with `concept` or `implementation`.

### Topics from a specific project

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_domain AS "Domain"
FROM #rocky/node AND #rocky/repo/taskify
SORT rocky_recall ASC
```

Replace `taskify` with any repo tag. Note: a topic that's been encountered in multiple projects only carries the *primary* repo as `rocky_repo`; the cross-project view in `rocky view` is authoritative.

### Everything sorted by recall

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_domain AS "Domain",
      rocky_days_since_review AS "Days ago"
FROM #rocky/node
SORT rocky_recall ASC
```

---

## Graph view

Because each topic is a regular Obsidian note, you can use Obsidian's graph view to visualise your knowledge. Topics in the same domain are grouped together. The `See also:` wikilinks create edges between related topics.

Filter the graph to `#rocky/node` to see only your PKG. Filter to `#rocky/domain/Auth` to zoom into a specific area.
