# Rocky

```
   ♫           ♪          ♫

     __|__
    /◉   ◉\         R  O  C  K  Y
    \ ─── /         Personal Knowledge Graph
     \_↑_/
    /|||||\          Stay sharp. Stay human.
```

Rocky is a CLI that keeps your technical knowledge sharp while AI agents handle the work. Every time your AI writes code, Rocky quizzes you on what just happened — not "what is JWT?" but "you're issuing tokens with a 15-minute expiry, what breaks when one expires mid-session?"

**[Full documentation →](https://nvme-git.github.io/rocky)**

---

## Install

```bash
cargo install --git https://github.com/NVME-git/rocky
```

Or download a binary from the [Releases](https://github.com/NVME-git/rocky/releases) page.

You need an `ANTHROPIC_API_KEY` in your environment, or [Ollama](https://ollama.com) running locally. See [Configuration](#configuration) for Ollama setup.

### Requirements

- A terminal
- An Anthropic API key **or** [Ollama](https://ollama.com) running locally

### Install with Cargo (recommended)

If you have Rust installed:

```bash
cargo install --git https://github.com/NVME-git/rocky
```

This downloads, compiles, and installs Rocky in one step. It will be available as `rocky` from anywhere on your system.

> **Don't have Rust?** Install it from [rustup.rs](https://rustup.rs) — it takes about 2 minutes.

### Download a prebuilt binary

Go to the [Releases page](https://github.com/NVME-git/rocky/releases) and download the binary for your operating system.

**Linux / macOS:**
```bash
chmod +x rocky
mv rocky ~/.local/bin/rocky
```

**Windows:** Move `rocky.exe` somewhere and add that folder to your PATH.

### Setting up your API key

Rocky needs to call an AI model to analyse topics and generate questions.

**Using Anthropic (Claude):**

1. Get an API key from [console.anthropic.com](https://console.anthropic.com)
2. Set it in your environment:

```bash
# Add to ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

Or create a `.env` file in your project directory.

**Using Ollama (free, runs locally):**

1. Install Ollama from [ollama.com](https://ollama.com)
2. Pull a model:
```bash
ollama pull qwen2.5-coder:7b
```
3. Create `~/.rocky/.rocky.toml`:
```toml
[llm]
provider = "ollama"
model = "qwen2.5-coder:7b"
```

### Verify the installation

```bash
rocky stats
```

You should see:

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  0
  Known:         0
  Fading:        0
  Gaps/weak:     0

  Quiz budget: 3/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  ♫ PKG is empty. Let us begin science, question?
```

### Set up the git hook

```bash
rocky install        # or: rocky install git
```

From now on, every `git commit` will trigger `rocky diff` automatically.

### Set up the Claude Code hook (optional)

```bash
rocky install claude
```

This writes the hook entry to `~/.claude/settings.json` automatically. It's silent — it won't interrupt your workflow.

---

## What it does

When you use AI tools to write code, you ship faster but stop practising. Rocky tracks a **Personal Knowledge Graph (PKG)** of everything you've learned, how well you know it, and how recently you used it. Knowledge fades — Rocky surfaces the right things at the right time.

### Why it exists

Every senior engineer knows the feeling: you stop writing something by hand, and six months later you can't remember how it works without looking it up. That's normal. That's how memory works.

What's new is the speed. AI tools don't just accelerate your output — they remove the need to think through problems entirely. You describe what you want, the AI writes it, you ship it. Fast. But the understanding that used to come from doing the work yourself? That doesn't happen anymore.

This is **AI skill atrophy** — and it compounds silently. You don't notice it until the day the AI gives you the wrong answer and you can't tell.

Rocky exists for one reason: **so you always know what your AI just built.**

### What makes it different

Most learning tools quiz you on definitions. Rocky doesn't care if you can define JWT. It cares whether you'd catch the bug.

Instead of asking "what is a refresh token?", Rocky asks:

> *You're issuing JWTs with a 15-minute expiry. A user is halfway through a checkout flow when their token expires. What happens — and how do you make the experience seamless without storing session state on the server, question?*

That's the kind of question that separates someone who read the docs from someone who's actually built with it. Rocky asks the second kind — every time.

### The five modes

- **Before a task** — describe it, get quizzed on what's new or fading
- **After a commit** — Rocky reads your diff and finds topics in the actual code
- **On demand** — run `rocky quiz` for a review session
- **During AI sessions** — Claude Code hook silently logs prompts; `rocky quiz` reviews them
- **Pre-commit review** — `rocky diff --staged` checks staged changes

---

## Quick start

```bash
# Before starting work on a task
rocky "add JWT authentication to my REST API"

# After committing — analyse the actual diff
rocky diff

# On-demand review session
rocky quiz

# Quiz yourself on a specific topic
rocky quiz "redis"

# See your knowledge graph
rocky ls

# Set up the git hook so Rocky runs after every commit
rocky install
```

### Your first session

1. **Describe your task:**
```bash
rocky "add user login with JWT tokens to my Express API"
```

2. **Answer the question** — Rocky evaluates whether you understand the implications, not just the definition.

3. **Check your knowledge graph:**
```bash
rocky ls
```

### Q&A options

At any question you can:
- **Type your answer** and press Enter
- **Press Enter** with nothing to skip (queues the topic for later)
- **Type `i`** to ignore the topic (useful for hallucinated topics)
- **Type `k`** if you already know this well (Rocky records it without a full Q&A)

---

## Commands

| Command | What it does |
|---|---|
| `rocky "task description"` | Analyse a task and quiz on new/fading topics |
| `rocky --after "message"` | Review topics from a completed task |
| `rocky diff` | Analyse your last commit's diff |
| `rocky diff HEAD~3` | Analyse a specific commit |
| `rocky diff --staged` | Analyse staged changes before committing |
| `rocky quiz` | On-demand review session |
| `rocky quiz <topic>` | Search and quiz on specific topics |
| `rocky ls` | Full topic list with recall, stability, difficulty |
| `rocky stats` | PKG summary (known / fading / gaps) |
| `rocky install` | Install git post-commit hook (default) |
| `rocky install claude` | Install Claude Code hook |
| `rocky uninstall` | Remove git post-commit hook |
| `rocky uninstall claude` | Remove Claude Code hook |
| `rocky export` | Write PKG to Markdown files |
| `rocky classify` | Assign taxonomy domains to existing topics |
| `rocky sync` | Commit PKG changes to git |
| `rocky sync --push` | Commit and push to remote |
| `rocky sync --init [url]` | Initialise PKG git repo |
| `rocky restore` | Rebuild PKG from pkg/pkg.json |
| `rocky config` | Show active configuration |
| `rocky delete "query"` | Remove topics from PKG |
| `rocky logs` | Show recent Claude Code prompts |
| `rocky edges` | List all edges in the implication graph |
| `rocky edges --stats` | Edge stats summary (totals by kind) |
| `rocky view` | Open interactive knowledge graph in the browser |
| `rocky hook` | Internal: called by Claude Code hook |

### `rocky "task description"`

Describe what you're about to work on. Rocky extracts the key topics and quizzes you on anything new or fading. No daily limits — you asked for it.

```bash
rocky "add pagination to my REST API"
rocky "set up CI/CD with GitHub Actions"
rocky "migrate the database from SQLite to Postgres"
```

### `rocky --after "message"`

Review topics from a task you already completed or a commit message you wrote.

```bash
rocky --after "added Redis caching to the user service"
```

### `rocky diff`

Analyse a git diff and quiz on topics found in the actual code changes.

```bash
rocky diff                 # analyse your last commit
rocky diff HEAD~3          # analyse a specific commit
rocky diff abc1234         # analyse by commit hash
rocky diff --staged        # analyse staged changes before committing
```

When triggered automatically (via git hook), daily limits and cooldown apply. When run manually, no limits.

### `rocky quiz`

On-demand review session. Works through three sources in priority order:

1. Topics you previously skipped (queued in `./.rocky`)
2. PKG topics with the lowest recall (most overdue for review)
3. New topics found in recent Claude Code prompts (last 24 hours)

```bash
rocky quiz               # general review
rocky quiz --hours 48    # look back further for prompt context
rocky quiz "redis"       # search and quiz on specific topics
```

Manual quiz calls always run — no daily limits.

### `rocky stats`

PKG summary showing total topics, known/fading/gaps breakdown, and quiz budget remaining.

### `rocky ls`

Full topic list with all metrics: recall, stability, difficulty, review count, last reviewed.

### `rocky install` / `rocky uninstall`

Install or remove hooks. Both default to `git` if no subcommand is given.

- `rocky install` / `rocky install git` — runs `rocky diff` after every `git commit`
- `rocky install claude` — silently logs Claude Code prompts
- `rocky uninstall` / `rocky uninstall git` — remove git hook
- `rocky uninstall claude` — remove Claude Code hook

### `rocky export`

Write all PKG topics to Markdown files. Rocky also exports automatically every time a topic is updated.

### `rocky classify`

Assign taxonomy domains to any topics that don't have one. The 13 domains: Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other.

### `rocky sync`

Commit pending PKG changes to git. See [Vault Sync & Backup](#vault-sync--backup).

```bash
rocky sync                                                # commit
rocky sync --push                                         # commit and push
rocky sync --init https://github.com/you/rocky-pkg.git   # init + set remote
rocky sync --status                                       # show changes
```

### `rocky restore`

Rebuild `graph.db` from `pkg/pkg.json`. Use when setting up Rocky on a new machine.

### `rocky delete "query"`

Search and remove topics from your PKG. Rocky shows matches and asks for confirmation.

### `rocky edges`

List all edges in the implication graph — the relationships Rocky has inferred between topics. Each edge has a kind, a strength, and a short description.

```bash
rocky edges                 # list all edges
rocky edges --stats         # summary: total count, most connected topic, breakdown by kind
```

Edge kinds: `implies`, `depends_on`, `conflicts_with`, `part_of`.

### `rocky view`

Open an interactive knowledge graph in your default browser. Nodes are colored by knowledge state (known/fading/gap) and grouped by domain. Click a node to see its edges and details.

```bash
rocky view
# ✓ Written to ~/.rocky/view.html
# → Opening in browser...
```

Rocky writes the graph to `~/.rocky/view.html` and opens it automatically. The graph uses D3.js force simulation — drag nodes, zoom in, filter by domain, and search for topics by name.

---

## The quiz experience

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Task: add JWT authentication to my REST API

  Analyzing topics...

  Rocky: New topic — JWT authentication
  Stateless token-based auth where the server signs a payload
  the client stores and sends back.

  Q1. You're issuing JWTs with a 15-minute expiry — when a token
      expires mid-session, what needs to happen on both sides for
      the experience to feel seamless, question?

  > The client stores a refresh token, sends it to a /refresh
    endpoint when the access token expires, gets a new access
    token without the user re-logging in. Server validates the
    refresh token against a database so it can be revoked.

  ♫ Fist my bump, friend! Is correct!
  ✓ JWT authentication added to PKG.
```

Questions end in ", question?" — Rocky's voice from *Project Hail Mary*. Set `personality = false` in config to get plain output.

---

## The knowledge model

Rocky tracks a **Personal Knowledge Graph (PKG)** — a local database of every topic you've encountered. Each topic has a retrievability score: a number between 0 and 1 that estimates how likely you are to recall it right now.

Knowledge decays. A topic you understood deeply three months ago might be at 65% today. Rocky knows this, and it surfaces things before they fade below the threshold — not after.

| Term | What it means |
|---|---|
| **PKG** | Personal Knowledge Graph — your local database of topics |
| **Known** | Recall is strong (90%+) — Rocky stays quiet |
| **Fading** | Recall is slipping (70–90%) — Rocky gives a reminder |
| **Gap** | Recall is low or topic is new — Rocky asks a question |
| **Retrievability** | Rocky's estimate of how likely you are to remember something right now |
| **Stability** | How deeply embedded the topic is — higher stability means slower decay |
| **Domain** | One of 13 taxonomy categories (Language, Auth, Database, DevOps, etc.) |

Rocky uses a memory model similar to Anki (spaced repetition). Topics you know well decay slowly. Topics you barely know decay fast. Over time, Rocky surfaces the right things at the right moments without spamming you.

---

## How Rocky works

Rocky has five modes of operation, each with different rules about when it quizzes you and what limits apply.

### Scenario 1: Manual task (`rocky "your task"`)

You describe what you're about to work on. Rocky extracts the topics, checks your PKG, and runs Socratic Q&A on anything new. No cooldown, no daily cap — you asked for it.

**Flow:**
1. Run `rocky "build a caching layer"`
2. Extract 2–5 key topics from task description
3. For each topic, classify against PKG:
   - **Known** (≥90%) → Mark encountered, move on
   - **Stale** (70–90%) → Write 2–3 sentence reminder, update PKG
   - **New / Gap** (<70%) → Generate Socratic question about implications
4. User answers → Record in PKG with appropriate confidence
5. Summary printed

### Scenario 2: Git commit hook (`rocky diff`)

After every `git commit`, Rocky analyses the diff for topics in your code. Cooldown and daily budget are enforced. Topics that can't be quizzed right now are queued in `./.rocky` for the next `rocky quiz`.

### Scenario 3: Claude Code hook (`rocky hook`)

Each prompt is silently logged to `./.rocky`. No quiz happens — this is just capture. Entries older than 24 hours are auto-deleted.

### Scenario 4: `rocky quiz`

No limits apply. Rocky works through a priority queue: queued topics first → PKG topics most overdue → new topics from recent prompts.

### Scenario 5: Pre-commit review (`rocky diff --staged`)

Review staged changes before committing. Behaves like the manual flow — no limits.

### How the PKG classifies topics

Every topic has a **retrievability score** — an estimate of how likely you are to recall it right now:
- ≥ 90% → **Known** — skipped automatically
- 70–90% → **Stale** — printed as reminder
- < 70% → **Gap** — full Socratic Q&A

Higher stability means the topic decays slower — if you've demonstrated solid understanding multiple times, Rocky won't ask you about it again for weeks.

### Domain taxonomy

Every topic is assigned to one of 13 domains:

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

---

## Configuration

Rocky reads `~/.rocky/.rocky.toml` (global) and `./.rocky.toml` (project override).

```toml
[llm]
provider = "ollama"          # "claude" (default) or "ollama"
model = "qwen2.5-coder:7b"
ollama_base_url = "http://localhost:11434"

[session]
daily_budget = 3             # max auto-triggered quizzes per day
min_gap_minutes = 120        # minimum gap between auto sessions

[export]
pkg_dir = "~/Documents/Obsidian/MyVault/rocky"

[ui]
personality = true           # Rocky's voice and ASCII art

[sync]
enabled = false              # opt-in PKG version control
auto_commit = true           # commit after each session
commit_visible = true        # print what was committed
remote = "origin"
branch = "main"
remind_push_sessions = 5     # remind to push every 5 sessions
remind_push_days = 0         # OR every N calendar days
```

### LLM settings

**Using Claude (Anthropic):**

```toml
[llm]
provider = "claude"
model = "claude-sonnet-4-6"
```

Requires `ANTHROPIC_API_KEY` set in your environment or a `.env` file.

**Using Ollama (local, free):**

```toml
[llm]
provider = "ollama"
model = "qwen2.5-coder:7b"
ollama_base_url = "http://localhost:11434"
```

| Your GPU VRAM | Recommended model |
|---|---|
| 6 GB | `qwen2.5-coder:7b` |
| 4 GB | `qwen2.5:3b` or `phi4-mini` |
| No GPU / CPU only | `qwen2.5:3b` (slow) |

### Session settings

| Setting | Default | What it does |
|---|---|---|
| `daily_budget` | 3 | Max quizzes Rocky will auto-trigger per day (manual calls bypass this) |
| `min_gap_minutes` | 120 | Minimum minutes between auto-triggered quizzes |

### Export settings

| Setting | Default | What it does |
|---|---|---|
| `pkg_dir` | `~/.rocky/pkg/` | Where Rocky writes Markdown notes and `pkg.json` |

### UI settings

| Setting | Default | What it does |
|---|---|---|
| `personality` | `true` | Rocky the alien's voice, ASCII art, and milestone celebrations |

### Sync settings

| Setting | Default | What it does |
|---|---|---|
| `enabled` | `false` | Enable PKG git tracking |
| `auto_commit` | `true` | Auto-commit after each session (when enabled) |
| `commit_visible` | `true` | Print commit message after auto-commit |
| `remote` | `"origin"` | Git remote name |
| `branch` | `"main"` | Branch to push to |
| `remind_push_sessions` | `5` | Remind to push every N sessions (0 = off) |
| `remind_push_days` | `0` | Remind every N calendar days (0 = off) |

### Where Rocky stores data

| Path | What it is |
|---|---|
| `~/.rocky/graph.db` | Your PKG — all topics, recall scores, review history |
| `~/.rocky/.rocky.toml` | Your global config |
| `~/.rocky/pkg/` | Markdown notes + `pkg.json` backup |
| `~/.rocky/pkg/pkg.json` | Full PKG export for backup and cross-machine restore |
| `./.rocky` | Per-project prompt log (only in hooked projects) |

`graph.db` is never tracked by git. Everything in `pkg/` is tracked when sync is enabled.

---

## Vault sync & backup

Rocky can version-control your PKG and sync it across machines:

```bash
# Enable in ~/.rocky/.rocky.toml
[sync]
enabled = true

# Init the git repo and set a remote
rocky sync --init https://github.com/you/rocky-pkg.git

# Rocky auto-commits after each session. Push when ready:
rocky sync --push

# On a new machine:
git clone https://github.com/you/rocky-pkg.git ~/.rocky
rocky restore
```

### What gets tracked

```
~/.rocky/
  pkg/                    ← tracked by git
    pkg.json              ← full PKG backup (nodes + session state)
    Rocky Dashboard.md    ← Dataview dashboard
    Rocky Review Queue.md ← review queue
    Language/             ← topics grouped by domain
      rust-ownership.md
    Database/
      redis-ttl-expiry.md
    Auth/
      jwt-authentication.md
  graph.db                ← NOT tracked (excluded by .gitignore)
  .gitignore              ← auto-created by Rocky
```

`graph.db` is the live runtime database. It's a binary file, bad for git, and always rebuildable from `pkg.json`.

### Cross-machine workflow

**Machine A** (your main machine):
```bash
rocky sync --init https://github.com/you/rocky-pkg.git
# do quizzes, auto-commits happen
rocky sync --push
```

**Machine B** (new machine):
```bash
git clone https://github.com/you/rocky-pkg.git ~/.rocky
rocky restore          # rebuilds graph.db from pkg/pkg.json
rocky stats            # all your topics are back
```

### Push reminders

Rocky tracks how many sessions have passed since your last push and reminds you:

```
♫ Rocky: 5 sessions unsynced — consider `rocky sync --push` to back up, question?
```

Configure: `remind_push_sessions = 5` or `remind_push_days = 7` (pick one, set other to 0).

---

## Obsidian integration

```bash
rocky export
```

Writes one Markdown note per topic into your PKG directory, grouped by domain (`Language/`, `Database/`, `Auth/`, etc.). Rocky auto-creates dashboard pages with Dataview queries and wikilinks between related topics. Point `pkg_dir` at your Obsidian vault to view everything there.

### Setup

1. Set `pkg_dir` in your config to a folder inside your Obsidian vault
2. Run `rocky export`
3. Install the Dataview plugin in Obsidian

### Vault structure

```
pkg/
  Rocky Dashboard.md       ← auto-created overview dashboard
  Rocky Review Queue.md    ← auto-created review queue
  pkg.json                 ← full PKG backup (for sync/restore)
  Language/
    rust-ownership.md
    python-decorators.md
  Database/
    redis-ttl-expiry.md
    sql-indexes.md
  Auth/
    jwt-authentication.md
  ...
```

### What the notes look like

Each topic becomes a file like `Auth/jwt-authentication.md`:

```markdown
---
rocky_id: jwt-authentication
rocky_kind: pattern
rocky_domain: Auth
rocky_difficulty: 0.3
rocky_stability: 8.5
rocky_retrievability: 0.94
rocky_last_reviewed: 2026-04-03
rocky_last_encountered: 2026-04-03
rocky_review_count: 3
rocky_days_since_review: 0
tags: [rocky/node, rocky/kind/pattern, rocky/domain/Auth]
---

# JWT authentication

Stateless token-based auth where the server signs a payload the client stores and sends back.

## Contexts
- add user login with JWT tokens to my Express API

## See also
- [[token-expiry-handling]]
- [[httponly-cookie-security]]
```

### Auto-created dashboards

- **Rocky Dashboard.md** — overview of your entire PKG with by-domain breakdown
- **Rocky Review Queue.md** — gaps and fading topics sorted by urgency

Both use Dataview queries and update automatically.

### Custom Dataview queries

**Topics that need attention:**
```
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed", rocky_kind AS "Kind"
FROM #rocky/node
WHERE rocky_retrievability < 0.7
SORT rocky_retrievability ASC
```

**Strongest topics:**
```
TABLE rocky_retrievability AS "Recall %", rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability >= 0.9
SORT rocky_review_count DESC
```

**Topics by domain:**
```
TABLE rocky_retrievability AS "Recall %", rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/domain/Auth
SORT rocky_retrievability ASC
```

**Everything sorted by recall:**
```
TABLE rocky_retrievability AS "Recall", rocky_domain AS "Domain", rocky_days_since_review AS "Days ago"
FROM #rocky/node
SORT rocky_retrievability ASC
```

### Graph view

Filter the graph to `#rocky/node` to see only your PKG. Filter to `#rocky/domain/Auth` to zoom into a specific area.

---

## Rocky the alien

```
      ♫
   __|__
  /◉   ◉\
  \ ─── /
   \_↑_/
  /|||||\
```

Rocky has a personality based on Rocky the alien from Andy Weir's *Project Hail Mary* — enthusiastic, direct, and genuinely rooting for you.

- `♫ Fist my bump, friend! Is correct!`
- `♫ Excite excite excite! Friend get it!`
- `♫ Is okay! Rocky also not know at first!`
- `♫ We are crew. We solve together.`

Questions end with ", question?" — Rocky's way of asking. Set `personality = false` in `[ui]` config for plain output.

---

## Claude Code hook

Log every prompt you send to Claude Code so `rocky quiz` has data to work with:

```bash
rocky install claude
```

This writes the hook entry to `~/.claude/settings.json` automatically. Remove it with `rocky uninstall claude`.

---

## License

MIT
