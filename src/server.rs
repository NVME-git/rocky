use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tower_http::cors::CorsLayer;
use colored::Colorize;

use crate::config::Config;
use crate::db::Db;
use crate::fsrs;
use crate::promptiq;
use crate::teacher::Teacher;
use crate::voice;

// ── state ────────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Db,
    pub teacher: Option<Teacher>,
    pub config: Config,
}

// ── public entry point ───────────────────────────────────────────────────────

pub fn run(db: &Db, cfg: &Config) -> Result<()> {
    let teacher = make_teacher_optional(cfg);
    let has_llm = teacher.is_some();
    let state = Arc::new(AppState {
        db: db.clone(),
        teacher,
        config: cfg.clone(),
    });

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let app = Router::new()
            .route("/", get(serve_app))
            .route("/api/data", get(get_data))
            .route("/api/sessions", get(get_sessions))
            .route("/api/quiz/start", post(quiz_start))
            .route("/api/quiz/assess", post(quiz_assess))
            .route("/api/quiz/evaluate", post(quiz_evaluate))
            .route("/api/transcribe", post(transcribe))
            .route("/api/prompt-iq", get(get_prompt_iq))
            .route("/api/topic/delete-question", post(delete_bank_question))
            .route("/api/topic/generate-questions", post(generate_bank_questions))
            .route("/api/feedback", get(read_feedback).post(write_feedback))
            .route("/api/feedback/append", post(append_feedback))
            .layer(CorsLayer::permissive())
            .with_state(state);

        // Allow ROCKY_BIND to override the default loopback bind. Used by the
        // Docker image to expose the UI on the container's external interface
        // (e.g. ROCKY_BIND=0.0.0.0:7777).
        let bind = std::env::var("ROCKY_BIND").unwrap_or_else(|_| "127.0.0.1:0".to_string());
        let listener = tokio::net::TcpListener::bind(&bind).await?;
        let addr = listener.local_addr()?;

        println!(
            "  {} Rocky running at http://{}",
            "✓".truecolor(29, 158, 117),
            addr
        );
        if has_llm {
            println!("  {} LLM quiz enabled", "✓".truecolor(29, 158, 117));
        } else {
            println!(
                "  {} LLM quiz disabled (no API key) — self-assessment only",
                "⚠".truecolor(239, 159, 39)
            );
        }

        // Skip auto-open when running headless (Docker, SSH, CI). The
        // ROCKY_BIND override is the signal the user has chosen to expose
        // the UI to something other than the local browser.
        if std::env::var("ROCKY_BIND").is_err() {
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open")
                .arg(format!("http://{addr}"))
                .spawn();
            #[cfg(not(target_os = "macos"))]
            let _ = std::process::Command::new("xdg-open")
                .arg(format!("http://{addr}"))
                .spawn();
        }

        axum::serve(listener, app).await?;
        Ok(())
    })
}

fn make_teacher_optional(cfg: &Config) -> Option<Teacher> {
    if cfg.llm_provider == "ollama" {
        Some(Teacher::ollama(
            cfg.ollama_base_url.clone(),
            cfg.llm_model.clone(),
        ))
    } else {
        dotenvy::dotenv().ok();
        std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .map(|key| Teacher::claude(key, cfg.llm_model.clone()))
    }
}

// ── routes ───────────────────────────────────────────────────────────────────

async fn serve_app() -> Html<&'static str> {
    Html(include_str!("app.html"))
}

async fn get_data(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let db = state.db.clone();
    let cfg = state.config.clone();
    let result = tokio::task::spawn_blocking(move || build_data_json(&db, &cfg))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(result))
}

async fn get_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || build_sessions_json(&db))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(result))
}

