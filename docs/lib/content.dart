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
     _____
   .'     '.
  /  .   .  \
 |  . _____ .|
 |   |     | |
 |   |_____|  |
  \   .   .  /
   '.______.'
```

Rocky has a personality based on Rocky the alien from Andy Weir's [*Project Hail Mary*](https://www.imdb.com/title/tt12042730/) — enthusiastic, direct, and genuinely rooting for you.

- `Fist my bump, friend! Is correct!`
- `Excite excite excite! Friend get it!`
- `Is okay! Rocky also not know at first!`
- `We are crew. We solve together.`

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
| **Repo tag** | The git project a topic originated from — parsed from the remote URL. Makes the PKG filterable by project |
| **Canonical Q&A** | A pre-generated question and ideal answer stored in the node at creation time, using the commit diff and project README as context |
| **Canonical clue** | A short hint stored alongside canonical Q&A — shown when you type `c` during a quiz. Generated at backfill time, or on-demand for manually-added topics |
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
  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  0
  Known:         0
  Fading:        0
  Gaps/weak:     0

  Quiz budget: 3/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  PKG is empty. Let us begin science, question?
```

---

## Set up the git hook

In your project directory, run:

```bash
rocky install        # or: rocky install git
```

You'll see:

```
     _____
   .'     '.
  /  .   .  \        R  O  C  K  Y
 |  . _____ .|       Personal Knowledge Graph
 |   |     | |
 |   |_____|  |       You observe. Question?
  \   .   .  /
   '.______.'


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
  Rocky · Personal Knowledge Graph
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

  Fist my bump, friend! Is correct!

  Good — you've covered the refresh flow and revocation. One thing worth
  double-checking: refresh tokens should be stored in httpOnly cookies,
  not localStorage, to prevent XSS from stealing them.

  ✓ JWT authentication added to PKG.
```

### Q&A options

At any question you can:

- **Type your answer** and press Enter — Rocky evaluates and gives feedback
- **Press Enter** with nothing to skip — queues the topic for later
- **`i`** — ignore the topic (useful when Rocky picks up a hallucinated or irrelevant topic)
- **`e`** — too easy, mark as known without a full Q&A
- **`s`** — regenerate a simpler version of the question
- **`h`** — regenerate a harder version with edge cases and deeper implications
- **`c`** — show a short clue without giving away the answer
- **`?`** — show a full explanation and mark with low confidence

Each question shows whether it's **(canonical)** — specific to your actual commit diff — or **(generated)** — created live from the topic description.

---

## Step 3: Check your knowledge graph

After a few sessions, see what you know:

```bash
rocky ls

  Rocky · Personal Knowledge Graph
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
| `e` | Too easy — mark as known without a full Q&A |
| `s` | Simpler — regenerate the question at lower difficulty |
| `h` | Harder — regenerate the question with edge cases and deeper implications |
| `c` | Clue — show a short hint without giving away the answer |
| `?` | Explain — show a full explanation and mark with low confidence |

Each question also shows whether it is **(canonical)** — pre-generated at backfill time from the actual commit diff — or **(generated)** — generated live. Canonical questions are more specific to your actual code.

---

## `rocky stats`

PKG summary.

```bash
rocky stats

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  24
  Known:         15
  Fading:         6
  Gaps/weak:      3

  Quiz budget: 2/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  Edges: 18 total  ·  Most connected: JWT authentication (4 edges)

  Good progress, friend. Keep science going.
```

---

## `rocky ls`

Full topic list with all metrics.

```bash
rocky ls

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

### Prompt logging only (no git hook)

Enable the `.rocky` prompt log in a project without installing the git hook. Useful when you want `rocky quiz` to pick up Claude Code prompts but don't want `rocky diff` to run on every commit.

```bash
rocky install prompt
rocky uninstall prompt
```

This creates the `.rocky` SQLite log file and adds it to `.gitignore`, but leaves the post-commit hook untouched.

---

## `rocky queue`

Show topics queued in this project. A topic is queued when you pressed Enter (skip) during a session — it's been encountered but not yet added to your PKG. Queued topics are stored in `./.rocky` and picked up automatically next time you run `rocky quiz`.

```bash
rocky queue
```

**When topics are queued:**

```
  3 queued topics (not yet in PKG):

  · Redis Lua scripting
    concept · Lua scripts in Redis run atomically — the whole script or nothing
    from: add Redis rate limiter

  · PostgreSQL transactions
    concept · All-or-nothing SQL execution with ACID guarantees
    from: add database connection

  · Docker multi-stage builds
    implementation · Build stages that produce smaller final images by discarding build tools
    from: Dockerize the app

  Run  rocky quiz  to work through the queue.
```

**When the queue is empty:**

```
  No topics queued — queue is clear.
```

**When Rocky hasn't been installed in this project:**

```
  No queue found for this project.
  Run `rocky install` or `rocky install prompt` to enable logging here.
```

---

## `rocky logs`

Show recent prompts logged in this project from the Claude Code hook (last 24 hours). Useful for seeing what you've been asking your AI to build before running `rocky quiz`.

```bash
rocky logs
```

**When prompts have been logged:**

```
  4 prompts in the last 24h:

  09:14 add JWT authentication middleware to the Axum router
  10:32 implement Redis caching for user sessions with TTL
  14:05 write sqlx migration for the users table with created_at and updated_at columns
  16:41 add rate limiting using Redis sorted sets — max 100 requests per minute per user
```

**When nothing has been logged today:**

```
  No prompts logged in the last 24 hours.
```

**When the Claude Code hook isn't installed:**

```
  No local log found.
  Run `rocky install` in this project to enable prompt logging.
```

`rocky quiz` automatically reads from this same log, so any prompts shown here will be analysed in your next quiz session.

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

## `rocky backfill`

Scan your git history and silently add topics to your PKG — no interactive Q&A. Perfect for seeding your knowledge graph when you first install Rocky on an existing project.

```bash
# Scan all your commits in the current repo
rocky backfill

# Include commits from all authors (default: your commits only)
rocky backfill --all-authors

# Limit to the most recent N commits
rocky backfill --limit 20

# Scan last 50 commits from all contributors
rocky backfill --all-authors --limit 50

# Retroactively generate clues for nodes that have canonical Q&A but no clue yet
rocky backfill --fill-clues
```

