"""
Session tracking for Rocky.

Enforces two smart silence rules:
  - Daily quiz budget: max 3 Socratic loops per day (configurable)
  - Cool-down window: no quiz within 2 hours of the last one (configurable)

Also logs every task description for future prompt quality tracking (Phase 5).
State is stored in the main PKG database so nothing extra to manage.
"""

import sqlite3
from datetime import date, datetime, timedelta
from pathlib import Path

from rocky.graph.store import DB_FILE

_DEFAULT_DAILY_BUDGET = 3
_DEFAULT_MIN_GAP_MINUTES = 120


class Session:
    def __init__(self, db_path: Path = DB_FILE,
                 daily_budget: int = _DEFAULT_DAILY_BUDGET,
                 min_gap_minutes: int = _DEFAULT_MIN_GAP_MINUTES):
        self.daily_budget = daily_budget
        self.min_gap_minutes = min_gap_minutes
        self._db = db_path

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self._db)
        conn.row_factory = sqlite3.Row
        return conn

    def _get(self, key: str) -> str | None:
        with self._connect() as conn:
            row = conn.execute(
                "SELECT value FROM session_state WHERE key = ?", (key,)
            ).fetchone()
            return row["value"] if row else None

    def _set(self, key: str, value: str):
        now = datetime.now().isoformat()
        with self._connect() as conn:
            conn.execute("""
                INSERT INTO session_state (key, value, updated_at)
                VALUES (?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            """, (key, value, now))

    # ── budget ────────────────────────────────────────────────────────────────

    def budget_remaining(self) -> int:
        """Quizzes remaining today. Resets automatically at midnight."""
        last_reset = self._get("budget_reset_date")
        today = date.today().isoformat()

        if last_reset != today:
            self._set("budget_reset_date", today)
            self._set("budget_used", "0")
            return DAILY_BUDGET

        used = int(self._get("budget_used") or "0")
        return max(0, self.daily_budget - used)

    def record_quiz(self):
        """Call once per completed Socratic loop to decrement the budget."""
        used = int(self._get("budget_used") or "0")
        self._set("budget_used", str(used + 1))
        self._set("last_quiz_at", datetime.now().isoformat())

    # ── cool-down ─────────────────────────────────────────────────────────────

    def cool_down_active(self) -> bool:
        """True if a quiz happened within the last MIN_GAP_MINUTES."""
        last = self._get("last_quiz_at")
        if not last:
            return False
        elapsed = datetime.now() - datetime.fromisoformat(last)
        return elapsed < timedelta(minutes=self.min_gap_minutes)

    def minutes_until_ready(self) -> int:
        last = self._get("last_quiz_at")
        if not last:
            return 0
        elapsed = datetime.now() - datetime.fromisoformat(last)
        remaining = timedelta(minutes=self.min_gap_minutes) - elapsed
        return max(0, int(remaining.total_seconds() / 60))

    # ── combined gate ─────────────────────────────────────────────────────────

    def can_quiz(self) -> tuple[bool, str]:
        """
        Returns (allowed, reason_if_blocked).
        Callers use this to decide whether to run the Socratic loop.
        """
        if self.cool_down_active():
            mins = self.minutes_until_ready()
            return False, f"cool-down active — Rocky ready again in ~{mins}m"
        if self.budget_remaining() == 0:
            return False, "daily quiz budget reached (3/day) — resets tomorrow"
        return True, ""

    # ── task logging ──────────────────────────────────────────────────────────

    def log_task(self, task: str, mode: str = "manual"):
        """Log a task description. Used by Phase 5 prompt quality tracking."""
        now = datetime.now().isoformat()
        with self._connect() as conn:
            conn.execute(
                "INSERT INTO task_log (task, mode, logged_at) VALUES (?, ?, ?)",
                (task, mode, now),
            )
