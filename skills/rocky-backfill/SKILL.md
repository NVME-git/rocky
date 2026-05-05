---
name: rocky-backfill
description: >
  Seed the user's Rocky personal knowledge graph (PKG) by extracting topics +
  generic question banks from historical git commits in the current project.
  Use when the user has just installed Rocky in a project with existing history,
  or wants to fill gaps the post-commit queue missed. Trigger phrases:
  "/rocky-backfill", "rocky backfill", "seed my PKG from this repo's history",
  "backfill topics from past commits".
---

You are Rocky's history-aware seeder. Same job as `/rocky-checkpoint` — extract
atomic learning topics and store them via the `rocky` CLI — but you operate on
arbitrary historical commits instead of the post-commit queue. The cross-project
PKG is global (`~/.rocky/graph.db`); merge by meaning across projects.

## What makes this different from `/rocky-checkpoint`

- **Source.** `rocky checkpoint history` instead of `rocky checkpoint diff`.
  History is read-only and never drains anything — there is no `mark` step.
- **Volume.** A backfill run can cover dozens of commits. Pace yourself: stop
  early if the user asks, and report progress between batches.
- **Generic questions.** The questions you generate must NOT leak repo-specific
  identifiers — they should test the underlying concept so they remain useful
  if the user revisits the topic in a different project later.

## Procedure

1. **Confirm scope with the user** if they didn't specify. Ask:
   - How many commits to scan (default: 50, newest first).
   - Their commits only or all authors (default: their commits).

2. **Read the history**:
   ```bash
   rocky checkpoint history --limit <N> [--all-authors]
   ```
   Returns the same JSON shape as `rocky checkpoint diff` — `commits[]` with
   `sha`, `subject`, `message`, `diff`. Empty `commits` → tell the user
   "Nothing to backfill — no commits matched." and stop.

3. **Read the global dedup list** (across all projects):
   ```bash
   rocky list --json
   ```
   Look for matches by *meaning*, not just name. Cross-project merges go to
   the existing canonical name.

4. **Read this project's context**:
   ```bash
   rocky context
   ```
   Use `summary` to ground topic naming. `null` summary → suggest
   `rocky explore` first, but continue.

5. **Extract topics commit-by-commit.** For each commit:
   - 1–3 atomic learnings — one specific concept, pattern, or implementation
     detail per topic. Skip pure plumbing.
   - Cross-project dedup: if a near-match exists, use the existing topic's
     name *exactly*.
   - Store via `rocky add-topic` (same flags + format as `/rocky-checkpoint`).

6. **For each topic just stored, add a generic question bank**. Generate
   1–3 questions per topic and call `rocky add-question` for each:
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

7. **No `rocky checkpoint mark`** — backfill never drains a queue. Skip
   this step entirely.

8. **Report**, one line:
   `Backfilled N commits → M new topics, K merged (J across projects), Q questions added.`

## Rules

- **Atomicity beats coverage.** One sharp topic > three vague ones.
- **Generic > specific.** When in doubt, strip the identifier. The user can
  still see the source commit via `rocky topic <name>` if they want detail.
- **Don't invent.** Skip commits too small to extract a real learning from.
- **Stop on user signal.** Long backfills should be interruptible — check
  in every ~10 commits if the user is in the loop.

## Boundaries

Writes only to `~/.rocky/graph.db`. Doesn't push, sync, or share externally.
Doesn't modify the working tree. To undo, the user can run
`rocky delete-topic <name>` per topic.
