mod config;
mod db;
mod fsrs;
mod local_log;
mod node;
mod obsidian;
mod personality;
mod session;
mod sync;
mod teacher;

use std::io::{self, Read as _, Write as _};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

use config::Config;
use db::Db;
use node::Kind;
use session::Session;
use teacher::{Teacher, TopicInfo};

const MAX_QUESTIONS: u32 = 3;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "rocky",
    about = "Rocky — Personal Knowledge Graph\n\n  ♫  Stay sharp. Stay human.\n\n  Quiz yourself on what your AI agent just built so you never lose the thread.",
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
    List,
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
    /// Show active configuration
    Config,
    /// On-demand quiz — general review, or search for specific topics
    Quiz {
        /// Search for specific topics to quiz (substring match). Omit for full review.
        topic: Option<String>,
        /// Look back N hours for prompt context (default: 24)
        #[arg(long, default_value = "24")]
        hours: u32,
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
        query: String,
    },
    /// Export all PKG topics to your PKG directory (Obsidian-compatible markdown)
    Export,
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
}

#[derive(Subcommand)]
enum HookTarget {
    /// Git post-commit hook — runs `rocky diff` after every commit (default)
    Git,
    /// Claude Code hook — logs prompts to `.rocky` for `rocky quiz` to review
    Claude,
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {e:#}", "error:".red().bold());
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
                        println!("  {} {msg}", "✓".green());
                        println!();
                        println!("  {}", "Rocky will run after every commit in this repo.".dimmed());
                        println!("  {}", "Use  rocky quiz  for an on-demand session anytime.".dimmed());
                    } else {
                        println!("  {} {msg}", "✗".red());
                    }
                }
                HookTarget::Claude => {
                    let (ok, msg) = install_claude_hook()?;
                    if ok {
                        p.banner();
                        println!("  {} {msg}", "✓".green());
                        println!();
                        println!("  {}", "Rocky will silently log every Claude Code prompt.".dimmed());
                        println!("  {}", "Run  rocky quiz  to review topics from recent sessions.".dimmed());
                    } else {
                        println!("  {} {msg}", "✗".red());
                    }
                }
            }
        }
        Some(Cmd::Uninstall { target }) => {
            match target.unwrap_or(HookTarget::Git) {
                HookTarget::Git => {
                    let (ok, msg) = uninstall_git_hook()?;
                    if ok {
                        println!("  {} {msg}", "✓".green());
                    } else {
                        println!("  {} {msg}", "✗".red());
                    }
                }
                HookTarget::Claude => {
                    let (ok, msg) = uninstall_claude_hook()?;
                    if ok {
                        println!("  {} {msg}", "✓".green());
                    } else {
                        println!("  {} {msg}", "✗".red());
                    }
                }
            }
        }
        Some(Cmd::Config) => cfg.show(),
        Some(Cmd::Quiz { topic, hours }) => {
            let teacher = make_teacher(&cfg)?;
            let session = Session::new(
                Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                cfg.daily_budget,
                cfg.min_gap_minutes,
            );
            if let Some(query) = topic {
                run_quiz_topic(&db, &teacher, &session, &p, &query)?;
            } else {
                run_quiz(&db, &teacher, &session, &p, hours)?;
            }
            auto_sync(&db, &cfg);
        }
        Some(Cmd::Stats) => show_stats(&db, &cfg, &p)?,
        Some(Cmd::List) => list_topics(&db)?,
        Some(Cmd::Delete { query }) => delete_topics(&db, &cfg.pkg_dir, &query)?,
        Some(Cmd::Export) => run_export(&db, &cfg)?,
        Some(Cmd::Logs) => run_logs()?,
        Some(Cmd::Diff { git_ref, staged }) => {
            let teacher = make_teacher(&cfg)?;
            let session = Session::new(
                Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                cfg.daily_budget,
                cfg.min_gap_minutes,
            );
            run_diff(&db, &teacher, &session, &p, git_ref.as_deref(), staged)?;
            auto_sync(&db, &cfg);
        }
        Some(Cmd::Hook) => unreachable!(),
        Some(Cmd::Sync { init, push, status }) => {
            run_sync(&db, &cfg, init, push, status)?;
        }
        Some(Cmd::Restore) => {
            let pkg_json = cfg.pkg_dir.join("pkg.json");
            let count = db.import_pkg_json(&pkg_json)?;
            println!("  {} Restored {count} topics from {}", "✓".green(), pkg_json.display());
            println!("  {}", "Run  rocky export  to sync PKG files.".dimmed());
        }
        Some(Cmd::Classify) => {
            run_classify(&db, &make_teacher(&cfg)?)?;
        }
        None => {
            if let Some(msg) = cli.after {
                let teacher = make_teacher(&cfg)?;
                let session = Session::new(
                    Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                    cfg.daily_budget,
                    cfg.min_gap_minutes,
                );
                run_task(&db, &teacher, &session, &p, &msg, "after")?;
                auto_sync(&db, &cfg);
            } else if let Some(task) = cli.task {
                let teacher = make_teacher(&cfg)?;
                let session = Session::new(
                    Db::open(&cfg.db_path, &cfg.pkg_dir)?,
                    cfg.daily_budget,
                    cfg.min_gap_minutes,
                );
                run_task(&db, &teacher, &session, &p, &task, "manual")?;
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
    println!("  {}", format!("Known:         {known}").green());
    println!("  {}", format!("Fading:        {stale}").yellow());
    println!("  {}", format!("Gaps/weak:     {gaps}").red());

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
    if let Some(mood) = p.pkg_mood(known, total) {
        println!("\n  {mood}");
    }
    println!();
    Ok(())
}

fn list_topics(db: &Db) -> Result<()> {
    print_header();
    let nodes = db.all_nodes()?;
    if nodes.is_empty() {
        println!("\n  PKG is empty. Run a task to populate it.");
        return Ok(());
    }

    println!(
        "\n  {:<35} {:<14} {:<14} {:<7} {:<7} {:<7} {}",
        "Topic", "Kind", "Recall", "Stab", "Diff", "Reviews", "Last Reviewed"
    );
    println!("  {}", "─".repeat(100).dimmed());

    let mut sorted = nodes;
    sorted.sort_by(|a, b| {
        let ra = fsrs::retrievability(a.stability, a.last_reviewed);
        let rb = fsrs::retrievability(b.stability, b.last_reviewed);
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });

    for node in sorted {
        let r = fsrs::retrievability(node.stability, node.last_reviewed);
        let cls = fsrs::classify(r);
        let filled = (r * 10.0) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
        let topic_str = &node.topic[..node.topic.len().min(34)];
        // Stability: shown as days (how long until recall would hit 90%)
        let stab_str = format!("{:.1}d", node.stability);
        // Difficulty: 0.0–1.0, lower is easier
        let diff_str = format!("{:.2}", node.difficulty);
        let line = format!(
            "  {:<35} {:<14} {} {:.0}%  {:<7} {:<7} {:<7} {}",
            topic_str,
            node.kind.as_str(),
            bar,
            r * 100.0,
            stab_str,
            diff_str,
            node.review_count,
            node.last_reviewed
        );
        let colored = match cls {
            "known" => line.green().to_string(),
            "stale" => line.yellow().to_string(),
            _ => line.red().to_string(),
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

fn run_task(db: &Db, teacher: &Teacher, session: &Session, p: &personality::Personality, task: &str, mode: &str) -> Result<()> {
    print_header();
    println!("\n{} {task}\n", "Task:".bold());

    if mode == "after" && is_hotfix(task) {
        println!("  {}", "Hotfix detected — Rocky stepping back.".dimmed());
        return Ok(());
    }

    println!("{}", "Analyzing topics...".dimmed());
    let topics = teacher
        .extract_topics(task)
        .context("Failed to extract topics from task")?;

    if topics.is_empty() {
        println!("{}", "No significant topics found. Proceed freely.".green());
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
                    format!("  ✓ {topic}").green(),
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
                    format!("  ~ {topic}").yellow(),
                    format!("(recall faded to {:.0}%)", r * 100.0).dimmed()
                );
                if quiz_allowed && quizzed < budget {
                    println!("{}", "  Refreshing...".dimmed());
                    let reminder = teacher.generate_reminder(topic, n, task)?;
                    println!("\n  {} {reminder}\n", "Rocky:".yellow().bold());
                    db.add_or_update(
                        topic,
                        0.5,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.domain,
                        &topic_info.description,
                        task,
                        None,
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
                    let completed =
                        run_socratic_loop(db, teacher, topic_info, task, &known_topic_names, p)?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                } else {
                    queued.push(topic.clone());
                    queue_for_later(topic, &topic_info.kind, &topic_info.description, task);
                }
            }
        }
    }

    println!();

    if !queued.is_empty() {
        println!(
            "  {}",
            format!("Queued for next session: {}", queued.join(", ")).dimmed()
        );
    }

    if !quiz_allowed && (new_count > 0 || stale_count > 0) {
        println!("  {}", block_reason.yellow());
    } else if new_count == 0 && stale_count == 0 {
        println!("{}", "All topics are in your PKG. You're good to go.".green());
    }

    let (total, known, stale, _) = db.summary()?;
    println!(
        "  {}",
        format!("PKG: {known} known | {stale} fading | {total} total").dimmed()
    );

    Ok(())
}

fn run_socratic_loop(
    db: &Db,
    teacher: &Teacher,
    topic_info: &TopicInfo,
    task: &str,
    known_topics: &[String],
    p: &personality::Personality,
) -> Result<bool> {
    let topic = &topic_info.topic;
    println!("\n{} New topic — {topic}", "Rocky:".cyan().bold());
    println!("{}", format!("  {}", topic_info.description).dimmed());
    println!();

    let mut total_score = 0.0f64;
    let mut questions_asked = 0u32;
    let mut last_question = String::new();
    let mut last_answer = String::new();
    let mut question = teacher.generate_question(
        topic,
        &topic_info.description,
        task,
        known_topics,
        1,
    )?;

    while questions_asked < MAX_QUESTIONS {
        questions_asked += 1;
        let display_q = p.format_question(&question);
        println!("{} {display_q}", format!("Q{questions_asked}.").bold());
        println!(
            "{}",
            "   [e] too easy  [?] explain it  [i] not relevant  or type your answer:".dimmed()
        );
        print!("   > ");
        io::stdout().flush()?;

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Err(_) | Ok(0) => {
                println!("\n   Skipped.");
                break;
            }
            Ok(_) => {}
        }
        let answer = line.trim();

        // Skip / quit — queue for later without touching PKG
        if answer.is_empty() {
            if let Some(msg) = p.skipped() { println!("   {msg}"); }
            else { println!("{}", "   Skipped — topic queued for next session.".yellow()); }
            queue_for_later(topic, topic_info.kind.as_str(), &topic_info.description, task);
            return Ok(true);
        }

        // Ignore — LLM hallucinated or topic is irrelevant, discard silently
        if answer.eq_ignore_ascii_case("i") {
            if let Some(msg) = p.ignored() { println!("   {msg}"); }
            else { println!("{}", "   Ignored — not added to PKG.".dimmed()); }
            return Ok(false);
        }

        // Too easy — self-report high confidence
        if answer.eq_ignore_ascii_case("e") {
            db.add_or_update(
                topic,
                0.75,
                &Kind::from_str(&topic_info.kind),
                &topic_info.domain,
                &topic_info.description,
                task,
                None,
            )?;
            if let Some(msg) = p.too_easy() { println!("   {msg}"); }
            else { println!("{}", "   Marked as known.".green()); }
            print_milestone(db, p, topic);
            return Ok(true);
        }

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
            println!("\n   {}\n", explanation.cyan());
            println!("{}", "   Saved to PKG — revisit before your next task.".yellow());
            db.add_or_update(
                topic,
                0.2,
                &Kind::from_str(&topic_info.kind),
                &topic_info.domain,
                &topic_info.description,
                task,
                None,
            )?;
            return Ok(true);
        }

        // Normal answer — evaluate it
        last_question = question.clone();
        last_answer = answer.to_string();

        println!("{}", "   Evaluating...".dimmed());
        let result =
            teacher.evaluate_answer(topic, &question, answer, &topic_info.description)?;
        total_score += result.score;

        println!("\n   {}", result.feedback.cyan());

        if result.understood {
            db.add_or_update(
                topic,
                result.score,
                &Kind::from_str(&topic_info.kind),
                &topic_info.domain,
                &topic_info.description,
                task,
                None,
            )?;
            if let Some(msg) = p.correct() { println!("   {msg}"); }
            else { println!("{}", "   Added to your PKG.".green()); }
            print_milestone(db, p, topic);
            return Ok(true);
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
    println!("{}", "\n   Let me walk you through this one.\n".yellow());
    let explanation = teacher.generate_explanation(
        topic,
        &topic_info.description,
        &last_question,
        &last_answer,
        task,
    )?;
    println!("   {}\n", explanation.cyan());

    if let Some(msg) = p.failed() { println!("   {msg}"); }
    else if avg_score >= 0.4 {
        println!("{}", "   Added to PKG with partial confidence — you're on the right track.".yellow());
    } else {
        println!("{}", "   Added to PKG — come back to this one.".red());
    }
    db.add_or_update(
        topic,
        avg_score,
        &Kind::from_str(&topic_info.kind),
        &topic_info.domain,
        &topic_info.description,
        task,
        None,
    )?;
    Ok(true)
}

fn run_quiz_topic(db: &Db, teacher: &Teacher, session: &Session, p: &personality::Personality, query: &str) -> Result<()> {
    print_header();
    p.print_rocky(false);

    let matches = db.search_nodes(query)?;
    if matches.is_empty() {
        println!("\n  {} No topics found matching \"{}\".", "✗".red(), query);
        return Ok(());
    }

    println!("\n  Topics matching \"{}\":\n", query);
    for (i, node) in matches.iter().enumerate() {
        let r = fsrs::retrievability(node.stability, node.last_reviewed);
        let cls = fsrs::classify(r);
        let label = format!("  [{}]  {} ({:.0}%)", i + 1, node.topic, r * 100.0);
        let colored = match cls {
            "known" => label.green().to_string(),
            "stale" => label.yellow().to_string(),
            _ => label.red().to_string(),
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
        let completed = run_socratic_loop(db, teacher, &topic_info, context, &known_topic_names, p)?;
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

fn run_quiz(db: &Db, teacher: &Teacher, session: &Session, p: &personality::Personality, hours: u32) -> Result<()> {
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
    let all_nodes = db.all_nodes()?;
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
        println!("{}", "\n  Nothing to review — all topics are solid.".green());
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
            let completed = run_socratic_loop(db, teacher, &topic_info, context, &known_topic_names, p)?;
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
                format!("  ~ {topic}").yellow(),
                format!("(faded to {:.0}%)", r * 100.0).dimmed()
            );
            let reminder = teacher.generate_reminder(topic, node, context)?;
            println!("\n  {} {reminder}\n", "Rocky:".yellow().bold());
            db.add_or_update(
                topic, 0.5, &node.kind, &node.domain, &node.description, context, None,
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
            let completed = run_socratic_loop(db, teacher, &topic_info, context, &known_topic_names, p)?;
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
            let completed = run_socratic_loop(db, teacher, topic_info, combined, &known_topic_names, p)?;
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

fn delete_topics(db: &Db, pkg_dir: &std::path::Path, query: &str) -> Result<()> {
    let matches = db.search_nodes(query)?;

    if matches.is_empty() {
        println!("  {} No topics found matching \"{}\".", "✗".red(), query);
        return Ok(());
    }

    println!("\n  Matches for \"{}\":\n", query);
    for (i, node) in matches.iter().enumerate() {
        let r = fsrs::retrievability(node.stability, node.last_reviewed);
        let cls = fsrs::classify(r);
        let color_fn: fn(&str) -> colored::ColoredString = match cls {
            "known" => |s| s.green(),
            "stale" => |s| s.yellow(),
            _ => |s| s.red(),
        };
        println!(
            "  {}  {} {}",
            format!("[{}]", i + 1).bold(),
            color_fn(&node.topic),
            format!("({:.0}% recall · {})", r * 100.0, node.kind.as_str()).dimmed(),
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
            println!("  {} Invalid selection.", "✗".red());
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
        println!("  {} Deleted \"{}\".", "✓".green(), node.topic);
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
        println!("{}", "No significant topics found in this diff.".green());
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
                    format!("  ✓ {topic}").green(),
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
                    format!("  ~ {topic}").yellow(),
                    format!("(recall faded to {:.0}%)", r * 100.0).dimmed()
                );
                if quiz_allowed && quizzed < budget {
                    println!("{}", "  Refreshing...".dimmed());
                    let reminder = teacher.generate_reminder(topic, n, &label)?;
                    println!("\n  {} {reminder}\n", "Rocky:".yellow().bold());
                    db.add_or_update(
                        topic,
                        0.5,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.domain,
                        &topic_info.description,
                        &label,
                        None,
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
                    let completed =
                        run_socratic_loop(db, teacher, topic_info, &label, &known_topic_names, p)?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                } else {
                    queued.push(topic.clone());
                    queue_for_later(topic, &topic_info.kind, &topic_info.description, &label);
                }
            }
        }
    }

    println!();
    if !queued.is_empty() {
        println!(
            "  {}",
            format!("Queued for next session: {}", queued.join(", ")).dimmed()
        );
    }
    if !quiz_allowed && (new_count > 0 || stale_count > 0) {
        println!("  {}", block_reason.yellow());
    } else if new_count == 0 && stale_count == 0 {
        println!("{}", "All topics already in your PKG.".green());
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
            "✓".green(),
            cfg.pkg_dir.display()
        );
        return Ok(());
    }
    let count = obsidian::write_all(&nodes, &cfg.pkg_dir)?;
    println!(
        "  {} Exported {count} topic{} + dashboard to {}",
        "✓".green(),
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

fn install_git_hook() -> Result<(bool, String)> {
    let hook_path = std::path::Path::new(".git/hooks/post-commit");
    if !std::path::Path::new(".git").exists() {
        return Ok((false, "not a git repository".into()));
    }
    if hook_path.exists() {
        let existing = std::fs::read_to_string(hook_path)?;
        if existing.contains("rocky") {
            return Ok((false, "hook already installed".into()));
        }
        let appended = format!("{existing}\nrocky diff\n");
        std::fs::write(hook_path, appended)?;
    } else {
        std::fs::write(hook_path, "#!/bin/sh\nrocky diff\n")?;
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

    let entry = serde_json::json!({"command": "rocky hook"});
    if arr.iter().any(|v| v == &entry) {
        return Ok((false, "Claude Code hook already installed".into()));
    }

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

    let entry = serde_json::json!({"command": "rocky hook"});
    let removed = if let Some(arr) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("UserPromptSubmit"))
        .and_then(|v| v.as_array_mut())
    {
        let before = arr.len();
        arr.retain(|v| v != &entry);
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
            println!("  {} Initialised git repo at {}", "✓".green(), rocky_dir.display());
        } else {
            println!("  {} Git repo already exists", "✓".green());
        }
        if let Some(url) = remote_url {
            sync::set_remote(rocky_dir, &cfg.sync.remote, &url)?;
            println!("  {} Remote '{}' set to {url}", "✓".green(), cfg.sync.remote);
            println!("  {}", "Run `rocky sync --push` to push your PKG.".dimmed());
        } else {
            println!("  {}", format!("Add a remote: rocky sync --init <url>  or  git -C {} remote add origin <url>", rocky_dir.display()).dimmed());
        }
        return Ok(());
    }

    if !sync::is_git_repo(rocky_dir) {
        println!("  {} Vault is not a git repo — run `rocky sync --init` first.", "✗".red());
        return Ok(());
    }

    // Write pkg.json then commit
    let nodes = db.all_nodes()?;
    obsidian::write_all(&nodes, &cfg.pkg_dir)?;
    db.export_pkg_json(&cfg.pkg_dir.join("pkg.json"))?;

    let msg = build_commit_message(db);
    match sync::commit(rocky_dir, &msg)? {
        true  => println!("  {} {msg}", "✓".green()),
        false => println!("  {} Nothing changed since last commit.", "·".dimmed()),
    }

    if push {
        sync::push(rocky_dir, &cfg.sync.remote, &cfg.sync.branch)?;
        sync::reset_push_counter(db)?;
        println!("  {} Pushed to {}/{}", "✓".green(), cfg.sync.remote, cfg.sync.branch);
    }

    Ok(())
}

fn run_classify(db: &Db, teacher: &Teacher) -> Result<()> {
    let undomained = db.undomained_nodes()?;
    if undomained.is_empty() {
        println!("  {} All topics already have a domain.", "✓".green());
        return Ok(());
    }
    println!("  Classifying {} topic{} into domains...", undomained.len(), if undomained.len() == 1 { "" } else { "s" });
    let pairs: Vec<(String, String)> = undomained.iter()
        .map(|n| (n.topic.clone(), n.description.clone()))
        .collect();
    let results = teacher.classify_domains(&pairs)?;
    for (topic, domain) in &results {
        let node_id = topic.to_lowercase().trim().replace(' ', "-");
        db.set_domain(&node_id, domain)?;
        println!("  {} {} → {}", "✓".green(), topic, domain.cyan());
    }
    println!("  {} Run `rocky export` to update PKG files.", "·".dimmed());
    Ok(())
}

/// Auto-sync after a session: export PKG, write pkg.json, commit if configured.
fn auto_sync(db: &Db, cfg: &Config) {
    if !cfg.sync.enabled || !cfg.sync.auto_commit { return; }
    if !sync::is_git_repo(&cfg.rocky_dir) { return; }

    let nodes = match db.all_nodes() { Ok(n) => n, Err(_) => return };
    obsidian::write_all(&nodes, &cfg.pkg_dir).ok();
    db.export_pkg_json(&cfg.pkg_dir.join("pkg.json")).ok();

    let msg = build_commit_message(db);
    match sync::commit(&cfg.rocky_dir, &msg) {
        Ok(true) => {
            if cfg.sync.commit_visible {
                println!("  {} {}", "✓".green(), msg);
            }
            sync::increment_sessions_since_push(db).ok();
        }
        Ok(false) => {} // nothing to commit
        Err(_) => {}    // best-effort
    }

    // Push reminder
    if let Some(reminder) = sync::push_reminder(db, &cfg.sync) {
        println!("  {}", reminder.yellow());
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

fn print_header() {
    println!("\n{}  {}", " ♫".cyan(), "Rocky · Personal Knowledge Graph".bold());
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