Rocky reads the diff for each commit, extracts topics the same way `rocky diff` does, and adds any that aren't already in your PKG. It sets initial retrievability to 0.5 (neutral — you saw the code but weren't quizzed). Edges are generated for all new topics after the scan completes.

Each new topic is enriched at insertion time:

- **Commit date** — `created_at` and `last_reviewed` are set to the actual commit date, not today. A topic from six months ago decays correctly from when you first encountered it.
- **Repo tag** — the node is tagged with the project name (parsed from the git remote URL). Topics accumulate in one PKG across all your projects, and you can filter by repo in `rocky view`.
- **Canonical Q&A** — Rocky generates a question, ideal answer, and a short clue for each new topic using the commit diff and a cached summary of the project README. These are stored in the node: the question is used next time you're quizzed; the clue is shown when you type `c`; the ideal answer guides evaluation.
- **Project summary** — Rocky reads your README, summarises it, and caches the summary in `~/.rocky/summaries/<repo>.txt`. The summary is shown in `rocky view` when you filter by that repo.

**Example output:**

```
  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Taxonomy skeleton ready.
  Scanning last 10 commits by alex@example.com (10 commits)…

  Project: Rust REST API — task management backend with JWT auth,
           PostgreSQL, Redis caching, and Docker deployment.

  [1/10] a3f8c12 init: Axum server scaffold with tokio runtime — no new topics
  [2/10] b7d4e19 feat: sqlx PgPool + migration runner — no new topics
  [3/10] c1a2d83 feat: JWT auth middleware — no new topics
  [4/10] d9f3b41 feat: Redis caching layer — no new topics
  [5/10] e4c8a27 feat: per-user rate limiting — no new topics
  [6/10] f2b7e94 feat: Docker multi-stage build — 2 new
    + Docker multi-stage builds
    + container image optimization
  [7/10] g8d1c35 feat: GitHub Actions CI pipeline — 2 new
    + GitHub Actions workflow syntax
    + CI/CD pipeline design
  [8/10] h5e4b72 feat: OpenAPI spec with utoipa — 1 new
    + OpenAPI specification
  [9/10] i3f6d28 fix: handle expired tokens in middleware — no new topics
  [10/10] j7a9c14 docs: API documentation and README — no new topics

  ◈ Generating edges for 5 new topics…

  ✓ Added 5 new topics · 47 already in PKG
  Run  rocky quiz  to start reviewing them.
```

After backfill, run `rocky quiz` to start reviewing the newly discovered topics.

---

## `rocky delete "query"`

Search your PKG and remove topics that are no longer relevant.

```bash
rocky delete "jwt"
rocky delete "react hooks"
```

Rocky shows all matching topics and asks you to confirm before deleting. The corresponding PKG file is also removed.

### Date-based deletion

Remove topics added within a specific date range — useful for cleaning up after a bad session or resetting topics from a period when you were exploring unfamiliar technology.

```bash
# Delete all topics added on or after a date
rocky delete --since 2026-04-01

# Delete all topics added on or before a date
rocky delete --before 2026-01-01

# Combine for a precise date range
rocky delete --since 2026-03-01 --before 2026-03-31
```

You can combine a search query with date flags:

```bash
# Delete "redis" topics added in March
rocky delete "redis" --since 2026-03-01 --before 2026-03-31
```

Rocky always shows what will be deleted and asks for confirmation before removing anything.

---

## `rocky config`

Show what config Rocky is currently using.

```bash
rocky config
```

---

## `rocky edges`

List all edges in the implication graph — the relationships Rocky has inferred between topics in your PKG.

```bash
rocky edges

  SOURCE                         TARGET                         KIND               STR   DESCRIPTION
  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
  JWT authentication             token expiry handling          implies            0.90  ...
  Redis TTL expiry               cache invalidation             depends_on         0.75  ...
  SQL indexes                    query optimisation             implies            0.85  ...
```

```bash
# Edge stats summary
rocky edges --stats

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

**Graph** — D3.js force simulation. Drag nodes, zoom in/out, filter by domain, search topics by name. Click any node to open a detail panel showing retrievability, stability, review history, all connected edges, and — when available — the pre-generated question and ideal answer for that topic. Click any edge to see the reason Rocky created it.

**Spread slider** — controls the repulsion force between nodes. Drag right to spread the graph out; drag left to pull clusters together. Useful when many nodes overlap after a large backfill.

**Timeline scrubber** — a range slider below the controls lets you rewind your knowledge graph to any point in time. As you scrub backward, nodes dim and disappear (topics you hadn't learned yet). Scrub forward to watch them light up — your personal growth, visualised. The current node count and date are shown next to the scrubber.

**Domain panel** — a list on the left shows every active domain with a count of visible nodes. Click a domain to highlight only that cluster and its edges.

**Repo filter** — when your PKG contains topics from more than one project, repo filter buttons appear in the controls. Click a repo to show only its topics and display the project summary at the top of the graph. Works across projects accumulated over time via `rocky backfill`.

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

## Scenario 6: Git history backfill (`rocky backfill`)

Seed your PKG from your git history without any interactive Q&A. Useful when you first install Rocky on an existing project — Rocky scans your commits, extracts topics from each diff, and adds anything new to the PKG with a neutral starting retrievability of 0.5.

**Flow:**

1. Run `rocky backfill` (optionally with `--all-authors` or `--limit N`)
2. Resolve author filter (default: current `git user.email` only)
3. Fetch all matching commit SHAs from `git log`, oldest first
4. Detect the project name from the git remote URL (fallback: directory name)
5. Load or create the project README summary, cached in `~/.rocky/summaries/<repo>.txt`
6. For each commit:
   - Read the commit date and diff
   - Extract topics using code-aware analysis (same as `rocky diff`)
   - For each topic not already in the PKG:
     - Add it with retrievability 0.5, `created_at` and `last_reviewed` set to the commit date, tagged with the repo name
     - Generate a canonical question, ideal answer, and short clue using the diff + commit message + README summary
     - Store all three in the node: the question is used at quiz time, the clue is available on request, the answer guides evaluation
   - Print `+ topic name` for each new topic added
7. After all commits: generate edges for all newly added topics in bulk
8. Print summary: `✓ Added N new topics · M already in PKG`
9. Prompt to run `rocky quiz` to start reviewing

Backfill never overwrites existing PKG entries — if a topic is already in your PKG, it's counted as "already in PKG" and skipped. Each topic is owned by the first commit that introduced it; later commits that mention the same topic do not update it.

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
Rocky: 5 sessions unsynced — consider `rocky sync --push` to back up, question?
```

