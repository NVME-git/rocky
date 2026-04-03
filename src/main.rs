mod config;
mod db;
mod fsrs;
mod node;
mod obsidian;
mod session;
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
#[command(name = "rocky", about = "Rocky — understand what your agents build.")]
struct Cli {
    /// Task description to analyze before starting work
    task: Option<String>,

    /// After-mode: review topics from a completed task or commit message
    #[arg(long, value_name = "MSG")]
    after: Option<String>,

    /// Show PKG stats
    #[arg(long)]
    stats: bool,

    /// List all topics in your PKG
    #[arg(long)]
    list: bool,

    #[command(subcommand)]
    subcommand: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Install git post-commit hook
    Install,
    /// Remove git post-commit hook
    Uninstall,
    /// Show active configuration
    Config,
    /// On-demand quiz from recent AI agent tasks
    Quiz {
        /// Look back N hours for tasks (default: 24)
        #[arg(long, default_value = "24")]
        hours: u32,
    },
    /// Claude Code hook — reads JSON from stdin, logs prompt (non-blocking)
    Hook,
    /// Search for and delete topics from your PKG
    Delete {
        /// Search query (substring match on topic name and description)
        query: String,
    },
    /// Export all PKG topics to your Obsidian vault
    Export,
    /// Analyze a git diff and quiz on topics found in the code changes
    Diff {
        /// Git ref to diff against HEAD (e.g. HEAD~1, main). Defaults to last commit.
        #[arg(value_name = "REF")]
        git_ref: Option<String>,
        /// Analyze staged changes instead of a commit
        #[arg(long)]
        staged: bool,
    },
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
    let db = Db::open(&cfg.db_path, &cfg.obsidian_vault)?;

