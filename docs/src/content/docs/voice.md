---
title: "Voice (alpha — push-to-talk)"
order: 10
---
The web UI now has a 🎤 button next to the answer textarea. Hold it, speak your answer, release — the transcript drops into the textarea so you can edit before submitting. Default backend is fully local: a `whisper.cpp` subprocess Rocky shells out to. Nothing leaves the machine.

## Setup — one line

```bash
curl -fsSL https://raw.githubusercontent.com/NVME-git/rocky/main/scripts/install-whisper.sh | sh
```

The installer:
- Detects platform (`linux-x64`, `linux-arm64`, `macos-arm64`, `macos-x64`)
- Installs `whisper-cli` — via `brew install whisper-cpp` on macOS, building from source on Linux (`cmake` + a C++ compiler required, ~2-5 min)
- Pulls `ggml-base.en.bin` (~142 MB) into `~/.rocky/models/`
- Adds a `[voice]` block to `~/.config/rocky/config.toml`

After it finishes, restart `rocky view` and the mic button is live.

## Two backends

| Backend | Latency | Privacy | Setup |
|---|---|---|---|
| `whisper-cpp` *(default)* | ~1-2 s on CPU | ✓ fully local | install-whisper.sh |
| `browser` *(opt-in)* | real-time | ✗ Chrome → Google, Safari → Apple | flip a config flag |

Browser mode is **forbidden when `privacy.strict = true`** — Rocky won't let you accidentally exfiltrate audio. Enable it explicitly with:

```toml
[voice]
provider        = "browser"
browser_consent = true
```

## What ships in v0.2

- Web UI mic button (push-to-hold, WAV encoded client-side at 16 kHz mono)
- POST `/api/transcribe` endpoint that calls the configured STT provider
- `whisper.cpp` subprocess invocation with clear errors when the binary or model is missing (the web UI surfaces an inline link to the installer)
- One-line installer (`scripts/install-whisper.sh`)

## What's planned for later

- `rocky quiz --voice` — fully hands-free CLI session: Rocky speaks the question via OS TTS, captures your answer with `cpal` + `webrtc-vad` (700ms silence ends an utterance), evaluates, repeats
- In-binary `whisper-rs` build (`cargo install --features voice`) for users who want one binary, no PATH dependency
- Real-time streaming transcription (Pattern C in [ADR 0006](https://github.com/NVME-git/rocky/blob/main/docs/decisions/0006-voice-architecture.md))

There is **no** wake-word / always-on listening — push-to-talk is the only model. By design.

## Disabling voice

```toml
[voice]
provider = "off"
```

Mic button stops working immediately, no rebuild needed.
