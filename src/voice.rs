/// Voice input (STT) for Rocky. Decisions captured in
/// docs/decisions/0006-voice-architecture.md.
///
/// Wired surfaces (planned for v0.2):
///   - Web UI: POST /api/transcribe uploads a WAV, Stt::transcribe returns text
///   - CLI:    `rocky quiz --voice` captures audio via `cpal`, runs silence
///             detection via `webrtc-vad`, then calls Stt::transcribe
///
/// Current status: **skeleton only.** `WhisperCppSubprocess::transcribe` returns
/// an error until the v0.2 build wires up the actual binary invocation. The
/// route exists so the UI can be stubbed against a real endpoint shape.
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
    fn transcribe(&self, _wav_bytes: &[u8]) -> Result<String> {
        // v0.2 implementation plan (not yet wired):
        //   1. Write wav_bytes to a tempfile (tempfile crate).
        //   2. Spawn: whisper-cli -m <model> -of <out_stem> -otxt -nt <wav_path>
        //   3. Read <out_stem>.txt, strip trailing whitespace, return.
        //   4. On spawn failure (ENOENT), return a specific error so the web UI
        //      can surface the install-script link.
        bail!("voice STT not wired yet — see docs/decisions/0006-voice-architecture.md")
    }
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
}
