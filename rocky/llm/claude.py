"""Anthropic Claude backend for Rocky."""

import os
import anthropic

DEFAULT_MODEL = "claude-sonnet-4-6"


class ClaudeProvider:
    def __init__(self, model: str = DEFAULT_MODEL):
        self.model = model
        self._client: anthropic.Anthropic | None = None

    def _get_client(self) -> anthropic.Anthropic:
        if self._client is None:
            api_key = os.environ.get("ANTHROPIC_API_KEY")
            if not api_key:
                raise RuntimeError(
                    "ANTHROPIC_API_KEY not set. Add it to your .env file or environment."
                )
            self._client = anthropic.Anthropic(api_key=api_key)
        return self._client

    def complete(self, system: str, user: str) -> str:
        client = self._get_client()
        response = client.messages.create(
            model=self.model,
            max_tokens=1024,
            system=system,
            messages=[{"role": "user", "content": user}],
        )
        return response.content[0].text.strip()
