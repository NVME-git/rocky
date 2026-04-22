# Architecture Decision Records

Each file captures one decision in the [Michael Nygard ADR format](https://github.com/joelparkerhenderson/architecture-decision-record): context, decision, consequences. ADRs are *historical* — when a decision is reversed, write a new ADR that supersedes the old one rather than editing the old file.

## Index

| # | Title | Status |
|---|---|---|
| [0001](0001-rich-context-pipeline.md) | Rich-context pipeline (explore + queue + session-end) | Accepted |
| [0002](0002-dedup-and-question-bank.md) | Layer-1 dedup + question bank with rotation | Accepted |
| [0003](0003-llm-resilience.md) | LLM resilience: timeouts, retries, lenient parsing | Accepted |
| [0004](0004-config-paths-and-privacy.md) | Config layout, ROCKY_HOME, and privacy.strict | Accepted |
| [0005](0005-rocky-iq-and-ui.md) | Rocky IQ score + sidebar UI redesign | Accepted |
| [0006](0006-voice-architecture.md) | Voice architecture (planned for v0.2) | Proposed |

## Convention

When adding a new ADR:

1. Number it sequentially (next is `0007-...`).
2. Use kebab-case for the title slug.
3. Status starts as `Proposed` and moves to `Accepted` once the work lands. Use `Superseded by NNNN` to retire one.
4. Keep it tight — context is *why this even came up*, not project background. Decision is *what we chose*. Consequences include positive AND negative — if there are no costs, the ADR is incomplete.

## Discoverability

Rocky's `rocky explore` command reads `docs/architecture/`, `docs/decisions/`, and similar directories. Adding ADRs here makes them part of the project context that future quiz questions are grounded in.
