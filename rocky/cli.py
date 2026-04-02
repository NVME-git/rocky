#!/usr/bin/env python3
"""
Rocky — Personal Knowledge Graph CLI.

Usage:
    rocky "add JWT authentication to my REST API"
    rocky --stats
    rocky --list
"""

import argparse
import sys
from dotenv import load_dotenv

load_dotenv()

from rocky.graph.store import PKG
from rocky.graph import fsrs
import rocky.teacher as teacher

MAX_QUESTIONS = 3


def color(text: str, code: str) -> str:
    codes = {"green": "32", "yellow": "33", "red": "31", "cyan": "36", "bold": "1", "dim": "2"}
    return f"\033[{codes.get(code, '0')}m{text}\033[0m"


def print_header():
    print(color("\n Rocky", "bold"))
    print(color(" ─────────────────────────────", "dim"))


def run_socratic_loop(topic: str, topic_info: dict, task: str,
                      known_topics: list[str], pkg: PKG):
    """Drive the Socratic Q&A loop for a new or poorly understood topic."""
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
            return

        print(color("   Evaluating...", "dim"))
        result = teacher.evaluate_answer(topic, question, answer, topic_info["description"])
        score = result.get("score", 0.0)
        total_score += score

        print(f"\n   {color(result.get('feedback', ''), 'cyan')}")

        if result.get("understood"):
            print(color("   Added to your PKG.", "green"))
            pkg.add_or_update(topic, score, kind=topic_info.get("kind", "concept"),
                              description=topic_info["description"], context=task)
            return

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


def run_task(task: str, pkg: PKG):
    print_header()
    print(f"\n{color('Task:', 'bold')} {task}\n")
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
            print(color("  Refreshing...", "dim"))
            reminder = teacher.generate_reminder(topic, node, task)
            print(f"\n  {color('Rocky:', 'yellow')} {reminder}\n")
            pkg.add_or_update(topic, 0.5, kind=topic_info.get("kind", node.kind),
                              context=task)

        else:
            new_count += 1
            run_socratic_loop(topic, topic_info, task, known_topic_names, pkg)

    print()
    if new_count == 0 and stale_count == 0:
        print(color("All topics are in your PKG. You're good to go.", "green"))
    else:
        stats = pkg.summary()
        print(color(f"PKG: {stats['known']} known | "
                    f"{stats['stale']} fading | {stats['total']} total", "dim"))


def show_stats(pkg: PKG):
    print_header()
    stats = pkg.summary()
    print(f"\n  Total topics:  {stats['total']}")
    print(color(f"  Known:         {stats['known']}", "green"))
    print(color(f"  Fading:        {stats['stale']}", "yellow"))
    print(color(f"  Gaps/weak:     {stats['gaps']}", "red"))
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
    parser.add_argument("task", nargs="?", help="Task description to analyze")
    parser.add_argument("--stats", action="store_true", help="Show PKG stats")
    parser.add_argument("--list", action="store_true", help="List all topics in your PKG")

    args = parser.parse_args()
    pkg = PKG()

    if args.stats:
        show_stats(pkg)
    elif args.list:
        list_topics(pkg)
    elif args.task:
        run_task(args.task, pkg)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
