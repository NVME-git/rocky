---
name: rocky-promptiq
description: >
  Re-evaluate the user's recent prompts using your own judgment to produce a
  PromptIQ score (0-100) per prompt and short feedback. Use this when the user
  asks to refresh their PromptIQ, see how their prompting is improving, or get
  feedback on recent prompts. Trigger phrases: "/rocky-promptiq",
  "rescore my prompts", "evaluate my prompting", "promptiq".
---

# Rocky · PromptIQ

Walk through the user's recent prompts and score each one for *prompting
quality* — how well-formed, specific, and actionable each prompt was as a
direction to an AI agent. This is independent of whether the AI succeeded —
you're judging the user's input, not the model's output.

## Scoring rubric (0-100, weighted)

| Dimension | What earns points | Weight |
|---|---|---|
| **Specificity** | Names files, functions, libraries, frameworks. Concrete nouns. | 30 |
| **Context** | References to current state ("the X currently does Y"), constraints, why-now. | 20 |
| **Actionability** | Imperative verb + concrete object + clear deliverable. | 20 |
| **Verification** | Mentions tests, edge cases, error handling, acceptance criteria. | 15 |
| **Anti-patterns** (penalty) | Vague phrasing without follow-up: "build me X", "fix it", "make it work". | -20 |

Examples to anchor your scale:
- *"build me an app"* → ~15 (no specificity, no context, anti-pattern)
- *"build me a MERN stack todo app with JWT auth"* → ~50 (specific tech + scope)
- *"refactor `parse_csv` to handle empty rows; add a test that covers the empty case"* → ~85 (specificity, actionability, verification)

## Steps

1. Pull the prompts that haven't been agent-rescored yet:
   ```bash
   rocky prompts --unscored --since-days 7
   ```
   Returns a JSON array of `{id, ts, source, project_path, prompt, heuristic_score}`.

2. For each prompt, judge it against the rubric above. Compute a 0-100 score and
   write a one-line feedback note (≤160 chars). The feedback should call out the
   weakest dimension specifically — e.g. "vague: no file/function references" or
   "good: specific + verification clear".

3. Persist each result:
   ```bash
   rocky prompt-eval --id <N> --score <0..100> --feedback "<one line>"
   ```

4. After all prompts in the batch, print a short summary to the user:
   - Number of prompts scored
   - Average score this batch
   - Trend vs the heuristic-only baseline
   - Top 1-2 patterns to work on (synthesized from feedback)

5. Suggest the user open the dashboard tile (`rocky view` → PromptIQ) for the
   live trend.

## Notes

- The heuristic baseline already scored each prompt at log time. Your role is to
  refine that score with judgment the heuristic can't apply (e.g., "this prompt
  is short but it's a good clarifying question, so don't penalize length").
- Keep feedback specific and actionable. Avoid generic praise/criticism.
- If a prompt is genuinely ambiguous as scored (e.g., a chunk of code with no
  English text), score it neutrally (~50) and note "non-prompt content".
