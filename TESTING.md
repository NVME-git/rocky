# Rocky · End-to-End Test Plan

Smoke-test every shipped feature against a fresh checkout. Tick each box as you go.

The current architecture is **agent-driven extraction** via Claude Code skills.
The legacy **Stop-hook → Ollama** path still works as a fallback for users
without Claude Code, but `rocky install claude-all` no longer installs it.

---

## 0 · Prereqs (one-time on this machine)

```bash
rustc --version && cargo --version          # Rust ≥ 1.85
git --version && node --version             # Node ≥ 20
docker --version && docker compose version   # for §10/§11
curl -s http://localhost:11434/api/tags | python3 -m json.tool | grep '"name"'
which arecord                                # Linux CLI voice — optional
```

Back up your real PKG so the test doesn't clobber it:

```bash
cp -r ~/.config/rocky ~/.config/rocky.bak 2>/dev/null
cp -r ~/.rocky ~/.rocky.bak 2>/dev/null
```

- [ ] Rust ≥ 1.85, Node ≥ 20, Docker present
- [ ] Ollama reachable with at least one general model (`llama3.1:8b`, `qwen2.5-coder:7b`, etc.)
- [ ] Backups exist at `~/.config/rocky.bak` and `~/.rocky.bak`

---

## 1 · Build from source

```bash
cd /path/to/rocky
cargo build --release
cargo install --path .              # updates ~/.cargo/bin/rocky
cp ~/.cargo/bin/rocky ~/.local/bin/rocky 2>/dev/null   # if your $PATH points there
```

- [ ] `cargo build --release` finishes without errors
- [ ] `rocky --help` lists the new commands: `add-topic`, `add-question`, `delete-topic`, `context`, `checkpoint`, `due`, `topic`, `review`
- [ ] `cargo test --release` — all **43** tests pass

---

## 2 · Bootstrap a throwaway project

```bash
# Wipe existing test state
rm -rf /tmp/test-project ~/.config/rocky ~/.rocky

# Create a project with a real README so explore has something to read
mkdir /tmp/test-project && cd /tmp/test-project && git init -q

cat > README.md << 'EOF'
# URL Shortener
A lightweight URL shortening service built with Node.js and Express.
Converts long URLs into short base62 codes, stores mappings in SQLite,
and redirects visitors via HTTP 302. Includes rate limiting and URL validation.
## Stack
- Node.js 20 / Express 4 / better-sqlite3 / Jest
EOF

git add README.md && git commit -q -m "init: project scaffold with README"

# Configure Rocky to use Ollama (skip if you have ANTHROPIC_API_KEY set)
mkdir -p ~/.config/rocky
cat > ~/.config/rocky/config.toml << 'EOF'
[llm]
provider = "ollama"
model = "llama3.1:8b"
EOF
```

- [ ] Project at `/tmp/test-project` with README + initial commit
- [ ] `~/.config/rocky/config.toml` written

---

## 3 · Rich-context pipeline (skill-driven, default)

### 3a · The metric model

Rocky ranks topics by **`recall_now = retrievability × mastery`**:

| Term | Meaning |
|---|---|
| `retrievability` (R) | Time decay since last review — pure FSRS |
| `mastery` (M) | Mean of last 3 review scores; default 0.5 if no real reviews yet |
| `recall_now` | What ranking + classification use everywhere |

Classification: `known` ≥ 0.6 · `stale` ≥ 0.3 · `gap` < 0.3.

**Why two factors**: pure R bounces back to 1.0 after any review (correct or
wrong) because "you reviewed it today." Multiplying by M means a wrong answer
(M → 0) drops `recall_now` to ~0 immediately, surfacing the gap. A
freshly-extracted topic with no quiz history starts at M=0.5 → recall ~0.5
("stale") — knowing requires demonstrated mastery, not just freshness.

### 3b · Explore + install

```bash
cd /tmp/test-project
rocky explore                                              # builds project context (~30s on Ollama)
rocky explore --show                                       # verify a multi-paragraph summary

rocky install claude-all                                   # installs: prompt logger + queue git hook + 2 skills
cat .git/hooks/post-commit                                 # full path: /home/.../.local/bin/rocky post-commit
ls ~/.claude/skills/rocky-checkpoint ~/.claude/skills/rocky-quiz   # both SKILL.md files
```

`claude-all` no longer installs the Stop hook (which used to fire `rocky session-end` per turn → Ollama → 60–120 s pause). Skill-driven extraction is on-demand. To opt into the legacy auto-fire behaviour: `rocky install stop`.

