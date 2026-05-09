---
title: "What is Rocky?"
order: 1
---
## Why it exists

Every senior engineer knows the feeling: you stop writing something by hand, and six months later you can't remember how it works without looking it up. That's normal. That's how memory works.

What's new is the speed. AI tools don't just accelerate your output — they remove the need to think through problems entirely. You describe what you want, the AI writes it, you ship it. Fast. But the understanding that used to come from doing the work yourself? That doesn't happen anymore.

This is **AI skill atrophy** — and it compounds silently. You don't notice it until the day the AI gives you the wrong answer and you can't tell.

Rocky exists for one reason: **to shine a light on blind spots in systems developers are responsible for — systems co-created with AI agents.**

---

## What makes it different

Most learning tools quiz you on definitions. Rocky doesn't care if you can define JWT. It cares whether you'd catch the bug.

Instead of asking "what is a refresh token?", Rocky asks:

> *You're issuing JWTs with a 15-minute expiry. A user is halfway through a checkout flow when their token expires. What happens — and how do you make the experience seamless without storing session state on the server, question?*

That's the kind of question that separates someone who read the docs from someone who's actually built with it. Rocky asks the second kind — every time.

---

## How it works

Rocky's primary mode is alongside your AI agent. When Claude Code is doing the work, Rocky watches what got built, extracts the topics that live in the changes, and turns each one into a question you'll need to be able to answer for the system you now own.

### With your AI agent (Claude Code)

This is the canonical Rocky workflow. After `rocky install claude`, the agent picks up five skills it can invoke at the right moments — no separate Ollama call, no per-prompt logging in the critical path:

- **`/rocky-review` — the default** — umbrella skill that runs `/rocky-checkpoint` and `/rocky-promptiq` back-to-back at the end of a session, then prints a single fused summary. Use this unless you need one of the underlying skills on its own.
- `/rocky-checkpoint` — extract topics from recent commits + the conversation. Called by `/rocky-review`, but invoke directly from CI / automation so a failure stays isolated to one queue.
- `/rocky-quiz` — runs a Socratic review session inside the agent session, using the canonical questions Rocky stored.
- `/rocky-backfill` — seeds the PKG from a project's existing git history when you're new to a repo or first installing Rocky.
- `/rocky-promptiq` — re-evaluates your recent prompts and produces a PromptIQ score with feedback. Also called by `/rocky-review`; suitable for a slower CI cadence (weekly, etc.).

```bash
rocky install claude     # one-time: drops the skills into ~/.claude/skills/
# ... work normally with the agent ...
# inside Claude: /rocky-review  (at end of session — runs checkpoint + promptiq)
#                /rocky-quiz    (any time — drill weakest topics)
```

The alternative workflows below exist for the moments you're not in an agent session — useful supplements, not the main story.

:::details Alternative workflows
### Before a task

Tell Rocky what you're about to build. Rocky extracts the key topics, checks what you already know, and asks a focused question on anything new or fading — before you've touched a single line of code.

```bash
rocky "add rate limiting to the API using Redis"
```

Rocky finds: *Redis sorted sets, token bucket algorithm, atomic Lua scripting.*
You know the first two. It asks you about Lua scripting in Redis — specifically, why you'd need it and what breaks without it.

### After a commit

Every `git commit` triggers `rocky diff`. Rocky reads your actual code changes — not the commit message — and surfaces the topics that live in what you just shipped.

```bash
git commit -m "add Redis rate limiter"
# Rocky runs automatically:
# ~ Redis Lua scripting  (recall fading to 71%)
#   Reminder: Lua scripts in Redis run atomically — the whole script or nothing.
#   This is why you use them for rate limiting: checking and incrementing the
#   counter must be a single operation, or two requests can both pass the check
#   before either increments.
```

### On demand

Run a review session any time. Rocky works through what's most overdue, what you've recently skipped, and new topics from your recent AI sessions.

```bash
rocky quiz               # general review
rocky quiz "redis"       # targeted — search and pick topics to drill
```

### Pre-commit review

Check your staged changes before you commit. Useful when you've been working with an AI and want to make sure you actually understand what's about to land.

```bash
rocky diff --staged
```

:::

---

## The knowledge model

Rocky tracks a **Personal Knowledge Graph (PKG)** — a local database of every topic you've encountered. Each topic has a **recall** score: a number between 0 and 1 that combines two things:

- **Retrievability (R)** — freshness from spaced-repetition decay. High right after a review, lower as time passes.
- **Mastery (M)** — the mean of your last three review scores (default 0.5 if you've never been quizzed). Captures *how well you actually answered*, not just how recently.

`recall = R × M`. Freshness alone doesn't count as knowing — if you got the wrong idea last time, retrieving it quickly today doesn't help you. Both factors have to be high before Rocky considers a topic *known*.

| Bucket | recall | What Rocky does |
|---|---|---|
| Known | ≥ 0.6 | Stays quiet |
| Fading | 0.3 – 0.6 | Surfaces it for review |
| Gap | < 0.3 | Asks a question |

Knowledge decays. A topic you understood deeply three months ago might be at 25% today — Rocky surfaces it before you trip over it.

---

## Rocky the alien

```
     _____
   .'     '.
  /  .   .  \
 |  . _____ .|
 |   |     | |
 |   |_____|  |
  \   .   .  /
   '.______.'
```

Rocky has a personality based on Rocky the alien from Andy Weir's [*Project Hail Mary*](https://www.imdb.com/title/tt12042730/) — enthusiastic, direct, and genuinely rooting for you.

- `Fist my bump, friend! Is correct!`
- `Excite excite excite! Friend get it!`
- `Is okay! Rocky also not know at first!`
- `We are crew. We solve together.`

Questions end with ", question?" — Rocky's way of asking. Set `personality = false` in `[ui]` config for plain output.

---

## Key concepts

| Term | What it means |
|---|---|
| **PKG** | Personal Knowledge Graph — your local database of topics |
| **Known** | Recall ≥ 0.6 — Rocky stays quiet |
| **Fading** | Recall 0.3 – 0.6 — Rocky surfaces it as a review candidate (called "stale" internally) |
| **Gap** | Recall < 0.3 — Rocky asks a question |
| **Retrievability (R)** | Spaced-repetition freshness — decays with time since the last review |
| **Mastery (M)** | Mean of your last three review scores (default 0.5 if never quizzed) |
| **Recall** | `R × M` — the combined score Rocky classifies on |
| **Stability** | How deeply embedded the topic is — higher stability means slower decay |
| **Initial stability** | Set by topic kind when first created: Concept = 4.0, Pattern = 2.5, Implementation = 1.5 |
| **Domain** | One of 13 taxonomy categories (Language, Auth, Database, DevOps, etc.) |
| **Edge** | A relationship between two topics in the PKG — generated automatically by Rocky after new topics are added |
| **Edge kind** | The type of relationship: `implies`, `depends_on`, `conflicts_with`, or `part_of` |
| **Cross-concept question** | A question that bridges two related topics — asked when Rocky detects a relevant edge and both topics have strong recall |
| **Repos** | The git projects a topic has been encountered in. Topics merge by canonical name across projects, so `repos[]` accumulates as the same idea reappears in different repos |
| **Question bank** | Up to ~4 implication-grounded questions stored per topic at extraction time. Surfaced in the web UI, picked from at quiz time |
| **Canonical clue** | A short hint stored alongside the question bank — shown when you type `c` during a quiz |