Configure the threshold in `~/.rocky/.rocky.toml`:

```toml
[sync]
remind_push_sessions = 5   # every 5 sessions (default)
remind_push_days = 7       # OR every 7 days — pick one, set other to 0
```

Set both to `0` to disable reminders entirely.
''';

const kWalkthrough = r'''
# Demo Usecase: A PKG from Scratch

```graphlink
graphs/stage7.html|Open the full interactive graph
```

This is a complete, realistic example of building a Personal Knowledge Graph across two projects:

- **taskify** — a Rust REST API with JWT auth, PostgreSQL, Redis caching, and Docker deployment
- **home-bank** — a Python data pipeline that imports bank CSV exports, categorises transactions, and models accounts with double-entry bookkeeping

We follow commits across both projects. The full interactive graph (stage 7) shows 21 topics across both repos — use the **repo filter** buttons to isolate either project and see its summary.

---

## Setup

Fresh project. Empty PKG.

```bash
~ $ mkdir taskify && cd taskify && git init
~/taskify $ rocky install          # install the git hook
~/taskify $ rocky stats

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  0
  Known:         0
  Fading:        0
  Gaps/weak:     0

  Quiz budget: 3/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  PKG is empty. Let us begin science, question?
```

**`rocky view` at this point:** A blank canvas. A few taxonomy skeleton nodes (Language, Database, Auth…) float at the edges, lightly dimmed. Nothing in the centre.

---

## Commit 1 — Project scaffold

```bash
~/taskify $ rocky "set up Rust Axum web server with tokio and tower middleware"

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Task: set up Rust Axum web server with tokio and tower middleware

  Analyzing topics...

  Rocky: New topic — Rust async/await
    Rust's async/await builds on zero-cost state machines — when you write
    async fn, the compiler generates a state machine, not threads.

  Q1. You have a Rust async function that makes two independent HTTP calls.
      Right now they run sequentially. How would you run them concurrently,
      and what's the difference between join! and select!, question?

  > tokio::join! runs both futures concurrently and waits for both to finish.
    select! waits for whichever resolves first and cancels the other. For two
    independent calls where I want both results, join! is correct.

  Fist my bump, friend! Is correct!

  Good distinction. Worth noting: join! doesn't parallelize — both futures
  still run on a single thread unless you spawn tasks. tokio::spawn moves
  them to the thread pool.

  ✓ Rust async/await added to PKG.

  Rocky: New topic — tokio runtime
    The async executor that drives Rust futures — manages the thread pool
    and event loop.

  Q2. You add #[tokio::main] to your main function. What does it actually do,
      and what's the difference between the current_thread and multi_thread
      flavours, question?

  > #[tokio::main] is a macro that creates a tokio runtime and calls block_on
    on your main future. current_thread runs everything on one thread;
    multi_thread (default) uses a thread pool matching your CPU cores.

  Excite excite excite! Friend get it!

  ✓ tokio runtime added to PKG.

  Rocky: New topic — Axum framework
    A Rust web framework built on hyper and tower — composable routing with
    extractors and type-safe middleware.

  Q3. You want to add request logging to every route in Axum. What's the
      difference between a tower Layer and a tower Service, and how do they
      compose in Axum, question?

  > A Service is the core abstraction — takes a request, returns a future
    with a response. A Layer wraps a Service to add behaviour (logging,
    auth, etc.). Router::layer applies a tower Layer to all routes.

  We are crew. We solve together. Is correct!

  ✓ Axum framework added to PKG.
```

```bash
~/taskify $ git commit -m "init: Axum server scaffold with tokio runtime"
# Rocky runs automatically (git hook):
#
#   ~ Rust async/await  (just reviewed — budget reached, skipping)
#   ✓ No new topics in diff beyond what was just quizzed.
```

**PKG after commit 1:**

```bash
~/taskify $ rocky ls

  Topic                    Kind     Recall         Stab  Diff  Reviews  Last Reviewed
  ─────────────────────────────────────────────────────────────────────────────────────
  Rust async/await         concept  ██████████ 95%  4.5   0.3   1        2026-04-01
  tokio runtime            concept  ██████████ 95%  4.5   0.2   1        2026-04-01
  Axum framework           concept  ██████████ 95%  4.5   0.3   1        2026-04-01
```

**`rocky view` after commit 1:** Three nodes cluster in the centre, connected by auto-inferred edges: *tokio runtime → Axum framework* (`depends_on`, strength 0.90), *Rust async/await → tokio runtime* (`depends_on`, strength 0.88). All three glow bright gold. The Language taxonomy node pulls them together on the left side of the canvas. **Click any node** to open the detail panel — you'll see the question Rocky asked, your answer, and the feedback, all stored against that topic.

```graphlink
graphs/stage1.html|Open interactive graph: commit 1
```

---

## Commit 2 — Database and migrations

```bash
~/taskify $ rocky "add PostgreSQL with sqlx — connection pooling and migrations"

  Task: add PostgreSQL with sqlx — connection pooling and migrations

  Rocky: New topic — sqlx connection pooling
    Async database driver for Rust with compile-time query checking —
    PgPool manages a pool of reusable PostgreSQL connections.

  Q1. Your API gets a burst of 200 simultaneous requests. With sqlx PgPool,
      what happens when all pool connections are in use, question?

  > New requests wait until a connection is freed. PgPool has a max_connections
    setting — if the pool is exhausted the request blocks until one becomes
    available, and returns an error if it waits longer than connect_timeout.

  Fist my bump, friend! Is correct!

  ✓ sqlx connection pooling added to PKG.

  Rocky: New topic — database migrations
    Versioned, incremental changes to your database schema — tracked so
    they apply once and in order across all environments.

  Q2. You run sqlx migrate run in CI. A migration ran on staging but you
      changed it locally before it hit production. What happens when
      production tries to apply it, question?

  > It would just re-run the migration on production since the file changed.

  Almost — but there is problem. Rocky must ask more.

  Q3. sqlx stores a checksum of each migration file and tracks which have
      already run. If the checksum differs from what ran before, what does
      sqlx do — and why does it refuse rather than re-apply?

  > Oh — sqlx refuses to run it entirely. Re-running a migration that
    already applied could corrupt the schema. The checksum is a safety
    guard: it detects edits after the fact and blocks them.

  Is correct, friend! Rocky also learn this hard way.

  ✓ database migrations added to PKG.

  Rocky: New topic — PostgreSQL transactions
    All-or-nothing execution of multiple SQL statements — ACID guarantees.

  > (Enter — skip, I know this already)

  Topic queued for later.
```

