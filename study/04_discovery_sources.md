# 04 · Topic Discovery — How Topics Enter the PKG

**Source files:** `src/main.rs` — `run_diff()`, `run_backfill()`, `run_quiz()`, `run_task()`

```mermaid
flowchart LR
    subgraph SOURCES["Entry sources"]
        direction TB
        DIFF["rocky diff\nor git post-commit hook\n─────────────────\nSource: git diff\nTriggered: after commit\nor rocky diff SHA"]
        BACKFILL["rocky backfill\n─────────────────\nSource: git log history\nTriggered: manual only"]
        TASK["rocky \"task desc\"\n─────────────────\nSource: user text\nTriggered: pre-work"]
        QUIZ["rocky quiz\n─────────────────\nSource: .rocky prompt log\nTriggered: manual"]
    end

    subgraph PKG["PKG  (~/.rocky/graph.db)"]
        direction TB
        NODE["Node added\nto nodes table"]
        CTX["Context entry\nto contexts table"]
        QA["canonical_question\ncanonical_answer\ncanonical_clue"]
        EDGES_GEN["Edges generated\nbetween new topics\n+ domain skeleton edges"]
    end

    DIFF     -->|"new topics\ninitial_r = 0.5\ncreated_at = today"| NODE
    BACKFILL -->|"new topics\ninitial_r = 0.5\ncreated_at = COMMIT DATE"| NODE
    TASK     -->|"topics after\nQ&A passes"| NODE
    QUIZ     -->|"topics after\nQ&A passes"| NODE

    NODE --> CTX
    NODE --> QA
    NODE --> EDGES_GEN

    subgraph ROCKY_FILE[".rocky  (per-project SQLite)"]
        PROMPT_LOG["prompt_text\ntimestamp\nsession_id\n─────────────\nauto-deleted after 24h"]
    end

    CLAUDE_HOOK["Claude Code\nprompt submitted"] --> PROMPT_LOG
    PROMPT_LOG -->|"rocky quiz\nreads last 24h"| QUIZ
```

---

## Per-source field values at insertion

| Field | `rocky diff` | `rocky backfill` | `rocky "task"` | `rocky quiz` (logs) |
|---|---|---|---|---|
| `created_at` | today | **commit date** | today | today |
| `last_reviewed` | today | **commit date** | today (after Q&A) | today (after Q&A) |
| `stability` | 2.0 | 2.0 | depends on Q&A score | depends on Q&A score |
| `canonical_question` | generated at insert | generated at insert | ❌ empty | ❌ empty |
| `canonical_answer` | generated at insert | generated at insert | ❌ empty | ❌ empty |
| `canonical_clue` | generated at insert | generated at insert | ❌ empty | ❌ empty |
| `repo` | from git remote | from git remote | ❌ empty | ❌ empty |
| `review_count` | 0 | 0 | 1 (from Q&A) | 1 (from Q&A) |
| Co-Authored-By detectable? | ✅ yes | ✅ yes (in commit body) | ❌ no | ❌ no |

---

## Session limits (auto-triggered only)

`rocky diff` via git hook respects these limits. Manual calls bypass them.

| Setting | Default | Effect |
|---|---|---|
| `daily_budget` | 3 | Max auto-triggered quizzes per day |
| `min_gap_minutes` | 120 | Min gap between auto-triggered sessions |

When budget/cooldown blocks a session, topics are queued in `.rocky` instead.

---

## The .rocky prompt log

- Location: `./.rocky` (SQLite, per project directory)
- Written by: `rocky hook` (Claude Code `UserPromptSubmit` hook)
- Read by: `rocky quiz` (scans last 24h by default, `--hours N` to extend)
- Entries older than 24h are auto-deleted
- Each entry: `prompt_text`, `timestamp`, `session_id`

---

## 📝 Annotation space

> Add your notes here. Questions to consider:
>
> - For `rocky backfill`, should Co-Authored-By be parsed from `git log --format=%B`?
> - For `rocky "task"`, should `source = 'task_prompt'` or `source = 'ai_prompt'`?
>   The user described intent — they haven't asked AI to build it yet.
> - Should topics from `rocky quiz` (logs) enter at lower initial stability
>   since they're pure `ai_prompt` origin?
> - Should `repo` be populated for task/quiz-sourced topics using the current `git remote`?
