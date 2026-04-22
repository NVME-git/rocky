# 0001 — Rich-context pipeline (explore + queue + session-end)

**Status:** Accepted (2026-04-22, alpha)
**Supersedes:** the original "one LLM call per commit at commit time" path

## Context

The original `rocky diff` flow called the LLM with just the commit message and the raw diff at commit time, then immediately stored a one-sentence description per topic. At quiz time the LLM had only that one sentence to work from, so it pattern-matched to a generic template — *"What are the implications of X?"* — which the user reported as **vague and formulaic**.

The root cause was thin context. The model could not generate a question grounded in *why* the developer made the change, *what trade-offs* they weighed, or *how it interacts* with existing project structure, because none of that information was present in the diff alone.

Three relevant pieces of context exist on the developer's machine but were not being used:
1. The project itself (README, CLAUDE.md, ADRs, recent commits).
2. The Claude Code session transcript (the agent's reasoning, what files it consulted, what the user actually asked for).
3. Multiple commits in a single session — the LLM was processing them one-at-a-time as if each was independent.

## Decision

Replace the single-step pipeline with a three-stage one:

1. **`rocky explore`** — once per project, summarise CLAUDE.md / README / `docs/` / `architecture/` / recent commits via the LLM into a multi-paragraph **project context** stored in the `project_context` SQLite table. Re-run when the project's shape changes (or auto-nudge after 20 commits / 14 days of staleness).

2. **`rocky post-commit`** — runs from the git post-commit hook. **No LLM call.** Just appends `(commit_sha, commit_msg, diff)` to the `pending_diffs` queue and returns immediately. Keeps `git commit` snappy.

3. **`rocky session-end`** — runs from the Claude Code Stop hook. Drains `pending_diffs`, reads the recent Claude Code session transcript (privacy-stripped — file paths and command names only, no code content), and makes one LLM call per topic that produces:
   - A rich description grounded in project context + diff + transcript.
   - A **question bank** of 4 implication-grounded Q+A+clue triples (see [0003](#)).
   - Topic name reuse via Layer-1 dedup (see [0002](#)).

Capped at 5 commits per Stop-hook invocation to avoid blocking the terminal; remaining commits stay queued for the next Stop event.

## Consequences

**Positive:**
- Question quality is materially higher. Verified end-to-end against `llama3.1:8b` via `scripts/tutorial.sh`: questions like *"What would change if Redis were not available for storing refresh tokens?"* and *"What are the implications of not rotating refresh tokens on every use?"* — implication-grounded, project-specific.
- `git commit` no longer waits on LLM — Rocky's overhead at commit time is a few SQLite inserts.
- Multiple commits in a session are processed together, so cross-commit dedup works (third commit on the same concept becomes `encounter +1` instead of a duplicate node).

**Negative / costs:**
- Three new commands and two new SQLite tables (`project_context`, `pending_diffs`) increase surface area.
- Stop-hook latency is higher (we batch more work into it). Mitigated by the 5-commit cap.
- Requires Claude Code to install a Stop hook — a one-line setting, but still a setup step.

**Verification gate:**
- `scripts/tutorial.sh --noninteractive` walks the full pipeline against an isolated `~/.rocky-tutorial/` and must finish with non-empty `question_bank` for the topics it generates.