async fn get_prompt_iq(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> Result<Value, anyhow::Error> {
        let _ = promptiq::ensure_migrated(&db);
        let report = promptiq::current_iq()?;
        Ok(serde_json::to_value(&report)?)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    .map(Json)
}

/// POST /api/transcribe — raw audio bytes in (WAV PCM 16 kHz mono preferred),
/// JSON {transcript: "..."} out. Status codes the web UI knows about:
///   - 200 OK         — transcript in the response body
///   - 400 Bad Request — provider is "browser" (client should run Web Speech)
///   - 413 Payload Too Large — > 25 MB audio
///   - 422 Unprocessable Entity — empty audio buffer
///   - 500 Internal Server Error — STT failed (model error, whisper crash, etc)
///   - 502 Bad Gateway — whisper binary spawn error (typically not installed)
///   - 503 Service Unavailable — voice.provider = "off"
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024; // 25 MB hard cap

async fn transcribe(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, (StatusCode, String)> {
    let cfg = &state.config;

    if cfg.voice.provider == "off" {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "voice is disabled. Set [voice] provider in ~/.config/rocky/config.toml \
             (recommended: \"whisper-cpp\" — see scripts/install-whisper.sh)".into(),
        ));
    }
    if cfg.voice.provider == "browser" {
        return Err((
            StatusCode::BAD_REQUEST,
            "browser STT runs client-side; do not POST audio to /api/transcribe".into(),
        ));
    }
    if body.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "empty audio body".into()));
    }
    if body.len() > MAX_AUDIO_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("audio is {} bytes, max is {} ({} MB)",
                body.len(), MAX_AUDIO_BYTES, MAX_AUDIO_BYTES / (1024*1024)),
        ));
    }

    let stt = voice::make_stt(cfg)
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, "voice provider produced no STT".into()))?;

    // Whisper subprocess can take seconds — push it off the async runtime.
    let bytes = body.to_vec();
    let transcript = tokio::task::spawn_blocking(move || stt.transcribe(&bytes))
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "transcription task panicked".into()))?
        .map_err(|e| {
            let msg = e.to_string();
            // Distinguish "binary not installed" from generic STT failures so the
            // web UI can surface the install-script link.
            let code = if msg.contains("not found in PATH") || msg.contains("model not found") {
                StatusCode::BAD_GATEWAY
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (code, msg)
        })?;

    Ok(Json(json!({ "transcript": transcript })))
}

// ── quiz endpoints ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct QuizStartReq {
    node_ids: Vec<String>,
}

#[derive(Serialize)]
struct QuizQuestion {
    node_id: String,
    topic: String,
    domain: String,
    description: String,
    question: String,
    answer: String,
    clue: String,
    has_canonical: bool,
}

#[derive(Serialize)]
struct QuizStartResp {
    questions: Vec<QuizQuestion>,
    llm_available: bool,
}

async fn quiz_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QuizStartReq>,
) -> Result<Json<QuizStartResp>, StatusCode> {
    let db = state.db.clone();
    let has_llm = state.teacher.is_some();
    // Single-topic quizzes get the *full* bank surfaced (capped at 8) so the
    // user can step through every question they've authored. Multi-topic
    // quizzes return one question per topic — otherwise a "quiz top 5" call
    // could balloon to 25+ questions.
    let single_topic = req.node_ids.len() == 1;
    let questions = tokio::task::spawn_blocking(move || -> Result<Vec<QuizQuestion>> {
        let mut qs = Vec::new();
        for nid in &req.node_ids {
            if let Some(node) = db.get_node_by_id(nid)? {
                if !node.question_bank.is_empty() {
                    // Sort indices by asked_count ascending so the freshest
                    // question comes first. Stable on ties (preserve insertion
                    // order for predictability).
                    let mut idx: Vec<usize> = (0..node.question_bank.len()).collect();
                    idx.sort_by_key(|&i| node.question_bank[i].asked_count);
                    let take = if single_topic { idx.len().min(8) } else { 1 };

                    let mut bank = node.question_bank.clone();
                    for &i in idx.iter().take(take) {
                        let q = &node.question_bank[i];
                        qs.push(QuizQuestion {
                            node_id: nid.clone(),
                            topic: node.topic.clone(),
                            domain: node.domain.clone(),
                            description: node.description.clone(),
                            question: q.question.clone(),
                            answer: q.answer.clone(),
                            clue: q.clue.clone(),
                            has_canonical: true,
                        });
                        // Bump asked_count so subsequent sessions rotate.
                        bank[i].asked_count += 1;
                    }
                    let _ = db.set_question_bank(&node.topic, &bank);
                    continue;
                }
                // No bank → fall back to canonical, then "Explain:", then bare.
                let (question, answer, clue, has_canonical) =
                    if !node.canonical_question.is_empty() {
                        (
                            node.canonical_question.clone(),
                            node.canonical_answer.clone(),
                            node.canonical_clue.clone(),
                            true,
                        )
                    } else if !node.description.is_empty() {
                        (
                            format!("Explain: {} — {}", node.topic, node.description),
                            String::new(),
                            String::new(),
                            false,
                        )
                    } else {
                        (
                            format!("What do you know about {}?", node.topic),
                            String::new(),
                            String::new(),
                            false,
                        )
                    };
                qs.push(QuizQuestion {
                    node_id: nid.clone(),
                    topic: node.topic.clone(),
                    domain: node.domain.clone(),
                    description: node.description.clone(),
                    question,
                    answer,
                    clue,
                    has_canonical,
                });
            }
        }
        Ok(qs)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QuizStartResp {
        questions,
        llm_available: has_llm,
    }))
}

