# Rocky — Feature Backlog

Ideas and planned features, organised by theme. Items marked **[planned]** have documentation written.
Items marked **[idea]** are under consideration. Items marked **[in progress]** are being built.

---

## Teaching Quality

### Good Catch metric — track when the user corrects the agent `[idea]`

A counter + log of the times the user spots an agent omission, drift, or wrong
assumption that the agent didn't surface itself. Each entry records: when, what
the agent reported, what the user caught, the resulting correction, and an
optional repo link. Surfaced in `rocky view` next to Rocky IQ and PromptIQ as
a third axis — *audit vigilance*: how good is the user at keeping their
AI-driven work honest?

Why it matters: PKG quality and prompting skill (Rocky IQ + PromptIQ) measure
*output*. Good Catches measure *review discipline* — the trait that prevents
silent over-trust of agent output. A user with high Rocky IQ + high PromptIQ
but zero Good Catches has either a perfect agent or a missing audit habit.

Concrete worked example (the catch that motivated this entry):

- **2026-05-07** · repo: [home_bank](/home/nabs/Work/home_bank)
- **What the agent reported:** "home_bank backfill done — 85 topics, 0 cross-project merges. The home_bank vocabulary is genuinely different from Gammon/MountyPathon/NVME."
- **What the user caught:** *"For home bank, did you check other branches as well? There is a recent branch with a novel approach that could be learned from."*
- **The actual gap:** `rocky checkpoint history` shells out to `git log` with no `--all` — only walks the current HEAD's commits. Other branches with unmerged work (5 unique commits on `copilot/update-parsers-for-amount-storage`) were invisible to the subagent. The "novel approach" — integer-arithmetic for currency precision — happened to be already captured because its merged version landed on the active redesign branch, but that was luck, not design.
- **Correction:** Investigation surfaced two real backlog items: (1) `rocky checkpoint history` should support `--all-branches` or `--branch <name>`; (2) the agent should self-audit "did I see every branch worth seeing?" before reporting done. The user's catch shifted both from invisible to fixable.

Implementation hints:

- New `rocky catch` primitive: `rocky catch --summary "..." --correction "..." --repo <name>` writes a row to a `good_catches` table
- Lightweight: no LLM call, no UI — the user logs catches manually after they happen
- A skill `/rocky-catch` could prompt for the four fields when invoked
- Surface in `rocky stats` and the web view's dashboard tile
- Pair with the conversation-pass topic extraction — many catches naturally come up in dialogue and could be auto-extracted as both a topic AND a catch entry

Pairs well with the existing AI-source tagging idea below — a topic that was caught (vs handed to the user by the agent unprompted) gets a higher independence score.

### Struggle score affects stability `[planned]`

Track how many follow-ups, clues, and scaffolding requests were made before a topic was marked
"understood". A topic answered straight through should earn higher stability than one that required
three follow-up questions and a clue. This makes *quality* of understanding matter, not just outcome.

### AI-source tagging `[planned]`

