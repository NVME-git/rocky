# Commands

## Before a task

Describe what you're about to work on. Rocky extracts the key topics and quizzes you on anything new or fading.

```bash
rocky "your task description here"
```

**Examples:**
```bash
rocky "add pagination to my REST API"
rocky "set up CI/CD with GitHub Actions"
rocky "migrate the database from SQLite to Postgres"
```

---

## After a commit

Review the topics in your most recent commit message:

```bash
rocky --after "your commit message or task summary"
```

This is useful if you worked with an AI and want to review what just got built.

---

## Analyse a git diff

This is more powerful than `--after` — Rocky reads the actual code changes, not just the commit message, and extracts topics from what was literally written:

```bash
# Analyse your last commit's changes
rocky diff

# Analyse a specific commit
rocky diff HEAD~3

# Analyse staged changes before you commit
rocky diff --staged
```

---

## View your knowledge graph

```bash
# Summary of what you know
rocky --stats

# Full list of all topics with recall bars
rocky --list
```

---

## Quiz from recent AI sessions

If you have the Claude Code hook set up, this reviews the last 24 hours of AI prompts and quizzes you on what came up:

```bash
rocky quiz

# Look back further
rocky quiz --hours 48
```

---

## Delete a topic

Search your PKG and remove topics that are no longer relevant:

```bash
rocky delete "jwt"
rocky delete "react hooks"
```

Rocky will show all matching topics and ask you to confirm before deleting.

---

## Export to Obsidian

Write all your PKG topics as markdown files to your Obsidian vault:

```bash
rocky export
```

Rocky also does this automatically every time a topic is updated. See [Obsidian Integration](obsidian.md) for how to query your PKG inside Obsidian.

---

## Git hook

```bash
# Install — runs rocky diff after every commit in this repo
rocky install

# Remove the hook
rocky uninstall
```

---

## Configuration

```bash
# Show what config Rocky is currently using
rocky config
```

---

## Claude Code hook

Rocky can silently log every prompt you send to Claude Code, so `rocky quiz` has data to work with. Set this up in your Claude Code settings — see [Configuration](configuration.md).

```bash
# This is called automatically by Claude Code — you don't run it manually
rocky hook
```
