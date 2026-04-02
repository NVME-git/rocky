"""LLM provider protocol — any backend Rocky uses must implement this."""

from typing import Protocol, runtime_checkable


@runtime_checkable
class LLMProvider(Protocol):
    def complete(self, system: str, user: str) -> str:
        """Send a system + user message, return the assistant's text response."""
        ...
