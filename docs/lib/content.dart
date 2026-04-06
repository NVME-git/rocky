// All documentation content as markdown strings.
// Each constant corresponds to one section of the docs.

const kIntroduction = r'''
# What is Rocky?

Rocky is a command-line tool that keeps your technical knowledge sharp while AI agents handle more and more of your work.

---

## Why it exists

Every senior engineer knows the feeling: you stop writing something by hand, and six months later you can't remember how it works without looking it up. That's normal. That's how memory works.

What's new is the speed. AI tools don't just accelerate your output — they remove the need to think through problems entirely. You describe what you want, the AI writes it, you ship it. Fast. But the understanding that used to come from doing the work yourself? That doesn't happen anymore.

This is **AI skill atrophy** — and it compounds silently. You don't notice it until the day the AI gives you the wrong answer and you can't tell.

Rocky exists for one reason: **so you always know what your AI just built.**

---

## What makes it different

Most learning tools quiz you on definitions. Rocky doesn't care if you can define JWT. It cares whether you'd catch the bug.

Instead of asking "what is a refresh token?", Rocky asks:

> *You're issuing JWTs with a 15-minute expiry. A user is halfway through a checkout flow when their token expires. What happens — and how do you make the experience seamless without storing session state on the server, question?*

That's the kind of question that separates someone who read the docs from someone who's actually built with it. Rocky asks the second kind — every time.

---

## How it works

Rocky integrates into the moments where understanding matters most. There are five modes, each with its own rules about when it runs and what limits apply — see **How It Works** for the full detail.

### Before a task

Tell Rocky what you're about to build. Rocky extracts the key topics, checks what you already know, and asks a focused question on anything new or fading — before you've touched a single line of code.

```bash
rocky "add rate limiting to the API using Redis"
```

Rocky finds: *Redis sorted sets, token bucket algorithm, atomic Lua scripting.*
You know the first two. It asks you about Lua scripting in Redis — specifically, why you'd need it and what breaks without it.

### After a commit

Every `git commit` triggers `rocky diff`. Rocky reads your actual code changes — not the commit message — and surfaces the topics that live in what you just shipped.

```bash
git commit -m "add Redis rate limiter"
# Rocky runs automatically:
# ~ Redis Lua scripting  (recall fading to 71%)
#   Reminder: Lua scripts in Redis run atomically — the whole script or nothing.
#   This is why you use them for rate limiting: checking and incrementing the
#   counter must be a single operation, or two requests can both pass the check
#   before either increments.
```

### On demand

Run a review session any time. Rocky works through what's most overdue, what you've recently skipped, and new topics from your recent AI sessions.

```bash
rocky quiz               # general review
rocky quiz "redis"       # targeted — search and pick topics to drill
```

### During AI sessions

If you use Claude Code, Rocky silently logs every prompt you send. No interruption. Run `rocky quiz` later and Rocky knows exactly what topics your AI handled for you today.

```bash
rocky install claude     # one-time setup
# ... work normally ...
rocky quiz               # end-of-day review
```

### Pre-commit review

Check your staged changes before you commit. Useful when you've been working with an AI and want to make sure you actually understand what's about to land.

```bash
rocky diff --staged
```

---

## The knowledge model

Rocky tracks a **Personal Knowledge Graph (PKG)** — a local database of every topic you've encountered. Each topic has a retrievability score: a number between 0 and 1 that estimates how likely you are to recall it right now.

Knowledge decays. A topic you understood deeply three months ago might be at 65% today. Rocky knows this, and it surfaces things before they fade below the threshold — not after.

When you answer well, stability increases and the topic decays slower. When you struggle, Rocky comes back sooner. Over time, the PKG reflects your actual knowledge — not the version of yourself that existed when you first learned something.

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

## Key concepts

| Term | What it means |
|---|---|
| **PKG** | Personal Knowledge Graph — your local database of topics |
| **Known** | Recall is strong (90%+) — Rocky stays quiet |
| **Fading** | Recall is slipping (70–90%) — Rocky gives a reminder |
| **Gap** | Recall is low or topic is new — Rocky asks a question |
| **Retrievability** | Rocky's estimate of how likely you are to remember something right now |
| **Stability** | How deeply embedded the topic is — higher stability means slower decay |
| **Domain** | One of 13 taxonomy categories (Language, Auth, Database, DevOps, etc.) |
| **Edge** | A relationship between two topics in the PKG — generated automatically by Rocky after new topics are added |
| **Edge kind** | The type of relationship: `implies`, `depends_on`, `conflicts_with`, or `part_of` |
| **Cross-concept question** | A question that bridges two related topics — asked when Rocky detects a relevant edge and both topics have strong recall |
''';

