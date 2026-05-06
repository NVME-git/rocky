// All documentation content as markdown strings.
// Each constant corresponds to one section of the docs.

const kIntroduction = r'''
## Why it exists

Every senior engineer knows the feeling: you stop writing something by hand, and six months later you can't remember how it works without looking it up. That's normal. That's how memory works.

What's new is the speed. AI tools don't just accelerate your output — they remove the need to think through problems entirely. You describe what you want, the AI writes it, you ship it. Fast. But the understanding that used to come from doing the work yourself? That doesn't happen anymore.

This is **AI skill atrophy** — and it compounds silently. You don't notice it until the day the AI gives you the wrong answer and you can't tell.

Rocky exists for one reason: **to shine a light on blind spots in systems developers are responsible for — systems co-created with AI agents.**

---

## What makes it different

Most learning tools quiz you on definitions. Rocky doesn't care if you can define JWT. It cares whether you'd catch the bug.

Instead of asking "what is a refresh token?", Rocky asks:

> *You're issuing JWTs with a 15-minute expiry. A user is halfway through a checkout flow when their token expires. What happens — and how do you make the experience seamless without storing session state on the server, question?*

That's the kind of question that separates someone who read the docs from someone who's actually built with it. Rocky asks the second kind — every time.

---

## How it works

Rocky's primary mode is alongside your AI agent. When Claude Code is doing the work, Rocky watches what got built, extracts the topics that live in the changes, and turns each one into a question you'll need to be able to answer for the system you now own.

### With your AI agent (Claude Code)

This is the canonical Rocky workflow. After `rocky install claude`, the agent picks up four skills it can invoke at the right moments — no separate Ollama call, no per-prompt logging in the critical path:

- `/rocky-checkpoint` — at the end of a session, the agent reads the diffs and transcript, extracts topics with question banks, and writes them straight into your PKG using the cross-project dedup list.
- `/rocky-quiz` — runs a Socratic review session inside the Claude session, using the canonical questions Rocky stored.
- `/rocky-backfill` — seeds the PKG from a project's existing git history when you're new to a repo or first installing Rocky.
- `/rocky-promptiq-rescore` — re-evaluates your recent prompts and produces a PromptIQ score with feedback.

```bash
rocky install claude     # one-time: drops the skills into ~/.claude/skills/
# ... work normally with the agent ...
# inside Claude: /rocky-checkpoint  (or /rocky-quiz any time)
```

The alternative workflows below exist for the moments you're not in an agent session — useful supplements, not the main story.

:::details Alternative workflows
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

### Pre-commit review

Check your staged changes before you commit. Useful when you've been working with an AI and want to make sure you actually understand what's about to land.

```bash
rocky diff --staged
```
:::

---

## The knowledge model

Rocky tracks a **Personal Knowledge Graph (PKG)** — a local database of every topic you've encountered. Each topic has a **recall** score: a number between 0 and 1 that combines two things:

- **Retrievability (R)** — freshness from spaced-repetition decay. High right after a review, lower as time passes.
- **Mastery (M)** — the mean of your last three review scores (default 0.5 if you've never been quizzed). Captures *how well you actually answered*, not just how recently.

`recall = R × M`. Freshness alone doesn't count as knowing — if you got the wrong idea last time, retrieving it quickly today doesn't help you. Both factors have to be high before Rocky considers a topic *known*.

| Bucket | recall | What Rocky does |
|---|---|---|
| Known | ≥ 0.6 | Stays quiet |
| Fading | 0.3 – 0.6 | Surfaces it for review |
| Gap | < 0.3 | Asks a question |

Knowledge decays. A topic you understood deeply three months ago might be at 25% today — Rocky surfaces it before you trip over it.

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
| **Known** | Recall ≥ 0.6 — Rocky stays quiet |
| **Fading** | Recall 0.3 – 0.6 — Rocky surfaces it as a review candidate (called "stale" internally) |
| **Gap** | Recall < 0.3 — Rocky asks a question |
| **Retrievability (R)** | Spaced-repetition freshness — decays with time since the last review |
| **Mastery (M)** | Mean of your last three review scores (default 0.5 if never quizzed) |
| **Recall** | `R × M` — the combined score Rocky classifies on |
| **Stability** | How deeply embedded the topic is — higher stability means slower decay |
| **Initial stability** | Set by topic kind when first created: Concept = 4.0, Pattern = 2.5, Implementation = 1.5 |
| **Domain** | One of 13 taxonomy categories (Language, Auth, Database, DevOps, etc.) |
| **Edge** | A relationship between two topics in the PKG — generated automatically by Rocky after new topics are added |
| **Edge kind** | The type of relationship: `implies`, `depends_on`, `conflicts_with`, or `part_of` |
| **Cross-concept question** | A question that bridges two related topics — asked when Rocky detects a relevant edge and both topics have strong recall |
| **Repos** | The git projects a topic has been encountered in. Topics merge by canonical name across projects, so `repos[]` accumulates as the same idea reappears in different repos |
| **Question bank** | Up to ~4 implication-grounded questions stored per topic at extraction time. Surfaced in the web UI, picked from at quiz time |
| **Canonical clue** | A short hint stored alongside the question bank — shown when you type `c` during a quiz |
''';

