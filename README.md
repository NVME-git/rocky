# Rocky

```
   ♫           ♪          ♫

     __|__
    /◉   ◉\         R  O  C  K  Y
    \ ─── /         Personal Knowledge Graph
     \_↑_/
    /|||||\          Stay sharp. Stay human.
```

> **What's your Rocky IQ?**
>
> A live 0–100 score of how well you actually understand the code your AI is shipping. Decays when you stop engaging. Climbs when you can answer for it.

Rocky exists for one reason: **to shine a light on blind spots in systems developers are responsible for — systems co-created with AI agents.**

📚 **[Full documentation →](https://nvme-git.github.io/rocky)**

---

## Why it exists

Every senior engineer knows the feeling: you stop writing something by hand, and six months later you can't remember how it works without looking it up. That's normal. That's how memory works.

What's new is the speed. AI tools don't just accelerate your output — they remove the need to think through problems entirely. You describe what you want, the AI writes it, you ship it. Fast. But the understanding that used to come from doing the work yourself? That doesn't happen anymore.

This is **AI skill atrophy** — and it compounds silently. You won't notice it until the day the AI gets it wrong and you can't tell.

---

## Install

```bash
cargo install --git https://github.com/NVME-git/rocky
rocky install claude-all
```

That's it. Rocky's primary mode runs as **skills inside Claude Code** (or [OpenCode](https://opencode.ai)) — the agent's context window does the extraction. **No API key needed.** No Ollama. No daemon.

Don't have Rust? Get it from [rustup.rs](https://rustup.rs) (~2 min) or download a prebuilt binary from [Releases](https://github.com/NVME-git/rocky/releases).

For the standalone-CLI flow (Ollama or Anthropic API key), see [Installation → Alternative workflows](https://nvme-git.github.io/rocky) in the docs.

---

## What you actually do

Three steps. The first one is *work normally*.

```text
1. Open Claude Code in any project. Work on a real task.
   ──> Each commit silently queues its diff. Each prompt is logged.
       No interruption to your flow.

2. End of session, type:                /rocky-checkpoint
   ──> The agent reads the queue + transcript, extracts the
       concepts that came up, writes them to your PKG with a
       four-question bank each.

3. When you're ready to drill, run:     rocky view
   ──> Browser opens to your Dashboard with your live Rocky IQ,
       Knowledge Map, Review Queue (one-click "Quiz top 5"),
       Sessions, Projects, and a cinematic Saga timelapse.
```

Inline reviews without leaving the agent: type `/rocky-quiz` in Claude Code.
Adopting Rocky on an existing repo: type `/rocky-backfill` once.
Re-score how well you've been prompting: `/rocky-promptiq`.

---

## Where Rocky lives

| Surface | What it does |
|---|---|
| **Claude Code skills** | The four `/rocky-*` skills do the work — extraction, quizzing, backfill, prompt scoring |
| **`rocky view`** (web UI) | Recommended quiz interface — graph + IQ banner + voice (local `whisper.cpp`) + Saga timelapse |
| **Browser extension** | Capture YouTube + articles into your PKG — see [`extensions/browser/`](extensions/browser/) |
| **Obsidian export** | Markdown notes with Dataview dashboards in your vault |
| **CLI** | All the primitives the skills compose — useful for scripting, backups, ad-hoc analysis |

---

## Demo

A walkthrough video and a hosted example PKG are on the [Demo page](https://nvme-git.github.io/rocky) — click into the example before installing to feel the UI.

---

## How knowledge gets ranked

Rocky uses **FSRS** (the same family of spaced-repetition algorithm Anki moved to). Every topic has a `recall_now` score — `retrievability × mastery` — that combines freshness and how well you've actually been answering. Topics decay; Rocky surfaces the right ones at the right moments.

Your **Rocky IQ** is the rolling average across your PKG, scaled 0–100. It's the headline metric in the web UI and the browser extension badge.

---

## Sync & backup

Rocky version-controls your PKG with git so it's restorable on any machine.

```bash
rocky sync --init https://github.com/you/rocky-pkg.git
rocky sync --push                                       # commit + push
# on a fresh machine:
git clone https://github.com/you/rocky-pkg.git ~/.rocky
rocky restore
```

Full setup in [Sync & Backup](https://nvme-git.github.io/rocky).

---

## Voice (alpha)

Push-to-talk on the answer textarea in `rocky view`. Default backend is local `whisper.cpp` — nothing leaves the machine.

```bash
curl -fsSL https://raw.githubusercontent.com/NVME-git/rocky/main/scripts/install-whisper.sh | sh
```

Optional browser STT (Web Speech API) is opt-in and gated by `privacy.strict`. See [Voice](https://nvme-git.github.io/rocky).

---

## Open source. Team tier coming.

Rocky is free and open source for individuals — the entire flow above is and will remain MIT-licensed.

A **team tier** is on the roadmap: shared PKGs, manager dashboards, and a PR-gate that requires the author to demonstrate understanding of the topics introduced in their diff before it can merge. Designed for engineering leaders who want a measurable answer to "is the team actually keeping up with what AI is shipping?"

---

## Repo structure

```
rocky/
├── src/                          # Rust binary — the rocky CLI + web server
│   ├── main.rs                   # CLI entry, command dispatch, install/uninstall
│   ├── db.rs · node.rs           # PKG storage (SQLite) + topic model
│   ├── fsrs.rs                   # spaced-repetition scheduling
│   ├── server.rs · app.html      # `rocky view` axum server + bundled web UI
│   ├── teacher.rs · personality.rs   # LLM extraction + Rocky's voice
│   ├── promptiq.rs               # PromptIQ scoring
│   ├── sync.rs · obsidian.rs     # git PKG sync + Obsidian export
│   ├── voice.rs                  # whisper.cpp push-to-talk
│   ├── local_log.rs · session.rs # per-project queue + prompt log
│   └── config.rs                 # config + paths
│
├── skills/                       # Claude Code skills installed by `rocky install claude-all`
│   ├── rocky-checkpoint/         # extract topics from session diffs + transcript
│   ├── rocky-quiz/               # inline quiz on weakest topics
│   ├── rocky-backfill/           # one-shot seed from existing git history
│   └── rocky-promptiq/   # PromptIQ scoring of recent prompts
│
├── docs/                         # Astro static site → nvme-git.github.io/rocky/
│   ├── src/content/docs/         # markdown sources (one .md per section)
│   ├── src/components/           # HeroBlock, IqDial, Sidebar (vanilla, no framework)
│   ├── src/data/sections.ts      # single source of truth for sidebar order
│   ├── plugins/                  # remark + rehype plugins:
│   │                             #   :::details, ```youtube, terminal-chrome
│   ├── public/                   # PWA manifest, icons, favicon
│   └── decisions/                # ADRs (Nygard format)
│
├── extensions/
│   └── browser/                  # Chrome MV3 extension — capture YouTube + articles
│
├── scripts/
│   └── install-whisper.sh        # one-shot local STT install
│
├── .github/workflows/
│   └── pages.yml                 # Node 20 → npm ci → npm run build → upload docs/dist
│
├── Cargo.toml · Cargo.lock       # Rust crate
├── BACKLOG.md                    # planned + in-progress features
├── README.md                     # this file
└── .env.example                  # ANTHROPIC_API_KEY stub (only needed for non-agent flows)
```

---

## Architecture decisions

The major design choices — the rich-context pipeline, the FSRS-based recall model, the Rocky IQ score, the privacy mode, the agent-skill pivot — are recorded as ADRs in [`docs/decisions/`](docs/decisions/). Each ADR is one Nygard-style file: context, decision, consequences. Read those if you want to know *why* a thing is the way it is.

---

## License

MIT