- [ ] `rocky explore --show` prints a multi-paragraph project summary
- [ ] `cat .git/hooks/post-commit` shows the full binary path (not bare `rocky`)
- [ ] `~/.claude/skills/rocky-checkpoint/SKILL.md` and `rocky-quiz/SKILL.md` both exist

### 3c · Direct CLI smoke test (no Claude Code needed)

These are the commands the skills call. Verify them first to isolate any
issues from the skill prompts themselves.

```bash
cd /tmp/test-project

# Make a real commit so the queue isn't empty
cat > encode.js << 'EOF'
const CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
function encode(n) {
  let s = '';
  while (n > 0) { s = CHARS[n % 62] + s; n = Math.floor(n / 62); }
  return s || 'A';
}
function decode(s) {
  return [...s].reduce((acc, c) => acc * 62 + CHARS.indexOf(c), 0);
}
module.exports = { encode, decode };
EOF
git add encode.js && git commit -q -m "feat: base62 encoder/decoder for short codes"

# === Skill primitives ===
rocky checkpoint diff                  # JSON: pending commits (this project only)
rocky list --json                      # JSON: existing topics (GLOBAL — across all projects)
rocky context                          # JSON: project summary

rocky add-topic \
  --name "Base62 Encoding" \
  --description "Map an integer to a 62-char alphabet (A-Z a-z 0-9) by repeated mod-62 / floor-divide; produces a compact short code." \
  --domain "Language" --kind "implementation" \
  --commit "$(git rev-parse HEAD)" \
  --context "while (n > 0) { s = CHARS[n % 62] + s; n = Math.floor(n / 62); }"
# Returns: action ("created"|"merged"), retrievability, mastery, recall_now

rocky add-question --topic "Base62" \
  --question "Why use base62 instead of base10?" \
  --answer "Density: a larger alphabet means fewer chars to encode the same integer." \
  --clue "Think about character efficiency"
# Idempotent: re-running with the same question text returns added=false

rocky topic "Base62"                   # JSON: full record incl. question_bank, mastery, recall_now, repos
rocky due --limit 3                    # JSON: weakest first (sorted by recall_now ASC)
rocky review "Base62" --score 0.7 --question "Q" --answer "A" --feedback "F"
# Returns: new_retrievability, single review row recorded

rocky checkpoint mark                  # drain the queue
```

- [ ] All 9 commands return valid JSON without errors
- [ ] `rocky topic` JSON includes `mastery`, `recall_now`, `repos`, `question_bank` fields
- [ ] `rocky list` (no --json) shows a `Recall` column ranging from 0% to 100%
- [ ] `rocky stats` prints a `Weakest:` line naming the lowest-recall topic
- [ ] `rocky review` writes exactly **one** row (not two) — verify with `sqlite3 ~/.rocky/graph.db "SELECT COUNT(*) FROM reviews WHERE node_id='base62-encoding'"`

### 3d · Cross-project linking

Topics dedup across projects. The `repos[]` field accumulates every distinct project a topic has appeared in.

```bash
# Re-add the same topic from a different project — should merge, not duplicate
mkdir -p /tmp/another-project && cd /tmp/another-project && git init -q
rocky add-topic --name "Base62 Encoding" \
  --description "Same insight, second project" \
  --domain "Language" --kind "implementation"
# action: "merged"  → encounter_count went up, no new node

cd /tmp/test-project && rocky topic "Base62" | grep -E '"repos"|"encounter_count"'
# repos: ["another-project"]   (the source project name)
# encounter_count: bumped

rm -rf /tmp/another-project
```

- [ ] Same topic from a 2nd project returns `action: "merged"`
- [ ] `repos[]` accumulates the second project name
- [ ] Total topic count stays the same (no `Base62 Encoding (2)`)

### 3e · Skill flow in a fresh Claude session (manual)

The skills only re-read SKILL.md on Claude Code startup. To test the skills' actual prompts:

```bash
cd /tmp/test-project
# Queue 1-2 more commits so the agent has something to extract
cat > store.js << 'EOF'
const Database = require('better-sqlite3');
const db = new Database('urls.db');
db.exec('CREATE TABLE IF NOT EXISTS urls (code TEXT PRIMARY KEY, url TEXT NOT NULL)');
function save(code, url) {
  db.prepare('INSERT OR REPLACE INTO urls (code, url) VALUES (?, ?)').run(code, url);
}
module.exports = { save };
EOF
git add store.js && git commit -q -m "feat: SQLite-backed mapping store"

claude   # opens a fresh Claude Code session in /tmp/test-project
```

In that session:

1. Type `/rocky-checkpoint`. Watch what the agent does:
   - Calls `rocky checkpoint diff`, `rocky list --json`, `rocky context`
   - Calls `rocky add-topic` per learning, optionally with cross-project dedup
   - Calls `rocky checkpoint mark` at the end
   - Reports: `Checkpointed N commits → M new topics, K merged.`

2. Type `/rocky-quiz`. After the question, the menu must show:
   ```
   [s] simpler   [h] harder   [c] clue   [?] explain it   [x] delete
   or type your answer:
   ```

   Test each:
   - `[s]` → re-asks an easier version, no review row recorded
   - `[h]` → re-asks a deeper version, no review row recorded
   - `[c]` → reveals the clue, re-prompts (next response is graded)
   - `[?]` → reveals the answer + correction, scores **0.0** (admitting gap)
   - `[x]` → confirms then deletes via `rocky delete-topic` (`y` confirms, `n` aborts)
   - free-text → grades and records via `rocky review`

3. Verify a good question gets persisted: after a free-text answer, the
   skill should call `rocky add-question` so the question_bank grows.

- [ ] `/rocky-checkpoint` extracts topics, drains the queue, reports a one-liner
- [ ] `/rocky-quiz` shows the `[s/h/c/?/x]` menu after every question
- [ ] `[?]` reveals the answer and records score 0.0
- [ ] `[x]` then `y` removes the topic; `[x]` then `n` aborts cleanly
- [ ] After a free-text answer, the topic's `question_bank` length increases

---

## 4 · CLI quiz (legacy text path)

The Rust-native quiz still works for non-Claude-Code use:

```bash
cd /tmp/test-project
rocky quiz       # picks weakest topics by recall_now, asks via Ollama
rocky list       # confirm review_count went up on whatever you answered
rocky diff       # quizzes you on the latest commit's changes
```

- [ ] `rocky quiz` lets you answer at least one question; LLM gives feedback
- [ ] Score is recorded — `rocky list` shows higher `Reviews` count
- [ ] `rocky diff` against the latest commit asks code-grounded questions
- [ ] Topics it picks are the ones with lowest `recall_now`, not just lowest R

---

## 5 · Web UI + Rocky IQ

```bash
cd /tmp/test-project && rocky view       # opens browser + prints URL
```

- [ ] URL printed in terminal (e.g. `http://127.0.0.1:NNNNN`)
- [ ] Dashboard tab shows the **Rocky IQ** number and atrophy gauge
  - [ ] Atrophy reflects `recall_now`, not raw R — wrong-answered topics show high atrophy
- [ ] Map tab renders the knowledge graph with coloured nodes (no blank canvas)
- [ ] Click any topic node — detail panel must show:
  - [ ] **Recall (R × M)** label with the percentage and `R N% · M N%` sub-line
  - [ ] **Question Bank** section listing any persisted questions
  - [ ] **Projects** line if the topic has been seen in more than one repo
- [ ] Queue tab lists topics waiting for review (lowest recall_now first)
- [ ] Sessions tab shows per-day topic creation history
- [ ] Projects tab shows the test project + topic counts

API spot-check (the UI consumes this):

```bash
curl -s http://127.0.0.1:NNNNN/api/data | python3 -c "
import sys, json
d = json.load(sys.stdin)
n = next(x for x in d['nodes'] if x['kind'] not in ('domain','user','project_guard'))
print({k: n[k] for k in ['retrievability','mastery','recall_now','classification','question_bank','repos']})
print(f\"dueForReview top: {d['dueForReview'][0]}\")"
```

- [ ] Each node JSON has `retrievability`, `mastery`, `recall_now`, `classification`, `question_bank`, `repos`
- [ ] `dueForReview[0]` has the lowest `recall_now`

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

## 9 · Tutorial script

- [ ] From the repo root, `bash scripts/tutorial.sh` runs end-to-end
- [ ] Completes without unhandled errors
- [ ] Final state has topics, edges, and at least one review recorded

---

## 10 · Docker — demo profile

- [ ] `cd /path/to/rocky`
- [ ] `docker compose -f docker/docker-compose.yml build` (~5–10 min first time)
- [ ] On Linux: bind host Ollama to 0.0.0.0 so the container can reach it:
  ```bash
  pkill ollama
  OLLAMA_HOST=0.0.0.0:11434 ollama serve &
  ```
- [ ] `ROCKY_OLLAMA_MODEL=llama3.1:8b docker compose -f docker/docker-compose.yml --profile demo up`
- [ ] Watches automated walkthrough end-to-end
- [ ] After it finishes: `docker exec -it $(docker ps -q --filter ancestor=rocky:demo) bash`
- [ ] Inside the container: `rocky view` — visit from host browser
- [ ] Voice push-to-talk works through host browser
- [ ] `Ctrl-C` cleans up

