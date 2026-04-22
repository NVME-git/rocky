/// Voice input (STT) for Rocky. Decisions captured in
/// docs/decisions/0006-voice-architecture.md.
///
/// Surfaces:
///   - Web UI: POST /api/transcribe uploads a WAV, Stt::transcribe returns text
///   - CLI:    `rocky quiz --voice` — arecord/sox mic capture + whisper-cli STT + system TTS
use anyhow::{anyhow, bail, Result};

use crate::config::{Config, VoiceConfig};

pub trait Stt: std::fmt::Debug + Send + Sync {
    /// Transcribe a WAV byte buffer (expected: PCM 16 kHz mono, Whisper's native input).
    /// Callers should convert browser-produced Opus/WebM to WAV before calling.
    fn transcribe(&self, wav_bytes: &[u8]) -> Result<String>;
}

/// Shells out to the whisper.cpp `whisper-cli` binary. No native deps — works
/// with a user-installed whisper.cpp. Default provider.
#[derive(Debug)]
pub struct WhisperCppSubprocess {
    pub binary: String,
    pub model: String,
}

impl Stt for WhisperCppSubprocess {
    fn transcribe(&self, wav_bytes: &[u8]) -> Result<String> {
        if wav_bytes.is_empty() {
            bail!("empty audio buffer");
        }

        let model_path = expand_home(&self.model);
        if !std::path::Path::new(&model_path).exists() {
            bail!(
                "whisper model not found at {} — run scripts/install-whisper.sh \
                 to download the default ggml-base.en.bin",
                model_path
            );
        }

        // tempfile auto-cleans on drop. Use one tempdir for both the input WAV
        // and the .txt output whisper-cli writes alongside it.
        let dir = tempfile::tempdir().map_err(|e| anyhow!("tempdir: {e}"))?;
        let wav_path = dir.path().join("audio.wav");
        let out_stem = dir.path().join("audio");
        std::fs::write(&wav_path, wav_bytes).map_err(|e| anyhow!("write wav: {e}"))?;

        let output = std::process::Command::new(&self.binary)
            .arg("-m").arg(&model_path)
            .arg("-of").arg(&out_stem)
            .arg("-otxt")
            .arg("-nt")           // no timestamps in output
            .arg("-l").arg("en")
            .arg(&wav_path)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow!(
                        "whisper-cli binary `{}` not found in PATH. \
                         Install with scripts/install-whisper.sh, or set voice.binary in your config.",
                        self.binary
                    )
                } else {
                    anyhow!("failed to spawn whisper-cli: {e}")
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "whisper-cli failed (exit {:?}): {}",
                output.status.code(),
                stderr.lines().last().unwrap_or("(no stderr)")
            );
        }

        let txt_path = out_stem.with_extension("txt");
        let transcript = std::fs::read_to_string(&txt_path).map_err(|e| {
            anyhow!("whisper-cli succeeded but produced no .txt at {}: {e}", txt_path.display())
        })?;
        Ok(transcript.trim().to_string())
    }
}

pub(crate) fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

// ── CLI voice session ─────────────────────────────────────────────────────────

/// CLI voice session wrapping STT + TTS + mic recording.
/// Used by `rocky quiz --voice`.
pub struct CliVoice {
    pub stt: Box<dyn Stt>,
}

impl CliVoice {
    pub fn new(stt: Box<dyn Stt>) -> Self {
        Self { stt }
    }

    /// Speak `text` aloud using the platform TTS binary (non-blocking best-effort).
    pub fn speak(&self, text: &str) {
        speak_text(text);
    }

    /// Record from mic until the user presses Enter, then transcribe.
    pub fn record_and_transcribe(&self) -> Result<String> {
        let wav = record_until_enter()?;
        self.stt.transcribe(&wav)
    }
}

/// Speak text using the platform TTS binary (`say` on macOS, `espeak-ng`/`spd-say` on Linux).
/// Strips simple Markdown punctuation so prose sounds natural.
pub fn speak_text(text: &str) {
    let clean: String = text
        .chars()
        .filter(|c| !matches!(*c, '*' | '_' | '`' | '#' | '[' | ']'))
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        return;
    }

    if cfg!(target_os = "macos") {
        std::process::Command::new("say")
            .arg(clean)
            .status()
            .ok();
    } else {
        // Try espeak-ng first, fall back to spd-say
        let ok = std::process::Command::new("espeak-ng")
            .args(["-s", "150", clean])
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            std::process::Command::new("spd-say")
                .arg(clean)
                .stderr(std::process::Stdio::null())
                .status()
                .ok();
        }
    }
}

