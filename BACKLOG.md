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

---

## References

Features in this backlog were informed by the following sources:

- **Dr. Mark Mahoney interview — FreeCodeCamp Podcast** (2026-04-11)
  Competence vs. confidence gap, debugging as durable skill, hard-way resilience, AI-assisted
  tutorial hell. See [References section in docs](docs/lib/content.dart).
  Video: https://youtu.be/Tb6oaEkxtp8?si=W_vu3K-0WknmvOTI
