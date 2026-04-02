# PKG Demo

Runs a scripted scenario — a developer building a web app and delegating tasks to an AI agent — to show how the knowledge graph evolves across sessions.

Uses `demo/demo_graph.json` as an isolated graph. Your real `knowledge_graph.json` is never touched.

## Quick start

```bash
# From the project root, with your venv active:
python demo/seed.py       # populate the starting graph
python demo/run.py        # run all 5 tasks interactively
```

## What to observe

The demo graph starts with three topics pre-seeded at different ages:

| Topic | State at start |
|---|---|
| REST API design | **known** — reviewed 5 days ago |
| SQL query optimization | **stale** — 45 days ago (implementation decays in 30d half-life) |
| HTTP caching | **stale** — 120 days ago (concept, but confidence was only 55%) |

### Graph evolution across tasks

| Task | Expected behaviour |
|---|---|
| 1. PostgreSQL + connection pooling | REST API → silent pass. SQL optimization → reminder. Connection pooling → new Q&A |
| 2. Redis caching | HTTP caching surfaces as stale → reminder. Redis → new Q&A |
| 3. Alembic migrations | Likely all new. Tests whether SQL knowledge carries over conceptually |
| 4. Index + query plan optimization | SQL optimization may now be refreshed from task 1, or re-quizzed if still weak |
| 5. JWT + RBAC | Almost certainly new — full Socratic loop |

## Commands

```bash
python demo/run.py --list          # preview all tasks and what they test
python demo/run.py --task 3        # run a single task (1-indexed)
python demo/run.py --reset         # re-seed the graph back to starting state
```
