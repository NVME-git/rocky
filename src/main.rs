mod config;
mod db;
mod fsrs;
mod local_log;
mod node;
mod obsidian;
mod personality;
mod server;
mod session;
mod sync;
mod teacher;
mod transcript;
mod voice;

use std::io::{self, Read as _, Write as _};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

use config::Config;
use db::Db;
use node::Kind;
use session::Session;
use teacher::{Difficulty, Teacher, TopicInfo};

const MAX_QUESTIONS: u32 = 3;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "rocky",
    about = "Rocky — Personal Knowledge Graph\n\n  You observe. Question?\n\n  Quiz yourself on what your AI agent just built so you never lose the thread.",
)]
struct Cli {
    /// Task description to analyze before starting work
    task: Option<String>,

    /// After-mode: review topics from a completed task or commit message
    #[arg(long, value_name = "MSG")]
    after: Option<String>,

    #[command(subcommand)]
    subcommand: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show PKG stats
    Stats,
    /// List all topics in your PKG
    #[command(alias = "ls")]
    List {
        /// Filter by creation date: today, yesterday, week, month, or a number of days (e.g. 7)
        #[arg(long)]
        since: Option<String>,
        /// Emit a JSON array (used by the rocky-checkpoint skill for dedup lookup)
        #[arg(long)]
        json: bool,
    },
    /// Add a topic to the PKG (low-level — invoked by the rocky-checkpoint skill).
    /// In the agent-driven workflow, the agent extracts topics from recent commits
    /// and calls this for each one. Bypasses any LLM call inside Rocky.
    AddTopic {
        /// Topic name (noun phrase, ≤8 words)
        #[arg(long)]
        name: String,
        /// One-sentence description of the topic
        #[arg(long)]
        description: String,
        /// Domain hint: Language, Database, Auth, API, Frontend, DevOps, Architecture,
        /// Performance, Security, Testing, Tooling, Data, Other
        #[arg(long, default_value = "Other")]
        domain: String,
        /// Kind: concept | pattern | implementation
        #[arg(long, default_value = "concept")]
        kind: String,
        /// Diff hunk or code fragment that grounds future quiz questions
        #[arg(long)]
        context: Option<String>,
        /// Commit SHA this topic was learned from
        #[arg(long)]
        commit: Option<String>,
        /// Initial recall score (0..1). Defaults to 0.7 (newly-introduced).
        #[arg(long, default_value = "0.7")]
        score: f64,
    },
    /// Emit the project context summary (from `rocky explore`) as JSON.
    /// Used by the rocky-checkpoint skill to ground topic extraction in the
    /// project's vocabulary.
    Context,
    /// Inspect / drain the post-commit queue. Used by the rocky-checkpoint skill.
    Checkpoint {
        #[command(subcommand)]
        action: CheckpointAction,
    },
    /// List topics due for review as JSON. Used by the rocky-quiz skill.
    Due {
        /// Maximum number of topics to surface (lowest retrievability first)
        #[arg(long, default_value = "5")]
        limit: usize,
    },
    /// Look up a topic by name and print its full record as JSON.
    /// Used by the rocky-quiz skill to fetch question banks + history.
    Topic {
        /// Topic name (substring match — must resolve to exactly one topic)
        name: String,
    },
    /// Append a single question to a topic's question_bank (used by the rocky-quiz
    /// skill so good questions persist across sessions for rotation).
    AddQuestion {
        /// Topic name (substring match — must resolve to exactly one topic)
        #[arg(long)]
        topic: String,
        /// The question
        #[arg(long)]
        question: String,
        /// Ideal answer (≤200 chars)
        #[arg(long)]
        answer: String,
        /// Short clue / hint shown when the user asks for one
        #[arg(long, default_value = "")]
        clue: String,
    },
    /// Hard-delete a topic from the PKG (used by the rocky-quiz skill's [x] action).
    DeleteTopic {
        /// Topic name (substring match — must resolve to exactly one topic)
        topic: String,
    },
    /// Record a quiz outcome for a topic. Used by the rocky-quiz skill.
    Review {
        /// Topic name (substring match) or topic id
        topic: String,
        /// Score in [0, 1] — 1.0 = perfect recall, 0.0 = total miss
        #[arg(long)]
        score: f64,
        /// The question that was asked
        #[arg(long)]
        question: Option<String>,
        /// The answer the user gave (recorded for audit)
        #[arg(long)]
        answer: Option<String>,
        /// Short feedback line shown to the user (recorded for audit)
        #[arg(long)]
        feedback: Option<String>,
    },
    /// Install a hook (git post-commit by default)
    Install {
        #[command(subcommand)]
        target: Option<HookTarget>,
    },
    /// Remove a hook (git post-commit by default)
    Uninstall {
        #[command(subcommand)]
        target: Option<HookTarget>,
    },
    /// Show all stored context for a topic: description, source commits, contexts, question bank
    #[command(alias = "show")]
    Inspect {
        /// Topic name (partial match)
        topic: String,
    },
    /// Show active configuration
    Config,
    /// On-demand quiz — general review, or search for specific topics
    Quiz {
        /// Search for specific topics to quiz (substring match). Omit for full review.
        topic: Option<String>,
        /// Look back N hours for prompt context (default: 24)
        #[arg(long, default_value = "24")]
        hours: u32,
        /// Voice mode: reads questions aloud (TTS) and transcribes mic input for answers.
        /// Requires [voice] provider = "whisper-cpp" in config and arecord (Linux) or rec/sox (macOS).
        #[arg(long)]
        voice: bool,
    },
    /// Analyze a git diff and quiz on topics found in the code changes
    Diff {
        /// Git ref to diff against HEAD (e.g. HEAD~1, main). Defaults to last commit.
        #[arg(value_name = "REF")]
        git_ref: Option<String>,
        /// Analyze staged changes instead of a commit
        #[arg(long)]
        staged: bool,
    },
    /// Search for and delete topics from your PKG
    Delete {
        /// Search query (substring match on topic name and description)
        query: Option<String>,
        /// Delete topics added on or after this date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        since: Option<String>,
        /// Delete topics added on or before this date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        before: Option<String>,
    },
    /// Export all PKG topics to your PKG directory (Obsidian-compatible markdown)
    Export,
    /// Show topics queued in this project (skipped during sessions, not yet in PKG)
    Queue,
    /// Show recent prompts logged in this project
    Logs,
    /// Claude Code hook — reads JSON from stdin, logs prompt (non-blocking)
    Hook,
    /// Commit PKG + pkg.json to git; optionally push or initialise the repo
    Sync {
        /// Initialise git repo and optionally set a remote URL
        #[arg(long, value_name = "REMOTE_URL")]
        init: Option<Option<String>>,
        /// Commit pending changes and push to configured remote
        #[arg(long)]
        push: bool,
        /// Show PKG git status
        #[arg(long)]
        status: bool,
    },
    /// Rebuild graph.db from pkg/pkg.json (use after cloning on a new machine)
    Restore,
    /// Assign taxonomy domains to existing undomained topics via LLM
    Classify,
    /// List all edges in the PKG (implication graph)
    Edges {
        /// Show edge stats summary instead of full list
        #[arg(long)]
        stats: bool,
    },
    /// Open interactive knowledge graph in the browser
    View,
    /// Scan git history and add topics to your PKG without interactive Q&A
    Backfill {
        /// Include commits from all authors (default: current user only)
        #[arg(long)]
        all_authors: bool,
        /// Maximum number of commits to scan (default: all)
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
        /// Retroactively generate missing clues for nodes that already have canonical Q&A
        #[arg(long)]
        fill_clues: bool,
        /// Retroactively generate question_bank for nodes that don't have one yet
        #[arg(long)]
        fill_question_bank: bool,
    },
    /// Build a project context summary from CLAUDE.md, README, docs, and recent commits.
    /// Used as grounding context for question generation. Run once when adding Rocky to a project.
    Explore {
        /// Re-summarise even if a recent context exists
        #[arg(long)]
        force: bool,
        /// Suppress output (used by Stop hook auto-refresh)
        #[arg(long)]
        quiet: bool,
        /// Print the stored project context for the current dir without regenerating
        #[arg(long)]
        show: bool,
    },
    /// Process queued commits + Claude Code session transcript at session end.
    /// Called by the Claude Code Stop hook. Generates rich nodes + question bank.
    SessionEnd {
        /// Look back N hours for transcript activity (default: 6)
        #[arg(long, default_value = "6")]
        hours: u32,
        /// Suppress output
        #[arg(long)]
        quiet: bool,
    },
    /// Silently queue the latest commit's diff for later batch processing.
    /// Intended for the git post-commit hook in Claude-aware queue mode.
    /// No LLM call. Stop hook (`rocky session-end`) does the enrichment.
    PostCommit,
    /// Find and interactively merge near-duplicate topics in your PKG.
    /// Scans all topics for word-overlap candidates, then lets you decide which to keep.
    Dedupe {
        /// Preview candidates without making any changes.
        #[arg(long)]
        dry_run: bool,
        /// Pre-filter candidates using the LLM before showing them to you (slower but more precise).
        #[arg(long)]
        auto: bool,
    },
}

#[derive(Subcommand)]
enum HookTarget {
    /// Git post-commit hook — runs `rocky diff` after every commit (default)
    Git,
    /// Claude Code hook — logs prompts to `.rocky` for `rocky quiz` to review
    Claude,
    /// Enable prompt logging for this project without the git hook
    Prompt,
    /// Claude Code Stop hook — runs `rocky session-end` when a session closes
    Stop,
    /// Install the rocky-checkpoint and rocky-quiz Claude Code skills.
    /// In the agent-driven workflow these replace the Stop-hook → Ollama path.
    Skills,
    /// Full Claude-aware install: prompt logging + Stop hook + queue-mode git hook
    ClaudeAll,
}

