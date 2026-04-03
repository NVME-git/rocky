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
model = "qwen2.5-coder:7b"  # any model available to your provider
ollama_base_url = "http://localhost:11434"  # only needed for ollama

[session]
daily_budget = 3      # max quizzes per day (default: 3)
min_gap_minutes = 120 # minimum minutes between quizzes (default: 120)

[export]
obsidian_vault = "~/Documents/Obsidian/MyVault/rocky"
```

---

## LLM settings

### Using Claude (Anthropic)

```toml
[llm]
provider = "claude"
model = "claude-sonnet-4-6"
```

Requires `ANTHROPIC_API_KEY` set in your environment or a `.env` file in your working directory:

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

**Recommended models by hardware:**

| Your GPU VRAM | Recommended model |
|---|---|
| 6 GB | `qwen2.5-coder:7b` |
| 4 GB | `qwen2.5:3b` or `phi4-mini` |
| No GPU / CPU only | `qwen2.5:3b` (slow) |

---

## Session settings

| Setting | Default | What it does |
|---|---|---|
| `daily_budget` | 3 | Max quizzes Rocky will run in one day |
| `min_gap_minutes` | 120 | Rocky won't quiz again within this many minutes |

These exist so Rocky doesn't interrupt you constantly. You can increase them if you want more practice.

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

Once set up, every prompt you send to Claude Code is silently logged. Run `rocky quiz` at any time to review what topics came up.

---

## Where Rocky stores data

Everything lives in `~/.rocky/`:

| File | What it is |
|---|---|
| `~/.rocky/graph.db` | Your PKG — all topics, recall scores, review history |
| `~/.rocky/.rocky.toml` | Your global config |
| `~/.rocky/vault/` | Obsidian markdown files (if you use Obsidian) |