const kInstallation = r'''
# Installation

Rocky is a single binary with no external dependencies. You download it and it just works.

## Requirements

- A terminal
- An Anthropic API key **or** [Ollama](https://ollama.com) running locally

## Option 1: Install with Cargo (recommended)

If you have Rust installed:

```bash
cargo install --git https://github.com/NVME-git/rocky
```

This downloads, compiles, and installs Rocky in one step. It will be available as `rocky` from anywhere on your system.

> **Don't have Rust?** Install it from [rustup.rs](https://rustup.rs) — it takes about 2 minutes.

## Option 2: Download a prebuilt binary

Go to the [Releases page](https://github.com/NVME-git/rocky/releases) and download the binary for your operating system.

**Linux / macOS:**
```bash
chmod +x rocky
mv rocky ~/.local/bin/rocky
```

**Windows:** Move `rocky.exe` somewhere and add that folder to your PATH.

---

## Setting up your API key

Rocky needs to call an AI model to analyse topics and generate questions.

### Using Anthropic (Claude)

1. Get an API key from [console.anthropic.com](https://console.anthropic.com)
2. Set it in your environment:

```bash
# Add to ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

Or create a `.env` file in your project directory.

### Using Ollama (free, runs locally)

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

See the **Configuration** page for model recommendations by GPU VRAM.

---

## Verify the installation

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

---

## Set up the git hook

In your project directory, run:

```bash
rocky install        # or: rocky install git
```

You'll see:

```
   ♫           ♪          ♫

     __|__
    /◉   ◉\         R  O  C  K  Y
    \ ─── /         Personal Knowledge Graph
     \_↑_/
    /|||||\          Stay sharp. Stay human.


  ✓ git post-commit hook installed — .rocky added to .gitignore

  Rocky will run after every commit in this repo.
  Use  rocky quiz  for an on-demand session anytime.
```

From now on, every `git commit` will trigger `rocky diff` automatically.

---

## Set up the Claude Code hook (optional)

If you use Claude Code, Rocky can silently log your prompts so `rocky quiz` has data to review:

```bash
rocky install claude
```

This writes the hook entry to `~/.claude/settings.json` automatically. It's silent — it won't interrupt your workflow.

---

You're ready. Head to **Quick Start** for your first session.
''';

