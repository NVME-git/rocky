---
title: "Commands"
order: 4
---
The recommended Rocky workflow runs almost entirely through your AI agent. After `rocky install claude-all`, the **agent skills** below are what you use day-to-day.

The CLI surface beneath those skills is also documented here, but most of it falls under **Alternative workflows** at the bottom — useful for scripting, backups, or running Rocky standalone, but not part of the default loop.

---

## Agent skills (Claude Code / OpenCode)

You invoke these by typing `/skill-name` inside an active agent session — not from the shell.

### `/rocky-review` — the recommended default

Umbrella skill. Runs `/rocky-checkpoint` (extracts learnings from recent commits) followed by `/rocky-promptiq` (rescores recent prompts), then prints a fused two-line summary:

```
/rocky-review complete.

  PKG  · Checkpointed N commits → M new topics, K merged
         (J cross-project), Q questions, E edges.
  Prompts · Rescored N prompts. Avg <score>, lift +<N> vs heuristic.
            Top pattern: <one-line synthesis>.
```

Deliberately runs the two passes **sequentially**, not blended into one — checkpoint is *generative* (mine atomic concepts), promptiq is *critical* (judge how you asked). Switching evaluative stance between two clean runs is cleaner than dragging them through one pass together.

Use this at the end of an interactive session. **For CI / automation, prefer the underlying skills directly** — call `/rocky-checkpoint` after every push and `/rocky-promptiq` on a slower cadence. Bundling them in CI makes a single failure look like two.

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

### `/rocky-promptiq`

Re-evaluates your recent prompts using the agent's own judgment and writes a **PromptIQ** score (0–100) and short feedback per prompt. Surfaces patterns: prompts that are too vague, prompts that don't include enough constraints, prompts that ask the agent to invent context it should be told.

PromptIQ is the first sibling of Rocky IQ in the IQ family — it measures the *input* side of the agent loop, where Rocky IQ measures the *output* side.

---

## CLI essentials

The minimum CLI surface for a first-time user. Everything else is in **Alternative workflows** below.

### `rocky install claude-all`

The one-time install. Wires the five skills (`rocky-review`, `rocky-checkpoint`, `rocky-quiz`, `rocky-backfill`, `rocky-promptiq`), the prompt-logging hook, and the queue-mode post-commit hook in one shot.

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
- `ROCKY_BIND=0.0.0.0:7777` — expose the UI off-loopback (e.g. when serving from a remote box).

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
