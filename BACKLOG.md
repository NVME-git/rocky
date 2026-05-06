# Rocky — Feature Backlog

Ideas and planned features, organised by theme. Items marked **[planned]** have documentation written.
Items marked **[idea]** are under consideration. Items marked **[in progress]** are being built.

---

## Teaching Quality

### Struggle score affects stability `[planned]`
Track how many follow-ups, clues, and scaffolding requests were made before a topic was marked
"understood". A topic answered straight through should earn higher stability than one that required
three follow-up questions and a clue. This makes *quality* of understanding matter, not just outcome.

### AI-source tagging `[planned]`
When a topic first enters the PKG via the Claude Code hook (i.e. seen in a prompt, not in the
author's own code), tag it `source: ai_prompt`. Quiz those topics more aggressively and flag them
in `rocky ls` as "AI-introduced" until the author has independently demonstrated understanding.

Rationale: confidence without competence. A developer can have a full PKG of "known" topics they
have only ever seen AI handle. Source tagging makes this visible.

### Independence score `[planned]`
A second axis alongside retrievability: how often has this topic appeared in the author's own diffs
(own_code) vs. only in AI-assisted prompts (ai_prompt)? Topics with high retrievability but low
independence are risk indicators. Surface these in `rocky stats` as a distinct category.

### Hard mode (no scaffolding) `[planned]`
`rocky quiz --hard` disables clues (`[c]`), explanations (`[?]`), and simplification (`[s]`).
Forces the author to either answer from genuine understanding or skip. Configurable as a permanent
setting in `.rocky.toml` for authors who want to build resilience deliberately.

### Debugging-focused question kind `[planned]`
A new question type generated at backfill/diff time: given this code change, what are the failure
modes? What would a bug look like in production and what would the stack trace tell you? Distinct
from the current Socratic implication question — more forensic, targeting the durable skill of
debugging that AI cannot fully replace.

### Pre-task gap framing `[planned]`
When running `rocky "task description"`, explicitly surface knowledge gaps as AI-risk warnings:
"You have a gap on Lua scripting in Redis — this is a topic AI will likely handle for you. Here
is the question you should be able to answer before trusting what it produces." Frames Rocky as a
check on AI output, not just a learning tool.

---

## Workflow Integration

### Rocky as a required PR check `[planned]`
A GitHub Actions workflow that runs Rocky as a required status check on pull requests. Before a
PR can be merged, Rocky analyses the diff, identifies topics introduced, and checks whether the
author has those topics in their PKG at a minimum retrievability threshold.

Supports GitHub's stacked PRs feature: each layer of a stack must demonstrate understanding of
the topics introduced in that layer before it can merge into the next.

See: [Roadmap section in docs](docs/lib/content.dart) for full design.

### Launch agent quiz/teach session from rocky view `[idea]`
Button in the rocky view web UI that starts a Claude Code or OpenCode session
with `/rocky-quiz` or `/rocky-teach` pre-loaded. Bridges the in-page quiz
(quick, no agent) with the agent-driven modes (richer adaptive grading,
teacher persona role-play).

Phased implementation:
1. **Click-to-copy + toast** — button copies `cd <project> && claude "/rocky-quiz"`
   to clipboard, shows "Pasted — run it in your terminal." Bulletproof, works
   on any OS, ships in an afternoon. Good default.
2. **Server-spawned terminal (opt-in)** — new `POST /api/launch?mode=quiz|teach`
   endpoint. Server detects terminal via `$TERMINAL` then a fallback chain
   (kitty → ghostty → alacritty → foot → gnome-terminal), spawns it with
   the agent CLI and starter prompt. Behind a config flag (e.g.
   `[view] launch_terminal = true`) so SSH-tunnel users aren't surprised
   by terminals opening on the remote machine.

Open questions to settle before building: (a) verify `claude "<prompt>"` and
the OpenCode equivalent both accept a positional starter prompt cleanly;
(b) UX disambiguation from the existing in-page quiz at `/api/quiz/*` —
labelling needs to make clear when to pick which.

Out of scope for this entry but worth noting: a custom URL scheme
(`rocky://quiz?project=...`) would be a third path, but adds an install
step and a browser security prompt — defer unless the click-to-copy or
terminal-spawn paths prove insufficient.

### Rocky as a retroactive clue filler `[in progress]`
`rocky backfill --fill-clues` — generates missing clues for nodes that already have canonical Q&A.
Useful after upgrading Rocky versions that add new node fields. Foundation for a future scheduler
that keeps the PKG enriched as new features are added.

---

## Graph & Memory

### Canonical clue generation `[in progress]`
Short hint stored alongside canonical Q&A, generated at backfill time from the diff. Shown when
the user types `[c]` during a quiz. For manually-added topics, generated on-demand at runtime.

### Scheduled graph enrichment `[idea]`
A background scheduler that runs enrichment passes over the PKG when new features are added —
generating missing fields (clues, cross-concept edges, debugging questions) without requiring
manual intervention. Goal: quiz time is always fast because all pre-computation happened offline.

### Make `rocky add-topic` merge-update semantics explicit `[idea]`
On a merge (the topic name already exists), `rocky add-topic` silently preserves
the existing `description` and `domain` even when new values are passed via
`--description` / `--domain`. Only the `--context` is appended and the
question-bank insert proceeds normally. Surfaced when re-running
`/rocky-checkpoint` against an evolved topic during a conversation pass —
the new framing was lost and the old (sometimes incorrect) framing stayed.

The conservative default is fine when add-topic is being called from a
post-commit hook on autopilot (don't let a stale auto-extracted line
clobber a curated description). It's a footgun when an agent is deliberately
re-framing a topic with new context. Two ways out:

1. **Update on explicit flag pass** — if `--description` or `--domain` was
   supplied on the merge call, replace; otherwise keep. The CLI already
   knows whether the flag was present vs default.
2. **Separate `rocky update-topic` primitive** — clean separation of "create
   or merge-by-context" from "in-place edit." Skill could call update-topic
   when a merge target's framing has clearly drifted.

Lean toward #1 — single primitive, behaves the way a caller intuitively
expects ("if I passed it, I meant it"). #2 is cleaner architecturally but
adds a new command surface for a behavior that's almost always paired with
add-topic anyway.

### Conversation-source tagging in topic listings `[planned]`
Visually distinguish chat-sourced topics from commit-sourced topics in `rocky list`,
`rocky topic`, `rocky inspect`, and the dashboard. With the conversation-reflection
pass added to `/rocky-checkpoint`, topics can now enter the PKG without a backing
commit — the schema already supports this (a topic with zero non-empty SHAs across
its `topic_encounters` is chat-only), but display paths render them identically to
commit-grounded topics.

Why it matters: when reviewing the PKG, the user needs to know which topics have
real code grounding vs. which are vibes-from-a-conversation. Without the tag, the
two classes are indistinguishable and chat topics inherit unearned authority.
Pairs with the existing AI-source tagging idea above — both are about making the
provenance of a topic legible at a glance.

Cheapest implementation: derive the tag at display time from `topic_encounters`
(no schema migration), surface as a `[chat]` badge in CLI output and a field in
`node_to_json` so the web view picks it up too.

### Embedding-based semantic dedup `[planned]`
Replace today's lexical dedup (slugified node id + `topic_jaccard` token-set similarity in
`rocky dedupe`) with vector similarity. Embed each topic's name + description with a small local
model (e.g. Ollama `nomic-embed-text`, ~270 MB), store the vector alongside the node, and on add
look up nearest neighbours; merge if cosine > ~0.85. The LLM is reserved as a tiebreaker for
ambiguous mid-band cases instead of being the primary dedup mechanism.

**Why move past Jaccard:** the current heuristic is order-insensitive and catches "Bank Question"
↔ "Question Bank" cleanly, but misses true synonyms with no shared root — `Auth` vs
`Authentication`, `DB` vs `Database`, `Pooling` vs `Pool`. The substring fallback in
`is_candidate_pair` papers over a few of those, but anything where the wording diverges entirely
(`Connection Pool Sizing` ↔ `How many DB connections is too many?`) still slips through. Cosine
on sentence embeddings collapses both surface-form and semantic variants, deterministically.

**Tradeoff:** adds an embedding model dependency (~100-300 MB depending on choice) and a `vector`
column on `nodes`. Worth it once the PKG grows past a few thousand topics, where false-negative
duplicates start fragmenting review effort. See ADR 0002 for the dedup history.

---

## References

Features in this backlog were informed by the following sources:

- **Dr. Mark Mahoney interview — FreeCodeCamp Podcast** (2026-04-11)
  Competence vs. confidence gap, debugging as durable skill, hard-way resilience, AI-assisted
  tutorial hell. See [References section in docs](docs/lib/content.dart).
  Video: https://youtu.be/Tb6oaEkxtp8?si=W_vu3K-0WknmvOTI
