# Rocky · End-to-End Test Plan

Smoke-test every shipped feature against a fresh checkout. Tick each box as you go.

## 0 · Prereqs (one-time on this machine)

Run these to verify everything is in place before starting:

```bash
# Rust — must be ≥ 1.85 (Rocky uses edition 2021 + recent crate deps)
rustc --version && cargo --version

# Core toolchain
git --version && node --version && docker --version && docker compose version

# Ollama — must be reachable and have at least one general model
curl -s http://localhost:11434/api/tags | python3 -m json.tool | grep '"name"'
```

Expected output (versions may differ, but all must be present):
```
rustc 1.93.1  (or newer)
cargo 1.93.1
git version 2.x
v22.x  (Node ≥ 20)
Docker version 29.x
Docker Compose version 5.x
"name": "llama3.1:8b"  (or llama3.2 / qwen2.5-coder:7b)
```

If Ollama isn't running:
```bash
ollama serve &
ollama pull llama3.1:8b   # or qwen2.5-coder:7b
```

- [ ] `rustc --version` reports ≥ 1.85
- [ ] `node --version` reports ≥ 20
- [ ] `docker` and `docker compose` are present
- [ ] Ollama is reachable and has a general model pulled
- [ ] (Optional) `arecord` (Linux) or `rec`/`sox` (macOS) for CLI voice — `which arecord`
- [ ] Back up your real config so the test doesn't clobber it:
  ```bash
  cp -r ~/.config/rocky ~/.config/rocky.bak 2>/dev/null
  cp -r ~/.rocky ~/.rocky.bak 2>/dev/null
  ```

---

## 1 · Build from source

You can either clone a fresh copy or build from your working copy — both are equivalent.

**Option A — fresh clone:**
```bash
git clone https://github.com/NVME-git/rocky.git ~/rocky-test && cd ~/rocky-test
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

**Option B — working copy (recommended if already developing):**
```bash
cd /path/to/rocky
cargo build --release
cargo install --path .   # updates ~/.cargo/bin/rocky to today's build
```

If `which rocky` resolves to a different location (e.g. `~/.local/bin/rocky`), replace it:
```bash
cp ~/.cargo/bin/rocky ~/.local/bin/rocky
```

- [ ] `cargo build --release` — finishes without errors
- [ ] `rocky --help` prints the command list
- [ ] `cargo test --release` — all tests pass (40 expected)

---

## 2 · Bootstrap a throwaway project

Create the project with a real README so `rocky explore` produces a useful context summary.
A bare `package.json` gives Rocky nothing to work with.

```bash
# Wipe any existing test state first
rm -rf /tmp/test-project
rm -rf ~/.config/rocky ~/.rocky

# Create project
mkdir /tmp/test-project && cd /tmp/test-project
git init

cat > README.md << 'EOF'
# URL Shortener
A lightweight URL shortening service built with Node.js and Express.
Converts long URLs into short base62 codes, stores mappings in SQLite,
and redirects visitors via HTTP 302. Includes rate limiting and URL validation.
## Stack
- Node.js 20 / Express 4 / better-sqlite3 / Jest
EOF

