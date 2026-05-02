# 0002 — Layer-1 dedup + question bank with rotation

**Status:** Accepted (2026-04-22, alpha)
**Related:** [0001 — Rich-context pipeline](0001-rich-context-pipeline.md)

## Context

Two independent problems shared a fix:

**Problem A — duplicate topics.** The original pipeline saw each commit in isolation, so semantically equivalent topics accumulated as separate nodes (`"JWT refresh token handling"`, `"token refresh flow"`, `"JWT token lifecycle"`). Each had its own FSRS review history and got quizzed independently — fragmenting the user's effort.

**Problem B — question monotony.** Each node stored exactly one canonical Q+A. The user got the same question every time the topic came up for review, so they memorised the question instead of the underlying concept.

## Decision

**Layer-1 dedup at extraction time.** When `session-end` calls the LLM to extract topics from a diff, the prompt receives the **full list of existing topic names** (with domains for context). The system prompt instructs the model to *return the EXACT existing topic name rather than create a new entry* when a finding is semantically equivalent. The implementation is in `Teacher::extract_topics_with_dedup`. When a returned name matches an existing node, Rocky bumps `encounter_count` and appends to `source_commits[]` instead of inserting a new row — FSRS state is preserved.

**Layer-2 dedup is deferred** (will be a `rocky dedupe` interactive command). Layer-1 catches >80% of duplicates in practice; Layer-2 is the safety net for long-tail near-duplicates that slip through. Ships in v0.2 or later.

**Question bank.** Each node now stores `question_bank: Vec<QuestionBankItem>` — typically 4 Q+A+clue triples generated together at `session-end` time (one LLM call per node). Each item carries an `asked_count` field. At quiz time the runtime picks the **least-asked** question, falling back to the canonical Q&A only if the bank is empty (legacy nodes).

## Consequences

**Positive:**
- Verified in `scripts/tutorial.sh`: a third commit touching the same refresh-token concept produces `◇ Token Rotation (existing — encounter +1)` instead of a near-duplicate.
- Quiz variety: the user sees 4 different angles on the same topic across reviews — implications, trade-offs, dependencies, and counterfactuals — without needing 4 separate review sessions.
- `encounter_count` becomes a useful signal in the UI (`×N` badges in the Sessions tab) showing which topics keep recurring.

**Negative / costs:**
- The extraction prompt grows by the size of the existing topic list (currently ~2KB at 335 topics — well below any token limit).
- Question-bank generation is the slowest step in `session-end`: ~1 LLM call per topic, each producing JSON of ~1.5KB. Mitigated by the 600s Ollama timeout + retry (see [0003](0003-llm-resilience.md)).
- Layer-1 dedup is opportunistic — the LLM occasionally still creates near-duplicates with a different surface form. Layer-2 will close that gap.

**Migration:**
- `rocky backfill --fill-question-bank` regenerates banks for legacy nodes that pre-date this work.

## Update (2026-05-03)

- **Layer-1 is now agent-side, not server-side.** ADR [0007](0007-skill-only-extraction-and-promptiq.md) moved topic extraction onto the `rocky-checkpoint` skill. The skill reads `rocky list --json` for the global topic list and is instructed to reuse the existing name when a new finding is semantically equivalent — same dedup contract as before, just from the agent loaded with full session context instead of from a Rust-side LLM call. The original implementation, `Teacher::extract_topics_with_dedup`, has been removed as dead code.
- **Layer-2 shipped** as `rocky dedupe`, using `topic_jaccard` (word-set Jaccard ≥ 0.5) plus a substring fallback for the `Auth` ↔ `Authentication` shape (`main.rs:topic_jaccard`, `is_candidate_pair`).
- **Layer-3 is on the backlog.** Lexical similarity misses true synonyms with no shared root (`DB` ↔ `Database`, `Pooling` ↔ `Pool`) and pure semantic equivalents (`Connection Pool Sizing` ↔ `How many DB connections is too many?`). The next evolution is embedding-based semantic dedup — embed name+description with a local model, store the vector alongside the node, merge on cosine > ~0.85, fall back to the LLM only for the ambiguous middle band. Tracked in [BACKLOG.md](../../BACKLOG.md#embedding-based-semantic-dedup-planned). When that lands it will likely warrant its own ADR superseding the dedup portions of this one.