const kInstallation = r'''
# Installation

Rocky is a single binary with no external dependencies. You download it and it just works.

## Requirements

- A terminal
- **Claude Code** (recommended) or **[OpenCode](https://opencode.ai)** — Rocky's primary mode runs as skills inside your coding agent, using its context window for extraction. Rocky itself doesn't call an LLM in this flow, so **no API key is needed**.

For workflows outside an agent session — running `rocky` from a terminal, or a plain post-commit hook without the Claude Code skills — you can optionally configure a standalone LLM provider. See **Alternative workflows** at the bottom of this page.

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

## Verify the installation

```bash
rocky stats
```

You should see a banner and an empty PKG:

```
  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  0
  Known:         0
  Fading:        0
  Gaps/weak:     0

  PKG is empty. Let us begin science, question?
```

---

## Set up Claude Code (recommended)

This is the canonical Rocky workflow. One command wires up the four skills, the prompt-logging hook, and the git queue:

```bash
rocky install claude-all
```

`claude-all` drops these four skills into `~/.claude/skills/`:

- `/rocky-checkpoint` — end-of-session: read recent diffs + transcript, extract topics with **generic question banks** (no repo-specific identifiers), merge into the PKG (using the global dedup list so the same idea across projects becomes one node with multiple `repos[]`).
- `/rocky-quiz` — Socratic review inside the Claude session, picking from the canonical question bank.
- `/rocky-backfill` — one-shot seeder for projects that already had history when Rocky was installed.
- `/rocky-promptiq-rescore` — re-evaluates your recent prompts and produces a PromptIQ score with feedback.

It also enables prompt logging so the skills can see your session transcript, and rewires the git post-commit hook so commits queue diffs (which `/rocky-checkpoint` drains) instead of running an LLM directly.

For OpenCode or another compatible coding agent, point it at `~/.claude/skills/` (or copy the skill files into the agent's equivalent directory) — the skills are agent-generic markdown.

Granular install options if you want pieces individually:

```bash
rocky install skills      # just the skills, into ~/.claude/skills/
rocky install claude      # just the prompt-logging hook
```

You're ready. Head to **Quick Start** for your first session.

---

:::details Alternative workflows (standalone CLI without an agent)
These are for when you want Rocky to run outside an agent session — say, a plain `git commit` triggers `rocky diff`, or you run `rocky quiz` from a terminal without Claude Code open. Rocky needs its own LLM provider to do this.

### Plain git post-commit hook

Skip the queue mode and have `rocky diff` run directly after each commit:

```bash
rocky install        # or: rocky install git
```

Requires a provider configured below.

### Provider option A: Ollama (free, runs locally)

The recommended standalone provider. Local, free, no API key.

1. Install Ollama from [ollama.com](https://ollama.com)
2. Pull a model:

```bash
ollama pull qwen2.5-coder:7b
```

3. Create `~/.config/rocky/config.toml`:

```toml
[llm]
provider = "ollama"
model = "qwen2.5-coder:7b"   # or "llama3.1:8b" for general-purpose
```

See the **Configuration** page for model recommendations by GPU VRAM.

### Provider option B: Anthropic (Claude API)

Use this if you don't want to run a local model and you'd rather pay per-token.

1. Get an API key from [console.anthropic.com](https://console.anthropic.com)
2. Set it in your environment:

```bash
# Add to ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

Or create a `.env` file in your project directory.

### Legacy: per-turn Stop hook

```bash
rocky install stop
```

Auto-extraction on every Claude turn via Ollama. Not wired by `claude-all` — the per-turn latency was a real complaint — but available as opt-in.

> Coming from a Rocky version that wrote `~/.rocky/.rocky.toml`? It auto-migrates on first run.
:::
''';

