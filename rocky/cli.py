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

from rocky.graph.store import KnowledgeGraph
import rocky.teacher as teacher

# Confidence adjustments
CONFIDENCE_CORRECT = 0.15
CONFIDENCE_PARTIAL = 0.07
CONFIDENCE_INITIAL = 0.45
MAX_QUESTIONS = 3


def color(text: str, code: str) -> str:
    codes = {"green": "32", "yellow": "33", "red": "31", "cyan": "36", "bold": "1", "dim": "2"}
    return f"\033[{codes.get(code, '0')}m{text}\033[0m"


def print_header():
    print(color("\n Rocky", "bold"))
    print(color(" ─────────────────────────────", "dim"))


def run_socratic_loop(topic: str, topic_info: dict, task: str,
                      known_topics: list[str], graph: KnowledgeGraph):
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
            graph.add_or_update(topic, 0.1, kind=topic_info.get("kind", "concept"),
                                description=topic_info["description"], context=task)
            return

        print(color("   Evaluating...", "dim"))
        result = teacher.evaluate_answer(topic, question, answer, topic_info["description"])
        score = result.get("score", 0.0)
        total_score += score
        feedback = result.get("feedback", "")

        print(f"\n   {color(feedback, 'cyan')}")

        if result.get("understood"):
            print(color("   Added to your PKG.", "green"))
            conf_delta = CONFIDENCE_INITIAL + (score * CONFIDENCE_CORRECT)
            graph.add_or_update(topic, conf_delta, kind=topic_info.get("kind", "concept"),
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
        graph.add_or_update(topic, CONFIDENCE_INITIAL * avg_score,
                            kind=topic_info.get("kind", "concept"),
                            description=topic_info["description"], context=task)
    else:
        print(color("\n   Topic saved — revisit this one before proceeding.", "red"))
        graph.add_or_update(topic, 0.1, kind=topic_info.get("kind", "concept"),
                            description=topic_info["description"], context=task)


def run_task(task: str, graph: KnowledgeGraph):
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
        n["topic"] for n in graph.all_topics()
        if graph.classify(n["topic"]) == "known"
    ]

    new_count = 0
    stale_count = 0

    for topic_info in topics:
        topic = topic_info["topic"]
        classification = graph.classify(topic)

        if classification == "known":
            print(color(f"  ✓ {topic}", "green") +
                  color(f" ({graph.effective_confidence(topic):.0%} confidence)", "dim"))
            graph.mark_encountered(topic)

        elif classification == "stale":
            stale_count += 1
            node = graph.get(topic)
            eff = graph.effective_confidence(topic)
            print(color(f"  ~ {topic}", "yellow") +
                  color(f" (confidence faded to {eff:.0%})", "dim"))
            print(color("  Refreshing...", "dim"))
            reminder = teacher.generate_reminder(topic, node, task)
            print(f"\n  {color('Rocky:', 'yellow')} {reminder}\n")
            graph.add_or_update(topic, 0.05, kind=topic_info.get("kind", node.get("kind", "concept")),
                                context=task)

        else:
            new_count += 1
            run_socratic_loop(topic, topic_info, task, known_topic_names, graph)

    print()
    if new_count == 0 and stale_count == 0:
        print(color("All topics are in your PKG. You're good to go.", "green"))
    else:
        stats = graph.summary()
        print(color(f"PKG: {stats['known']} known | "
                    f"{stats['stale']} fading | {stats['total']} total", "dim"))


def show_stats(graph: KnowledgeGraph):
    print_header()
    stats = graph.summary()
    print(f"\n  Total topics:  {stats['total']}")
    print(color(f"  Known:         {stats['known']}", "green"))
    print(color(f"  Fading:        {stats['stale']}", "yellow"))
    print(color(f"  Gaps/weak:     {stats['gaps']}", "red"))
    print()


def list_topics(graph: KnowledgeGraph):
    print_header()
    nodes = graph.all_topics()
    if not nodes:
        print("\n  PKG is empty. Run a task to populate it.")
        return

    print(f"\n  {'Topic':<35} {'Kind':<15} {'Confidence':<12} {'Last Reviewed'}")
    print(color("  " + "─" * 75, "dim"))

    for node in sorted(nodes, key=lambda n: graph.effective_confidence(n["topic"]), reverse=True):
        topic = node["topic"]
        kind = node.get("kind", "concept")
        eff = graph.effective_confidence(topic)
        classification = graph.classify(topic)
        color_code = "green" if classification == "known" else "yellow" if classification == "stale" else "red"
        bar = "█" * int(eff * 10) + "░" * (10 - int(eff * 10))
        print(f"  {color(topic[:34], color_code):<35} {kind:<15} {bar} {eff:.0%}  {node['last_reviewed']}")
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
    graph = KnowledgeGraph()

    if args.stats:
        show_stats(graph)
    elif args.list:
        list_topics(graph)
    elif args.task:
        run_task(args.task, graph)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