```bash
~/taskify $ git commit -m "feat: sqlx PgPool + migration runner"
```

**PKG after commit 2:**

```bash
~/taskify $ rocky ls

  Topic                    Kind           Recall         Stab  Diff  Reviews  Last Reviewed
  ──────────────────────────────────────────────────────────────────────────────────────────
  Rust async/await         concept        █████████░ 92%  4.5   0.3   1        2026-04-01
  tokio runtime            concept        █████████░ 92%  4.5   0.2   1        2026-04-01
  Axum framework           concept        █████████░ 92%  4.5   0.3   1        2026-04-01
  sqlx connection pooling  implementation ██████████ 96%  4.8   0.3   1        2026-04-02
  database migrations      concept        ██████████ 96%  5.0   0.2   1        2026-04-02
```

```bash
~/taskify $ rocky queue

  1 queued topic (not yet in PKG):

  · PostgreSQL transactions
    concept · All-or-nothing SQL execution with ACID guarantees
    from: add PostgreSQL with sqlx — connection pooling and migrations

  Run  rocky quiz  to work through the queue.
```

**`rocky view` after commit 2:** Five nodes — the three Rust nodes from commit 1 remain bright gold but have started their slow decay (92%). Two new nodes appear near the Database taxonomy anchor. New edges: *sqlx connection pooling → database migrations* (`depends_on`, 0.80), *database migrations → sqlx connection pooling* (`implies`, 0.78). Scrub the timeline slider back to "April 1" and watch the Database nodes disappear.

```graphlink
graphs/stage2.html|Open interactive graph: commit 2
```

---

## Commit 3 — JWT authentication

```bash
~/taskify $ rocky "implement JWT auth middleware — issue tokens, validate on protected routes"

  Task: implement JWT auth middleware — issue tokens, validate on protected routes

  Rocky: New topic — JWT authentication
    Stateless token-based auth where the server signs a payload the client
    stores and sends back on every request.

  Q1. You're issuing JWTs with a 15-minute expiry. A user's access token
      expires mid-checkout. What needs to happen on both sides for the
      experience to feel seamless, without storing session state on the
      server, question?

  > The client stores a refresh token in an httpOnly cookie. When the access
    token expires, the client sends the refresh token to /refresh. The server
    validates it against a database (so it can be revoked) and issues a new
    access token. The user never sees a login screen.

  Fist my bump, friend! Is correct!

  ✓ JWT authentication added to PKG.

  Rocky: New topic — httpOnly cookie security
    Cookies with the HttpOnly flag cannot be read by JavaScript — only sent
    by the browser automatically on requests to the matching domain.

  Q2. You're choosing between localStorage and an httpOnly cookie for your
      refresh token. Your app loads third-party analytics JavaScript. What's
      the attack surface difference, question?

  > ?

     Fetching explanation...

   localStorage lives in the browser's JS heap — any script on the page,
   including third-party analytics, can read window.localStorage. An XSS
   payload or compromised analytics snippet silently exfiltrates the token.
   httpOnly cookies are never exposed to JavaScript at all; the browser
   attaches them to requests automatically but scripts can't read them.
   With third-party JS on the page, httpOnly is the only safe choice.

   Saved to PKG — revisit before your next task.

  Rocky: New topic — token expiry handling
    ...

  > k  (mark as known — no Q&A needed)

  ✓ token expiry handling added to PKG.
```

```bash
~/taskify $ git commit -m "feat: JWT auth middleware with refresh token rotation"
```

```bash
~/taskify $ rocky stats

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  8
  Known:         7
  Fading:        1
  Gaps/weak:     0

  Quiz budget: 0/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  Edges: 6 total  ·  Most connected: JWT authentication (3 edges)

  Good progress, friend. PKG growing. Keep science going.
```

**`rocky view` after commit 3:** An Auth cluster has appeared in the upper-right — JWT authentication, httpOnly cookie security, and token expiry handling form a triangle. The most notable edge: *JWT authentication → httpOnly cookie security* (`implies`, 0.92). The Database cluster sits lower-left. The Language cluster anchors the left. All nodes glow gold. Scrub the timeline from April 1 to April 3 to watch each cluster grow in.

```graphlink
graphs/stage3.html|Open interactive graph: commit 3
```

---

## Commit 4 — Redis caching

```bash
~/taskify $ rocky "add Redis caching for user sessions and frequently accessed data"

  Task: add Redis caching for user sessions and frequently accessed data

  Rocky: New topic — Redis TTL expiry
    Time-based automatic key removal in Redis — keys expire and are deleted
    after a configured duration.

  Q1. You cache a user's profile in Redis with a 5-minute TTL. The user
      updates their profile. The API writes the update to Postgres and
      returns 200. A second request 30 seconds later hits Redis — what
      does it get, and how do you fix it, question?

  > It gets stale data — the cache still has the old profile. The fix is
    either write-through (update both Postgres and Redis on writes) or cache
    invalidation (delete the key on write and let the next read repopulate).
    Write-through keeps reads fast; invalidation is simpler but causes one
    cache miss after each write.

  Is correct! Rocky appreciate the trade-off answer.

  ✓ Redis TTL expiry added to PKG.

  Rocky: New topic — cache invalidation strategies
    Approaches for keeping a cache consistent with its source of truth —
    write-through, cache-aside, TTL expiry, and explicit invalidation.

  Q2. Your system has multiple API servers. Server A caches user:123 in its
      local memory. Server B updates user:123. Server A's cache is now stale.
      How does Redis solve this, question?

  > Each server could subscribe to a Redis pub/sub channel and invalidate
    its local cache when it gets the message?

  That is one approach — but Rocky want simpler answer. Think about
    where the cache itself lives.

  Q3. The root issue is in-memory caches are per-process and can't see each
      other. Where does Redis sit relative to your API servers, and why does
      that eliminate the staleness problem entirely?

  > Redis is shared outside the processes — all servers read and write
    the same cache. There is no per-server copy to go stale. Cache-aside
    means any server that updates Redis, all others immediately see it.

  Fist my bump, friend!

  ✓ cache invalidation strategies added to PKG.
```

