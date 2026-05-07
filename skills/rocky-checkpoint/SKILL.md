---
name: rocky-checkpoint
description: >
  Extract atomic learning topics from recent commits and add them to the user's
  Rocky personal knowledge graph (PKG). Run after the user (or you) makes one
  or more commits in a project that has Rocky installed. Trigger phrases:
  "/rocky-checkpoint", "rocky checkpoint", "extract topics from these commits",
  "add to my PKG".
---

You are Rocky's topic extractor. Rocky is a personal knowledge graph that
spaces-repeats whatever the user is learning. After commits land, *you* — the
agent already loaded with full context of what changed and why — extract 1–3
atomic topics per commit and store them via the `rocky` CLI. No separate LLM
call. No questions asked of the user mid-flow.

## The PKG is global, the context is per-project

Rocky stores all topics in a single graph (`~/.rocky/graph.db`). When you call
`rocky list --json` you see *every* topic from *every* project the user has
ever checkpointed — that is intentional. A "Connection Pool Sizing" learned
in project A should merge with the same insight reappearing in project B,
not duplicate. The merged topic accumulates `repos[]` so the user can see
which projects each idea spans.

The *generation* context, by contrast, is per-project: `rocky context` and
`rocky checkpoint diff` only return data for the current project. Use that
context to *name* and *describe* topics in this codebase's vocabulary, but
*dedup* against the global list.

## Procedure

Run these steps in order. Stop and report if any step fails.

1. **Read the queue** (this project only):
   ```bash
   rocky checkpoint diff
   ```
   JSON with `pending_count` and `commits[]` (`sha`, `subject`, `message`,
   `diff`). An empty queue is no longer a stop condition — proceed to the
   conversation pass in step 4. Only stop early if *both* the queue is empty
   AND the conversation pass yields no candidates (reported at step 8).

2. **Read the global dedup list** (across all projects):
   ```bash
   rocky list --json
   ```
   Every existing topic. Look for matches by *meaning*, not just name. A
   topic in another project on the same idea should be merged, not duplicated.

3. **Read this project's context**:
   ```bash
   rocky context
   ```
   Use `summary` to ground topic naming in this codebase's vocabulary.
   `null` summary → suggest `rocky explore` first, but continue.

4. **Extract topics — two passes.**

   **Pass A — commit pass** (skip if the queue was empty). For each commit:
   - Identify 1–3 atomic learnings — one specific concept, pattern, or
     implementation detail per topic. Not "URL stuff". Not the whole commit.
   - Skip pure plumbing (formatting, version bumps, trivial renames). Empty
     commits and merge commits produce zero topics.
   - **Cross-project dedup**: if a near-match exists anywhere in the global
     list (same concept under any name), call `rocky add-topic` with the
     **existing topic's name** to merge instead of creating a duplicate.

   **Pass B — conversation reflection.** Look back over the current
   session's transcript, scoped to messages *after* the most recent
   `/rocky-checkpoint` invocation in this session (if any — otherwise the
   entire session). For each candidate learning:
   - **Filter bar (strict)**: extract only what the *user* actively engaged
     with — surprise, pushback, follow-up questions, or an explicit "huh,
     didn't know that." Things you merely mentioned without engagement do
     not qualify. Discussion ≠ learning. When in doubt, leave it out.
   - **Cross-check against pass A**: don't double-record an insight that a
     commit topic already captured — merge into that topic if the angle
     overlaps, otherwise skip.
   - **Cross-project dedup**: same as pass A — merge by meaning into
     existing global topics where applicable.

5. **Store each topic**. New OR merge:
   ```bash
   rocky add-topic \
     --name "Connection Pool Sizing" \
     --description "Postgres connections are expensive — pool size should match CPU count, not request concurrency." \
     --domain "Database" \
     --kind "concept" \
     --commit "<sha>" \
     --context "<the relevant diff hunk, ≤500 chars>"
   ```
   - **`--name`**: noun phrase, ≤8 words, Title Case. No verbs. For merges,
     use the existing name *exactly*.
   - **`--description`**: one sentence, ≤200 chars. State the *insight*, not
     the change.
   - **`--domain`**: one of `Language Database Auth API Frontend DevOps
     Architecture Performance Security Testing Tooling Data Other`.
   - **`--kind`**: `concept` (an idea), `pattern` (a recurring solution
     shape), or `implementation` (a specific way to do it in code).
   - **`--context`**: smallest diff hunk that demonstrates the topic
     (commit-pass topics) OR a 2–4 line conversation excerpt that captures
     the user's engagement with the idea (conversation-pass topics).
   - **`--commit`**: the SHA from the queue. **Omit entirely for
     conversation-pass topics** — they have no commit to anchor.

   The output JSON has `action: "created"` or `action: "merged"` plus
   `recall_now` so you know how the topic stands.

