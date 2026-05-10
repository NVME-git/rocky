/// Rocky — the AI teacher.
/// Calls the Anthropic API (or Ollama) for topic extraction, question generation,
/// and answer evaluation.
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::node::Node;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Difficulty {
    Simpler,
    Normal,
    Harder,
}

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
    /// Local LLMs sometimes omit `kind`. Default to "concept" rather than panic-parsing.
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub description: String,
}

fn default_kind() -> String { "concept".into() }

/// Parse a TopicInfo list from LLM output that may be:
///   1. A bare JSON array     `[{...}, {...}]`
///   2. An object with a "topics" key  `{"topics": [...]}`
///   3. A single object        `{"topic": "...", ...}`  (treated as one-element list)
///
/// Local LLMs love to wrap JSON inconsistently — accepting all three keeps
/// the alpha pipeline from blowing up on a single bad response.
fn parse_topics_lenient(cleaned: &str) -> Result<Vec<TopicInfo>> {
    if let Ok(v) = serde_json::from_str::<Vec<TopicInfo>>(cleaned) {
        return Ok(v);
    }
    let val: Value = serde_json::from_str(cleaned)
        .map_err(|e| anyhow!("LLM did not return parseable JSON: {e}"))?;
    if let Some(arr) = val.get("topics").and_then(|t| t.as_array()) {
        return serde_json::from_value(Value::Array(arr.clone()))
            .map_err(|e| anyhow!("LLM topics array failed to parse: {e}"));
    }
    if val.get("topic").is_some() {
        let one: TopicInfo = serde_json::from_value(val)
            .map_err(|e| anyhow!("LLM single-topic object failed to parse: {e}"))?;
        return Ok(vec![one]);
    }
    Err(anyhow!("LLM returned JSON but no topic list could be extracted"))
}

#[derive(Debug, Deserialize)]
pub struct EvalResult {
    pub score: f64,
    pub understood: bool,
    pub feedback: String,
    pub followup: Option<String>,
}

