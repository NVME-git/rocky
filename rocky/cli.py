#!/usr/bin/env python3
"""
Rocky — Personal Knowledge Graph CLI.

Usage:
    rocky "add JWT authentication to my REST API"   # before mode (manual)
    rocky --after "fix: resolve null pointer in auth"  # after a commit
    rocky --stats
    rocky --list
    rocky install    # install git post-commit hook
    rocky uninstall  # remove git hook
"""

import argparse
import re
import sys
from dotenv import load_dotenv

load_dotenv()

from rocky.config import Config
from rocky.graph.store import PKG
from rocky.graph import fsrs
from rocky.capture.session import Session
import rocky.teacher as teacher

MAX_QUESTIONS = 3

# Commit message patterns that indicate a hotfix — Rocky stays quiet
_HOTFIX_RE = re.compile(
    r"^(fix|hotfix|bugfix|patch)(\(.+\))?[!:]|^\[hotfix\]",
    re.IGNORECASE,
)


def color(text: str, code: str) -> str:
    codes = {"green": "32", "yellow": "33", "red": "31", "cyan": "36", "bold": "1", "dim": "2"}
    return f"\033[{codes.get(code, '0')}m{text}\033[0m"


def print_header():
    print(color("\n Rocky", "bold"))
    print(color(" ─────────────────────────────", "dim"))


def _is_hotfix(message: str) -> bool:
    return bool(_HOTFIX_RE.match(message.strip()))


def run_socratic_loop(topic: str, topic_info: dict, task: str,
                      known_topics: list[str], pkg: PKG, session: Session) -> bool:
    """
    Drive the Socratic Q&A loop for a new or poorly understood topic.
    Returns True if a quiz was completed (budget should be decremented).
    """
    print(f"\n{color('Rocky:', 'cyan')} New topic — {topic}")
    print(color(f"  {topic_info['description']}", "dim"))
    print()

    total_score = 0.0
    questions_asked = 0
    question = teacher.generate_question(
        topic, topic_info["description"], task, known_topics, question_num=1
    )

    while questions_asked < MAX_QUESTIONS:
        questions_asked += 1
        print(color(f"Q{questions_asked}. ", "bold") + question)
        print(color("   (Press Enter to skip, type your answer below)", "dim"))

        try:
            answer = input("   > ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n   Skipped.")
            break

        if not answer:
            print(color("   Skipped — topic flagged for review later.", "yellow"))
            pkg.add_or_update(topic, 0.0, kind=topic_info.get("kind", "concept"),
                              description=topic_info["description"], context=task)
            return True

        print(color("   Evaluating...", "dim"))
        result = teacher.evaluate_answer(topic, question, answer, topic_info["description"])
        score = result.get("score", 0.0)
        total_score += score

        print(f"\n   {color(result.get('feedback', ''), 'cyan')}")

        if result.get("understood"):
            print(color("   Added to your PKG.", "green"))
            pkg.add_or_update(topic, score, kind=topic_info.get("kind", "concept"),
                              description=topic_info["description"], context=task)
            return True

        followup = result.get("followup")
        if followup and questions_asked < MAX_QUESTIONS:
            print()
            question = followup
        else:
            break

    avg_score = total_score / max(questions_asked, 1)
    if avg_score >= 0.4:
        print(color("\n   Partial understanding — added to PKG with lower confidence.", "yellow"))
    else:
        print(color("\n   Topic saved — revisit this one before proceeding.", "red"))
    pkg.add_or_update(topic, avg_score, kind=topic_info.get("kind", "concept"),
                      description=topic_info["description"], context=task)
    return True


def run_task(task: str, pkg: PKG, session: Session | None = None, mode: str = "manual"):
    if session is None:
        session = Session()

    session.log_task(task, mode=mode)

    print_header()
    print(f"\n{color('Task:', 'bold')} {task}\n")

    # Hotfix guard — after mode only
    if mode == "after" and _is_hotfix(task):
        print(color("  Hotfix detected — Rocky stepping back.", "dim"))
        return

    print(color("Analyzing topics...", "dim"))
    try:
        topics = teacher.extract_topics(task)
    except Exception as e:
        print(color(f"Error extracting topics: {e}", "red"))
        sys.exit(1)

    if not topics:
        print(color("No significant topics found. Proceed freely.", "green"))
        return

    known_topic_names = [
        n.topic for n in pkg.all_topics()
        if fsrs.classify(fsrs.retrievability(n.stability, n.last_reviewed)) == "known"
    ]

    new_count = 0
    stale_count = 0
    queued = []  # topics over budget, to report at the end

    # Check session gates before starting any quiz
    quiz_allowed, block_reason = session.can_quiz()
    budget = session.budget_remaining() if quiz_allowed else 0
    quizzed = 0

    for topic_info in topics:
        topic = topic_info["topic"]
        classification = pkg.classify(topic)

        if classification == "known":
            r = pkg.retrievability(topic)
            print(color(f"  ✓ {topic}", "green") +
                  color(f" ({r:.0%})", "dim"))
            pkg.mark_encountered(topic)

        elif classification == "stale":
            stale_count += 1
            node = pkg.get(topic)
            r = pkg.retrievability(topic)
            print(color(f"  ~ {topic}", "yellow") +
                  color(f" (recall faded to {r:.0%})", "dim"))

            if quiz_allowed and quizzed < budget:
                print(color("  Refreshing...", "dim"))
                reminder = teacher.generate_reminder(topic, node, task)
                print(f"\n  {color('Rocky:', 'yellow')} {reminder}\n")
                pkg.add_or_update(topic, 0.5, kind=topic_info.get("kind", node.kind), context=task)
                session.record_quiz()
                quizzed += 1
            else:
                queued.append(topic)

        else:  # new
            new_count += 1
            if quiz_allowed and quizzed < budget:
                completed = run_socratic_loop(topic, topic_info, task, known_topic_names, pkg, session)
                if completed:
                    session.record_quiz()
                    quizzed += 1
            else:
                queued.append(topic)
                pkg.add_or_update(topic, 0.0, kind=topic_info.get("kind", "concept"),
                                  description=topic_info["description"], context=task)

    print()

    if queued:
        print(color(f"  Queued for next session: {', '.join(queued)}", "dim"))

    if not quiz_allowed and (new_count > 0 or stale_count > 0):
        print(color(f"  {block_reason}", "yellow"))
    elif new_count == 0 and stale_count == 0:
        print(color("All topics are in your PKG. You're good to go.", "green"))

    stats = pkg.summary()
    print(color(f"  PKG: {stats['known']} known | {stats['stale']} fading | {stats['total']} total", "dim"))


