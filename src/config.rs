/// Rocky configuration.
///
/// Loads from (in order, later overrides earlier):
///   1. Built-in defaults
///   2. ~/.rocky/.rocky.toml  (global user config)
///   3. ./.rocky.toml         (project-level override)
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
struct TomlFile {
    llm: Option<LlmSection>,
    session: Option<SessionSection>,
    export: Option<ExportSection>,
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
    obsidian_vault: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub llm_provider: String,
    pub llm_model: String,
    pub ollama_base_url: String,
    pub daily_budget: u32,
    pub min_gap_minutes: u32,
    pub obsidian_vault: PathBuf,
    pub db_path: PathBuf,
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
            obsidian_vault: rocky_dir.join("vault"),
            db_path: rocky_dir.join("graph.db"),
        }
    }
}

fn dirs() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rocky")
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut cfg = Self::default();
        let global = dirs().join(".rocky.toml");
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
            if let Some(v) = e.obsidian_vault {
                let expanded = v.replacen("~/", &format!("{}/", dirs::home_dir()
                    .unwrap_or_default().display()), 1);
                self.obsidian_vault = PathBuf::from(expanded);
            }
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
        println!("    obsidian_vault   = {}", self.obsidian_vault.display());
        println!();
    }
}
