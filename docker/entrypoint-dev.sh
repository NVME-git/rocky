#!/usr/bin/env bash
# Dev profile entrypoint.
#
# Drops the user into bash with rocky on PATH, a writable workspace,
# config bootstrapped from the template, and an Ollama health check.
set -euo pipefail

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

ROCKY_OLLAMA_HOST="${ROCKY_OLLAMA_HOST:-host.docker.internal:11434}"
ROCKY_OLLAMA_MODEL="${ROCKY_OLLAMA_MODEL:-llama3.2}"

# ── Config bootstrap ─────────────────────────────────────────────────────────
mkdir -p "$HOME/.config/rocky" "$HOME/.rocky"
if [[ ! -f "$HOME/.config/rocky/config.toml" ]]; then
    sed -e "s|\${ROCKY_OLLAMA_HOST}|${ROCKY_OLLAMA_HOST}|g" \
        -e "s|\${ROCKY_OLLAMA_MODEL}|${ROCKY_OLLAMA_MODEL}|g" \
        /opt/config-template.toml > "$HOME/.config/rocky/config.toml"
fi

# ── Banner ───────────────────────────────────────────────────────────────────
echo
echo -e "${CYAN}  ╭─────────────────────────────────────────────────────────────╮${NC}"
echo -e "${CYAN}  │                                                             │${NC}"
echo -e "${CYAN}  │   Rocky · containerized dev environment                     │${NC}"
echo -e "${CYAN}  │                                                             │${NC}"
echo -e "${CYAN}  ╰─────────────────────────────────────────────────────────────╯${NC}"
echo

# ── Ollama check ─────────────────────────────────────────────────────────────
echo -ne "  Checking Ollama at ${ROCKY_OLLAMA_HOST} ... "
if curl -sf --max-time 3 "http://${ROCKY_OLLAMA_HOST}/api/tags" > /dev/null; then
    echo -e "${GREEN}reachable${NC}"
    if ! curl -sf "http://${ROCKY_OLLAMA_HOST}/api/tags" | grep -q "\"${ROCKY_OLLAMA_MODEL}"; then
        echo -e "  ${YELLOW}⚠  model '${ROCKY_OLLAMA_MODEL}' not found on host Ollama.${NC}"
        echo -e "     Run on the host:  ${CYAN}ollama pull ${ROCKY_OLLAMA_MODEL}${NC}"
    fi
else
    echo -e "${RED}unreachable${NC}"
    echo -e "  ${YELLOW}Start Ollama on the host first:  ${CYAN}ollama serve${NC}"
    echo -e "  ${YELLOW}Then pull a model:              ${CYAN}ollama pull ${ROCKY_OLLAMA_MODEL}${NC}"
fi
echo

# ── Hint ─────────────────────────────────────────────────────────────────────
cat <<EOF
  Workspace: /workspace  (mount your repo here)
  Config:    ~/.config/rocky/config.toml
  Data:      ~/.rocky/

  Try:
    rocky --help
    rocky explore                        # if /workspace has a repo
    rocky install                        # post-commit hook
    rocky install claude-all             # full Claude-aware setup
    rocky quiz
    ROCKY_BIND=0.0.0.0:7777 rocky view   # then open http://localhost:7777
    rocky quiz --voice                   # CLI voice (needs host audio passthrough)

  Toy project (if you don't have a repo to mount):
    cp -r /opt/toy-project /workspace/urlshort && cd /workspace/urlshort
    git init && git add . && git commit -m "init"
    rocky explore && rocky install

EOF

exec bash --login
