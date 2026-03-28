# PKG — Personal Knowledge Graph

A CLI tool that combats AI skill atrophy in developers.

## The problem

Engineers using AI agents ship faster but stop learning. When a major issue hits, they can't debug it because they never internalized the code. PKG intercepts that pattern by quizzing you — in real time, on the actual topics your agent is about to handle.

## How it works

1. You describe a task you're delegating to an AI agent
2. PKG extracts the key technical topics from that task (via Claude)
3. Each topic is classified against your personal knowledge graph: **known / stale / new**
   - **Known** → silent pass, you're good to go
   - **Stale** → a brief contextual reminder before you proceed
   - **New** → a Socratic Q&A loop focused on implications, not facts
4. Your graph grows as you demonstrate understanding. Confidence decays over time so you get re-tested on things you haven't touched in a while.

Questions target consequences and trade-offs — "how does this affect your bulk insert job?" not "what is an index?"

## Installation

```bash
git clone <repo>
cd pkg
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
cp .env.example .env
# edit .env and add your ANTHROPIC_API_KEY
```

## Usage

```bash
# Analyze a task before handing it to an agent
python pkg.py "add JWT authentication to my REST API"

# Show knowledge graph stats
python pkg.py --stats

# List all topics with confidence levels
python pkg.py --list
```

### Example session

```
 PKG — Personal Knowledge Graph
 ─────────────────────────────

Task: add JWT authentication to my REST API

Analyzing topics...
  ✓ REST API design (94% confidence)

New topic: JWT authentication
  Stateless token-based auth where the server signs a payload the client stores and sends back.

Q1. If your REST API issues a JWT on login and a user's account is suspended 10 minutes later,
    what happens to their in-flight requests — and how would you address it?
   > The token would still be valid until expiry since JWTs are stateless. You'd need a token
     blacklist or short expiry + refresh token pattern to handle revocation.

   Good — you've identified the core revocation problem and two real mitigations. The trade-off
   between short expiry (more auth overhead) and a blacklist (stateful, defeats some JWT benefits)
   is exactly the gotcha here.
   Got it. Added to your knowledge graph.
```

## Knowledge graph

The graph is stored locally in `knowledge_graph.json`. Topics have:
- **confidence** (0–1): how well you understand the topic
- **kind**: `concept` | `pattern` | `implementation` — each decays at a different rate
- **decay**: confidence fades over time (half-life: concepts 180d, patterns 90d, implementations 30d)

The graph is personal and grows from your actual work — it is not shared or synced anywhere.

## Configuration

| Variable | Description |
|---|---|
| `ANTHROPIC_API_KEY` | Required. Get one at console.anthropic.com |

## Files

```
pkg.py          — CLI entry point and Q&A loop
engine.py       — Claude API calls: topic extraction, question generation, evaluation
graph.py        — Knowledge graph storage and confidence decay logic
graph_cli.py    — JSON-over-stdout interface for use by Claude Code skill integration
```
