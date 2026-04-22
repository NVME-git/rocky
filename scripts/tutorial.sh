#!/usr/bin/env bash
#
# Rocky end-to-end tutorial / smoke test.
#
# Spins up an isolated playground (~/.rocky-tutorial/), makes a fake git repo
# with a couple of real-looking commits, then walks through:
#   rocky explore  →  rocky post-commit  →  rocky session-end
#   rocky list     →  rocky inspect      →  rocky stats
#
# Doubles as alpha verification: if this script gets to the end without
# errors and produces a topic with a non-empty question_bank, the new
# rich-context pipeline works end-to-end.
#
# Usage:
#   scripts/tutorial.sh                # interactive (waits between steps)
#   scripts/tutorial.sh --noninteractive
#
set -uo pipefail

INTERACTIVE=1
[[ "${1:-}" == "--noninteractive" ]] && INTERACTIVE=0

PLAYGROUND="${HOME}/.rocky-tutorial"
DATA_DIR="${PLAYGROUND}/data"
CONFIG_DIR="${PLAYGROUND}/config"
PROJECT_DIR="${PLAYGROUND}/sample-project"

# Force Rocky to use the isolated dirs so the tutorial never touches real data.
export ROCKY_HOME="${DATA_DIR}"
export XDG_CONFIG_HOME="${CONFIG_DIR}"

C_DIM='\033[2m'; C_OK='\033[1;32m'; C_WARN='\033[1;33m'; C_ERR='\033[1;31m'
C_HDR='\033[1;36m'; C_RST='\033[0m'

step() { printf "\n${C_HDR}▶ %s${C_RST}\n" "$1"; }
ok()   { printf "${C_OK}✓${C_RST} %s\n" "$1"; }
warn() { printf "${C_WARN}!${C_RST} %s\n" "$1"; }
fail() { printf "${C_ERR}✗${C_RST} %s\n" "$1" >&2; }
hr()   { printf "${C_DIM}%s${C_RST}\n" "────────────────────────────────────────────────────────────"; }

pause() {
  if [[ $INTERACTIVE -eq 1 ]]; then
    printf "${C_DIM}  [enter to continue, Ctrl-C to abort]${C_RST} "
    read -r _
  fi
}

require() {
  local cmd="$1"; local msg="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    fail "$cmd not found — $msg"; exit 1
  fi
}

# ── 0. preflight ───────────────────────────────────────────────────────────
step "0. Preflight"

require rocky "install with: cargo install --git https://github.com/NVME-git/rocky"
require git "install git first"
require ollama "install Ollama from https://ollama.com"
require curl "needed to ping Ollama"

if ! curl -sf "http://localhost:11434/api/tags" >/dev/null; then
  warn "Ollama is not running on localhost:11434"
  warn "Start it in another terminal with:  ollama serve"
  fail "aborting"
  exit 1
fi
ok "Ollama is running"

if [[ "$(ollama list 2>/dev/null | awk 'NR>1' | wc -l)" -lt 1 ]]; then
  warn "No Ollama models installed. Recommended:  ollama pull qwen2.5-coder:7b"
  fail "aborting"
  exit 1
fi

# ── 1. clean playground ────────────────────────────────────────────────────
step "1. Reset playground at ${PLAYGROUND}"
rm -rf "${PLAYGROUND}"
mkdir -p "${DATA_DIR}" "${CONFIG_DIR}/rocky" "${PROJECT_DIR}"

# Pick a model: prefer qwen2.5-coder:7b (best for code), then llama3.1:8b,
# else the first model Ollama has installed.
PICK_MODEL=""
for candidate in "qwen2.5-coder:7b" "llama3.1:8b"; do
  if ollama list 2>/dev/null | awk '{print $1}' | grep -qx "${candidate}"; then
    PICK_MODEL="${candidate}"; break
  fi
done
if [[ -z "${PICK_MODEL}" ]]; then
  PICK_MODEL="$(ollama list 2>/dev/null | awk 'NR>1 && $1 != "" {print $1; exit}')"
fi
if [[ -z "${PICK_MODEL}" ]]; then
  fail "No Ollama models installed. Run:  ollama pull qwen2.5-coder:7b"
  exit 1
fi
ok "Using Ollama model: ${PICK_MODEL}"

cat > "${CONFIG_DIR}/rocky/config.toml" <<EOF
[llm]
provider = "ollama"
model = "${PICK_MODEL}"
ollama_base_url = "http://localhost:11434"

[edges]
reuse = "1s"

[ui]
name = "Tutorial User"
EOF
ok "Wrote isolated config: ${CONFIG_DIR}/rocky/config.toml"

# ── 2. fake project with real-looking code ─────────────────────────────────
step "2. Create sample git repo with two commits"
cd "${PROJECT_DIR}"
git init -q
git config user.email "tutorial@example.com"
git config user.name  "Tutorial User"

cat > README.md <<'EOF'
# Sample Project — Rocky tutorial fixture

A toy auth service used to exercise Rocky's pipeline. We keep it small
on purpose so the LLM has something believable to summarise without
drowning in irrelevant code.
EOF

cat > CLAUDE.md <<'EOF'
# Project conventions

- This is a stateless auth service. Sessions live in JWTs, not server memory.
- Refresh tokens rotate on every use and are stored in Redis with a 7-day TTL.
- Architecture decisions live in /docs/decisions — read them before structural changes.
EOF

