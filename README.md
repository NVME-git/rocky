# Rocky

A command-line tool that keeps your coding skills sharp while AI agents do the work.

**[Read the docs →](https://nvme-git.github.io/rocky)**

## What it does

When you use AI tools to write code, you get things done faster — but you stop practising yourself. Rocky quizzes you on the topics your AI is handling, so you stay genuinely sharp.

```bash
rocky "add JWT authentication to my REST API"
```

Rocky figures out what topics are involved, checks what you already know, and asks a focused question on anything new or fading — not "what is JWT?" but "you're issuing tokens with a 15-minute expiry, what happens when one expires mid-session?"

## Install

```bash
cargo install --git https://github.com/NVME-git/rocky
```

Or download a binary from the [Releases](https://github.com/NVME-git/rocky/releases) page.

## Quick start

```bash
# Before a task
rocky "migrate my database from SQLite to Postgres"

# After a commit — analyse the actual code diff
rocky diff

# See your knowledge graph
rocky --list

# Quiz yourself on recent AI sessions
rocky quiz
```

## Documentation

Full documentation at **[nvme-git.github.io/rocky](https://nvme-git.github.io/rocky)**

## License

MIT