const kQuickstart = r'''
# Quick Start

This walks you through your first session with Rocky in about 5 minutes.

---

## Step 1: Describe what you're about to work on

Before you start a task — before you open your editor or ask your AI assistant anything — tell Rocky what you're doing:

```bash
rocky "add user login with JWT tokens to my Express API"
```

Rocky analyses the task, checks your PKG, and quizzes you on anything new or fading:

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Task: add user login with JWT tokens to my Express API

  Analyzing topics...

  Rocky: New topic — JWT authentication
    Stateless token-based auth where the server signs a payload the
    client stores and sends back.

  Q1. You're issuing JWTs with a 15-minute expiry — when a user's token
      expires mid-session, what needs to happen on both sides for the
      experience to feel seamless, question?
  > 
```

---

## Step 2: Answer the question

Type your answer and press Enter. Rocky evaluates whether you understand the implications — not just the definition.

```
  > The client needs to store a refresh token separately. When the access
    token expires, the client sends the refresh token to get a new one
    without making the user log in again. The server validates the refresh
    token against a database so it can be revoked.

  ♫ Fist my bump, friend! Is correct!

  Good — you've covered the refresh flow and revocation. One thing worth
  double-checking: refresh tokens should be stored in httpOnly cookies,
  not localStorage, to prevent XSS from stealing them.

  ✓ JWT authentication added to PKG.
```

### Q&A options

At any question you can:

- **Type your answer** and press Enter
- **Press Enter** with nothing to skip (queues the topic for later)
- **Type `i`** to ignore the topic (useful when Rocky picks up a hallucinated or irrelevant topic)
- **Type `k`** if you already know this well (Rocky records it without a full Q&A)

---

## Step 3: Check your knowledge graph

After a few sessions, see what you know:

```bash
rocky ls
```

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Topic                           Kind           Recall         Stab   Diff  Reviews  Last Reviewed
  ────────────────────────────────────────────────────────────────────────────────────────────────
  JWT authentication              pattern        ██████████ 97%  8.2    0.3   3        2026-04-03
  httpOnly cookie security        concept        ████████░░ 81%  5.1    0.4   2        2026-03-28
  SQL injection prevention        pattern        ██████░░░░ 63%  3.0    0.5   1        2026-03-10
  database indexing               implementation ████░░░░░░ 42%  1.8    0.6   1        2026-02-15
```

- **Recall** — how likely you are to remember this right now
- **Stab** (stability) — how deeply embedded it is; higher means slower decay
- **Diff** (difficulty) — how hard you've found this historically
- **Reviews** — how many times you've been quizzed on this

Green = solid, yellow = fading, red = needs attention.

---

## Step 4: Set up the git hook (optional but recommended)

This makes Rocky automatically run after every commit, analysing the actual code changes:

```bash
cd your-project
rocky install        # installs the git hook (default)
```

From now on, every `git commit` triggers `rocky diff` automatically.

---

## Step 5: Quiz yourself on recent AI-assisted work

If you use Claude Code with the hook set up, Rocky logs your prompts in the background. Run this to review what topics came up:

```bash
rocky quiz
```

### Quiz on a specific topic

```bash
rocky quiz "redis"
```

Rocky searches your PKG and queued topics for anything matching "redis", shows you the options, and lets you pick which ones to quiz:

```
  Matching topics for "redis":

  [1]  Redis TTL expiry           (gap    · 38% recall)
       How Redis handles key expiration and its effect on cache consistency.
  [2]  Redis pub/sub              (fading · 74% recall)
       Event-driven messaging with Redis channels.
  [3]  Redis cluster sharding     (known  · 91% recall)

  Select topics to quiz (e.g. 1,2 or all, or Enter to cancel):
  > 1,2
```

---

## What happens over time

Rocky uses a memory model similar to Anki (spaced repetition). Topics you know well decay slowly. Topics you barely know decay fast. Over time, Rocky surfaces the right things at the right moments without spamming you.

By default, Rocky runs a maximum of 3 quizzes per day via automatic triggers (git hook, Claude Code hook), with a 2-hour gap between them. Manual `rocky quiz` calls always run — no limits.
''';

