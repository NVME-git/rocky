# Rocky — Project Plan

*Synthesized from: user answers (2026-04-02) + research across SRS, PKM, AI tutoring, developer tooling, local LLMs, graph DBs, and competitive landscape.*

---

## Naming Convention

- **Rocky** — the tool overall, and the AI teacher/agent character
- **PKG** — the Personal Knowledge Graph (the data layer: what you know, how well, how recently)

Rocky coordinates the lifecycle of both. The teacher is not useful without the graph. The graph is not useful without a teacher. The coordination between them — timing, routing, deciding when to intervene — is the product.

---

## What Rocky Actually Is

A coordination system that sits between a developer and their AI agents. Rocky observes what work is delegated, tracks what the developer understands in their PKG, and teaches — via Socratic questioning — only when the timing is right.

Rocky has two halves:
- **The PKG**: a living record of what you understand, how well, and how recently
- **Rocky (the teacher)**: an AI character who asks implication-focused questions and knows when to stay quiet

---

## What to Keep, Change, and Discard

### Keep
- The Socratic question philosophy (implications, not facts) — research confirms this is correct
- Silent pass for known topics — never interrupt flow
- The `classify → remind/quiz → update` loop structure
- Claude Code skill integration via `graph_cli.py`

### Change
- **Storage**: JSON → SQLite + markdown files (Obsidian-compatible vault)
- **SRS model**: simple half-life decay → FSRS-inspired DSR model (Difficulty, Stability, Retrievability per node)
- **Trigger model**: manual-only → configurable (before / after / session-end / manual)
- **LLM layer**: Claude-only → abstracted provider (Claude + Ollama)
- **PKG location**: local `knowledge_graph.json` → `~/.rocky/graph.db` (global, follows the user)

### Discard
- `knowledge_graph.json` as the source of truth
- Hardcoded `GRAPH_FILE` path constant
- The current `confidence_delta` model (too simple, no difficulty tracking)

---

## Architecture

```
rocky/
├── rocky/
│   ├── cli.py              # entry point — argparse, output formatting
│   ├── teacher.py          # Rocky: topic extraction, question gen, answer eval
│   ├── config.py           # config loading (.rocky.toml or env vars)
│   │
│   ├── graph/              # PKG subsystem
│   │   ├── store.py        # SQLite read/write, node CRUD (~/.rocky/graph.db)
│   │   ├── fsrs.py         # DSR model: Difficulty, Stability, Retrievability
│   │   └── node.py         # Node dataclass
│   │
│   ├── capture/
│   │   ├── session.py      # session state: last quiz time, topics quizzed, task log
│   │   └── hooks.py        # git hook and shell hook installers
│   │
│   ├── export/
│   │   ├── obsidian.py     # writes markdown vault files with YAML frontmatter
│   │   └── report.py       # CLI progress summaries
│   │
│   └── llm/
│       ├── base.py         # LLMProvider protocol
│       ├── claude.py       # Anthropic SDK backend
│       └── ollama.py       # Ollama OpenAI-compatible backend
│
├── hooks/
│   ├── pre-commit          # installable git hook
│   └── rocky-session-end   # shell hook for session-end mode
│
├── demo/                   # existing demo (update paths after rename)
├── CLAUDE.md
├── ROCKY_PLAN.md
└── pyproject.toml          # replaces requirements.txt
```

---

## The PKG: FSRS-Inspired DSR Model

Replace the current half-life confidence decay with three values per PKG node:

| Field | Description | Default |
|---|---|---|
| `difficulty` | How hard this topic is for this user (0–1) | 0.3 |
| `stability` | Days until retrievability drops to 90% | 1.0 |
| `retrievability` | Current probability of recall — computed, not stored | — |

**Retrievability formula:**
```
R = (1 + elapsed_days / (9 * stability)) ^ -1
```

**When Rocky intervenes:**

| R value | Action |
|---|---|
| R ≥ 0.90 | Silent pass |
| 0.70 ≤ R < 0.90 | Rocky gives a brief reminder |
| R < 0.70 | Rocky runs the Socratic Q&A loop |

**After a successful quiz, stability increases:**
```
stability_new = stability * difficulty_multiplier * (1 + base_gain)
```

The key insight from FSRS: reviewing right as something is fading gives the biggest stability boost. Reviewing immediately after learning gives almost nothing. This is why Rocky's work-context trigger is better than scheduled review — it fires naturally when topics are being encountered.

---

## The Trigger Model

Rocky should never be the reason work slows down. The default is **after** mode — review what the agent just did, not block what it's about to do.

