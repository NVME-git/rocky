"""
Rocky — the AI teacher.
Claude-powered engine: topic extraction, question generation, answer evaluation.
"""

import os
import anthropic

MODEL = "claude-sonnet-4-6"

_client: anthropic.Anthropic | None = None


def _get_client() -> anthropic.Anthropic:
    global _client
    if _client is None:
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError(
                "ANTHROPIC_API_KEY not set. Add it to your .env file or environment."
            )
        _client = anthropic.Anthropic(api_key=api_key)
    return _client


def _ask(system: str, user: str) -> str:
    client = _get_client()
    response = client.messages.create(
        model=MODEL,
        max_tokens=1024,
        system=system,
        messages=[{"role": "user", "content": user}],
    )
    return response.content[0].text.strip()


def extract_topics(task_description: str) -> list[dict]:
    """
    Extract the key technical topics from a task description.
    Returns list of {topic, kind, description}.
    """
    system = """You are a technical knowledge analyst. Given a task description,
extract the distinct technical topics a developer needs to understand to complete it.

Return ONLY valid JSON: a list of objects with keys:
- "topic": concise topic name (2-5 words)
- "kind": one of "concept", "pattern", "implementation"
- "description": one sentence explaining what this topic is

Focus on non-trivial topics where understanding gaps could cause problems.
Skip obvious boilerplate. Return 2-5 topics maximum.

Example output:
[
  {"topic": "JWT authentication", "kind": "pattern", "description": "Stateless token-based auth where the server signs a payload the client stores and sends back."},
  {"topic": "token expiry handling", "kind": "implementation", "description": "How to detect, communicate, and refresh expired tokens in an API."}
]"""

    result = _ask(system, f"Task: {task_description}")
    result = result.strip()
    if result.startswith("```"):
        result = result.split("```")[1]
        if result.startswith("json"):
            result = result[4:]
    import json
    return json.loads(result)


def generate_question(topic: str, topic_description: str, context: str,
                      known_topics: list[str], question_num: int = 1) -> str:
    """
    Generate a Socratic, implication-focused question about a topic.
    Avoids factual recall — focuses on consequences and connections.
    """
    known_str = ", ".join(known_topics) if known_topics else "none yet"
    system = """You are a Socratic technical mentor. Your job is to generate ONE question
that forces a developer to reason about the IMPLICATIONS and CONSEQUENCES of a technical topic,
not just recall facts.

Rules:
- Ask about how the topic affects other things they've built or will build
- Ask about what breaks, changes, or becomes constrained when using this approach
- Ask about trade-offs and when NOT to use this approach
- Do NOT ask "what is X" or "define X" — assume basic awareness
- The question should be specific to their task context
- Keep it to 1-2 sentences
- Return ONLY the question, no preamble"""

    user = f"""Topic: {topic}
Description: {topic_description}
Task context: {context}
Developer's known topics: {known_str}
Question number: {question_num} (vary difficulty/angle if > 1)"""

    return _ask(system, user)


def evaluate_answer(topic: str, question: str, answer: str,
                    topic_description: str) -> dict:
    """
    Evaluate a developer's answer. Returns:
    {score: 0.0-1.0, feedback: str, understood: bool, followup: str|None}
    """
    system = """You are evaluating whether a developer genuinely understands the implications
of a technical topic based on their answer to a Socratic question.

Evaluate on:
1. Do they demonstrate understanding of consequences, not just surface knowledge?
2. Do they show awareness of how this affects related systems?
3. Is there evidence they could reason through related problems?

Return ONLY valid JSON:
{
  "score": <0.0 to 1.0>,
  "understood": <true if score >= 0.65>,
  "feedback": "<1-2 sentences of specific feedback>",
  "followup": "<a follow-up question if score < 0.65, else null>"
}"""

    user = f"""Topic: {topic}
Description: {topic_description}
Question asked: {question}
Developer's answer: {answer}"""

    import json
    result = _ask(system, user).strip()
    if result.startswith("```"):
        result = result.split("```")[1]
        if result.startswith("json"):
            result = result[4:]
    return json.loads(result)


def generate_reminder(topic: str, node: dict, context: str) -> str:
    """Generate a brief reminder for a stale topic."""
    system = """You are a Socratic technical mentor. Given a topic a developer learned before
but hasn't revisited recently, write a 2-3 sentence reminder that:
1. Refreshes the core idea
2. Connects it to their current task
3. Flags one thing worth double-checking given their current context

Be concise. No fluff."""

    user = f"""Topic: {topic}
What they learned: {node.get('description', 'No description saved')}
Previous contexts: {', '.join(node.get('contexts', [])[:3])}
Current task context: {context}
Days since reviewed: calculated from {node.get('last_reviewed', 'unknown')}"""

    return _ask(system, user)