#[derive(Deserialize)]
struct QuizAssessReq {
    node_id: String,
    score: f64,
    question: String,
}

#[derive(Serialize)]
struct QuizAssessResp {
    ok: bool,
    new_retrievability: f64,
}

async fn quiz_assess(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QuizAssessReq>,
) -> Result<Json<QuizAssessResp>, StatusCode> {
    let db = state.db.clone();
    let new_r = tokio::task::spawn_blocking(move || {
        db.record_quiz_review(&req.node_id, req.score, &req.question)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QuizAssessResp {
        ok: true,
        new_retrievability: new_r,
    }))
}

#[derive(Deserialize)]
struct QuizEvalReq {
    node_id: String,
    question: String,
    answer: String,
}

#[derive(Serialize)]
struct QuizEvalResp {
    score: f64,
    feedback: String,
    followup: Option<String>,
}

async fn quiz_evaluate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QuizEvalReq>,
) -> Result<Json<QuizEvalResp>, StatusCode> {
    // Verify LLM is available before doing any work
    if state.teacher.is_none() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let db = state.db.clone();
    let node_id = req.node_id.clone();

    // Fetch node info for context
    let node = {
        let db2 = db.clone();
        let nid = node_id.clone();
        tokio::task::spawn_blocking(move || db2.get_node_by_id(&nid))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?
    };

    // Teacher is Send+Sync (holds reqwest::blocking::Client) — safe to reference across await
    let topic = node.topic.clone();
    let description = node.description.clone();
    let canonical_answer = if node.canonical_answer.is_empty() {
        None
    } else {
        Some(node.canonical_answer.clone())
    };
    let question = req.question.clone();
    let answer = req.answer.clone();

    // We need to call teacher in a blocking context.
    // Teacher is behind Arc<AppState> which is not moved into the closure,
    // so we need to construct a new teacher or use unsafe. Instead, let's
    // use the fact that Teacher's ask() uses reqwest::blocking::Client
    // which is Clone. We'll reconstruct for the spawn_blocking.
    // Actually, since AppState is Arc and teacher is &Teacher, we can clone the Arc.
    let state2 = state.clone();
    let personality = state.config.personality;
    let eval = tokio::task::spawn_blocking(move || -> Result<crate::teacher::EvalResult> {
        let teacher = state2.teacher.as_ref().unwrap();
        teacher.evaluate_answer(
            &topic,
            &question,
            &answer,
            &description,
            canonical_answer.as_deref(),
            personality,
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Record the review (record_quiz_review also calls add_review internally)
    let score = eval.score;
    let question2 = req.question.clone();
    let answer2 = req.answer.clone();
    let feedback2 = eval.feedback.clone();
    tokio::task::spawn_blocking(move || {
        // Update FSRS + write review row
        let _ = db.record_quiz_review(&node_id, score, &question2);
        // Override the review row with full answer/feedback details
        let _ = db.add_review(&node_id, &question2, &answer2, &feedback2, score);
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QuizEvalResp {
        score: eval.score,
        feedback: eval.feedback,
        followup: eval.followup,
    }))
}

// ── data assembly ────────────────────────────────────────────────────────────
// Extracted from run_view_projects() in main.rs

fn build_data_json(db: &Db, cfg: &Config) -> Result<Value> {
    let all_nodes = db.all_nodes()?;
    let topic_nodes: Vec<_> = all_nodes.iter().filter(|n| !n.kind.is_domain()).collect();
    let domain_nodes: Vec<_> = all_nodes.iter().filter(|n| n.kind.is_domain()).collect();

    let user_id = "__user__";
    let user_name = &cfg.user_name;

    // ── per-project aggregation
    let mut projects: Map<String, Value> = Map::new();
    let mut repo_topics: std::collections::HashMap<String, Vec<&crate::node::Node>> =
        std::collections::HashMap::new();

    for n in &topic_nodes {
        let repo = if n.repo.is_empty() {
            "Other".to_string()
        } else {
            n.repo.clone()
        };
        repo_topics.entry(repo).or_default().push(n);
    }

    for (repo, nodes_in_repo) in &repo_topics {
        let mut domain_counts: Map<String, Value> = Map::new();
        let mut known = 0u32;
        let mut fading = 0u32;
        let mut gap = 0u32;

        for n in nodes_in_repo {
            let (_, _, recall) = db.node_recall(n);
            let cls = fsrs::classify(recall);
            match cls {
                "known" => known += 1,
                "stale" => fading += 1,
                _ => gap += 1,
            }
            let d = if n.domain.is_empty() { "Other" } else { &n.domain };
            let count = domain_counts.entry(d.to_string()).or_insert(json!(0));
            *count = json!(count.as_u64().unwrap_or(0) + 1);
        }

        projects.insert(
            repo.clone(),
            json!({
                "topicCount": nodes_in_repo.len(),
                "domains": domain_counts,
                "health": { "known": known, "fading": fading, "gap": gap },
            }),
        );
    }

    // ── cross-project flows
    let all_edges = db.get_all_edges()?;

    let node_repo: std::collections::HashMap<&str, &str> = topic_nodes
        .iter()
        .map(|n| {
            (
                n.id.as_str(),
                if n.repo.is_empty() { "Other" } else { n.repo.as_str() },
            )
        })
        .collect();

    let mut flow_map: std::collections::HashMap<(String, String), [u32; 4]> =
        std::collections::HashMap::new();
    for e in &all_edges {
        let sr = node_repo
            .get(e.source_id.as_str())
            .copied()
            .unwrap_or("Other");
        let tr = node_repo
            .get(e.target_id.as_str())
            .copied()
            .unwrap_or("Other");
        let counts = flow_map
            .entry((sr.to_string(), tr.to_string()))
            .or_insert([0; 4]);
        match e.kind.as_str() {
            "implies" => counts[0] += 1,
            "depends_on" => counts[1] += 1,
            "part_of" => counts[2] += 1,
            "conflicts_with" => counts[3] += 1,
            _ => {}
        }
    }

    let flows: Vec<Value> = flow_map
        .into_iter()
        .filter(|((s, t), _)| s != t)
        .map(|((s, t), c)| {
            json!({
                "source": s, "target": t,
                "implies": c[0], "depends_on": c[1], "part_of": c[2], "conflicts_with": c[3],
                "total": c[0] + c[1] + c[2] + c[3],
            })
        })
        .collect();

    // ── timeline
    let timeline: Vec<Value> = topic_nodes
        .iter()
        .map(|n| {
            json!({
                "repo": if n.repo.is_empty() { "Other" } else { &n.repo },
                "domain": if n.domain.is_empty() { "Other" } else { &n.domain },
                "date": n.created_at.to_string(),
                "topic": n.topic,
            })
        })
        .collect();

    // ── full node/edge arrays for graph
    // Active domain *names* — every distinct domain that has at least one
    // topic. We build stars from this set even if the DB has no kind=domain
    // rows for them (in which case we synthesise the star).
    let mut active_domain_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for n in &topic_nodes {
        let d = if n.domain.is_empty() { "Other".to_string() } else { n.domain.clone() };
        active_domain_names.insert(d);
    }

    // Build the unified list of stars (one per active domain). Real Node rows
    // win; missing names get synthesised IDs so the front-end can render them.
    let stars: Vec<StarRef> = {
        let mut out: Vec<StarRef> = Vec::new();
        let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
        for dn in &domain_nodes {
            if active_domain_names.contains(&dn.topic) {
                out.push(StarRef {
                    id: dn.id.clone(),
                    name: dn.topic.clone(),
                    description: dn.description.clone(),
                    last_reviewed: dn.last_reviewed.to_string(),
                    created_at: dn.created_at.to_string(),
                });
                covered.insert(dn.topic.clone());
            }
        }
        for name in &active_domain_names {
            if !covered.contains(name) {
                out.push(StarRef {
                    id: format!("__star_{}", name),
                    name: name.clone(),
                    description: String::new(),
                    last_reviewed: String::new(),
                    created_at: String::new(),
                });
            }
        }
        out
    };

    // Deterministic spatial layout — every node gets (x, y) and bob params.
    let layout = compute_space_layout(&topic_nodes, &stars, user_id);
    let pos_of = |id: &str| layout.get(id).copied().unwrap_or_default();

    let mut nodes_json: Vec<Value> = topic_nodes
        .iter()
        .map(|n| {
            let (r, m, recall) = db.node_recall(n);
            let cls = fsrs::classify(recall);
            let reviews: Vec<Value> = db
                .get_reviews(&n.id)
                .unwrap_or_default()
                .into_iter()
                .map(|rev| {
                    json!({
                        "date": rev.reviewed_at, "question": rev.question,
                        "answer": rev.answer, "feedback": rev.feedback, "score": rev.score,
                    })
                })
                .collect();
            let p = pos_of(&n.id);
            json!({
                "id": n.id, "topic": n.topic, "kind": n.kind.as_str(),
                "domain": n.domain, "description": n.description,
                "stability": n.stability, "difficulty": n.difficulty,
                "retrievability": r, "mastery": m, "recall_now": recall,
                "classification": cls,
                "last_reviewed": n.last_reviewed.to_string(),
                "last_encountered": n.last_encountered.to_string(),
                "encounter_count": n.encounter_count,
                "review_count": n.review_count, "created_at": n.created_at.to_string(),
                "repo": if n.repo.is_empty() { "Other" } else { &n.repo },
                "repos": n.repos,
                "canonical_question": n.canonical_question,
                "canonical_answer": n.canonical_answer,
                "canonical_clue": n.canonical_clue,
                "question_bank": n.question_bank,
                "reviews": reviews,
                "x": p.x, "y": p.y, "bob_phase": p.bob_phase, "bob_speed": p.bob_speed,
            })
        })
        .collect();

    for s in &stars {
        let p = pos_of(&s.id);
        nodes_json.push(json!({
            "id": s.id, "topic": s.name, "kind": "domain",
            "domain": s.name, "description": s.description,
            "stability": 999.0, "difficulty": 0.0,
            "retrievability": 1.0, "classification": "known",
            "last_reviewed": s.last_reviewed,
            "review_count": 0, "created_at": s.created_at,
            "x": p.x, "y": p.y, "bob_phase": 0.0, "bob_speed": 0.0,
        }));
    }

    nodes_json.push(json!({
        "id": user_id, "topic": user_name, "kind": "user",
        "domain": "", "description": "Your personal knowledge graph",
        "stability": 999.0, "difficulty": 0.0,
        "retrievability": 1.0, "classification": "known",
        "last_reviewed": "", "review_count": 0, "created_at": "",
        "x": 0.0, "y": 0.0, "bob_phase": 0.0, "bob_speed": 0.0,
    }));

    let topic_ids: std::collections::HashSet<&str> =
        topic_nodes.iter().map(|n| n.id.as_str()).collect();
    let star_ids: std::collections::HashSet<&str> =
        stars.iter().map(|s| s.id.as_str()).collect();
    let all_visible: std::collections::HashSet<&str> = topic_ids
        .iter()
        .chain(star_ids.iter())
        .chain(std::iter::once(&user_id))
        .copied()
        .collect();

    let edges_json: Vec<Value> = {
        let mut ej: Vec<Value> = all_edges
            .into_iter()
            .filter(|e| {
                all_visible.contains(e.source_id.as_str())
                    && all_visible.contains(e.target_id.as_str())
            })
            .map(|e| {
                json!({
                    "id": e.id, "source": e.source_id, "target": e.target_id,
                    "kind": e.kind.as_str(), "description": e.description, "strength": e.strength,
                })
            })
            .collect();

        for s in &stars {
            ej.push(json!({
                "id": format!("{}-user", s.id),
                "source": s.id, "target": user_id,
                "kind": "part_of", "description": "", "strength": 1.0,
            }));
        }
        ej
    };

    // ── summaries
    let summaries_dir = cfg.rocky_dir.join("summaries");
    let mut repo_summaries: Map<String, Value> = Map::new();
    if let Ok(entries) = std::fs::read_dir(&summaries_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("txt") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(summary) = std::fs::read_to_string(&path) {
                        repo_summaries
                            .insert(stem.to_string(), Value::String(summary.trim().to_string()));
                    }
                }
            }
        }
    }

    let atrophy = atrophy_score(db, &topic_nodes);
    let domain_health = domain_health_breakdown(db, &topic_nodes);
    let due_for_review = due_for_review_list(db, &topic_nodes, 8);
    let recently_added = recently_added_list(&topic_nodes, 8);

    // Take a daily snapshot of both IQs for trend tracking. Idempotent —
    // the latest call within a day overwrites the same row.
    let rocky_iq = ((1.0 - atrophy) * 100.0).round().clamp(0.0, 100.0) as i64;
    let prompt_iq_now = promptiq::current_iq().ok().map(|r| r.current);
    let _ = db.record_iq_snapshot(rocky_iq, prompt_iq_now);
    let iq_history = db.recent_iq_snapshots(30).unwrap_or_default();
    // Week-over-week delta for Rocky IQ — current vs the snapshot ~7 days ago
    // (or the oldest available, whichever is later).
    let rocky_iq_delta: i64 = if iq_history.len() >= 2 {
        let cur = iq_history.last().map(|s| s.rocky_iq).unwrap_or(rocky_iq);
        let baseline_idx = iq_history.len().saturating_sub(8);
        let baseline = iq_history[baseline_idx].rocky_iq;
        cur - baseline
    } else { 0 };

    Ok(json!({
        "userName": user_name,
        "domains": ["Language","Database","Auth","API","Frontend","DevOps","Architecture","Performance","Security","Testing","Tooling","Data","Other"],
        "projects": projects,
        "flows": flows,
        "timeline": timeline,
        "nodes": nodes_json,
        "edges": edges_json,
        "summaries": repo_summaries,
        // Dashboard tab
        "atrophyScore": atrophy,
        "rockyIq": rocky_iq,
        "rockyIqDelta": rocky_iq_delta,
        "iqHistory": iq_history,
        "domainHealth": domain_health,
        "dueForReview": due_for_review,
        "recentlyAdded": recently_added,
        // Personality phrase banks for the web quiz — picks a Rocky quote
        // per score bucket, matching the CLI flow. Bool flag controls whether
        // the client shows them at all.
        "personality": crate::personality::banks_json(cfg.personality),
        // Clipboard prompt templates + LLM Copy-and-Go targets for the bank
        // "Teach me this" buttons. Variables ({topic}, {question}, …) are
        // interpolated client-side; one button is rendered per target id.
        // Two templates: student (LLM teaches user) vs teacher (user teaches
        // LLM, Feynman-style) — UI toggle picks which one fires.
        "teach": serde_json::json!({
            "student_prompt": cfg.teach.student_prompt,
            "teacher_prompt": cfg.teach.teacher_prompt,
            "targets": cfg.teach.targets,
        }),
    }))
}

