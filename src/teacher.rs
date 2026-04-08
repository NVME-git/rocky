/// Rocky — the AI teacher.
/// Calls the Anthropic API (or Ollama) for topic extraction, question generation,
/// and answer evaluation.
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::node::Node;

#[derive(Debug, Deserialize, Clone)]
pub struct GeneratedEdge {
    pub target: String,
    pub kind: String,
    pub description: String,
    pub strength: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TopicInfo {
    pub topic: String,
    pub kind: String,
    pub domain: String,
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
- "domain": one of "Language", "Database", "Auth", "API", "Frontend", "DevOps", "Architecture", "Performance", "Security", "Testing", "Tooling", "Data", "Other"
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
- "domain": one of "Language", "Database", "Auth", "API", "Frontend", "DevOps", "Architecture", "Performance", "Security", "Testing", "Tooling", "Data", "Other"
- "description": one sentence explaining what this topic is

Focus on non-trivial topics where understanding gaps could cause problems.
Skip obvious boilerplate. Return 2-5 topics maximum.

Example output:
[
  {"topic": "JWT authentication", "kind": "pattern", "domain": "Auth", "description": "Stateless token-based auth where the server signs a payload the client stores and sends back."},
  {"topic": "token expiry handling", "kind": "implementation", "domain": "Auth", "description": "How to detect, communicate, and refresh expired tokens in an API."}
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
        canonical_answer: Option<&str>,
    ) -> Result<EvalResult> {
        let system = r#"You are evaluating whether a developer genuinely understands the implications
of a technical topic based on their answer to a Socratic question.

Evaluate on:
1. Do they demonstrate understanding of consequences, not just surface knowledge?
2. Do they show awareness of how this affects related systems?
3. Is there evidence they could reason through related problems?

If an ideal answer is provided, use it as a reference for what a complete answer looks like —
but do not penalise for different phrasing or approach, only for missing key insights.

Return ONLY valid JSON:
{
  "score": <0.0 to 1.0>,
  "understood": <true if score >= 0.65>,
  "feedback": "<1-2 sentences of specific feedback>",
  "followup": "<a follow-up question if score < 0.65, else null>"
}"#;

        let ideal = canonical_answer
            .filter(|s| !s.is_empty())
            .map(|s| format!("\n\nIdeal answer (reference): {s}"))
            .unwrap_or_default();

        let user = format!(
            "Topic: {topic}\nDescription: {description}\nQuestion asked: {question}\nDeveloper's answer: {answer}{ideal}"
        );

        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        Ok(serde_json::from_str(cleaned)?)
    }

    /// Pre-generate a question + ideal answer at topic creation time using full diff context.
    /// Returns (question, ideal_answer). Fails silently — never blocks node insertion.
    pub fn generate_question_and_answer(
        &self,
        topic: &str,
        description: &str,
        commit_msg: &str,
        diff: &str,
        project_summary: &str,
    ) -> Result<(String, String)> {
        let system = r#"You are a Socratic technical mentor pre-generating a quiz question for a developer's personal knowledge graph.

Generate ONE question that forces the developer to reason about the IMPLICATIONS and CONSEQUENCES of this topic — not just recall facts.
The question must be grounded in the actual code changes shown in the diff.

Rules:
- Ask about what breaks, changes, or becomes constrained when using this approach in their specific code
- Ask about trade-offs visible from the diff, or when NOT to use this approach
- Do NOT ask "what is X" or "define X"
- 1-2 sentences, specific to the code shown

Also write an ideal answer: 3-5 sentences demonstrating genuine understanding of consequences and trade-offs,
referencing the specific context from the diff.

Return ONLY valid JSON:
{"question": "<the question>", "answer": "<ideal answer>"}"#;

        let diff_excerpt = if diff.len() > 2500 { &diff[..2500] } else { diff };
        let project = if project_summary.is_empty() { "unknown project" } else { project_summary };

        let user = format!(
            "Topic: {topic}\nDescription: {description}\n\nProject: {project}\nCommit: {commit_msg}\n\nDiff:\n{diff_excerpt}"
        );

        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        #[derive(Deserialize)]
        struct QA { question: String, answer: String }
        let qa: QA = serde_json::from_str(cleaned)
            .map_err(|e| anyhow!("QA parse failed: {e} — raw: {raw}"))?;
        Ok((qa.question, qa.answer))
    }

    /// Summarise a README into 2-3 sentences for use as project context.
    pub fn summarize_readme(&self, readme: &str) -> Result<String> {
        let system = "Summarise this software project's README in 2-3 sentences covering: what it does, its main technologies/stack, and its primary purpose. Be specific and technical. Return only the summary, no preamble.";
        let excerpt = if readme.len() > 4000 { &readme[..4000] } else { readme };
        self.ask(system, excerpt)
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

    /// Classify a list of topic names into domains in a single LLM call.
    /// Returns Vec<(topic, domain)>.
    pub fn classify_domains(&self, topics: &[(String, String)]) -> Result<Vec<(String, String)>> {
        let system = r#"You are classifying technical topics into knowledge domains.
For each topic+description pair, assign exactly one domain from this list:
Language, Database, Auth, API, Frontend, DevOps, Architecture, Performance, Security, Testing, Tooling, Data, Other

Return ONLY valid JSON: array of objects with "topic" and "domain" keys.
Use the exact topic string provided, unchanged."#;

        let input: Vec<serde_json::Value> = topics.iter()
            .map(|(t, d)| serde_json::json!({"topic": t, "description": d}))
            .collect();
        let user = serde_json::to_string(&input)?;
        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        let result: Vec<serde_json::Value> = serde_json::from_str(cleaned)?;

        Ok(result.into_iter().filter_map(|v| {
            let topic = v["topic"].as_str()?.to_string();
            let domain = v["domain"].as_str()?.to_string();
            Some((topic, domain))
        }).collect())
    }

    /// Generate a cross-concept question that asks how two connected topics interact.
    pub fn generate_cross_concept_question(
        &self,
        topic_a: &str,
        desc_a: &str,
        topic_b: &str,
        desc_b: &str,
        edge_kind: &str,
        edge_description: &str,
        task_context: &str,
    ) -> Result<String> {
        let system = r#"You are a Socratic technical mentor. Two concepts in a developer's knowledge graph are connected.
Generate ONE question that forces them to reason about how these two concepts interact in practice.

Rules:
- Ask about a real scenario where understanding both concepts together matters
- Focus on what breaks, changes, or must be considered when using one alongside the other
- The question must reference both concepts explicitly
- Keep it to 1-2 sentences
- Return ONLY the question, no preamble"#;

        let user = format!(
            "Concept A: {topic_a} — {desc_a}\nConcept B: {topic_b} — {desc_b}\nRelationship: {edge_kind} ({edge_description})\nTask context: {task_context}"
        );

        self.ask(system, &user)
    }

    /// Generate implication edges between a newly added topic and existing PKG nodes.
    /// Returns up to 4 edges. Fails silently — never blocks the quiz flow.
    pub fn generate_edges(
        &self,
        new_topic: &str,
        new_description: &str,
        existing: &[(String, String)], // (topic, description)
    ) -> Result<Vec<GeneratedEdge>> {
        if existing.is_empty() {
            return Ok(vec![]);
        }

        let system = r#"You are building an implication graph for a personal knowledge graph.
A new topic has just been added. Identify at most 4 strongly related topics from the existing list.

Relationship kinds:
- "implies": understanding the new topic strongly implies you should also understand the target
- "depends_on": the new topic requires understanding the target as a prerequisite
- "conflicts_with": these topics involve genuine trade-offs or contradictory approaches in practice
- "part_of": the new topic is a specific instance, specialisation, or subcomponent of the target

ONLY include relationships with strength ≥ 0.6 — skip anything tangential or loosely related.
strength: 0.6 (clearly related) → 0.8 (closely coupled) → 1.0 (foundational dependency).

For "description": explain in one concrete sentence WHY this relationship exists — what breaks or changes
if you misunderstand one while knowing the other. Do not just restate the topic names.

Return ONLY valid JSON array using the exact topic strings from the existing list:
[{"target": "<exact topic>", "kind": "implies"|"depends_on"|"conflicts_with"|"part_of", "description": "<one concrete sentence explaining the reason>", "strength": <0.6–1.0>}]
If no strongly related relationships exist return: []"#;

        let existing_list = existing
            .iter()
            .map(|(t, d)| format!("- {t}: {d}"))
            .collect::<Vec<_>>()
            .join("\n");

        let user = format!(
            "New topic: {new_topic}\nDescription: {new_description}\n\nExisting topics:\n{existing_list}"
        );

        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        Ok(serde_json::from_str(cleaned).unwrap_or_default())
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
