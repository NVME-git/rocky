# Obsidian Integration

Rocky can write your entire knowledge graph into an [Obsidian](https://obsidian.md) vault as markdown files. Each topic becomes a note with metadata that Obsidian's Dataview plugin can query and visualise.

## Setup

### 1. Tell Rocky where your vault is

In `~/.rocky/.rocky.toml`:

```toml
[export]
obsidian_vault = "~/Documents/Obsidian/MyVault/rocky"
```

Replace the path with the actual location of your Obsidian vault. Rocky will create the `rocky/` subfolder inside it.

### 2. Export your topics

```bash
rocky export
```

This writes one `.md` file per topic into your vault. Rocky also does this automatically every time a topic is updated, so your vault stays in sync.

### 3. Install the Dataview plugin in Obsidian

1. Open Obsidian → Settings → Community plugins
2. Search for "Dataview" and install it
3. Enable it

---

## What the notes look like

Each topic becomes a file like `jwt-authentication.md`:

```markdown
---
rocky_id: jwt-authentication
rocky_kind: pattern
rocky_difficulty: 0.3
rocky_stability: 8.5
rocky_retrievability: 0.94
rocky_last_reviewed: 2026-04-03
rocky_last_encountered: 2026-04-03
rocky_review_count: 3
rocky_days_since_review: 0
tags: [rocky/node, rocky/kind/pattern]
---

# JWT authentication

Stateless token-based auth where the server signs a payload the client stores and sends back.

## Contexts
- add user login with JWT tokens to my Express API
```

---

## Dataview queries

Once your notes are in Obsidian, you can create dashboards using Dataview queries.

### Topics that need attention

Paste this into any Obsidian note (in a code block marked `dataview`):

```
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed", rocky_kind AS "Kind"
FROM #rocky/node
WHERE rocky_retrievability < 0.7
SORT rocky_retrievability ASC
```

This shows everything that's fading or weak, sorted by most urgent.

### Your strongest topics

```
TABLE rocky_retrievability AS "Recall %", rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability >= 0.9
SORT rocky_review_count DESC
```

### Topics by type

```
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/kind/implementation
SORT rocky_retrievability ASC
```

Replace `implementation` with `concept` or `pattern` to filter by kind.

### Everything, sorted by recall

```
TABLE rocky_retrievability AS "Recall", rocky_kind AS "Kind", rocky_days_since_review AS "Days ago"
FROM #rocky/node
SORT rocky_retrievability ASC
```

---

## Graph view

Because each topic is a regular Obsidian note, you can use Obsidian's graph view to see all your Rocky topics alongside your other notes. Topics share tags (`#rocky/node`, `#rocky/kind/pattern`, etc.) so you can filter the graph to show only your knowledge graph.