/// AI Atrophy Score (0..1, higher = more atrophy among recent material).
///
/// For each non-domain node:
///   weight = max(0, 1 - days_since_created / 60)   — recent material counts more
///   decay  = max(0, 0.6 - recall_now)              — only nodes below the "known" threshold
///   contribution = weight * decay
///
/// score = (sum contribution) / (sum weight) normalized into 0..1 by /0.6.
/// `recall_now` (= retrievability × mastery) catches both "I forgot it" and
/// "I never knew it" — a topic the user just answered wrong is correctly atrophied.
fn atrophy_score(db: &crate::db::Db, nodes: &[&crate::node::Node]) -> f64 {
    let today = chrono::Local::now().date_naive();
    let mut total_weight = 0.0_f64;
    let mut total_contrib = 0.0_f64;
    for n in nodes {
        let days = (today - n.created_at).num_days() as f64;
        let weight = (1.0_f64 - (days / 60.0_f64)).max(0.0);
        if weight == 0.0 { continue; }
        let (_, _, recall) = db.node_recall(n);
        let decay = (0.6_f64 - recall).max(0.0);
        total_weight += weight;
        total_contrib += weight * decay;
    }
    if total_weight == 0.0 {
        return 0.0;
    }
    ((total_contrib / total_weight) / 0.6).clamp(0.0, 1.0)
}

