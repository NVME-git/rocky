/// Rocky configuration.
///
/// Loads from (in order, later overrides earlier):
///   1. Built-in defaults
///   2. ~/.config/rocky/config.toml  (global user config)
///   3. ./.rocky.toml                (project-level override)
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

/// How long before a cross-concept edge can fire again.
#[derive(Debug, Clone)]
pub enum EdgeReuse {
    /// Always fire the edge if a qualifying one exists.
    Off,
    /// Only reuse an edge after N days.
    Days(u32),
    /// Only reuse an edge after N quiz events.
    Sessions(u32),
}

impl EdgeReuse {
    /// Parse "off", "14d", "5s" etc.
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("off") {
            return Some(Self::Off);
        }
        if let Some(n) = s.strip_suffix('d').or_else(|| s.strip_suffix('D')) {
            return n.parse::<u32>().ok().map(Self::Days);
        }
        if let Some(n) = s.strip_suffix('s').or_else(|| s.strip_suffix('S')) {
            return n.parse::<u32>().ok().map(Self::Sessions);
        }
        None
    }
}

#[derive(Debug, Deserialize, Default)]
struct TomlFile {
    llm: Option<LlmSection>,
    session: Option<SessionSection>,
    export: Option<ExportSection>,
    ui: Option<UiSection>,
    sync: Option<SyncSection>,
    edges: Option<EdgesSection>,
    privacy: Option<PrivacySection>,
    voice: Option<VoiceSection>,
    promptiq: Option<PromptIqSection>,
}

