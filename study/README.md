<style>
body {
    font-family: 'Roboto', sans-serif;
}
code, pre {
    font-family: 'Fira Code', 'Courier New', monospace;
}
</style>

# Rocky — Study Diagrams

Interactive Excalidraw diagrams covering all current topic lifecycle logic.
Use these to study, annotate your decisions, and brief Claude on what to build next.

## How to open

**VS Code** — install the [Excalidraw extension](https://marketplace.visualstudio.com/items?itemName=pomdtr.excalidraw-editor),
then open any `.excalidraw` file directly.

**Browser** — drag any `.excalidraw` file onto [excalidraw.com](https://excalidraw.com).

## Files

| File | What it covers | Key source files |
|---|---|---|
| `01_topic_lifecycle.excalidraw` | Topic state machine — all states and transitions | `src/main.rs`, `src/db.rs` |
| `02_data_model.excalidraw` | Node struct + all DB tables and their fields | `src/node.rs`, `src/db.rs` |
| `03_quiz_flow.excalidraw` | Quiz session decision tree — question selection, all options, scoring | `src/main.rs run_quiz_topic()` |
| `04_discovery_sources.excalidraw` | The 5 ways topics enter the PKG, with initial values per source | `src/main.rs run_diff/backfill/session_end/explore()` |
| `05_fsrs_model.excalidraw` | Memory model — retrievability formula, decay examples, stability updates | `src/fsrs.rs`, `src/db.rs` |
| `06_source_classification.excalidraw` | The ai_prompt vs own_code problem space — signals, reliability, proposed approach | `BACKLOG.md` |
| `07_rich_context_pipeline.excalidraw` | The 3-stage enrichment pipeline: explore → post-commit queue → session-end | `src/main.rs`, `src/transcript.rs` |
| `08_rocky_iq_and_ui.excalidraw` | Rocky IQ formula + 5-tab web UI layout and data flow | `src/server.rs`, `src/app.html` |
| `09_voice_architecture.excalidraw` | Voice v0.2 — push-to-talk web UI, whisper.cpp backend, privacy tiers | `src/voice.rs`, `src/server.rs` |

## How to use

Each diagram has dashed **📝 ANNOTATION** boxes — these are your workspace.
Use them to record decisions, questions, or proposed changes.

When you're ready to brief Claude:

```
"Look at study/07_rich_context_pipeline.excalidraw — 
 I've added notes in the annotation boxes. 
 Implement based on what I've written there."
```

Claude can read `.excalidraw` files directly (they're JSON).

## Regenerating

If you want to reset a diagram to its original state:

```bash
python3 study/generate.py
```

This overwrites all `.excalidraw` files. **Save your annotations first** (export
as PNG from Excalidraw, or copy the JSON to a backup file).

## Open questions being studied

These are the decisions captured in the diagrams, waiting for your input:

1. **`co_authored` flag** — should it lower initial stability, or just be a display flag?
2. **`task_prompt` origin** — same treatment as `ai_prompt`, or different?
3. **Independence threshold** — what clean-pass count makes a topic "independently understood"?
4. **Retroactive source inference** — can we parse `Co-Authored-By` from git log for existing nodes?
5. **Prompted + committed** — when both signals exist, which wins?
6. **Struggle score storage** — which fields go in the `reviews` table vs. on the node itself?
7. **Session-end dedup threshold** — how similar must two topics be before layer-1 dedup rejects one?
8. **Rocky IQ recency window** — is 60-day half-life the right weight for recent vs. old topics?
9. **Voice CLI mode** — push-to-talk in terminal vs. always-listening with VAD?
