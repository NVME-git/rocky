---
title: "Installation"
order: 2
---
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
- `/rocky-promptiq` — re-evaluates your recent prompts and produces a PromptIQ score with feedback.

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
