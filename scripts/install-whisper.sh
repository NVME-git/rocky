#!/usr/bin/env bash
#
# scripts/install-whisper.sh
#
# One-shot installer for the local STT backend Rocky uses for `voice`.
# Detects platform, fetches a prebuilt whisper.cpp binary from the upstream
# GitHub releases, downloads the recommended `ggml-base.en.bin` model into
# ~/.rocky/models/, and adds (or refreshes) the [voice] block in your
# ~/.config/rocky/config.toml so the next `rocky view` is voice-enabled.
#
# Idempotent: re-running upgrades to the latest release without duplicating
# config entries.
#
# Usage:
#   ./scripts/install-whisper.sh                          # interactive
#   ./scripts/install-whisper.sh --noninteractive --yes   # CI / scripted
#   curl -fsSL <raw-url>/install-whisper.sh | sh          # one-liner
#
# Decisions in: docs/decisions/0006-voice-architecture.md
set -euo pipefail

ASSUME_YES=0
QUIET=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes|-y)            ASSUME_YES=1 ;;
    --noninteractive|-n) ASSUME_YES=1 ;;
    --quiet|-q)          QUIET=1 ;;
    -h|--help)
      grep -E "^#( |$)" "$0" | sed 's/^# \?//' | head -30
      exit 0 ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
  shift
done

C_DIM='\033[2m'; C_OK='\033[1;32m'; C_WARN='\033[1;33m'; C_ERR='\033[1;31m'
C_HDR='\033[1;36m'; C_RST='\033[0m'
log()  { [[ $QUIET -eq 1 ]] || printf "${C_DIM}  %s${C_RST}\n" "$*"; }
hdr()  { [[ $QUIET -eq 1 ]] || printf "\n${C_HDR}▶ %s${C_RST}\n" "$*"; }
ok()   { [[ $QUIET -eq 1 ]] || printf "${C_OK}✓${C_RST} %s\n" "$*"; }
warn() {                       printf "${C_WARN}!${C_RST} %s\n" "$*" >&2; }
fail() {                       printf "${C_ERR}✗${C_RST} %s\n" "$*" >&2; exit 1; }

confirm() {
  [[ $ASSUME_YES -eq 1 ]] && return 0
  printf "%s [y/N] " "$1"
  read -r ans
  [[ "$ans" =~ ^[Yy]$ ]]
}

# ── 1. Detect platform ───────────────────────────────────────────────────────
hdr "1. Detecting platform"

OS_KIND="$(uname -s)"
ARCH="$(uname -m)"
case "$OS_KIND" in
  Linux)
    case "$ARCH" in
      x86_64|amd64)   PLATFORM_TAG="linux-x64" ;;
      aarch64|arm64)  PLATFORM_TAG="linux-arm64" ;;
      *) fail "unsupported Linux arch: $ARCH (supported: x86_64, aarch64)" ;;
    esac ;;
  Darwin)
    case "$ARCH" in
      arm64) PLATFORM_TAG="macos-arm64" ;;
      x86_64) PLATFORM_TAG="macos-x64" ;;
      *) fail "unsupported macOS arch: $ARCH" ;;
    esac ;;
  *) fail "Windows is not supported by this script. See docs/decisions/0006-voice-architecture.md." ;;
esac
ok "Platform: $PLATFORM_TAG"

# ── 2. Resolve install paths ─────────────────────────────────────────────────
hdr "2. Resolving paths"

INSTALL_DIR="${ROCKY_VOICE_BIN_DIR:-$HOME/.local/bin}"
ROCKY_DATA_DIR="${ROCKY_HOME:-$HOME/.rocky}"
MODEL_DIR="$ROCKY_DATA_DIR/models"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/rocky"
CONFIG_FILE="$CONFIG_DIR/config.toml"
MODEL_NAME="ggml-base.en.bin"
MODEL_PATH="$MODEL_DIR/$MODEL_NAME"
BINARY_NAME="whisper-cli"
BINARY_PATH="$INSTALL_DIR/$BINARY_NAME"

mkdir -p "$INSTALL_DIR" "$MODEL_DIR" "$CONFIG_DIR"
log "Binary  → $BINARY_PATH"
log "Model   → $MODEL_PATH"
log "Config  → $CONFIG_FILE"

# ── 3. Verify required tools ─────────────────────────────────────────────────
hdr "3. Checking required tools"

for tool in curl tar mktemp; do
  command -v "$tool" >/dev/null 2>&1 || fail "$tool not installed"
done

