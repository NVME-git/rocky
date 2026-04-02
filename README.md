# Rocky

A developer tool that combats AI skill atrophy. Rocky watches what you delegate to AI agents, tracks what you understand in your **Personal Knowledge Graph (PKG)**, and quizzes you — via Socratic questioning — only when it matters.

## The problem

Engineers using AI agents ship faster but stop learning. When a major issue hits, they can't debug it because they never internalized the code. Rocky intercepts that pattern by quizzing you on the topics your agent is handling, updating your PKG as you go.

## How it works

1. You describe a task you're delegating to an AI agent
2. Rocky extracts the key technical topics from that task
3. Each topic is classified against your PKG: **known / stale / new**
   - **Known** → silent pass, you're good to go
   - **Stale** → Rocky gives you a brief contextual reminder
   - **New** → Rocky runs a Socratic Q&A loop focused on implications, not facts
4. Your PKG grows as you demonstrate understanding. Confidence decays over time so Rocky re-tests you on things you haven't touched in a while.

Rocky asks about consequences and trade-offs — "how does this affect your bulk insert job?" not "what is an index?"

## Installation

```bash
git clone <repo>
cd rocky
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
cp .env.example .env
# edit .env and add your ANTHROPIC_API_KEY
```

## Usage

```bash
# Describe a task before handing it to an agent
python pkg.py "add JWT authentication to my REST API"

# Show PKG stats
python pkg.py --stats

# List all topics in your PKG
python pkg.py --list
```

### Example session

```
 Rocky
 ─────────────────────────────

Task: add JWT authentication to my REST API

Analyzing topics...
  ✓ REST API design (94% confidence)

Rocky: New topic — JWT authentication
  Stateless token-based auth where the server signs a payload the client stores and sends back.

Q1. If your API issues a JWT on login and a user's account is suspended 10 minutes later,
    what happens to their in-flight requests — and how would you address it?
   > The token would still be valid until expiry since JWTs are stateless. You'd need a token
     blacklist or short expiry + refresh token pattern to handle revocation.

   Good — you've identified the core revocation problem and two real mitigations. The trade-off
   between short expiry (more auth overhead) and a blacklist (stateful, defeats some JWT benefits)
   is exactly the gotcha here.
   Added to your PKG.
```

## Your PKG

The Personal Knowledge Graph lives at `~/.rocky/graph.db`. Topics have:
- **retrievability** (0–1): current probability of recall, computed from stability and time elapsed
- **stability**: how long (in days) until retrievability drops to 90%
- **difficulty**: how hard this topic has been for you historically
- **kind**: `concept` | `pattern` | `implementation` — each has a different decay rate

The PKG is personal, global (follows you across projects), and never shared.

## Configuration

| Variable | Description |
|---|---|
| `ANTHROPIC_API_KEY` | Required. Get one at console.anthropic.com |

## Files

```
pkg.py          — CLI entry point and Q&A loop
engine.py       — Rocky: topic extraction, question generation, answer evaluation
graph.py        — PKG: storage, decay, and classification logic
graph_cli.py    — JSON-over-stdout interface for the Claude Code skill integration
```
