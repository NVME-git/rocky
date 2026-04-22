# 0004 — Config layout, ROCKY_HOME, and privacy.strict

**Status:** Accepted (2026-04-22, alpha)

## Context

Three related issues showed up around storage and isolation:

1. **Config in the wrong place.** The previous global config lived at `~/.rocky/.rocky.toml` — a hidden file inside an already-named directory. Editors and tooling expect user config in `~/.config/`. The path also meant the data dir mixed user-editable settings with generated state (`graph.db`, `pkg/`, `summaries/`).

2. **No way to isolate Rocky for testing.** The smoke-test script (`scripts/tutorial.sh`) needs to make a fake project, exercise the full pipeline, and then leave the user's real PKG untouched. Without isolation, a tutorial run would write 4 throwaway topics into the user's actual `~/.rocky/graph.db`.

3. **No hard guarantee against code exfiltration.** Rocky sends diffs to the configured LLM provider. With `provider = "claude"`, that diff goes to api.anthropic.com. Some users will install Rocky inside an org that strictly forbids code leaving the machine — they need a single config flag that *refuses* to use any non-local provider, even if they (or a teammate, or a future config edit) flipped to Claude by accident.

## Decision

**Split config from data along XDG conventions.**

| Path | Role |
|---|---|
| `~/.config/rocky/config.toml` | User-editable settings (loaded via `dirs::config_dir()`) |
| `~/.rocky/` | Data dir: `graph.db`, `pkg/`, `summaries/`, view assets |
| `./.rocky.toml` | Project-level config override (unchanged) |

A silent one-time auto-migration moves `~/.rocky/.rocky.toml` to `~/.config/rocky/config.toml` on first load if the new path doesn't exist. Prints one notice line on stderr.

**`ROCKY_HOME` env var overrides the data dir.** The tutorial script sets `ROCKY_HOME=~/.rocky-tutorial/data` so every Rocky invocation it makes touches that throwaway dir. Same trick works for any user who wants a per-context PKG (e.g. one for work, one for personal).

**`[privacy] strict = true` refuses non-local LLM providers.** Implemented at the boundary in `make_teacher`:
```rust
if cfg.privacy_strict && cfg.llm_provider != "ollama" {
    bail!("privacy.strict = true forbids non-local LLM providers, ...");
}
```
The user gets a clear error message at command time — not a silent send. They can flip `privacy.strict = false` if they explicitly want the cloud provider.

## Consequences

**Positive:**
- Config edits land in the conventional place, no surprise to anyone using `chezmoi` / `dotfiles` to manage `~/.config/`.
- Tutorial isolation works — `scripts/tutorial.sh` writes to a sandbox and a `rm -rf ~/.rocky-tutorial/` is a clean cleanup.
- Privacy-strict gives an org-policy-friendly story: *"Rocky is configured with `privacy.strict = true`, so it cannot send diffs externally even if reconfigured."*
- The legacy path migrates without user action — existing alpha users see one stderr notice and otherwise nothing changes.

**Negative / costs:**
- Two paths to remember (config dir vs data dir). Documented in the "Where Rocky stores data" table in README + Flutter docs.
- `ROCKY_HOME` is data-only — it does NOT redirect config. We rely on `XDG_CONFIG_HOME` for that, which the tutorial sets in tandem. A combined `ROCKY_PROFILE` could simplify this; deferred unless users hit it.

**Verification:**
- `scripts/tutorial.sh` sets both env vars and confirms its work touches only `~/.rocky-tutorial/`.