fn domain_health_breakdown(db: &crate::db::Db, nodes: &[&crate::node::Node]) -> Vec<Value> {
    use std::collections::HashMap;
    #[derive(Default)] struct Bucket { sum_r: f64, n: u32, known: u32, fading: u32, gap: u32 }
    let mut by_domain: HashMap<String, Bucket> = HashMap::new();
    for n in nodes {
        let d = if n.domain.is_empty() { "Other" } else { n.domain.as_str() };
        let (_, _, recall) = db.node_recall(n);
        let cls = fsrs::classify(recall);
        let b = by_domain.entry(d.to_string()).or_default();
        b.sum_r += recall; b.n += 1;
        match cls { "known" => b.known += 1, "stale" => b.fading += 1, _ => b.gap += 1 }
    }
    let mut out: Vec<Value> = by_domain.into_iter()
        .map(|(d, b)| json!({
            "domain": d,
            "count": b.n,
            "avg_recall": if b.n > 0 { b.sum_r / b.n as f64 } else { 0.0 },
            "known": b.known, "fading": b.fading, "gap": b.gap,
        }))
        .collect();
    out.sort_by(|a, b| {
        let ar = a["avg_recall"].as_f64().unwrap_or(0.0);
        let br = b["avg_recall"].as_f64().unwrap_or(0.0);
        ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn due_for_review_list(db: &crate::db::Db, nodes: &[&crate::node::Node], limit: usize) -> Vec<Value> {
    let mut scored: Vec<(f64, &crate::node::Node)> = nodes.iter()
        .map(|n| {
            let (_, _, recall) = db.node_recall(n);
            (recall, *n)
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(limit).map(|(recall, n)| json!({
        "id": n.id, "topic": n.topic, "domain": n.domain,
        "repo": if n.repo.is_empty() { "Other" } else { &n.repo },
        "recall_now": recall,
        "classification": fsrs::classify(recall),
        "review_count": n.review_count,
        "encounter_count": n.encounter_count,
        "last_reviewed": n.last_reviewed.to_string(),
        "last_encountered": n.last_encountered.to_string(),
    })).collect()
}

fn recently_added_list(nodes: &[&crate::node::Node], limit: usize) -> Vec<Value> {
    let mut by_date: Vec<&&crate::node::Node> = nodes.iter().collect();
    by_date.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    by_date.into_iter().take(limit).map(|n| json!({
        "id": n.id, "topic": n.topic, "domain": n.domain,
        "repo": if n.repo.is_empty() { "Other" } else { &n.repo },
        "review_count": n.review_count,
        "encounter_count": n.encounter_count,
        "created_at": n.created_at.to_string(),
        "kind": n.kind.as_str(),
    })).collect()
}

// ── space-map layout (deterministic) ─────────────────────────────────────────
//
// Domains sit on a circle around the user (the only thing at origin).
// Topics fill a Vogel sunflower around their parent domain. Every position is
// derived from a stable per-id hash, so the same topic lands in the same place
// across reloads — important for spatial memory while learning.

#[derive(Clone, Copy, Default)]
struct LayoutPos { x: f64, y: f64, bob_phase: f64, bob_speed: f64 }

/// A "star" — one per active domain. Real domain Node rows win; missing ones
/// get a synthesised StarRef so the front-end can still render them.
struct StarRef {
    id: String,
    name: String,
    description: String,
    last_reviewed: String,
    created_at: String,
}

fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn compute_space_layout(
    topic_nodes: &[&crate::node::Node],
    stars: &[StarRef],
    user_id: &str,
) -> std::collections::HashMap<String, LayoutPos> {
    use std::collections::HashMap;
    use std::f64::consts::{PI, TAU, FRAC_PI_2};

    let mut out: HashMap<String, LayoutPos> = HashMap::new();
    out.insert(user_id.to_string(), LayoutPos::default());

    // Group topics by domain (empty → "Other") and sort each cluster by id-hash.
    let mut by_domain: HashMap<String, Vec<&crate::node::Node>> = HashMap::new();
    for n in topic_nodes {
        let d = if n.domain.is_empty() { "Other".to_string() } else { n.domain.clone() };
        by_domain.entry(d).or_default().push(n);
    }
    for v in by_domain.values_mut() {
        v.sort_by_key(|n| fnv1a(&n.id));
    }

    // Cluster footprint scales with topic count. Inter-domain ring radius is
    // sized so the largest cluster never touches its neighbour.
    let cluster_outer = |n: usize| -> f64 { 150.0 + 58.0 * ((n + 1) as f64).sqrt() };
    let max_topics = by_domain.values().map(|v| v.len()).max().unwrap_or(0);
    let max_outer = cluster_outer(max_topics);

    // Stable angular slot per star — alphabetical by name.
    let mut star_idx: Vec<&StarRef> = stars.iter().collect();
    star_idx.sort_by(|a, b| a.name.cmp(&b.name));
    let n_dom = star_idx.len();

    let r_dom: f64 = if n_dom == 0 {
        0.0
    } else if n_dom == 1 {
        900.0_f64.max(max_outer + 200.0)
    } else {
        let chord = max_outer * 2.0 + 120.0;
        let needed = chord / (2.0 * (PI / n_dom as f64).sin());
        900.0_f64.max(needed)
    };

    let mut domain_centre: HashMap<String, (f64, f64)> = HashMap::new();
    for (i, s) in star_idx.iter().enumerate() {
        let theta = (i as f64) / (n_dom.max(1) as f64) * TAU - FRAC_PI_2;
        let x = r_dom * theta.cos();
        let y = r_dom * theta.sin();
        out.insert(s.id.clone(), LayoutPos { x, y, bob_phase: 0.0, bob_speed: 0.0 });
        domain_centre.insert(s.name.clone(), (x, y));
    }

    // Place topics around their domain — Vogel sunflower with per-domain phase.
    let golden = PI * (3.0 - 5.0_f64.sqrt());
    for (domain, topics) in &by_domain {
        let (cx, cy) = *domain_centre.get(domain).unwrap_or(&(0.0, 0.0));
        let phase = (fnv1a(domain) % 6283) as f64 / 1000.0;
        for (i, n) in topics.iter().enumerate() {
            let r = 150.0 + 58.0 * ((i + 1) as f64).sqrt();
            let theta = (i as f64) * golden + phase;
            let h = fnv1a(&n.id);
            let bob_phase = (h % 6283) as f64 / 1000.0;
            let bob_speed = 0.30 + ((h >> 16) % 1000) as f64 / 1000.0 * 0.50;
            out.insert(n.id.clone(), LayoutPos {
                x: cx + r * theta.cos(),
                y: cy + r * theta.sin(),
                bob_phase,
                bob_speed,
            });
        }
    }

    out
}

fn build_sessions_json(db: &Db) -> Result<Value> {
    use std::collections::BTreeMap;
    let nodes = db.all_nodes()?;
    let mut by_day: BTreeMap<chrono::NaiveDate, Vec<&crate::node::Node>> = BTreeMap::new();
    for n in nodes.iter().filter(|n| !n.kind.is_domain()) {
        by_day.entry(n.created_at).or_default().push(n);
    }
    let entries: Vec<Value> = by_day.into_iter().rev().map(|(date, ns)| {
        let topics: Vec<Value> = ns.iter().map(|n| json!({
            "id": n.id, "topic": n.topic, "domain": n.domain,
            "kind": n.kind.as_str(),
            "encounter_count": n.encounter_count,
            "source_commits": n.source_commits,
            "repo": if n.repo.is_empty() { "Other" } else { &n.repo },
        })).collect();
        json!({
            "date": date.to_string(),
            "count": ns.len(),
            "topics": topics,
        })
    }).collect();
    Ok(json!({ "sessions": entries }))
}

// ── Question-bank edits ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DeleteQuestionReq {
    node_id: String,
    q_idx: usize,
    reason: String,
}

async fn delete_bank_question(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteQuestionReq>,
) -> Result<Json<Value>, StatusCode> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let node = db.get_node_by_id(&req.node_id)?
            .ok_or_else(|| anyhow::anyhow!("node not found"))?;
        let mut bank = node.question_bank.clone();
        if req.q_idx < bank.len() {
            bank.remove(req.q_idx);
            db.set_question_bank(&node.topic, &bank)?;
        }
        let reason = req.reason.trim();
        if !reason.is_empty() {
            append_feedback_entry(&format!("[card-deleted] {reason}"))?;
        }
        Ok(())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct GenerateQuestionsReq {
    node_id: String,
}

async fn generate_bank_questions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateQuestionsReq>,
) -> Result<Json<Value>, StatusCode> {
    if state.teacher.is_none() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let state2 = state.clone();
    let added = tokio::task::spawn_blocking(move || -> Result<usize> {
        let db = &state2.db;
        let teacher = state2.teacher.as_ref().unwrap();
        let node = db.get_node_by_id(&req.node_id)?
            .ok_or_else(|| anyhow::anyhow!("node not found"))?;
        // Build peer context from this node's edges so the new questions are
        // grounded in actual graph connections, not just isolated description.
        let mut peer_context = String::new();
        if let Ok(edges) = db.get_edges_for_node(&node.id) {
            for edge in edges.iter().take(8) {
                let peer_id = if edge.source_id == node.id { &edge.target_id } else { &edge.source_id };
                if let Ok(Some(peer)) = db.get_node(peer_id) {
                    peer_context.push_str(&format!(
                        "- {} ({}): {}\n",
                        peer.topic, edge.kind.as_str(), peer.description
                    ));
                }
            }
        }
        let diff_excerpt = node.contexts.join("\n");
        let new_qs = teacher.generate_question_bank(
            &node.topic,
            &node.description,
            &peer_context,
            "",
            &diff_excerpt,
        )?;
        if new_qs.is_empty() { return Ok(0); }
        let mut bank = node.question_bank.clone();
        let added = new_qs.len();
        for q in new_qs {
            if !bank.iter().any(|b| b.question == q.question) {
                bank.push(q);
            }
        }
        db.set_question_bank(&node.topic, &bank)?;
        Ok(added)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true, "added": added })))
}

// ── Feedback file ────────────────────────────────────────────────────────────

fn append_feedback_entry(line: &str) -> Result<()> {
    use std::io::Write;
    let path = crate::config::feedback_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(f, "- {stamp} · {line}")?;
    Ok(())
}

async fn read_feedback() -> Result<String, StatusCode> {
    let path = crate::config::feedback_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
struct WriteFeedbackReq {
    content: String,
}

async fn write_feedback(Json(req): Json<WriteFeedbackReq>) -> Result<Json<Value>, StatusCode> {
    let path = crate::config::feedback_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    std::fs::write(&path, req.content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct AppendFeedbackReq {
    text: String,
}

async fn append_feedback(Json(req): Json<AppendFeedbackReq>) -> Result<Json<Value>, StatusCode> {
    let text = req.text.trim();
    if text.is_empty() {
        return Ok(Json(json!({ "ok": true, "skipped": true })));
    }
    append_feedback_entry(text).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}