When a topic first enters the PKG via the Claude Code hook (i.e. seen in a prompt, not in the
author's own code), tag it `source: ai_prompt`. Quiz those topics more aggressively and flag them
in `rocky ls` as "AI-introduced" until the author has independently demonstrated understanding.

Rationale: confidence without competence. A developer can have a full PKG of "known" topics they
have only ever seen AI handle. Source tagging makes this visible.

### Independence score `[planned]`

A second axis alongside retrievability: how often has this topic appeared in the author's own diffs
(own_code) vs. only in AI-assisted prompts (ai_prompt)? Topics with high retrievability but low
independence are risk indicators. Surface these in `rocky stats` as a distinct category.

### Hard mode (no scaffolding) `[planned]`

`rocky quiz --hard` disables clues (`[c]`), explanations (`[?]`), and simplification (`[s]`).
Forces the author to either answer from genuine understanding or skip. Configurable as a permanent
setting in `.rocky.toml` for authors who want to build resilience deliberately.

### Debugging-focused question kind `[planned]`

A new question type generated at backfill/diff time: given this code change, what are the failure
modes? What would a bug look like in production and what would the stack trace tell you? Distinct
from the current Socratic implication question — more forensic, targeting the durable skill of
debugging that AI cannot fully replace.

### Pre-task gap framing `[planned]`

When running `rocky "task description"`, explicitly surface knowledge gaps as AI-risk warnings:
"You have a gap on Lua scripting in Redis — this is a topic AI will likely handle for you. Here
is the question you should be able to answer before trusting what it produces." Frames Rocky as a
check on AI output, not just a learning tool.

---

## Workflow Integration

### Drop the `--all-authors` filter (default is broken in the AI era) `[idea]`

`rocky checkpoint history` and `/rocky-backfill` default to current-git-user-only.
The original intent was filtering out OSS-contributor noise, but in an AI-driven
workflow the AI agents (Copilot, Claude, Sonnet, etc.) appear as separate authors
and their commits ARE the user's work — they orchestrated and reviewed it. The
filter today removes exactly the commits worth capturing.

Concrete repro: backfilling Gammon (2026-05-07) returned 20 of 31 commits because
11 were AI-authored; those 11 contained the meatier feature work.

Two fixes:

1. Flip the default to all-authors and rename the opposite flag `--mine-only`
   for the rare cases someone wants the old behaviour.
2. Drop the filter entirely; users who care can pipe their own `git log` filter
   into the backfill.

Lean toward (2). The filter assumes a "my code vs. contributor noise" model
that doesn't match how solo devs orchestrating AI actually own work in 2026.

### Subagent walks repo history with HEAD context `[idea]`

A backfill mode where a fresh subagent is spawned per repo with full read access
to the codebase at HEAD, *before* it walks commit history. Reads CLAUDE.md /
README.md / key source files first to build a mental model of what the
codebase IS today, then extracts topics grounded in that resolved understanding
— not just diff hunks in isolation.

Why: topic names today come from diff text, which approximates the codebase's
vocabulary. A subagent that has read the actual current source can name topics
in the language the codebase uses, and can tell whether a deltas's final shape
still survives at HEAD or was reverted/superseded later. Diff-only extraction
can't.

Implementation paths:

1. New `rocky backfill --subagent` subcommand that shells out to `claude` /
   `opencode` with a self-contained brief. Cleanest UX; depends on agent CLI.
2. New `/rocky-backfill-isolated` skill explicitly written for an isolated
   subagent context (no transcript, no inherited assumptions).
3. Manual: user spawns the subagent themselves via the Agent tool in their
   IDE/CLI session, pasting in a brief.

Pairs with the resumable-backfill design in NOTES/backfill-v2.md — a subagent
brief should plug into the same `next-batch` / `mark` plumbing once that ships,
so subagents can checkpoint each commit and be killed/restarted cleanly.

### PromptIQ tile + recent-prompts UI improvements `[idea]`

The PromptIQ banner in `rocky view` ([src/app.html:519-527](src/app.html))
currently shows the score, delta, sparkline, and a meta line. It auto-refreshes
when the page reloads. Six gaps that the `/rocky-promptiq` rescore should — but
doesn't yet — surface in the UI:

1. **Per-prompt rescore badge.** The recent-prompts panel doesn't distinguish
   heuristic-only scores from LLM-judged ones. Add a small `[LLM]` badge or
   colour shift on rescored prompts so the user can tell *which scores can be
   trusted* vs *which are conservative guesses*. Highest-value-lowest-effort
   of the six.
2. **"Last rescored" timestamp on the tile.** A subtle `· last rescored 12m ago`
   under the meta text so the user knows how stale the LLM scores are. Pairs
   with #1 — together they tell the user how much to trust the headline.
3. **One-click rescore button on the tile.** Pairs with the
   *Launch agent quiz/teach session from rocky view* backlog item.
   Clipboard-copy + toast pattern: button copies `claude "/rocky-promptiq"`
   and tells the user to paste in their terminal. Closes the loop without
   forcing them to remember the slash command.
4. **Feedback excerpt per prompt in the recent-prompts panel.** After
   rescoring, each prompt has a one-line feedback note in the DB; surface it
   inline so the user sees *why* the score is what it is, not just the number.
5. **Per-source split when applicable.** The backend already supports per-source
   breakdown (the `rocky prompt-iq` CLI shows it). If the user has multiple
   sources logged, the tile should show `claude: 42 · opencode: 35` instead of
   a single aggregate that hides divergence.
6. **Heuristic-vs-LLM divergence indicator.** When the LLM rescore differs
   significantly from the heuristic (|Δ| > ~15), highlight that prompt — these
   are the cases where the heuristic was most wrong and the user benefits most
   from the audit. (Today's PKG had a 26-point heuristic→LLM lift on the
   aggregate; the per-prompt divergence is informative on its own.)

Recommend shipping #1 + #2 first as a pair, then #3 alongside whatever ships
for the *Launch agent terminal from rocky view* backlog item. #4–#6 can come
later as the prompts panel grows beyond the current "recent 5" surface.

### Rocky as a required PR check `[planned]`

A GitHub Actions workflow that runs Rocky as a required status check on pull requests. Before a
PR can be merged, Rocky analyses the diff, identifies topics introduced, and checks whether the
author has those topics in their PKG at a minimum retrievability threshold.

Supports GitHub's stacked PRs feature: each layer of a stack must demonstrate understanding of
the topics introduced in that layer before it can merge into the next.

See: [Roadmap section in docs](docs/lib/content.dart) for full design.

### Launch agent quiz/teach session from rocky view `[idea]`

Button in the rocky view web UI that starts a Claude Code or OpenCode session
with `/rocky-quiz` or `/rocky-teach` pre-loaded. Bridges the in-page quiz
(quick, no agent) with the agent-driven modes (richer adaptive grading,
teacher persona role-play).

Phased implementation:

1. **Click-to-copy + toast** — button copies `cd <project> && claude "/rocky-quiz"`
   to clipboard, shows "Pasted — run it in your terminal." Bulletproof, works
   on any OS, ships in an afternoon. Good default.
2. **Server-spawned terminal (opt-in)** — new `POST /api/launch?mode=quiz|teach`
   endpoint. Server detects terminal via `$TERMINAL` then a fallback chain
   (kitty → ghostty → alacritty → foot → gnome-terminal), spawns it with
   the agent CLI and starter prompt. Behind a config flag (e.g.
   `[view] launch_terminal = true`) so SSH-tunnel users aren't surprised
   by terminals opening on the remote machine.

Open questions to settle before building: (a) verify `claude "<prompt>"` and
the OpenCode equivalent both accept a positional starter prompt cleanly;
(b) UX disambiguation from the existing in-page quiz at `/api/quiz/*` —
labelling needs to make clear when to pick which.

Out of scope for this entry but worth noting: a custom URL scheme
(`rocky://quiz?project=...`) would be a third path, but adds an install
step and a browser security prompt — defer unless the click-to-copy or
terminal-spawn paths prove insufficient.

### Embedded LLM chat panel inside `rocky view` `[idea]`

Inline chat panel right in the topic detail card — clicking Teach Me opens
a conversation surface inside Rocky instead of copying a prompt and sending
the user to ChatGPT/Claude/etc. Backed by the user's Anthropic / OpenAI
key or a local Ollama, configured via `~/.config/rocky/config.toml`.

Why this matters: it eliminates the round-trip-back-into-Rocky problem
that originally motivated the browser-extension scraper. The conversation
is already structured data the moment the user closes the panel, ready
to feed straight into `Teacher::enrich_topic_from_conversation` — no
DOM scraping, no extension install, no fragile per-vendor selectors.

Trade-offs to weigh before building: loses the user's preferred LLM brand
/ chat history continuity, adds an API-key configuration step (or Ollama
dependency). The current paste-back flow stays as the universal fallback
for anyone who'd rather chat in their LLM tab and copy-paste the result
back into Rocky.

### Rocky as a retroactive clue filler `[in progress]`

`rocky backfill --fill-clues` — generates missing clues for nodes that already have canonical Q&A.
Useful after upgrading Rocky versions that add new node fields. Foundation for a future scheduler
that keeps the PKG enriched as new features are added.

---

## Graph & Memory

### Canonical clue generation `[in progress]`

Short hint stored alongside canonical Q&A, generated at backfill time from the diff. Shown when
the user types `[c]` during a quiz. For manually-added topics, generated on-demand at runtime.

### Scheduled graph enrichment `[idea]`

A background scheduler that runs enrichment passes over the PKG when new features are added —
generating missing fields (clues, cross-concept edges, debugging questions) without requiring
manual intervention. Goal: quiz time is always fast because all pre-computation happened offline.

### Subagent backfill orchestration — cwd drift safeguards `[idea]`

Subagent-driven backfill runs (the pattern we used for Gammon and MountyPathon
on 2026-05-07) are vulnerable to cwd drift: most agent harnesses reset the
shell's working directory between tool calls, so a subagent that does
`cd /path/to/repo` once and then issues 30+ `rocky add-topic` calls will see
each subsequent call run from the harness's default cwd, not the project.

Concrete impact (already observed): the MountyPathon subagent correctly
chained `cd && rocky` early in its run, then drifted; result was 13 topics
all tagged `repo=rocky` instead of `MountyPathon`. The Gammon subagent (same
brief) didn't drift — bug is intermittent and behavioural, not deterministic.

This will affect every future automated backfill until either:

1. **Rocky side**: `rocky add-topic --repo <name>` flag (covered by the
   sibling backlog entry below) so the caller is explicit and cwd doesn't
   matter.
2. **Orchestration side**: a verified-after-write loop where the subagent
   reads each topic back via `rocky topic` and confirms `repo == expected`
   before moving on. Cleaner-but-slower; catches drift early instead of
   needing a post-hoc SQL patch.
3. **Brief template**: every backfill brief should include a "cwd trap"
   warning that explicitly says *every* `rocky` invocation must be chained
   with `cd /path/to/repo && rocky ...` (not just the first one), and ideally
   a sanity check that runs `rocky list --json | jq '.[-1].repo'` after each
   batch.

Pair with the resumable-backfill design in NOTES/backfill-v2.md — the
`backfill_items` table should record the project_path and the subagent
should derive `--repo` from that explicitly, eliminating the cwd-derivation
path entirely.

### `rocky add-topic` mis-tags `repo` when cwd doesn't match the source `[idea]`

`rocky add-topic` infers the `repo` field from `basename(cwd)` at call time. In
the agent-subagent backfill flow (and any other flow where the rocky CLI is
invoked from a different working directory than the project being extracted),
this tags every topic with the wrong project name. The MountyPathon backfill
on 2026-05-07 produced 13 topics correctly source-commit-linked but all tagged
`repo=rocky` because the agent's bash tool resets cwd between calls and the
agent didn't `cd /path/to/project &&` before every single invocation.

Workaround the subagent applied: `source_commits` is preserved correctly, so
tracing the real repo is possible — but the `repo` field is what `rocky list`
groups by, so listings are wrong.

Two fixes:

1. Add a `--repo <name>` (or `--repo-root <path>`) flag to `rocky add-topic`
   that overrides the cwd-inference. Callers in cross-cwd contexts pass it
   explicitly; existing callers keep the inferred default. Cleanest.
2. Have rocky derive the repo from the commit SHA itself (run `git -C <cwd>
   rev-parse --show-toplevel` on the SHA's containing directory). Fragile —
   doesn't work if rocky has no idea where the SHA lives.

Lean toward (1). Pair with the resumable-backfill design in NOTES/backfill-v2.md
— the new `rocky backfill plan` command should record the project path per item
and the agent should pass it to add-topic via the new flag.

### Make `rocky add-topic` merge-update semantics explicit `[idea]`

On a merge (the topic name already exists), `rocky add-topic` silently preserves
the existing `description` and `domain` even when new values are passed via
`--description` / `--domain`. Only the `--context` is appended and the
question-bank insert proceeds normally. Surfaced when re-running
`/rocky-checkpoint` against an evolved topic during a conversation pass —
the new framing was lost and the old (sometimes incorrect) framing stayed.

The conservative default is fine when add-topic is being called from a
post-commit hook on autopilot (don't let a stale auto-extracted line
clobber a curated description). It's a footgun when an agent is deliberately
re-framing a topic with new context. Two ways out:

1. **Update on explicit flag pass** — if `--description` or `--domain` was
   supplied on the merge call, replace; otherwise keep. The CLI already
   knows whether the flag was present vs default.
2. **Separate `rocky update-topic` primitive** — clean separation of "create
   or merge-by-context" from "in-place edit." Skill could call update-topic
   when a merge target's framing has clearly drifted.

Lean toward #1 — single primitive, behaves the way a caller intuitively
expects ("if I passed it, I meant it"). #2 is cleaner architecturally but
adds a new command surface for a behavior that's almost always paired with
add-topic anyway.

### Conversation-source tagging in topic listings `[planned]`

Visually distinguish chat-sourced topics from commit-sourced topics in `rocky list`,
`rocky topic`, `rocky inspect`, and the dashboard. With the conversation-reflection
pass added to `/rocky-checkpoint`, topics can now enter the PKG without a backing
commit — the schema already supports this (a topic with zero non-empty SHAs across
its `topic_encounters` is chat-only), but display paths render them identically to
commit-grounded topics.

Why it matters: when reviewing the PKG, the user needs to know which topics have
real code grounding vs. which are vibes-from-a-conversation. Without the tag, the
two classes are indistinguishable and chat topics inherit unearned authority.
Pairs with the existing AI-source tagging idea above — both are about making the
provenance of a topic legible at a glance.

Cheapest implementation: derive the tag at display time from `topic_encounters`
(no schema migration), surface as a `[chat]` badge in CLI output and a field in
`node_to_json` so the web view picks it up too.

### Embedding-based semantic dedup `[planned]`

Replace today's lexical dedup (slugified node id + `topic_jaccard` token-set similarity in
`rocky dedupe`) with vector similarity. Embed each topic's name + description with a small local
model (e.g. Ollama `nomic-embed-text`, ~270 MB), store the vector alongside the node, and on add
look up nearest neighbours; merge if cosine > ~0.85. The LLM is reserved as a tiebreaker for
ambiguous mid-band cases instead of being the primary dedup mechanism.

**Why move past Jaccard:** the current heuristic is order-insensitive and catches "Bank Question"
↔ "Question Bank" cleanly, but misses true synonyms with no shared root — `Auth` vs
`Authentication`, `DB` vs `Database`, `Pooling` vs `Pool`. The substring fallback in
`is_candidate_pair` papers over a few of those, but anything where the wording diverges entirely
(`Connection Pool Sizing` ↔ `How many DB connections is too many?`) still slips through. Cosine
on sentence embeddings collapses both surface-form and semantic variants, deterministically.

**Tradeoff:** adds an embedding model dependency (~100-300 MB depending on choice) and a `vector`
column on `nodes`. Worth it once the PKG grows past a few thousand topics, where false-negative
duplicates start fragmenting review effort. See ADR 0002 for the dedup history.

---

## Knowledge Map

### Deep-zoom planet detail — clouds, swirls, surface texture `[idea]`

At very deep zoom (k > ~1.4) topic nodes still render as flat coloured circles.
Add visual definition so they read as actual planets rather than dots: keep
the same recall-based hue (red → amber → green) but layer in a soft radial
gradient + a low-frequency noise/cloud mask + a faint atmospheric ring. The
star (domain) nodes can get a subtle corona at the same zoom band.

Cheapest implementation path: pre-bake a small set of planet textures
(say 4–6 hue variants × 3 noise patterns) once into PIXI textures via
`PIXI.RenderTexture` at boot, then replace the per-topic `Graphics` circle
with a `PIXI.Sprite` referencing the right texture when `mapCameraK > 1.4`,
fall back to the existing circle below. Tinting handles the recall-colour
gradient so we don't need a texture per topic. Cost is one-time at boot;
per-frame is unchanged.

Why it matters: the space metaphor only fully clicks at deep zoom — that's
where the user expects the planet to feel like a body, not a UI dot. This
was promised by the visual language and not yet delivered.

---

## References

Features in this backlog were informed by the following sources:

- **Dr. Mark Mahoney interview — FreeCodeCamp Podcast** (2026-04-11)
  Competence vs. confidence gap, debugging as durable skill, hard-way resilience, AI-assisted
  tutorial hell. See [References section in docs](docs/lib/content.dart).
  Video: <https://youtu.be/Tb6oaEkxtp8?si=W_vu3K-0WknmvOTI>