#[derive(Debug, Deserialize)]
struct PromptIqSection {
    enabled: Option<bool>,
    /// "off" | "immediate" | "silent"
    feedback: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrivacySection {
    /// When true, refuse to send code/diffs to non-local LLM providers.
    /// Forces ollama. Errors clearly if provider="claude" is configured.
    strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VoiceSection {
    /// "whisper-cpp" (default), "whisper-rs" (requires --features voice), "browser", "off"
    provider: Option<String>,
    /// Path to the GGML model file, e.g. ~/.rocky/models/whisper-base.en.bin
    model: Option<String>,
    /// Name or absolute path of the whisper-cpp binary. Defaults to "whisper-cli" on PATH.
    binary: Option<String>,
    /// Milliseconds of silence that ends an utterance in CLI hands-free mode.
    silence_ms: Option<u32>,
    /// "browser" (web speechSynthesis), "system" (say/espeak-ng), "off"
    tts: Option<String>,
    /// Whether the user has acknowledged that provider="browser" sends audio to a cloud STT.
    /// Set automatically by the web UI on first click of the mic button.
    browser_consent: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct EdgesSection {
    reuse: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlmSection {
    provider: Option<String>,
    model: Option<String>,
    ollama_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionSection {
    daily_budget: Option<u32>,
    min_gap_minutes: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ExportSection {
    pkg_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UiSection {
    personality: Option<bool>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SyncSection {
    enabled: Option<bool>,
    auto_commit: Option<bool>,
    commit_visible: Option<bool>,
    remind_push_sessions: Option<u32>,
    remind_push_days: Option<u32>,
    remote: Option<String>,
    branch: Option<String>,
}

/// Sync configuration — public, passed to sync module.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub enabled: bool,
    pub auto_commit: bool,
    pub commit_visible: bool,
    pub remind_push_sessions: u32,
    pub remind_push_days: u32,
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub llm_provider: String,
    pub llm_model: String,
    pub ollama_base_url: String,
    pub daily_budget: u32,
    pub min_gap_minutes: u32,
    pub pkg_dir: PathBuf,
    pub db_path: PathBuf,
    pub rocky_dir: PathBuf,
    pub personality: bool,
    pub user_name: String,
    pub sync: SyncConfig,
    pub edge_reuse: EdgeReuse,
    /// When true: refuse to send code to remote LLMs. Local-only mode.
    pub privacy_strict: bool,
    pub voice: VoiceConfig,
    pub promptiq: PromptIqConfig,
}

#[derive(Debug, Clone)]
pub struct PromptIqConfig {
    pub enabled: bool,
    /// "off" | "immediate" | "silent"
    pub feedback: String,
}

impl Default for PromptIqConfig {
    fn default() -> Self {
        Self { enabled: true, feedback: "silent".into() }
    }
}

#[derive(Debug, Clone)]
pub struct VoiceConfig {
    pub provider: String,         // "whisper-cpp" | "whisper-rs" | "browser" | "off"
    pub model: String,            // path to ggml model
    pub binary: String,           // whisper-cpp binary name or path
    pub silence_ms: u32,          // CLI VAD silence threshold
    pub tts: String,              // "browser" | "system" | "off"
    pub browser_consent: bool,    // explicit opt-in for cloud STT
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            provider: "off".into(),
            model: "~/.rocky/models/whisper-base.en.bin".into(),
            binary: "whisper-cli".into(),
            silence_ms: 700,
            tts: "browser".into(),
            browser_consent: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let rocky_dir = dirs();
        Self {
            llm_provider: "claude".into(),
            llm_model: "claude-sonnet-4-6".into(),
            ollama_base_url: "http://localhost:11434".into(),
            daily_budget: 3,
            min_gap_minutes: 120,
            pkg_dir: rocky_dir.join("pkg"),
            db_path: rocky_dir.join("graph.db"),
            rocky_dir: rocky_dir.clone(),
            personality: true,
            user_name: git_user_name(),
            edge_reuse: EdgeReuse::Off,
            sync: SyncConfig {
                enabled: false,
                auto_commit: true,
                commit_visible: true,
                remind_push_sessions: 5,
                remind_push_days: 0,
                remote: "origin".into(),
                branch: "main".into(),
            },
            privacy_strict: false,
            voice: VoiceConfig::default(),
            promptiq: PromptIqConfig::default(),
        }
    }
}

/// Data directory: $ROCKY_HOME, falling back to ~/.rocky/ (graph.db, pkg/, summaries/).
/// Honouring ROCKY_HOME makes the tutorial / smoke-test script trivially isolatable.
fn dirs() -> PathBuf {
    if let Ok(custom) = std::env::var("ROCKY_HOME") {
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rocky")
}

/// User feedback file: ~/.rocky/FEEDBACK.md (or $ROCKY_HOME/FEEDBACK.md).
pub fn feedback_path() -> PathBuf {
    dirs().join("FEEDBACK.md")
}

/// Config directory: ~/.config/rocky/
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".config"))
        .join("rocky")
}

/// One-time migration: ~/.rocky/.rocky.toml → ~/.config/rocky/config.toml.
/// Silent no-op when the new path already exists or the legacy file is absent.
fn migrate_legacy_config_path() {
    let new_path = config_dir().join("config.toml");
    if new_path.exists() {
        return;
    }
    let legacy = dirs().join(".rocky.toml");
    if !legacy.exists() {
        return;
    }
    if let Some(parent) = new_path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    if std::fs::rename(&legacy, &new_path).is_ok() {
        eprintln!(
            "rocky: migrated config {} → {}",
            legacy.display(),
            new_path.display()
        );
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        migrate_legacy_config_path();
        let mut cfg = Self::default();
        let global = config_dir().join("config.toml");
        let local = PathBuf::from(".rocky.toml");
        for path in [global, local] {
            if path.exists() {
                let text = std::fs::read_to_string(&path)?;
                let file: TomlFile = toml::from_str(&text)?;
                cfg.apply(file);
            }
        }
        Ok(cfg)
    }

    fn apply(&mut self, file: TomlFile) {
        if let Some(llm) = file.llm {
            if let Some(v) = llm.provider { self.llm_provider = v; }
            if let Some(v) = llm.model { self.llm_model = v; }
            if let Some(v) = llm.ollama_base_url { self.ollama_base_url = v; }
        }
        if let Some(s) = file.session {
            if let Some(v) = s.daily_budget { self.daily_budget = v; }
            if let Some(v) = s.min_gap_minutes { self.min_gap_minutes = v; }
        }
        if let Some(e) = file.export {
            if let Some(v) = e.pkg_dir {
                let expanded = v.replacen("~/", &format!("{}/", dirs::home_dir()
                    .unwrap_or_default().display()), 1);
                self.pkg_dir = PathBuf::from(expanded);
            }
        }
        if let Some(ui) = file.ui {
            if let Some(v) = ui.personality { self.personality = v; }
            if let Some(v) = ui.name { self.user_name = v; }
        }
        if let Some(e) = file.edges {
            if let Some(v) = e.reuse {
                if let Some(r) = EdgeReuse::parse(&v) {
                    self.edge_reuse = r;
                }
            }
        }
        if let Some(s) = file.sync {
            if let Some(v) = s.enabled { self.sync.enabled = v; }
            if let Some(v) = s.auto_commit { self.sync.auto_commit = v; }
            if let Some(v) = s.commit_visible { self.sync.commit_visible = v; }
            if let Some(v) = s.remind_push_sessions { self.sync.remind_push_sessions = v; }
            if let Some(v) = s.remind_push_days { self.sync.remind_push_days = v; }
            if let Some(v) = s.remote { self.sync.remote = v; }
            if let Some(v) = s.branch { self.sync.branch = v; }
        }
        if let Some(p) = file.privacy {
            if let Some(v) = p.strict { self.privacy_strict = v; }
        }
        if let Some(v) = file.voice {
            if let Some(x) = v.provider { self.voice.provider = x; }
            if let Some(x) = v.model {
                let expanded = x.replacen("~/", &format!("{}/", dirs::home_dir()
                    .unwrap_or_default().display()), 1);
                self.voice.model = expanded;
            }
            if let Some(x) = v.binary { self.voice.binary = x; }
            if let Some(x) = v.silence_ms { self.voice.silence_ms = x; }
            if let Some(x) = v.tts { self.voice.tts = x; }
            if let Some(x) = v.browser_consent { self.voice.browser_consent = x; }
        }
        if let Some(p) = file.promptiq {
            if let Some(v) = p.enabled { self.promptiq.enabled = v; }
            if let Some(v) = p.feedback { self.promptiq.feedback = v; }
        }
    }

    pub fn show(&self) {
        use colored::Colorize;
        println!("\n{}", " Active config".bold());
        println!("{}", " ─────────────────────────────".dimmed());
        println!("\n  [llm]");
        println!("    provider         = {}", self.llm_provider);
        println!("    model            = {}", self.llm_model);
        if self.llm_provider == "ollama" {
            println!("    ollama_base_url  = {}", self.ollama_base_url);
        }
        println!("\n  [session]");
        println!("    daily_budget     = {}", self.daily_budget);
        println!("    min_gap_minutes  = {}", self.min_gap_minutes);
        println!("\n  [export]");
        println!("    pkg_dir          = {}", self.pkg_dir.display());
        println!("\n  [edges]");
        let reuse_str = match &self.edge_reuse {
            EdgeReuse::Off => "off (always use edge if available)".to_string(),
            EdgeReuse::Days(n) => format!("{n}d"),
            EdgeReuse::Sessions(n) => format!("{n}s"),
        };
        println!("    reuse            = {reuse_str}");
        println!("\n  [privacy]");
        println!("    strict           = {}  {}",
            self.privacy_strict,
            if self.privacy_strict {
                "(local-only — diffs/code never leave this machine)"
            } else {
                "(diffs are sent to the configured llm provider)"
            }.dimmed()
        );
        println!("\n  [voice]");
        println!("    provider         = {}", self.voice.provider);
        if self.voice.provider != "off" {
            println!("    model            = {}", self.voice.model);
            println!("    binary           = {}", self.voice.binary);
            println!("    silence_ms       = {}", self.voice.silence_ms);
            println!("    tts              = {}", self.voice.tts);
            if self.voice.provider == "browser" {
                println!("    browser_consent  = {}", self.voice.browser_consent);
            }
        }
        println!("\n  [ui]");
        println!("    personality      = {}", self.personality);
        println!("\n  [sync]");
        println!("    enabled          = {}", self.sync.enabled);
        if self.sync.enabled {
            println!("    auto_commit      = {}", self.sync.auto_commit);
            println!("    commit_visible   = {}", self.sync.commit_visible);
            println!("    remote           = {}", self.sync.remote);
            println!("    branch           = {}", self.sync.branch);
            if self.sync.remind_push_sessions > 0 {
                println!("    remind_push      = every {} sessions", self.sync.remind_push_sessions);
            } else if self.sync.remind_push_days > 0 {
                println!("    remind_push      = every {} days", self.sync.remind_push_days);
            } else {
                println!("    remind_push      = off");
            }
        }
        println!();
    }
}

fn git_user_name() -> String {
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Me".to_string())
}
