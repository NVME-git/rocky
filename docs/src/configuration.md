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

## Claude Code hook

To make Rocky log your AI prompts automatically, add this to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {"command": "rocky hook"}
    ]
  }
}
```

Every prompt is silently logged to `./.rocky` in your project folder (only if `rocky install` was run there). Run `rocky quiz` at any time to review topics from recent prompts.

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
