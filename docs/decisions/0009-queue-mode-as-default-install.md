# 0009 — Queue-mode as the only post-commit hook

Date: 2026-05-08 · Status: Accepted
Related: [0007 — Skill-only extraction](0007-skill-only-extraction-and-promptiq.md), [0008 — Agent-side generic questions](0008-agent-generic-questions-and-canonical-persistence.md)

## Context

`rocky install` had two paths for installing the git post-commit hook:

- **`rocky install git`** (the *default*) — wrote `rocky diff` into
  `.git/hooks/post-commit`. The legacy interactive flow: every commit
  shells out to a TTY-driven topic-extraction loop, prompting the user to
  add topics one at a time.
- **`rocky install claude-all`** — wrote `rocky post-commit` into the
  hook. The agent-driven queue mode: every commit silently enqueues its
  diff for the `/rocky-checkpoint` skill to drain at session end.

The skill-driven workflow has been the documented path since ADR 0007. The
legacy interactive path remained as the install default mostly by inertia.

## Failure mode

Three things went wrong simultaneously and produced silent data loss:

1. The post-commit hook returned exit 0 for every commit (`rocky diff`
   succeeds whether or not a topic is added), so there was no error
   signal in CI, in the user's terminal, or in any observable output.
2. The `/rocky-checkpoint` skill's only signal of trouble was an empty
   `pending_count` from `rocky checkpoint diff`. To anyone running the
   skill, this looked like "no recent commits to extract from" — the
   indistinguishable fingerprint of "the queue genuinely had nothing"
   and "the hook never enqueued anything in the first place".
3. The skill-driven workflow is documented as the default in the
   README, the ADR series (0007), and the install help text for
   `claude-all`. A new user reading the docs and running `rocky install`
   without a subcommand got the wrong hook — silently — for the entire
   life of their PKG until they noticed topics weren't appearing.

The bug surfaced in this session: the user had made many commits across
hours of UI work, and `/rocky-checkpoint` consistently reported zero
commit topics. Diagnosing it required reading the post-commit hook file
by hand and noticing it called `rocky diff` instead of `rocky post-commit`.

The fix at install time is one line. The data lost in the meantime is
unrecoverable through the queue (the diffs are gone from the queue
anyway — they were never enqueued). `rocky checkpoint history`
backfills retrospectively, but only because `git log` retains the
diffs independently of Rocky's queue.

## Decision

The legacy `rocky diff` path is removed from the post-commit hook
installer. `rocky install` (no subcommand) and `rocky install git` both
install the queue-mode hook. The `rocky diff` command itself is **kept**
as a manual invocation — useful for ad-hoc "quiz me on what I just
committed" — but no install path wires it as a hook anymore.

Concretely:

- The legacy `install_git_hook()` (which wrote `rocky diff` to the
  hook) is deleted.
- `install_git_hook_queue_mode()` is renamed to `install_git_hook()` so
  the dispatcher in `Cmd::Install` for `HookTarget::Git` now installs
  the queue-mode hook.
- The legacy installer's behaviour of upgrading-in-place is preserved:
  if the hook already contains `rocky diff` from a pre-fix install, the
  installer rewrites it to `rocky post-commit` — anyone re-running
  `rocky install` gets fixed without manually editing the hook.
- The `HookTarget::Git` doc string changes from *"runs `rocky diff`
  after every commit"* to *"silently queues each commit for the
  `/rocky-checkpoint` skill"*.
- `rocky install claude-all` still calls the same installer function —
  the deprecation makes one less code path to maintain, not a
  regression for `claude-all` users.

## Consequences

**Positive:**

- New users running `rocky install` get the documented agent-driven
  workflow on the first try. The previously-loud failure mode (data
  silently dropped) becomes the previously-impossible failure mode.
- One installer function, one hook command. No more `*_queue_mode`
  suffix, no more parallel paths to keep in sync.
- The upgrade-in-place behaviour means existing installs heal on the
  next `rocky install` call without requiring users to edit
  `.git/hooks/post-commit` by hand or run `rocky uninstall git` first.

**Negative:**

- Anyone who genuinely relied on `rocky diff` as a post-commit hook
  (the interactive flow firing on every commit) loses that integration.
  They can run `rocky diff` manually any time for the same effect.
  Worth noting in the changelog.
- The skill is now the single supported "what to do after a commit"
  path. If the skill ecosystem changes (Claude Code or OpenCode breaks
  skill discovery), users have no fallback wired in. A future ADR may
  re-introduce a non-skill path; for now, the skill ecosystem is stable
  and queue-mode is portable across both supported agent harnesses.

## Related backlog

- The same diagnosis pattern — *successful hook, empty downstream* —
  applies anywhere Rocky writes to a queue-shaped store. Consider a
  health metric on consumer-side empty-rate (e.g.,
  `/rocky-checkpoint` reporting "queue empty for N runs") so the
  silent-drop failure mode can't return.
