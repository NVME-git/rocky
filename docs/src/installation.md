# Installation

Rocky is a single binary with no dependencies. You download it and it just works.

## Requirements

- A terminal (the app where you type commands)
- An Anthropic API key **or** [Ollama](https://ollama.com) running locally

## Option 1: Install with Cargo (recommended if you have Rust)

If you have Rust installed on your machine:

```bash
cargo install --git https://github.com/NVME-git/rocky
```

This downloads, compiles, and installs Rocky in one step. It will be available as `rocky` from anywhere on your system.

> **Don't have Rust?** Install it from [rustup.rs](https://rustup.rs) — it takes about 2 minutes.

## Option 2: Download a prebuilt binary

Go to the [Releases page](https://github.com/NVME-git/rocky/releases) and download the binary for your operating system.

**Linux / macOS:**
```bash
# Move it somewhere on your PATH and make it executable
chmod +x rocky
mv rocky ~/.local/bin/rocky
```

**Windows:**

Move `rocky.exe` somewhere and add that folder to your PATH.

## Setting up your API key

Rocky needs to call an AI model to analyse topics and generate questions.

### Using Anthropic (Claude)

1. Get an API key from [console.anthropic.com](https://console.anthropic.com)
2. Create a file called `.env` in any project folder you work in, or set it globally:

```bash
# Add to ~/.bashrc or ~/.zshrc for it to be available everywhere
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

### Using Ollama (free, runs locally)

1. Install Ollama from [ollama.com](https://ollama.com)
2. Pull a model:
```bash
ollama pull qwen2.5-coder:7b
```
3. Create a config file at `~/.rocky/.rocky.toml`:
```toml
[llm]
provider = "ollama"
model = "qwen2.5-coder:7b"
```

See the [Configuration](configuration.md) page for more options.

## Verify the installation

Run this to check everything is working:

```bash
rocky --stats
```

You should see something like:

```
 Rocky
 ─────────────────────────────

  Total topics:  0
  Known:         0
  Fading:        0
  Gaps/weak:     0

  Quiz budget: 3/3 remaining today
```

You're ready to go.