git add README.md
git commit -m "init: project scaffold with README"
```

Configure Rocky to use Ollama (skip if using Claude with API key set):
```bash
mkdir -p ~/.config/rocky
cat > ~/.config/rocky/config.toml << 'EOF'
[llm]
provider = "ollama"
model = "llama3.1:8b"
EOF
```

- [ ] Project directory created with a meaningful README
- [ ] `~/.config/rocky` written with correct provider

---

## 3 · Rich-context pipeline (explore → post-commit → session-end)

**Explore** reads docs + recent commits and synthesises a project summary stored in the DB.
This summary grounds all future quiz questions in the actual codebase.

```bash
cd /tmp/test-project
rocky explore          # builds and stores context
rocky explore --show   # verify the summary was generated
```

**Install hooks** — use `claude-all` to wire both the git post-commit hook (silent queue)
and the Claude Code Stop hook (batch process at session end):

```bash
rocky install claude-all
cat .git/hooks/post-commit   # should show full path: /home/.../.local/bin/rocky post-commit
```

> **Note:** The hook must use the full binary path, not just `rocky`, so it works inside
> Claude Code's subprocess environment where PATH is stripped.
> This is fixed as of commit cbb5b12 — re-run `rocky install claude-all` if upgrading from an older binary.

**Workflow A — Solo dev commit (immediate queue):**

```bash
echo "const RETRY_LIMIT = 3;" >> config.js
git add config.js
git commit -m "feat: add RETRY_LIMIT constant"
# hook fires silently — nothing printed
rocky session-end    # processes queue, extracts topics, adds to PKG
rocky stats          # should show 1-2 new topics
```

**Workflow B — Agent session:**

```bash
# Open Claude Code in this directory
claude
# Ask Claude to make 1-2 commits (e.g. "Add store.js and validate.js, commit each separately")
# The Stop hook fires after EACH agent turn (not only on /exit) — this is Claude Code's design.
# Rocky processes the queue each time Claude finishes responding.
rocky stats          # topics visible after Claude's first completed turn
```

> **Stop hook behaviour:** Claude Code's Stop hook fires when the agent finishes each response
> and returns control to the user — not only when you type `/exit`. This means `session-end`
> runs after every prompt you send to Claude, processing whatever commits are queued at that
> moment. With a local LLM (Ollama) this causes a noticeable pause between prompts while
> topic extraction runs. With a fast model (Anthropic API or `claude-code` provider) this
> is imperceptible.

> **Known limitation:** If you run many Claude sessions in the same project within a few
> hours, session-end loads all transcripts and the combined context may exceed the LLM's
> context window. Fixed in commit cbb5b12 (num_ctx=8192). If it still happens, run:
> `rocky session-end --hours 1`

- [ ] `rocky explore --show` prints a multi-paragraph project summary (not "No context stored")
- [ ] `cat .git/hooks/post-commit` shows the full binary path
- [ ] Solo commit → `rocky session-end` → `rocky stats` shows new topics
- [ ] Agent session — after Claude's first completed turn, `rocky stats` shows new topics
- [ ] `rocky list` lists topics with retrievability colours

---

## 4 · Quiz (text)

- [ ] `rocky quiz` lets you answer at least one question; LLM gives feedback
- [ ] Score is recorded — re-run `rocky list` and confirm review_count went up
- [ ] `rocky diff` against the latest commit asks code-grounded questions

---

## 5 · Web UI + Rocky IQ

- [ ] `rocky view` opens at `http://127.0.0.1:<port>` and prints the URL
- [ ] Dashboard tab shows the **Rocky IQ** number, atrophy gauge, due/recent lists
- [ ] Map tab renders the knowledge graph with coloured nodes (no blank canvas)
- [ ] Queue tab lists topics waiting for review
- [ ] Sessions tab shows per-day topic creation history
- [ ] Projects tab shows the test project + topic counts

---

## 6 · Voice — web UI push-to-talk

- [ ] `bash scripts/install-whisper.sh` succeeds (whisper-cli + base.en model)
- [ ] In `rocky view`, the 🎤 button appears in the answer box
- [ ] Hold the button, speak a sentence, release
- [ ] Transcript appears in the textarea (no audio leaves the machine)
- [ ] Submit the answer — quiz flow continues normally

---

## 7 · Voice — CLI mode

- [ ] `which arecord` (Linux) or `which rec` (macOS) returns a path
- [ ] `rocky quiz --voice` reads the question aloud via system TTS
- [ ] Press Enter on an empty input → recording starts
- [ ] Press Enter again → recording stops, "Heard: …" preview appears
- [ ] Confirm or override → answer is evaluated

---

## 8 · Dedupe

- [ ] `rocky dedupe --dry-run` finds candidate near-duplicate pairs (or exits cleanly if none)
- [ ] `rocky dedupe` shows pairs with `[a]`/`[b]`/`[m]`/`[s]`/`[q]` options
- [ ] `[m]` calls the LLM, suggests a canonical name + description, lets you accept or override
- [ ] After merging, the surviving node has the merged contexts and review history

---

## 9 · Tutorial script (smoke-test everything in one shot)

- [ ] From `~/rocky-test`, run `bash scripts/tutorial.sh` end-to-end
- [ ] Completes without unhandled errors
- [ ] Final state has topics, edges, and at least one review recorded

---

## 10 · Docker — demo profile

- [ ] `cd ~/rocky-test`
- [ ] `docker compose -f docker/docker-compose.yml build` (~5–10 min first time)
- [ ] On Linux: ensure host Ollama is bound to 0.0.0.0 so the container can reach it:
  ```bash
  pkill ollama
  OLLAMA_HOST=0.0.0.0:11434 ollama serve &
  ```
  (or use `network_mode: host` — see `docker/README.md`)
- [ ] `ROCKY_OLLAMA_MODEL=llama3.1:8b docker compose -f docker/docker-compose.yml --profile demo up`
- [ ] Watches automated walkthrough: stage commits → explore → install → backfill → session-end → stats
- [ ] After the demo finishes, `docker exec -it $(docker ps -q --filter ancestor=rocky:demo) bash`
- [ ] Inside the container: `rocky view` — visit `http://localhost:7777` from the host browser
- [ ] Web UI loads; voice push-to-talk works through the host browser
- [ ] `Ctrl-C` outside cleans up

