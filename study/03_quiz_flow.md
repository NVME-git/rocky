# 03 · Quiz Session — Decision Tree

**Source files:** `src/main.rs` — `run_quiz_topic()` ~line 1130

```mermaid
flowchart TD
    START([Topic selected for quiz])

    START --> CC_CHECK

    CC_CHECK{Cross-concept edge?\npeer node with matching edge\nboth recall > 0.8?}

    CC_CHECK -- YES\nfire edge --> CC_Q[generate_cross_concept_question\n🤖 generated — live\nuses edge description + both topics]
    CC_CHECK -- NO edge qualifies --> CAN_CHECK

    CAN_CHECK{canonical_question\nnon-empty?}
    CAN_CHECK -- YES --> CAN_Q[use canonical_question\n✅ canonical — from diff\nloaded from DB]
    CAN_CHECK -- NO --> GEN_Q[generate_question\n🤖 generated — live\nfrom description + task context]

    CC_Q  --> DISPLAY
    CAN_Q --> DISPLAY
    GEN_Q --> DISPLAY

    DISPLAY[Display question\n+ label: canonical or generated\nLoad: canonical_answer, canonical_clue]

    DISPLAY --> OPTS

    OPTS{User input?}

    OPTS -- Enter blank --> SKIP[Queue topic in .rocky\nexit session]
    OPTS -- i --> IGNORE[Ignored\nnot added to PKG\nexit]
    OPTS -- e --> EASY[score = 0.75\nadd to PKG as known\nexit]
    OPTS -- s --> SIMPLER[generate_question\nDifficulty.Simpler\nloop back to DISPLAY]
    OPTS -- h --> HARDER[generate_question\nDifficulty.Harder\nloop back to DISPLAY]
    OPTS -- c --> CLUE{canonical_clue\nexists?}
    CLUE -- YES --> SHOW_CLUE[show canonical_clue\nno API call\nloop back to DISPLAY]
    CLUE -- NO --> GEN_CLUE[generate_clue\n🤖 live API call\nloop back to DISPLAY]
    OPTS -- ? --> EXPLAIN_NOW[generate_explanation\nscore = 0.2\nadd to PKG\nexit]
    OPTS -- typed answer --> EVAL

    EVAL[evaluate_answer\ntopic, question, answer\ndescription, canonical_answer]

    EVAL --> SCORE_CHECK

    SCORE_CHECK{score ≥ 0.65?\nunderstood = true?}
    SCORE_CHECK -- YES --> ADD_KNOWN[add to PKG\nhigh confidence\nexit]
    SCORE_CHECK -- NO --> FOLLOWUP_CHECK

    FOLLOWUP_CHECK{has followup question?\nquestions_asked < 3?}
    FOLLOWUP_CHECK -- YES --> FOLLOWUP[set question = followup\nloop back to DISPLAY]
    FOLLOWUP_CHECK -- NO --> EXHAUST[generate_explanation\nscore = avg of attempts\nadd to PKG low confidence\nexit]

    classDef known    fill:#064e3b,stroke:#10b981,color:#d1fae5
    classDef fading   fill:#451a03,stroke:#f59e0b,color:#fef3c7
    classDef gap      fill:#450a0a,stroke:#ef4444,color:#fee2e2
    classDef ai       fill:#0c4a6e,stroke:#06b6d4,color:#e0f2fe
    classDef terminal fill:#1e1b4b,stroke:#6366f1,color:#e0e7ff
    classDef proc     fill:#0f172a,stroke:#334155,color:#cbd5e1

    class ADD_KNOWN,EASY,CAN_Q known
    class FOLLOWUP,FOLLOWUP_CHECK fading
    class EXHAUST,EXPLAIN_NOW gap
    class CC_Q,GEN_Q,GEN_CLUE,SIMPLER,HARDER ai
    class SKIP,IGNORE terminal
    class EVAL,DISPLAY,OPTS,SCORE_CHECK,CC_CHECK,CAN_CHECK,CLUE proc
```

---

## Question source types

| Label shown | How generated | API call at quiz time? |
|---|---|---|
| `(canonical)` | At backfill — uses full diff + commit + README | No — read from DB |
| `(generated)` | Live — uses topic description + task context | Yes |
| Cross-concept | Live — uses edge description + both node descriptions | Yes |

---

## Scoring reference

| Action | Score passed to `add_or_update` |
|---|---|
| `e` too easy | 0.75 |
| `?` explain | 0.2 |
| Understood on first/second attempt | `eval_result.score` (0.65–1.0) |
| Exhausted (avg of attempts) | typically 0.2–0.5 |
| Stale reminder (no quiz) | 0.5 |

---

## MAX_QUESTIONS

The follow-up loop runs at most **3 times** (`MAX_QUESTIONS = 3` in `src/main.rs`).
After 3 attempts without `understood = true`, Rocky gives the full explanation.

---

## 📝 Annotation space

> Add your notes here. Questions to consider:
>
> - Where exactly should struggle score be tracked? (each loop iteration? each option used?)
> - Should `[s]` simpler reset the question counter, or count as an attempt?
> - Should clue usage affect the final score passed to `add_or_update`?
> - Should there be a `[k]` mark-as-known shortcut distinct from `[e]` too easy?
> - Is MAX_QUESTIONS = 3 right, or should it be configurable?
