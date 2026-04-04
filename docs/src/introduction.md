# What is Rocky?

Rocky is a command-line tool that keeps your technical knowledge sharp while AI agents handle more and more of your work.

---

## Why it exists

Every senior engineer knows the feeling: you stop writing something by hand, and six months later you can't remember how it works without looking it up. That's normal. That's how memory works.

What's new is the speed. AI tools don't just accelerate your output — they remove the need to think through problems entirely. You describe what you want, the AI writes it, you ship it. Fast. But the understanding that used to come from doing the work yourself? That doesn't happen anymore.

This is **AI skill atrophy** — and it compounds silently. You don't notice it until the day the AI gives you the wrong answer and you can't tell.

Rocky exists for one reason: **so you always know what your AI just built.**

---

## What makes it different

Most learning tools quiz you on definitions. Rocky doesn't care if you can define JWT. It cares whether you'd catch the bug.

Instead of asking "what is a refresh token?", Rocky asks:

> *You're issuing JWTs with a 15-minute expiry. A user is halfway through a checkout flow when their token expires. What happens — and how do you make the experience seamless without storing session state on the server, question?*

That's the kind of question that separates someone who read the docs from someone who's actually built with it. Rocky asks the second kind — every time.

---

## How it works

Rocky integrates into the moments where understanding matters most. There are five modes, each with its own rules about when it runs and what limits apply — see [How Rocky Works](how-it-works.md) for the full detail.

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

### During AI sessions

If you use Claude Code, Rocky silently logs every prompt you send. No interruption. Run `rocky quiz` later and Rocky knows exactly what topics your AI handled for you today.

```bash
rocky install claude     # one-time setup
# ... work normally ...
rocky quiz               # end-of-day review
```

### Pre-commit review

Check your staged changes before you commit. Useful when you've been working with an AI and want to make sure you actually understand what's about to land.

```bash
rocky diff --staged
```

---

## The knowledge model

Rocky tracks a **Personal Knowledge Graph (PKG)** — a local database of every topic you've encountered. Each topic has a retrievability score: a number between 0 and 1 that estimates how likely you are to recall it right now.

Knowledge decays. A topic you understood deeply three months ago might be at 65% today. Rocky knows this, and it surfaces things before they fade below the threshold — not after.

When you answer well, stability increases and the topic decays slower. When you struggle, Rocky comes back sooner. Over time, the PKG reflects your actual knowledge — not the version of yourself that existed when you first learned something.

---

## Rocky the alien

```
      ♫
   __|__
  /◉   ◉\
  \ ─── /
   \_↑_/
  /|||||\
```

Rocky has a personality based on Rocky the alien from Andy Weir's *Project Hail Mary* — enthusiastic, direct, and genuinely rooting for you.

- `♫ Fist my bump, friend! Is correct!`
- `♫ Excite excite excite! Friend get it!`
- `♫ Is okay! Rocky also not know at first!`
- `♫ We are crew. We solve together.`

Questions end with ", question?" — Rocky's way of asking. Set `personality = false` in `[ui]` config for plain output.

---

## Key concepts

| Term | What it means |
|---|---|
| **PKG** | Personal Knowledge Graph — your local database of topics |
| **Known** | Recall is strong (90%+) — Rocky stays quiet |
| **Fading** | Recall is slipping (70–90%) — Rocky gives a reminder |
| **Gap** | Recall is low or topic is new — Rocky asks a question |
| **Retrievability** | Rocky's estimate of how likely you are to remember something right now |
| **Stability** | How deeply embedded the topic is — higher stability means slower decay |
| **Domain** | One of 13 taxonomy categories (Language, Auth, Database, DevOps, etc.) |