/// Check whether a mic recorder binary is available on this system.
pub fn recorder_available() -> bool {
    let cmd = if cfg!(target_os = "macos") { "rec" } else { "arecord" };
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Record audio from the default microphone at 16 kHz mono until the user presses
/// Enter. Returns raw WAV bytes suitable for `Stt::transcribe`.
///
/// Uses `arecord` on Linux and `rec` (SoX) on macOS — no native Rust audio deps.
pub fn record_until_enter() -> Result<Vec<u8>> {
    let dir = tempfile::tempdir().map_err(|e| anyhow!("tempdir: {e}"))?;
    let wav_path = dir.path().join("cli_recording.wav");
    let wav_path_bg = wav_path.clone();

    let (tx, rx) = std::sync::mpsc::channel::<()>();

    let recorder = std::thread::spawn(move || {
        let mut cmd = if cfg!(target_os = "macos") {
            let mut c = std::process::Command::new("rec");
            c.args([
                "-q", "-r", "16000", "-c", "1",
                wav_path_bg.to_str().unwrap_or("audio.wav"),
            ]);
            c
        } else {
            let mut c = std::process::Command::new("arecord");
            c.args([
                "-q", "-f", "S16_LE", "-r", "16000", "-c", "1",
                wav_path_bg.to_str().unwrap_or("audio.wav"),
            ]);
            c
        };
        let mut child = match cmd.stderr(std::process::Stdio::null()).spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("   recorder failed to start: {e}");
                return;
            }
        };
        rx.recv().ok(); // block until stop signal
        child.kill().ok();
        child.wait().ok();
    });

    // Main thread waits for user to press Enter
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;

    tx.send(()).ok();
    recorder.join().ok();

    // Give OS a moment to flush file buffers
    std::thread::sleep(std::time::Duration::from_millis(150));

    let bytes = std::fs::read(&wav_path)
        .map_err(|e| anyhow!("could not read recording: {e}"))?;

    if bytes.len() < 100 {
        bail!("recording too short — is a microphone available?");
    }
    Ok(bytes)
}

/// Browser-side STT placeholder. The actual Web Speech API call happens in JS;
/// this server-side type is only used for config validation and the refusal
/// check against privacy.strict.
#[derive(Debug)]
pub struct BrowserStt;

impl Stt for BrowserStt {
    fn transcribe(&self, _wav_bytes: &[u8]) -> Result<String> {
        Err(anyhow!("browser STT runs client-side; server should not call transcribe()"))
    }
}

/// Construct the configured STT provider, honouring privacy.strict and the
/// off-by-default posture. Returns `None` for `provider = "off"`.
pub fn make_stt(cfg: &Config) -> Result<Option<Box<dyn Stt>>> {
    let vc: &VoiceConfig = &cfg.voice;
    match vc.provider.as_str() {
        "off" => Ok(None),
        "browser" => {
            if cfg.privacy_strict {
                bail!(
                    "voice.provider = \"browser\" is forbidden when privacy.strict = true \
                     (browsers send audio to a cloud STT). Use \"whisper-cpp\" instead."
                );
            }
            if !vc.browser_consent {
                bail!(
                    "voice.provider = \"browser\" requires explicit consent. Set \
                     voice.browser_consent = true in your config, or click the mic \
                     button in the web UI once to accept the consent prompt."
                );
            }
            Ok(Some(Box::new(BrowserStt)))
        }
        "whisper-cpp" => Ok(Some(Box::new(WhisperCppSubprocess {
            binary: vc.binary.clone(),
            model: vc.model.clone(),
        }))),
        "whisper-rs" => {
            bail!(
                "voice.provider = \"whisper-rs\" requires `cargo install --features voice`. \
                 See docs/decisions/0006-voice-architecture.md."
            )
        }
        other => bail!("unknown voice.provider \"{other}\" — expected off|whisper-cpp|browser|whisper-rs"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn default_config_has_voice_off() {
        let cfg = Config::default();
        let stt = make_stt(&cfg).unwrap();
        assert!(stt.is_none());
    }

    #[test]
    fn browser_requires_consent() {
        let mut cfg = Config::default();
        cfg.voice.provider = "browser".into();
        let err = make_stt(&cfg).unwrap_err();
        assert!(err.to_string().contains("consent"));
    }

    #[test]
    fn browser_forbidden_under_strict_privacy() {
        let mut cfg = Config::default();
        cfg.voice.provider = "browser".into();
        cfg.voice.browser_consent = true;
        cfg.privacy_strict = true;
        let err = make_stt(&cfg).unwrap_err();
        assert!(err.to_string().contains("privacy.strict"));
    }

    #[test]
    fn whisper_cpp_constructs() {
        let mut cfg = Config::default();
        cfg.voice.provider = "whisper-cpp".into();
        let stt = make_stt(&cfg).unwrap();
        assert!(stt.is_some());
    }

    #[test]
    fn whisper_cpp_errors_on_missing_model() {
        let stt = WhisperCppSubprocess {
            binary: "whisper-cli".into(),
            model: "/definitely/not/here.bin".into(),
        };
        let err = stt.transcribe(&[1, 2, 3, 4]).unwrap_err().to_string();
        assert!(err.contains("whisper model not found"), "got: {err}");
    }

    #[test]
    fn whisper_cpp_errors_on_empty_audio() {
        let stt = WhisperCppSubprocess {
            binary: "whisper-cli".into(),
            model: "/tmp/nonexistent.bin".into(),
        };
        let err = stt.transcribe(&[]).unwrap_err().to_string();
        assert!(err.contains("empty audio"), "got: {err}");
    }

    #[test]
    fn expand_home_replaces_tilde() {
        let h = dirs::home_dir().unwrap();
        let s = expand_home("~/foo/bar.bin");
        assert!(s.starts_with(&*h.to_string_lossy()), "got: {s}");
        assert!(s.ends_with("/foo/bar.bin"), "got: {s}");
    }

    #[test]
    fn expand_home_passthrough_for_absolute() {
        assert_eq!(expand_home("/etc/x"), "/etc/x");
    }
}
