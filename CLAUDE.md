# Rocky

Developer tool that combats AI skill atrophy. Rocky (the AI teacher) quizzes developers on topics their agents handle, updating their Personal Knowledge Graph (PKG) as they go.

**Naming convention:**
- **Rocky** = the tool overall, and the AI teacher/agent character
- **PKG** = the Personal Knowledge Graph (the data layer — what you know, how well, how recently)

## How to run

Always activate the venv first:

```bash
source .venv/bin/activate
```

Main CLI:

```bash
rocky "your task description here"
rocky --stats
rocky --list
```

Demo (isolated PKG, safe to experiment with):

```bash
python demo/seed.py          # seed starting state
python demo/run.py           # run all 5 tasks interactively
python demo/run.py --task 1  # run a single task
python demo/run.py --reset   # re-seed the PKG
```

## Key files

| File | Purpose |
|---|---|
| `rocky/cli.py` | CLI entry point, Q&A loop, output formatting |
| `rocky/teacher.py` | Rocky: Claude API calls for topic extraction, question generation, answer evaluation |
| `rocky/graph/store.py` | PKG: storage, confidence decay, classification (known/stale/new) |
| `graph_cli.py` | JSON-over-stdout wrapper used by the Claude Code skill integration (stays at root) |
| `demo/seed.py` | Populates `demo/demo_graph.json` with pre-aged topics |
| `demo/run.py` | Runs the 5-task demo scenario against the demo PKG |
| `demo/tasks.py` | The task list and notes on what each task tests |

## Data files

- `~/.rocky/knowledge_graph.json` — global PKG (JSON), follows the user across projects
- `demo/demo_graph.json` — demo PKG, gitignored, reset with `demo/run.py --reset`

## Environment

Requires `ANTHROPIC_API_KEY` in `.env` (see `.env.example`). Rocky uses `claude-sonnet-4-6` by default.

## Architecture (planned — see ROCKY_PLAN.md)

```
rocky/
├── cli.py          — entry point
├── teacher.py      — Rocky: LLM calls, Socratic Q&A
├── config.py       — configuration
├── graph/          — PKG subsystem (store, FSRS model, node)
├── capture/        — session tracking, hooks
├── export/         — Obsidian vault writer, CLI reports
└── llm/            — provider abstraction (Claude + Ollama)
```

The core loop: extract topics → classify each against PKG (known/stale/new) → Rocky generates reminder (stale) or runs Socratic Q&A (new) → PKG updated.

PKG node state uses a DSR model (Difficulty, Stability, Retrievability) per topic. Retrievability is the trigger threshold, not a simple confidence value.
