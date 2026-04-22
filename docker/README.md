# Rocky in Docker

A self-contained Rocky environment for showcasing the demo and for using
Rocky against any repo without installing the toolchain on your host.

## What's in the image

- `rocky` (release build of this repo)
- `whisper-cli` + `ggml-base.en.bin` (~142 MB) for local STT
- Node.js 20, git, sqlite3, jq (handy for the toy project + general use)
- `/opt/toy-project` — a tiny TypeScript URL shortener used by the demo
- Two entrypoints: `entrypoint-demo` (automated walkthrough) and `entrypoint-dev` (interactive shell)

Final image is ~1.2 GB.

---

## Prerequisites

Rocky talks to an LLM. The Docker image is wired to use **Ollama on the host**.

1. Install Ollama on your host and start it:
   ```bash
   ollama serve            # in one terminal
   ollama pull llama3.2    # in another
   ```
2. Verify it works: `curl http://localhost:11434/api/tags`

You can swap the model with `ROCKY_OLLAMA_MODEL=mistral` (and `ollama pull mistral`).

> **Linux gotcha**: by default Ollama binds to `127.0.0.1:11434`, which is
> *not* reachable from inside the container even with `host-gateway`. Either:
>
> 1. Start Ollama bound to all interfaces:
>    `OLLAMA_HOST=0.0.0.0:11434 ollama serve`
> 2. Or run the container with `--network host` and set
>    `ROCKY_OLLAMA_HOST=localhost:11434`.
>
> macOS/Windows Docker Desktop handles this transparently.

---

## Build the image

From the repo root:

```bash
docker compose -f docker/docker-compose.yml build
```

First build takes ~5–10 min (Rust release build + whisper.cpp compile + base.en model download). Subsequent builds reuse the cargo dep cache layer.

---

## Demo profile — automated lifecycle walkthrough

Best for screen recordings and quick "what does Rocky do" showcases.

```bash
docker compose -f docker/docker-compose.yml --profile demo up
```

What happens:

1. Sanity-checks Ollama is reachable on the host.
2. Stages `urlshort` into `/workspace` and creates a realistic 5-commit history.
3. Runs `rocky explore --show` (project context summary).
4. Runs `rocky install` (installs the post-commit hook).
5. Runs `rocky backfill --limit 5` (extracts concepts from the staged commits).
6. Runs `rocky session-end` (drains the queue if anything's there).
7. Prints `rocky stats` and `rocky list` so you can see what landed.
8. Sleeps so you can `exec` in and try the web UI:
   ```bash
   docker exec -it $(docker ps -q --filter ancestor=rocky:demo) bash
   rocky view              # bound to 0.0.0.0:7777 inside the container
   # then open http://localhost:7777 in your host browser
   ```

The container is ephemeral — re-running `up` gives you a clean graph.

Tune the pacing with `DEMO_PAUSE=0` (instant) or `DEMO_PAUSE=5` (slow walkthrough).

---

## Dev profile — interactive shell against any repo

Best for real daily use or for demoing Rocky against a real codebase.

```bash
# Use the bundled toy project (default)
docker compose -f docker/docker-compose.yml --profile dev run --rm rocky-dev

# Use your own repo
ROCKY_WORKSPACE=/abs/path/to/your/repo \
    docker compose -f docker/docker-compose.yml --profile dev run --rm rocky-dev
```

You land in a bash shell with:

- `/workspace` mounted to the repo you chose
- `rocky` on `PATH`
- Config at `~/.config/rocky/config.toml` (templated to point at host Ollama)
- Knowledge graph in `~/.rocky/` — **persisted across runs** in the `rocky-data` named volume
- Port `7777` forwarded to the host

Suggested first commands:

```bash
rocky explore               # build project context
rocky install               # post-commit hook
rocky install claude-all    # full Claude-aware setup (post-commit + Stop hook)
# ... do some commits ...
rocky session-end           # generate questions
rocky quiz                  # take the quiz
rocky view                  # open http://localhost:7777 in your host browser
```

To wipe the persistent graph and start fresh:

```bash
docker compose -f docker/docker-compose.yml down -v
```

---

## Voice — what works and what doesn't

| Mode | Works in Docker? | How |
|------|------------------|-----|
| Web UI push-to-talk (`rocky view` 🎤 button) | ✅ | Browser captures the mic on the host, posts WAV to `/api/transcribe`, container runs `whisper-cli` |
| CLI voice (`rocky quiz --voice`) | ⚠️ Off by default | Needs `--device /dev/snd --group-add audio` and ALSA configured. Not enabled in compose. |
| TTS read-aloud | ❌ | No audio output device by default |

For a voice demo, use the web UI.

---

## Configuration overrides

Set these in your shell before running compose:

| Env var | Default | What |
|---------|---------|------|
| `ROCKY_OLLAMA_HOST` | `host.docker.internal:11434` | Ollama endpoint reachable from the container |
| `ROCKY_OLLAMA_MODEL` | `llama3.2` | Model name (must be `ollama pull`'d on the host) |
| `ROCKY_WORKSPACE` (dev only) | `./toy-project` | Absolute path to the repo to mount at `/workspace` |
| `DEMO_PAUSE` (demo only) | `2` | Seconds between demo stages |

To use a cloud LLM instead of Ollama, edit `docker/config-template.toml` and pass `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` via `environment:` in `docker-compose.yml`.

---

## Troubleshooting

**"Ollama at host.docker.internal:11434 ... unreachable"**
- Make sure `ollama serve` is running on the host.
- On Linux, double-check `extra_hosts: host.docker.internal:host-gateway` is honoured (Docker 20.10+).
- Test from inside the container: `curl http://host.docker.internal:11434/api/tags`.

**"model 'llama3.2' not found"**
- Run `ollama pull llama3.2` on the host.

**Web UI returns 502 from `/api/transcribe`**
- Check `whisper-cli` is on PATH inside the container: `which whisper-cli`.
- Confirm the model file exists: `ls /opt/whisper-models/ggml-base.en.bin`.

**Permission errors on mounted `/workspace`**
- The container runs as UID 1000. If your host user is a different UID, either run as root with `--user 0` or chown your workspace dir.