#[derive(Subcommand)]
enum CheckpointAction {
    /// Print the queued commits + their diffs as JSON. Read-only.
    Diff,
    /// Drain the queue for this project (call after extraction succeeds).
    Mark,
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {e:#}", "error:".truecolor(226, 75, 74).bold());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Hook subcommand runs before config/db setup — must be non-blocking
    if matches!(cli.subcommand, Some(Cmd::Hook)) {
        return run_hook();
    }

    let cfg = Config::load()?;
    let db = Db::open(&cfg.db_path, &cfg.pkg_dir)?;
    let p = personality::Personality::new(cfg.personality);

    match cli.subcommand {
        Some(Cmd::Install { target }) => {
            match target.unwrap_or(HookTarget::Git) {
                HookTarget::Git => {
                    let (ok, msg) = install_git_hook()?;
                    if ok {
                        p.banner();
                        println!("  {} {msg}", "✓".truecolor(29, 158, 117));
                        println!();
                        println!("  {}", "Rocky will run after every commit in this repo.".dimmed());
                        println!("  {}", "Use  rocky quiz  for an on-demand session anytime.".dimmed());
                    } else {
                        println!("  {} {msg}", "✗".truecolor(226, 75, 74));
                    }
                }
                HookTarget::Claude => {
                    let (ok, msg) = install_claude_hook()?;
                    if ok {
                        p.banner();
                        println!("  {} {msg}", "✓".truecolor(29, 158, 117));
                        println!();
                        println!("  {}", "Rocky will silently log every Claude Code prompt.".dimmed());
                        println!("  {}", "Run  rocky install prompt  in each project to enable logging there.".dimmed());
                        println!("  {}", "Run  rocky quiz  to review topics from recent sessions.".dimmed());
                    } else {
                        println!("  {} {msg}", "✗".truecolor(226, 75, 74));
                    }
                }
                HookTarget::Prompt => {
                    let (ok, msg) = local_log::install_prompt_marker()?;
                    if ok {
                        println!("  {} {msg}", "✓".truecolor(29, 158, 117));
                        println!("  {}", "Prompt logging enabled for this project.".dimmed());
                        println!("  {}", "Run  rocky quiz  after a Claude Code session to review topics.".dimmed());
                    } else {
                        println!("  {} {msg}", "✗".truecolor(226, 75, 74));
                    }
                }
                HookTarget::Stop => {
                    let (ok, msg) = install_claude_stop_hook()?;
                    let glyph = if ok { "✓".truecolor(29, 158, 117) } else { "✗".truecolor(226, 75, 74) };
                    println!("  {glyph} {msg}");
                    if ok {
                        println!("  {}", "Rocky will batch-process commits + transcript when each Claude Code session ends.".dimmed());
                    }
                }
                HookTarget::Skills => {
                    p.banner();
                    let results = install_claude_skills()?;
                    for (ok, msg) in &results {
                        let glyph = if *ok { "✓".truecolor(29, 158, 117) } else { "·".dimmed() };
                        println!("  {glyph} {msg}");
                    }
                    println!();
                    println!("  {}", "In Claude Code, type  /rocky-checkpoint  after a commit to extract topics.".dimmed());
                    println!("  {}", "Type  /rocky-quiz  any time to drill the weakest topics.".dimmed());
                }
                HookTarget::ClaudeAll => {
                    p.banner();
                    println!("  {}", "Installing Claude-aware integration (skill-driven default):".dimmed());

                    let (ok1, msg1) = install_claude_hook()?;
                    println!("    {} {msg1}", if ok1 { "✓".truecolor(29, 158, 117) } else { "·".dimmed() });

                    let (ok3, msg3) = install_git_hook_queue_mode()?;
                    println!("    {} {msg3}", if ok3 { "✓".truecolor(29, 158, 117) } else { "·".dimmed() });

                    let (ok4, msg4) = local_log::install_prompt_marker()?;
                    println!("    {} {msg4}", if ok4 { "✓".truecolor(29, 158, 117) } else { "·".dimmed() });

                    for (ok, msg) in install_claude_skills()? {
                        println!("    {} {msg}", if ok { "✓".truecolor(29, 158, 117) } else { "·".dimmed() });
                    }

                    println!();
                    println!("  {}", "Next: run  rocky explore  to build the project context summary.".dimmed());
                    println!("  {}", "Then in Claude Code:  /rocky-checkpoint  after each commit, or  /rocky-quiz  any time.".dimmed());
                    println!("  {}", "(For legacy auto-extraction at every Claude turn:  rocky install stop)".dimmed());
                }
            }
        }
        Some(Cmd::Uninstall { target }) => {
            match target.unwrap_or(HookTarget::Git) {
                HookTarget::Git => {
                    let (ok, msg) = uninstall_git_hook()?;
                    if ok {
                        println!("  {} {msg}", "✓".truecolor(29, 158, 117));
                    } else {
                        println!("  {} {msg}", "✗".truecolor(226, 75, 74));
                    }
                }
                HookTarget::Claude => {
                    let (ok, msg) = uninstall_claude_hook()?;
                    if ok {
                        println!("  {} {msg}", "✓".truecolor(29, 158, 117));
                    } else {
                        println!("  {} {msg}", "✗".truecolor(226, 75, 74));
                    }
                }
                HookTarget::Prompt => {
                    let (ok, msg) = local_log::uninstall_prompt_marker()?;
                    if ok {
                        println!("  {} {msg}", "✓".truecolor(29, 158, 117));
                    } else {
                        println!("  {} {msg}", "✗".truecolor(226, 75, 74));
                    }
                }
                HookTarget::Stop => {
                    let (ok, msg) = uninstall_claude_stop_hook()?;
                    let glyph = if ok { "✓".truecolor(29, 158, 117) } else { "✗".truecolor(226, 75, 74) };
                    println!("  {glyph} {msg}");
                }
                HookTarget::Skills => {
                    for (ok, msg) in uninstall_claude_skills()? {
                        let glyph = if ok { "✓".truecolor(29, 158, 117) } else { "·".dimmed() };
                        println!("  {glyph} {msg}");
                    }
                }
                HookTarget::ClaudeAll => {
                    let _ = uninstall_claude_hook()?;
                    let _ = uninstall_claude_stop_hook()?;
                    let _ = uninstall_git_hook()?;
                    let _ = local_log::uninstall_prompt_marker()?;
                    let _ = uninstall_claude_skills()?;
                    println!("  {} Claude integration removed.", "✓".truecolor(29, 158, 117));
                }
            }
        }
        Some(Cmd::Config) => cfg.show(),
        Some(Cmd::Quiz { topic, hours, voice }) => {
            let teacher = make_teacher(&cfg)?;
            let cli_voice: Option<voice::CliVoice> = if voice {
                match voice::make_stt(&cfg)? {
                    Some(stt) => {
                        if !voice::recorder_available() {
                            eprintln!(
                                "  {} No recorder found — install arecord (Linux: sudo apt install alsa-utils) \
                                 or rec (macOS: brew install sox).",
                                "!".truecolor(239, 159, 39)
                            );
                            None
                        } else {
                            Some(voice::CliVoice::new(stt))
                        }
                    }
                    None => {
                        eprintln!(
                            "  {} Voice is off — set [voice] provider = \"whisper-cpp\" in ~/.config/rocky/config.toml.",
                            "!".truecolor(239, 159, 39)
                        );
                        None
                    }
                }
            } else {
                None
            };
            let session = Session::new(
                Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                cfg.daily_budget,
                cfg.min_gap_minutes,
            );
            if let Some(query) = topic {
                run_quiz_topic(&db, &teacher, &session, &p, &query, &cfg.edge_reuse, cli_voice.as_ref())?;
            } else {
                run_quiz(&db, &teacher, &session, &p, hours, &cfg.edge_reuse, cli_voice.as_ref())?;
            }
            auto_sync(&db, &cfg);
        }
        Some(Cmd::Dedupe { dry_run, auto }) => {
            let teacher = make_teacher(&cfg)?;
            run_dedupe(&db, &teacher, dry_run, auto)?;
        }
        Some(Cmd::Inspect { topic }) => inspect_topic(&db, &topic)?,
        Some(Cmd::Stats) => show_stats(&db, &cfg, &p)?,
        Some(Cmd::List { since, json }) => {
            if json {
                emit_topics_json(&db, since.as_deref())?;
            } else {
                list_topics(&db, since.as_deref())?;
            }
        }
        Some(Cmd::AddTopic { name, description, domain, kind, context, commit, score }) => {
            run_add_topic(&db, &name, &description, &domain, &kind, context.as_deref(), commit.as_deref(), score)?;
        }
        Some(Cmd::Context) => emit_project_context_json(&db)?,
        Some(Cmd::Checkpoint { action }) => match action {
            CheckpointAction::Diff => emit_checkpoint_diff_json(&db)?,
            CheckpointAction::Mark => run_checkpoint_mark(&db)?,
        },
        Some(Cmd::Due { limit }) => emit_due_json(&db, limit)?,
        Some(Cmd::Topic { name }) => emit_topic_json(&db, &name)?,
        Some(Cmd::AddQuestion { topic, question, answer, clue }) => {
            run_add_question(&db, &topic, &question, &answer, &clue)?;
        }
        Some(Cmd::DeleteTopic { topic }) => {
            run_delete_topic(&db, &topic)?;
        }
        Some(Cmd::Review { topic, score, question, answer, feedback }) => {
            run_record_review(&db, &topic, score, question.as_deref(), answer.as_deref(), feedback.as_deref())?;
        }
        Some(Cmd::Delete { query, since, before }) => {
            delete_topics(&db, &cfg.pkg_dir, query.as_deref(), since.as_deref(), before.as_deref())?;
        }
        Some(Cmd::Export) => run_export(&db, &cfg)?,
        Some(Cmd::Queue) => run_queue()?,
        Some(Cmd::Logs) => run_logs()?,
        Some(Cmd::Diff { git_ref, staged }) => {
            let teacher = make_teacher(&cfg)?;
            let session = Session::new(
                Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                cfg.daily_budget,
                cfg.min_gap_minutes,
            );
            run_diff(&db, &teacher, &session, &p, git_ref.as_deref(), staged, &cfg.edge_reuse)?;
            auto_sync(&db, &cfg);
        }
        Some(Cmd::Hook) => unreachable!(),
        Some(Cmd::Sync { init, push, status }) => {
            run_sync(&db, &cfg, init, push, status)?;
        }
        Some(Cmd::Restore) => {
            let pkg_json = cfg.pkg_dir.join("pkg.json");
            let count = db.import_pkg_json(&pkg_json)?;
            println!("  {} Restored {count} topics from {}", "✓".truecolor(29, 158, 117), pkg_json.display());
            println!("  {}", "Run  rocky export  to sync PKG files.".dimmed());
        }
        Some(Cmd::Classify) => {
            run_classify(&db, &make_teacher(&cfg)?)?;
        }
        Some(Cmd::Edges { stats }) => {
            show_edges(&db, stats)?;
        }
        Some(Cmd::View) => {
            server::run(&db, &cfg)?;
        }
        Some(Cmd::Backfill { all_authors, limit, fill_clues, fill_question_bank }) => {
            run_backfill(&db, &make_teacher(&cfg)?, &cfg, all_authors, limit, fill_clues, fill_question_bank)?;
        }
        Some(Cmd::Explore { force, quiet, show }) => {
            if show {
                show_project_context(&db)?;
            } else {
                run_explore(&db, &make_teacher(&cfg)?, force, quiet)?;
            }
        }
        Some(Cmd::SessionEnd { hours, quiet }) => {
            run_session_end(&db, &make_teacher(&cfg)?, &cfg, hours, quiet)?;
        }
        Some(Cmd::PostCommit) => {
            run_post_commit(&db)?;
        }
        None => {
            if let Some(msg) = cli.after {
                let teacher = make_teacher(&cfg)?;
                let session = Session::new(
                    Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                    cfg.daily_budget,
                    cfg.min_gap_minutes,
                );
                run_task(&db, &teacher, &session, &p, &msg, "after", &cfg.edge_reuse)?;
                auto_sync(&db, &cfg);
            } else if let Some(task) = cli.task {
                let teacher = make_teacher(&cfg)?;
                let session = Session::new(
                    Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                    cfg.daily_budget,
                    cfg.min_gap_minutes,
                );
                run_task(&db, &teacher, &session, &p, &task, "manual", &cfg.edge_reuse)?;
                auto_sync(&db, &cfg);
            } else {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
            }
        }
    }

    Ok(())
}

fn make_teacher(cfg: &Config) -> Result<Teacher> {
    if cfg.privacy_strict && cfg.llm_provider != "ollama" {
        anyhow::bail!(
            "privacy.strict = true forbids non-local LLM providers, but provider = \"{}\".\n  \
             Either switch to Ollama in your config, or disable privacy.strict.",
            cfg.llm_provider
        );
    }
    if cfg.llm_provider == "ollama" {
        Ok(Teacher::ollama(cfg.ollama_base_url.clone(), cfg.llm_model.clone()))
    } else {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY not set — add it to .env or your environment")?;
        Ok(Teacher::claude(key, cfg.llm_model.clone()))
    }
}

// ── commands ──────────────────────────────────────────────────────────────────

fn show_stats(db: &Db, cfg: &Config, p: &personality::Personality) -> Result<()> {
    print_header();
    let (total, known, stale, gaps) = db.summary()?;
    println!("\n  Total topics:  {total}");
    println!("  {}", format!("Known:         {known}").truecolor(29, 158, 117));
    println!("  {}", format!("Fading:        {stale}").truecolor(239, 159, 39));
    println!("  {}", format!("Gaps/weak:     {gaps}").truecolor(226, 75, 74));

    // Surface the single weakest topic so the user has an obvious next action.
    let mut weakest: Option<(f64, String)> = None;
    for n in db.all_nodes()?.iter().filter(|n| !n.kind.is_domain()) {
        let (_, _, recall) = db.node_recall(n);
        match weakest {
            None => weakest = Some((recall, n.topic.clone())),
            Some((cur, _)) if recall < cur => weakest = Some((recall, n.topic.clone())),
            _ => {}
        }
    }
    if let Some((recall, topic)) = weakest {
        println!(
            "  {}",
            format!("Weakest:       {topic}  (recall {:.0}%)", recall * 100.0).dimmed()
        );
    }

    nudge_if_context_stale(db);

    let session = Session::new(
        Db::open(&cfg.db_path, &cfg.pkg_dir)?,
        cfg.daily_budget,
        cfg.min_gap_minutes,
    );
    let budget = session.budget_remaining()?;
    let (allowed, reason) = session.can_quiz()?;
    if !allowed {
        println!("\n  {}", reason.dimmed());
    } else {
        println!(
            "\n  {}",
            format!(
                "Quiz budget: {budget}/{} remaining today  ·  provider: {} ({})",
                cfg.daily_budget, cfg.llm_provider, cfg.llm_model
            )
            .dimmed()
        );
    }
    let (edge_total, most_connected, most_count, _) = db.edge_stats()?;
    if edge_total > 0 {
        println!(
            "  {}",
            format!(
                "Edges: {edge_total} total  ·  Most connected: {most_connected} ({most_count} edges)"
            )
            .dimmed()
        );
    }

    if let Some(mood) = p.pkg_mood(known, total) {
        println!("\n  {mood}");
    }
    println!();
    Ok(())
}

fn parse_since_days(since: &str) -> Option<i64> {
    match since.trim().to_lowercase().as_str() {
        "today"     => Some(0),
        "yesterday" => Some(1),
        "week"      => Some(7),
        "month"     => Some(30),
        other => other.strip_suffix('d').unwrap_or(other).parse::<i64>().ok(),
    }
}

fn list_topics(db: &Db, since: Option<&str>) -> Result<()> {
    print_header();
    let cutoff: Option<chrono::NaiveDate> = since.and_then(|s| parse_since_days(s)).map(|days| {
        chrono::Local::now().date_naive() - chrono::Duration::days(days)
    });

    let nodes: Vec<_> = db.all_nodes()?
        .into_iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| cutoff.map_or(true, |c| n.created_at >= c))
        .collect();

    if nodes.is_empty() {
        if let Some(s) = since {
            println!("\n  No topics added matching --since {s}.");
        } else {
            println!("\n  PKG is empty. Run a task to populate it.");
        }
        return Ok(());
    }

    println!(
        "\n  {:<35} {:<14} {:<14} {:<14} {:<7} {:<7} {:<7} {}",
        "Topic", "Domain", "Kind", "Recall", "Stab", "Diff", "Reviews", "Last Reviewed"
    );
    println!("  {}", "─".repeat(115).dimmed());

    // Sort by recall_now descending (most-recalled at top, gaps at bottom).
    let mut scored: Vec<_> = nodes
        .into_iter()
        .map(|n| {
            let (_, _, recall) = db.node_recall(&n);
            (recall, n)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (recall, node) in scored {
        let cls = fsrs::classify(recall);
        let filled = (recall * 10.0) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
        let topic_str = &node.topic[..node.topic.len().min(34)];
        let domain_str = if node.domain.is_empty() { "—" } else { &node.domain };
        let domain_str = &domain_str[..domain_str.len().min(13)];
        let stab_str = format!("{:.1}d", node.stability);
        let diff_str = format!("{:.2}", node.difficulty);
        let line = format!(
            "  {:<35} {:<14} {:<14} {} {:.0}%  {:<7} {:<7} {:<7} {}",
            topic_str,
            domain_str,
            node.kind.as_str(),
            bar,
            recall * 100.0,
            stab_str,
            diff_str,
            node.review_count,
            node.last_reviewed
        );
        let colored = match cls {
            "known" => line.truecolor(29, 158, 117).to_string(),
            "stale" => line.truecolor(239, 159, 39).to_string(),
            _ => line.truecolor(226, 75, 74).to_string(),
        };
        println!("{colored}");
    }

    println!(
        "\n  {}",
        "Stab = days until recall hits 90%  ·  Diff = topic difficulty for you (0=easy, 1=hard)  ·  Reviews = times quizzed".dimmed()
    );
    println!();
    Ok(())
}

fn inspect_topic(db: &Db, query: &str) -> Result<()> {
    let nodes = db.all_nodes()?;
    let q = query.to_lowercase();
    let matches: Vec<_> = nodes.iter().filter(|n| n.topic.to_lowercase().contains(&q)).collect();

    if matches.is_empty() {
        println!("\n  No topic matching '{query}'.");
        return Ok(());
    }
    if matches.len() > 1 {
        println!("\n  Multiple matches — be more specific:");
        for n in &matches {
            println!("    · {}", n.topic);
        }
        return Ok(());
    }

    let n = matches[0];
    let div = "─".repeat(60);

    println!("\n  {}", div.dimmed());
    println!("  {} {}", "◆".truecolor(29, 158, 117), n.topic.bold());
    println!("  {}", div.dimmed());

    println!("\n  {:<18} {}", "Kind:".dimmed(), n.kind.as_str());
    println!("  {:<18} {}", "Domain:".dimmed(), if n.domain.is_empty() { "—" } else { &n.domain });
    println!("  {:<18} {}", "Created:".dimmed(), n.created_at);
    println!("  {:<18} {}", "Last reviewed:".dimmed(), n.last_reviewed);
    println!("  {:<18} {}", "Reviews:".dimmed(), n.review_count);
    println!("  {:<18} {:.2}", "Difficulty:".dimmed(), n.difficulty);
    println!("  {:<18} {:.1}d", "Stability:".dimmed(), n.stability);
    println!("  {:<18} {}", "Encounters:".dimmed(), n.encounter_count);

    println!("\n  {}", "Description:".dimmed());
    println!("  {}", n.description);

    if !n.source_commits.is_empty() {
        println!("\n  {}", "Source commits:".dimmed());
        for sha in &n.source_commits {
            println!("    · {}", sha);
        }
    }

    if !n.contexts.is_empty() {
        println!("\n  {}", "Creation contexts:".dimmed());
        for ctx in &n.contexts {
            println!("    · {}", ctx);
        }
    }

    if !n.canonical_question.is_empty() {
        println!("\n  {}", "Canonical Q&A:".dimmed());
        println!("  Q: {}", n.canonical_question);
        println!("  A: {}", n.canonical_answer);
        if !n.canonical_clue.is_empty() {
            println!("  Clue: {}", n.canonical_clue.dimmed());
        }
    }

    if !n.question_bank.is_empty() {
        println!("\n  {} ({} questions)", "Question bank:".dimmed(), n.question_bank.len());
        for (i, q) in n.question_bank.iter().enumerate() {
            println!("\n  {}. {} {}", i + 1, q.question, format!("[asked {}×]", q.asked_count).dimmed());
            println!("     A: {}", q.answer);
            println!("     Clue: {}", q.clue.dimmed());
        }
    }

    println!("\n  {}", div.dimmed());
    println!();
    Ok(())
}

// Smaller diff limit for backfill — keeps LLM requests fast and avoids timeouts
const MAX_BACKFILL_DIFF_CHARS: usize = 6_000;
// Pause between commits to avoid overwhelming a local Ollama instance
const BACKFILL_COMMIT_DELAY_MS: u64 = 1_000;
// Pause between edge-generation calls after all nodes are added
const BACKFILL_EDGE_DELAY_MS: u64 = 800;

fn run_backfill(db: &Db, teacher: &Teacher, cfg: &Config, all_authors: bool, limit: Option<usize>, fill_clues: bool, fill_question_bank: bool) -> Result<()> {
    use std::process::Command;

    // ── Fill-question-bank mode: retroactively generate question banks ─────────
    if fill_question_bank {
        let missing = db.nodes_missing_question_bank()?;
        if missing.is_empty() {
            println!("  {} All nodes already have a question bank.", "✓".truecolor(29, 158, 117));
            return Ok(());
        }

        // Try to ground bank generation in the project context for the current dir.
        let project_path = std::env::current_dir()
            .ok()
            .and_then(|p| p.canonicalize().ok())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let project_summary = if project_path.is_empty() {
            String::new()
        } else {
            db.get_project_context(&project_path)
                .ok()
                .flatten()
                .map(|c| c.summary)
                .unwrap_or_default()
        };

        println!("  Generating question banks for {} node(s)...\n", missing.len());
        let mut filled = 0usize;
        for node in &missing {
            print!("    {} {}... ", "·".dimmed(), node.topic);
            io::stdout().flush()?;
            // Synthesise a "diff excerpt" from any creation contexts we have on
            // record. Better than nothing for legacy nodes; richer for nodes
            // created via session-end (which stored a real commit message).
            let diff_excerpt = node.contexts.join("\n");
            match teacher.generate_question_bank(
                &node.topic,
                &node.description,
                &project_summary,
                "",            // no transcript context for backfill
                &diff_excerpt,
            ) {
                Ok(bank) if !bank.is_empty() => {
                    db.set_question_bank(&node.topic, &bank).ok();
                    filled += 1;
                    println!("{}", format!("done ({} qs)", bank.len()).truecolor(29, 158, 117));
                }
                Ok(_) => println!("{}", "empty".truecolor(239, 159, 39)),
                Err(e) => println!("{}", format!("failed ({e})").truecolor(231, 130, 132)),
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        println!("\n  {} {}/{} question banks generated.",
            "✓".truecolor(29, 158, 117), filled, missing.len());
        return Ok(());
    }

    // ── Fill-clues mode: retroactively generate clues for existing canonical nodes ──
    if fill_clues {
        let missing = db.nodes_missing_clue()?;
        if missing.is_empty() {
            println!("  {} All canonical nodes already have clues.", "✓".truecolor(29, 158, 117));
            return Ok(());
        }
        println!("  Generating clues for {} node(s)...\n", missing.len());
        let mut filled = 0usize;
        for node in &missing {
            print!("    {} {}... ", "·".dimmed(), node.topic);
            io::stdout().flush()?;
            match teacher.generate_clue(&node.topic, &node.description, &node.canonical_question) {
                Ok(clue) => {
                    db.set_canonical_qa(&node.topic, &node.canonical_question, &node.canonical_answer, &clue).ok();
                    filled += 1;
                    println!("{}", "done".truecolor(29, 158, 117));
                }
                Err(e) => println!("{}", format!("failed ({e})").truecolor(231, 130, 132)),
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        println!("\n  {} {}/{} clues generated.", "✓".truecolor(29, 158, 117), filled, missing.len());
        return Ok(());
    }

    print_header();

    // ── Seed taxonomy skeleton ────────────────────────────────────────────────
    db.ensure_taxonomy_skeleton()
        .context("Failed to seed taxonomy skeleton")?;
    println!("\n  {}", "Taxonomy skeleton ready.".dimmed());

    // ── Resolve author filter ─────────────────────────────────────────────────
    let author_email = if all_authors {
        None
    } else {
        let out = Command::new("git").args(["config", "user.email"]).output()?;
        let email = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if email.is_empty() {
            eprintln!("  {} Could not read git user.email — use --all-authors to skip filtering.", "!".truecolor(239, 159, 39));
            return Ok(());
        }
        Some(email)
    };

    // ── Get commit SHAs (oldest first) ────────────────────────────────────────
    let mut log_args = vec!["log", "--pretty=format:%H", "--reverse"];
    let author_arg;
    if let Some(ref email) = author_email {
        author_arg = format!("--author={email}");
        log_args.push(&author_arg);
    }
    let log_out = Command::new("git").args(&log_args).output()
        .context("Failed to run git log — are you in a git repository?")?;
    let all_shas: Vec<String> = String::from_utf8_lossy(&log_out.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if all_shas.is_empty() {
        let scope = author_email.as_deref().unwrap_or("all authors");
        println!("  No commits found for {scope}.");
        return Ok(());
    }

    let shas: Vec<String> = match limit {
        Some(n) => all_shas.into_iter().rev().take(n).collect::<Vec<_>>().into_iter().rev().collect(),
        None => all_shas,
    };

    let scope_desc = match (&author_email, limit) {
        (Some(e), Some(n)) => format!("last {n} commits by {e}"),
        (Some(e), None) => format!("all commits by {e}"),
        (None, Some(n)) => format!("last {n} commits"),
        (None, None) => "all commits".to_string(),
    };

    println!("  Scanning {scope_desc} ({} commits)…\n", shas.len());

    // Snapshot existing node IDs (excluding skeleton) so we know what's truly new.
    // Updated as nodes are added so later commits don't re-process the same topic.
    let mut existing_ids: std::collections::HashSet<String> = db
        .all_nodes()?
        .into_iter()
        .filter(|n| !n.kind.is_domain())
        .map(|n| Db::node_id_static(&n.topic))
        .collect();

    let repo = detect_repo_name();
    let project_summary = load_or_create_project_summary(&cfg.rocky_dir, &repo, teacher);
    if !project_summary.is_empty() {
        println!("  {}", format!("Project: {project_summary}").dimmed());
    }

    let mut added = 0usize;
    let mut skipped = 0usize;

    // ── Process commits ───────────────────────────────────────────────────────
    for (i, sha) in shas.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(BACKFILL_COMMIT_DELAY_MS));
        }

        let short = &sha[..7];
        let cdate = commit_date(sha);

        let msg_out = Command::new("git")
            .args(["log", "-1", "--pretty=%B", sha])
            .output()?;
        let commit_msg = String::from_utf8_lossy(&msg_out.stdout).trim().to_string();
        let commit_label = commit_msg.lines().next().unwrap_or(short).to_string();

        let raw_diff = {
            let diff_out = Command::new("git")
                .args(["diff", &format!("{sha}^"), sha, "--"])
                .output()?;
            if diff_out.status.success() {
                String::from_utf8_lossy(&diff_out.stdout).to_string()
            } else {
                let show_out = Command::new("git")
                    .args(["show", "--format=", sha])
                    .output()?;
                String::from_utf8_lossy(&show_out.stdout).to_string()
            }
        };

        if raw_diff.trim().is_empty() {
            continue;
        }

        let diff = truncate_to(raw_diff.trim(), MAX_BACKFILL_DIFF_CHARS);

        print!("  [{}/{}] {short} {}", i + 1, shas.len(), commit_label.dimmed());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let topics = match teacher.extract_topics_from_diff(&commit_msg, &diff) {
            Ok(t) => t,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(2));
                match teacher.extract_topics_from_diff(&commit_msg, &diff) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!(" — failed: {e}");
                        continue;
                    }
                }
            }
        };

        let new_topics: Vec<_> = topics
            .iter()
            .filter(|t| !existing_ids.contains(&Db::node_id_static(&t.topic)))
            .collect();

        if new_topics.is_empty() {
            println!(" {}", "— no new topics".dimmed());
            skipped += topics.len();
            continue;
        }

        println!(" — {} new", new_topics.len());

        // Insert nodes with commit date and repo tag
        let mut commit_new: Vec<(String, String)> = Vec::new();
        for t in &new_topics {
            db.add_or_update(
                &t.topic,
                0.5,
                &Kind::from_str(&t.kind),
                &t.domain,
                &t.description,
                &commit_label,
                cdate,
                &repo,
                cdate,
            )?;
            // Mark as seen so later commits in this run don't re-process the same topic
            existing_ids.insert(Db::node_id_static(&t.topic));
            println!("    {} {}", "+".truecolor(29, 158, 117), t.topic);

            // Pre-generate question + ideal answer + clue from diff context (silent on failure)
            if let Ok((q, a, clue)) = teacher.generate_question_and_answer(
                &t.topic, &t.description, &commit_msg, &diff, &project_summary,
            ) {
                db.set_canonical_qa(&t.topic, &q, &a, &clue).ok();
            }

            commit_new.push((t.topic.clone(), t.description.clone()));
            added += 1;
        }
        skipped += topics.len() - new_topics.len();

        println!("    {} linking…", "◈".truecolor(6, 182, 212));
        generate_edges_for_new_nodes(db, teacher, &commit_new, BACKFILL_EDGE_DELAY_MS);
    }

