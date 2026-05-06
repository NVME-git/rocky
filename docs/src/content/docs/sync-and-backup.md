---
title: "PKG Sync & Backup"
order: 7
---
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
