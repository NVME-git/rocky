# 01 · Topic Lifecycle — State Machine

**Source files:** `src/main.rs` · `src/db.rs` · `src/fsrs.rs`

```mermaid
stateDiagram-v2
    direction TB

    [*] --> UNDISCOVERED

    state "NOT IN PKG" as not_in_pkg {
        UNDISCOVERED : UNDISCOVERED \n never seen by Rocky
        QUEUED : QUEUED\nstored in .rocky (local SQLite)
    }

    state "IN PKG" as in_pkg {
        GAP : GAP\nretrievability < 0.7
        FADING : FADING\n0.7 ≤ R < 0.9
        KNOWN : KNOWN\nR ≥ 0.9
    }

    state "TERMINAL" as terminal {
        IGNORED : IGNORED\nnot in PKG, dismissed
        DELETED : DELETED\nrocky delete
    }

    UNDISCOVERED --> QUEUED        : rocky diff / backfill\nrocky quiz (from logs)\nrocky "task"
    QUEUED --> GAP                 : Q&A, score < 0.65
    QUEUED --> KNOWN               : 'e' too easy (score = 0.75)
    QUEUED --> IGNORED             : 'i' ignore
    QUEUED --> QUEUED              : Enter (skip — stays queued)
    GAP --> FADING                 : quiz, partial score 0.3–0.65
    GAP --> KNOWN                  : quiz, score ≥ 0.65
    GAP --> EXPLAINED              : '?' explain during quiz
    EXPLAINED --> FADING           : score = 0.2, added to PKG
    FADING --> KNOWN               : quiz, score ≥ 0.65
    KNOWN --> FADING               : time decay (any stability)
    FADING --> GAP                 : time decay (low stability)
    GAP --> DELETED                : rocky delete
    FADING --> DELETED             : rocky delete
    KNOWN --> DELETED              : rocky delete
    IGNORED --> [*]
    DELETED --> [*]
```

---

## State descriptions

| State | Retrievability | What Rocky does |
|---|---|---|
| UNDISCOVERED | — | Never appeared in any diff, prompt, or task |
| QUEUED | — | Seen, stored in `.rocky`, awaiting first Q&A |
| GAP | R < 0.7 | Full Socratic Q&A — cross-concept or canonical or live |
| FADING | 0.7–0.9 | 2–3 sentence reminder, small stability bump |
| KNOWN | R ≥ 0.9 | Mark encountered silently, no quiz |
| EXPLAINED | — | Transient: user typed `?`, got explanation, low confidence |
| IGNORED | — | User typed `i` — dismissed, not tracked |
| DELETED | — | `rocky delete` — removed from PKG entirely |

---

## Decay: how KNOWN becomes GAP over time

Decay is passive — no user action required. Governed by:

```
R = (1 + t / (9 × S)) ^ -1
```

- `t` = days since `last_reviewed`
- `S` = stability (increases with good answers)

A topic with `S = 2` (new) drops below the KNOWN threshold after ~2 days.
A topic with `S = 10` (reviewed several times) stays KNOWN for ~11 days.

---

## 📝 Annotation space

> Add your notes here. Questions to consider:
>
> - Are there missing states? (e.g. a `CO_AUTHORED` or `AI_ASSISTED` sub-state?)
> - Should `task_prompt` topics enter QUEUED or go straight to Q&A?
> - Should the IGNORED state be permanent, or should it expire after N days?
> - What should happen when a KNOWN topic appears in a new diff?