    println!();
    println!(
        "  {} Added {} new topic{} · {} already in PKG",
        "✓".truecolor(29, 158, 117),
        added,
        if added == 1 { "" } else { "s" },
        skipped
    );
    if added > 0 {
        println!("  {}", "Run  rocky quiz  to start reviewing them.".dimmed());
    }

    Ok(())
}

fn truncate_to(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let truncated = &s[..max_chars];
    let end = truncated.rfind('\n').unwrap_or(max_chars);
    format!("{}\n\n[... truncated ...]", &s[..end])
}


/// Called after all user-facing Q&A is done — generates implication edges silently.
/// Errors are logged as warnings; they never surface to the user or abort anything.
fn generate_edges_for_new_nodes(db: &Db, teacher: &Teacher, new_topics: &[(String, String)], delay_ms: u64) {
    use db::{Edge, EdgeKind};

    let existing = match db.all_nodes() {
        Ok(nodes) => nodes,
        Err(_) => return,
    };

    // Build the existing-node list: exclude newly added topics and domain skeleton nodes —
    // domain edges are created deterministically below, not via LLM
    let existing_summaries: Vec<(String, String)> = existing
        .iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| !new_topics.iter().any(|(t, _)| t == &n.topic))
        .take(30)
        .map(|n| (n.topic.clone(), n.description.clone()))
        .collect();

    for (i, (topic, description)) in new_topics.iter().enumerate() {
        if delay_ms > 0 && i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        // ── Free edge: topic → its domain skeleton node ───────────────────────
        if let Ok(Some(topic_node)) = db.get_node(topic) {
            if !topic_node.domain.is_empty() {
                if let Ok(Some(domain_node)) = db.get_node(&topic_node.domain) {
                    if domain_node.kind.is_domain() {
                        let kind = db::EdgeKind::PartOf;
                        if !db.edge_exists(&topic_node.id, &domain_node.id, &kind).unwrap_or(true) {
                            let edge_id = format!("{}-{}-part_of", topic_node.id, domain_node.id);
                            let _ = db.insert_edge(&Edge {
                                id: edge_id,
                                source_id: topic_node.id.clone(),
                                target_id: domain_node.id.clone(),
                                kind,
                                description: format!("{topic} is a topic within the {d} domain.", d = topic_node.domain),
                                strength: 1.0,
                                created_at: chrono::Local::now().naive_local().to_string(),
                                last_fired: None,
                                last_fired_session: None,
                            });
                        }
                    }
                }
            }
        }

        // ── LLM edges: cross-topic relationships ──────────────────────────────
        let generated = match teacher.generate_edges(topic, description, &existing_summaries) {
            Ok(edges) => edges,
            Err(e) => {
                eprintln!("  [rocky] edge generation warning: {e}");
                continue;
            }
        };

        // Resolve source node ID
        let source_id = match db.get_node(topic) {
            Ok(Some(n)) => n.id,
            _ => continue,
        };

        for edge in generated.iter().filter(|e| e.strength >= 0.6).take(4) {
            // Match target topic name to a node in the DB
            let target_node = match db.get_node(&edge.target) {
                Ok(Some(n)) => n,
                _ => continue,
            };

            let kind = EdgeKind::from_str(&edge.kind);

            // Dedup: skip if this pair+kind already exists
            if db.edge_exists(&source_id, &target_node.id, &kind).unwrap_or(true) {
                continue;
            }

            let edge_id = format!(
                "{}-{}-{}",
                source_id.replace(' ', "-"),
                target_node.id.replace(' ', "-"),
                kind.as_str()
            );

            let e = Edge {
                id: edge_id,
                source_id: source_id.clone(),
                target_id: target_node.id.clone(),
                kind,
                description: edge.description.clone(),
                strength: edge.strength.clamp(0.1, 1.0),
                created_at: chrono::Local::now().naive_local().to_string(),
                last_fired: None,
                last_fired_session: None,
            };

            if let Err(err) = db.insert_edge(&e) {
                eprintln!("  [rocky] failed to insert edge: {err}");
            }
        }
    }
}

fn show_edges(db: &Db, stats: bool) -> Result<()> {
    use db::EdgeKind;
    print_header();

    if stats {
        let (total, most_connected, most_count, avg_strength) = db.edge_stats()?;
        println!("\n  {} Edge Stats\n", "◈".bold().truecolor(6, 182, 212));
        println!("  {:<22} {}", "Total edges:".dimmed(), total.to_string().bold());
        if total > 0 {
            println!("  {:<22} {}", "Avg strength:".dimmed(), format!("{avg_strength:.2}").bold());
        }
        if !most_connected.is_empty() {
            println!(
                "  {:<22} {} {}",
                "Most connected:".dimmed(),
                most_connected.bold().truecolor(239, 159, 39),
                format!("({most_count} edges)").dimmed()
            );
        }

        // Breakdown by kind
        let edges = db.get_all_edges()?;
        let mut by_kind: std::collections::HashMap<String, (usize, f64)> = std::collections::HashMap::new();
        for e in &edges {
            let entry = by_kind.entry(e.kind.as_str().to_string()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += e.strength;
        }
        println!("\n  {}", "By kind:".dimmed());
        println!("  {:<22} {:<8} {}", "".dimmed(), "count".dimmed(), "avg str".dimmed());
        for kind_str in &["implies", "depends_on", "conflicts_with", "part_of"] {
            let (count, strength_sum) = by_kind.get(*kind_str).copied().unwrap_or((0, 0.0));
            let avg = if count > 0 { strength_sum / count as f64 } else { 0.0 };
            let colored_kind = match EdgeKind::from_str(kind_str) {
                EdgeKind::Implies       => kind_str.truecolor(6, 182, 212),
                EdgeKind::DependsOn     => kind_str.truecolor(239, 159, 39),
                EdgeKind::ConflictsWith => kind_str.truecolor(226, 75, 74),
                EdgeKind::PartOf        => kind_str.truecolor(29, 158, 117),
            };
            println!("    {:<20} {:<8} {}", colored_kind, count, if count > 0 { format!("{avg:.2}") } else { String::new() });
        }
        println!();
        return Ok(());
    }

    let edges = db.get_all_edges()?;

    if edges.is_empty() {
        println!(
            "\n  {}\n",
            "No edges yet. Rocky will build the implication graph as you add topics.".dimmed()
        );
        return Ok(());
    }

    println!(
        "\n  {:<30} {:<30} {:<16} {:<6}  {}",
        "SOURCE".dimmed(),
        "TARGET".dimmed(),
        "KIND".dimmed(),
        "STR".dimmed(),
        "DESCRIPTION".dimmed(),
    );
    println!("  {}", "─".repeat(110).dimmed());

    for e in &edges {
        // Resolve node IDs to topic names
        let src = db.get_node(&e.source_id)?
            .map(|n| n.topic)
            .unwrap_or_else(|| e.source_id.clone());
        let tgt = db.get_node(&e.target_id)?
            .map(|n| n.topic)
            .unwrap_or_else(|| e.target_id.clone());

        let src_str = &src[..src.len().min(28)];
        let tgt_str = &tgt[..tgt.len().min(28)];
        let desc_str = &e.description[..e.description.len().min(48)];

        let kind_colored = match e.kind {
            EdgeKind::Implies       => e.kind.as_str().truecolor(6, 182, 212).to_string(),
            EdgeKind::DependsOn     => e.kind.as_str().truecolor(239, 159, 39).to_string(),
            EdgeKind::ConflictsWith => e.kind.as_str().truecolor(226, 75, 74).to_string(),
            EdgeKind::PartOf        => e.kind.as_str().truecolor(29, 158, 117).to_string(),
        };

        println!(
            "  {:<30} {:<30} {:<25} {:.2}   {}",
            src_str,
            tgt_str,
            kind_colored,
            e.strength,
            desc_str.dimmed(),
        );
    }

    println!("\n  {} edges total\n", edges.len().to_string().bold());
    Ok(())
}

fn run_task(db: &Db, teacher: &Teacher, session: &Session, p: &personality::Personality, task: &str, mode: &str, edge_reuse: &config::EdgeReuse) -> Result<()> {
    print_header();
    p.print_rocky(false);
    println!("\n{} {task}\n", "Task:".bold());
    let repo = detect_repo_name();
    let commit_dt = head_commit_date();

    if mode == "after" && is_hotfix(task) {
        println!("  {}", "Hotfix detected — Rocky stepping back.".dimmed());
        return Ok(());
    }

    println!("{}", "Analyzing topics...".dimmed());
    let topics = teacher
        .extract_topics(task)
        .context("Failed to extract topics from task")?;

    if topics.is_empty() {
        println!("{}", "No significant topics found. Proceed freely.".truecolor(29, 158, 117));
        return Ok(());
    }

    let known_topic_names: Vec<String> = db
        .all_nodes()?
        .into_iter()
        .filter(|n| {
            fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) == "known"
        })
        .map(|n| n.topic)
        .collect();

    // Manual invocations bypass the cooldown — the user is explicitly asking to learn.
    // Automatic triggers (git hook, claude-code) respect the full cooldown + budget gate.
    let (quiz_allowed, block_reason) = if mode == "manual" {
        session.can_quiz_manual()?
    } else {
        session.can_quiz()?
    };
    let budget = if quiz_allowed { session.budget_remaining()? } else { 0 };
    let mut quizzed = 0u32;
    let mut queued: Vec<String> = Vec::new();
    let mut new_count = 0u32;
    let mut stale_count = 0u32;
    let mut newly_added: Vec<(String, String)> = Vec::new(); // (topic, description)

    for topic_info in &topics {
        let topic = &topic_info.topic;
        let node = db.get_node(topic)?;
        let classification = match &node {
            None => "new",
            Some(n) => fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)),
        };

        match classification {
            "known" => {
                let r = node
                    .as_ref()
                    .map(|n| fsrs::retrievability(n.stability, n.last_reviewed))
                    .unwrap_or(1.0);
                println!(
                    "{} {}",
                    format!("  ✓ {topic}").truecolor(29, 158, 117),
                    format!("({:.0}%)", r * 100.0).dimmed()
                );
                db.mark_encountered(topic)?;
            }
            "stale" => {
                stale_count += 1;
                let n = node.as_ref().unwrap();
                let r = fsrs::retrievability(n.stability, n.last_reviewed);
                println!(
                    "{} {}",
                    format!("  ~ {topic}").truecolor(239, 159, 39),
                    format!("(recall faded to {:.0}%)", r * 100.0).dimmed()
                );
                if quiz_allowed && quizzed < budget {
                    println!("{}", "  Refreshing...".dimmed());
                    let reminder = teacher.generate_reminder(topic, n, task)?;
                    println!("\n  {} {reminder}\n", "Rocky:".truecolor(239, 159, 39).bold());
                    db.add_or_update(
                        topic,
                        0.5,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.domain,
                        &topic_info.description,
                        task,
                        None, "", None,
                    )?;
                    session.record_quiz()?;
                    quizzed += 1;
                } else {
                    queued.push(topic.clone());
                    queue_for_later(topic, &topic_info.kind, &topic_info.description, task);
                }
            }
            _ => {
                new_count += 1;
                if quiz_allowed && quizzed < budget {
                    let (completed, node_added) =
                        run_socratic_loop(db, teacher, topic_info, task, &known_topic_names, p, edge_reuse, &repo, commit_dt, None)?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                    if node_added {
                        newly_added.push((topic.clone(), topic_info.description.clone()));
                    }
                } else {
                    queued.push(topic.clone());
                    queue_for_later(topic, &topic_info.kind, &topic_info.description, task);
                }
            }
        }
    }

    // Generate edges for newly added nodes (after all user interaction is done)
    if !newly_added.is_empty() {
        generate_edges_for_new_nodes(db, teacher, &newly_added, 0);
    }

    println!();

    if !queued.is_empty() {
        println!(
            "  {}",
            format!("Queued for next session: {}", queued.join(", ")).dimmed()
        );
    }

    if !quiz_allowed && (new_count > 0 || stale_count > 0) {
        println!("  {}", block_reason.truecolor(239, 159, 39));
    } else if new_count == 0 && stale_count == 0 {
        println!("{}", "All topics are in your PKG. You're good to go.".truecolor(29, 158, 117));
    }

    let (total, known, stale, _) = db.summary()?;
    println!(
        "  {}",
        format!("PKG: {known} known | {stale} fading | {total} total").dimmed()
    );

    Ok(())
}