---

## 11 · Docker — dev profile (real workflow)

- [ ] `ROCKY_WORKSPACE=/abs/path/to/repo docker compose -f docker/docker-compose.yml --profile dev run --rm rocky-dev`
- [ ] Banner shows Ollama "reachable"
- [ ] `rocky explore && rocky install claude-all`
- [ ] Make a commit on the host — hook fires, queues the SHA
- [ ] Back in container: `rocky checkpoint diff` shows the queued commit (or `rocky session-end` for the legacy path)
- [ ] `rocky view` from inside, browse from host
- [ ] Exit and re-enter — knowledge graph persists in `rocky-data` volume
- [ ] `docker compose -f docker/docker-compose.yml down -v` wipes the volume cleanly

---

## 12 · VS Code extension

- [ ] `cd extensions/vscode && npm install && npm run compile`
- [ ] Open `extensions/vscode` in VS Code, press **F5**
- [ ] Extension Development Host launches with Rocky icon in the activity bar
- [ ] Without `rocky.serverUrl`: status bar shows `Rocky: N topics` (reads `~/.rocky/pkg/pkg.json`)
  - [ ] Run `rocky export` first to populate it
- [ ] Set `rocky.serverUrl` to a running `rocky view` (e.g. `http://127.0.0.1:NNNNN`)
- [ ] Status bar switches to `Rocky IQ: NN` with colour-coded background
- [ ] Sidebar tabs: **Due for Review**, **All Topics**, **Sessions**, **Summary** all populate
  - [ ] "Due" uses the new `recall_now` ordering (wrong-answered topics first)
- [ ] Click any topic → topic detail webview opens with retrievability bar + reviews
- [ ] Hover any matching word in an editor → tooltip shows topic info
- [ ] `Rocky: Open Knowledge Graph` renders the force-directed graph
- [ ] `Rocky: Quiz Weakest Topics` opens the quiz panel; assess buttons work
- [ ] After 30 s, status bar IQ value refreshes automatically

---

## 13 · Browser extension

- [ ] `cd extensions/browser`
- [ ] `chrome://extensions` → Developer mode → Load unpacked → select `extensions/browser/`
- [ ] Open the popup → connection dot is grey ("not connected")
- [ ] Settings tab: paste `rocky view` URL → Save → Sync → reports `✓ Synced N topics`
- [ ] Header IQ pill appears, colour-coded
- [ ] Toolbar badge shows the same IQ number
- [ ] Capture tab: visit any tech article → URL/title pre-filled, suggested topics appear
- [ ] Select topics → Add → status confirms; entry shows up in History
- [ ] Right-click on a page → "Add page to Rocky PKG" → reopen popup, capture is queued
- [ ] Captures stay local: `chrome.storage.local.get('captureHistory')` in DevTools

---

## 14 · Restore your real config

```bash
rm -rf ~/.config/rocky ~/.rocky
mv ~/.config/rocky.bak ~/.config/rocky 2>/dev/null
mv ~/.rocky.bak ~/.rocky 2>/dev/null
rm -rf /tmp/test-project /tmp/another-project
rocky uninstall stop 2>/dev/null   # in case anything was opted in
```

- [ ] Real PKG restored
- [ ] `rocky stats` shows your actual topics
- [ ] Test project + Ollama config wiped

---

## When something fails

| Symptom | Likely cause | Fix |
|---|---|---|
| `error: failed to compile … edition2024` | Rust < 1.85 | `rustup update stable` |
| `whisper-cli: libwhisper.so.1 not found` | whisper.cpp built with shared libs | Rebuild with `-DBUILD_SHARED_LIBS=OFF` |
| Container can't reach `host.docker.internal:11434` (Linux) | Ollama bound to 127.0.0.1 | `OLLAMA_HOST=0.0.0.0:11434 ollama serve` *or* `--network host` |
| VS Code status bar still shows old "N topics" text | `rocky.serverUrl` not set | Settings → `rocky.serverUrl` → web UI URL |
| Browser badge empty after upgrade | Sync hasn't run since the badge UX changed | Popup → Settings → Sync |
| `rocky quiz --voice` exits with "no recorder" | `arecord`/`rec` missing | `apt install alsa-utils` (Linux) or `brew install sox` (macOS) |
| `rocky checkpoint diff` is empty after a commit | post-commit hook didn't fire | `cat .git/hooks/post-commit` — must show full binary path; reinstall via `rocky install claude-all` |
| Skill prompt looks stale | Claude Code only reads SKILL.md on session start | Quit + reopen `claude` |