const kCommands = r'''
# Commands

## `rocky "task description"`

Describe what you're about to work on. Rocky extracts the key topics and quizzes you on anything new or fading.

```bash
rocky "add pagination to my REST API"
rocky "set up CI/CD with GitHub Actions"
rocky "migrate the database from SQLite to Postgres"
```

No daily limits — you asked for it.

---

## `rocky --after "message"`

Review topics from a task you already completed or a commit message you wrote.

```bash
rocky --after "added Redis caching to the user service"
```

Useful when you worked with an AI and want to review what just got built, without a diff.

---

## `rocky diff`

Analyse a git diff and quiz on topics found in the actual code changes.

```bash
# Analyse your last commit
rocky diff

# Analyse a specific commit
rocky diff HEAD~3
rocky diff abc1234

# Analyse staged changes before committing
rocky diff --staged
```

This is more powerful than `--after` — Rocky reads the actual code, not just the commit message.

When triggered automatically (via git hook), daily limits and cooldown apply. When run manually, no limits.

---

## `rocky quiz`

On-demand review session. Works through three sources in priority order:

1. Topics you previously skipped (queued in `./.rocky`)
2. PKG topics with the lowest recall (most overdue for review)
3. New topics found in recent Claude Code prompts (last 24 hours)

```bash
rocky quiz

# Look back further for prompt context
rocky quiz --hours 48
```

Manual quiz calls always run — no daily limits.

### Quiz on a specific topic

Search your PKG and queued topics, then select which to quiz:

```bash
rocky quiz "redis"
rocky quiz "auth"
```

```
  Matching topics for "redis":

  [1]  Redis TTL expiry           (gap    · 38% recall)
  [2]  Redis pub/sub              (fading · 74% recall)
  [3]  Redis cluster sharding     (known  · 91% recall)

  Select topics to quiz (e.g. 1,2 or all, or Enter to cancel):
  > 1,2
```

You can select individual topics, a comma-separated list, or `all`.

---

## Q&A options

During any question, you have these choices:

| Input | What happens |
|---|---|
| Type your answer + Enter | Rocky evaluates and gives feedback |
| Enter (blank) | Skip — topic is queued for later |
| `i` | Ignore — dismiss the topic entirely (useful for hallucinated topics) |
| `k` | Mark as known — Rocky records it without a full Q&A |

---

## `rocky stats`

PKG summary.

```bash
rocky stats
```

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  24
  Known:         15
  Fading:         6
  Gaps/weak:      3

  Quiz budget: 2/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  Edges: 18 total  ·  Most connected: JWT authentication (4 edges)

  ♫ Good progress, friend. Keep science going.
```

---

## `rocky ls`

Full topic list with all metrics.

```bash
rocky ls
```

```
  Topic                           Kind           Recall         Stab   Diff  Reviews  Last Reviewed
  ────────────────────────────────────────────────────────────────────────────────────────────────
  JWT authentication              pattern        ██████████ 97%  8.2    0.3   3        2026-04-03
  Redis TTL expiry                implementation ██████░░░░ 63%  3.0    0.5   1        2026-03-10
  database indexing               concept        ████░░░░░░ 42%  1.8    0.6   1        2026-02-15
```

Columns:
- **Recall** — retrievability score (probability of recall right now)
- **Stab** — stability (how embedded; higher = decays slower)
- **Diff** — difficulty (how hard you've historically found this)
- **Reviews** — number of times quizzed

---

## `rocky install` / `rocky uninstall`

Install or remove hooks. Both commands default to `git` if no subcommand is given.

### Git hook (default)

Runs `rocky diff` automatically after every `git commit` in the current repo.

```bash
rocky install          # same as: rocky install git
rocky install git

rocky uninstall        # same as: rocky uninstall git
rocky uninstall git
```

`rocky install git` also adds `.rocky` to `.gitignore` so your local prompt log isn't committed.

### Claude Code hook

Silently logs every prompt you send to Claude Code so `rocky quiz` has data to review. Writes the hook entry to `~/.claude/settings.json`.

```bash
rocky install claude
rocky uninstall claude
```

After installing, every Claude Code prompt in any project is logged. Run `rocky quiz` at any time to review what topics came up.

---

## `rocky export`

Write all PKG topics to your Obsidian PKG as Markdown files.

```bash
rocky export
```

Rocky also exports automatically every time a topic is updated. See **Obsidian Integration**.

---

## `rocky classify`

Assign taxonomy domains to any topics that don't have one. Useful for topics created before domain support was added.

```bash
rocky classify
# Classifying 8 topics...
# ✓ Redis TTL expiry → Database
# ✓ Rust ownership model → Language
# ✓ JWT authentication → Auth
# ...
# Run  rocky export  to update PKG files.
```

The 13 domains Rocky uses: Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other.

---

## `rocky sync`

Commit any pending PKG changes to git. Shows what changed.

```bash
rocky sync
# ✓ Rocky: 15/24 known — 2026-04-04
```

```bash
# Commit and push to the configured remote
rocky sync --push

# Initialise git repo (optionally set a remote at the same time)
rocky sync --init
rocky sync --init https://github.com/you/rocky-pkg.git

# Show PKG git status
rocky sync --status
```

See **Vault Sync & Backup** for full setup guide.

---

## `rocky restore`

Rebuild `graph.db` from `pkg/pkg.json`. Use this when setting up Rocky on a new machine after cloning your PKG.

```bash
git clone https://github.com/you/rocky-pkg.git ~/.rocky
rocky restore
# ✓ Restored 42 topics from ~/.rocky/pkg/pkg.json
```

---

## `rocky delete "query"`

Search your PKG and remove topics that are no longer relevant.

```bash
rocky delete "jwt"
rocky delete "react hooks"
```

Rocky shows all matching topics and asks you to confirm before deleting. The corresponding PKG file is also removed.

---

## `rocky config`

Show what config Rocky is currently using.

```bash
rocky config
```

---

## `rocky logs`

Show recent prompts logged in this project (from the Claude Code hook).

```bash
rocky logs
```

---

## `rocky edges`

List all edges in the implication graph — the relationships Rocky has inferred between topics in your PKG.

```bash
rocky edges
```

```
  SOURCE                         TARGET                         KIND               STR   DESCRIPTION
  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
  JWT authentication             token expiry handling          implies            0.90  ...
  Redis TTL expiry               cache invalidation             depends_on         0.75  ...
  SQL indexes                    query optimisation             implies            0.85  ...
```

```bash
# Edge stats summary
rocky edges --stats
```

```
  ◈ Edge Stats

  Total edges:           12
  Avg strength:          0.78
  Most connected:        JWT authentication (5 edges)

  By kind:               count    avg str
    implies              7        0.82
    depends_on           3        0.74
    conflicts_with       1        0.60
    part_of              1        0.80
```

Edge kinds: `implies`, `depends_on`, `conflicts_with`, `part_of`.

---

## `rocky view`

Open an interactive knowledge graph in your default browser. Nodes are colored by knowledge state (known/fading/gap) and sized by stability.

```bash
rocky view
# ✓ Written to ~/.rocky/view.html
# → Opening in browser...
```

Rocky writes the graph to `~/.rocky/view.html` and opens it automatically.

**Graph** — D3.js force simulation. Drag nodes, zoom in/out, filter by domain, search topics by name. Click any node to open a detail panel showing retrievability, stability, review history, and all connected edges.

**Timeline scrubber** — a range slider below the controls lets you rewind your knowledge graph to any point in time. As you scrub backward, nodes dim and disappear (topics you hadn't learned yet). Scrub forward to watch them light up — your personal growth, visualised. The current node count and date are shown next to the scrubber.

---

## `rocky hook`

Called automatically by the Claude Code hook — logs the prompt to `./.rocky`. You don't run this manually.
''';