/// Returns `(quiz_counted, node_added)`.
fn run_socratic_loop(
    db: &Db,
    teacher: &Teacher,
    topic_info: &TopicInfo,
    task: &str,
    known_topics: &[String],
    p: &personality::Personality,
    edge_reuse: &config::EdgeReuse,
    repo: &str,
    node_date: Option<chrono::NaiveDate>,
    cli_voice: Option<&voice::CliVoice>,
) -> Result<(bool, bool)> {
    let topic = &topic_info.topic;
    println!("\n{} New topic — {topic}", "Rocky:".truecolor(6, 182, 212).bold());
    println!("{}", format!("  {}", topic_info.description).dimmed());
    println!();

    let mut total_score = 0.0f64;
    let mut questions_asked = 0u32;
    let mut last_question = String::new();
    let mut last_answer = String::new();
    let mut fired_edge_id: Option<String> = None;

    // Edge-aware question: if this node has an edge to a well-known concept
    // that hasn't been cross-quizzed recently, use a cross-concept question.
    let mut question = 'q: {
        if let Ok(Some(node)) = db.get_node(topic) {
            if let Ok(edges) = db.get_edges_for_node(&node.id) {
                for edge in &edges {
                    // Find the peer node (the one that isn't the current topic)
                    let peer_id = if edge.source_id == node.id {
                        &edge.target_id
                    } else {
                        &edge.source_id
                    };
                    // Peer must be well-recalled (> 0.65)
                    let peer_node = match db.get_node(peer_id) {
                        Ok(Some(n)) => n,
                        _ => continue,
                    };
                    let peer_recall = fsrs::retrievability(peer_node.stability, peer_node.last_reviewed);
                    if peer_recall < 0.65 {
                        continue;
                    }
                    // Edge must not have been fired too recently (per config)
                    let recently_fired = match edge_reuse {
                        config::EdgeReuse::Off => false, // always eligible
                        config::EdgeReuse::Days(n) => {
                            let cutoff = chrono::Local::now().date_naive()
                                - chrono::Duration::days(*n as i64);
                            edge.last_fired.as_deref().map(|s| {
                                chrono::NaiveDate::parse_from_str(&s[..10.min(s.len())], "%Y-%m-%d")
                                    .map(|d| d > cutoff)
                                    .unwrap_or(false)
                            }).unwrap_or(false)
                        }
                        config::EdgeReuse::Sessions(n) => {
                            if let Some(fired_at) = edge.last_fired_session {
                                let current = db.total_quizzes().unwrap_or(0);
                                (current - fired_at) < *n as i64
                            } else {
                                false
                            }
                        }
                    };
                    if recently_fired {
                        continue;
                    }
                    // Qualifying edge found — generate cross-concept question
                    if let Ok(q) = teacher.generate_cross_concept_question(
                        topic,
                        &topic_info.description,
                        &peer_node.topic,
                        &peer_node.description,
                        edge.kind.as_str(),
                        &edge.description,
                        task,
                    ) {
                        fired_edge_id = Some(edge.id.clone());
                        break 'q q;
                    }
                }
            }
        }
        // Prefer the question_bank (rotates through least-asked), fall back to
        // canonical_question, then generate live.
        if let Ok(Some(ref n)) = db.get_node(topic) {
            if !n.question_bank.is_empty() {
                if let Some((idx, q)) = pick_least_asked(&n.question_bank) {
                    let mut bank = n.question_bank.clone();
                    bank[idx].asked_count = bank[idx].asked_count.saturating_add(1);
                    db.set_question_bank(topic, &bank).ok();
                    // Sync canonical_qa to the chosen question so answer/clue lookups below
                    // pick up the correct triple without further changes.
                    db.set_canonical_qa(topic, &q.question, &q.answer, &q.clue).ok();
                    break 'q q.question.clone();
                }
            }
            if !n.canonical_question.is_empty() {
                break 'q n.canonical_question.clone();
            }
        }
        teacher.generate_question(topic, &topic_info.description, task, known_topics, 1, Difficulty::Normal)?
    };

    // Track whether the initial question was canonical (pre-generated from diff)
    let is_canonical = db.get_node(topic).ok()
        .flatten()
        .map(|n| !n.canonical_question.is_empty() && n.canonical_question == question)
        .unwrap_or(false);

    // Load canonical answer and clue for use as evaluator reference and hint
    let (canonical_answer, canonical_clue): (Option<String>, Option<String>) = db.get_node(topic).ok()
        .flatten()
        .map(|n| (
            if n.canonical_answer.is_empty() { None } else { Some(n.canonical_answer) },
            if n.canonical_clue.is_empty() { None } else { Some(n.canonical_clue) },
        ))
        .unwrap_or((None, None));

    // Track current difficulty for adaptive re-generation
    let mut current_difficulty = Difficulty::Normal;

    while questions_asked < MAX_QUESTIONS {
        questions_asked += 1;
        let display_q = p.format_question(&question);
        let source_label = if is_canonical && questions_asked == 1 && current_difficulty == Difficulty::Normal {
            " (canonical)".dimmed().to_string()
        } else {
            " (generated)".dimmed().to_string()
        };

        // Speak question aloud in voice mode (plain text, no ANSI)
        if let Some(v) = cli_voice {
            v.speak(&question);
        }

        println!("{}{} {display_q}", format!("Q{questions_asked}.").bold(), source_label);

        let hint_line = if cli_voice.is_some() {
            "   [s] simpler  [h] harder  [c] clue  [?] explain it  [x] delete  or type / [Enter] to record:"
        } else {
            "   [s] simpler  [h] harder  [c] clue  [?] explain it  [x] delete  or type your answer:"
        };
        println!("{}", hint_line.dimmed());
        print!("   > ");
        io::stdout().flush()?;

        let answer: String = read_answer_cli(cli_voice)?;
        if answer.starts_with('\x00') {
            // EOF / error sentinel
            println!("\n   Skipped.");
            break;
        }

        // Skip / quit — queue for later without touching PKG
        if answer.is_empty() {
            if let Some(msg) = p.skipped() { println!("   {msg}"); }
            else { println!("{}", "   Skipped — topic queued for next session.".truecolor(239, 159, 39)); }
            queue_for_later(topic, topic_info.kind.as_str(), &topic_info.description, task);
            return Ok((true, false));
        }

        // Delete — hard-remove from the PKG (replaces old [i] not-relevant + [e] too-easy hacks).
        // Confirms first because this cascades reviews + contexts.
        if answer.eq_ignore_ascii_case("x") {
            print!("   Delete \"{topic}\" from your PKG? This is permanent. [y/N] ");
            io::stdout().flush()?;
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm)?;
            if confirm.trim().eq_ignore_ascii_case("y") {
                if db.delete_node_by_topic(topic).unwrap_or(false) {
                    println!("{}", format!("   Deleted: {topic}").truecolor(226, 75, 74));
                } else {
                    println!("{}", "   Topic not found in PKG (nothing deleted).".dimmed());
                }
            } else {
                println!("{}", "   Cancelled.".dimmed());
            }
            return Ok((false, false));
        }

        // Simpler — regenerate with reduced difficulty
        if answer.eq_ignore_ascii_case("s") {
            println!("{}", "   Generating a simpler question...".dimmed());
            current_difficulty = Difficulty::Simpler;
            question = teacher.generate_question(
                topic, &topic_info.description, task, known_topics, questions_asked, Difficulty::Simpler,
            ).unwrap_or(question);
            questions_asked -= 1;
            continue;
        }

        // Harder — regenerate with increased difficulty
        if answer.eq_ignore_ascii_case("h") {
            println!("{}", "   Generating a harder question...".dimmed());
            current_difficulty = Difficulty::Harder;
            question = teacher.generate_question(
                topic, &topic_info.description, task, known_topics, questions_asked, Difficulty::Harder,
            ).unwrap_or(question);
            questions_asked -= 1;
            continue;
        }

        // Clue — show precomputed clue or generate one at runtime
        if answer.eq_ignore_ascii_case("c") {
            if let Some(ref clue) = canonical_clue {
                println!("\n   {}\n", format!("Clue: {clue}").truecolor(167, 139, 250));
            } else {
                println!("{}", "   Generating clue...".dimmed());
                match teacher.generate_clue(topic, &topic_info.description, &question) {
                    Ok(clue) => println!("\n   {}\n", format!("Clue: {clue}").truecolor(167, 139, 250)),
                    Err(_) => println!("{}", "   Could not generate clue.".dimmed()),
                }
            }
            questions_asked -= 1;
            continue;
        }

        // Fire the cross-concept edge (if any) the first time the user gives a real answer
        if let Some(edge_id) = fired_edge_id.take() {
            db.fire_edge(&edge_id).ok();
        }

        // [e] too easy was removed — it was a loophole letting users mass-mark
        // topics as known without demonstrating mastery. The new metric requires
        // an actual quiz score to drive recall_now upward.

        // Explain it — show explanation, mark with low confidence
        if answer == "?" {
            println!("{}", "   Fetching explanation...".dimmed());
            let explanation = teacher.generate_explanation(
                topic,
                &topic_info.description,
                &question,
                "",
                task,
            )?;
            println!("\n   {}\n", explanation.truecolor(6, 182, 212));
            println!("{}", "   Saved to PKG — revisit before your next task.".truecolor(239, 159, 39));
            db.add_or_update(
                topic,
                0.2,
                &Kind::from_str(&topic_info.kind),
                &topic_info.domain,
                &topic_info.description,
                task,
                node_date, repo, node_date,
            )?;
            db.add_review(&Db::node_id_static(topic), &question, "", &explanation, 0.2).ok();
            return Ok((true, true));
        }

        // Normal answer — evaluate it
        last_question = question.clone();
        last_answer = answer.clone();

        println!("{}", "   Evaluating...".dimmed());
        let result = teacher.evaluate_answer(
            topic, &question, &answer, &topic_info.description,
            canonical_answer.as_deref(),
        )?;
        total_score += result.score;

        println!("\n   {}", result.feedback.truecolor(6, 182, 212));

        if result.understood {
            db.add_or_update(
                topic,
                result.score,
                &Kind::from_str(&topic_info.kind),
                &topic_info.domain,
                &topic_info.description,
                task,
                node_date, repo, node_date,
            )?;
            db.add_review(&Db::node_id_static(topic), &question, &answer, &result.feedback, result.score).ok();
            if let Some(msg) = p.correct() { println!("   {msg}"); }
            else { println!("{}", "   Added to your PKG.".truecolor(29, 158, 117)); }
            print_milestone(db, p, topic);
            return Ok((true, true));
        }

        if let Some(followup) = result.followup {
            if questions_asked < MAX_QUESTIONS {
                if let Some(msg) = p.partial() { println!("   {msg}"); }
                println!();
                question = followup;
                continue;
            }
        }
        break;
    }

    // Exhausted questions without understanding — give the full explanation
    let avg_score = total_score / questions_asked.max(1) as f64;
    println!("{}", "\n   Let me walk you through this one.\n".truecolor(239, 159, 39));
    let explanation = teacher.generate_explanation(
        topic,
        &topic_info.description,
        &last_question,
        &last_answer,
        task,
    )?;
    println!("   {}\n", explanation.truecolor(6, 182, 212));

    if let Some(msg) = p.failed() { println!("   {msg}"); }
    else if avg_score >= 0.4 {
        println!("{}", "   Added to PKG with partial confidence — you're on the right track.".truecolor(239, 159, 39));
    } else {
        println!("{}", "   Added to PKG — come back to this one.".truecolor(226, 75, 74));
    }
    db.add_or_update(
        topic,
        avg_score,
        &Kind::from_str(&topic_info.kind),
        &topic_info.domain,
        &topic_info.description,
        task,
        node_date, repo, node_date,
    )?;
    db.add_review(&Db::node_id_static(topic), &last_question, &last_answer, &explanation, avg_score).ok();
    Ok((true, true))
}

fn run_quiz_topic(db: &Db, teacher: &Teacher, session: &Session, p: &personality::Personality, query: &str, edge_reuse: &config::EdgeReuse, cli_voice: Option<&voice::CliVoice>) -> Result<()> {
    print_header();
    p.print_rocky(false);

    let matches = db.search_nodes(query)?;
    if matches.is_empty() {
        println!("\n  {} No topics found matching \"{}\".", "✗".truecolor(226, 75, 74), query);
        return Ok(());
    }

    println!("\n  Topics matching \"{}\":\n", query);
    for (i, node) in matches.iter().enumerate() {
        let r = fsrs::retrievability(node.stability, node.last_reviewed);
        let cls = fsrs::classify(r);
        let label = format!("  [{}]  {} ({:.0}%)", i + 1, node.topic, r * 100.0);
        let colored = match cls {
            "known" => label.truecolor(29, 158, 117).to_string(),
            "stale" => label.truecolor(239, 159, 39).to_string(),
            _ => label.truecolor(226, 75, 74).to_string(),
        };
        println!("{}", colored);
        println!("       {}", node.description.dimmed());
    }

    println!();
    println!(
        "  {}",
        "Enter numbers to quiz (e.g. 1 3), 'a' for all, or Enter to cancel:".dimmed()
    );
    print!("  > ");
    io::stdout().flush()?;

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Err(_) | Ok(0) => return Ok(()),
        Ok(_) => {}
    }
    let input = line.trim();
    if input.is_empty() {
        return Ok(());
    }

    let selected: Vec<&crate::node::Node> = if input.eq_ignore_ascii_case("a") {
        matches.iter().collect()
    } else {
        let mut sel = Vec::new();
        for part in input.split_whitespace() {
            if let Ok(n) = part.parse::<usize>() {
                if n >= 1 && n <= matches.len() {
                    sel.push(&matches[n - 1]);
                }
            }
        }
        sel
    };

    if selected.is_empty() {
        println!("  No valid selection.");
        return Ok(());
    }

    let known_topic_names: Vec<String> = db
        .all_nodes()?
        .into_iter()
        .filter(|n| fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) == "known")
        .map(|n| n.topic)
        .collect();

    let selected_count = selected.len();
    println!("\n  Quizzing {} topic{}.\n", selected_count, if selected_count == 1 { "" } else { "s" });

    for node in selected {
        let topic_info = TopicInfo {
            topic: node.topic.clone(),
            kind: node.kind.as_str().to_string(),
            domain: node.domain.clone(),
            description: node.description.clone(),
        };
        let context = node.contexts.first().map(|s| s.as_str()).unwrap_or("manual review");
        let (completed, _) = run_socratic_loop(db, teacher, &topic_info, context, &known_topic_names, p, edge_reuse, "", None, cli_voice)?;
        if completed {
            session.record_quiz()?;
        }
    }

    println!();
    let (total, known, stale, _) = db.summary()?;
    println!(
        "  {}",
        format!("PKG: {known} known | {stale} fading | {total} total").dimmed()
    );
    if let Some(msg) = p.session_done(selected_count as u32) { println!("  {msg}"); }
    println!();
    Ok(())
}