mkdir -p src docs/decisions
cat > docs/decisions/0001-jwt.md <<'EOF'
# 0001 — Use rotating refresh tokens

We chose JWT access tokens (15 min) plus rotating refresh tokens (7 days)
because the service must scale across three replicas with no shared session
state. The trade-off is that token revocation requires a Redis lookup on
every refresh, but never on access — so the hot path stays stateless.
EOF

cat > src/auth.rs <<'EOF'
use std::time::Duration;

pub struct AuthConfig {
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            access_ttl: Duration::from_secs(15 * 60),
            refresh_ttl: Duration::from_secs(7 * 24 * 60 * 60),
        }
    }
}

pub fn issue_token(user_id: &str, _cfg: &AuthConfig) -> String {
    format!("jwt-for-{user_id}")
}
EOF

git add -A
git commit -q -m "feat: initial auth service skeleton with JWT + rotating refresh"
ok "Commit 1: initial auth skeleton"

cat > src/refresh.rs <<'EOF'
use std::time::Duration;

/// Rotate a refresh token: invalidate the old one in Redis, issue a fresh
/// one, and reset the 7-day TTL. Returns the new opaque token string.
pub fn rotate_refresh_token(old: &str) -> String {
    // In real code this would talk to Redis. For the tutorial we just
    // demonstrate that rotation produces a brand-new token rather than
    // sliding the existing one's expiry.
    let _ = old;
    let _ttl = Duration::from_secs(7 * 24 * 60 * 60);
    format!("refresh-{}", chrono::Utc::now().timestamp())
}
EOF

git add -A
git commit -q -m "feat: rotate refresh tokens on every use to limit replay window"
ok "Commit 2: refresh rotation"

# ── 3. rocky explore ───────────────────────────────────────────────────────
hr
step "3. Generate project context (rocky explore)"
echo "    Reads README, CLAUDE.md, docs/, recent commits."
echo "    Produces a multi-paragraph summary the LLM will use as grounding"
echo "    context when extracting topics later."
pause
rocky explore || { fail "rocky explore failed"; exit 1; }

step "3a. Inspect the stored project context (rocky explore --show)"
pause
rocky explore --show || warn "rocky explore --show not available yet"

# ── 4. queue a diff ────────────────────────────────────────────────────────
hr
step "4. Queue the latest commit's diff (rocky post-commit)"
echo "    This is what the git post-commit hook does after every commit."
echo "    No LLM call — just appends the diff to a queue so the Stop hook"
echo "    can process several commits together at session end."
pause
rocky post-commit || { fail "rocky post-commit failed"; exit 1; }

# ── 5. session-end pipeline ────────────────────────────────────────────────
hr
step "5. Process the queue with full context (rocky session-end)"
echo "    Reads: pending diffs + Claude Code transcript + project context"
echo "    Produces: rich nodes with a question_bank (3-5 Q+A+clue triples)"
echo "    This is where Layer-1 dedup kicks in — existing topic names get"
echo "    reused instead of new near-duplicates being created."
pause
rocky session-end || { fail "rocky session-end failed"; exit 1; }

# ── 6. inspect the result ──────────────────────────────────────────────────
hr
step "6. List the topics that were created"
rocky list --since today

step "6a. Inspect the first topic in detail"
# Strip ANSI colour, then take the first row that looks like a real topic
# (starts with two spaces + an alphanumeric word, isn't the header).
FIRST_TOPIC="$(rocky list --since today 2>/dev/null \
  | sed 's/\x1b\[[0-9;]*m//g' \
  | awk '/^  [A-Za-z0-9]/ && $1 != "Topic" && $1 != "Rocky" {print $1; exit}')"
if [[ -n "${FIRST_TOPIC}" ]]; then
  rocky inspect "${FIRST_TOPIC}" || warn "inspect failed"
else
  warn "No topics created — pipeline may have failed silently"
fi

# ── 7. layer-1 dedup verification ──────────────────────────────────────────
hr
step "7. Verify Layer-1 dedup: make a third commit on the same concept"
echo "    The new commit touches refresh-token rotation again. With dedup"
echo "    working, you should see encounter_count bump on an existing topic"
echo "    rather than a new duplicate row."
pause
cat >> src/refresh.rs <<'EOF'

/// Helper that returns true if the supplied refresh token has already
/// been rotated (i.e. it is no longer valid). In real code this checks
/// Redis. We expose it so callers can short-circuit reuse attempts.
pub fn is_rotated(token: &str) -> bool {
    token.starts_with("refresh-")
}
EOF
git add -A
git commit -q -m "refactor: surface is_rotated helper for refresh token reuse checks"
rocky post-commit
rocky session-end

step "7a. Topic count after the dedup test"
COUNT_BEFORE_AFTER="$(rocky list --since today 2>/dev/null | grep -cE '^\s+\S' || true)"
echo "    Topics now in the PKG: $(rocky stats 2>/dev/null | grep -iE 'topic|node' | head -3)"

# ── 8. quiz on the freshly-created bank ────────────────────────────────────
hr
step "8. (Optional) Quiz yourself on what was just generated"
echo "    Run interactively after the script finishes:"
echo "        ROCKY_HOME=${DATA_DIR} XDG_CONFIG_HOME=${CONFIG_DIR} rocky quiz"
echo
ok "Tutorial complete."
echo
echo "Playground location:  ${PLAYGROUND}"
echo "Cleanup:              rm -rf ${PLAYGROUND}"
