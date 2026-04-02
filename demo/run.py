#!/usr/bin/env python3
"""
PKG Demo — watch the knowledge graph evolve through a realistic dev scenario.

Setup:
    python demo/seed.py       # populate the demo graph with a starting state

Run:
    python demo/run.py              # run all tasks interactively
    python demo/run.py --task 2     # run a single task (1-indexed)
    python demo/run.py --reset      # reset graph back to seeded state
    python demo/run.py --list       # show available tasks

The demo uses demo/demo_graph.json — your real knowledge_graph.json is untouched.
"""

import argparse
import subprocess
import sys
from pathlib import Path

from dotenv import load_dotenv

# Load env from project root (.env lives there)
load_dotenv(Path(__file__).parent.parent / ".env")

# Make parent package importable
sys.path.insert(0, str(Path(__file__).parent.parent))

from graph import KnowledgeGraph
from pkg import color, run_task
from demo.tasks import TASKS

DEMO_GRAPH = Path(__file__).parent / "demo_graph.json"


def print_divider(label: str = ""):
    width = 62
    if label:
        pad = (width - len(label) - 2) // 2
        print(f"\n{color('─' * pad + ' ' + label + ' ' + '─' * pad, 'dim')}\n")
    else:
        print(f"\n{color('─' * width, 'dim')}\n")


def print_graph_state(kg: KnowledgeGraph):
    nodes = kg.all_topics()
    if not nodes:
        print("  (graph is empty — run seed.py first)")
        return

    print(f"  {'Topic':<36} {'Confidence':<14} Status")
    print(color("  " + "─" * 62, "dim"))

    for n in sorted(nodes, key=lambda x: kg.effective_confidence(x["topic"]), reverse=True):
        topic = n["topic"]
        c = kg.classify(topic)
        eff = kg.effective_confidence(topic)
        bar = "█" * int(eff * 10) + "░" * (10 - int(eff * 10))
        label = color("known", "green") if c == "known" else color("stale", "yellow") if c == "stale" else color("new/weak", "red")
        symbol = color("✓", "green") if c == "known" else color("~", "yellow") if c == "stale" else color("?", "red")
        print(f"  {symbol} {topic[:35]:<35} {bar} {eff:.0%}  {label}")
    print()


def run_demo(indices: list[int]):
    if not DEMO_GRAPH.exists():
        print(color("Demo graph not found. Run: python demo/seed.py", "red"))
        sys.exit(1)

    kg = KnowledgeGraph(path=DEMO_GRAPH)

    print_divider("STARTING STATE")
    print_graph_state(kg)

    for i in indices:
        task, note = TASKS[i]
        print_divider(f"TASK {i + 1} of {len(TASKS)}")
        print(color(f"  Note: {note}", "dim"))
        print()

        run_task(task, kg)

        # Reload from disk to reflect any saves made inside run_task
        kg = KnowledgeGraph(path=DEMO_GRAPH)

        print_divider(f"GRAPH AFTER TASK {i + 1}")
        print_graph_state(kg)

        if i < indices[-1]:
            try:
                input(color("  Press Enter for next task, Ctrl-C to stop...", "dim"))
            except (EOFError, KeyboardInterrupt):
                print()
                break


def main():
    parser = argparse.ArgumentParser(
        prog="demo/run.py",
        description="PKG Demo — watch the knowledge graph evolve",
    )
    parser.add_argument("--task", type=int, metavar="N", help="Run only task N (1-indexed)")
    parser.add_argument("--reset", action="store_true", help="Reset graph to seeded state")
    parser.add_argument("--list", action="store_true", help="List demo tasks")
    args = parser.parse_args()

    if args.reset:
        seed_script = Path(__file__).parent / "seed.py"
        subprocess.run([sys.executable, str(seed_script)], check=True)
        return

    if args.list:
        print("\nDemo tasks:\n")
        for i, (task, note) in enumerate(TASKS, 1):
            print(f"  {i}. {task}")
            print(color(f"     {note}", "dim"))
            print()
        return

    if args.task is not None:
        if args.task < 1 or args.task > len(TASKS):
            print(f"--task must be between 1 and {len(TASKS)}")
            sys.exit(1)
        run_demo([args.task - 1])
    else:
        run_demo(list(range(len(TASKS))))


if __name__ == "__main__":
    main()