fn run_quiz(db: &Db, teacher: &Teacher, session: &Session, p: &personality::Personality, hours: u32, edge_reuse: &config::EdgeReuse, cli_voice: Option<&voice::CliVoice>) -> Result<()> {
    print_header();
    p.print_rocky(false);

    // Streak
    if let Ok(streak) = session.update_streak() {
        if let Some(msg) = p.streak(streak) {
            println!("  {msg}\n");
        }
    }

    // ── Step 1: Queued topics (from previous sessions that hit the cooldown) ──
    let local_log = local_log::LocalLog::open_existing();
    let queued_topics = local_log
        .as_ref()
        .and_then(|l| l.get_queued_topics().ok())
        .unwrap_or_default();

    // ── Step 2: PKG stale/gap topics ──────────────────────────────────────────
    let all_nodes: Vec<_> = db.all_nodes()?.into_iter().filter(|n| !n.kind.is_domain()).collect();
    let mut due: Vec<_> = all_nodes
        .iter()
        .filter(|n| {
            fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) != "known"
        })
        .collect();
    due.sort_by(|a, b| {
        let ra = fsrs::retrievability(a.stability, a.last_reviewed);
        let rb = fsrs::retrievability(b.stability, b.last_reviewed);
        ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── Step 3: Recent prompts for additional context ─────────────────────────
    let prompt_context: Option<String> = local_log
        .as_ref()
        .and_then(|l| l.recent_prompts(hours).ok())
        .filter(|entries| !entries.is_empty())
        .map(|entries| {
            let combined = entries
                .iter()
                .take(10)
                .map(|(p, _)| p.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            println!(
                "{}",
                format!("  Using {} recent prompt{} as context.", entries.len().min(10),
                    if entries.len() == 1 { "" } else { "s" }).dimmed()
            );
            combined
        });

    if queued_topics.is_empty() && due.is_empty() && prompt_context.is_none() {
        println!("{}", "\n  Nothing to review — all topics are solid.".truecolor(29, 158, 117));
        println!("{}", "  Run some tasks or check back later.".dimmed());
        println!();
        return Ok(());
    }

    let known_topic_names: Vec<String> = all_nodes
        .iter()
        .filter(|n| fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) == "known")
        .map(|n| n.topic.clone())
        .collect();

    // ── Step 4: Quiz queued topics first (these are overdue) ─────────────────
    if !queued_topics.is_empty() {
        println!(
            "{}",
            format!("\n  {} topic{} queued from last session:\n",
                queued_topics.len(), if queued_topics.len() == 1 { "" } else { "s" }).dimmed()
        );
        for (topic, kind, description, context) in &queued_topics {
            use crate::teacher::TopicInfo;
            let topic_info = TopicInfo {
                topic: topic.clone(),
                kind: kind.clone(),
                domain: String::new(),
                description: description.clone(),
            };
            let (completed, _) = run_socratic_loop(db, teacher, &topic_info, context, &known_topic_names, p, edge_reuse, "", None, cli_voice)?;
            if completed {
                session.record_quiz()?;
                // Remove from queue now that it has been properly reviewed
                if let Some(log) = &local_log {
                    log.remove_queued_topic(topic).ok();
                }
            }
        }
    }

    // ── Step 5: Quiz PKG-due topics ───────────────────────────────────────────
    if !due.is_empty() {
        println!(
            "{}",
            format!("\n  {} topic{} need review:\n", due.len(), if due.len() == 1 { "" } else { "s" }).dimmed()
        );
    }

    let context = prompt_context.as_deref().unwrap_or("general review");

    for node in &due {
        let topic = &node.topic;
        let r = fsrs::retrievability(node.stability, node.last_reviewed);
        let classification = fsrs::classify(r);

        if classification == "stale" {
            println!(
                "{} {}",
                format!("  ~ {topic}").truecolor(239, 159, 39),
                format!("(faded to {:.0}%)", r * 100.0).dimmed()
            );
            let reminder = teacher.generate_reminder(topic, node, context)?;
            println!("\n  {} {reminder}\n", "Rocky:".truecolor(239, 159, 39).bold());
            db.add_or_update(
                topic, 0.5, &node.kind, &node.domain, &node.description, context, None, "", None,
            )?;
            session.record_quiz()?;
        } else {
            // gap — run full Socratic loop
            use crate::teacher::TopicInfo;
            let topic_info = TopicInfo {
                topic: node.topic.clone(),
                kind: node.kind.as_str().to_string(),
            domain: node.domain.clone(),
                description: node.description.clone(),
            };
            let (completed, _) = run_socratic_loop(db, teacher, &topic_info, context, &known_topic_names, p, edge_reuse, "", None, cli_voice)?;
            if completed {
                session.record_quiz()?;
            }
        }
    }

    // ── Step 4: Extract and quiz on any prompt-derived topics not in PKG ──────
    if let Some(combined) = &prompt_context {
        println!("{}", "\n  Checking recent prompts for new topics...".dimmed());
        let prompt_topics = teacher.extract_topics(combined)?;
        let new_from_prompts: Vec<_> = prompt_topics
            .iter()
            .filter(|t| {
                matches!(db.get_node(&t.topic), Ok(None))
            })
            .collect();

        for topic_info in new_from_prompts {
            let (completed, _) = run_socratic_loop(db, teacher, topic_info, combined, &known_topic_names, p, edge_reuse, "", None, cli_voice)?;
            if completed {
                session.record_quiz()?;
            }
        }
    }

    println!();
    let (total, known, stale, _) = db.summary()?;
    println!(
        "  {}",
        format!("PKG: {known} known | {stale} fading | {total} total").dimmed()
    );
    if let Some(msg) = p.session_done(0) { println!("  {msg}"); }
    println!();
    Ok(())
}

fn delete_topics(
    db: &Db,
    pkg_dir: &std::path::Path,
    query: Option<&str>,
    since: Option<&str>,
    before: Option<&str>,
) -> Result<()> {
    // Must have at least one filter
    if query.is_none() && since.is_none() && before.is_none() {
        println!("  {} Provide a search query, --since DATE, --before DATE, or a combination.", "✗".truecolor(226, 75, 74));
        return Ok(());
    }

    // Build candidate list
    let mut matches: Vec<node::Node> = if since.is_some() || before.is_some() {
        let mut nodes = db.get_nodes_by_date(since, before)?;
        // Further filter by text query if also provided
        if let Some(q) = query {
            let q = q.to_lowercase();
            nodes.retain(|n| n.topic.to_lowercase().contains(&q) || n.description.to_lowercase().contains(&q));
        }
        nodes
    } else {
        db.search_nodes(query.unwrap_or(""))?
    };

    // Never surface domain skeleton nodes
    matches.retain(|n| !n.kind.is_domain());

    if matches.is_empty() {
        println!("  {} No topics found.", "✗".truecolor(226, 75, 74));
        return Ok(());
    }

    // Heading
    let heading = match (query, since, before) {
        (Some(q), None, None) => format!("Matches for \"{}\"", q),
        (None, Some(s), None) => format!("Topics added since {s}"),
        (None, None, Some(b)) => format!("Topics added before {b}"),
        (None, Some(s), Some(b)) => format!("Topics added between {s} and {b}"),
        (Some(q), Some(s), None) => format!("\"{}\" added since {s}", q),
        (Some(q), None, Some(b)) => format!("\"{}\" added before {b}", q),
        (Some(q), Some(s), Some(b)) => format!("\"{}\" added between {s} and {b}", q),
        (None, None, None) => unreachable!(),
    };
    println!("\n  {}:\n", heading);

    for (i, node) in matches.iter().enumerate() {
        let r = fsrs::retrievability(node.stability, node.last_reviewed);
        let cls = fsrs::classify(r);
        let color_fn: fn(&str) -> colored::ColoredString = match cls {
            "known" => |s| s.truecolor(29, 158, 117),
            "stale" => |s| s.truecolor(239, 159, 39),
            _ => |s| s.truecolor(226, 75, 74),
        };
        println!(
            "  {}  {} {}",
            format!("[{}]", i + 1).bold(),
            color_fn(&node.topic),
            format!("({:.0}% recall · {} · added {})", r * 100.0, node.kind.as_str(), node.created_at).dimmed(),
        );
        if !node.description.is_empty() {
            println!("       {}", node.description.dimmed());
        }
    }

    println!();
    if matches.len() == 1 {
        print!("  Delete \"{}\"? [y/N] ", matches[0].topic);
    } else {
        print!("  Enter number to delete, \"all\" to delete all, or Enter to cancel: ");
    }
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() || input.eq_ignore_ascii_case("n") {
        println!("  Cancelled.");
        return Ok(());
    }

    let to_delete: Vec<&node::Node> = if matches.len() == 1
        && (input.eq_ignore_ascii_case("y") || input == "1")
    {
        vec![&matches[0]]
    } else if input.eq_ignore_ascii_case("all") {
        matches.iter().collect()
    } else if let Ok(n) = input.parse::<usize>() {
        if n == 0 || n > matches.len() {
            println!("  {} Invalid selection.", "✗".truecolor(226, 75, 74));
            return Ok(());
        }
        vec![&matches[n - 1]]
    } else {
        println!("  Cancelled.");
        return Ok(());
    };

    for node in &to_delete {
        db.delete_node(&node.id)?;
        obsidian::delete_node(&node.id, &node.domain, pkg_dir);
        println!("  {} Deleted \"{}\".", "✓".truecolor(29, 158, 117), node.topic);
    }

    Ok(())
}

const MAX_DIFF_CHARS: usize = 12_000;

fn read_diff(git_ref: Option<&str>, staged: bool) -> Result<(String, String)> {
    // Returns (commit_msg, diff_text)
    use std::process::Command;

    if staged {
        let diff = Command::new("git").args(["diff", "--cached"]).output()?;
        let msg = Command::new("git")
            .args(["diff", "--cached", "--stat"])
            .output()?;
        return Ok((
            String::from_utf8_lossy(&msg.stdout).trim().to_string(),
            truncate_diff(&String::from_utf8_lossy(&diff.stdout)),
        ));
    }

    // Get commit message
    let commit_msg = match git_ref {
        Some(r) => {
            let out = Command::new("git")
                .args(["log", "-1", "--pretty=%B", r])
                .output()?;
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        None => {
            let out = Command::new("git")
                .args(["log", "-1", "--pretty=%B"])
                .output()?;
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
    };

    // Get diff
    let diff_out = match git_ref {
        Some(r) => Command::new("git").args(["diff", r, "HEAD"]).output()?,
        None => Command::new("git").args(["diff", "HEAD~1", "HEAD"]).output()?,
    };

    let diff = String::from_utf8_lossy(&diff_out.stdout).to_string();
    if diff.trim().is_empty() {
        anyhow::bail!("No diff found. Make sure there is at least one commit, or use --staged for staged changes.");
    }

    Ok((commit_msg, truncate_diff(&diff)))
}

fn truncate_diff(diff: &str) -> String {
    if diff.len() <= MAX_DIFF_CHARS {
        return diff.to_string();
    }
    // Keep the first MAX_DIFF_CHARS chars but end at a clean line boundary
    let truncated = &diff[..MAX_DIFF_CHARS];
    let end = truncated.rfind('\n').unwrap_or(MAX_DIFF_CHARS);
    format!(
        "{}\n\n[... diff truncated at {MAX_DIFF_CHARS} chars ...]",
        &diff[..end]
    )
}

fn run_diff(
    db: &Db,
    teacher: &Teacher,
    session: &Session,
    p: &personality::Personality,
    git_ref: Option<&str>,
    staged: bool,
    edge_reuse: &config::EdgeReuse,
) -> Result<()> {
    print_header();

    let (commit_msg, diff) = read_diff(git_ref, staged)
        .context("Failed to read git diff — are you in a git repository?")?;

    let label = if staged {
        "staged changes".to_string()
    } else {
        commit_msg.lines().next().unwrap_or("last commit").to_string()
    };

    println!("\n{} {label}\n", "Diff:".bold());
    println!("{}", "Analyzing code changes...".dimmed());

    let topics = teacher
        .extract_topics_from_diff(&commit_msg, &diff)
        .context("Failed to extract topics from diff")?;

    if topics.is_empty() {
        println!("{}", "No significant topics found in this diff.".truecolor(29, 158, 117));
        return Ok(());
    }

    let known_topic_names: Vec<String> = db
        .all_nodes()?
        .into_iter()
        .filter(|n| {
            fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)) == "known"
        })
        .map(|n| n.topic)
        .collect();

    // rocky diff is always a manual invocation — bypass cooldown, check budget only.
    let (quiz_allowed, block_reason) = session.can_quiz_manual()?;
    let budget = if quiz_allowed { session.budget_remaining()? } else { 0 };
    let mut quizzed = 0u32;
    let mut queued: Vec<String> = Vec::new();
    let mut new_count = 0u32;
    let mut stale_count = 0u32;
    let mut newly_added: Vec<(String, String)> = Vec::new();

    for topic_info in &topics {
        let topic = &topic_info.topic;
        let node = db.get_node(topic)?;
        let classification = match &node {
            None => "new",
            Some(n) => fsrs::classify(fsrs::retrievability(n.stability, n.last_reviewed)),
        };

        match classification {
            "known" => {
                let r = node
                    .as_ref()
                    .map(|n| fsrs::retrievability(n.stability, n.last_reviewed))
                    .unwrap_or(1.0);
                println!(
                    "{} {}",
                    format!("  ✓ {topic}").truecolor(29, 158, 117),
                    format!("({:.0}%)", r * 100.0).dimmed()
                );
                db.mark_encountered(topic)?;
            }
            "stale" => {
                stale_count += 1;
                let n = node.as_ref().unwrap();
                let r = fsrs::retrievability(n.stability, n.last_reviewed);
                println!(
                    "{} {}",
                    format!("  ~ {topic}").truecolor(239, 159, 39),
                    format!("(recall faded to {:.0}%)", r * 100.0).dimmed()
                );
                if quiz_allowed && quizzed < budget {
                    println!("{}", "  Refreshing...".dimmed());
                    let reminder = teacher.generate_reminder(topic, n, &label)?;
                    println!("\n  {} {reminder}\n", "Rocky:".truecolor(239, 159, 39).bold());
                    db.add_or_update(
                        topic,
                        0.5,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.domain,
                        &topic_info.description,
                        &label,
                        None, "", None,
                    )?;
                    session.record_quiz()?;
                    quizzed += 1;
                } else {
                    queued.push(topic.clone());
                    queue_for_later(topic, &topic_info.kind, &topic_info.description, &label);
                }
            }
            _ => {
                new_count += 1;
                if quiz_allowed && quizzed < budget {
                    let (completed, node_added) =
                        run_socratic_loop(db, teacher, topic_info, &label, &known_topic_names, p, edge_reuse, "", None, None)?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                    if node_added {
                        newly_added.push((topic.clone(), topic_info.description.clone()));
                    }
                } else {
                    queued.push(topic.clone());
                    queue_for_later(topic, &topic_info.kind, &topic_info.description, &label);
                }
            }
        }
    }

    if !newly_added.is_empty() {
        generate_edges_for_new_nodes(db, teacher, &newly_added, 0);
    }

    println!();
    if !queued.is_empty() {
        println!(
            "  {}",
            format!("Queued for next session: {}", queued.join(", ")).dimmed()
        );
    }
    if !quiz_allowed && (new_count > 0 || stale_count > 0) {
        println!("  {}", block_reason.truecolor(239, 159, 39));
    } else if new_count == 0 && stale_count == 0 {
        println!("{}", "All topics already in your PKG.".truecolor(29, 158, 117));
    }

    let (total, known, stale, _) = db.summary()?;
    println!(
        "  {}",
        format!("PKG: {known} known | {stale} fading | {total} total").dimmed()
    );
    Ok(())
}

fn run_export(db: &Db, cfg: &Config) -> Result<()> {
    let nodes = db.all_nodes()?;
    if nodes.is_empty() {
        // Still write dashboard pages so user can see empty-state queries
        obsidian::write_dashboard_pages(&cfg.pkg_dir)?;
        println!("  PKG is empty — no topic files to export.");
        println!(
            "  {} Dashboard pages written to {}",
            "✓".truecolor(29, 158, 117),
            cfg.pkg_dir.display()
        );
        return Ok(());
    }
    let edges = db.get_all_edges().unwrap_or_default();
    let count = obsidian::write_all(db, &nodes, &edges, &cfg.pkg_dir)?;
    println!(
        "  {} Exported {count} topic{} + dashboard to {}",
        "✓".truecolor(29, 158, 117),
        if count == 1 { "" } else { "s" },
        cfg.pkg_dir.display()
    );
    println!(
        "  {}",
        "Open 'Rocky Dashboard.md' and 'Rocky Review Queue.md' in Obsidian.".dimmed()
    );
    Ok(())
}

fn run_hook() -> Result<()> {
    // Only log if this project has Rocky hooks installed — never log globally.
    if !local_log::is_hook_installed() {
        return Ok(());
    }

    let mut input = String::new();
    io::stdin().lock().read_to_string(&mut input).ok();

    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&input) {
        if let Some(prompt) = data.get("prompt").and_then(|v| v.as_str()) {
            let prompt = prompt.trim();
            if !prompt.is_empty() {
                if let Some(log) = local_log::LocalLog::open_if_configured() {
                    log.log_prompt(prompt).ok();
                }
            }
        }
    }
    Ok(())
}

fn run_queue() -> Result<()> {
    print_header();
    match local_log::LocalLog::open_existing() {
        None => {
            println!("\n  No queue found for this project.");
            println!("  {}", "Run `rocky install` or `rocky install prompt` to enable logging here.".dimmed());
        }
        Some(log) => {
            let topics = log.get_queued_topics()?;
            if topics.is_empty() {
                println!("{}", "\n  No topics queued — queue is clear.".dimmed());
            } else {
                println!(
                    "\n  {} queued topic{} (not yet in PKG):\n",
                    topics.len(),
                    if topics.len() == 1 { "" } else { "s" }
                );
                for (topic, kind, description, context) in &topics {
                    println!("  {} {}", "·".truecolor(239, 159, 39), topic.bold());
                    println!("    {} · {}", kind.dimmed(), description.dimmed());
                    if !context.is_empty() {
                        println!("    {}", format!("from: {context}").dimmed());
                    }
                    println!();
                }
                println!("  {}", "Run  rocky quiz  to work through the queue.".dimmed());
            }
        }
    }
    println!();
    Ok(())
}

fn run_logs() -> Result<()> {
    print_header();
    match local_log::LocalLog::open_existing() {
        None => {
            println!("\n  No local log found.");
            println!("  {}", "Run `rocky install` in this project to enable prompt logging.".dimmed());
        }
        Some(log) => {
            let entries = log.recent_prompts(24)?;
            if entries.is_empty() {
                println!("{}", "\n  No prompts logged in the last 24 hours.".dimmed());
            } else {
                println!("\n  {} prompt{} in the last 24h:\n", entries.len(), if entries.len() == 1 { "" } else { "s" });
                for (prompt, logged_at) in &entries {
                    let time = &logged_at[11..16]; // HH:MM
                    println!("  {} {}", time.dimmed(), prompt);
                }
            }
        }
    }
    println!();
    Ok(())
}

// ── git hooks ─────────────────────────────────────────────────────────────────

fn rocky_bin_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "rocky".into())
}