def show_stats(pkg: PKG, config: "Config | None" = None):
    print_header()
    stats = pkg.summary()
    print(f"\n  Total topics:  {stats['total']}")
    print(color(f"  Known:         {stats['known']}", "green"))
    print(color(f"  Fading:        {stats['stale']}", "yellow"))
    print(color(f"  Gaps/weak:     {stats['gaps']}", "red"))

    if config:
        session = Session(daily_budget=config.daily_budget, min_gap_minutes=config.min_gap_minutes)
        budget = session.budget_remaining()
        allowed, reason = session.can_quiz()
        if not allowed:
            print(color(f"\n  {reason}", "dim"))
        else:
            print(color(f"\n  Quiz budget: {budget}/{config.daily_budget} remaining today"
                        f"  ·  provider: {config.llm_provider} ({config.llm_model})", "dim"))
    print()


def list_topics(pkg: PKG):
    print_header()
    nodes = pkg.all_topics()
    if not nodes:
        print("\n  PKG is empty. Run a task to populate it.")
        return

    print(f"\n  {'Topic':<35} {'Kind':<15} {'Recall':<12} {'Last Reviewed'}")
    print(color("  " + "─" * 75, "dim"))

    for node in sorted(nodes, key=lambda n: fsrs.retrievability(n.stability, n.last_reviewed), reverse=True):
        r = fsrs.retrievability(node.stability, node.last_reviewed)
        classification = fsrs.classify(r)
        color_code = "green" if classification == "known" else "yellow" if classification == "stale" else "red"
        bar = "█" * int(r * 10) + "░" * (10 - int(r * 10))
        print(f"  {color(node.topic[:34], color_code):<35} {node.kind:<15} {bar} {r:.0%}  {node.last_reviewed}")
    print()


def main():
    parser = argparse.ArgumentParser(
        prog="rocky",
        description="Rocky — understand what your agents build.",
    )
    subparsers = parser.add_subparsers(dest="subcommand")

    # rocky install / uninstall / config
    subparsers.add_parser("install", help="Install git post-commit hook")
    subparsers.add_parser("uninstall", help="Remove git post-commit hook")
    subparsers.add_parser("config", help="Show active configuration")

    # rocky [task] [flags]
    parser.add_argument("task", nargs="?", help="Task description to analyze")
    parser.add_argument("--after", metavar="MSG",
                        help="After-mode: review topics from a commit message or completed task")
    parser.add_argument("--stats", action="store_true", help="Show PKG stats")
    parser.add_argument("--list", action="store_true", help="List all topics in your PKG")

    args = parser.parse_args()

    if args.subcommand == "install":
        from rocky.capture.hooks import install_git_hook
        ok, msg = install_git_hook()
        print(color(f"  {'✓' if ok else '✗'} {msg}", "green" if ok else "red"))
        return

    if args.subcommand == "uninstall":
        from rocky.capture.hooks import uninstall_git_hook
        ok, msg = uninstall_git_hook()
        print(color(f"  {'✓' if ok else '✗'} {msg}", "green" if ok else "red"))
        return

    config = Config.load()
    teacher.configure(config.create_provider())

    if args.subcommand == "config":
        config.show()
        return

    from pathlib import Path
    pkg = PKG(vault_dir=Path(config.obsidian_vault))
    session = Session(
        daily_budget=config.daily_budget,
        min_gap_minutes=config.min_gap_minutes,
    )

    if args.stats:
        show_stats(pkg, config)
    elif args.list:
        list_topics(pkg)
    elif args.after:
        run_task(args.after, pkg, session, mode="after")
    elif args.task:
        run_task(args.task, pkg, session, mode="manual")
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