const kConfiguration = r'''
# Configuration

Rocky looks for config files in two places, applied in this order (later overrides earlier):

1. `~/.rocky/.rocky.toml` — your global settings, applies everywhere
2. `./.rocky.toml` — project-level override, only applies in that folder

If neither exists, Rocky uses sensible defaults.

---

## Full example

```toml
[llm]
provider = "ollama"          # "claude" (default) or "ollama"
model = "qwen2.5-coder:7b"
ollama_base_url = "http://localhost:11434"

[session]
daily_budget = 3
min_gap_minutes = 120

[export]
pkg_dir = "~/Documents/Obsidian/MyVault/rocky"

[ui]
personality = true           # Rocky's voice and ASCII art (default: true)

[edges]
reuse = "off"                # "off" = always reuse, "14d" = 14-day cooldown, "5s" = 5-session cooldown

[sync]
enabled = false              # opt-in — enable PKG version control
auto_commit = true           # commit after each quiz/task/diff session
commit_visible = true        # print what was committed (set false to silence)
remote = "origin"
branch = "main"
remind_push_sessions = 5     # remind to push every N sessions (0 = off)
remind_push_days = 0         # OR remind every N calendar days (0 = off)
```

---

## LLM settings

### Using Claude (Anthropic)

```toml
[llm]
provider = "claude"
model = "claude-sonnet-4-6"
```

Requires `ANTHROPIC_API_KEY` set in your environment or a `.env` file:

```bash
ANTHROPIC_API_KEY=sk-ant-your-key-here
```

### Using Ollama (local, free)

```toml
[llm]
provider = "ollama"
model = "qwen2.5-coder:7b"
ollama_base_url = "http://localhost:11434"
```

Make sure Ollama is running (`ollama serve`) and the model is pulled (`ollama pull qwen2.5-coder:7b`).

| Your GPU VRAM | Recommended model |
|---|---|
| 6 GB | `qwen2.5-coder:7b` |
| 4 GB | `qwen2.5:3b` or `phi4-mini` |
| No GPU / CPU only | `qwen2.5:3b` (slow) |

---

## Session settings

| Setting | Default | What it does |
|---|---|---|
| `daily_budget` | 3 | Max quizzes Rocky will auto-trigger per day (manual calls bypass this) |
| `min_gap_minutes` | 120 | Minimum minutes between auto-triggered quizzes |

These only apply to automatic triggers (git hooks, Claude Code hook). Manual `rocky quiz` always runs.

---

## Export settings

| Setting | Default | What it does |
|---|---|---|
| `pkg_dir` | `~/.rocky/pkg/` | Where Rocky writes Markdown notes and `pkg.json` |

If you use Obsidian, set `pkg_dir` to a folder inside your Obsidian vault:

```toml
[export]
pkg_dir = "~/Documents/Obsidian/MyVault/rocky"
```

---

## UI settings

| Setting | Default | What it does |
|---|---|---|
| `personality` | `true` | Rocky the alien's voice, ASCII art, and milestone celebrations |

Set `personality = false` for plain, quiet output.

---

## Sync settings

Rocky can version-control your PKG and `pkg.json` as a git repo. Disabled by default — opt in when ready.

| Setting | Default | What it does |
|---|---|---|
| `enabled` | `false` | Enable PKG git tracking |
| `auto_commit` | `true` | Auto-commit after each session (when enabled) |
| `commit_visible` | `true` | Print `✓ Rocky: ...` after auto-commit |
| `remote` | `"origin"` | Git remote name |
| `branch` | `"main"` | Branch to push to |
| `remind_push_sessions` | `5` | Remind to push every N sessions (0 = off) |
| `remind_push_days` | `0` | Remind every N calendar days instead (0 = off) |

Only one of `remind_push_sessions` or `remind_push_days` should be non-zero.

### Getting started with sync

```bash
# 1. Enable sync in your config
echo '[sync]
enabled = true' >> ~/.rocky/.rocky.toml

# 2. Initialise the git repo (optionally set a remote at the same time)
rocky sync --init https://github.com/you/rocky-pkg.git

# 3. Do some quizzes, then push manually when ready
rocky sync --push

# 4. On a new machine: clone and restore
git clone https://github.com/you/rocky-pkg.git ~/.rocky
rocky restore
```

---

## Edges settings

Rocky automatically infers relationships between topics in your PKG and uses them to ask cross-concept questions. The `reuse` setting controls how often the same edge can be used as a question source.

| Setting | Default | What it does |
|---|---|---|
| `reuse` | `"off"` | How soon the same edge can trigger a cross-concept question again |

**Values:**

| Value | Behaviour |
|---|---|
| `"off"` | Always use an edge if it's the best fit |
| `"14d"` | Don't reuse the same edge within 14 calendar days |
| `"5s"` | Don't reuse the same edge within 5 quiz sessions |

```toml
[edges]
reuse = "14d"
```

Use a days-based cooldown if you want variety. Use `"off"` (default) to always surface the most relevant relationship regardless of recency.

---

## Claude Code hook

To make Rocky log your AI prompts automatically:

```bash
rocky install claude
```

This writes the hook entry to `~/.claude/settings.json`. Every prompt is silently logged — run `rocky quiz` at any time to review topics from recent sessions.

To remove:

```bash
rocky uninstall claude
```

---

## Where Rocky stores data

| Path | What it is |
|---|---|
| `~/.rocky/graph.db` | Your PKG — all topics, recall scores, review history |
| `~/.rocky/.rocky.toml` | Your global config |
| `~/.rocky/pkg/` | Markdown notes + `pkg.json` backup |
| `~/.rocky/pkg/pkg.json` | Full PKG export for backup and cross-machine restore |
| `./.rocky` | Per-project prompt log (only in hooked projects) |

`graph.db` is never tracked by git. Everything in `pkg/` is tracked when sync is enabled.
''';