fn install_git_hook() -> Result<(bool, String)> {
    let hook_path = std::path::Path::new(".git/hooks/post-commit");
    if !std::path::Path::new(".git").exists() {
        return Ok((false, "not a git repository".into()));
    }
    let bin = rocky_bin_path();
    let cmd = format!("{bin} diff");
    if hook_path.exists() {
        let existing = std::fs::read_to_string(hook_path)?;
        if existing.contains("rocky") {
            return Ok((false, "hook already installed".into()));
        }
        let appended = format!("{existing}\n{cmd}\n");
        std::fs::write(hook_path, appended)?;
    } else {
        std::fs::write(hook_path, format!("#!/bin/sh\n{cmd}\n"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(hook_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    local_log::ensure_gitignored()?;
    Ok((true, "git post-commit hook installed — .rocky added to .gitignore".into()))
}

fn uninstall_git_hook() -> Result<(bool, String)> {
    let hook_path = std::path::Path::new(".git/hooks/post-commit");
    if !hook_path.exists() {
        return Ok((false, "no hook found".into()));
    }
    let content = std::fs::read_to_string(hook_path)?;
    if !content.contains("rocky") {
        return Ok((false, "hook not managed by Rocky".into()));
    }
    let cleaned: String = content
        .lines()
        .filter(|l| !l.contains("rocky"))
        .collect::<Vec<_>>()
        .join("\n");
    if cleaned.trim().is_empty() || cleaned.trim() == "#!/bin/sh" {
        std::fs::remove_file(hook_path)?;
    } else {
        std::fs::write(hook_path, cleaned)?;
    }
    Ok((true, "git post-commit hook removed".into()))
}

fn claude_settings_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

fn install_claude_hook() -> Result<(bool, String)> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("could not locate home directory"))?;

    let mut settings: serde_json::Value = if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        serde_json::from_str(&text).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let hooks = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("settings.json is not a JSON object"))?
        .entry("hooks")
        .or_insert(serde_json::json!({}));

    let submit = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks is not a JSON object"))?
        .entry("UserPromptSubmit")
        .or_insert(serde_json::json!([]));

    let arr = submit
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("UserPromptSubmit is not an array"))?;

    // Use the resolved binary path so the hook works regardless of shell PATH
    let rocky_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "rocky".to_string());
    let cmd = format!("{rocky_bin} hook");

    // Check if already installed in either the new nested format or the old flat format
    let already = arr.iter().any(|v| {
        // New format: {"matcher": "RockyHook", "hooks": [{"command": "...", "type": "command"}]}
        let in_nested = v.get("hooks")
            .and_then(|h| h.as_array())
            .map(|hooks| hooks.iter().any(|h| {
                h.get("command").and_then(|c| c.as_str())
                    .map(|c| c == cmd || c.ends_with("/rocky hook") || c == "rocky hook")
                    .unwrap_or(false)
            }))
            .unwrap_or(false);
        // Old flat format: {"command": "..."}
        let in_flat = v.get("command").and_then(|c| c.as_str())
            .map(|c| c == cmd || c == "rocky hook" || c.ends_with("/rocky hook"))
            .unwrap_or(false);
        in_nested || in_flat
    });
    if already {
        return Ok((false, "Claude Code hook already installed".into()));
    }

    let entry = serde_json::json!({
        "hooks": [{"command": cmd, "type": "command"}]
    });
    arr.push(entry);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&settings)?)?;
    Ok((true, format!("Claude Code hook installed — added to {}", path.display())))
}

fn uninstall_claude_hook() -> Result<(bool, String)> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("could not locate home directory"))?;

    if !path.exists() {
        return Ok((false, "~/.claude/settings.json not found".into()));
    }

    let text = std::fs::read_to_string(&path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or(serde_json::json!({}));

    let removed = if let Some(arr) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("UserPromptSubmit"))
        .and_then(|v| v.as_array_mut())
    {
        let before = arr.len();
        // Remove any Rocky hook entry — new nested format or old flat format
        arr.retain(|v| {
            let is_flat = v.get("command")
                .and_then(|c| c.as_str())
                .map(|c| c == "rocky hook" || c.ends_with("/rocky hook"))
                .unwrap_or(false);
            let is_nested = v.get("hooks")
                .and_then(|h| h.as_array())
                .map(|hooks| hooks.iter().any(|h| {
                    h.get("command").and_then(|c| c.as_str())
                        .map(|c| c == "rocky hook" || c.ends_with("/rocky hook"))
                        .unwrap_or(false)
                }))
                .unwrap_or(false);
            !is_flat && !is_nested
        });
        arr.len() < before
    } else {
        false
    };

    if !removed {
        return Ok((false, "Claude Code hook not found in settings.json".into()));
    }

    std::fs::write(&path, serde_json::to_string_pretty(&settings)?)?;
    Ok((true, "Claude Code hook removed".into()))
}

// ── Stop hook (Claude Code session-end) ──────────────────────────────────────

const STOP_HOOK_MATCHER_TAG: &str = "rocky session-end";

fn install_claude_stop_hook() -> Result<(bool, String)> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("could not locate home directory"))?;

    let mut settings: serde_json::Value = if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        serde_json::from_str(&text).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let hooks = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("settings.json is not a JSON object"))?
        .entry("hooks")
        .or_insert(serde_json::json!({}));

    let stop = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks is not a JSON object"))?
        .entry("Stop")
        .or_insert(serde_json::json!([]));

    let arr = stop
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("Stop is not an array"))?;

    let rocky_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "rocky".to_string());
    let cmd = format!("{rocky_bin} session-end --quiet");

    let already = arr.iter().any(|v| {
        v.get("hooks")
            .and_then(|h| h.as_array())
            .map(|hs| hs.iter().any(|h| {
                h.get("command").and_then(|c| c.as_str())
                    .map(|c| c.contains(STOP_HOOK_MATCHER_TAG))
                    .unwrap_or(false)
            }))
            .unwrap_or(false)
    });
    if already {
        return Ok((false, "Claude Code Stop hook already installed".into()));
    }

    arr.push(serde_json::json!({
        "hooks": [{"command": cmd, "type": "command"}]
    }));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&settings)?)?;
    Ok((true, format!("Claude Code Stop hook installed in {}", path.display())))
}

fn uninstall_claude_stop_hook() -> Result<(bool, String)> {
    let path = claude_settings_path()
        .ok_or_else(|| anyhow::anyhow!("could not locate home directory"))?;
    if !path.exists() {
        return Ok((false, "~/.claude/settings.json not found".into()));
    }

    let text = std::fs::read_to_string(&path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or(serde_json::json!({}));

    let removed = if let Some(arr) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("Stop"))
        .and_then(|v| v.as_array_mut())
    {
        let before = arr.len();
        arr.retain(|v| {
            let is_rocky = v.get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| hs.iter().any(|h| {
                    h.get("command").and_then(|c| c.as_str())
                        .map(|c| c.contains(STOP_HOOK_MATCHER_TAG))
                        .unwrap_or(false)
                }))
                .unwrap_or(false);
            !is_rocky
        });
        arr.len() < before
    } else {
        false
    };

    if !removed {
        return Ok((false, "Claude Code Stop hook not found".into()));
    }

    std::fs::write(&path, serde_json::to_string_pretty(&settings)?)?;
    Ok((true, "Claude Code Stop hook removed".into()))
}

/// Like install_git_hook but uses queue-mode (`rocky post-commit`) so commits
/// stay fast and the Stop hook handles enrichment in one batch at session end.
fn install_git_hook_queue_mode() -> Result<(bool, String)> {
    let hook_path = std::path::Path::new(".git/hooks/post-commit");
    if !std::path::Path::new(".git").exists() {
        return Ok((false, "not a git repository".into()));
    }
    let bin = rocky_bin_path();
    let cmd = format!("{bin} post-commit");
    if hook_path.exists() {
        let existing = std::fs::read_to_string(hook_path)?;
        if existing.contains("post-commit") {
            return Ok((false, "queue-mode git hook already installed".into()));
        }
        // Replace any prior `rocky diff` invocation with the full-path queue command
        if existing.contains("rocky diff") || existing.contains(&format!("{bin} diff")) {
            let updated = existing
                .replace(&format!("{bin} diff"), &cmd)
                .replace("rocky diff", &cmd);
            std::fs::write(hook_path, updated)?;
            local_log::ensure_gitignored()?;
            return Ok((true, "git hook switched to queue mode".into()));
        }
        let appended = format!("{existing}\n{cmd}\n");
        std::fs::write(hook_path, appended)?;
    } else {
        std::fs::write(hook_path, format!("#!/bin/sh\n{cmd}\n"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(hook_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    local_log::ensure_gitignored()?;
    Ok((true, "git hook installed in queue mode".into()))
}

// ── skill-facing CLI primitives ──────────────────────────────────────────────
//
// These commands exist for the rocky-checkpoint and rocky-quiz Claude Code skills.
// They are pure storage/lookup wrappers — no LLM calls — so the agent (which is
// already running and has full context) does the thinking, and Rocky just records
// the result.

fn project_path_str() -> Result<String> {
    Ok(std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .to_string())
}

fn project_repo_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

fn node_to_json(db: &Db, n: &node::Node) -> serde_json::Value {
    let (r, m, recall) = db.node_recall(n);
    serde_json::json!({
        "id": n.id,
        "topic": n.topic,
        "kind": n.kind.as_str(),
        "domain": n.domain,
        "description": n.description,
        "difficulty": n.difficulty,
        "stability": n.stability,
        "retrievability": r,
        "mastery": m,
        "recall_now": recall,
        "classification": fsrs::classify(recall),
        "last_reviewed": n.last_reviewed.to_string(),
        "last_encountered": n.last_encountered.to_string(),
        "review_count": n.review_count,
        "encounter_count": n.encounter_count,
        "created_at": n.created_at.to_string(),
        "contexts": n.contexts,
        "source_commits": n.source_commits,
        "canonical_question": n.canonical_question,
        "canonical_answer": n.canonical_answer,
        "canonical_clue": n.canonical_clue,
        "question_bank": n.question_bank,
        "repo": n.repo,
        "repos": n.repos,
    })
}

fn emit_topics_json(db: &Db, since: Option<&str>) -> Result<()> {
    let cutoff: Option<chrono::NaiveDate> = since.and_then(|s| parse_since_days(s)).map(|days| {
        chrono::Local::now().date_naive() - chrono::Duration::days(days)
    });
    let nodes: Vec<serde_json::Value> = db.all_nodes()?
        .into_iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| cutoff.map_or(true, |c| n.created_at >= c))
        .map(|n| node_to_json(db, &n))
        .collect();
    println!("{}", serde_json::to_string_pretty(&nodes)?);
    Ok(())
}

fn emit_project_context_json(db: &Db) -> Result<()> {
    let project_path = project_path_str()?;
    let ctx = db.get_project_context(&project_path)?;
    let value = match ctx {
        Some(c) => serde_json::json!({
            "project_path": project_path,
            "summary": c.summary,
            "sources": c.sources,
            "last_explored_at": c.last_explored_at,
            "commits_since_explore": c.commits_since_explore,
        }),
        None => serde_json::json!({
            "project_path": project_path,
            "summary": null,
            "sources": [],
            "last_explored_at": null,
            "commits_since_explore": 0,
        }),
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn emit_checkpoint_diff_json(db: &Db) -> Result<()> {
    let project_path = project_path_str()?;
    let pending = db.peek_pending_diffs(&project_path)?;
    let commits: Vec<serde_json::Value> = pending.iter().map(|p| serde_json::json!({
        "id": p.id,
        "sha": p.commit_sha,
        "subject": p.commit_msg.lines().next().unwrap_or("").to_string(),
        "message": p.commit_msg,
        "diff": p.diff,
        "queued_at": p.queued_at,
    })).collect();
    let envelope = serde_json::json!({
        "project_path": project_path,
        "pending_count": commits.len(),
        "commits": commits,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

fn run_checkpoint_mark(db: &Db) -> Result<()> {
    let project_path = project_path_str()?;
    let drained = db.drain_pending_diffs(&project_path)?;
    let envelope = serde_json::json!({
        "project_path": project_path,
        "drained": drained.len(),
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

fn run_add_topic(
    db: &Db,
    name: &str,
    description: &str,
    domain: &str,
    kind: &str,
    context: Option<&str>,
    commit: Option<&str>,
    score: f64,
) -> Result<()> {
    let kind_enum = node::Kind::from_str(kind);
    let pre_existing = db.get_node(name)?.is_some();
    db.add_or_update(
        name,
        score,
        &kind_enum,
        domain,
        description,
        context.unwrap_or(""),
        None,
        &project_repo_name(),
        None,
    )?;
    let repo = project_repo_name();
    if let Some(sha) = commit {
        if !sha.is_empty() {
            db.record_topic_encounter(name, sha, &repo)?;
        }
    } else if !repo.is_empty() {
        // No commit linked but we still want to record cross-project encounters.
        db.record_topic_encounter(name, "", &repo)?;
    }

    // Surface the new recall_now so the skill can report meaningfully.
    let (r, m, recall) = db.get_node(name)?
        .map(|n| db.node_recall(&n))
        .unwrap_or((1.0, 0.5, 0.5));

    let action = if pre_existing { "merged" } else { "created" };
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "topic": name,
        "action": action,
        "domain": domain,
        "kind": kind,
        "retrievability": r,
        "mastery": m,
        "recall_now": recall,
    }))?);
    Ok(())
}

fn emit_due_json(db: &Db, limit: usize) -> Result<()> {
    // Rank by recall_now ascending (lowest first = most urgent to drill).
    let mut scored: Vec<(f64, node::Node)> = db.all_nodes()?
        .into_iter()
        .filter(|n| !n.kind.is_domain())
        .map(|n| {
            let (_, _, recall) = db.node_recall(&n);
            (recall, n)
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    let value: Vec<serde_json::Value> = scored.iter().map(|(_, n)| node_to_json(db, n)).collect();
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn emit_topic_json(db: &Db, name: &str) -> Result<()> {
    let nodes = db.all_nodes()?;
    let q = name.to_lowercase();
    let matches: Vec<&node::Node> = nodes
        .iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| n.topic.to_lowercase().contains(&q) || n.id == name)
        .collect();
    if matches.is_empty() {
        anyhow::bail!("no topic matching '{name}'");
    }
    if matches.len() > 1 {
        let names: Vec<&str> = matches.iter().map(|n| n.topic.as_str()).collect();
        anyhow::bail!("ambiguous: {} topics match '{name}': {}", names.len(), names.join(", "));
    }
    let n = matches[0];
    let mut value = node_to_json(db, n);
    let reviews = db.get_reviews(&n.id).unwrap_or_default();
    value["recent_reviews"] = serde_json::json!(reviews.iter().rev().take(5).map(|r| {
        serde_json::json!({
            "reviewed_at": r.reviewed_at,
            "question": r.question,
            "answer": r.answer,
            "feedback": r.feedback,
            "score": r.score,
        })
    }).collect::<Vec<_>>());
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn run_record_review(
    db: &Db,
    topic_query: &str,
    score: f64,
    question: Option<&str>,
    answer: Option<&str>,
    feedback: Option<&str>,
) -> Result<()> {
    if !(0.0..=1.0).contains(&score) {
        anyhow::bail!("score must be in [0, 1] (got {score})");
    }
    let nodes = db.all_nodes()?;
    let q = topic_query.to_lowercase();
    let matches: Vec<&node::Node> = nodes
        .iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| n.id == topic_query || n.topic.to_lowercase().contains(&q))
        .collect();
    if matches.is_empty() {
        anyhow::bail!("no topic matching '{topic_query}'");
    }
    if matches.len() > 1 {
        let names: Vec<&str> = matches.iter().map(|n| n.topic.as_str()).collect();
        anyhow::bail!("ambiguous: {} topics match '{topic_query}': {}", names.len(), names.join(", "));
    }
    let n = matches[0];
    let q_text = question.unwrap_or("");
    let a_text = answer.unwrap_or("");
    let f_text = feedback.unwrap_or("");
    let new_r = db.record_quiz_review_full(&n.id, score, q_text, a_text, f_text)?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "topic": n.topic,
        "score": score,
        "new_retrievability": new_r,
        "review_count": n.review_count + 1,
    }))?);
    Ok(())
}

fn run_add_question(db: &Db, topic_query: &str, question: &str, answer: &str, clue: &str) -> Result<()> {
    let nodes = db.all_nodes()?;
    let q = topic_query.to_lowercase();
    let matches: Vec<&node::Node> = nodes
        .iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| n.id == topic_query || n.topic.to_lowercase().contains(&q))
        .collect();
    if matches.is_empty() {
        anyhow::bail!("no topic matching '{topic_query}'");
    }
    if matches.len() > 1 {
        let names: Vec<&str> = matches.iter().map(|n| n.topic.as_str()).collect();
        anyhow::bail!("ambiguous: {} topics match '{topic_query}': {}", names.len(), names.join(", "));
    }
    let n = matches[0];
    let item = node::QuestionBankItem {
        question: question.trim().to_string(),
        answer: answer.trim().to_string(),
        clue: clue.trim().to_string(),
        asked_count: 0,
    };
    let added = db.append_question(&n.topic, item)?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "topic": n.topic,
        "added": added,
        "reason": if added { "appended" } else { "duplicate question text — skipped" },
    }))?);
    Ok(())
}

