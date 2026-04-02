"""
Rocky configuration.

Loads from (in order, later overrides earlier):
  1. Built-in defaults
  2. ~/.rocky/.rocky.toml  (global user config)
  3. ./.rocky.toml         (project-level override)

Example ~/.rocky/.rocky.toml:

    [llm]
    provider = "ollama"
    model = "qwen2.5:14b"
    ollama_base_url = "http://localhost:11434"

    [session]
    daily_budget = 5
    min_gap_minutes = 60

    [export]
    obsidian_vault = "~/Documents/Obsidian/MyVault/rocky"
"""

import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from rocky.llm.base import LLMProvider

_GLOBAL_CONFIG = Path.home() / ".rocky" / ".rocky.toml"
_LOCAL_CONFIG = Path(".rocky.toml")


@dataclass
class Config:
    # LLM
    llm_provider: str = "claude"
    llm_model: str = "claude-sonnet-4-6"
    ollama_base_url: str = "http://localhost:11434"

    # Session
    daily_budget: int = 3
    min_gap_minutes: int = 120

    # Export
    obsidian_vault: str = str(Path.home() / ".rocky" / "vault")

    @classmethod
    def load(cls) -> "Config":
        cfg = cls()
        for config_path in [_GLOBAL_CONFIG, _LOCAL_CONFIG]:
            if config_path.exists():
                with open(config_path, "rb") as f:
                    data = tomllib.load(f)
                cfg._apply(data)
        return cfg

    def _apply(self, data: dict):
        llm = data.get("llm", {})
        if "provider" in llm:
            self.llm_provider = llm["provider"]
        if "model" in llm:
            self.llm_model = llm["model"]
        if "ollama_base_url" in llm:
            self.ollama_base_url = llm["ollama_base_url"]

        session = data.get("session", {})
        if "daily_budget" in session:
            self.daily_budget = int(session["daily_budget"])
        if "min_gap_minutes" in session:
            self.min_gap_minutes = int(session["min_gap_minutes"])

        export = data.get("export", {})
        if "obsidian_vault" in export:
            self.obsidian_vault = str(Path(export["obsidian_vault"]).expanduser())

    def create_provider(self) -> LLMProvider:
        if self.llm_provider == "ollama":
            from rocky.llm.ollama import OllamaProvider
            return OllamaProvider(self.ollama_base_url, self.llm_model)
        from rocky.llm.claude import ClaudeProvider
        return ClaudeProvider(self.llm_model)

    def show(self):
        """Print active config to stdout."""
        from rocky.cli import color
        print(color("\n Active config", "bold"))
        print(color(" ─────────────────────────────", "dim"))
        print(f"\n  [llm]")
        print(f"    provider         = {self.llm_provider}")
        print(f"    model            = {self.llm_model}")
        if self.llm_provider == "ollama":
            print(f"    ollama_base_url  = {self.ollama_base_url}")
        print(f"\n  [session]")
        print(f"    daily_budget     = {self.daily_budget}")
        print(f"    min_gap_minutes  = {self.min_gap_minutes}")
        print(f"\n  [export]")
        print(f"    obsidian_vault   = {self.obsidian_vault}")

        sources = []
        if _GLOBAL_CONFIG.exists():
            sources.append(str(_GLOBAL_CONFIG))
        if _LOCAL_CONFIG.exists():
            sources.append(str(_LOCAL_CONFIG))
        if sources:
            print(color(f"\n  Loaded from: {', '.join(sources)}", "dim"))
        else:
            print(color(f"\n  Using defaults (no .rocky.toml found)", "dim"))
        print()