```bash
~/taskify $ git commit -m "feat: Redis caching layer for user sessions"
```

**PKG after commit 4:**

```bash
~/taskify $ rocky ls

  Topic                       Kind           Recall         Stab  Diff  Reviews  Last Reviewed
  ──────────────────────────────────────────────────────────────────────────────────────────────
  Rust async/await            concept        ████████░░ 85%  4.5   0.3   1        2026-04-01
  tokio runtime               concept        ████████░░ 85%  4.5   0.2   1        2026-04-01
  Axum framework              concept        ████████░░ 85%  4.5   0.3   1        2026-04-01
  sqlx connection pooling     implementation █████████░ 90%  4.8   0.3   1        2026-04-02
  database migrations         concept        █████████░ 90%  5.0   0.2   1        2026-04-02
  JWT authentication          pattern        █████████░ 91%  5.2   0.3   1        2026-04-03
  httpOnly cookie security    concept        ███████░░░ 68%  1.8   0.3   1        2026-04-03
  token expiry handling       concept        █████████░ 91%  4.8   0.2   1        2026-04-03
  Redis TTL expiry            implementation ██████████ 96%  5.1   0.3   1        2026-04-04
  cache invalidation          concept        ██████████ 96%  5.2   0.2   1        2026-04-04
```

**`rocky view` after commit 4:** Four clusters now visible. A Performance cluster has formed around cache invalidation, and Redis TTL expiry bridges the Database and Performance anchors. The Language cluster (Rust, tokio, Axum) has shifted slightly amber as their initial high recall starts to decay. Scrub the timeline from April 1 through April 4 to watch each cluster appear: Language → Database → Auth → Performance.

```graphlink
graphs/stage4.html|Open interactive graph: commit 4
```

---

## Commit 5 — Rate limiting

```bash
~/taskify $ rocky "implement per-user rate limiting with Redis sorted sets and Lua scripting"

  Task: implement per-user rate limiting with Redis sorted sets and Lua scripting

  Rocky: New topic — Redis sorted sets
    A Redis data structure mapping member strings to floating-point scores —
    ordered by score, O(log N) insert and rank queries.

  Rocky: New topic — Lua scripting in Redis

  Cross-concept edge detected! JWT authentication → rate limiting

  Q1. You know JWT authentication well. Your rate limiter identifies users
      by their JWT subject claim. A malicious client strips the Authorization
      header. The limiter falls back to IP-based limits. What's the attack
      vector and how do you close it, question?

  > IP-based fallback can be bypassed by rotating IPs via proxies or Tor.
    The fix: require auth on all rate-limited routes — return 401 if no
    valid JWT is present. Never fall back to IP for authenticated endpoints;
    IP limits only make sense for unauthenticated routes like /login.

  Excite! Cross-concept question! Friend connect the dots!

  ✓ Redis sorted sets added to PKG.

  Q2. Your rate limiter checks a Redis sorted set and increments a counter
      in two separate commands. Under load, two requests arrive simultaneously.
      Both see the count is under the limit and both pass. What's the fix,
      and what makes it correct, question?

  > This is a TOCTOU race — check-then-act without atomicity. The fix is a
    Lua script: Redis executes the entire script atomically — no other command
    can interleave. The script checks, conditionally increments, and returns
    the result as a single atomic operation.

  Fist my bump! Lua scripts in Redis — atomically or nothing!

  ✓ Lua scripting in Redis added to PKG.
```

```bash
~/taskify $ git commit -m "feat: per-user rate limiting with Redis sorted sets"
```

```bash
~/taskify $ rocky edges --stats

  ◈ Edge Stats

  Total edges:           14
  Avg strength:          0.81
  Most connected:        JWT authentication (5 edges)

  By kind:               count    avg str
    implies              8        0.84
    depends_on           4        0.78
    conflicts_with       1        0.65
    part_of              1        0.82
```

**`rocky view` after commit 5:** This is when the graph becomes interesting. JWT authentication is the most-connected node — edges radiate outward to token expiry handling, httpOnly cookie security, Redis sorted sets, and Lua scripting. The cross-concept edge between Auth and Database clusters appears in cyan (`implies`), crossing the gap between clusters. The `conflicts_with` edge glows red. Click any node to open its detail panel showing retrievability, stability, review history, and all connected edges.

```graphlink
graphs/stage5.html|Open interactive graph: commit 5
```

---

## One week later — decay begins

You've been shipping features. It's been 7 days since commit 1.

```bash
~/taskify $ rocky stats

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  12
  Known:         7
  Fading:        4
  Gaps/weak:     1

  Quiz budget: 3/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  Edges: 14 total  ·  Most connected: JWT authentication (5 edges)

  Some topics fading, friend. Time for science.
```

```bash
~/taskify $ rocky ls

  Topic                       Kind           Recall         Stab  Diff  Reviews  Last Reviewed
  ──────────────────────────────────────────────────────────────────────────────────────────────
  Rust async/await            concept        ██████░░░░ 63%  4.5   0.3   1        2026-04-01
  tokio runtime               concept        ███████░░░ 72%  4.5   0.2   1        2026-04-01
  Axum framework              concept        ███████░░░ 73%  4.5   0.3   1        2026-04-01
  sqlx connection pooling     implementation ████████░░ 80%  4.8   0.3   1        2026-04-02
  database migrations         concept        ████████░░ 81%  5.0   0.2   1        2026-04-02
  JWT authentication          pattern        ████████░░ 82%  5.2   0.3   1        2026-04-03
  httpOnly cookie security    concept        ████████░░ 82%  5.0   0.3   1        2026-04-03
  token expiry handling       concept        ████████░░ 83%  4.8   0.2   1        2026-04-03
  Redis TTL expiry            implementation ████████░░ 78%  5.1   0.3   1        2026-04-04
  cache invalidation          concept        ████████░░ 79%  5.2   0.2   1        2026-04-04
  Redis sorted sets           implementation ██████████ 95%  5.3   0.3   1        2026-04-07
  Lua scripting in Redis      concept        ██████████ 95%  5.5   0.2   1        2026-04-07
```

