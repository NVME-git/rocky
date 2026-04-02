"""
Rocky report — prompt quality tracking and PKG health summary.

Shows:
  1. PKG health — topic counts, avg retrievability, topics at risk
  2. Task activity — weekly breakdown of tasks logged
  3. Prompt complexity — avg word count over time as a specificity proxy
  4. Aspirational roadmap — LLM-suggested next topics based on recent work + PKG gaps
"""

import sqlite3
from datetime import date, datetime, timedelta
from pathlib import Path

from rocky.graph.store import PKG, DB_FILE
from rocky.graph import fsrs
from rocky.graph.node import Node


# ── helpers ───────────────────────────────────────────────────────────────────

def _bar(value: float, width: int = 10) -> str:
    filled = round(value * width)
    return "█" * filled + "░" * (width - filled)


def _connect(db_path: Path = DB_FILE) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn


def _week_label(dt: datetime) -> str:
    """Return ISO week string like '2026-W12'."""
    return dt.strftime("%G-W%V")


# ── data queries ──────────────────────────────────────────────────────────────

def _task_log(db_path: Path, weeks: int = 8) -> list[dict]:
    cutoff = (datetime.now() - timedelta(weeks=weeks)).isoformat()
    with _connect(db_path) as conn:
        rows = conn.execute(
            "SELECT task, logged_at FROM task_log WHERE logged_at >= ? ORDER BY logged_at",
            (cutoff,),
        ).fetchall()
    return [{"task": r["task"], "logged_at": datetime.fromisoformat(r["logged_at"])} for r in rows]


def _weekly_activity(tasks: list[dict]) -> list[tuple[str, int, float]]:
    """Return list of (week_label, task_count, avg_word_count)."""
    from collections import defaultdict
    counts: dict[str, list[int]] = defaultdict(list)
    for t in tasks:
        wk = _week_label(t["logged_at"])
        counts[wk].append(len(t["task"].split()))

    return [
        (wk, len(words), sum(words) / len(words))
        for wk, words in sorted(counts.items())
    ]


def _at_risk(pkg: PKG, top_n: int = 5) -> list[Node]:
    nodes = pkg.all_topics()
    fading = [
        n for n in nodes
        if fsrs.classify(fsrs.retrievability(n.stability, n.last_reviewed)) == "stale"
    ]
    return sorted(fading, key=lambda n: fsrs.retrievability(n.stability, n.last_reviewed))[:top_n]


def _roadmap_prompt(pkg: PKG, recent_tasks: list[dict]) -> str:
    nodes = pkg.all_topics()
    known = [n.topic for n in nodes if fsrs.classify(fsrs.retrievability(n.stability, n.last_reviewed)) == "known"]
    gaps = [n.topic for n in nodes if fsrs.classify(fsrs.retrievability(n.stability, n.last_reviewed)) != "known"]
    recent = [t["task"] for t in recent_tasks[-10:]]

    system = """You are a technical learning advisor. Based on a developer's current knowledge
graph and recent work, suggest 3-5 specific topics they should actively learn next.

Rules:
- Suggest topics that are adjacent to what they already know (not random)
- Prioritise topics that appear implicitly in their recent work but aren't in their graph yet
- Frame each suggestion as a one-sentence explanation of WHY it matters given their context
- Return ONLY a numbered list, no preamble"""

    user = f"""Known topics: {', '.join(known) if known else 'none yet'}
Knowledge gaps (in PKG but fading): {', '.join(gaps) if gaps else 'none'}
Recent tasks delegated to AI agents:
{chr(10).join(f'- {t}' for t in recent)}"""

    return system, user


# ── formatters ────────────────────────────────────────────────────────────────

def _color(text: str, code: str) -> str:
    codes = {"green": "32", "yellow": "33", "red": "31", "cyan": "36", "bold": "1", "dim": "2"}
    return f"\033[{codes.get(code, '0')}m{text}\033[0m"


def _section(title: str):
    print(f"\n{_color(title, 'bold')}")
    print(_color(" " + "─" * (len(title) - 1), "dim"))


