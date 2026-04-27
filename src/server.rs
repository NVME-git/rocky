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
    let questions = tokio::task::spawn_blocking(move || -> Result<Vec<QuizQuestion>> {
        let mut qs = Vec::new();
        for nid in &req.node_ids {
            if let Some(node) = db.get_node_by_id(nid)? {
                let question = if !node.canonical_question.is_empty() {
                    node.canonical_question.clone()
                } else if !node.description.is_empty() {
                    format!("Explain: {} — {}", node.topic, node.description)
                } else {
                    format!("What do you know about {}?", node.topic)
                };
                qs.push(QuizQuestion {
                    node_id: nid.clone(),
                    topic: node.topic.clone(),
                    domain: node.domain.clone(),
                    description: node.description.clone(),
                    question,
                    clue: node.canonical_clue.clone(),
                    has_canonical: !node.canonical_question.is_empty(),
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
    let eval = tokio::task::spawn_blocking(move || -> Result<crate::teacher::EvalResult> {
        let teacher = state2.teacher.as_ref().unwrap();
        teacher.evaluate_answer(
            &topic,
            &question,
            &answer,
            &description,
            canonical_answer.as_deref(),
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
    let active_domains: std::collections::HashSet<&str> = topic_nodes
        .iter()
        .map(|n| n.domain.as_str())
        .filter(|d| !d.is_empty())
        .collect();

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
            json!({
                "id": n.id, "topic": n.topic, "kind": n.kind.as_str(),
                "domain": n.domain, "description": n.description,
                "stability": n.stability, "difficulty": n.difficulty,
                "retrievability": r, "mastery": m, "recall_now": recall,
                "classification": cls,
                "last_reviewed": n.last_reviewed.to_string(),
                "review_count": n.review_count, "created_at": n.created_at.to_string(),
                "repo": if n.repo.is_empty() { "Other" } else { &n.repo },
                "repos": n.repos,
                "canonical_question": n.canonical_question,
                "canonical_answer": n.canonical_answer,
                "canonical_clue": n.canonical_clue,
                "question_bank": n.question_bank,
                "reviews": reviews,
            })
        })
        .collect();

    for dn in &domain_nodes {
        if !active_domains.contains(dn.topic.as_str()) {
            continue;
        }
        nodes_json.push(json!({
            "id": dn.id, "topic": dn.topic, "kind": "domain",
            "domain": dn.topic, "description": dn.description,
            "stability": 999.0, "difficulty": 0.0,
            "retrievability": 1.0, "classification": "known",
            "last_reviewed": dn.last_reviewed.to_string(),
            "review_count": 0, "created_at": dn.created_at.to_string(),
        }));
    }

    nodes_json.push(json!({
        "id": user_id, "topic": user_name, "kind": "user",
        "domain": "", "description": "Your personal knowledge graph",
        "stability": 999.0, "difficulty": 0.0,
        "retrievability": 1.0, "classification": "known",
        "last_reviewed": "", "review_count": 0, "created_at": "",
    }));

    let topic_ids: std::collections::HashSet<&str> =
        topic_nodes.iter().map(|n| n.id.as_str()).collect();
    let domain_ids: std::collections::HashSet<&str> = domain_nodes
        .iter()
        .filter(|n| active_domains.contains(n.topic.as_str()))
        .map(|n| n.id.as_str())
        .collect();
    let all_visible: std::collections::HashSet<&str> = topic_ids
        .iter()
        .chain(domain_ids.iter())
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

        for dn in &domain_nodes {
            if !active_domains.contains(dn.topic.as_str()) {
                continue;
            }
            ej.push(json!({
                "id": format!("{}-user", dn.id),
                "source": dn.id, "target": user_id,
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
        "domainHealth": domain_health,
        "dueForReview": due_for_review,
        "recentlyAdded": recently_added,
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
        "recall_now": recall,
        "last_reviewed": n.last_reviewed.to_string(),
    })).collect()
}

fn recently_added_list(nodes: &[&crate::node::Node], limit: usize) -> Vec<Value> {
    let mut by_date: Vec<&&crate::node::Node> = nodes.iter().collect();
    by_date.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    by_date.into_iter().take(limit).map(|n| json!({
        "id": n.id, "topic": n.topic, "domain": n.domain,
        "created_at": n.created_at.to_string(),
        "kind": n.kind.as_str(),
    })).collect()
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
