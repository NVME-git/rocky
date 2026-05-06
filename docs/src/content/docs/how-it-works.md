---
title: "How Rocky Works"
order: 6
---
## The pipeline at a glance

Rocky doesn't extract topics one commit at a time. It batches diffs across a whole working session, pairs them with the Claude Code transcript and a cached project summary, and produces **rich nodes with a question bank** (~4 implication-grounded Q+A+clue triples each) — driven by an agent already sitting in your editor.

| Stage | What happens | Where |
|---|---|---|
| `rocky explore` | Reads CLAUDE.md / README / docs / recent commits and synthesises a **project context** summary cached at `~/.rocky/summaries/<repo>.txt` | Run once per project, again after major shape changes |
| `rocky post-commit` (queue mode) | Silently appends the latest commit's diff to a per-project queue. **No LLM call.** | Wired by `rocky install claude-all` |
| `/rocky-checkpoint` (Claude skill) | Drains the diff queue, reads the active session's transcript, and writes nodes + **generic question banks** straight into the PKG. Stripped of repo-specific identifiers so the same question still works when the topic resurfaces in a different project. Uses the **global** dedup list — same topic across two projects becomes one node with `repos[]` accumulating. | Invoked inside Claude Code, end of a session |
| `/rocky-quiz` (Claude skill) | Picks the weakest topics by `recall_now`, asks from the canonical question bank, records scores back to FSRS. | Invoked inside Claude Code any time |
| `/rocky-backfill` (Claude skill) | Seeds the PKG from a project's existing git history. Same extract-and-question loop as `/rocky-checkpoint` but driven by `rocky checkpoint history` instead of the post-commit queue, so it works on repos installed *after* the commits happened. | Invoked once per repo when adopting Rocky on an existing project |

The motivation: a single commit message like *"feat: rotate refresh tokens"* is too thin a context to ground good questions in. By batching at session end, the extractor has the project summary, the actual diffs, **and** the agent's reasoning trail. And by running inside Claude Code itself rather than shelling out to a local Ollama on every Stop event, the per-turn latency drops to zero — the heavy step only happens when you ask for it.

> The legacy Stop-hook → Ollama path still ships and is opt-in via `rocky install stop`. It runs after every Claude turn and is useful if you don't keep a Claude session open the whole day. The skill-driven default is faster and produces sharper extraction because it sees the full transcript at once.

### Inspecting what was generated

The web UI is the canonical viewer — see **Quick Start → Step 3** for screenshots of each tab. The Knowledge Map shows the topic graph; the Review Queue surfaces what's most overdue; the Sessions tab lets you scrub through what landed day by day.

### Cross-project dedup

When the checkpoint skill extracts topics, it's fed the **global** topic list (across every project Rocky knows about) and asked to reuse exact names where a new finding is semantically equivalent. The result: a topic like *Token Rotation* lives as one node with a `repos[]` array that accumulates as the same idea reappears in another codebase.

```
▸ refactor: surface is_rotated helper for refresh token reuse checks
  ◇ Token Rotation (existing — encounter +1, repos: [taskify, home-bank])

Done. 0 new topic(s), 1 encounter update(s).
```

The web UI's **Project** field on a topic shows everywhere it's appeared. Layer-2 (semantic dedup via embeddings + cosine, beyond today's lexical Jaccard) is on the [backlog](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md).

---

## How the PKG classifies topics

Every topic has a **recall** score: `recall_now = retrievability × mastery`.

- **Retrievability (R)** — FSRS freshness from spaced repetition. Decays with time since the last review. Decays slower when stability is high (you've demonstrated solid understanding).
- **Mastery (M)** — mean of the last 3 review scores (default 0.5 if you've never been quizzed). Captures *how well you've actually been answering*, not just how recently.

**Recall lifecycle:**

1. **Topic created** → stability set by kind (Concept 4.0, Pattern 2.5, Implementation 1.5), mastery defaults to 0.5
2. **Topic reviewed** → score 0.0 to 1.0 is recorded. Stability rises if score ≥ 0.65; it floors at 1.0 + difficulty bumps if score < 0.4. The new score replaces the oldest in the last-3 mastery window.
3. **Over time** → R decays. M is sticky until you take another quiz.
4. **Classification:**
   - recall ≥ 0.6 → **Known** — skipped automatically
   - 0.3 ≤ recall < 0.6 → **Fading** — surfaced as a review candidate
   - recall < 0.3 → **Gap** — full Socratic Q&A from the question bank

The multiplicative model is the point: a freshly-reviewed topic where you got the question wrong (R high, M low) is still a gap. Freshness alone doesn't count as knowing.

---

## Domain taxonomy

Every topic is assigned to one of 13 domains when it's first extracted. Domains group topics in the PKG into subfolders and are used for Obsidian graph view clustering.

| Domain | Examples |
|---|---|
| Language | Rust lifetimes, Python decorators, Go channels |
| Database | SQL indexes, Redis TTL, Postgres transactions |
| Auth | JWT, OAuth2, RBAC, session tokens |
| API | REST design, GraphQL, WebSockets |
| Frontend | React hooks, DOM events, CSS layout |
| DevOps | Docker networking, CI/CD pipelines |
| Architecture | Event sourcing, retry patterns, microservices |
| Performance | Caching strategies, query optimisation |
| Security | OWASP, encryption, input validation |
| Testing | Unit vs integration, mocking, TDD |
| Tooling | Build systems, package managers |
| Data | Algorithms, data structures, ML concepts |
| Other | Anything that doesn't fit above |

Use `rocky classify` to assign domains to any older topics that predate this feature.