const kHowItWorks = r'''
# How Rocky Works

Rocky has five modes of operation, each with different rules about when it quizzes you and what limits apply.

---

## Scenario 1: Manual task (`rocky "your task"`)

You describe what you're about to work on. Rocky extracts the topics, checks your PKG, and runs Socratic Q&A on anything new. No cooldown, no daily cap — you asked for it.

**Flow:**

1. Run `rocky "build a caching layer"`
2. Extract 2–5 key topics from task description
3. For each topic, classify against PKG:
   - **Known** (≥90% recall) → Mark encountered, move on
   - **Stale** (70–90%) → Write 2–3 sentence reminder, update PKG with small score bump
   - **New / Gap** (<70%) → Check for implication edges to other well-known topics; if a relevant edge exists, generate a cross-concept question linking the two topics; otherwise, generate a standard Socratic question
4. User answers:
   - **Understood** → Record in PKG with high confidence
   - **Not understood** → Give senior-engineer explanation, ask one follow-up, record partial confidence
   - **Too easy** (`k`) → Record as known — no Q&A needed
   - **Skip** (Enter) → Queue topic in `.rocky`, not added to PKG yet
   - **Ignore** (`i`) → Dismiss topic entirely, not added to PKG
5. After all Q&A: Rocky sends new topics to the LLM to infer relationships with existing PKG topics — edges are added silently in the background
6. Summary printed — done

---

## Scenario 2: Git commit hook (`rocky diff`)

After every `git commit`, Rocky analyses the diff for topics that appeared in your code. Cooldown and daily budget are enforced — this is automatic, not user-initiated. Topics that can't be quizzed right now are queued in `./.rocky` for the next `rocky quiz`.

**Flow:**

1. `git commit` → post-commit hook fires → runs `rocky diff`
2. Read staged diff + commit message
3. Extract topics from diff using code-aware analysis
4. Session checks:
   - **Cooldown active** → Queue all topics in `./.rocky`, print: "Rocky ready in ~Xm"
   - **Daily budget reached** → Queue all topics in `./.rocky`, print: "budget reached — resets tomorrow"
   - **OK** → Proceed
5. For each topic, classify against PKG:
   - **Known** → Mark encountered, silent
   - **Stale** → Print reminder to terminal
   - **New** → Run Socratic Q&A in terminal
6. If understood → Record in PKG. If not → Explain, record partial. If skip → Queue in `./.rocky`
7. Done

---

## Scenario 3: Claude Code hook (`rocky hook`)

When Claude Code is used in a project with Rocky configured, each prompt is silently logged to `./.rocky`. No quiz happens here — this is just capture.

**Flow:**

1. Claude Code prompt submitted
2. Is rocky hook installed? → If no, nothing happens
3. If yes → Log prompt text to `./.rocky` SQLite, auto-delete entries older than 24h
4. Silent — no output to user

---

## Scenario 4: `rocky quiz`

Explicitly request a learning session. No limits apply. Rocky works through a priority queue: queued topics first, then PKG topics most overdue for review, then anything new from recent prompts in `./.rocky`.

**Flow:**

1. Run `rocky quiz`
2. Load queued topics from `./.rocky`, ordered by queue time
3. If queued topics exist → Quiz each, remove from queue after completion
4. Load PKG topics sorted by urgency (lowest retrievability first)
5. If stale or gap topics exist → Quiz each (stale gets reminder first, gap gets full Socratic Q&A)
6. Is rocky hook installed in this project?
   - If yes → Read prompts from `./.rocky` from the last 24 hours
   - Extract new topics from prompt history
   - If new topics found → Run Socratic Q&A
7. Summary printed — done

---

## Scenario 5: Pre-commit review (`rocky diff --staged`)

Review your staged changes before committing. Behaves like the manual flow — no limits.

**Flow:**

1. Run `rocky diff --staged`
2. Read `git diff --staged` output
3. Extract topics from staged changes
4. For each topic, classify against PKG:
   - **Known** → Mark encountered
   - **Stale** → Print reminder
   - **New** → Run Socratic Q&A
5. If understood → Record in PKG. If not → Explain + follow-up, record partial. If skip → Queue in `./.rocky`
6. Proceed with commit

---

## How the PKG classifies topics

Every topic in the PKG has a **retrievability score** — an estimate of how likely you are to recall it right now, based on how long ago you last reviewed it and how stable your knowledge is.

**Retrievability lifecycle:**

1. **Topic reviewed** → Stability score increases based on answer quality
2. **Over time** → Retrievability decays (faster for unstable topics)
3. **Classification:**
   - ≥ 90% → **Known** — skipped automatically
   - 70–90% → **Stale** — printed as reminder
   - < 70% → **Gap** — full Socratic Q&A

Higher stability means the topic decays slower — if you've demonstrated solid understanding multiple times, Rocky won't ask you about it again for weeks.

---

## Domain taxonomy

Every topic is assigned to one of 13 domains when it's first extracted. Domains group topics in the PKG into subfolders and are used for Obsidian graph view clustering.

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

Use `rocky classify` to assign domains to any older topics that predate this feature.
''';

