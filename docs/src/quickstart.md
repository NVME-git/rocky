# Quick Start

This walks you through your first session with Rocky in about 5 minutes.

---

## Step 1: Describe what you're about to work on

Before you start a task — before you open your editor or ask your AI assistant anything — tell Rocky what you're doing:

```bash
rocky "add user login with JWT tokens to my Express API"
```

Rocky analyses the task, checks your PKG, and quizzes you on anything new or fading:

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Task: add user login with JWT tokens to my Express API

  Analyzing topics...

  Rocky: New topic — JWT authentication
    Stateless token-based auth where the server signs a payload the
    client stores and sends back.

  Q1. You're issuing JWTs with a 15-minute expiry — when a user's token
      expires mid-session, what needs to happen on both sides for the
      experience to feel seamless, question?
  > 
```

---

## Step 2: Answer the question

Type your answer and press Enter. Rocky evaluates whether you understand the implications — not just the definition.

```
  > The client needs to store a refresh token separately. When the access
    token expires, the client sends the refresh token to get a new one
    without making the user log in again. The server validates the refresh
    token against a database so it can be revoked.

  ♫ Fist my bump, friend! Is correct!

  Good — you've covered the refresh flow and revocation. One thing worth
  double-checking: refresh tokens should be stored in httpOnly cookies,
  not localStorage, to prevent XSS from stealing them.

  ✓ JWT authentication added to PKG.
```

### Q&A options

At any question you can:

- **Type your answer** and press Enter
- **Press Enter** with nothing to skip (queues the topic for later)
- **Type `i`** to ignore the topic (useful when Rocky picks up a hallucinated or irrelevant topic)
- **Type `k`** if you already know this well (Rocky records it without a full Q&A)

---

## Step 3: Check your knowledge graph

After a few sessions, see what you know:

```bash
rocky ls
```

```
  ♫  Rocky · Personal Knowledge Graph
  ──────────────────────────────────────

  Topic                           Kind           Recall         Stab   Diff  Reviews  Last Reviewed
  ────────────────────────────────────────────────────────────────────────────────────────────────
  JWT authentication              pattern        ██████████ 97%  8.2    0.3   3        2026-04-03
  httpOnly cookie security        concept        ████████░░ 81%  5.1    0.4   2        2026-03-28
  SQL injection prevention        pattern        ██████░░░░ 63%  3.0    0.5   1        2026-03-10
  database indexing               implementation ████░░░░░░ 42%  1.8    0.6   1        2026-02-15
```

- **Recall** — how likely you are to remember this right now
- **Stab** (stability) — how deeply embedded it is; higher means slower decay
- **Diff** (difficulty) — how hard you've found this historically
- **Reviews** — how many times you've been quizzed on this

Green = solid, yellow = fading, red = needs attention.

---

## Step 4: Set up the git hook (optional but recommended)

This makes Rocky automatically run after every commit, analysing the actual code changes:

```bash
cd your-project
rocky install        # installs the git hook (default)
```

From now on, every `git commit` triggers `rocky diff` automatically.

---

## Step 5: Quiz yourself on recent AI-assisted work

If you use Claude Code with the hook set up, Rocky logs your prompts in the background. Run this to review what topics came up:

```bash
rocky quiz
```

### Quiz on a specific topic

```bash
rocky quiz "redis"
```

Rocky searches your PKG and queued topics for anything matching "redis", shows you the options, and lets you pick which ones to quiz:

```
  Matching topics for "redis":

  [1]  Redis TTL expiry           (gap    · 38% recall)
       How Redis handles key expiration and its effect on cache consistency.
  [2]  Redis pub/sub              (fading · 74% recall)
       Event-driven messaging with Redis channels.
  [3]  Redis cluster sharding     (known  · 91% recall)

  Select topics to quiz (e.g. 1,2 or all, or Enter to cancel):
  > 1,2
```

---

## What happens over time

Rocky uses a memory model similar to Anki (spaced repetition). Topics you know well decay slowly. Topics you barely know decay fast. Over time, Rocky surfaces the right things at the right moments without spamming you.

By default, Rocky runs a maximum of 3 quizzes per day via automatic triggers (git hook, Claude Code hook), with a 2-hour gap between them. Manual `rocky quiz` calls always run — no limits.
