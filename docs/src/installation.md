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

See the [Configuration](configuration.md) page for model recommendations by GPU VRAM.

---

## Verify the installation

```bash
rocky stats
```

You should see:

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Total topics:  0
  Known:         0
  Fading:        0
  Gaps/weak:     0

  Quiz budget: 3/3 remaining today  ·  provider: claude (claude-sonnet-4-6)

  ♫ PKG is empty. Let us begin science, question?
```

---

## Set up the git hook

In your project directory, run:

```bash
rocky install
```

You'll see:

```
   ♫           ♪          ♫

     __|__
    /◉   ◉\         R  O  C  K  Y
    \ ─── /         Personal Knowledge Graph
     \_↑_/
    /|||||\          Stay sharp. Stay human.


  ✓ git post-commit hook installed — .rocky added to .gitignore

  Rocky will run after every commit in this repo.
  Use  rocky quiz  for an on-demand session anytime.
```

From now on, every `git commit` will trigger `rocky diff` automatically.

---

## Set up the Claude Code hook (optional)

If you use Claude Code, Rocky can silently log your prompts so `rocky quiz` has data to review. Add this to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      { "command": "rocky hook" }
    ]
  }
}
```

This is silent — it won't interrupt your workflow.

---

You're ready. Head to [Quick Start](quickstart.md) for your first session.