const kQuickstart = r'''
# Quick Start

This walks you through your first session with Rocky in about 5 minutes — using the recommended Claude Code (or OpenCode) flow.

> Haven't installed yet? Start at **Installation** — `rocky install claude-all` wires up everything you need before this page makes sense.

---

## Step 1: Work normally with your AI agent

Open Claude Code in a project, work on a real task. Don't change anything about your normal flow.

Behind the scenes:

- **Each commit** silently appends its diff to `./.rocky/queue/`. No LLM call on commit — there's no interruption to your flow.
- **Each prompt** is appended to `./.rocky/prompts.jsonl`. The skills will read this transcript later to ground topic extraction in your actual session.

After a real chunk of work, you'll have a queue and a transcript ready for extraction.

---

## Step 2: End of session — extract topics

In the same Claude Code session, type:

```
/rocky-checkpoint
```

The agent reads the queued diffs + your prompt transcript, identifies the new concepts that came up, and writes them to your PKG with a four-question bank each — generic enough that the same topic resurfacing in a different project still matches.

What you get:

- **New nodes** for each distinct concept the session introduced.
- **Updated nodes** when something already in your PKG appeared again — Rocky bumps the encounter count and refreshes its weighting.
- **Cross-project dedup** — the same idea across two projects becomes one node with multiple `repos[]` entries, not two duplicates.

No LLM cost on the Rocky side here either. The agent's own context window does the extraction.

---

## Step 3: Quiz in Rocky View (recommended)

The terminal works, but the browser is where Rocky actually shines:

```bash
rocky view
```

A local server starts on `127.0.0.1:<random-port>` and your browser opens straight into the **Dashboard**. The first thing you see is your **Rocky IQ** — a single 0–100 number that summarises how well you'd recall everything in your PKG right now.

Six tabs share the same data: **Dashboard**, **Knowledge Map**, **Review Queue**, **Sessions**, **Projects**, **Saga**. Click **Quiz top 5** in the Review Queue (or **Quiz ▶** on a project card) to start a focused session on your weakest topics. Answers go in via keyboard or the 🎤 button (voice is local via `whisper.cpp`; nothing leaves the machine). The IQ banner updates live as your scores come in.

**Why the browser instead of the terminal?** The graph view turns the PKG from a flat list into something you can actually feel — clusters, gaps, and the shape of what you've learned all become visible. The answer experience is also just nicer: real text wrapping, voice input, instant graph updates, and the IQ dial right there.

### Dashboard

Rocky IQ banner with the live score and gradient bar, recent activity, and weakest topics at a glance.

![Dashboard](screenshots/rocky-dashboard.png)

### Knowledge Map

Interactive 2-hop "wormhole" graph — bounded to ~28 nodes on screen at a time regardless of PKG size. Click any node to warp focus to it. Edges are colour-coded by kind, weakest topics glow for attention.

![Knowledge Map](screenshots/rocky-map.png)

### Review Queue

Flat list of every topic that needs attention, ranked by `recall_now` ascending. **Quiz top 5** is a single click — the most direct path from "open the app" to "start drilling."

![Review Queue](screenshots/rocky-queue.png)

### Sessions

What landed in your PKG day by day. Useful for "what did I learn this week?" — and for spotting weeks where nothing went in (a good sign you should check in).

![Sessions](screenshots/rocky-sessions.png)

### Projects

Per-project breakdown — which repos contributed which topics, with cross-project dedup made visible. **Quiz ▶** scopes a session to a single project.

![Projects](screenshots/rocky-projects.png)

The **Saga** tab is a cinematic timelapse of your knowledge graph growing over time — gated behind ~30 days of consistent reviewing as a reward, so it doesn't get a screenshot here.

---

## Step 4: Quick inline review without leaving the agent

If you'd rather not switch contexts, you can quiz directly inside Claude Code:

```
/rocky-quiz
```

The agent picks your weakest topics by `recall_now`, asks one question per topic from the canonical question bank, and writes the score back. Same FSRS state advancement as Rocky View — just text-only and inline.

This is best for *quick* reviews — a couple of questions between tasks. For a real study session, open Rocky View.

---

## What happens over time

Rocky uses FSRS — the same family of spaced-repetition algorithm Anki moved to. Topics you know well decay slowly. Topics you barely know decay fast. Over time, Rocky surfaces the right things at the right moments without spamming you.

Your **Rocky IQ** is built from `recall_now = retrievability × mastery` averaged across your PKG. It moves down when you stop engaging and up when you can answer for what you've shipped. There's no daily cap on `/rocky-checkpoint` or `rocky view` — you control when you extract and when you review.

---

:::details Alternative workflows (without an agent)
If you're not in an agent session and still want to use Rocky, the original commands still work — they just need a standalone LLM provider configured (see **Installation → Alternative workflows**).

### Pre-task quiz

```bash
rocky "add user login with JWT tokens to my Express API"
```

Rocky extracts topics from the description and quizzes you on anything new or fading — before you've touched a line of code.

### Post-commit quiz

```bash
rocky diff           # last commit
rocky diff HEAD~3    # specific commit
rocky diff --staged  # before committing
```

### On-demand terminal quiz

```bash
rocky quiz           # full review
rocky quiz "redis"   # search and pick topics
```

### Q&A options (terminal)

At any question:

- **Type your answer** and press Enter
- **Press Enter** with nothing — skip and queue for later
- **`s`** — simpler — regenerate at lower difficulty
- **`h`** — harder — regenerate with edge cases
- **`c`** — clue — short hint
- **`?`** — explain it — reveal the ideal answer (records 0.0)
- **`x`** — delete the topic (Rocky confirms first)

Rocky View has the same options as buttons in the quiz modal.
:::
''';

