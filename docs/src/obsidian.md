# Obsidian Integration

Rocky writes your entire knowledge graph as Markdown files into a PKG directory. Each topic becomes a note with metadata that Obsidian's Dataview plugin can query and visualise.

## Setup

### 1. Tell Rocky where your PKG directory is

In `~/.rocky/.rocky.toml`:

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
rocky_difficulty: 0.3
rocky_stability: 8.5
rocky_retrievability: 0.94
rocky_last_reviewed: 2026-04-03
rocky_last_encountered: 2026-04-03
rocky_review_count: 3
rocky_days_since_review: 0
tags: [rocky/node, rocky/kind/pattern, rocky/domain/Auth]
---

# JWT authentication

Stateless token-based auth where the server signs a payload the client stores and sends back.

## Contexts
- add user login with JWT tokens to my Express API

## See also
- [[token-expiry-handling]]
- [[httponly-cookie-security]]
```

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

```dataview
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed", rocky_kind AS "Kind"
FROM #rocky/node
WHERE rocky_retrievability < 0.7
SORT rocky_retrievability ASC
```

### Your strongest topics

```dataview
TABLE rocky_retrievability AS "Recall %", rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability >= 0.9
SORT rocky_review_count DESC
```

### Topics by domain

```dataview
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/domain/Auth
SORT rocky_retrievability ASC
```

Replace `Auth` with any domain name.

### Topics by kind

```dataview
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/kind/pattern
SORT rocky_retrievability ASC
```

Replace `pattern` with `concept` or `implementation`.

### Everything sorted by recall

```dataview
TABLE rocky_retrievability AS "Recall", rocky_domain AS "Domain", rocky_days_since_review AS "Days ago"
FROM #rocky/node
SORT rocky_retrievability ASC
```

---

## Graph view

Because each topic is a regular Obsidian note, you can use Obsidian's graph view to visualise your knowledge. Topics in the same domain are grouped together. The `See also:` wikilinks create edges between related topics.

Filter the graph to `#rocky/node` to see only your PKG. Filter to `#rocky/domain/Auth` to zoom into a specific area.
