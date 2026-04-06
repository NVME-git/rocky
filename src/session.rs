/// Session tracking — daily quiz budget and cool-down window.
use anyhow::Result;
use chrono::{Duration, Local};

use crate::db::Db;

pub struct Session {
    db: Db,
    pub daily_budget: u32,
    pub min_gap_minutes: u32,
}

impl Session {
    pub fn new(db: Db, daily_budget: u32, min_gap_minutes: u32) -> Self {
        Self { db, daily_budget, min_gap_minutes }
    }

    pub fn budget_remaining(&self) -> Result<u32> {
        let today = Local::now().date_naive().to_string();
        let last_reset = self.db.session_get("budget_reset_date")?;
        if last_reset.as_deref() != Some(&today) {
            self.db.session_set("budget_reset_date", &today)?;
            self.db.session_set("budget_used", "0")?;
            return Ok(self.daily_budget);
        }
        let used: u32 = self.db.session_get("budget_used")?
            .unwrap_or_default()
            .parse()
            .unwrap_or(0);
        Ok(self.daily_budget.saturating_sub(used))
    }

    pub fn record_quiz(&self) -> Result<()> {
        let used: u32 = self.db.session_get("budget_used")?
            .unwrap_or_default()
            .parse()
            .unwrap_or(0);
        self.db.session_set("budget_used", &(used + 1).to_string())?;
        self.db.session_set("last_quiz_at", &Local::now().naive_local().to_string())?;
        self.db.increment_total_quizzes()?;
        Ok(())
    }

    pub fn cool_down_active(&self) -> Result<bool> {
        let last = self.db.session_get("last_quiz_at")?;
        if let Some(last) = last {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&last, "%Y-%m-%d %H:%M:%S%.f") {
                let elapsed = Local::now().naive_local() - dt;
                return Ok(elapsed < Duration::minutes(self.min_gap_minutes as i64));
            }
        }
        Ok(false)
    }

    pub fn minutes_until_ready(&self) -> Result<i64> {
        let last = self.db.session_get("last_quiz_at")?;
        if let Some(last) = last {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&last, "%Y-%m-%d %H:%M:%S%.f") {
                let elapsed = Local::now().naive_local() - dt;
                let remaining = Duration::minutes(self.min_gap_minutes as i64) - elapsed;
                return Ok(remaining.num_minutes().max(0));
            }
        }
        Ok(0)
    }

    /// Returns (allowed, reason_if_blocked). Enforces both cooldown and daily budget.
    /// Use for automatic triggers (git hooks, Claude Code hook).
    pub fn can_quiz(&self) -> Result<(bool, String)> {
        if self.cool_down_active()? {
            let mins = self.minutes_until_ready()?;
            return Ok((false, format!("cool-down active — Rocky ready again in ~{mins}m")));
        }
        if self.budget_remaining()? == 0 {
            return Ok((false, format!("daily quiz budget reached ({}/day) — resets tomorrow", self.daily_budget)));
        }
        Ok((true, String::new()))
    }

    /// Always allows quizzing — no cooldown, no budget cap.
    /// Use when the user explicitly invokes Rocky, since they're choosing to learn.
    pub fn can_quiz_manual(&self) -> Result<(bool, String)> {
        Ok((true, String::new()))
    }

    /// Call once at the start of a quiz session to update the streak counter.
    /// Returns the new streak value.
    pub fn update_streak(&self) -> Result<u32> {
        let today = Local::now().date_naive().to_string();
        let yesterday = (Local::now().date_naive() - Duration::days(1)).to_string();
        let last_date = self.db.session_get("last_quiz_date")?;
        let current: u32 = self.db.session_get("quiz_streak")?
            .unwrap_or_default().parse().unwrap_or(0);
        let new_streak = match last_date.as_deref() {
            Some(d) if d == today     => current,       // already counted today
            Some(d) if d == yesterday => current + 1,   // consecutive day
            _                         => 1,             // first time or broken
        };
        self.db.session_set("last_quiz_date", &today)?;
        self.db.session_set("quiz_streak", &new_streak.to_string())?;
        Ok(new_streak)
    }

}