**`rocky view` after one week:** The same 12 nodes — but now in three colors. The Language cluster (Rust, tokio, Axum) has shifted amber to orange. Rust async/await glows red (gap — below 70%). The Auth and Database clusters are amber. The two Redis topics from this week are still bright gold. This is the core value of the timeline scrubber: it shows current decay state, not just when topics were added.

```graphlink
graphs/stage6.html|Open interactive graph: one week later
```

```bash
~/taskify $ rocky quiz

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  0 topics in queue.
  4 fading topics · 1 gap topic — starting review…

  ~ Rust async/await  (gap · 63% recall)
    Reminder: Rust's async/await uses state machines compiled at build time —
    when you .await, the compiler pauses execution and polls the future again
    when the resource is ready.

  Q1. You have an async function holding a std::sync::Mutex guard across an
      .await point. Your future gets suspended. What happens to other tasks
      that try to lock the same Mutex, question?

  > They deadlock. If a future holds a std::sync::Mutex across an await point,
    the thread is suspended with the lock held. No other task on that thread
    can acquire it. Use tokio::sync::Mutex for async contexts — its lock
    is async-aware and yields the thread instead of blocking it.

  Fist my bump, friend! Is correct!

  ✓ Rust async/await — stability increased to 6.8 · recall now at 94%
```

**`rocky view` after the quiz:** Rust async/await snaps from red back to gold. Click the node — the detail panel now shows **two review entries**: the Q&A from April 1 (initial stability 4.5) and today's harder question on April 8 (stability 6.8 — deeper embedding). Each entry shows the question, your answer, Rocky's feedback, and the score. The decay curve between those two points is visible in the timeline scrubber.

---

## Seeding from git history: `rocky backfill`

A week in, you realize you want your PKG to reflect *everything* in the git history — not just the topics you happened to ask Rocky about interactively.

```bash
~/taskify $ rocky backfill --limit 10

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Taxonomy skeleton ready.
  Scanning last 10 commits by alex@example.com (10 commits)…

  Project: Rust REST API — task management backend with JWT auth,
           PostgreSQL, Redis caching, and Docker deployment.

  [1/10] a3f8c12 init: Axum server scaffold with tokio runtime — no new topics
  [2/10] b7d4e19 feat: sqlx PgPool + migration runner — no new topics
  [3/10] c1a2d83 feat: JWT auth middleware with refresh token rotation — no new topics
  [4/10] d9f3b41 feat: Redis caching layer for user sessions — no new topics
  [5/10] e4c8a27 feat: per-user rate limiting with Redis sorted sets — no new topics
  [6/10] f2b7e94 feat: Docker multi-stage build — 2 new
    + Docker multi-stage builds
    + container image optimization
  [7/10] g8d1c35 feat: GitHub Actions CI pipeline — 2 new
    + GitHub Actions workflow syntax
    + CI/CD pipeline design
  [8/10] h5e4b72 feat: OpenAPI spec with utoipa — 1 new
    + OpenAPI specification
  [9/10] i3f6d28 fix: handle expired tokens in middleware — no new topics
  [10/10] j7a9c14 docs: README and API documentation — no new topics

  ◈ Generating edges for 5 new topics…

  ✓ Added 5 new topics · 12 already in PKG
  Run  rocky quiz  to start reviewing them.
```

Rocky tagged each new node as `taskify`, set `created_at` and `last_reviewed` to the actual commit date (not today), and generated a canonical question and ideal answer from the diff context. The Docker topic from March is already decaying correctly — its retrievability is calculated from March, not April.

```bash
~/taskify $ rocky stats

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  17
  Known:         12
  Fading:        4
  Gaps/weak:     1

  Quiz budget: 3/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  Edges: 21 total  ·  Most connected: JWT authentication (6 edges)
```

**`rocky view` after backfill:** Five new nodes appear, clustering near the DevOps and Architecture taxonomy anchors. Docker multi-stage builds links to container image optimization. GitHub Actions workflow syntax connects to CI/CD pipeline design. The graph now spans five distinct clusters. Filter by "DevOps" in the domain filter to highlight only that cluster and its edges.

```graphlink
graphs/stage7.html|Open interactive graph: after backfill
```

---

## Backfilling a second project

Your PKG lives at `~/.rocky` — it spans all your projects. Change directory to another repo and run `rocky backfill` there. Rocky will detect a different project name, summarise its README, and tag all new topics accordingly.

```bash
~/taskify $ cd ~/home-bank
~/home-bank $ rocky backfill

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Taxonomy skeleton ready.
  Scanning all commits by alex@example.com (66 commits)…

  Project: Python data pipeline that transforms raw bank statements into
           standardised, categorised transactions — supports CSV/PDF ingestion,
           multi-account double-entry bookkeeping, and Pandas-based analysis.

  [1/66] f3d26e5 Initial commit: Enhanced home banking system — 3 new
    + CSV parsing
    + transaction categorisation
    + double-entry bookkeeping
  [2/66] 5a92f2b feat: Add transaction categorization script — 2 new
    + Pandas DataFrame operations
    + project structure
  ...

  ◈ Generating edges for 12 new topics…

  ✓ Added 12 new topics · 17 already in PKG
  Run  rocky quiz  to start reviewing them.
```

Now open `rocky view`. Two repo filter buttons appear in the controls — **taskify** and **home-bank**. Click **home-bank** and:

- Only the 12 home-bank topics are shown, with edges between them
- The project summary appears at the top: *"Python data pipeline that transforms raw bank statements..."*
- Click **CSV parsing** — the detail panel shows the canonical question Rocky generated from the initial commit diff: *"Your CSV parser works on sample files but fails silently on production exports from a different bank. What are the three most common CSV format variations that break naive parsers, question?"*

