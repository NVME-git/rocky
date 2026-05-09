---
name: rocky-review
description: >
  Wrap-up skill — runs rocky-checkpoint (extracts learnings from recent
  commits) followed by rocky-promptiq (rescores recent prompts), then
  prints a fused summary. Use as the default "end of work session"
  invocation. Trigger phrases: "/rocky-review", "rocky review",
  "wrap up the session", "end of session review".
---

# Rocky · Review

Umbrella skill that runs the two end-of-session passes back-to-back so
the user only has to invoke one command. This is the **recommended
default** for "I'm done for now, capture what I learned and grade how I
asked for it." For CI / automation, prefer the underlying skills directly
— `rocky-checkpoint` after every push, `rocky-promptiq` on a slower
cadence (weekly cron, etc.).

The two underlying skills evaluate different things:

- `rocky-checkpoint` is *generative* — it mines atomic durable concepts
  from the commits and the conversation, and writes them to the PKG.
- `rocky-promptiq` is *critical* — it scores the user's prompts against
  a rubric for prompting quality.

Running them in sequence (rather than blended into one pass) is
deliberate. Switching evaluative stance is easier between two clean runs
than inside one — interleaving "what should I learn" with "how well did
I prompt" risks dragging one into the other (e.g. "this prompt was
vague AND so this topic is shallow," which is a category error).

## Procedure

1. **Run rocky-checkpoint to completion.** Follow its skill file exactly
   — read the queue, dedup against the global graph, extract topics,
   add questions, add edges, drain the queue. The full skill body lives
   at `~/.claude/skills/rocky-checkpoint/SKILL.md` (or the OpenCode
   equivalent); load it into your context if it isn't already, and
   execute every step. Do *not* short-circuit it.

   Capture the final report line so you can fuse it later:
   `Checkpointed N commits → M commit topics + P conversation topics, K merged (J across projects), Q questions added, E edges.`

2. **If checkpoint succeeded, run rocky-promptiq.** Same rule — load and
   execute the full skill at `~/.claude/skills/rocky-promptiq/SKILL.md`.
   It pulls unscored prompts, judges each against the rubric, persists
   each evaluation via `rocky prompt-eval`, and prints a per-batch
   summary.

   Capture: number scored, batch average, lift over the heuristic
   baseline, top patterns to work on.

3. **If checkpoint failed (any non-recoverable error in step 1), do
   *not* run promptiq.** Report the checkpoint failure and stop.
   Failure isolation is part of the design — promptiq is independent
   work and there's no value in running it against a broken queue
   state. The user can re-invoke `/rocky-review` after fixing the
   underlying issue.

4. **Print one fused summary** to the user, format:

   ```
   /rocky-review complete.

     PKG  · Checkpointed N commits → M new topics, K merged
            (J cross-project), Q questions, E edges.
     Prompts · Rescored N prompts. Avg <score>, lift +<N> vs heuristic.
                Top pattern: <one-line synthesis>.
   ```

   Two lines, two domains, no preamble. The user already knows what
   they invoked.

5. **Suggest** `rocky view` for the live PKG graph + PromptIQ trend tab.

## Rules

- **Never blend the two passes.** Running checkpoint and promptiq
  concurrently in one LLM context tends to bleed evaluative stance
  between them. Run them strictly sequentially with a clean handoff.
- **The skills are the source of truth, not this file.** If the
  underlying skill files disagree with the fused summary format above,
  follow each skill's own instructions and adjust the summary accordingly.
- **Don't double-drain the queue.** rocky-checkpoint runs `rocky
  checkpoint mark` itself; this skill just invokes it.
- **Don't double-score prompts.** rocky-promptiq's `--unscored` flag
  already filters; trust it.
- **CI integrations should bypass this skill** and call
  `rocky-checkpoint` or `rocky-promptiq` directly. Bundling two
  skills makes sense for an interactive wrap-up, not for an automated
  hook that should fail loudly on a single broken step.

## Boundaries

This skill writes nothing of its own. All persistence is via the
underlying skills' `rocky add-topic`, `rocky add-question`,
`rocky add-edge`, `rocky checkpoint mark`, and `rocky prompt-eval`
calls. Same undo paths apply: `rocky delete-topic <name>` for any topic
recorded incorrectly; PromptIQ scores can be refreshed by re-invoking
the underlying skill.
