/// Rocky — the AI teacher.
/// Calls the Anthropic API (or Ollama) for topic extraction, question generation,
/// and answer evaluation.
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::node::Node;

#[derive(Debug, Deserialize, Clone)]
pub struct TopicInfo {
    pub topic: String,
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct EvalResult {
    pub score: f64,
    pub understood: bool,
    pub feedback: String,
    pub followup: Option<String>,
}

pub struct Teacher {
    provider: Provider,
}

enum Provider {
    Claude { api_key: String, model: String, client: reqwest::blocking::Client },
    Ollama { base_url: String, model: String, client: reqwest::blocking::Client },
}

impl Teacher {
    pub fn claude(api_key: String, model: String) -> Self {
        Self {
            provider: Provider::Claude {
                api_key,
                model,
                client: reqwest::blocking::Client::new(),
            },
        }
    }

    pub fn ollama(base_url: String, model: String) -> Self {
        Self {
            provider: Provider::Ollama {
                base_url,
                model,
                client: reqwest::blocking::Client::new(),
            },
        }
    }

    fn ask(&self, system: &str, user: &str) -> Result<String> {
        match &self.provider {
            Provider::Claude { api_key, model, client } => {
                let resp = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&json!({
                        "model": model,
                        "max_tokens": 1024,
                        "system": system,
                        "messages": [{"role": "user", "content": user}]
                    }))
                    .send()?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    return Err(anyhow!("Anthropic API error {status}: {body}"));
                }

                let data: Value = resp.json()?;
                Ok(data["content"][0]["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("no text in Anthropic response"))?
                    .to_string())
            }

            Provider::Ollama { base_url, model, client } => {
                let prompt = format!("System: {system}\n\nUser: {user}");
                let resp = client
                    .post(format!("{base_url}/api/generate"))
                    .json(&json!({
                        "model": model,
                        "prompt": prompt,
                        "stream": false
                    }))
                    .send()?;

                let data: Value = resp.json()?;
                Ok(data["response"]
                    .as_str()
                    .ok_or_else(|| anyhow!("no response from Ollama"))?
                    .to_string())
            }
        }
    }

    pub fn extract_topics_from_diff(&self, commit_msg: &str, diff: &str) -> Result<Vec<TopicInfo>> {
        let system = r#"You are a technical knowledge analyst reviewing a git diff.
Extract the distinct technical topics a developer needs to genuinely understand based on what changed in this diff.

Focus on:
- Libraries, frameworks, or APIs introduced or heavily used
- Patterns or architectural decisions visible in the code (e.g. retry logic, caching strategy, auth flow)
- Non-obvious implementation details that could cause bugs if misunderstood
- Any tricky language features or data structures used

Ignore:
- Trivial changes (renaming, formatting, comments)
- Boilerplate that requires no understanding

Return ONLY valid JSON: a list of objects with keys:
- "topic": concise topic name (2-5 words)
- "kind": one of "concept", "pattern", "implementation"
- "description": one sentence explaining what this topic is and why it matters

Return 2-6 topics maximum."#;

        let user = format!("Commit: {commit_msg}\n\nDiff:\n{diff}");
        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        Ok(serde_json::from_str(cleaned)?)
    }

    pub fn extract_topics(&self, task: &str) -> Result<Vec<TopicInfo>> {
        let system = r#"You are a technical knowledge analyst. Given a task description,
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
]"#;

        let raw = self.ask(system, &format!("Task: {task}"))?;
        let cleaned = strip_code_fence(&raw);
        Ok(serde_json::from_str(cleaned)?)
    }

    pub fn generate_question(
        &self,
        topic: &str,
        description: &str,
        context: &str,
        known_topics: &[String],
        question_num: u32,
    ) -> Result<String> {
        let system = r#"You are a Socratic technical mentor. Your job is to generate ONE question
that forces a developer to reason about the IMPLICATIONS and CONSEQUENCES of a technical topic,
not just recall facts.

Rules:
- Ask about how the topic affects other things they've built or will build
- Ask about what breaks, changes, or becomes constrained when using this approach
- Ask about trade-offs and when NOT to use this approach
- Do NOT ask "what is X" or "define X" — assume basic awareness
- The question should be specific to their task context
- Keep it to 1-2 sentences
- Return ONLY the question, no preamble"#;

        let known_str = if known_topics.is_empty() {
            "none yet".to_string()
        } else {
            known_topics.join(", ")
        };

        let user = format!(
            "Topic: {topic}\nDescription: {description}\nTask context: {context}\nDeveloper's known topics: {known_str}\nQuestion number: {question_num} (vary difficulty/angle if > 1)"
        );

        self.ask(system, &user)
    }

    pub fn evaluate_answer(
        &self,
        topic: &str,
        question: &str,
        answer: &str,
        description: &str,
    ) -> Result<EvalResult> {
        let system = r#"You are evaluating whether a developer genuinely understands the implications
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
}"#;

        let user = format!(
            "Topic: {topic}\nDescription: {description}\nQuestion asked: {question}\nDeveloper's answer: {answer}"
        );

        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        Ok(serde_json::from_str(cleaned)?)
    }

    pub fn generate_explanation(
        &self,
        topic: &str,
        description: &str,
        question: &str,
        answer: &str,
        context: &str,
    ) -> Result<String> {
        let system = r#"You are a senior engineer explaining a concept to a junior developer in a pair programming session.
They just answered a question incorrectly or got stuck. Give them the real answer.

Your explanation should:
1. Start with the core insight — the thing they're missing
2. Show concretely how it applies to their specific task
3. Give one mental model or rule of thumb they can remember
4. Flag the most common mistake people make with this

Tone: direct, warm, practical. Like explaining something over a coffee. No jargon without explanation.
Length: 4-6 sentences. No bullet points — write it as natural speech.
Do NOT start with "Great question" or any filler."#;

        let user = format!(
            "Topic: {topic}\nDescription: {description}\nTask context: {context}\nQuestion asked: {question}\nTheir answer: {answer}"
        );

        self.ask(system, &user)
    }

    pub fn generate_reminder(&self, topic: &str, node: &Node, context: &str) -> Result<String> {
        let system = r#"You are a Socratic technical mentor. Given a topic a developer learned before
but hasn't revisited recently, write a 2-3 sentence reminder that:
1. Refreshes the core idea
2. Connects it to their current task
3. Flags one thing worth double-checking given their current context

Be concise. No fluff."#;

        let today = chrono::Local::now().date_naive();
        let days_since = (today - node.last_reviewed).num_days();
        let contexts_str = node.contexts[..node.contexts.len().min(3)].join(", ");

        let user = format!(
            "Topic: {topic}\nWhat they learned: {}\nPrevious contexts: {contexts_str}\nCurrent task context: {context}\nDays since reviewed: {days_since}",
            node.description
        );

        self.ask(system, &user)
    }
}

fn strip_code_fence(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with("```") {
        let after = &s[3..];
        let after = after.strip_prefix("json").unwrap_or(after);
        let after = after.trim_start_matches('\n');
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
        return after.trim();
    }
    s
}