fn run_delete_topic(db: &Db, topic_query: &str) -> Result<()> {
    let nodes = db.all_nodes()?;
    let q = topic_query.to_lowercase();
    let matches: Vec<&node::Node> = nodes
        .iter()
        .filter(|n| !n.kind.is_domain())
        .filter(|n| n.id == topic_query || n.topic.to_lowercase().contains(&q))
        .collect();
    if matches.is_empty() {
        anyhow::bail!("no topic matching '{topic_query}'");
    }
    if matches.len() > 1 {
        let names: Vec<&str> = matches.iter().map(|n| n.topic.as_str()).collect();
        anyhow::bail!("ambiguous: {} topics match '{topic_query}': {}", names.len(), names.join(", "));
    }
    let n = matches[0];
    let deleted = db.delete_node_by_topic(&n.topic)?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "topic": n.topic,
        "deleted": deleted,
    }))?);
    Ok(())
}

// ── claude code skill installer ──────────────────────────────────────────────

const SKILL_CHECKPOINT: &str = include_str!("../skills/rocky-checkpoint/SKILL.md");
const SKILL_QUIZ: &str = include_str!("../skills/rocky-quiz/SKILL.md");

fn claude_skills_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("could not resolve home directory")?;
    Ok(home.join(".claude").join("skills"))
}

fn install_claude_skills() -> Result<Vec<(bool, String)>> {
    let base = claude_skills_dir()?;
    std::fs::create_dir_all(&base)?;
    let skills = [("rocky-checkpoint", SKILL_CHECKPOINT), ("rocky-quiz", SKILL_QUIZ)];
    let mut out = Vec::new();
    for (name, body) in skills {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("SKILL.md");
        std::fs::write(&path, body)?;
        out.push((true, format!("installed skill: {} → {}", name, path.display())));
    }
    Ok(out)
}

fn uninstall_claude_skills() -> Result<Vec<(bool, String)>> {
    let base = claude_skills_dir()?;
    let mut out = Vec::new();
    for name in ["rocky-checkpoint", "rocky-quiz"] {
        let dir = base.join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            out.push((true, format!("removed skill: {name}")));
        } else {
            out.push((false, format!("skill not present: {name}")));
        }
    }
    Ok(out)
}

// ── post-commit queue command ────────────────────────────────────────────────

fn run_post_commit(db: &Db) -> Result<()> {
    let project_path = std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .to_string();

    let sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let commit_msg = std::process::Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let diff = std::process::Command::new("git")
        .args(["show", "--stat", "--patch", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if sha.is_empty() || diff.is_empty() {
        return Ok(());
    }

    db.queue_pending_diff(&project_path, &sha, &commit_msg, &diff)?;
    db.bump_commits_since_explore(&project_path).ok();
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Check if adding this topic crossed a milestone and print Rocky's reaction.
fn print_milestone(db: &Db, p: &personality::Personality, _topic: &str) {
    if let Ok((_, known, _, _)) = db.summary() {
        if let Some(msg) = p.milestone(known) {
            p.print_rocky(true);
            println!("   {msg}");
        }
    }
}

/// Save a topic to the local project queue (.rocky) without writing to the global PKG.
/// Best-effort — silently ignored if not in a hooked project.
fn run_sync(db: &Db, cfg: &Config, init: Option<Option<String>>, push: bool, status: bool) -> Result<()> {
    use colored::Colorize;
    let rocky_dir = &cfg.rocky_dir;

    if status {
        println!("  {}", sync::status_summary(rocky_dir));
        let ahead = sync::commits_ahead(rocky_dir, &cfg.sync.remote, &cfg.sync.branch);
        if ahead > 0 {
            println!("  {} commit{} ahead of {}/{}", ahead, if ahead == 1 { "" } else { "s" }, cfg.sync.remote, cfg.sync.branch);
        }
        return Ok(());
    }

    if let Some(remote_url) = init {
        let newly_created = sync::ensure_repo(rocky_dir)?;
        if newly_created {
            println!("  {} Initialised git repo at {}", "✓".truecolor(29, 158, 117), rocky_dir.display());
        } else {
            println!("  {} Git repo already exists", "✓".truecolor(29, 158, 117));
        }
        if let Some(url) = remote_url {
            sync::set_remote(rocky_dir, &cfg.sync.remote, &url)?;
            println!("  {} Remote '{}' set to {url}", "✓".truecolor(29, 158, 117), cfg.sync.remote);
            println!("  {}", "Run `rocky sync --push` to push your PKG.".dimmed());
        } else {
            println!("  {}", format!("Add a remote: rocky sync --init <url>  or  git -C {} remote add origin <url>", rocky_dir.display()).dimmed());
        }
        return Ok(());
    }

    if !sync::is_git_repo(rocky_dir) {
        println!("  {} Vault is not a git repo — run `rocky sync --init` first.", "✗".truecolor(226, 75, 74));
        return Ok(());
    }

    // Write pkg.json then commit
    let nodes = db.all_nodes()?;
    let edges = db.get_all_edges().unwrap_or_default();
    obsidian::write_all(db, &nodes, &edges, &cfg.pkg_dir)?;
    db.export_pkg_json(&cfg.pkg_dir.join("pkg.json"))?;

    let msg = build_commit_message(db);
    match sync::commit(rocky_dir, &msg)? {
        true  => println!("  {} {msg}", "✓".truecolor(29, 158, 117)),
        false => println!("  {} Nothing changed since last commit.", "·".dimmed()),
    }

    if push {
        sync::push(rocky_dir, &cfg.sync.remote, &cfg.sync.branch)?;
        sync::reset_push_counter(db)?;
        println!("  {} Pushed to {}/{}", "✓".truecolor(29, 158, 117), cfg.sync.remote, cfg.sync.branch);
    }

    Ok(())
}

fn run_classify(db: &Db, teacher: &Teacher) -> Result<()> {
    // ── Step 1: Assign domains to any undomained nodes ────────────────────────
    let undomained = db.undomained_nodes()?;
    if !undomained.is_empty() {
        println!("  Classifying {} topic{} into domains...", undomained.len(), if undomained.len() == 1 { "" } else { "s" });
        let pairs: Vec<(String, String)> = undomained.iter()
            .map(|n| (n.topic.clone(), n.description.clone()))
            .collect();
        let results = teacher.classify_domains(&pairs)?;
        for (topic, domain) in &results {
            let node_id = topic.to_lowercase().trim().replace(' ', "-");
            db.set_domain(&node_id, domain)?;
            println!("  {} {} → {}", "✓".truecolor(29, 158, 117), topic, domain.truecolor(6, 182, 212));
        }
    } else {
        println!("  {} All topics already have a domain.", "✓".truecolor(29, 158, 117));
    }

    // ── Step 2: Ensure taxonomy skeleton exists ───────────────────────────────
    db.ensure_taxonomy_skeleton()?;

    // ── Step 3: Link all nodes to their domain skeleton node ─────────────────
    let linked = link_all_to_taxonomy(db)?;
    if linked > 0 {
        println!("  {} Linked {} topic{} to taxonomy skeleton.", "✓".truecolor(29, 158, 117), linked, if linked == 1 { "" } else { "s" });
    } else {
        println!("  {} All topics already linked to taxonomy.", "✓".truecolor(29, 158, 117));
    }

    if !undomained.is_empty() {
        println!("  {} Run `rocky export` to update PKG files.", "·".dimmed());
    }
    Ok(())
}

fn link_all_to_taxonomy(db: &Db) -> Result<usize> {
    use db::{Edge, EdgeKind};

    let nodes = db.all_nodes()?;
    let mut linked = 0;

    for node in nodes.iter().filter(|n| !n.kind.is_domain() && !n.domain.is_empty()) {
        let domain_node = match db.get_node(&node.domain)? {
            Some(n) if n.kind.is_domain() => n,
            _ => continue,
        };

        let kind = EdgeKind::PartOf;
        if db.edge_exists(&node.id, &domain_node.id, &kind).unwrap_or(true) {
            continue;
        }

        let edge_id = format!("{}-{}-part_of", node.id, domain_node.id);
        db.insert_edge(&Edge {
            id: edge_id,
            source_id: node.id.clone(),
            target_id: domain_node.id.clone(),
            kind,
            description: format!("{} is a topic within the {} domain.", node.topic, node.domain),
            strength: 1.0,
            created_at: chrono::Local::now().naive_local().to_string(),
            last_fired: None,
            last_fired_session: None,
        })?;
        linked += 1;
    }

    Ok(linked)
}

/// Auto-sync after a session: export PKG, write pkg.json, commit if configured.
fn auto_sync(db: &Db, cfg: &Config) {
    if !cfg.sync.enabled || !cfg.sync.auto_commit { return; }
    if !sync::is_git_repo(&cfg.rocky_dir) { return; }

    let nodes = match db.all_nodes() { Ok(n) => n, Err(_) => return };
    let edges = db.get_all_edges().unwrap_or_default();
    obsidian::write_all(db, &nodes, &edges, &cfg.pkg_dir).ok();
    db.export_pkg_json(&cfg.pkg_dir.join("pkg.json")).ok();

    let msg = build_commit_message(db);
    match sync::commit(&cfg.rocky_dir, &msg) {
        Ok(true) => {
            if cfg.sync.commit_visible {
                println!("  {} {}", "✓".truecolor(29, 158, 117), msg);
            }
            sync::increment_sessions_since_push(db).ok();
        }
        Ok(false) => {} // nothing to commit
        Err(_) => {}    // best-effort
    }

    // Push reminder
    if let Some(reminder) = sync::push_reminder(db, &cfg.sync) {
        println!("  {}", reminder.truecolor(239, 159, 39));
    }
}

fn build_commit_message(db: &Db) -> String {
    let (total, known, _, _) = db.summary().unwrap_or((0, 0, 0, 0));
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    format!("Rocky: {known}/{total} known — {date}")
}

fn queue_for_later(topic: &str, kind: &str, description: &str, context: &str) {
    if let Some(log) = local_log::LocalLog::open_if_configured() {
        log.queue_topic(topic, kind, description, context).ok();
    }
}

// ── project / repo helpers ────────────────────────────────────────────────────

/// Derive a short repo name from the git remote URL or fall back to the top-level dir name.
fn detect_repo_name() -> String {
    use std::process::Command;
    if let Ok(out) = Command::new("git").args(["remote", "get-url", "origin"]).output() {
        if out.status.success() {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let name = url.trim_end_matches(".git")
                .rsplit('/')
                .next()
                .or_else(|| url.trim_end_matches(".git").rsplit(':').next()
                    .and_then(|s| s.split('/').last()))
                .unwrap_or("")
                .to_string();
            if !name.is_empty() { return name; }
        }
    }
    if let Ok(out) = Command::new("git").args(["rev-parse", "--show-toplevel"]).output() {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(name) = std::path::Path::new(&path).file_name().and_then(|n| n.to_str()) {
                return name.to_string();
            }
        }
    }
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Get the date of a specific commit SHA.
fn commit_date(sha: &str) -> Option<chrono::NaiveDate> {
    use std::process::Command;
    let out = Command::new("git").args(["log", "-1", "--date=short", "--format=%ad", sha]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
}

/// Get the date of the HEAD commit.
fn head_commit_date() -> Option<chrono::NaiveDate> {
    commit_date("HEAD")
}

/// Load the project summary from cache, regenerating if the README has changed.
/// Cached in `<rocky_dir>/summaries/<repo>.txt`.
fn load_or_create_project_summary(
    rocky_dir: &std::path::Path,
    repo: &str,
    teacher: &teacher::Teacher,
) -> String {
    let summaries_dir = rocky_dir.join("summaries");
    let _ = std::fs::create_dir_all(&summaries_dir);
    let summary_path = summaries_dir.join(format!("{repo}.txt"));
    let hash_path    = summaries_dir.join(format!("{repo}.hash"));

    let readme = std::fs::read_to_string("README.md")
        .or_else(|_| std::fs::read_to_string("README"))
        .or_else(|_| std::fs::read_to_string("readme.md"))
        .unwrap_or_default();

    if readme.is_empty() {
        return std::fs::read_to_string(&summary_path).unwrap_or_default();
    }

    let fingerprint = format!("{}:{}", readme.len(), &readme[..readme.len().min(100)]);
    let cached_hash = std::fs::read_to_string(&hash_path).unwrap_or_default();

    if cached_hash.trim() == fingerprint.trim() {
        if let Ok(s) = std::fs::read_to_string(&summary_path) {
            if !s.trim().is_empty() { return s; }
        }
    }

    match teacher.summarize_readme(&readme) {
        Ok(summary) => {
            let _ = std::fs::write(&summary_path, &summary);
            let _ = std::fs::write(&hash_path, fingerprint);
            summary
        }
        Err(_) => std::fs::read_to_string(&summary_path).unwrap_or_default(),
    }
}

fn print_header() {
    println!("\n  {}", "Rocky · Personal Knowledge Graph".bold());
    println!("{}", " ──────────────────────────────────────".dimmed());
}

fn is_hotfix(msg: &str) -> bool {
    let lower = msg.trim().to_lowercase();
    lower.starts_with("fix")
        || lower.starts_with("hotfix")
        || lower.starts_with("bugfix")
        || lower.starts_with("patch")
        || lower.starts_with("[hotfix]")
}

/// Pick the bank entry that has been asked the fewest times.
/// Returns the index and a reference for borrowing convenience.
fn pick_least_asked(
    bank: &[crate::node::QuestionBankItem],
) -> Option<(usize, &crate::node::QuestionBankItem)> {
    bank.iter()
        .enumerate()
        .min_by_key(|(_, q)| q.asked_count)
        .map(|(i, q)| (i, q))
}

// ── Voice CLI helper ──────────────────────────────────────────────────────────

/// Read an answer from stdin. In voice mode, an empty line triggers mic recording.
/// Returns the answer text, or a string starting with `\x00` to signal EOF/skip.
fn read_answer_cli(cli_voice: Option<&voice::CliVoice>) -> Result<String> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Err(_) | Ok(0) => return Ok("\x00eof".into()),
        Ok(_) => {}
    }
    let input = line.trim().to_string();

    // In voice mode, empty Enter → record mic until Enter pressed again
    if input.is_empty() {
        if let Some(v) = cli_voice {
            println!("{}", "   🎤 Recording... press Enter to stop.".truecolor(6, 182, 212));
            match v.record_and_transcribe() {
                Ok(transcript) if !transcript.is_empty() => {
                    println!("{}", format!("   Heard: \"{}\"", transcript).truecolor(167, 139, 250));
                    println!("{}", "   Press Enter to submit, or type to override:".dimmed());
                    print!("   > ");
                    io::stdout().flush()?;
                    let mut confirm = String::new();
                    io::stdin().read_line(&mut confirm)?;
                    let override_text = confirm.trim().to_string();
                    return Ok(if override_text.is_empty() { transcript } else { override_text });
                }
                Ok(_) => {
                    println!("{}", "   (Nothing heard — treating as skip)".dimmed());
                    return Ok(String::new());
                }
                Err(e) => {
                    println!("{}", format!("   Recording failed: {e} — type your answer instead:").truecolor(226, 75, 74));
                    print!("   > ");
                    io::stdout().flush()?;
                    let mut fallback = String::new();
                    io::stdin().read_line(&mut fallback)?;
                    return Ok(fallback.trim().to_string());
                }
            }
        }
    }

    Ok(input)
}

// ── rocky dedupe ──────────────────────────────────────────────────────────────

/// Word-set Jaccard similarity for two topic name strings.
fn topic_jaccard(a: &str, b: &str) -> f64 {
    let words = |s: &str| -> std::collections::HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 3)
            .map(|w| w.to_lowercase())
            .collect()
    };
    let sa = words(a);
    let sb = words(b);
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let intersection = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    intersection as f64 / union as f64
}

fn is_candidate_pair(a: &str, b: &str) -> bool {
    if topic_jaccard(a, b) >= 0.5 {
        return true;
    }
    // Substring: one name fully contained in the other
    let an = a.to_lowercase();
    let bn = b.to_lowercase();
    an.contains(bn.as_str()) || bn.contains(an.as_str())
}

/// Merge two question banks, deduplicating by question text, capped at 8 items.
fn merge_question_banks(
    primary: &[crate::node::QuestionBankItem],
    secondary: &[crate::node::QuestionBankItem],
) -> Vec<crate::node::QuestionBankItem> {
    let mut result = primary.to_vec();
    let existing: std::collections::HashSet<String> =
        result.iter().map(|q| q.question.clone()).collect();
    for item in secondary {
        if !existing.contains(&item.question) && result.len() < 8 {
            result.push(item.clone());
        }
    }
    result
}

