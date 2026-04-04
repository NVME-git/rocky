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

You need an `ANTHROPIC_API_KEY` in your environment, or [Ollama](https://ollama.com) running locally. See [Configuration](https://nvme-git.github.io/rocky/configuration.html) for Ollama setup.

---

## What it does

When you use AI tools to write code, you ship faster but stop practising. Rocky tracks a **Personal Knowledge Graph (PKG)** of everything you've learned, how well you know it, and how recently you used it. Knowledge fades — Rocky surfaces the right things at the right time.

- **Before a task** — describe it, get quizzed on what's new or fading
- **After a commit** — Rocky reads your diff and finds topics in the actual code
- **During AI sessions** — Claude Code hook silently logs prompts; `rocky quiz` reviews them
- **Vault sync** — your PKG is version-controlled and syncs across machines

Rocky has a personality inspired by Rocky the alien from Andy Weir's *Project Hail Mary*. Fist my bump!

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

---

## Commands

| Command | What it does |
|---|---|
| `rocky "task description"` | Analyse a task and quiz on new/fading topics |
| `rocky diff` | Analyse your last commit's diff |
| `rocky diff HEAD~3` | Analyse a specific commit |
| `rocky diff --staged` | Analyse staged changes before committing |
| `rocky quiz` | On-demand review session |
| `rocky quiz <topic>` | Search and quiz on specific topics |
| `rocky ls` | Full topic list with recall, stability, difficulty |
| `rocky stats` | PKG summary (known / fading / gaps) |
| `rocky install` | Install git post-commit hook |
| `rocky uninstall` | Remove the hook |
| `rocky export` | Write PKG to Obsidian vault |
| `rocky classify` | Assign taxonomy domains to existing topics |
| `rocky sync` | Commit vault changes to git |
| `rocky sync --push` | Commit and push to remote |
| `rocky sync --init [url]` | Initialise vault git repo |
| `rocky restore` | Rebuild PKG from vault/pkg.json |
| `rocky config` | Show active configuration |
| `rocky delete "query"` | Remove topics from PKG |

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

## Vault sync

Rocky can version-control your PKG and sync it across machines:

```bash
# Enable in ~/.rocky/.rocky.toml
[sync]
enabled = true

# Init the git repo and set a remote
rocky sync --init https://github.com/you/rocky-vault.git

# Rocky auto-commits after each session. Push when ready:
rocky sync --push

# On a new machine:
git clone https://github.com/you/rocky-vault.git ~/.rocky
rocky restore
```

---

## Obsidian integration

```bash
rocky export
```

Writes one Markdown note per topic into your Obsidian vault, grouped by domain (`Language/`, `Database/`, `Auth/`, etc.). Rocky auto-creates dashboard pages with Dataview queries and wikilinks between related topics.

---

## Configuration

Rocky reads `~/.rocky/.rocky.toml` (global) and `./.rocky.toml` (project override).

```toml
[llm]
provider = "claude"          # or "ollama"
model    = "claude-sonnet-4-6"

[session]
daily_budget    = 3          # max auto-triggered quizzes per day
min_gap_minutes = 120        # minimum gap between auto sessions

[export]
obsidian_vault = "~/Documents/Obsidian/MyVault/rocky"

[ui]
personality = true           # Rocky's voice and ASCII art

[sync]
enabled               = false
auto_commit           = true
remind_push_sessions  = 5    # remind to push every 5 sessions
```

Full reference at [nvme-git.github.io/rocky/configuration.html](https://nvme-git.github.io/rocky/configuration.html).

---

## Claude Code hook

Log every prompt you send to Claude Code so `rocky quiz` has data to work with:

```json
{
  "hooks": {
    "UserPromptSubmit": [{ "command": "rocky hook" }]
  }
}
```

Add this to `~/.claude/settings.json`.

---

## License

MIT