/// Output of `Teacher::enrich_topic_from_conversation` — what to merge back
/// into a topic after a "Teach Me Copy & Go" conversation is pasted back
/// into the topic card's capture surface.
#[derive(Debug, Clone)]
pub struct EnrichmentResult {
    pub summary: String,
    pub refined_description: String,
    pub new_questions: Vec<crate::node::QuestionBankItem>,
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
                client: reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .unwrap_or_default(),
            },
        }
    }

    pub fn ollama(base_url: String, model: String) -> Self {
        // Local CPU inference of large generations (e.g. multi-paragraph summaries
        // or JSON question banks) can take minutes. Be generous.
        Self {
            provider: Provider::Ollama {
                base_url,
                model,
                client: reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(600))
                    .build()
                    .unwrap_or_default(),
            },
        }
    }

    fn ask(&self, system: &str, user: &str) -> Result<String> {
        self.ask_with_overrides(system, user, None, 1024)
    }

    /// Same as `ask`, but allows overriding the model and max_tokens for this call only.
    /// Used by `summarize_project_docs` (uses Opus) and `generate_question_bank` (needs more tokens).
    fn ask_with_overrides(
        &self,
        system: &str,
        user: &str,
        model_override: Option<&str>,
        max_tokens: u32,
    ) -> Result<String> {
        match &self.provider {
            Provider::Claude { api_key, model, client } => {
                let effective_model = model_override.unwrap_or(model);
                let resp = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&json!({
                        "model": effective_model,
                        "max_tokens": max_tokens,
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
                let _ = model_override;
                let prompt = format!("System: {system}\n\nUser: {user}");
                // num_ctx: ensure the full prompt fits (default 2048 is too small for
                // session-end which includes project context + existing topics + diff).
                // num_predict: honour the caller's max_tokens to bound output length.
                let payload = json!({
                    "model": model,
                    "prompt": prompt,
                    "stream": false,
                    "options": {
                        "num_ctx": 8192,
                        "num_predict": max_tokens
                    }
                });

                // Local Ollama can drop connections under sustained load (e.g. while
                // generating several question banks back-to-back). Retry transport
                // errors with backoff before bailing.
                let mut last_err: Option<anyhow::Error> = None;
                for attempt in 0..3 {
                    if attempt > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(500 * (1 << attempt)));
                    }
                    match client
                        .post(format!("{base_url}/api/generate"))
                        .json(&payload)
                        .send()
                    {
                        Ok(resp) => {
                            let data: Value = match resp.json() {
                                Ok(v) => v,
                                Err(e) => { last_err = Some(anyhow!(e)); continue; }
                            };
                            if let Some(s) = data["response"].as_str() {
                                return Ok(s.to_string());
                            }
                            last_err = Some(anyhow!("no response field from Ollama"));
                        }
                        Err(e) => { last_err = Some(anyhow!(e)); }
                    }
                }
                Err(last_err.unwrap_or_else(|| anyhow!("ollama request failed")))
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
        parse_topics_lenient(cleaned)
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
        parse_topics_lenient(cleaned)
    }

    pub fn generate_question(
        &self,
        topic: &str,
        description: &str,
        context: &str,
        known_topics: &[String],
        question_num: u32,
        difficulty: Difficulty,
    ) -> Result<String> {
        let difficulty_hint = match difficulty {
            Difficulty::Normal => "".to_string(),
            Difficulty::Simpler => "\nDifficulty adjustment: make the question SIMPLER — focus on the core mechanic or most basic consequence, avoiding edge cases. Suitable for someone just starting to understand this topic.".to_string(),
            Difficulty::Harder => "\nDifficulty adjustment: make the question HARDER — push into edge cases, subtle failure modes, or interactions with other systems. Assume solid foundational understanding.".to_string(),
        };

        let system = format!(r#"You are a Socratic technical mentor. Your job is to generate ONE question
that forces a developer to reason about the IMPLICATIONS and CONSEQUENCES of a technical topic,
not just recall facts.

Rules:
- Ask about how the topic affects other things they've built or will build
- Ask about what breaks, changes, or becomes constrained when using this approach
- Ask about trade-offs and when NOT to use this approach
- Do NOT ask "what is X" or "define X" — assume basic awareness
- The question should be specific to their task context
- Keep it to 1-2 sentences
- Return ONLY the question, no preamble{difficulty_hint}"#);

        let known_str = if known_topics.is_empty() {
            "none yet".to_string()
        } else {
            known_topics.join(", ")
        };

        let user = format!(
            "Topic: {topic}\nDescription: {description}\nTask context: {context}\nDeveloper's known topics: {known_str}\nQuestion number: {question_num} (vary difficulty/angle if > 1)"
        );

        self.ask(&system, &user)
    }

    /// Generate a short clue at runtime (used for manually-added topics without canonical clue).
    pub fn generate_clue(&self, topic: &str, description: &str, question: &str) -> Result<String> {
        let system = "You are a Socratic technical mentor. The developer is stuck on a quiz question. \
Give a SHORT clue (1-2 sentences) that nudges them in the right direction without giving away the answer. \
Focus on the core concept or the most important thing to think about. Return ONLY the clue, no preamble.";
        let user = format!("Topic: {topic}\nDescription: {description}\nQuestion: {question}");
        self.ask(system, &user)
    }

    pub fn evaluate_answer(
        &self,
        topic: &str,
        question: &str,
        answer: &str,
        description: &str,
        canonical_answer: Option<&str>,
        personality: bool,
    ) -> Result<EvalResult> {
        // Tone toggle: with personality on, the feedback addresses the user
        // as "friend" in Rocky's caveman-mentor voice; off, it's a precise
        // second-person ("you") technical voice. Only the feedback string
        // changes — score / understood / followup stay neutral JSON fields.
        let voice = if personality {
            "Address the developer as \"friend\". Voice: Rocky the caveman mentor — short sentences, \
playful ('is good', 'Rocky think', 'brain work excellent'), warm but technically sharp. \
Keep the technical content precise; the caveman flavour is in the framing words only."
        } else {
            "Address the developer as \"you\". Voice: precise, encouraging mentor. No fluff, no \
ornamentation."
        };
        let system = format!(
            r#"You are evaluating whether a developer genuinely understands the implications
of a technical topic based on their answer to a Socratic question.

{voice}

Evaluate on:
1. Do they demonstrate understanding of consequences, not just surface knowledge?
2. Do they show awareness of how this affects related systems?
3. Is there evidence they could reason through related problems?

If an ideal answer is provided, use it as a reference for what a complete answer looks like —
but do not penalise for different phrasing or approach, only for missing key insights.

Return ONLY valid JSON:
{{
  "score": <0.0 to 1.0>,
  "understood": <true if score >= 0.65>,
  "feedback": "<2-3 sentences of specific feedback in the chosen voice>",
  "followup": "<a follow-up question if score < 0.65, else null>"
}}"#
        );

        let ideal = canonical_answer
            .filter(|s| !s.is_empty())
            .map(|s| format!("\n\nIdeal answer (reference): {s}"))
            .unwrap_or_default();

        let user = format!(
            "Topic: {topic}\nDescription: {description}\nQuestion asked: {question}\nDeveloper's answer: {answer}{ideal}"
        );

        let raw = self.ask(&system, &user)?;
        let cleaned = strip_code_fence(&raw);
        Ok(serde_json::from_str(cleaned)?)
    }

    /// Pre-generate a question + ideal answer + clue at topic creation time using full diff context.
    /// Returns (question, ideal_answer, clue). Fails silently — never blocks node insertion.
    pub fn generate_question_and_answer(
        &self,
        topic: &str,
        description: &str,
        commit_msg: &str,
        diff: &str,
        project_summary: &str,
    ) -> Result<(String, String, String)> {
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

Also write a short clue (1-2 sentences) that nudges the developer in the right direction without giving away
the answer — something they can ask for if they get stuck.

Return ONLY valid JSON:
{"question": "<the question>", "answer": "<ideal answer>", "clue": "<short clue>"}"#;

        let diff_excerpt = if diff.len() > 2500 {
            let boundary = diff.char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= 2500)
                .last()
                .unwrap_or(0);
            &diff[..boundary]
        } else {
            diff
        };
        let project = if project_summary.is_empty() { "unknown project" } else { project_summary };

        let user = format!(
            "Topic: {topic}\nDescription: {description}\n\nProject: {project}\nCommit: {commit_msg}\n\nDiff:\n{diff_excerpt}"
        );

        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        #[derive(Deserialize)]
        struct QA { question: String, answer: String, clue: Option<String> }
        let qa: QA = serde_json::from_str(cleaned)
            .map_err(|e| anyhow!("QA parse failed: {e} — raw: {raw}"))?;
        Ok((qa.question, qa.answer, qa.clue.unwrap_or_default()))
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

    /// Summarise a project's documentation into a few-paragraph context record.
    /// Used by `rocky explore` — runs once per project and on periodic refresh.
    /// Uses Opus when provider=claude; ignored for Ollama (uses configured model).
    pub fn summarize_project_docs(
        &self,
        project_name: &str,
        docs: &[(String, String)],
        recent_commits: &[String],
    ) -> Result<String> {
        let system = r#"You are summarising a software project so that a knowledge-graph tool
can ground future quiz questions in real architectural context.

Produce a 3-5 paragraph summary that covers:
1. What this project does and the problem it solves.
2. Core architectural decisions and the patterns it uses (e.g. event-driven, layered, CQRS, etc.).
3. Key technologies, frameworks, and external dependencies.
4. Any explicit conventions or constraints (testing strategy, security posture, performance budgets).
5. Active areas of development based on recent commits.

Write in plain prose. No bullet lists unless absolutely necessary. Be specific and concrete —
this summary will be quoted verbatim into prompts that generate questions, so vague generalities
ruin downstream quality. Do not include code snippets, file paths, or anything that looks like
implementation detail. Concepts and decisions only."#;

        let mut input = format!("Project: {project_name}\n\n");

        if !docs.is_empty() {
            input.push_str("Documentation files:\n\n");
            for (path, content) in docs {
                let excerpt = if content.len() > 6000 {
                    let mut end = 6000;
                    while !content.is_char_boundary(end) && end > 0 {
                        end -= 1;
                    }
                    &content[..end]
                } else {
                    content.as_str()
                };
                input.push_str(&format!("--- {path} ---\n{excerpt}\n\n"));
            }
        }

        if !recent_commits.is_empty() {
            input.push_str("Recent commit messages:\n");
            for c in recent_commits.iter().take(30) {
                input.push_str(&format!("- {c}\n"));
            }
        }

        // Opus for higher-quality project synthesis. Only runs on `rocky explore`.
        self.ask_with_overrides(system, &input, Some("claude-opus-4-7"), 2048)
    }

    /// Pre-generate a bank of 3-5 questions per topic using rich context
    /// (project summary + transcript + diff). Returns parsed JSON questions.
    pub fn generate_question_bank(
        &self,
        topic: &str,
        description: &str,
        project_context: &str,
        transcript_block: &str,
        diff_excerpt: &str,
    ) -> Result<Vec<crate::node::QuestionBankItem>> {
        let system = r#"You are a Socratic technical mentor pre-generating a bank of quiz questions
for a developer's personal knowledge graph.

Generate 4 questions about the given topic. Each must:
- Force reasoning about IMPLICATIONS, TRADE-OFFS, or CONSEQUENCES — not recall of definitions.
- Be grounded in the actual context provided (project, session, code).
- Target a different angle: what breaks, when NOT to use it, how it interacts with adjacent
  systems, what would change if a key constraint was removed.
- Be 1-2 sentences, specific to this codebase.

For each question also produce:
- An ideal answer (3-5 sentences) demonstrating real understanding of the consequences.
- A short clue (1-2 sentences) that nudges without revealing the answer.

Return ONLY valid JSON of this exact shape:
{"questions": [
  {"question": "...", "answer": "...", "clue": "..."},
  ...
]}"#;

        let mut user = format!(
            "Topic: {topic}\nTopic description: {description}\n\n"
        );
        if !project_context.is_empty() {
            user.push_str(&format!("Project context:\n{project_context}\n\n"));
        }
        if !transcript_block.is_empty() {
            user.push_str(&format!("Recent session activity:\n{transcript_block}\n"));
        }
        if !diff_excerpt.is_empty() {
            let excerpt = if diff_excerpt.len() > 2500 {
                let mut end = 2500;
                while !diff_excerpt.is_char_boundary(end) && end > 0 {
                    end -= 1;
                }
                &diff_excerpt[..end]
            } else {
                diff_excerpt
            };
            user.push_str(&format!("Diff excerpt:\n{excerpt}\n"));
        }

        let raw = self.ask_with_overrides(&system, &user, None, 2048)?;
        let cleaned = strip_code_fence(&raw);

        #[derive(Deserialize)]
        struct Bank { questions: Vec<crate::node::QuestionBankItem> }
        let bank: Bank = serde_json::from_str(cleaned)
            .map_err(|e| anyhow!("question bank parse failed: {e} — raw: {raw}"))?;
        Ok(bank.questions)
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

    /// Given two duplicate topics, ask the LLM to pick the cleaner canonical name
    /// and write a unified description. Returns (name, description).
    pub fn suggest_merge_name(
        &self,
        a: &str,
        desc_a: &str,
        b: &str,
        desc_b: &str,
    ) -> Result<(String, String)> {
        let system = r#"You are a knowledge graph curator. Two topics are duplicates and will be merged.
Choose the clearer, more canonical name and write a single concise description.

Respond ONLY with valid JSON — no other text:
{"name": "chosen topic name", "description": "1-2 sentence unified description"}"#;

        let user = format!(
            "Topic A: {a}\nDescription A: {desc_a}\n\nTopic B: {b}\nDescription B: {desc_b}\n\nPick the best canonical name and write a unified description."
        );
        let raw = self.ask(system, &user)?;
        let cleaned = strip_code_fence(&raw);
        let val: serde_json::Value = serde_json::from_str(cleaned)
            .map_err(|e| anyhow::anyhow!("suggest_merge_name: bad JSON: {e}\nraw: {raw}"))?;
        let name = val["name"].as_str().unwrap_or(a).to_string();
        let desc = val["description"].as_str().unwrap_or(desc_a).to_string();
        Ok((name, desc))
    }

    /// Mine a captured "Teach Me" conversation for new question-bank items
    /// and (optionally) a refined description. The user pastes the dialogue
    /// into the topic card's capture surface, which POSTs it to /api/lessons;
    /// the server then asks the local LLM to extract durable lessons.
    ///
    /// Returns:
    /// - `summary`: one-line takeaway shown on the topic detail card.
    /// - `refined_description`: empty string when the existing description
    ///   already covers what the conversation taught.
    /// - `new_questions`: 1–3 fresh Q&A items, deduped against `existing_questions`.
    pub fn enrich_topic_from_conversation(
        &self,
        topic: &str,
        description: &str,
        existing_questions: &[String],
        conversation: &str,
    ) -> Result<EnrichmentResult> {
        let system = r#"You analyse a teaching conversation between a developer and an LLM
and extract durable lessons for the developer's personal knowledge graph.

The developer already has a topic card with a description and a question bank.
Your job is to harvest the conversation for new insights — NOT to recap it.

Return JSON of this exact shape:
{
  "summary": "<one sentence describing the most important thing the developer learned, or '' if the conversation added nothing new>",
  "refined_description": "<a tighter, more accurate ≤200-char description, or '' to keep the existing one>",
  "new_questions": [
    {"question": "...", "answer": "...", "clue": "..."}
  ]
}

Rules:
- Each new question must probe a non-trivial implication, trade-off, edge case,
  or consequence that emerged from the conversation. No definition recall.
- Skip anything that overlaps with the existing questions in spirit, not just wording.
- Return between 0 and 3 new questions. Empty array is fine when nothing
  durable was learned.
- "answer" is 3-5 sentences demonstrating real understanding.
- "clue" is 1-2 sentences nudging toward the answer without revealing it.
- Only set "refined_description" if the existing one is wrong or materially
  weaker than what the conversation revealed."#;

        let truncated = if conversation.len() > 12_000 {
            let mut end = 12_000;
            while !conversation.is_char_boundary(end) && end > 0 { end -= 1; }
            &conversation[..end]
        } else {
            conversation
        };

        let existing_block = if existing_questions.is_empty() {
            "(none)".to_string()
        } else {
            existing_questions
                .iter()
                .map(|q| format!("- {q}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let user = format!(
            "Topic: {topic}\nCurrent description: {description}\n\n\
             Existing question bank:\n{existing_block}\n\n\
             Conversation transcript:\n{truncated}"
        );

        let raw = self.ask_with_overrides(system, &user, None, 2048)?;
        let cleaned = strip_code_fence(&raw);

        #[derive(Deserialize)]
        struct Wire {
            #[serde(default)] summary: String,
            #[serde(default)] refined_description: String,
            #[serde(default)] new_questions: Vec<crate::node::QuestionBankItem>,
        }
        let wire: Wire = serde_json::from_str(cleaned)
            .map_err(|e| anyhow!("enrichment parse failed: {e} — raw: {raw}"))?;

        Ok(EnrichmentResult {
            summary: wire.summary,
            refined_description: wire.refined_description,
            new_questions: wire.new_questions,
        })
    }

    /// Ask the LLM whether two topics from the PKG represent the same concept.
    /// Used by `rocky dedupe` after the word-overlap heuristic narrows the candidate set.
    pub fn is_duplicate_pair(
        &self,
        a: &str,
        desc_a: &str,
        b: &str,
        desc_b: &str,
    ) -> Result<bool> {
        let system = r#"You are a knowledge graph deduplication assistant.
Given two topics from a developer's PKG, decide if they represent the same underlying concept.

Answer ONLY with one of these exact JSON values (no other text):
{"duplicate": true}
{"duplicate": false}

Rules:
- TRUE  if topics are the same concept with different wording (e.g. "exponential backoff" vs "retry with exponential backoff")
- FALSE if one is a meaningful sub-concept or specialisation of the other
- FALSE when in doubt — the human will confirm"#;

        let user = format!(
            "Topic A: {a}\nDescription A: {desc_a}\n\nTopic B: {b}\nDescription B: {desc_b}\n\nSame concept?"
        );
        let raw = self.ask(system, &user)?;
        Ok(raw.contains("\"duplicate\": true") || raw.contains("\"duplicate\":true"))
    }
}

fn strip_code_fence(s: &str) -> &str {
    let s = s.trim();
    // Find the first ``` fence anywhere in the response (handles preamble text).
    if let Some(fence_start) = s.find("```") {
        let after = &s[fence_start + 3..];
        let after = after.strip_prefix("json").unwrap_or(after);
        let after = after.trim_start_matches('\n');
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
        return after.trim();
    }
    // No fence — return as-is and let the caller try to parse it.
    s
}