| Mode | When Rocky activates |
|---|---|
| `before` | Before handing task to agent — deliberate learning |
| `after` | After task marked done or git commit — **default** |
| `session-end` | End of day / shell exit — lowest friction, batch review |
| `manual` | Explicit `rocky quiz` command — current behaviour |

### Smart Silence Rules (always apply)

Rocky never asks more than **3 questions per session** regardless of how many topics are overdue. Additional overdue topics queue and surface in future sessions when they're relevant to new work.

- R ≥ 0.90 for all topics → skip entirely, no output
- Quizzed in the last 2 hours → skip (configurable: `min_gap_minutes`)
- Hotfix commit (message starts with `fix:` or `hotfix:`) → skip
- Fewer than 2 new/stale topics → one-line summary only, no quiz
- Topics quizzed are filtered to those relevant to the current work context — forgetting OAuth doesn't surface during a SQL migration

Session state (last quiz time, topics quizzed, tasks logged) is tracked in `session.py` so the budget and cool-down have something to work against.

---

## The LLM Abstraction

```python
class LLMProvider(Protocol):
    def complete(self, system: str, user: str) -> str: ...
```

Two backends:
- `ClaudeProvider` — Anthropic SDK, model configurable
- `OllamaProvider` — OpenAI-compatible SDK pointed at `localhost:11434`

Config (`.rocky.toml`):
```toml
[llm]
provider = "claude"          # or "ollama"
model = "claude-sonnet-4-6"  # or "qwen2.5:14b"
ollama_base_url = "http://localhost:11434"
```

**Quality note**: 8B local models produce adequate questions but unreliable answer evaluation. 14B+ is meaningfully better. Rocky defaults to Claude; Ollama is an opt-in for privacy-conscious or cost-sensitive users.

---

## The PKG as Obsidian Vault

Each PKG node is also a markdown file written to `~/.rocky/vault/` (default on, opt-in for other connectors later). This gives Obsidian interop for free — no plugin required, just point Obsidian at the vault directory.

```markdown
---
rocky_id: jwt-authentication
rocky_domain: backend
rocky_kind: pattern
rocky_difficulty: 0.42
rocky_stability: 14.3
rocky_last_reviewed: 2026-03-28
rocky_review_count: 3
tags: [rocky/node, rocky/domain/backend]
---

# JWT Authentication

Stateless token-based auth where the server signs a payload the client stores and sends back.

## Contexts
- add JWT authentication to my REST API (2026-03-10)
- implement role-based access control (2026-03-28)
```

Dataview queries work out of the box. The vault is version-controllable. Rocky never requires Obsidian to be running.

---

## Phased Plan

### Phase 1 — Rename and Restructure
- Rename and reorganise into the package layout above
- Replace `requirements.txt` with `pyproject.toml`
- `rocky` available as a global CLI (`pip install -e .`)
- All existing behaviour preserved — no new features

### Phase 2 — Better PKG
- Replace JSON storage with SQLite at `~/.rocky/graph.db`
- Implement DSR model (`store.py`, `fsrs.py`, `node.py`)
- Write Obsidian vault files on every PKG update
- All CLI commands (`rocky "task"`, `--stats`, `--list`) still work

### Phase 3 — Better Triggers
- Implement trigger modes (before / after / session-end / manual)
- Git hook installer (`rocky install --hook git`)
- Smart silence rules + session budget
- Task descriptions logged to SQLite (groundwork for prompt quality tracking)

### Phase 4 — LLM Abstraction + Ollama
- Extract `llm/` layer with provider protocol
- Implement Ollama backend
- Config file support (`.rocky.toml`)

### Phase 5 — Prompt Quality Tracking
- Analyse logged task descriptions for complexity and specificity over time
- Session-end report: PKG growth + prompt evolution
- Aspirational PKG roadmap: given your current graph, what should you learn next?

---

## Decisions Made

| Decision | Choice | Rationale |
|---|---|---|
| PKG location | `~/.rocky/graph.db` global | The PKG follows the user, not the project |
| Obsidian vault | Default on in dev, opt-in later | Free interop, add other connectors as opt-ins |
| Distribution | Global CLI via `pip install` | Developer tool, not a project dependency |
| Language | Python | Rocky is IO-bound (API latency), not CPU-bound. Rust adds complexity with no measurable gain |
| Storage | SQLite + sqlite-vec | Single file, zero infra, version-controllable |

## What Does NOT Go in v1

- Web UI or dashboard — Rocky lives in the terminal
- Team or shared PKGs
- Non-developer use cases
- Paid tier or accounts
- Mobile