const kObsidian = r'''
# Obsidian Integration

Rocky writes your entire knowledge graph as Markdown files into a PKG directory. Each topic becomes a note with metadata that Obsidian's Dataview plugin can query and visualise.

## Setup

### 1. Tell Rocky where your PKG directory is

In `~/.rocky/.rocky.toml`:

```toml
[export]
pkg_dir = "~/Documents/Obsidian/MyVault/rocky"
```

Rocky will write all files directly into the path you set. If pointing to an Obsidian vault, set it to a subfolder like `~/Documents/Obsidian/MyVault/rocky`.

### 2. Export your topics

```bash
rocky export
```

This writes one `.md` file per topic, grouped by domain into subfolders. Rocky also exports automatically every time a topic is updated, so your PKG stays in sync.

### 3. Install the Dataview plugin in Obsidian

1. Open Obsidian → Settings → Community plugins
2. Search for "Dataview" and install it
3. Enable it

---

## Vault structure

Topics are grouped by domain into subfolders:

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

The 13 domains: Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other.

---

## What the notes look like

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

Related topics are linked via `See also:` wikilinks, so Obsidian's graph view shows the connections between your topics.

---

## Auto-created dashboard pages

Rocky automatically creates two dashboard pages when you export:

### Rocky Dashboard.md

An overview of your entire PKG, including a by-domain breakdown and a table of all topics sorted by recall.

### Rocky Review Queue.md

Topics that need attention — your gaps and fading topics, sorted by most urgent.

Both pages use Dataview queries and update automatically as your PKG changes.

---

## Custom Dataview queries

You can add your own queries to any Obsidian note.

### Topics that need attention

```
TABLE rocky_retrievability AS "Recall %",
      rocky_last_reviewed AS "Last Reviewed",
      rocky_kind AS "Kind"
FROM #rocky/node
WHERE rocky_retrievability < 0.7
SORT rocky_retrievability ASC
```

### Your strongest topics

```
TABLE rocky_retrievability AS "Recall %",
      rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_retrievability >= 0.9
SORT rocky_review_count DESC
```

### Topics by domain

```
TABLE rocky_retrievability AS "Recall %",
      rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/domain/Auth
SORT rocky_retrievability ASC
```

Replace `Auth` with any domain name.

### Topics by kind

```
TABLE rocky_retrievability AS "Recall %",
      rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/kind/pattern
SORT rocky_retrievability ASC
```

Replace `pattern` with `concept` or `implementation`.

### Everything sorted by recall

```
TABLE rocky_retrievability AS "Recall",
      rocky_domain AS "Domain",
      rocky_days_since_review AS "Days ago"
FROM #rocky/node
SORT rocky_retrievability ASC
```

---

## Graph view

Because each topic is a regular Obsidian note, you can use Obsidian's graph view to visualise your knowledge. Topics in the same domain are grouped together. The `See also:` wikilinks create edges between related topics.

Filter the graph to `#rocky/node` to see only your PKG. Filter to `#rocky/domain/Auth` to zoom into a specific area.
''';

