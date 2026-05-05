# 0008 — Agent-side generic question generation + canonical Q&A persistence

Date: 2026-05-05 · Status: Accepted
Related: [0002 — Layer-1 dedup + question bank](0002-dedup-and-question-bank.md), [0007 — Skill-only extraction](0007-skill-only-extraction-and-promptiq.md)

## Context

Two related gaps surfaced together while testing post-commit extraction.

**Gap 1 — empty canonical_question on post-commit topics.** Topics created
through the `rocky diff` flow (the post-commit hook for non-skill users) and
through `run_socratic_loop` were inserted with empty `canonical_question`,
`canonical_answer`, and `canonical_clue` fields. Backfill (`rocky backfill`)
correctly pre-generated these via `Teacher::generate_question_and_answer` and
saved with `db.set_canonical_qa`; the diff/socratic path never did.

The visible symptom: clicking *quiz me* on such a topic in the web UI fell
through to the fallback branch in `server.rs:259-265` —

```rust
let question = if !node.canonical_question.is_empty() {
    node.canonical_question.clone()
} else if !node.description.is_empty() {
    format!("Explain: {} — {}", node.topic, node.description)
} else {
    format!("What do you know about {}?", node.topic)
};
```

— so the user saw `Explain: <topic> — <full description>`, with the answer
already inside the question.

**Gap 2 — questions leaked repo-specific identifiers.** The
`Teacher::generate_question_and_answer` prompt is grounded in the diff and
explicitly tells the model to "ask about what breaks, changes, or becomes
constrained when using this approach in their *specific code*". That's the
right framing for an immediate post-commit quiz where context is fresh, but
it produces questions that read poorly when the same topic resurfaces a year
later in a different project. Function names, parameter names, file paths,
and project terminology bake into the question text. The user reading
*"What constraint does `bobInner()` impose on `warp-node`'s transform
attribute?"* on a generic spaced-repetition card has lost the underlying
concept under a wall of identifiers.

A separate observation strengthened the case: the `/rocky-checkpoint` skill
already runs *inside the agent* (Claude Code or OpenCode) with full project
context — far richer than a single diff excerpt. Asking it to also generate
questions, with explicit redaction rules, is strictly more capable than
asking the Rust-side `Teacher` to do the same job. Per [0007](0007-skill-only-extraction-and-promptiq.md),
extraction has already moved into the skill; the question-generation path
is the natural next step for the same architectural reason.

## Decision

**Persist canonical Q+A+clue for every post-commit topic.** `run_socratic_loop`
now accepts a `pregenerated_qa: Option<&(String, String, String)>` parameter.
`run_diff` calls `Teacher::generate_question_and_answer` once per new topic
before invoking the loop, and the loop persists the triple via
`db.set_canonical_qa` from each `add_or_update` site (success / explain /
exhaust). Persistence is *non-destructive* — if a node already has a
canonical question, the existing value is preserved (manual curation wins
over auto-generation). The pre-generated question doubles as the loop's
first quiz prompt, so we don't pay for two LLM calls.

The previous skip-on-quiz contract is unchanged: an empty answer still calls
`queue_for_later` and returns *without* inserting a node. The new
persistence only fires on the same code paths that already inserted nodes.

**Move generic question generation into the skill layer.** The
`/rocky-checkpoint` skill now generates 1–3 generic questions per topic and
stores them via `rocky add-question`. The skill prompt enumerates explicit
redaction rules:

- Strip function/method/class/struct/variable names from the diff
  (`bobInner()` → "the wrapper function").
- Strip file paths and module names.
- Strip parameter and field names that are project-local
  (`pregenerated_qa` → "a pre-generated question/answer pair").
- Strip project-specific terminology.
- Keep library/framework names *only* when the topic is genuinely about
  that library.

Questions must test implications, trade-offs, or consequences — not recall
of definitions — and each item in a multi-question bank for one topic must
hit a different angle. The agent has the full diff plus access to read any
file in the repo while applying these rules, which the Rust-side
`Teacher::generate_question_bank` (a single LLM call with a fixed prompt
and a 2500-char diff excerpt) cannot match.

**Add `/rocky-backfill` for agent-driven historical seeding.** A new
`rocky checkpoint history --limit N [--all-authors]` CLI subcommand emits
recent commits in the same JSON envelope shape as `rocky checkpoint diff`
(`commits[]` with `sha`, `subject`, `message`, `diff`). The new
`/rocky-backfill` skill consumes that envelope and runs the same
extract-and-question loop as `/rocky-checkpoint`, with two differences:
the source is arbitrary git history rather than the post-commit queue,
and there is no `rocky checkpoint mark` step (history is read-only).

`rocky backfill` (the headless Rust command) keeps working unchanged. We
have, in effect, two backfill paths now: a fast headless one that pays
per-commit Anthropic-or-Ollama tokens, and a slower agent-driven one that
produces strictly better questions and reuses the agent the user is
already paying for. Recommended workflow: headless `rocky backfill` for
bulk first-pass, `/rocky-backfill` for recent commits where question
quality matters most.

## Consequences

**Good**
- "Quiz me" on a post-commit topic surfaces a real Socratic question
  instead of `Explain: <topic> — <description>`.
- Questions stored from the skill layer don't degrade as the user moves
  between projects — the same topic surfacing in a new repo asks the same
  generic question.
- One mental model for question generation: the skill, holding all the
  context, makes the call. The Rust-side `Teacher::generate_question_bank`
  remains as a fallback for the headless `rocky backfill --fill-question-bank`
  path and for users without an agent.
- `/rocky-backfill` reuses the existing checkpoint envelope, so the skill
  body is short and the CLI surface only grew by one subcommand.
- Manual curation is preserved: `set_canonical_qa` from the loop is
  guarded by an "already-set" check, so users editing canonical Q&A
  through future tools won't have it overwritten by the next post-commit
  run.

**Bad**
- Two question-generation pipelines live in the codebase: agent-side
  (skill) and server-side (`Teacher::generate_question_and_answer` and
  `Teacher::generate_question_bank`). The server-side one is now the
  fallback, not the primary path; we keep it because not every user runs
  through an agent. Code surface stays slightly larger than a single-path
  design.
- Skill output quality depends on the agent following the redaction rules.
  Models that aren't careful will still leak identifiers. Mitigation: the
  rules are listed verbatim in `SKILL.md` with examples; model regressions
  surface as user-visible question quality, which the user can fix with
  `rocky add-question`.
- Agent-driven backfill is slow and token-expensive compared to the
  headless path. Documented as an explicit trade-off; the user picks.

**Migration**
- Existing topics with empty `canonical_question` and `question_bank`:
  run `rocky backfill --fill-question-bank` for headless generation, or
  `/rocky-backfill --limit <N>` for agent-driven generic generation.
- No schema changes. `set_canonical_qa` and `set_question_bank` were
  already in the DB layer.

**Reversed by**
- An ADR introducing a single question-generation pipeline (likely
  agent-only, deprecating the Rust-side fallback). Would require justifying
  the loss of agent-free operation.
- An ADR replacing `rocky checkpoint history` with a more general history
  API that also serves rocky-quiz / rocky-explore.