---

## 11 · Docker — dev profile (real workflow)

- [ ] Mount your own repo:
  ```bash
  ROCKY_WORKSPACE=/abs/path/to/repo \
    docker compose -f docker/docker-compose.yml --profile dev run --rm rocky-dev
  ```
- [ ] Banner shows Ollama "reachable"
- [ ] `rocky explore && rocky install claude-all`
- [ ] Make a commit on the host (in the same repo) — hook fires, queues the SHA
- [ ] Back in container: `rocky session-end` drains the queue
- [ ] `rocky view` from inside, browse from host at `http://localhost:7777`
- [ ] Exit and re-enter — knowledge graph persists in the `rocky-data` volume
- [ ] `docker compose -f docker/docker-compose.yml down -v` wipes the volume cleanly

---

## 12 · VS Code extension

- [ ] `cd extensions/vscode && npm install && npm run compile`
- [ ] Open the `extensions/vscode` folder in VS Code, press **F5**
- [ ] Extension Development Host launches with Rocky icon in the activity bar
- [ ] Without `rocky.serverUrl`: status bar shows `Rocky: N topics` (reads pkg.json)
  - [ ] `rocky export` first to populate `~/.rocky/pkg/pkg.json`
- [ ] Set `rocky.serverUrl` to a running `rocky view` (e.g. `http://127.0.0.1:7777`)
- [ ] Status bar switches to `Rocky IQ: NN` with colour-coded background
- [ ] Sidebar tabs: **Due for Review**, **All Topics**, **Sessions**, **Summary** all populate
- [ ] Sessions tab shows per-day topic groups (requires server)
- [ ] Click any topic → topic detail webview opens with retrievability bar + reviews
- [ ] Hover any matching word in an editor → tooltip shows topic info
- [ ] `Rocky: Open Knowledge Graph` command renders the force-directed graph
- [ ] `Rocky: Quiz Weakest Topics` opens the quiz panel; assess buttons work
- [ ] After 30 s, status bar IQ value refreshes automatically

---

## 13 · Browser extension

- [ ] `cd extensions/browser` (no build step — `.js` is committed)
- [ ] Open `chrome://extensions`, enable **Developer mode**, **Load unpacked** → select the `extensions/browser/` directory
- [ ] Open the popup → connection dot is grey ("not connected")
- [ ] Settings tab: paste `http://127.0.0.1:7777` (your `rocky view` URL) → **Save** → **Sync**
- [ ] Sync reports `✓ Synced N topics`
- [ ] Header IQ pill appears, colour-coded (green ≥80, amber ≥70, red <70)
- [ ] Toolbar badge shows the same IQ number
- [ ] Capture tab: visit any tech article, click the extension → URL/title pre-filled, suggested topics appear based on page keywords
- [ ] Select a few topics → **Add** → status confirms; the entry shows up in the History tab
- [ ] Right-click on a page → "Add page to Rocky PKG" → reopen popup, capture is queued
- [ ] Verify captures stay local: `chrome.storage.local.get('captureHistory')` in DevTools

---

## 14 · Restore your real config

- [ ] `rm -rf ~/.config/rocky ~/.rocky`
- [ ] `mv ~/.config/rocky.bak ~/.config/rocky 2>/dev/null`
- [ ] `mv ~/.rocky.bak ~/.rocky 2>/dev/null`
- [ ] `rm -rf /tmp/test-project ~/rocky-test`

---

## When something fails

| Symptom | Likely cause | Fix |
|---|---|---|
| `error: failed to compile … edition2024` | Rust < 1.85 | `rustup update stable` |
| `whisper-cli: libwhisper.so.1 not found` | Built whisper.cpp with shared libs | Rebuild with `-DBUILD_SHARED_LIBS=OFF` (Docker image already does this) |
| Container can't reach `host.docker.internal:11434` (Linux) | Ollama bound to 127.0.0.1 | `OLLAMA_HOST=0.0.0.0:11434 ollama serve` *or* `--network host` |
| VS Code status bar still shows old "N topics" text | Server URL not set | Settings → `rocky.serverUrl` → `http://127.0.0.1:7777` |
| Browser badge empty after upgrade | Sync hasn't run since the badge UX changed | Open popup → Settings → **Sync** |
| `rocky quiz --voice` exits with "no recorder" | `arecord`/`rec` not on PATH | `apt install alsa-utils` (Linux) or `brew install sox` (macOS) |