    match cli.subcommand {
        Some(Cmd::Install) => {
            let (ok, msg) = install_git_hook()?;
            if ok {
                println!("  {} {msg}", "✓".green());
            } else {
                println!("  {} {msg}", "✗".red());
            }
        }
        Some(Cmd::Uninstall) => {
            let (ok, msg) = uninstall_git_hook()?;
            if ok {
                println!("  {} {msg}", "✓".green());
            } else {
                println!("  {} {msg}", "✗".red());
            }
        }
        Some(Cmd::Config) => cfg.show(),
        Some(Cmd::Quiz { hours }) => {
            let teacher = make_teacher(&cfg)?;
            let session = Session::new(
                Db::open(&cfg.db_path, &cfg.obsidian_vault)?,
                cfg.daily_budget,
                cfg.min_gap_minutes,
            );
            run_quiz(&db, &teacher, &session, hours)?;
        }
        Some(Cmd::Delete { query }) => delete_topics(&db, &query)?,
        Some(Cmd::Export) => run_export(&db, &cfg)?,
        Some(Cmd::Diff { git_ref, staged }) => {
            let teacher = make_teacher(&cfg)?;
            let session = Session::new(
                Db::open(&cfg.db_path, &cfg.obsidian_vault)?,
                cfg.daily_budget,
                cfg.min_gap_minutes,
            );
            run_diff(&db, &teacher, &session, git_ref.as_deref(), staged)?;
        }
        Some(Cmd::Hook) => unreachable!(),
        None => {
            if cli.stats {
                show_stats(&db, &cfg)?;
            } else if cli.list {
                list_topics(&db)?;
            } else if let Some(msg) = cli.after {
                let teacher = make_teacher(&cfg)?;
                let session = Session::new(
                    Db::open(&cfg.db_path, &cfg.obsidian_vault)?,
                    cfg.daily_budget,
                    cfg.min_gap_minutes,
                );
                run_task(&db, &teacher, &session, &msg, "after")?;
            } else if let Some(task) = cli.task {
                let teacher = make_teacher(&cfg)?;
                let session = Session::new(
                    Db::open(&cfg.db_path, &cfg.obsidian_vault)?,
                    cfg.daily_budget,
                    cfg.min_gap_minutes,
                );
                run_task(&db, &teacher, &session, &task, "manual")?;
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

fn show_stats(db: &Db, cfg: &Config) -> Result<()> {
    print_header();
    let (total, known, stale, gaps) = db.summary()?;
    println!("\n  Total topics:  {total}");
    println!("  {}", format!("Known:         {known}").green());
    println!("  {}", format!("Fading:        {stale}").yellow());
    println!("  {}", format!("Gaps/weak:     {gaps}").red());

    let session = Session::new(
        Db::open(&cfg.db_path, &cfg.obsidian_vault)?,
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
        "\n  {:<35} {:<15} {:<12} {}",
        "Topic", "Kind", "Recall", "Last Reviewed"
    );
    println!("  {}", "─".repeat(75).dimmed());

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
        let line = format!(
            "  {:<35} {:<15} {} {:.0}%  {}",
            topic_str,
            node.kind.as_str(),
            bar,
            r * 100.0,
            node.last_reviewed
        );
        let colored = match cls {
            "known" => line.green().to_string(),
            "stale" => line.yellow().to_string(),
            _ => line.red().to_string(),
        };
        println!("{colored}");
    }
    println!();
    Ok(())
}

fn run_task(db: &Db, teacher: &Teacher, session: &Session, task: &str, mode: &str) -> Result<()> {
    session.log_task(task, mode)?;
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

    let (quiz_allowed, block_reason) = session.can_quiz()?;
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
                        &topic_info.description,
                        task,
                        None,
                    )?;
                    session.record_quiz()?;
                    quizzed += 1;
                } else {
                    queued.push(topic.clone());
                }
            }
            _ => {
                new_count += 1;
                if quiz_allowed && quizzed < budget {
                    let completed =
                        run_socratic_loop(db, teacher, topic_info, task, &known_topic_names)?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                } else {
                    queued.push(topic.clone());
                    db.add_or_update(
                        topic,
                        0.0,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.description,
                        task,
                        None,
                    )?;
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
) -> Result<bool> {
    let topic = &topic_info.topic;
    println!("\n{} New topic — {topic}", "Rocky:".cyan().bold());
    println!("{}", format!("  {}", topic_info.description).dimmed());
    println!();

    let mut total_score = 0.0f64;
    let mut questions_asked = 0u32;
    let mut question = teacher.generate_question(
        topic,
        &topic_info.description,
        task,
        known_topics,
        1,
    )?;

    while questions_asked < MAX_QUESTIONS {
        questions_asked += 1;
        println!("{} {question}", format!("Q{questions_asked}.").bold());
        println!("{}", "   (Press Enter to skip, type your answer below)".dimmed());
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

        if answer.is_empty() {
            println!("{}", "   Skipped — topic flagged for review later.".yellow());
            db.add_or_update(
                topic,
                0.0,
                &Kind::from_str(&topic_info.kind),
                &topic_info.description,
                task,
                None,
            )?;
            return Ok(true);
        }

        println!("{}", "   Evaluating...".dimmed());
        let result =
            teacher.evaluate_answer(topic, &question, answer, &topic_info.description)?;
        total_score += result.score;

        println!("\n   {}", result.feedback.cyan());

        if result.understood {
            println!("{}", "   Added to your PKG.".green());
            db.add_or_update(
                topic,
                result.score,
                &Kind::from_str(&topic_info.kind),
                &topic_info.description,
                task,
                None,
            )?;
            return Ok(true);
        }

        if let Some(followup) = result.followup {
            if questions_asked < MAX_QUESTIONS {
                println!();
                question = followup;
                continue;
            }
        }
        break;
    }

    let avg_score = total_score / questions_asked.max(1) as f64;
    if avg_score >= 0.4 {
        println!("{}", "\n   Partial understanding — added to PKG with lower confidence.".yellow());
    } else {
        println!("{}", "\n   Topic saved — revisit this one before proceeding.".red());
    }
    db.add_or_update(
        topic,
        avg_score,
        &Kind::from_str(&topic_info.kind),
        &topic_info.description,
        task,
        None,
    )?;
    Ok(true)
}

fn run_quiz(db: &Db, teacher: &Teacher, session: &Session, hours: u32) -> Result<()> {
    print_header();

    let tasks = db.recent_tasks(hours)?;
    if tasks.is_empty() {
        println!(
            "{}",
            format!("\n  No Claude Code prompts logged in the last {hours}h.").dimmed()
        );
        println!("{}", "  Run some tasks in Claude Code first, then come back.".dimmed());
        println!();
        return Ok(());
    }

    println!(
        "{}",
        format!(
            "\n  Reviewing {} prompt{} from the last {hours}h...",
            tasks.len(),
            if tasks.len() == 1 { "" } else { "s" }
        )
        .dimmed()
    );

    let combined = tasks[..tasks.len().min(10)].join(" | ");

    let (quiz_allowed, block_reason) = session.can_quiz()?;
    if !quiz_allowed {
        println!("\n  {}", block_reason.yellow());
        println!();
        return Ok(());
    }

    println!("{}", "  Analyzing topics...".dimmed());
    let topics = teacher.extract_topics(&combined)?;

    if topics.is_empty() {
        println!("{}", "\n  No significant topics found in recent prompts.".green());
        println!();
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

    let budget = session.budget_remaining()?;
    let mut quizzed = 0u32;
    let mut queued: Vec<String> = Vec::new();

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
                    format!("\n  ✓ {topic}").green(),
                    format!("({:.0}% recall)", r * 100.0).dimmed()
                );
                db.mark_encountered(topic)?;
            }
            "stale" => {
                if quizzed < budget {
                    let n = node.as_ref().unwrap();
                    let r = fsrs::retrievability(n.stability, n.last_reviewed);
                    println!(
                        "{} {}",
                        format!("\n  ~ {topic}").yellow(),
                        format!("(faded to {:.0}%)", r * 100.0).dimmed()
                    );
                    let reminder = teacher.generate_reminder(topic, n, &combined)?;
                    println!("\n  {} {reminder}\n", "Rocky:".yellow().bold());
                    db.add_or_update(
                        topic,
                        0.5,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.description,
                        &combined,
                        None,
                    )?;
                    session.record_quiz()?;
                    quizzed += 1;
                } else {
                    queued.push(topic.clone());
                }
            }
            _ => {
                if quizzed < budget {
                    let completed = run_socratic_loop(
                        db,
                        teacher,
                        topic_info,
                        &combined,
                        &known_topic_names,
                    )?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                } else {
                    queued.push(topic.clone());
                    db.add_or_update(
                        topic,
                        0.0,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.description,
                        &combined,
                        None,
                    )?;
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

    let (total, known, stale, _) = db.summary()?;
    println!(
        "  {}",
        format!("PKG: {known} known | {stale} fading | {total} total").dimmed()
    );
    println!();
    Ok(())
}

fn delete_topics(db: &Db, query: &str) -> Result<()> {
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

    let (quiz_allowed, block_reason) = session.can_quiz()?;
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
                        &topic_info.description,
                        &label,
                        None,
                    )?;
                    session.record_quiz()?;
                    quizzed += 1;
                } else {
                    queued.push(topic.clone());
                }
            }
            _ => {
                new_count += 1;
                if quiz_allowed && quizzed < budget {
                    let completed =
                        run_socratic_loop(db, teacher, topic_info, &label, &known_topic_names)?;
                    if completed {
                        session.record_quiz()?;
                        quizzed += 1;
                    }
                } else {
                    queued.push(topic.clone());
                    db.add_or_update(
                        topic,
                        0.0,
                        &Kind::from_str(&topic_info.kind),
                        &topic_info.description,
                        &label,
                        None,
                    )?;
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
        println!("  PKG is empty — nothing to export.");
        return Ok(());
    }
    let count = obsidian::write_all(&nodes, &cfg.obsidian_vault)?;
    println!(
        "  {} Exported {count} topic{} to {}",
        "✓".green(),
        if count == 1 { "" } else { "s" },
        cfg.obsidian_vault.display()
    );
    Ok(())
}

fn run_hook() -> Result<()> {
    let mut input = String::new();
    io::stdin().lock().read_to_string(&mut input).ok();

    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&input) {
        if let Some(prompt) = data.get("prompt").and_then(|v| v.as_str()) {
            let prompt = prompt.trim();
            if !prompt.is_empty() {
                if let Ok(cfg) = Config::load() {
                    if let Ok(db) = Db::open(&cfg.db_path, &cfg.obsidian_vault) {
                        db.log_task(prompt, "claude-code").ok();
                    }
                }
            }
        }
    }
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
    Ok((true, "git post-commit hook installed".into()))
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

// ── helpers ───────────────────────────────────────────────────────────────────

fn print_header() {
    println!("\n{}", " Rocky".bold());
    println!("{}", " ─────────────────────────────".dimmed());
}

fn is_hotfix(msg: &str) -> bool {
    let lower = msg.trim().to_lowercase();
    lower.starts_with("fix")
        || lower.starts_with("hotfix")
        || lower.starts_with("bugfix")
        || lower.starts_with("patch")
        || lower.starts_with("[hotfix]")
}