fn run_dedupe(db: &Db, teacher: &Teacher, dry_run: bool, auto: bool) -> Result<()> {
    print_header();
    if dry_run {
        println!("  {} Dry-run mode — no changes will be written.\n", "~".truecolor(239, 159, 39));
    }
    println!("  {}", "Scanning PKG for duplicate topics...".dimmed());

    let all_nodes: Vec<_> = db.all_nodes()?.into_iter().filter(|n| !n.kind.is_domain()).collect();
    if all_nodes.len() < 2 {
        println!("  {} Not enough topics to scan.", "✗".truecolor(226, 75, 74));
        return Ok(());
    }

    // Build candidate pairs via word-overlap heuristic
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for i in 0..all_nodes.len() {
        for j in (i + 1)..all_nodes.len() {
            if is_candidate_pair(&all_nodes[i].topic, &all_nodes[j].topic) {
                candidates.push((i, j));
            }
        }
    }

    if candidates.is_empty() {
        println!("  {} No near-duplicate candidates found.", "✓".truecolor(29, 158, 117));
        return Ok(());
    }

    println!(
        "  {} candidate pair{} found.\n",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "s" }
    );

    let mut merged_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merge_count = 0usize;

    for (i, j) in &candidates {
        let a = &all_nodes[*i];
        let b = &all_nodes[*j];

        if merged_ids.contains(&a.id) || merged_ids.contains(&b.id) {
            continue;
        }

        // Optional LLM pre-filter
        if auto {
            match teacher.is_duplicate_pair(&a.topic, &a.description, &b.topic, &b.description) {
                Ok(false) => continue,
                Err(e) => {
                    eprintln!("  {} LLM check failed for pair ({}, {}): {e}", "!".truecolor(239, 159, 39), a.topic, b.topic);
                }
                Ok(true) => {}
            }
        }

        println!(
            "  ┌─ A: {} {}",
            a.topic.truecolor(6, 182, 212).bold(),
            format!("({})", a.domain).dimmed()
        );
        println!("  │     {}", a.description.dimmed());
        println!("  │");
        println!(
            "  └─ B: {} {}",
            b.topic.truecolor(239, 159, 39).bold(),
            format!("({})", b.domain).dimmed()
        );
        println!("        {}", b.description.dimmed());
        println!();
        println!(
            "  {}",
            "[a] merge → A   [b] merge → B   [m] merge → best name (LLM)   [s] skip   [q] quit".dimmed()
        );
        print!("  > ");
        io::stdout().flush()?;

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Err(_) | Ok(0) => break,
            Ok(_) => {}
        }

        match line.trim().to_lowercase().as_str() {
            "a" => {
                if !dry_run {
                    let merged_bank = merge_question_banks(&a.question_bank, &b.question_bank);
                    db.merge_nodes(&a.id, &b.id)?;
                    if !merged_bank.is_empty() {
                        db.set_question_bank(&a.topic, &merged_bank).ok();
                    }
                }
                println!(
                    "  {} Merged into A: \"{}\"\n",
                    "✓".truecolor(29, 158, 117),
                    a.topic
                );
                merged_ids.insert(b.id.clone());
                merge_count += 1;
            }
            "b" => {
                if !dry_run {
                    let merged_bank = merge_question_banks(&b.question_bank, &a.question_bank);
                    db.merge_nodes(&b.id, &a.id)?;
                    if !merged_bank.is_empty() {
                        db.set_question_bank(&b.topic, &merged_bank).ok();
                    }
                }
                println!(
                    "  {} Merged into B: \"{}\"\n",
                    "✓".truecolor(29, 158, 117),
                    b.topic
                );
                merged_ids.insert(a.id.clone());
                merge_count += 1;
            }
            "m" => {
                // LLM picks the best canonical name + description; user can override
                println!("{}", "  Asking LLM for best canonical name...".dimmed());
                let (suggested_name, suggested_desc) = match teacher.suggest_merge_name(
                    &a.topic, &a.description, &b.topic, &b.description,
                ) {
                    Ok(pair) => pair,
                    Err(e) => {
                        println!("  {} LLM failed: {e} — falling back to A.", "!".truecolor(239, 159, 39));
                        (a.topic.clone(), a.description.clone())
                    }
                };

                println!(
                    "  {} \"{}\"",
                    "LLM suggests:".truecolor(167, 139, 250).bold(),
                    suggested_name.truecolor(167, 139, 250)
                );
                println!("            {}", suggested_desc.dimmed());
                println!("{}", "  Press Enter to accept, or type a custom name:".dimmed());
                print!("  > ");
                io::stdout().flush()?;

                let mut name_line = String::new();
                io::stdin().read_line(&mut name_line)?;
                let final_name = {
                    let typed = name_line.trim();
                    if typed.is_empty() { suggested_name.clone() } else { typed.to_string() }
                };
                let final_desc = if final_name == suggested_name {
                    suggested_desc.clone()
                } else {
                    // User typed their own name — keep the LLM description unless it mentions
                    // either original topic name (would be misleading)
                    suggested_desc.clone()
                };

                if !dry_run {
                    let merged_bank = merge_question_banks(&a.question_bank, &b.question_bank);
                    // Merge B into A, then rename A to the chosen name
                    db.merge_nodes(&a.id, &b.id)?;
                    db.update_topic_name(&a.id, &final_name, &final_desc)?;
                    if !merged_bank.is_empty() {
                        db.set_question_bank(&final_name, &merged_bank).ok();
                    }
                }
                println!(
                    "  {} Merged into: \"{}\"\n",
                    "✓".truecolor(29, 158, 117),
                    final_name
                );
                merged_ids.insert(b.id.clone());
                merge_count += 1;
            }
            "q" => {
                println!("  Quitting.");
                break;
            }
            _ => {
                println!("  Skipped.\n");
            }
        }
    }

    println!(
        "  {} topic{} merged{}.",
        merge_count,
        if merge_count == 1 { "" } else { "s" },
        if dry_run { " (dry-run — nothing written)" } else { "" }
    );
    Ok(())
}

/// If the project context is missing or stale, print a one-line nudge.
/// Stale = > 20 commits since explore OR > 14 days since last_explored_at.
fn nudge_if_context_stale(db: &Db) {
    let project_path = match std::env::current_dir().and_then(|p| p.canonicalize()) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };
    let ctx = match db.get_project_context(&project_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            println!(
                "\n  {} {}",
                "·".dimmed(),
                "No project context yet — run  rocky explore  to ground future questions in real architecture.".dimmed()
            );
            return;
        }
        Err(_) => return,
    };

    let stale_by_commits = ctx.commits_since_explore > 20;
    let stale_by_age = chrono::NaiveDateTime::parse_from_str(&ctx.last_explored_at, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&ctx.last_explored_at, "%Y-%m-%d %H:%M:%S"))
        .map(|d| (chrono::Local::now().naive_local() - d).num_days() > 14)
        .unwrap_or(false);

    if stale_by_commits || stale_by_age {
        let reason = if stale_by_commits {
            format!("{} commits since last explore", ctx.commits_since_explore)
        } else {
            "context is over 14 days old".into()
        };
        println!(
            "\n  {} Project context is stale ({}). Run  rocky explore  to refresh.",
            "!".truecolor(239, 159, 39),
            reason
        );
    }
}

// ── rocky explore ─────────────────────────────────────────────────────────────

const EXPLORE_DOC_FILES: &[&str] = &[
    "CLAUDE.md",
    "README.md",
    "README",
    "readme.md",
    "ARCHITECTURE.md",
    "architecture.md",
    "DESIGN.md",
];

const EXPLORE_DOC_DIRS: &[&str] = &[
    "docs/decisions",
    "docs/architecture",
    "docs/adr",
    "docs",
    "architecture",
];

const MAX_EXPLORE_FILE_BYTES: u64 = 64 * 1024;
const MAX_EXPLORE_DOCS: usize = 12;

fn show_project_context(db: &Db) -> Result<()> {
    let project_path = std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .to_string();
    print_header();
    println!("  Project: {}\n", project_path.dimmed());

    let ctx = match db.get_project_context(&project_path)? {
        Some(c) => c,
        None => {
            println!("  No project context stored. Run  rocky explore  to generate one.\n");
            return Ok(());
        }
    };

    let div = "─".repeat(60);
    println!("  {}", div.dimmed());
    println!("  {:<22} {}", "Last explored:".dimmed(), ctx.last_explored_at);
    println!("  {:<22} {}", "Commits since:".dimmed(), ctx.commits_since_explore);
    if !ctx.sources.is_empty() {
        println!("  {:<22}", "Sources:".dimmed());
        for s in &ctx.sources {
            println!("    · {s}");
        }
    }
    println!("  {}", div.dimmed());
    println!("\n{}\n", ctx.summary);
    println!("  {}", div.dimmed());
    println!("  {}", "Re-run with  rocky explore --force  to regenerate.".dimmed());
    println!();
    Ok(())
}

fn run_explore(db: &Db, teacher: &Teacher, force: bool, quiet: bool) -> Result<()> {
    let project_path = std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .to_string();
    let project_name = std::path::Path::new(&project_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());

    if !quiet {
        print_header();
        println!("  Exploring {}...\n", project_path.dimmed());
    }

    if !force {
        if let Ok(Some(existing)) = db.get_project_context(&project_path) {
            let stale = existing.commits_since_explore > 20;
            if !stale {
                if !quiet {
                    println!("  {} Project context already up to date ({} commits since last explore).",
                        "·".dimmed(), existing.commits_since_explore);
                    println!("  {}", "Use --force to refresh anyway.".dimmed());
                }
                return Ok(());
            }
        }
    }

    let docs = collect_project_docs(&std::path::PathBuf::from(&project_path))?;
    if docs.is_empty() && !quiet {
        println!("  {} No documentation files found. Project context will be sparse.", "!".truecolor(239, 159, 39));
    }

    if !quiet && !docs.is_empty() {
        println!("  {} Reading {} documentation file(s):", "·".dimmed(), docs.len());
        for (path, _) in &docs {
            println!("    · {path}");
        }
        println!();
    }

    let commits = recent_commit_messages(30);
    if !quiet {
        println!("  {} Scanned {} recent commit(s).", "·".dimmed(), commits.len());
        println!("  {} Synthesising project context...", "·".dimmed());
    }

    let summary = teacher.summarize_project_docs(&project_name, &docs, &commits)
        .context("project context summarisation failed")?;

    let sources: Vec<String> = docs.iter().map(|(p, _)| p.clone()).collect();
    db.upsert_project_context(&project_path, &summary, &sources)?;

    if !quiet {
        println!("  {} Project context saved.\n", "✓".truecolor(29, 158, 117));
        println!("  {}", "Future quiz questions will be grounded in this summary.".dimmed());
        println!("  {}", "Re-run  rocky explore --force  whenever the project's shape changes.".dimmed());
    }
    Ok(())
}

/// Collect known documentation files from the project, capped to MAX_EXPLORE_DOCS.
fn collect_project_docs(project_root: &std::path::Path) -> Result<Vec<(String, String)>> {
    let mut found: Vec<(String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<std::path::PathBuf> = std::collections::HashSet::new();

    // Top-level marker files
    for name in EXPLORE_DOC_FILES {
        let path = project_root.join(name);
        if let Ok(canonical) = path.canonicalize() {
            if seen.insert(canonical.clone()) {
                if let Some(content) = read_capped(&path) {
                    found.push((name.to_string(), content));
                    if found.len() >= MAX_EXPLORE_DOCS {
                        return Ok(found);
                    }
                }
            }
        }
    }

    // Walk a few known doc directories (one level deep)
    for dir in EXPLORE_DOC_DIRS {
        let dir_path = project_root.join(dir);
        if !dir_path.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir_path) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let mut paths: Vec<std::path::PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
                p.is_file() && (ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("txt") || name.starts_with("ADR"))
            })
            .collect();
        paths.sort();
        for path in paths {
            if let Ok(canonical) = path.canonicalize() {
                if !seen.insert(canonical) {
                    continue;
                }
            }
            if let Some(content) = read_capped(&path) {
                let display = path.strip_prefix(project_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                found.push((display, content));
                if found.len() >= MAX_EXPLORE_DOCS {
                    return Ok(found);
                }
            }
        }
    }

    Ok(found)
}

fn read_capped(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_EXPLORE_FILE_BYTES {
        // Read only the first portion to keep prompt size bounded
        let mut f = std::fs::File::open(path).ok()?;
        let mut buf = vec![0u8; MAX_EXPLORE_FILE_BYTES as usize];
        let n = std::io::Read::read(&mut f, &mut buf).ok()?;
        buf.truncate(n);
        // Drop trailing partial UTF-8 sequence
        while !buf.is_empty() && std::str::from_utf8(&buf).is_err() {
            buf.pop();
        }
        return String::from_utf8(buf).ok();
    }
    std::fs::read_to_string(path).ok()
}

fn recent_commit_messages(limit: usize) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["log", &format!("-{limit}"), "--pretty=format:%s"])
        .output()
        .ok();
    match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

// ── rocky session-end ─────────────────────────────────────────────────────────

/// Cap commits processed per Stop-hook invocation so we never block the user's
/// terminal for minutes while they're trying to exit Claude Code. Anything over
/// the cap stays queued and gets picked up by the next session-end (or
/// `rocky session-end` invoked manually).
const MAX_COMMITS_PER_SESSION_END: usize = 5;

fn run_session_end(
    db: &Db,
    teacher: &Teacher,
    cfg: &Config,
    hours: u32,
    quiet: bool,
) -> Result<()> {
    let project_path = std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .to_string();

    // 1. Drain queued diffs (cap to keep terminal-exit latency bounded)
    let mut pending = db.drain_pending_diffs(&project_path)?;
    if pending.is_empty() {
        if !quiet {
            println!("  {} No pending commits to process.", "·".dimmed());
        }
        return Ok(());
    }

    let deferred: Vec<crate::db::PendingDiff> = if pending.len() > MAX_COMMITS_PER_SESSION_END {
        pending.split_off(MAX_COMMITS_PER_SESSION_END)
    } else {
        Vec::new()
    };

    if !quiet {
        print_header();
        println!("  Processing {} commit(s) from this session...\n", pending.len());
        if !deferred.is_empty() {
            println!(
                "  {} {} additional commit(s) deferred to the next session-end.",
                "·".dimmed(),
                deferred.len()
            );
        }
    }

    // 2. Load project context (may be empty)
    let project_ctx = db.get_project_context(&project_path)?
        .map(|c| c.summary)
        .unwrap_or_default();

    // 3. Read session transcript
    let transcript = transcript::read_recent(
        &std::path::PathBuf::from(&project_path),
        hours,
    ).unwrap_or_default();
    let transcript_block = transcript.to_prompt_block();

    if !quiet && !transcript.is_empty() {
        println!("  {} Loaded transcript: {} prompts, {} agent messages, {} files read",
            "·".dimmed(),
            transcript.user_prompts.len(),
            transcript.assistant_messages.len(),
            transcript.files_read.len(),
        );
    }

    // 4. Build existing topic list (for Layer 1 dedup)
    let existing: Vec<(String, String)> = db.all_nodes()?
        .into_iter()
        .filter(|n| !n.kind.is_domain())
        .map(|n| (n.topic, n.domain))
        .collect();

    let repo = detect_repo_name();
    let mut new_topics = 0u32;
    let mut deduped = 0u32;

    // 5. Process each pending diff
    for pd in &pending {
        if !quiet {
            println!("\n  {} {}", "▸".truecolor(124, 158, 243), pd.commit_msg.lines().next().unwrap_or(""));
        }

        let topics = match teacher.extract_topics_with_dedup(
            &pd.commit_msg,
            &pd.diff,
            &existing,
            &project_ctx,
        ) {
            Ok(t) => t,
            Err(e) => {
                if !quiet {
                    eprintln!("    {} extract failed: {e}", "!".truecolor(226, 75, 74));
                }
                continue;
            }
        };

        for t in topics {
            let topic_id = Db::node_id_static(&t.topic);
            let exists = db.get_node_by_id(&topic_id)?.is_some();

            if exists {
                // Layer 1 dedup hit — record encounter, no node creation
                db.record_topic_encounter(&t.topic, &pd.commit_sha, &repo).ok();
                deduped += 1;
                if !quiet {
                    println!("    {} {} (existing — encounter +1)", "◇".dimmed(), t.topic.dimmed());
                }
                continue;
            }

            // New topic: insert node with rich data
            db.add_or_update(
                &t.topic,
                0.5,
                &Kind::from_str(&t.kind),
                &t.domain,
                &t.description,
                &pd.commit_msg,
                None,
                &repo,
                None,
            )?;

            // Generate question bank using ALL the rich context
            match teacher.generate_question_bank(
                &t.topic,
                &t.description,
                &project_ctx,
                &transcript_block,
                &pd.diff,
            ) {
                Ok(bank) if !bank.is_empty() => {
                    db.set_question_bank(&t.topic, &bank).ok();
                    // Mirror the first question into the canonical_question slot
                    // so the existing quiz flow has something to grab without changes.
                    if let Some(first) = bank.first() {
                        db.set_canonical_qa(&t.topic, &first.question, &first.answer, &first.clue).ok();
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    if !quiet {
                        eprintln!("    {} question bank failed for {}: {e}", "!".truecolor(239, 159, 39), t.topic);
                    }
                }
            }

            // Bump source_commits + repos with this commit SHA / repo
            db.record_topic_encounter(&t.topic, &pd.commit_sha, &repo).ok();
            new_topics += 1;

            if !quiet {
                println!("    {} {}", "+".truecolor(29, 158, 117), t.topic);
            }
        }
    }

    // Re-queue anything we deferred so it isn't lost — the next session-end will
    // pick it up. Done after the main loop in case any of those calls fail; we
    // never want to silently drop a commit's diff.
    for d in &deferred {
        db.queue_pending_diff(&project_path, &d.commit_sha, &d.commit_msg, &d.diff).ok();
    }

    if !quiet {
        println!();
        println!(
            "  {} {new_topics} new topic(s), {deduped} encounter update(s).",
            "Done.".truecolor(29, 158, 117).bold()
        );
        if !deferred.is_empty() {
            println!(
                "  {} {} commit(s) re-queued for the next session-end.",
                "·".dimmed(),
                deferred.len()
            );
        }
        println!("  {}", "Run  rocky quiz  to review the new material.".dimmed());
    }

    auto_sync(db, cfg);
    Ok(())
}
