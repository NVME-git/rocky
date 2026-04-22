<style>
body {
    font-family: 'Roboto', sans-serif;
}
code, pre {
    font-family: 'Fira Code', 'Courier New', monospace;
}
</style>

# 05 · Memory Model — FSRS (Free Spaced Repetition Scheduler)

**Source files:** `src/fsrs.rs` · `src/db.rs` — `add_or_update()`

---

## Core formula

```
R  =  ( 1  +  t / (9 × S) ) ^ -1

  R  =  retrievability  (0.0 to 1.0)
  t  =  days since last_reviewed
  S  =  stability  (stored on node, increases with good answers)
```

```mermaid
flowchart LR
    subgraph INPUTS["Inputs"]
        T["t\ndays since\nlast_reviewed"]
        S["S\nstability\nstored on node"]
    end

    subgraph FORMULA["R = (1 + t÷(9×S))^-1"]
        COMPUTE["compute R"]
    end

    subgraph CLASSIFY["classify(R)  →  src/fsrs.rs"]
        KNOWN["R ≥ 0.9\nKNOWN\nskip silently"]
        FADING["0.7 ≤ R < 0.9\nFADING\nprint reminder"]
        GAP["R < 0.7\nGAP\nfull Q&A"]
    end

    T --> COMPUTE
    S --> COMPUTE
    COMPUTE -->|R| CLASSIFY
    COMPUTE --> KNOWN
    COMPUTE --> FADING
    COMPUTE --> GAP
```

---

## Decay examples — days until R = 0.5

| Stability | Interpretation | Days until R = 0.5 | Days until KNOWN threshold (R=0.9) broken |
|---|---|---|---|
| S = 2.0 | New topic, first insertion | **18 days** | ~2.2 days |
| S = 5.0 | Reviewed once, decent answer | **45 days** | ~5.6 days |
| S = 10.0 | Strong repeated understanding | **90 days** | ~11 days |
| S = 20.0 | Deep, well-reviewed topic | **180 days** | ~22 days |
| S = 999 | Domain skeleton node | Never (R ≈ 1.0) | Never |

---

## How stability is updated

`src/db.rs — add_or_update()` — called after every quiz or encounter.

```mermaid
flowchart TD
    SCORE["score  0.0–1.0\npassed from quiz outcome"]

    SCORE --> FIRST_TIME{First time\nseeing this topic?}

    FIRST_TIME -- YES --> INSERT["INSERT node\nstability = 2.0\ndifficulty = 0.3\nreview_count = 0"]

    FIRST_TIME -- NO --> UPDATE["UPDATE node\nnew_stability = old_s × (1 + score × 0.9)\nnew_difficulty = old_d × 0.9 + score × 0.1\nreview_count += 1\nlast_reviewed = today"]

    INSERT --> DONE["Node in DB"]
    UPDATE --> DONE
```

---

## Score values by action

| User action | Score | Effect on stability |
|---|---|---|
| `e` too easy | 0.75 | Moderate increase |
| `?` explain (got stuck) | 0.2 | Minimal increase — resurfaces soon |
| Understood, first attempt | eval result (0.65–1.0) | Good–strong increase |
| Understood after follow-up | eval result (0.65–1.0) | Same — but took more attempts |
| Exhausted (avg of attempts) | ~0.2–0.4 | Small increase — resurfaces soon |
| Stale reminder (no Q&A) | 0.5 | Small bump — marks encounter |

---

## Initial stability by source

```mermaid
flowchart LR
    DIFF["rocky diff\nbackfill"] -->|stability = 2.0| PKG["PKG node"]
    TASK["rocky task\n(Q&A passed)"] -->|stability = fn(score)| PKG
    QUIZ["rocky quiz\n(Q&A passed)"] -->|stability = fn(score)| PKG
    DOMAIN["Domain skeleton\nensure_taxonomy_skeleton"] -->|stability = 999| PKG
```

---

## Proposed change: co_authored lowers initial stability

```
IF co_authored = true:
  initial stability = 1.5  (instead of 2.0)
  initial difficulty = 0.4  (instead of 0.3)

Rationale:
  AI-generated code = weaker assumption the dev understood it at commit time.
  Lower stability → topic resurfaces sooner → earlier verification.
```

---

## Rocky IQ

Rocky IQ is the inverse-atrophy score displayed in the Dashboard. It answers:
**"What fraction of your recently-learned knowledge are you still retaining?"**

```
atrophy_score = weighted_avg( max(0, 0.7 - R(node)) / 0.7 )
                weighted by recency: w = max(0, 1 - days_since_created / 60)

Rocky IQ = round( (1 - atrophy_score) × 100 )
```

- Nodes older than 60 days contribute zero weight (60-day sliding window)
- Only non-domain nodes are included
- A score of 100 = all recent topics fully retained (R ≥ 0.7)
- A score below 70 = significant atrophy — Rocky IQ shown in red in the Dashboard

```mermaid
flowchart LR
    subgraph WINDOW["60-day recency window"]
        direction LR
        NEW["created < 60 days ago\nw = 1 - (days/60)\n0 < w ≤ 1"]
        OLD["created ≥ 60 days ago\nw = 0\nexcluded"]
    end

    NEW -->|weighted decay gap| ATROPHY["atrophy_score\n0.0 → 1.0"]
    ATROPHY --> IQ["Rocky IQ\n= (1 - atrophy) × 100\n0 → 100"]

    IQ -->|"≥ 80"| GREEN["green — strong retention"]
    IQ -->|"70–79"| YELLOW["yellow — some decay"]
    IQ -->|"< 70"| RED["red — review needed"]
```

---

## Domain health breakdown

The Dashboard also shows per-domain health bars using `domain_health_breakdown()`:

| Field | How computed |
|---|---|
| `avg_recall` | mean R across all nodes in the domain |
| `known` | count with R ≥ 0.9 |
| `fading` | count with 0.7 ≤ R < 0.9 |
| `gap` | count with R < 0.7 |

Domains are sorted ascending by `avg_recall` (worst domains first) to draw attention
to the weakest areas.

---

## 📝 Annotation space

> Add your notes here. Questions to consider:
>
> - Is `initial_stability = 1.5` for `co_authored` the right value? Too aggressive? Not enough?
> - Should the stability update formula change for scaffolded passes?
>   (e.g. `score × 0.7` instead of `score × 0.9` if clue was used)
> - Should difficulty decay differently for co_authored topics?
> - Is there a case for storing the full retrievability curve, not just current stability?
> - Is the 60-day half-window for Rocky IQ the right recency cutoff?
> - Should Rocky IQ be factored across domains separately and then averaged?
