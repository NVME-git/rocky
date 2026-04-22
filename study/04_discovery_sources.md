<style>
body {
    font-family: 'Roboto', sans-serif;
}
code, pre {
    font-family: 'Fira Code', 'Courier New', monospace;
}
</style>

# 04 · Topic Discovery — How Topics Enter the PKG

**Source files:** `src/main.rs` — `run_diff()`, `run_backfill()`, `run_session_end()`, `run_explore()`, `run_task()`

> **As of `feat/rich-context-enrichment`** the primary hook-based path changed from
> `rocky diff` (per-commit, live LLM) to a **queue + batch** model:
> `rocky post-commit` (queue only) → `rocky session-end` (batch with transcript).

```mermaid
flowchart LR
    subgraph SOURCES["Entry sources"]
        direction TB
        SESSION_END["rocky session-end\n(Stop hook)\n─────────────────\nSource: queued commits\n+ Claude Code transcript\nTriggered: session close\nor manual"]
        EXPLORE["rocky explore\n─────────────────\nSource: git log history\n+ README/Cargo.toml\nTriggered: once or --force"]
        BACKFILL["rocky backfill\n─────────────────\nSource: git log history\nTriggered: manual only"]
        TASK["rocky \"task desc\"\n─────────────────\nSource: user text\nTriggered: pre-work"]
        DIFF["rocky diff\n(legacy / manual)\n─────────────────\nSource: git diff SHA\nTriggered: manual only\n(hook was replaced by\npost-commit + session-end)"]
    end

    subgraph QUEUE[".rocky commit queue\n(per-project SQLite)"]
        direction TB
        COMMIT_SHA["queued_sha\ntimestamp"]
    end

    subgraph PKG["PKG  (~/.rocky/graph.db)"]
        direction TB
        NODE["Node added\nto nodes table"]
        CTX["Context entry\nto contexts table"]
        BANK["question_bank\n(4 Q+A+clue triples\ngenerated at session-end)"]
        EDGES_GEN["Edges generated\nbetween new topics\n+ domain skeleton edges"]
        PROJ_CTX["project_context\n(explore stores\nproject summary)"]
    end

    GIT_HOOK["git post-commit\nhook"] -->|"rocky post-commit\nqueue SHA only\nno LLM"| COMMIT_SHA
    SESSION_END -->|"drain queue\ngit diff each SHA\n+ read transcript"| NODE
    EXPLORE     -->|"new project context\n+ taxonomy topics"| NODE
    EXPLORE     -->|"project summary"| PROJ_CTX
    BACKFILL    -->|"new topics\ncreated_at = COMMIT DATE"| NODE
    TASK        -->|"topics after\nQ&A passes"| NODE
    DIFF        -->|"new topics\ncreated_at = today"| NODE

    NODE --> CTX
    NODE --> BANK
    NODE --> EDGES_GEN

    subgraph TRANSCRIPT["Claude Code transcript\n(~/.claude/projects/<repo>/*.jsonl)"]
        JSONL["JSONL log\nper-session\nread by rocky session-end\n--hours N (default 6)"]
    end

    SESSION_END -->|"read last N hours"| JSONL
```

---

## Per-source field values at insertion

| Field | `rocky session-end` | `rocky explore` | `rocky backfill` | `rocky "task"` |
|---|---|---|---|---|
| `created_at` | today | today | **commit date** | today |
| `last_reviewed` | today | today | **commit date** | today (after Q&A) |
| `stability` | 2.0 | 2.0 | 2.0 | depends on Q&A score |
| `question_bank` | ✅ 4-item bank generated | ✅ 4-item bank generated | ✅ generated | ❌ empty |
| `canonical_question` | set (legacy compat) | set (legacy compat) | set (legacy compat) | ❌ empty |
| `repo` | from git remote | from git remote | from git remote | ❌ empty |
| `review_count` | 0 | 0 | 0 | 1 (from Q&A) |
| Context richness | commit msg + transcript | README + git log | commit message | user text only |
| Co-Authored-By detectable? | ✅ yes (queued commits) | ❌ no | ✅ yes | ❌ no |

---

## Hook installation modes

Rocky supports two hook modes selected at `rocky hook install` time:

| Mode | git hook command | Enrichment trigger |
|---|---|---|
| **Queue mode** (default) | `rocky post-commit` | `rocky session-end` (Stop hook) |
| **Legacy mode** | `rocky diff` | Immediate, per commit |

Queue mode is preferred when using Claude Code — commits are batched and enriched
with the session transcript at session end, giving richer context per topic.

The Stop hook is a Claude Code `stop` hook registered in `~/.claude/settings.json`:
```json
{ "hooks": { "Stop": [{ "matcher": "", "command": "rocky session-end --quiet" }] } }
```

---

## rocky explore — project context

`rocky explore` reads the project's git log, README, and manifest files to build a
"project context" summary stored in the DB. This context is:
- Injected into every subsequent extraction prompt (so topics know which repo they came from)
- Viewable with `rocky explore --show`
- Refreshed on demand with `rocky explore --force`

Run it once when setting up Rocky on a new project.

---

## Layer-1 dedup

Before each LLM extraction call, Rocky passes the existing topic list into the prompt:
```
Existing topics (do not re-extract): rust lifetime, borrow checker, ...
```
The LLM is instructed to skip topics that are semantically equivalent to existing ones.
This prevents duplicates from accumulating across sessions.

---

## 📝 Annotation space

> Add your notes here. Questions to consider:
>
> - For `rocky backfill`, should Co-Authored-By be parsed from `git log --format=%B`?
> - Should topics from task/explore enter at lower initial stability since they lack quiz history?
> - Should `repo` be populated for task-sourced topics using the current `git remote`?
> - Should `rocky explore` re-run automatically if the project context is older than N days?
> - Is the layer-1 dedup prompt injection approach reliable enough, or do we need cosine similarity?
