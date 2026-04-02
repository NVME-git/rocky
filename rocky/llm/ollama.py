"""
Ollama backend for Rocky.

Calls Ollama's OpenAI-compatible chat endpoint using urllib (no extra dependencies).
Requires Ollama running locally: https://ollama.com

Recommended models for Rocky:
  - qwen2.5:14b   — best quality/size trade-off for question gen + answer eval
  - llama3.1:8b   — lighter, adequate for question generation
  - phi3.5        — smallest, use only for question generation

Note: 8B models produce acceptable questions but unreliable answer evaluation.
14B+ closes most of the quality gap vs Claude Sonnet.
"""

import json
import urllib.error
import urllib.request

DEFAULT_BASE_URL = "http://localhost:11434"
DEFAULT_MODEL = "qwen2.5:14b"


class OllamaProvider:
    def __init__(self, base_url: str = DEFAULT_BASE_URL, model: str = DEFAULT_MODEL):
        self.base_url = base_url.rstrip("/")
        self.model = model

    def complete(self, system: str, user: str) -> str:
        payload = json.dumps({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            "stream": False,
        }).encode()

        req = urllib.request.Request(
            f"{self.base_url}/api/chat",
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read())
                return data["message"]["content"].strip()
        except urllib.error.URLError as e:
            raise RuntimeError(
                f"Cannot reach Ollama at {self.base_url}. "
                f"Is it running? Try: ollama serve\n{e}"
            )
        except (KeyError, json.JSONDecodeError) as e:
            raise RuntimeError(f"Unexpected response from Ollama: {e}")
