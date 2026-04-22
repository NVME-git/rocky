# 0003 — LLM resilience: timeouts, retries, lenient parsing

**Status:** Accepted (2026-04-22, alpha)

## Context

End-to-end testing of the rich-context pipeline against `llama3.1:8b` running locally on Ollama surfaced three classes of failure that were not present (or rare) when using Claude:

1. **Connection drops mid-session.** `reqwest::blocking::Client::new()` defaults to no read timeout, but Ollama's HTTP layer occasionally drops connections under sustained load — for example, when generating four question banks back-to-back. The first 1-2 requests succeeded; the third would fail with `error sending request for url`.

2. **Long generations exceeding silent defaults.** Question-bank generation routinely produces 1.5KB of JSON, which on CPU with `llama3.1:8b` can take 60-120s. Various transport-layer timeouts in the chain (proxy, OS, default reqwest behaviour) were inconsistent.

3. **Inconsistently-shaped JSON.** The same prompt could return:
   - A bare array: `[{...}, {...}]`
   - A wrapped object: `{"topics": [{...}, ...]}`
   - A single object: `{"topic": "...", ...}`
   - A topic missing the `kind` field entirely.

   Each shape would crash `serde_json::from_str::<Vec<TopicInfo>>` with a different error — and the alpha tester would see *"Failed to extract topics from diff"* on their first commit.

The Claude provider also benefits from these fixes but rarely triggered them in testing.

## Decision

**Generous, retry-aware HTTP clients.** Both providers now build clients with explicit timeouts:
- Claude: 120s
- Ollama: 600s

Ollama additionally wraps each request in **3 attempts with exponential backoff** (500ms, 1s, 2s). Retries fire only on transport errors — not on HTTP status errors or successful-but-empty responses.

**Tolerant JSON parsing.** `parse_topics_lenient(cleaned)` accepts all three observed shapes and is used by every topic-extraction call site (`extract_topics_from_diff`, `extract_topics`, `extract_topics_with_dedup`). The `TopicInfo` struct uses `#[serde(default)]` for non-essential fields:
```rust
#[serde(default = "default_kind")]  pub kind: String,        // → "concept"
#[serde(default)]                    pub domain: String,
#[serde(default)]                    pub description: String,
```

**No silent fallbacks for genuinely missing data.** If the LLM returns valid JSON but no extractable topic list, we return `Err` with a clear message — better than inventing topics from nothing.

## Consequences

**Positive:**
- Tutorial-script run rate improved from ~50% (2 of 4 question banks failed) to ~100% across multiple runs.
- The `kind` field in particular is mostly cosmetic at the application layer — defaulting to `"concept"` keeps the pipeline moving when the model omits it.
- Future LLMs (smaller/larger) get the same robustness for free.

**Negative / costs:**
- A genuinely broken response now takes up to ~13s before bailing (3 attempts × backoff). For an interactive flow this would be too long; for `session-end` (which already takes minutes) it's acceptable.
- Hiding malformed-JSON behind `parse_topics_lenient` could mask a *systematic* prompt regression. Mitigation: when topic counts plummet across runs, look at raw responses in `~/.rocky/` debug logs (planned).

**Open question:**
- Should we also store the raw LLM response on parse failure so we can iterate on the prompt? Probably yes — a `~/.rocky/llm-failures.jsonl` log. Deferred to v0.2.
