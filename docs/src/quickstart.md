# Quick Start

This walks you through your first session with Rocky in about 5 minutes.

## Step 1: Describe what you're about to work on

Before you start a task — before you open your editor or ask your AI assistant anything — tell Rocky what you're doing:

```bash
rocky "add user login with JWT tokens to my Express API"
```

Rocky will analyse the task and respond with something like:

```
 Rocky
 ─────────────────────────────

 Task: add user login with JWT tokens to my Express API

Analyzing topics...

Rocky: New topic — JWT authentication
  Stateless token-based auth where the server signs a payload the client stores and sends back.

Q1. You're issuing JWTs with a 15-minute expiry — when a user's token expires mid-session,
    what needs to happen on both the client and server side for the experience to feel seamless?
   (Press Enter to skip, type your answer below)
   >
```

## Step 2: Answer the question

Type your answer and press Enter. Rocky evaluates whether you actually understand the implications — not just the definition.

```
   > The client needs to store a refresh token separately. When the access token 
     expires, the client automatically sends the refresh token to get a new one 
     without making the user log in again. The server needs a separate endpoint 
     for this and needs to validate the refresh token, ideally checking it against 
     a database so it can be revoked.

   Good — you've covered the refresh flow and revocation. One thing to double-check:
   refresh tokens should be stored in httpOnly cookies, not localStorage, to prevent 
   XSS attacks from stealing them.
   Added to your PKG.
```

## Step 3: Check your knowledge graph

After a few sessions, you can see what you know:

```bash
rocky --list
```

```
 Rocky
 ─────────────────────────────

  Topic                               Kind            Recall         Last Reviewed
  ───────────────────────────────────────────────────────────────────────────────
  JWT authentication                  pattern         ██████████ 97%  2026-04-03
  httpOnly cookie security            concept         ████████░░ 81%  2026-03-28
  SQL injection prevention            pattern         ██████░░░░ 63%  2026-03-10
  database indexing                   implementation  ████░░░░░░ 42%  2026-02-15
```

The bar shows your current recall. Topics in green are solid, yellow are fading, red need attention.

## Step 4: Set up the git hook (optional but recommended)

This makes Rocky automatically run after every commit, analysing the actual code changes:

```bash
cd your-project
rocky install
```

From now on, every time you commit, Rocky looks at your diff and quizzes you on what just changed.

## Step 5: Quiz yourself on recent AI-assisted work

If you use Claude Code or another AI assistant, Rocky logs your prompts in the background. Run this to review what topics came up:

```bash
rocky quiz
```

This is useful at the end of the day — Rocky reviews everything your AI handled and makes sure you understand it.

---

## What happens over time

Rocky uses a memory model similar to Anki (spaced repetition). Topics you know well decay slowly. Topics you barely know decay fast. Over time, Rocky surfaces the right things at the right moments without spamming you.

By default, Rocky runs a maximum of 3 quizzes per day with a 2-hour gap between them. This is intentional — it keeps Rocky from feeling like a chore.
