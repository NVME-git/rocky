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

Install or remove the git post-commit hook.

```bash
# Install — Rocky runs after every commit in this repo
rocky install

# Remove
rocky uninstall
```

`rocky install` also adds `.rocky` to `.gitignore` so your local prompt log isn't committed.

---

## `rocky export`

Write all PKG topics to your Obsidian vault as Markdown files.

```bash
rocky export
```

Rocky also exports automatically every time a topic is updated. See [Obsidian Integration](obsidian.md).

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
# Run  rocky export  to update vault files.
```

The 13 domains Rocky uses: Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other.

---

## `rocky sync`

Commit any pending vault changes to git. Shows what changed.

```bash
rocky sync
# ✓ Rocky: 15/24 known — 2026-04-04
```

```bash
# Commit and push to the configured remote
rocky sync --push

# Initialise git repo (optionally set a remote at the same time)
rocky sync --init
rocky sync --init https://github.com/you/rocky-vault.git

# Show vault git status
rocky sync --status
```

See [Vault Sync & Backup](sync.md) for full setup guide.

---

## `rocky restore`

Rebuild `graph.db` from `vault/pkg.json`. Use this when setting up Rocky on a new machine after cloning your vault.

```bash
git clone https://github.com/you/rocky-vault.git ~/.rocky
rocky restore
# ✓ Restored 42 topics from ~/.rocky/vault/pkg.json
```

---

## `rocky delete "query"`

Search your PKG and remove topics that are no longer relevant.

```bash
rocky delete "jwt"
rocky delete "react hooks"
```

Rocky shows all matching topics and asks you to confirm before deleting. The corresponding vault file is also removed.

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

## `rocky hook`

Called automatically by the Claude Code hook — logs the prompt to `./.rocky`. You don't run this manually.