const kCommands = r'''
# Commands

The recommended Rocky workflow runs almost entirely through your AI agent. After `rocky install claude-all`, the **agent skills** below are what you use day-to-day.

The CLI surface beneath those skills is also documented here, but most of it falls under **Alternative workflows** at the bottom — useful for scripting, backups, or running Rocky standalone, but not part of the default loop.

---

## Agent skills (Claude Code / OpenCode)

You invoke these by typing `/skill-name` inside an active agent session — not from the shell.

### `/rocky-checkpoint`

End-of-session extractor. The agent reads recent diffs queued by the post-commit hook plus the prompt transcript, identifies the new concepts, and writes them to your PKG with a four-question bank each.

- **Cross-project dedup** — the same idea across two repos becomes one node with `repos[]` accumulating.
- **Agent-generic question banks** — stripped of repo-specific identifiers, so the same question still works when the topic resurfaces elsewhere.
- **Idempotent** — running it twice on the same queue is a no-op.

Internally the skill composes `rocky checkpoint diff`, `rocky context`, `rocky add-topic`, `rocky add-question`, and `rocky checkpoint mark`. See the **PKG primitives** section in Alternative workflows if you want to build your own variant.

Run after a focused work session.

### `/rocky-quiz`

Inline review session inside the agent. Picks the topics with the lowest `recall_now`, asks one question per topic from the canonical question bank, and writes scores back via FSRS.

Best for short reviews between tasks. For a real session, open `rocky view` — the browser is just nicer for actual study.

Internally: `rocky due --limit N` → `rocky topic <name>` → `rocky review`.

### `/rocky-backfill`

One-shot seeder for a project that already had commits when you installed Rocky. Reads commit history (`rocky checkpoint history`) and walks it the same way `/rocky-checkpoint` walks the post-commit queue.

Run **once** per project after install, then forget it exists.

### `/rocky-promptiq-rescore`

Re-evaluates your recent prompts using the agent's own judgment and writes a **PromptIQ** score (0–100) and short feedback per prompt. Surfaces patterns: prompts that are too vague, prompts that don't include enough constraints, prompts that ask the agent to invent context it should be told.

PromptIQ is the first sibling of Rocky IQ in the IQ family — it measures the *input* side of the agent loop, where Rocky IQ measures the *output* side.

---

## CLI essentials

The minimum CLI surface for a first-time user. Everything else is in **Alternative workflows** below.

### `rocky install claude-all`

The one-time install. Wires the four skills, the prompt-logging hook, and the queue-mode post-commit hook in one shot.

```bash
rocky install claude-all
rocky uninstall claude-all
```

See **Installation** for granular sub-targets (`rocky install skills`, `rocky install claude`, etc.) when you want pieces individually.

### `rocky stats`

Quick PKG summary. Useful right after install to confirm everything's wired, and any time you want a one-line "where am I" check.

```bash
rocky stats

  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  24
  Known:         15
  Fading:         6
  Gaps/weak:     3

  Edges: 18 total  ·  Most connected: JWT authentication (4 edges)
  Weakest: SQL injection prevention (recall 18%)
```

The **Weakest** line gives you a one-line "next thing to drill" pointer without opening anything else.

### `rocky view`

Open the web UI. **This is the recommended quiz interface** — see **Quick Start → Step 3** for the full walkthrough.

```bash
rocky view
# ✓ Rocky running at http://127.0.0.1:XXXXX
```

Six tabs: **Dashboard** (Rocky IQ banner), **Knowledge Map** (interactive 2-hop wormhole graph), **Review Queue** (one-click *Quiz top 5*), **Sessions**, **Projects**, **Saga** (cinematic timelapse). Voice input via the 🎤 button uses local `whisper.cpp`.

Environment overrides:

- `ROCKY_NO_OPEN=1` — skip auto-opening the browser (useful when iterating on the UI source).
- `ROCKY_BIND=0.0.0.0:7777` — expose the UI off-loopback (used by the Docker image).

### `rocky sync` / `rocky restore`

Backup and cross-machine restore. Read **Vault Sync & Backup** for the full setup.

```bash
rocky sync          # commit pending PKG changes locally
rocky sync --push   # commit and push to your configured remote
rocky restore       # rebuild graph.db from pkg/pkg.json on a fresh machine
```

---

:::details Alternative workflows (terminal-only, no agent)
Everything below is for using Rocky without an agent. Most of these need a standalone LLM provider configured (see **Installation → Alternative workflows**).

### `rocky "task description"`

Describe what you're about to work on. Rocky extracts the key topics and quizzes you on anything new or fading.

```bash
rocky "add pagination to my REST API"
rocky "set up CI/CD with GitHub Actions"
rocky "migrate the database from SQLite to Postgres"
```

No daily limits — you asked for it.

### `rocky --after "message"`

Review topics from a task you already completed or a commit message you wrote.

```bash
rocky --after "added Redis caching to the user service"
```

Useful when you worked with an AI and want to review what just got built, without a diff.

### `rocky diff`

Analyse a git diff and quiz on topics found in the actual code changes.

```bash
rocky diff              # last commit
rocky diff HEAD~3       # specific commit
rocky diff abc1234      # by hash
rocky diff --staged     # staged changes before committing
```

This is more powerful than `--after` — Rocky reads the actual code, not just the message. Auto-triggered via the standalone `rocky install` git hook; manual calls have no daily limits.

### `rocky quiz`

Terminal review session. Works through three sources in priority order:

1. Topics you previously skipped (queued in `./.rocky`)
2. PKG topics with the lowest recall (most overdue for review)
3. New topics found in recent Claude Code prompts (last 24 hours)

```bash
rocky quiz                  # general review
rocky quiz --hours 48       # look back further for prompt context
rocky quiz "redis"          # search and pick specific topics
```

Manual quiz calls always run — no daily limits.

#### Q&A keystrokes (terminal)

| Input | What happens |
|---|---|
| Type your answer + Enter | Rocky evaluates and gives feedback |
| Enter (blank) | Skip — topic is queued for later |
| `s` | Simpler — regenerate the question at lower difficulty |
| `h` | Harder — regenerate the question with edge cases and deeper implications |
| `c` | Clue — show a short hint without giving away the answer |
| `?` | Explain it — reveal the ideal answer and record a 0.0 score (you didn't get it) |
| `x` | Delete the topic (Rocky asks for confirmation before removing) |

The web UI exposes the same options as buttons in the quiz modal. Each question is labelled as either **bank** (pre-generated at extraction time) or **live** (generated now). Bank questions are sharper because they were grounded in real diffs.

### `rocky ls`

Full topic list with all metrics — the terminal version of the Knowledge Map / Review Queue tabs in `rocky view`.

```bash
rocky ls

  Topic                           Kind           Recall         Stab   Diff  Reviews  Last Reviewed
  ────────────────────────────────────────────────────────────────────────────────────────────────
  JWT authentication              pattern        ██████████ 97%  8.2    0.3   3        2026-04-03
  Redis TTL expiry                implementation ██████░░░░ 63%  3.0    0.5   1        2026-03-10
  database indexing               concept        ████░░░░░░ 42%  1.8    0.6   1        2026-02-15
```

Columns:

- **Recall** — `retrievability × mastery`
- **Stab** — stability (how embedded; higher = decays slower; initial value set by kind)
- **Diff** — difficulty (how hard you've historically found it)
- **Reviews** — number of times quizzed

### `rocky queue`

Show topics queued in this project (skipped during a session, not yet added to the PKG). Picked up automatically next time you run `rocky quiz`.

```bash
rocky queue
```

### `rocky logs`

Show recent prompts logged in this project from the Claude Code hook (last 24 hours). Useful for seeing what you've been asking your AI to build before running `rocky quiz`.

```bash
rocky logs
```

`rocky quiz` reads from this same log automatically.

### `rocky export`

Manually write all PKG topics to your Obsidian vault as Markdown files. Rocky also exports automatically every time a topic is updated. See **Obsidian Integration**.

```bash
rocky export
```

### `rocky classify`

Assign taxonomy domains to any topics that don't have one. Useful for topics created before domain support was added.

```bash
rocky classify
```

The 13 domains: Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other.

### `rocky backfill`

Standalone equivalent of `/rocky-backfill` — scan git history and silently add topics. No interactive Q&A.

```bash
rocky backfill                                # your commits in the current repo
rocky backfill --all-authors                  # include collaborators
rocky backfill --limit 50                     # cap commit count
rocky backfill --fill-clues                   # backfill missing clues for legacy nodes
rocky backfill --fill-question-bank           # backfill missing question banks for legacy nodes
```

Initial mastery defaults to 0.5, so backfilled topics land in the *fading* bucket — backfill alone isn't a quiz. You still need to review.

Each new topic is enriched at insertion time with the actual commit date (so decay starts from when you first encountered it), the repo tag, canonical Q&A, and a cached project summary in `~/.rocky/summaries/<repo>.txt`.

### `rocky delete`

Remove topics by name (substring match) or date range. Always confirms before deleting.

```bash
rocky delete "jwt"                                            # by name
rocky delete --since 2026-04-01                               # by date
rocky delete --before 2026-01-01
rocky delete "redis" --since 2026-03-01 --before 2026-03-31   # combined
```

### `rocky config`

Print the active configuration.

```bash
rocky config
```

### `rocky edges`

List or summarise inferred relationships in the PKG.

```bash
rocky edges                  # full list
rocky edges --stats          # totals by kind
```

Edge kinds: `implies`, `depends_on`, `conflicts_with`, `part_of`.

### `rocky feedback`

Open `~/.rocky/FEEDBACK.md` in `$EDITOR` to write your thoughts on Rocky. The same file is shown in the web UI's Feedback tab.

```bash
rocky feedback
```

### Standalone install variants

Beyond `claude-all`, the install command has narrower targets for non-agent setups:

```bash
rocky install                  # plain git post-commit hook (calls rocky diff directly, not queue mode)
rocky install git              # same as above, explicit
rocky install prompt           # prompt log only, no git hook
rocky install stop             # legacy: per-turn Stop hook → Ollama auto-extraction (high latency)

rocky uninstall                # remove git hook
rocky uninstall git | claude | claude-all | skills | prompt | stop
```

`rocky install` (no subcommand) gives you a plain post-commit hook that calls `rocky diff` directly — the pre-`claude-all` flow. Requires a standalone LLM provider.

---

### PKG primitives (called by skills, also useful by hand)

The skill-driven extraction in `claude-all` is built on small scriptable commands. The four `/rocky-*` skills compose them; you can call them directly when you want fine control or for ad-hoc tooling.

#### `rocky add-topic`

Insert a topic into the PKG without going through LLM extraction.

```bash
rocky add-topic --name "CRDT merge semantics" \
                --description "Conflict-free replicated data types: state merges are commutative and idempotent so concurrent edits converge without coordination." \
                --kind concept \
                --domain Architecture \
                --context "$(git show HEAD -- src/replicate.rs)" \
                --commit "$(git rev-parse HEAD)" \
                --score 0.7
```

The current repo is detected from the git remote, so `repos[]` accumulates automatically — call from any project and the same canonical name lands as one node with multiple repos.

#### `rocky add-question`

Append a question to an existing topic's question bank. Idempotent on the question text.

```bash
rocky add-question --topic "CRDT merge semantics" \
                   --question "Why can't last-write-wins replace a CRDT?" \
                   --answer "LWW silently drops concurrent edits; CRDTs preserve them via commutative merge." \
                   --clue "Think about concurrent edits."
```

#### `rocky delete-topic`

Remove a topic by name (substring match — must resolve to exactly one). FK-cascades to reviews and contexts.

```bash
rocky delete-topic "JWT authentication"
```

#### `rocky context`

Emit the cached project context summary (the one `rocky explore` produces) as JSON. The checkpoint skill reads this to ground extraction in the project's vocabulary.

```bash
rocky context
```

#### `rocky checkpoint`

Three subcommands the `/rocky-checkpoint` and `/rocky-backfill` skills use:

```bash
rocky checkpoint diff                                # JSON dump of queued post-commit diffs (read-only)
rocky checkpoint mark                                # drain the queue (call after extraction succeeds)
rocky checkpoint history --limit 50                  # JSON dump of recent commits (newest first)
rocky checkpoint history --limit 200 --all-authors   # include collaborators' commits
```

`history` returns the same envelope shape as `diff` — `commits[]` with `sha`, `subject`, `message`, `diff` — so the same skill iteration works for both.

#### `rocky due`

JSON list of topics ranked by `recall_now` ascending (most-overdue first).

```bash
rocky due --limit 10
```

Pipe into `jq` to slice.

#### `rocky topic`

Look up a single topic by name and print its full record as JSON: kind, domain, recall, mastery, question bank, repos, contexts, review history.

```bash
rocky topic "CRDT merge semantics"
```

#### `rocky review`

Record a review score for a topic without going through interactive Q&A. The quiz skill uses this to write back after each question.

```bash
rocky review "CRDT merge semantics" \
             --score 0.8 \
             --question "Why can't last-write-wins replace a CRDT?" \
             --answer "..." \
             --feedback "Good — covered partial ordering correctly."
```

#### `rocky list --json`

Same data as `rocky ls`, emitted as the full PKG JSON.

```bash
rocky list --json | jq '.[] | select(.recall_now < 0.3)'
```

#### `rocky hook`

Called automatically by the Claude Code prompt-logging hook. Don't run manually.
:::
''';