def print_report(pkg: PKG, db_path: Path = DB_FILE):
    import rocky.teacher as teacher

    print(_color("\n Rocky Report", "bold"))
    print(_color(" ─────────────────────────────", "dim"))

    # ── PKG health ────────────────────────────────────────────────────────────
    _section(" PKG Health")
    nodes = pkg.all_topics()
    total = len(nodes)

    if total == 0:
        print(_color("\n  PKG is empty — run a few tasks to populate it.", "dim"))
    else:
        stats = pkg.summary()
        retrievabilities = [fsrs.retrievability(n.stability, n.last_reviewed) for n in nodes]
        avg_r = sum(retrievabilities) / len(retrievabilities)

        print(f"\n  Total topics   {total}")
        print(_color(f"  Known          {stats['known']:<4} {_bar(stats['known']/total)} {stats['known']/total:.0%}", "green"))
        print(_color(f"  Fading         {stats['stale']:<4} {_bar(stats['stale']/total)} {stats['stale']/total:.0%}", "yellow"))
        print(_color(f"  Gaps           {stats['gaps']:<4} {_bar(stats['gaps']/total)} {stats['gaps']/total:.0%}", "red"))
        print(f"  Avg recall     {_bar(avg_r)} {avg_r:.0%}")

    # ── task activity ─────────────────────────────────────────────────────────
    _section(" Task Activity  (last 8 weeks)")
    tasks = _task_log(db_path, weeks=8)

    if not tasks:
        print(_color("\n  No tasks logged yet — run `rocky \"your task\"` to start.", "dim"))
    else:
        weekly = _weekly_activity(tasks)
        max_count = max(c for _, c, _ in weekly) if weekly else 1

        for wk, count, avg_words in weekly:
            bar = "█" * round(count / max_count * 12)
            print(f"  {wk}  {_color(bar.ljust(12), 'cyan')}  {count} task{'s' if count != 1 else ''}"
                  f"  {_color(f'~{avg_words:.0f} words/task', 'dim')}")

        # Prompt complexity trend
        if len(weekly) >= 2:
            first_avg = weekly[0][2]
            last_avg = weekly[-1][2]
            delta = last_avg - first_avg
            if abs(delta) >= 2:
                direction = "more specific" if delta > 0 else "shorter"
                arrow = "↑" if delta > 0 else "↓"
                code = "green" if delta > 0 else "yellow"
                print(_color(
                    f"\n  Prompt specificity {arrow}  "
                    f"{first_avg:.0f} → {last_avg:.0f} avg words/task  ({direction})",
                    code,
                ))
            else:
                print(_color("\n  Prompt length stable — check back as your PKG grows.", "dim"))

    # ── topics at risk ────────────────────────────────────────────────────────
    at_risk = _at_risk(pkg)
    if at_risk:
        _section(" Topics at Risk  (review soon)")
        print()
        for n in at_risk:
            r = fsrs.retrievability(n.stability, n.last_reviewed)
            days = (date.today() - n.last_reviewed).days
            print(f"  {_color('~', 'yellow')} {n.topic:<35} "
                  f"{_color(f'R={r:.0%}', 'yellow')}  last reviewed {days}d ago")

    # ── aspirational roadmap ──────────────────────────────────────────────────
    _section(" What to Learn Next")
    recent_tasks = _task_log(db_path, weeks=4)

    if not recent_tasks and total == 0:
        print(_color("\n  Run some tasks first — Rocky will suggest next topics once your PKG has data.", "dim"))
    else:
        print(_color("\n  Rocky is thinking...", "dim"), end="\r")
        try:
            system, user = _roadmap_prompt(pkg, recent_tasks)
            suggestions = teacher._ask(system, user)
            print(" " * 30, end="\r")  # clear the "thinking" line
            print()
            for line in suggestions.strip().splitlines():
                print(f"  {line}")
        except Exception as e:
            print(_color(f"\n  Could not generate suggestions: {e}", "dim"))

    print()