# Note: we currently rely on a build-from-source fallback for Linux because
# whisper.cpp does not publish prebuilt binaries for every platform tag.
# macOS users can use `brew install whisper-cpp`; if that's already installed
# we'll detect it and just point Rocky at it.
if command -v whisper-cli >/dev/null 2>&1; then
  EXISTING="$(command -v whisper-cli)"
  ok "whisper-cli already installed at $EXISTING — skipping binary download"
  BINARY_PATH="$EXISTING"
else
  if [[ "$OS_KIND" == "Darwin" ]] && command -v brew >/dev/null 2>&1; then
    if confirm "Use Homebrew to install whisper-cpp (recommended on macOS)?"; then
      hdr "4a. brew install whisper-cpp"
      brew install whisper-cpp
      BINARY_PATH="$(command -v whisper-cli)"
      ok "Installed via brew at $BINARY_PATH"
    fi
  fi

  if [[ ! -x "$BINARY_PATH" ]]; then
    hdr "4b. Build whisper.cpp from source"
    log "No prebuilt available for $PLATFORM_TAG — building from upstream."
    log "This needs cmake + a C++ compiler. Estimated time: 2-5 min."
    confirm "Continue with source build?" || fail "aborted by user"

    for tool in git cmake make; do
      command -v "$tool" >/dev/null 2>&1 \
        || fail "$tool not installed — install it and re-run"
    done

    BUILD_DIR="$(mktemp -d)"
    trap 'rm -rf "$BUILD_DIR"' EXIT
    log "Building in $BUILD_DIR ..."
    git clone --depth=1 https://github.com/ggerganov/whisper.cpp "$BUILD_DIR/whisper.cpp" >/dev/null 2>&1 \
      || fail "git clone failed"
    (
      cd "$BUILD_DIR/whisper.cpp"
      cmake -B build -DGGML_NATIVE=ON >/dev/null
      cmake --build build --config Release -j "$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 2)" >/dev/null
    ) || fail "build failed — check $BUILD_DIR/whisper.cpp/build for logs"

    BUILT_BIN="$BUILD_DIR/whisper.cpp/build/bin/whisper-cli"
    [[ -x "$BUILT_BIN" ]] || fail "expected built binary at $BUILT_BIN, not found"
    install -m 0755 "$BUILT_BIN" "$BINARY_PATH"
    ok "Installed binary → $BINARY_PATH"
  fi
fi

# ── 5. Download model ────────────────────────────────────────────────────────
hdr "5. Downloading $MODEL_NAME (~142 MB) if missing"
if [[ -s "$MODEL_PATH" ]]; then
  ok "Model already present at $MODEL_PATH ($(du -h "$MODEL_PATH" | awk '{print $1}'))"
else
  MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$MODEL_NAME"
  log "Fetching $MODEL_URL"
  curl -fL --retry 3 --progress-bar -o "$MODEL_PATH.partial" "$MODEL_URL" \
    || { rm -f "$MODEL_PATH.partial"; fail "model download failed"; }
  mv "$MODEL_PATH.partial" "$MODEL_PATH"
  ok "Model installed → $MODEL_PATH"
fi

# ── 6. Patch ~/.config/rocky/config.toml ─────────────────────────────────────
hdr "6. Updating $CONFIG_FILE"
TMP_CFG="$(mktemp)"
trap 'rm -f "$TMP_CFG"' EXIT

if [[ -f "$CONFIG_FILE" ]] && grep -q "^\[voice\]" "$CONFIG_FILE"; then
  warn "[voice] section already exists in $CONFIG_FILE — leaving it untouched."
  warn "  If you want to refresh it, delete the section and re-run this script."
else
  if [[ -f "$CONFIG_FILE" ]]; then
    cp "$CONFIG_FILE" "$TMP_CFG"
    printf "\n" >> "$TMP_CFG"
  else
    : > "$TMP_CFG"
  fi
  cat >> "$TMP_CFG" <<EOF
[voice]
provider   = "whisper-cpp"
model      = "$MODEL_PATH"
binary     = "$BINARY_PATH"
silence_ms = 700
tts        = "browser"
EOF
  mv "$TMP_CFG" "$CONFIG_FILE"
  ok "Wrote [voice] block to $CONFIG_FILE"
fi

# ── 7. Sanity-check ──────────────────────────────────────────────────────────
hdr "7. Sanity check"
if "$BINARY_PATH" --help >/dev/null 2>&1; then
  ok "$BINARY_PATH responds to --help"
else
  warn "$BINARY_PATH did not respond cleanly to --help — Rocky may still work, but check the binary."
fi

printf "\n${C_OK}✓ Voice setup complete.${C_RST}\n"
printf "  Try: ${C_HDR}rocky view${C_RST} → click the 🎤 button next to the answer box.\n"
printf "  Or:  ${C_HDR}rocky quiz --voice${C_RST} for a fully hands-free CLI session (planned).\n\n"