const kConfiguration = r'''
# Configuration

Rocky looks for config files in two places, applied in this order (later overrides earlier):

1. `~/.config/rocky/config.toml` — your global settings, applies everywhere
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
enabled = true' >> ~/.config/rocky/config.toml

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

## Claude Code integration

The recommended setup wires Rocky's two skills, the prompt-logging hook, and the queue-mode git hook in one shot:

```bash
rocky install claude-all
rocky uninstall claude-all
```

After install, type `/rocky-checkpoint` in any Claude session to extract topics into the PKG, `/rocky-quiz` to drill the weakest, or `/rocky-backfill` to seed the PKG from a project's existing git history with generic question banks. All three skills operate on the global PKG (cross-project dedup via `repos[]`).

If you want pieces:

| Command | What it installs |
|---|---|
| `rocky install skills` | `~/.claude/skills/{rocky-checkpoint,rocky-quiz,rocky-backfill,rocky-promptiq-rescore}/SKILL.md` |
| `rocky install claude` | Just the prompt-logging entry in `~/.claude/settings.json` |
| `rocky install stop`   | Legacy: Stop hook → Ollama auto-extract on every Claude turn |

---

## Where Rocky stores data

| Path | What it is |
|---|---|
| `~/.config/rocky/config.toml` | Your global config |
| `~/.rocky/graph.db` | Your PKG — all topics, recall scores, review history |
| `~/.rocky/pkg/` | Markdown notes + `pkg.json` backup |
| `~/.rocky/pkg/pkg.json` | Full PKG export for backup and cross-machine restore |
| `~/.rocky/summaries/` | Cached project context summaries (`rocky explore`) |
| `./.rocky` | Per-project prompt log (only in hooked projects) |
| `$ROCKY_HOME` | If set, overrides the `~/.rocky/` data dir entirely (useful for testing) |

`graph.db` is never tracked by git. Everything in `pkg/` is tracked when sync is enabled.
''';