Click **All** to return to the full cross-project view. The timeline scrubber now shows topics appearing from October 2025 (home-bank's first commit) through April 2026 (taskify's latest).

---

## Viewing the full graph

```bash
~/taskify $ rocky view
# ✓ Written to ~/.rocky/view.html
# → Opening in browser...
```

The interactive graph opens in your browser:

- **Nodes** — colored by recall: gold (≥90%), amber (70–90%), red (<70%)
- **Node size** — proportional to stability (deeper knowledge = bigger node)
- **Edges** — colored by kind: cyan (`implies`), yellow (`depends_on`), red (`conflicts_with`), green (`part_of`)
- **Spread slider** — controls node repulsion; drag right to spread the graph, left to cluster it
- **Domain panel** — left sidebar showing active domains and node counts; click to highlight a cluster
- **Repo filter** — filter the graph to a single project; shows the project summary and only that repo's topics
- **Search** — type "redis" to highlight all Redis-related nodes
- **Node detail panel** — click any node to see retrievability score, stability, all connected edges, review history, and the pre-generated question + ideal answer
- **Edge detail** — click any edge to see the reason Rocky created it
- **Timeline scrubber** — drag the range slider to any date to see what your PKG looked like at that point. Scrub from October 2025 to April 2026 to watch both projects grow in.

The graph is a single self-contained HTML file at `~/.rocky/view.html`.

```graphlink
graphs/stage7.html|Open the full demo graph (21 topics, taskify + home-bank)
```

---

## The full PKG at a glance

```bash
~/taskify $ rocky edges

  SOURCE                         TARGET                         KIND               STR   DESCRIPTION
  ────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  Rust async/await               tokio runtime                  depends_on         0.88  Rust async code requires a...
  tokio runtime                  Axum framework                 depends_on         0.90  Axum is built on hyper/tow...
  sqlx connection pooling        PostgreSQL transactions         implies            0.85  Connection pools are used w...
  JWT authentication             httpOnly cookie security        implies            0.92  JWTs stored insecurely are...
  JWT authentication             token expiry handling           implies            0.89  Issuing JWTs requires handl...
  JWT authentication             Redis sorted sets               implies            0.78  Rate limiting often uses JW...
  Redis TTL expiry               cache invalidation              depends_on         0.75  TTL is one invalidation stra...
  Redis sorted sets              Lua scripting in Redis          depends_on         0.87  Atomic sorted set operations...
  Lua scripting in Redis         Redis sorted sets               part_of            0.82  Lua scripting is used in con...
  Docker multi-stage builds      container image optimization    implies            0.80  Multi-stage builds reduce ima...
  GitHub Actions workflow syntax CI/CD pipeline design           part_of            0.85  GitHub Actions is one CI/CD...
  cache invalidation             JWT authentication              conflicts_with     0.65  Stateless JWTs can't be inva...

  21 edges total
```

This is your knowledge graph for one project, one week in. Each edge is a relationship Rocky inferred from the topics in your code — the ones worth understanding together, not just in isolation.

After backfilling **home-bank**, four more topics appear in the Data domain: CSV parsing, transaction categorisation, Pandas DataFrame operations, and double-entry bookkeeping. They carry commit dates from October 2025 — Rocky knows exactly how long ago you last touched that code.
''';

const kRoadmap = r'''
# Roadmap

These features are planned or under active consideration. They are tracked in [BACKLOG.md](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md) in the repository.

---

## Teaching quality

### Struggle score affects stability

Rocky currently marks a topic as "understood" regardless of how much scaffolding was needed to get there. A topic answered immediately should earn higher stability than one that required three follow-ups and two clues.

**Planned change:** Track a struggle score per review — follow-up count, clue requests, simplification requests. Weight stability gain inversely: a clean first-attempt answer builds stability faster than a scaffolded one.

---

### AI-source tagging

A topic can enter your PKG two ways: from your own code (a diff you wrote, a commit you made) or from an AI-assisted prompt (Rocky saw the topic mentioned in something you asked Claude to build). These are not the same thing. The first comes with struggle and context; the second might come with neither.

**Planned change:** Tag each topic with its origin — `own_code` or `ai_prompt`. Topics tagged `ai_prompt` are quizzed more aggressively. `rocky ls` and `rocky stats` surface them as a distinct category: topics you know about but may never have had to reason through independently.

---

### Independence score

A second axis alongside retrievability: what fraction of this topic's appearances came from your own diffs vs. AI-generated code?

**Planned change:** Track per-topic encounter sources. In `rocky stats`, show a new category — "AI-dependent" — for topics with high retrievability but low independent-code encounters. These are the risk areas: you could ship them confidently and be lost if the AI gets them wrong.

---

### Hard mode

Rocky provides clues, explanations, and question simplification to help when you're stuck. These are useful — but used too freely, they let you navigate a quiz session without ever sitting with genuine uncertainty.

**Planned change:** `rocky quiz --hard` disables `[c]` clue, `[?]` explain, and `[s]` simpler for the session. Configurable permanently in `.rocky.toml`:

```toml
[ui]
hard_mode = true
```

You either answer from what you actually know, or you skip and admit the gap.

---

### Debugging-focused question kind

Rocky's Socratic questions ask about implications and consequences. That covers a lot — but the most durable engineering skill is debugging: given something broken, why did it break and how would you find it?

**Planned change:** A new question kind generated at backfill and diff time alongside the canonical question: *given this code change, what would a production bug look like and what would the stack trace tell you?* Distinct from the implication question. Targets the skill AI is least likely to replace.

---

### Pre-task gap framing

When you run `rocky "task description"`, Rocky currently tells you what you know and quizzes you on gaps. The framing could be sharper: these aren't just gaps — they are specifically the topics you are about to ask AI to handle for you.

**Planned change:** For each gap topic in a pre-task session, Rocky labels it as an AI-risk: *"You have a gap on Lua scripting in Redis. This is something AI will likely write for you. Here is the question you should be able to answer before trusting what it produces."* Rocky becomes an explicit check on AI output, not just a learning tool.

---

## Workflow integration

### Rocky as a required PR check

A GitHub Actions workflow that makes Rocky a required status check on pull requests. Before a PR can merge, Rocky analyses the diff, identifies the topics introduced, and checks whether the author has those topics in their PKG above a configurable retrievability threshold.

