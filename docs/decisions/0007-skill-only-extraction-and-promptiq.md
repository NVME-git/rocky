# 0007 — Skill-only extraction + PromptIQ as a co-equal KPI

Date: 2026-04-27 · Status: Accepted

## Context

Two threads converge here.

**Thread 1 — extraction model.** Rocky shipped two ways to turn raw activity
into PKG nodes:

1. **Stop hook → `rocky session-end` → Ollama.** Fired after every Claude
   Code turn. Rocky parsed the session transcript, called a local LLM, did its
   own dedupe, inserted nodes. Cost: per-turn latency (1–3 s) and a hard
   dependency on Ollama for the auto path.
2. **`/rocky-checkpoint` skill.** Triggered by the user inside the Claude
   session. The agent itself does the extraction in-context — it already has
   the transcript loaded. Rocky just provides primitives (`rocky context`,
   `rocky list --json`, `rocky add-topic`, `rocky add-question`) and accepts
   the result.

Path 2 produces sharper extraction (the agent sees the full session, not just
the last turn) and removes the per-turn latency. Path 1 has no remaining
advantage for Claude Code or OpenCode users — both agents support skills.

**Thread 2 — what PKG quality says about you.** Rocky's existing **Rocky IQ**
score answers *"are you holding on to what your AI built?"* It's a knowledge-
retention metric. There's a complementary question worth measuring: *"are you
getting better at directing AI?"*

The same dev who once typed *"build me an app"* and now types *"refactor the
error path in `parse_csv` to handle the empty-row case, add a test that
covers it"* has demonstrably grown. Today nothing in Rocky reflects that. The
prompt log already exists (originally built to feed the now-deprecated quiz
prompt-context surface); it's a ready-made data source for a new KPI.

## Decision

**Drop Path 1 entirely.** Skill-only extraction.

- Remove the Stop hook target, `rocky session-end`, the transcript parser,
  and the related installer/uninstaller code.
- The `claude-all` install path no longer mentions a "legacy auto-extraction"
  fallback. It installs the skills, the prompt log, and the queue-mode git
  hook.
- Ollama remains a supported `[llm]` provider for Rocky's own LLM calls
  (`rocky explore`, `rocky backfill --fill-question-bank`, `rocky classify`),
  but is never auto-invoked per-turn anymore.

**OpenCode works the same way as Claude Code, no extra glue.** OpenCode's
skill discovery already includes `~/.claude/skills/<name>/SKILL.md`, so the
existing install location is read by both agents. We add an
`--opencode` flag on `rocky install skills` for users who want skills written
into OpenCode's preferred location (`~/.config/opencode/skills/`) too — but
the default behaviour remains writing to the Claude path, which both agents
read.

**Add PromptIQ.** A 0–100 KPI scored at prompt-log time via a fast Rust
heuristic on five dimensions:

| Dimension | What it measures | Max contribution |
|---|---|---|
| Specificity | File paths, function names, library/framework references | +30 |
| Context | Length above threshold, references to current state | +20 |
| Actionability | Imperative verb + concrete object | +20 |
| Verification | Mentions tests, edge cases, error handling, acceptance criteria | +15 |
| Anti-patterns | Vague phrasing without follow-up ("build me X", "fix it") | -20 |

The heuristic runs synchronously when a prompt is logged (~1 ms in pure local
computation, no network). For accuracy, an agent-driven pass refines it: the
new `/rocky-promptiq-rescore` skill walks recent prompts and overwrites
their scores using the agent's own reasoning. Same architectural model as
`/rocky-checkpoint` and `/rocky-quiz` — Rocky never makes its own LLM call
for this; the agent in your editor does.

**Where prompts live.** Promote the per-project `./.rocky` SQLite prompts to
a global `~/.rocky/prompts.db` so PromptIQ aggregates across all your
projects. On first read, prompts from any per-project `./.rocky` we can find
(walking known repos in the PKG) are migrated into the global table with their
original timestamps + project path preserved. The per-project log stays
operational so existing tooling that reads `./.rocky` keeps working — the
global is the canonical store, the per-project is a deprecated mirror.

**Sources.** `rocky prompt --log "<text>" --source <name>` is generic and
agent-agnostic. The Claude hook calls it internally; users wiring up
OpenCode or other tools (shell aliases, IDE plugins) call the same primitive.
PromptIQ surfaces a per-source breakdown so you can see whether your Claude
prompts and your OpenCode prompts are improving at different rates.

**Feedback modes** (configurable in `~/.config/rocky/config.toml`):

```toml
[promptiq]
enabled  = true
feedback = "silent"   # off | immediate | silent
```

- `off` — score stored, no feedback computed.
- `silent` — feedback stored alongside score; viewable later in `rocky view`
  or via `rocky prompt-iq --recent`.
- `immediate` — feedback printed to stderr from the hook so it surfaces in
  the agent's hook-output area. Best-effort: rendering depends on the agent.

**Surface.** A dashboard tile in `rocky view` next to Rocky IQ:

```
PromptIQ: 67  ·  ↑ 5 this week
```

Future phases (out of scope for this ADR): a Prompts tab with sort/filter,
a PromptIQ trend overlay on the Saga end-card, and LLM-evaluated
dimensions feeding back into the heuristic for calibration.

## Consequences

**Good**
- One mental model for extraction: type `/rocky-checkpoint`. No competing path.
- One LLM provider config bundle to reason about.
- Rocky stops shipping a transcript parser and a Stop-hook installer it no
  longer recommends.
- PromptIQ gives users a second growth signal that doesn't depend on quiz
  performance — useful for engineers who don't quiz often but want to know
  they're improving.
- Heuristic-at-log-time keeps the dashboard live without paying for an LLM
  per prompt. The agent-driven rescore is opt-in, batched, and free for the
  user (uses the agent they're already paying for).
- OpenCode + Ollama users get a fully local-first path: their agent uses
  Ollama, Rocky never calls an LLM directly, the skills handle extraction.

**Bad**
- Anyone currently relying on `rocky install stop` for auto-extraction
  breaks. They have to switch to `/rocky-checkpoint`.
- The 1 ms heuristic blocks the prompt-log hook. Confirmed acceptable
  during this ADR — it's pure local computation, dwarfed by the agent's own
  startup time.
- Heuristic scores will drift from agent-judged scores until the rescore
  skill catches up. Acceptable; the dashboard tile shows both numbers
  separately when a prompt has been agent-rescored.

**Reversed by**
- An ADR introducing per-prompt LLM scoring inside Rocky (would require a
  new provider plumbing decision).
- An ADR re-introducing automatic per-turn extraction (would need to justify
  the latency cost).