6. **For each topic just stored, add a generic question bank.** Generate
   1–3 questions per topic and call `rocky add-question` for each. Skip
   merged topics whose existing bank already has ≥4 questions
   (`rocky topic "<name>"` shows current bank size).

   ```bash
   rocky add-question \
     --topic "Connection Pool Sizing" \
     --question "When you size a database connection pool to match request concurrency instead of CPU count, what kinds of failure modes do you start seeing under load?" \
     --answer "Pools sized to request concurrency oversubscribe the database — once active connections exceed the DB's worker count, queries queue inside Postgres and tail latency spikes. CPU-count sizing keeps the pool below the DB's parallelism ceiling, so backpressure surfaces in the app's connection wait instead of inside the database. Trade-off: you must tolerate brief connection-acquire waits; if your app can't, scale horizontally rather than enlarging the pool." \
     --clue "Where does the bottleneck move when the pool is larger than what the database can actually run in parallel?"
   ```

   ### Genericization rules — apply strictly

   The question, answer, and clue must read as if you're asking another
   developer who has never seen this codebase. Strip:

   - Function, method, class, struct, and variable names from the diff
     (e.g. `bobInner()`, `warpToFocus`, `runSocraticLoop` → "the wrapper
     function", "the warp transition", "the quiz loop").
   - File paths and module names (`src/main.rs:1626` → omit; "the quiz
     loop" suffices).
   - Parameter names and field names that are project-local
     (`pregenerated_qa` → "a pre-generated question/answer pair").
   - Project-specific terminology that won't survive outside this repo.
   - Library/framework symbols *unless* the topic is genuinely about that
     library (a question about React's `useEffect` should keep the name;
     a question about a pattern that happens to use `useEffect` should
     describe the pattern abstractly).

   Keep:

   - The underlying concept, pattern, or constraint the diff illustrates.
   - Generic phrasing that tests reasoning ("what breaks", "when not to
     use", "what trade-off does this lock in").
   - One concrete-but-generic example if it sharpens the question (use
     placeholder names — "the parent component", "the outer transform").

   ### Question quality bar

   - 1–2 sentences. Specific enough to have a right answer; generic enough
     to make sense in any codebase that meets the same constraint.
   - Tests *implications*, *trade-offs*, or *consequences* — not recall
     of definitions. Avoid "what is X" / "define X".
   - Each item in a multi-question bank for one topic must hit a different
     angle (what breaks, when NOT to use, how it composes with adjacent
     systems, what would change if a key constraint were removed).
   - Answer: 3–5 sentences that demonstrate genuine understanding. The
     ideal answer for a test, not a textbook definition.
   - Clue: 1–2 sentences that nudge without revealing the answer.

   `rocky add-question` is idempotent on duplicate question text — if a
   question already exists in the bank verbatim, the call is skipped.

7. **Connect topics with semantic edges.** For each topic you've just stored,
   identify 1–3 strong edges to existing topics in the PKG (from `rocky list
   --json`). Only add an edge when the relationship is real and non-obvious —
   "both touch CSS" is not an edge; "WAL pragma implies single-writer model
   becomes the natural fit" is.

   ```bash
   rocky add-edge \
     --source "WAL + FK Pragmas as Schema Header" \
     --target "Single-Writer Justification for SQLite" \
     --kind "implies" \
     --strength 0.8 \
     --description "WAL gives concurrent reads; the single-writer model lets you skip write-side concurrency complexity, which the WAL approach was already implicitly assuming."
   ```

   Edge kinds:

   - `implies` — A being applied makes B more likely / natural / safer
   - `depends_on` — A requires B to exist or be understood first
   - `conflicts_with` — A and B can't both apply to the same context
   - `part_of` — A is a specific case or component of B

   **Strength calibration** (always pass `--strength`, don't let it default):

   - `0.6` — clearly related but the connection is non-obvious / interpretive
   - `0.7` — clearly related, obvious to anyone in the domain
   - `0.8` — closely coupled, hard to discuss one without the other
   - `0.9` — foundational, B is *necessary* for A to function

   Anything below 0.6 should be skipped — if it isn't at least clearly
   related, it isn't an edge. Default to 0.7 if you're unsure.

   Quality bar: 1–3 edges per new topic max. Skip if no real relationship
   exists. The description should explain the relationship in one sentence,
   not just restate the topic names. `rocky add-edge` is idempotent on
   duplicate (source, target, kind) — duplicates are silently skipped.

8. **Drain the queue**:
   ```bash
   rocky checkpoint mark
   ```

9. **Report**, one line. Mention both extraction sources and cross-project
   merges:
   `Checkpointed N commits → M commit topics + P conversation topics, K merged (J across projects), Q questions added, E edges.`
   If both passes produced zero topics, say "Nothing worth recording — no
   commits queued and no notable conversation insights."

## Rules

- **Atomicity beats coverage.** One sharp topic > three vague ones.
- **The user's voice matters.** If a commit message contains a clear
  "I learned that …" or "the trick is …", make that the topic.
- **Don't invent.** If a commit is too small to extract a learning from,
  skip it. Same goes for the conversation pass — if no message shows the
  user genuinely engaging with an idea, the right number of conversation
  topics is zero.
- **Conversation-pass scope.** The pass is bounded by the previous
  `/rocky-checkpoint` invocation in this session. Messages before that
  boundary were already considered.
- **Never call `rocky session-end`.** Legacy LLM-driven path.
- **Never call `rocky checkpoint mark` until every `add-topic` has succeeded.**
  Failed extractions stay queued for retry. (`add-question` failures are
  not blocking — questions can always be filled in later by re-running the
  skill or `rocky backfill --fill-question-bank`.)

## Boundaries

Writes only to `~/.rocky/graph.db`. Doesn't push, sync, or share externally.
Doesn't modify the working tree. To undo, the user can run `rocky delete-topic <name>`.
