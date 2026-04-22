#!/usr/bin/env bash
# Demo profile entrypoint.
#
# Walks through the full Rocky lifecycle against a bundled toy project,
# pausing briefly between stages so a viewer can read the output.
# Designed for screen recordings and live demos.
set -euo pipefail

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

ROCKY_OLLAMA_HOST="${ROCKY_OLLAMA_HOST:-host.docker.internal:11434}"
ROCKY_OLLAMA_MODEL="${ROCKY_OLLAMA_MODEL:-llama3.2}"
DEMO_PAUSE="${DEMO_PAUSE:-2}"

pause() { sleep "${DEMO_PAUSE}"; }

step() {
    echo
    echo -e "${MAGENTA}━━━ ${BOLD}$1${NC}${MAGENTA} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo
    pause
}

run() {
    echo -e "${CYAN}\$ $*${NC}"
    "$@"
    echo
}

# ── Config ───────────────────────────────────────────────────────────────────
mkdir -p "$HOME/.config/rocky" "$HOME/.rocky"
sed -e "s|\${ROCKY_OLLAMA_HOST}|${ROCKY_OLLAMA_HOST}|g" \
    -e "s|\${ROCKY_OLLAMA_MODEL}|${ROCKY_OLLAMA_MODEL}|g" \
    /opt/config-template.toml > "$HOME/.config/rocky/config.toml"

# ── Ollama gate ──────────────────────────────────────────────────────────────
echo
echo -e "${CYAN}  ╭─────────────────────────────────────────────────────────────╮${NC}"
echo -e "${CYAN}  │   Rocky demo · automated lifecycle walkthrough              │${NC}"
echo -e "${CYAN}  ╰─────────────────────────────────────────────────────────────╯${NC}"
echo
echo -ne "  Ollama at ${ROCKY_OLLAMA_HOST} ... "
if ! curl -sf --max-time 5 "http://${ROCKY_OLLAMA_HOST}/api/tags" > /dev/null; then
    echo -e "${YELLOW}unreachable — start it on the host first:${NC}"
    echo "    ollama serve"
    echo "    ollama pull ${ROCKY_OLLAMA_MODEL}"
    exit 1
fi
echo -e "${GREEN}reachable${NC}"

if ! curl -sf "http://${ROCKY_OLLAMA_HOST}/api/tags" | grep -q "\"${ROCKY_OLLAMA_MODEL}"; then
    echo -e "  ${YELLOW}Model '${ROCKY_OLLAMA_MODEL}' not found. On the host run:${NC}"
    echo "    ollama pull ${ROCKY_OLLAMA_MODEL}"
    exit 1
fi
pause

# ── Stage the toy project as fresh commits ───────────────────────────────────
step "1 · Stage toy project (urlshort) with simulated commit history"
cp -r /opt/toy-project /workspace/urlshort
cd /workspace/urlshort
rm -rf .git node_modules dist
git init -q
git add README.md package.json tsconfig.json
git commit -q -m "init: scaffold typescript url shortener"
git add src/store.ts src/server.ts
git commit -q -m "feat: express server backed by sqlite store"
git add src/encode.ts tests/encode.test.ts
git commit -q -m "feat: base62 short ids with round-trip tests"
git add src/rateLimit.ts
git commit -q -m "feat: sliding-window rate limit on POST /shorten"
git add src/validate.ts
git commit -q -m "fix: reject javascript: and data: schemes before storing"
git log --oneline

# ── Explore ──────────────────────────────────────────────────────────────────
step "2 · rocky explore — build project context summary"
run rocky explore --show

# ── Install hooks (visible, but commits are already there) ───────────────────
step "3 · rocky install — wire the Claude-aware post-commit + stop hooks"
run rocky install claude-all || run rocky install

# ── Backfill the existing commits into the graph ─────────────────────────────
step "4 · rocky backfill — extract concepts from the commits we just made"
run rocky backfill --limit 5

# ── Run session-end too, in case anything was queued ─────────────────────────
step "5 · rocky session-end — drain anything left in the post-commit queue"
run rocky session-end || true

# ── Show what landed in the graph ────────────────────────────────────────────
step "6 · rocky stats / list — see the knowledge graph that just formed"
run rocky stats
run rocky list

# ── Wrap up ──────────────────────────────────────────────────────────────────
step "7 · Next steps"
cat <<EOF
  ${GREEN}✓${NC} Demo lifecycle complete.

  Web UI (open in the host browser):
    ${CYAN}ROCKY_BIND=0.0.0.0:7777 rocky view${NC}
    then visit http://localhost:7777
    (web UI voice push-to-talk works there — browser captures the mic)

  Interactive quiz:
    ${CYAN}rocky quiz${NC}

  Re-running the demo profile gives you a clean graph each time.
EOF

# Keep the container alive long enough for the user to optionally run rocky view.
echo
echo -e "  ${YELLOW}Sleeping — Ctrl-C to exit, or docker exec into this container.${NC}"
sleep infinity