**Planned design:**

```yaml
# .github/workflows/rocky-check.yml
on: [pull_request]

jobs:
  rocky:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Rocky knowledge check
        uses: NVME-git/rocky-action@v1
        with:
          min_retrievability: 0.7
          pkg_url: ${{ secrets.ROCKY_PKG_URL }}   # your private PKG repo
```

Rocky fetches your PKG, runs the diff against the PR, and reports which topics are below threshold. The check passes when all introduced topics are either known or explicitly marked as accepted gaps.

**GitHub stacked PRs:** When using stacked PRs (where each PR builds on the previous), the Rocky check can be scoped per-layer — each PR must demonstrate understanding of only the topics *it* introduces, not the entire stack. This scales the requirement to the size of the change.

**Configuration in `.rocky.toml`:**

```toml
[pr_check]
enabled = true
min_retrievability = 0.7   # topics below this threshold block merge
allow_bypass = false        # if true, author can override with a reason
notify_reviewer = true      # post a comment showing topic scores
```

---

## Graph enrichment

### Scheduled enrichment passes

As Rocky adds new node fields (canonical clue, debugging questions, independence score), existing nodes need to be retroactively enriched. Currently this requires manual commands (`rocky backfill --fill-clues`).

**Planned change:** A background scheduler that detects missing fields and runs enrichment silently — so quiz time is always fast and the PKG stays current without manual intervention.

---

*All items above are tracked in [BACKLOG.md](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md). If you have ideas or want to contribute, open an issue.*
''';

const kReferences = r'''
# References & Research

Rocky's design is informed by research and practitioner perspectives on learning, memory, and the impact of AI on technical skill. This section collects sources that shaped features or prompted new thinking.

---

## AI, Competence, and the Role of Struggle in Learning

**FreeCodeCamp Podcast — Dr. Mark Mahoney** · April 2026

*Watch the full interview: [youtu.be/Tb6oaEkxtp8](https://youtu.be/Tb6oaEkxtp8?si=W_vu3K-0WknmvOTI)*

Dr. Mark Mahoney is a computer science professor at Carthage College with over 20 years of teaching experience and the creator of [Playback Press](https://playbackpress.com), a platform for interactive programming education. This interview with Quincy Larson on the FreeCodeCamp podcast covers the impact of AI on programming education and professional practice.

---

### The competence vs. confidence gap

Mahoney's sharpest observation: AI tools give learners *confidence* without necessarily giving them *competence*. You can complete a project, see it work, feel good about it — and still be unable to reason through it independently the next time.

This is a new form of tutorial hell. The old version meant watching someone else solve problems without ever solving them yourself. The AI version means having problems solved *for* you, in code that runs, ships, and disappears into your codebase — leaving no trace of the struggle that would have built real understanding.

Rocky's response to this is the core of its design: every topic must pass a Socratic question grounded in consequences, not definitions. But Mahoney's point pushes further — the source of a topic's entry into the PKG matters as much as whether you can answer a question about it. A topic you encountered in your own debugging is different from a topic you saw in AI-generated code. **[AI-source tagging](roadmap)** is the planned feature that makes this distinction explicit.

---

### "The hard way" builds resilience

Mahoney expresses concern that students who rely on AI miss the grind of debugging — the hours spent staring at a problem with no help coming. That struggle isn't just inefficiency; it builds the resilience and pattern recognition that makes a developer effective under pressure.

This maps directly to Rocky's scaffolding options — clues, explanations, simplification. These are useful when genuinely stuck, but used reflexively they become an escape from the productive discomfort that creates competence. **[Hard mode](roadmap)** is the planned response: a configuration that removes the escape routes, forcing genuine engagement or an honest skip.

---

### Debugging as the durable skill

When asked which skills AI won't replace, Mahoney's answer is immediate: problem-solving and debugging. Not because AI can't debug — it can — but because the *judgment* to know when an AI's debug is wrong requires the same forensic instincts that only come from having debugged things yourself.

Rocky's current question format asks about implications and consequences. **[Debugging-focused questions](roadmap)** extend this into forensic territory: given this code change, what would a production failure look like? What would the stack trace tell you? These questions can't be answered by pattern-matching on documentation — they require the kind of thinking Mahoney identifies as durable.

---

### Iterative planning before code

Mahoney describes his own AI workflow: use the tool to iterate on a *plan* first, refuse to let it generate code until the plan is solid. This disciplines the collaboration — the developer stays in the decision seat, and the AI handles execution within defined constraints.

Rocky's pre-task mode (`rocky "task description"`) already reviews what you know before work begins. **[Pre-task gap framing](roadmap)** makes the AI-risk dimension explicit: for each gap topic, Rocky tells you *this is something AI will likely write for you — here is the question you should be able to answer before trusting the output*. The pre-task session becomes a readiness check for supervised AI use, not just a general review.

---

### Motivation as the irreplaceable human element

Mahoney's view of his primary role as a professor: not to deliver information — AI can do that — but to motivate, inspire passion, and model what it looks like to care deeply about the craft. An LLM can explain recursion; it cannot make a student feel that recursion is worth understanding.

Rocky takes a different angle on this: Rocky the alien is enthusiastic, direct, and genuinely invested in your progress. The personality isn't decoration — it's an attempt to make the quiz feel like a conversation with someone rooting for you, not a test you're taking alone. That's a limited version of what Mahoney describes, but it's the right direction.

---

### Rocky as a required PR check

One idea that emerged from this discussion: if AI is handling more and more of the code in a pull request, what guarantees does a reviewer have that the author understands what they're merging?

The conventional answer is code review. But code review is good at catching logic errors, not at detecting whether the author could reason through the code without the AI that wrote it.

Rocky's PR check (**[planned feature](roadmap)**) addresses this at the workflow level: before a PR can merge, Rocky verifies the author's PKG shows adequate recall on the topics introduced. Paired with GitHub's stacked PRs feature — where PRs build on each other in a reviewable stack — this creates a layer-by-layer knowledge check: each PR in the stack must demonstrate understanding of only what it introduces.

This doesn't slow down shipping. It makes the assumption behind shipping — *"the author knows what this does"* — verifiable.

---

*Have a paper, talk, or post that shaped your thinking on learning and AI? Open an issue or PR.*
''';