const kHowItWorks = r'''
# How Rocky Works

## The pipeline at a glance

Rocky doesn't extract topics one commit at a time. It batches diffs across a whole working session, pairs them with the Claude Code transcript and a cached project summary, and produces **rich nodes with a question bank** (~4 implication-grounded Q+A+clue triples each) — driven by an agent already sitting in your editor.

| Stage | What happens | Where |
|---|---|---|
| `rocky explore` | Reads CLAUDE.md / README / docs / recent commits and synthesises a **project context** summary cached at `~/.rocky/summaries/<repo>.txt` | Run once per project, again after major shape changes |
| `rocky post-commit` (queue mode) | Silently appends the latest commit's diff to a per-project queue. **No LLM call.** | Wired by `rocky install claude-all` |
| `/rocky-checkpoint` (Claude skill) | Drains the diff queue, reads the active session's transcript, and writes nodes + **generic question banks** straight into the PKG. Stripped of repo-specific identifiers so the same question still works when the topic resurfaces in a different project. Uses the **global** dedup list — same topic across two projects becomes one node with `repos[]` accumulating. | Invoked inside Claude Code, end of a session |
| `/rocky-quiz` (Claude skill) | Picks the weakest topics by `recall_now`, asks from the canonical question bank, records scores back to FSRS. | Invoked inside Claude Code any time |
| `/rocky-backfill` (Claude skill) | Seeds the PKG from a project's existing git history. Same extract-and-question loop as `/rocky-checkpoint` but driven by `rocky checkpoint history` instead of the post-commit queue, so it works on repos installed *after* the commits happened. | Invoked once per repo when adopting Rocky on an existing project |

The motivation: a single commit message like *"feat: rotate refresh tokens"* is too thin a context to ground good questions in. By batching at session end, the extractor has the project summary, the actual diffs, **and** the agent's reasoning trail. And by running inside Claude Code itself rather than shelling out to a local Ollama on every Stop event, the per-turn latency drops to zero — the heavy step only happens when you ask for it.

> The legacy Stop-hook → Ollama path still ships and is opt-in via `rocky install stop`. It runs after every Claude turn and is useful if you don't keep a Claude session open the whole day. The skill-driven default is faster and produces sharper extraction because it sees the full transcript at once.

### Inspecting what was generated

The web UI is the canonical viewer — see **Quick Start → Step 3** for screenshots of each tab. The Knowledge Map shows the topic graph; the Review Queue surfaces what's most overdue; the Sessions tab lets you scrub through what landed day by day.

### Cross-project dedup

When the checkpoint skill extracts topics, it's fed the **global** topic list (across every project Rocky knows about) and asked to reuse exact names where a new finding is semantically equivalent. The result: a topic like *Token Rotation* lives as one node with a `repos[]` array that accumulates as the same idea reappears in another codebase.

```
▸ refactor: surface is_rotated helper for refresh token reuse checks
  ◇ Token Rotation (existing — encounter +1, repos: [taskify, home-bank])

Done. 0 new topic(s), 1 encounter update(s).
```

The web UI's **Project** field on a topic shows everywhere it's appeared. Layer-2 (semantic dedup via embeddings + cosine, beyond today's lexical Jaccard) is on the [backlog](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md).

---

## How the PKG classifies topics

Every topic has a **recall** score: `recall_now = retrievability × mastery`.

- **Retrievability (R)** — FSRS freshness from spaced repetition. Decays with time since the last review. Decays slower when stability is high (you've demonstrated solid understanding).
- **Mastery (M)** — mean of the last 3 review scores (default 0.5 if you've never been quizzed). Captures *how well you've actually been answering*, not just how recently.

**Recall lifecycle:**

1. **Topic created** → stability set by kind (Concept 4.0, Pattern 2.5, Implementation 1.5), mastery defaults to 0.5
2. **Topic reviewed** → score 0.0 to 1.0 is recorded. Stability rises if score ≥ 0.65; it floors at 1.0 + difficulty bumps if score < 0.4. The new score replaces the oldest in the last-3 mastery window.
3. **Over time** → R decays. M is sticky until you take another quiz.
4. **Classification:**
   - recall ≥ 0.6 → **Known** — skipped automatically
   - 0.3 ≤ recall < 0.6 → **Fading** — surfaced as a review candidate
   - recall < 0.3 → **Gap** — full Socratic Q&A from the question bank

The multiplicative model is the point: a freshly-reviewed topic where you got the question wrong (R high, M low) is still a gap. Freshness alone doesn't count as knowing.

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

In `~/.config/rocky/config.toml`:

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
rocky_repo: taskify
rocky_status: known
rocky_difficulty: 0.300
rocky_stability: 8.50
rocky_retrievability: 0.9400
rocky_mastery: 0.9000
rocky_recall: 0.8460
rocky_last_reviewed: 2026-04-03
rocky_last_encountered: 2026-04-03
rocky_review_count: 3
rocky_days_since_review: 0
tags: [rocky/node, rocky/kind/pattern, rocky/status/known, rocky/domain/auth, rocky/repo/taskify]
---

# JWT authentication

Stateless token-based auth where the server signs a payload the client stores and sends back.

## Contexts
- add user login with JWT tokens to my Express API

## See also
- [[token-expiry-handling]]
- [[httponly-cookie-security]]
```

The three frontmatter fields that drive Dataview queries:

- `rocky_retrievability` — FSRS freshness alone (R)
- `rocky_mastery` — mean of last-3 review scores (M)
- `rocky_recall` — `R × M`, the field the dashboard pages classify on

`rocky_status` is one of `known | fading | gap`, derived from `rocky_recall` against the same 0.6/0.3 thresholds Rocky uses elsewhere. Use whichever is most ergonomic for your queries.

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
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      round(rocky_mastery * 100) + "%" AS "Mastery",
      rocky_last_reviewed AS "Last Reviewed",
      rocky_kind AS "Kind"
FROM #rocky/node
WHERE rocky_recall < 0.6
SORT rocky_recall ASC
```

### Your strongest topics

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_review_count AS "Reviews"
FROM #rocky/node
WHERE rocky_recall >= 0.6
SORT rocky_review_count DESC
```

### Topics by domain

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/domain/auth
SORT rocky_recall ASC
```

Replace `auth` with any domain name (lowercased).

### Topics by kind

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_last_reviewed AS "Last Reviewed"
FROM #rocky/node AND #rocky/kind/pattern
SORT rocky_recall ASC
```

Replace `pattern` with `concept` or `implementation`.

### Topics from a specific project

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_domain AS "Domain"
FROM #rocky/node AND #rocky/repo/taskify
SORT rocky_recall ASC
```

Replace `taskify` with any repo tag. Note: a topic that's been encountered in multiple projects only carries the *primary* repo as `rocky_repo`; the cross-project view in `rocky view` is authoritative.

### Everything sorted by recall

```
TABLE round(rocky_recall * 100) + "%" AS "Recall",
      rocky_domain AS "Domain",
      rocky_days_since_review AS "Days ago"
FROM #rocky/node
SORT rocky_recall ASC
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
# Enable in config (~/.config/rocky/config.toml)
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

Configure the threshold in `~/.config/rocky/config.toml`:

```toml
[sync]
remind_push_sessions = 5   # every 5 sessions (default)
remind_push_days = 7       # OR every 7 days — pick one, set other to 0
```

Set both to `0` to disable reminders entirely.
''';

