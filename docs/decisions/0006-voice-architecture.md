# 0006 — Voice architecture (planned for v0.2)

**Status:** Proposed (2026-04-22)
**Decision date:** Decisions locked; build pending in next release.

## Context

Voice in Rocky has obvious value once the rich-context pipeline is producing real questions: typing a multi-sentence answer to *"What would change if Redis were not available for storing refresh tokens?"* is high-friction, especially for the morning-coffee or walking-the-dog review session that Rocky is otherwise ideal for.

Voice opens up two distinct surfaces:
- **Pattern A — Push-to-talk in the web UI.** Click a 🎤 button next to the answer textarea, speak, transcript appears.
- **Pattern B — CLI hands-free mode.** `rocky quiz --voice` reads the question aloud, listens for an answer, evaluates, repeats. End-to-end no-keyboard.
- *Pattern C — real-time conversational* — out of scope. Adds streaming partials, interruption handling, multi-turn state. Defer to v0.3+.

Across STT/TTS providers there is a sharp split between *zero-setup but cloud* (Web Speech API → Google) and *local but install-required* (whisper.cpp / Piper).

## Decision

| Axis | Choice |
|---|---|
| **Patterns at launch** | A + B (share backend) |
| **STT default** | `whisper.cpp` subprocess with `ggml-base.en.bin` (142MB, ~7× realtime on CPU) |
| **STT browser opt-in** | Allowed via `provider = "browser"` with explicit consent modal. **Refused when `privacy.strict = true`.** |
| **TTS at launch** | Browser `speechSynthesis` only. Piper is a future config option. |
| **Build distribution** | `cargo install --features voice` (off by default). Default Rocky binary stays small. |
| **Always-on / wake-word** | Never. Push-to-talk only. |
| **Platforms at launch** | macOS + Linux. Windows whisper.cpp install deferred to v0.3. |

**Architecture sketch:**

```
src/voice.rs           — pub trait Stt { fn transcribe(&self, wav: &[u8]) -> Result<String> }
                         impls: WhisperCppSubprocess (default), WhisperRs (#[cfg(feature="voice")])
src/server.rs          — POST /api/transcribe (multipart audio in, JSON {transcript} out)
src/main.rs            — rocky quiz --voice: cpal mic capture + webrtc-vad + STT + TTS loop
src/app.html           — 🎤 button + MediaRecorder + fetch /api/transcribe
src/config.rs          — [voice] { provider, model, binary, silence_ms, tts }
scripts/install-whisper.sh — curl-pipeable installer: detects platform, grabs prebuilt
                              from ggerganov/whisper.cpp releases, drops binary in
                              ~/.local/bin and model in ~/.rocky/models
```

**Default config after one-line install:**
```toml
[voice]
provider = "whisper-cpp"
model = "~/.rocky/models/whisper-base.en.bin"
binary = "whisper-cli"
silence_ms = 700              # how long of a pause ends an utterance
tts = "browser"               # web UI uses speechSynthesis; CLI also exposes "system" + "off"
```

**Failure UX:**
- `whisper-cli` not found → `/api/transcribe` returns 503 with body pointing at the install script. The mic button surfaces this inline.
- TTS unavailable on Linux without `espeak-ng` → falls back to printing the question, doesn't kill the session.
- `provider = "browser"` with `privacy.strict = true` → CLI refuses at startup; web UI hides the mic button entirely.

## Consequences

**Positive:**
- Default Rocky install is unchanged in size and compile time. Voice is purely opt-in.
- One curl-pipeable install line bridges the gap to Tier-2 (local whisper.cpp). Tier-3 (`--features voice`, in-binary) exists for users who want one-binary deploys.
- Privacy-strict mode covers voice the same way it covers diffs — there is no path to send audio off-machine that the strict flag doesn't block.

**Negative / costs:**
- Subprocess STT adds per-utterance latency from process start (~50-150ms) on top of model inference. Acceptable for a quiz answer; not for streaming.
- The `cpal` + `webrtc-vad` dependency chain pulls in some C bindings — guarded behind `--features voice`.
- Three install tiers (browser opt-in / one-line install / cargo feature) increases doc complexity. Mitigated by a dedicated **Voice setup** page in the Flutter docs.

**Verification gate (before merging the v0.2 voice branch):**
- `rocky quiz --voice` completes a 3-question session end-to-end on macOS and Linux with the default `base.en` model.
- Web UI mic button transcribes a 5-second utterance to within ~80% word-accuracy on a technical sentence (e.g. *"the Redis instance stores refresh tokens with a seven-day TTL"*).
- `privacy.strict = true` causes both `--voice` and the web mic button to error/hide with a clear message.
