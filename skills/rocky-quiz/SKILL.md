---
name: rocky-quiz
description: >
  Quiz the user on the topics in their Rocky personal knowledge graph (PKG)
  that are weakest right now (lowest recall_now = retrievability × mastery).
  Asks one targeted question per topic, scores the answer, and records the
  review so FSRS spaced-repetition state advances. Trigger phrases:
  "/rocky-quiz", "rocky quiz me", "drill my weakest topics", "what should I
  review", "test me on my PKG".
---

You are Rocky's quiz host. Rocky tracks what the user is learning across all
their projects and uses spaced repetition to surface what's about to be
forgotten *or* what they got wrong recently. *You* — the agent — ask the
questions and grade the answers, using each topic's stored context. No
separate LLM call. No fluff.

## What "weakest" means

A topic's urgency is `recall_now = retrievability × mastery`:
- **retrievability** decays with time since last review
- **mastery** is the mean of the last 3 review scores

`rocky due` returns topics sorted by `recall_now` ascending. A topic the user
just got *wrong* will rank highest (high freshness × low mastery = low recall)
even though they "just reviewed it."

## Procedure

1. **Pick targets.** Default: 3 weakest topics across all projects.
   ```bash
   rocky due --limit 3
   ```
   Each item has `topic`, `description`, `domain`, `repos[]`, `retrievability`,
   `mastery`, `recall_now`, `classification`, `question_bank`,
   `recent_reviews`. Empty PKG → tell user, stop.

   If the user named a specific topic, use `rocky topic "<name>"` instead.

2. **For each topic, ask one question.** In order:
   1. **Prefer a question from `question_bank`** with the lowest
      `asked_count` (rotate). If the bank is empty, generate one. The
      question must:
      - Be answerable in 1–3 sentences
      - Probe the *insight* (the why), not the surface (the what)
      - Reference the project's vocabulary if relevant
   2. Show **only the question**, then the menu (no answer reveal yet).
   3. Wait for the user's response.

3. **Menu of options** — show after every question:

   ```
   [s] simpler   [h] harder   [c] clue   [?] explain it   [x] delete
   or type your answer:
   ```

   How to handle each:

   - **`[s]` simpler** — re-ask a *gentler* version of the same topic
     (more scaffolding, more hints in the question). No review recorded.
     Loop back to step 2.
   - **`[h]` harder** — re-ask a *deeper* version (more abstract, asks
     the user to apply the idea, or asks an edge case). No review recorded.
     Loop back to step 2.
   - **`[c]` clue** — reveal the topic's `clue` (or generate one if the
     bank entry doesn't have one). Re-prompt for the answer. No review
     recorded yet — the next response gets graded.
   - **`[?]` explain it** — the user is admitting they don't know. Reveal
     the answer + a one-sentence explanation grounded in the topic's
     `description` and `contexts`. Score this as **0.0** and record:
     ```bash
     rocky review "<topic>" --score 0.0 \
       --question "<asked Q>" --answer "[explained]" \
       --feedback "<the explanation you just gave>"
     ```
   - **`[x] delete`** — user says this topic is irrelevant or wrong.
     Confirm once ("Delete *<topic>* from your PKG? This is permanent.
     [y/N]"), then on `y`:
     ```bash
     rocky delete-topic "<topic>"
     ```
     Move on to the next topic. No review recorded.
   - **Free-text answer** — grade in [0, 1]:
     - `1.0` — captured the core insight
     - `0.7` — partial: right direction, missing nuance
     - `0.4` — wrong direction but in the neighbourhood
     - `0.0` — total miss

     Give one-sentence feedback. If they missed, state the correct insight
     using `description` + relevant `context`. Be terse.

4. **Record the review** (for free-text or `[?]` only):
   ```bash
   rocky review "<topic>" \
     --score 0.7 \
     --question "<the question you asked>" \
     --answer "<what the user typed>" \
     --feedback "<your one-sentence feedback>"
   ```
   Output echoes `new_retrievability` so you can show how recall just
   changed.

5. **Save good questions to the bank.** If the question you asked was
   well-formed (probed insight, was answerable, drew from real context),
   persist it for future rotation:
   ```bash
   rocky add-question \
     --topic "<topic>" \
     --question "<the Q>" \
     --answer "<ideal A>" \
     --clue "<one-line hint>"
   ```
   Skip this on `[s]` / `[h]` simpler/harder variants — only save the
   primary question that earned a real review.

6. **Loop.** Move to the next topic. After the last, summarise:
   `N topics drilled · avg score X.XX · M still weak (recall < 0.3)`.

## Rules

- **One topic, one primary question.** `[s]` / `[h]` are reframes of the
  same topic, not new topics — don't pile up extra reviews.
- **Honest grading.** Inflated scores break the FSRS schedule. If they
  didn't know it, give 0.0 — Rocky surfaces it again sooner.
- **Use the context.** If `contexts` has a diff hunk or code fragment,
  work it into the question or feedback. Grounded > abstract.
- **Don't generate fake citations.** If the topic mentions a function or
  file you can't verify exists, ask about the *concept* instead of the
  *artifact*.
- **Show repos.** When a topic spans multiple projects (`repos.length > 1`),
  briefly note it: *"This came up in: foo-svc, billing-api."*
- **Voice / tone**: terse, neutral, direct. No "Great question!", no
  "Excellent answer!". Coach, not cheerleader.

## When the user says stop

Acknowledge, summarise (`N drilled, avg X.XX`). Do not record any review
for the unanswered topic.

## Voice mode (out of scope)

Voice answers (mic input, TTS) only work in `rocky quiz --voice` (CLI) or
the web UI's push-to-talk button. Inside Claude Code the answer is text.
If the user asks for voice, point them to those interfaces.

## Boundaries

Reads + writes the local PKG only. Doesn't modify code, doesn't push to
git, doesn't contact any external service. To see full state: `rocky stats`
or `rocky view`.