const kWalkthrough = r'''
# Demo

A real Rocky session, end to end — `rocky install claude-all`, work with the agent, `/rocky-checkpoint`, then open `rocky view` and watch the IQ banner climb.

```youtube
PLACEHOLDER
```

---

## Try it yourself, no install

Click into a hosted example PKG to feel the UI before you commit to the install:

[Open the example Rocky View →](https://example.rocky.example/)

> The example is read-only. Click any node, open the Knowledge Map, scrub through Saga — same UI you'll get when you run `rocky view` on your own machine.
''';


const kVoice = r'''
# Voice (alpha — push-to-talk)

The web UI now has a 🎤 button next to the answer textarea. Hold it, speak your answer, release — the transcript drops into the textarea so you can edit before submitting. Default backend is fully local: a `whisper.cpp` subprocess Rocky shells out to. Nothing leaves the machine.

## Setup — one line

```bash
curl -fsSL https://raw.githubusercontent.com/NVME-git/rocky/main/scripts/install-whisper.sh | sh
```

The installer:
- Detects platform (`linux-x64`, `linux-arm64`, `macos-arm64`, `macos-x64`)
- Installs `whisper-cli` — via `brew install whisper-cpp` on macOS, building from source on Linux (`cmake` + a C++ compiler required, ~2-5 min)
- Pulls `ggml-base.en.bin` (~142 MB) into `~/.rocky/models/`
- Adds a `[voice]` block to `~/.config/rocky/config.toml`

After it finishes, restart `rocky view` and the mic button is live.

## Two backends

| Backend | Latency | Privacy | Setup |
|---|---|---|---|
| `whisper-cpp` *(default)* | ~1-2 s on CPU | ✓ fully local | install-whisper.sh |
| `browser` *(opt-in)* | real-time | ✗ Chrome → Google, Safari → Apple | flip a config flag |

Browser mode is **forbidden when `privacy.strict = true`** — Rocky won't let you accidentally exfiltrate audio. Enable it explicitly with:

```toml
[voice]
provider        = "browser"
browser_consent = true
```

## What ships in v0.2

- Web UI mic button (push-to-hold, WAV encoded client-side at 16 kHz mono)
- POST `/api/transcribe` endpoint that calls the configured STT provider
- `whisper.cpp` subprocess invocation with clear errors when the binary or model is missing (the web UI surfaces an inline link to the installer)
- One-line installer (`scripts/install-whisper.sh`)

## What's planned for later

- `rocky quiz --voice` — fully hands-free CLI session: Rocky speaks the question via OS TTS, captures your answer with `cpal` + `webrtc-vad` (700ms silence ends an utterance), evaluates, repeats
- In-binary `whisper-rs` build (`cargo install --features voice`) for users who want one binary, no PATH dependency
- Real-time streaming transcription (Pattern C in [ADR 0006](https://github.com/NVME-git/rocky/blob/main/docs/decisions/0006-voice-architecture.md))

There is **no** wake-word / always-on listening — push-to-talk is the only model. By design.

## Disabling voice

```toml
[voice]
provider = "off"
```

Mic button stops working immediately, no rebuild needed.
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

Rocky's response to this is the core of its design: every topic must pass a Socratic question grounded in consequences, not definitions. But Mahoney's point pushes further — the source of a topic's entry into the PKG matters as much as whether you can answer a question about it. A topic you encountered in your own debugging is different from a topic you saw in AI-generated code. **[AI-source tagging](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** is the planned feature that makes this distinction explicit.

---

### "The hard way" builds resilience

Mahoney expresses concern that students who rely on AI miss the grind of debugging — the hours spent staring at a problem with no help coming. That struggle isn't just inefficiency; it builds the resilience and pattern recognition that makes a developer effective under pressure.

This maps directly to Rocky's scaffolding options — clues, explanations, simplification. These are useful when genuinely stuck, but used reflexively they become an escape from the productive discomfort that creates competence. **[Hard mode](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** is the planned response: a configuration that removes the escape routes, forcing genuine engagement or an honest skip.

---

### Debugging as the durable skill

When asked which skills AI won't replace, Mahoney's answer is immediate: problem-solving and debugging. Not because AI can't debug — it can — but because the *judgment* to know when an AI's debug is wrong requires the same forensic instincts that only come from having debugged things yourself.

Rocky's current question format asks about implications and consequences. **[Debugging-focused questions](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** extend this into forensic territory: given this code change, what would a production failure look like? What would the stack trace tell you? These questions can't be answered by pattern-matching on documentation — they require the kind of thinking Mahoney identifies as durable.

---

### Iterative planning before code

Mahoney describes his own AI workflow: use the tool to iterate on a *plan* first, refuse to let it generate code until the plan is solid. This disciplines the collaboration — the developer stays in the decision seat, and the AI handles execution within defined constraints.

Rocky's pre-task mode (`rocky "task description"`) already reviews what you know before work begins. **[Pre-task gap framing](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** makes the AI-risk dimension explicit: for each gap topic, Rocky tells you *this is something AI will likely write for you — here is the question you should be able to answer before trusting the output*. The pre-task session becomes a readiness check for supervised AI use, not just a general review.

---

### Motivation as the irreplaceable human element

Mahoney's view of his primary role as a professor: not to deliver information — AI can do that — but to motivate, inspire passion, and model what it looks like to care deeply about the craft. An LLM can explain recursion; it cannot make a student feel that recursion is worth understanding.

Rocky takes a different angle on this: Rocky the alien is enthusiastic, direct, and genuinely invested in your progress. The personality isn't decoration — it's an attempt to make the quiz feel like a conversation with someone rooting for you, not a test you're taking alone. That's a limited version of what Mahoney describes, but it's the right direction.

---

### Rocky as a required PR check

One idea that emerged from this discussion: if AI is handling more and more of the code in a pull request, what guarantees does a reviewer have that the author understands what they're merging?

The conventional answer is code review. But code review is good at catching logic errors, not at detecting whether the author could reason through the code without the AI that wrote it.

Rocky's PR check (**[planned feature](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)**) addresses this at the workflow level: before a PR can merge, Rocky verifies the author's PKG shows adequate recall on the topics introduced. Paired with GitHub's stacked PRs feature — where PRs build on each other in a reviewable stack — this creates a layer-by-layer knowledge check: each PR in the stack must demonstrate understanding of only what it introduces.

This doesn't slow down shipping. It makes the assumption behind shipping — *"the author knows what this does"* — verifiable.

---

*Have a paper, talk, or post that shaped your thinking on learning and AI? Open an issue or PR.*
''';