const kSync = r'''
# PKG Sync & Backup

Rocky can version-control your PKG using git, giving you backup, history, and cross-machine sync.

---

## What gets tracked

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

`graph.db` is the live runtime database. It's a binary file, bad for git, and always rebuildable from `pkg.json`. The PKG directory is the human-readable, git-friendly projection of your knowledge.

---

## Quick start

```bash
# Enable in config (~/.rocky/.rocky.toml)
[sync]
enabled = true

# Initialise git repo + set a remote
rocky sync --init https://github.com/you/rocky-pkg.git

# Run some quizzes — PKG commits automatically after each session
rocky quiz

# Push when ready
rocky sync --push
```

---

## Commands

### `rocky sync`

Commit any pending PKG changes. Shows what changed.

```bash
rocky sync
# ✓ Rocky: 12/15 known — 2026-04-04
```

### `rocky sync --push`

Commit and push to the configured remote.

### `rocky sync --init [url]`

Initialise the git repo in `~/.rocky/`. Optionally set a remote URL at the same time.

```bash
rocky sync --init                                         # init only
rocky sync --init https://github.com/you/rocky-pkg.git   # init + set remote
```

### `rocky sync --status`

Show how many files changed since the last commit and how many commits are ahead of the remote.

### `rocky restore`

Rebuild `graph.db` from `pkg/pkg.json`. Use this when setting up Rocky on a new machine after cloning your PKG.

```bash
git clone https://github.com/you/rocky-pkg.git ~/.rocky
rocky restore
# ✓ Restored 42 topics from ~/.rocky/pkg/pkg.json
```

### `rocky backup`

Write `pkg/pkg.json` without committing. Useful for a quick snapshot before making changes.

---

## Topic domains and PKG structure

Rocky classifies every topic into one of 13 domains:

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

The domain becomes the subfolder in the PKG, which Obsidian uses for graph view grouping. Topics also link to related notes via `See also:` wikilinks, forming a web of connections in the graph view.

To assign domains to existing topics that predate this feature:

```bash
rocky classify
# Classifying 8 topics into domains...
# ✓ Redis TTL expiry → Database
# ✓ Rust ownership model → Language
# ...
# · Run `rocky export` to update PKG files.
```

---

## Cross-machine workflow

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

---

## Push reminders

Rocky tracks how many sessions have passed since your last push and reminds you when the threshold is hit:

```
♫ Rocky: 5 sessions unsynced — consider `rocky sync --push` to back up, question?
```

Configure the threshold in `~/.rocky/.rocky.toml`:

```toml
[sync]
remind_push_sessions = 5   # every 5 sessions (default)
remind_push_days = 7       # OR every 7 days — pick one, set other to 0
```

Set both to `0` to disable reminders entirely.
''';
