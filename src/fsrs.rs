/// FSRS-inspired DSR model.
///
/// D (difficulty)    — how hard this topic has been for this user (0–1)
/// S (stability)     — days until retrievability drops to 90%
/// R (retrievability) — current probability of recall
use chrono::Local;

use crate::node::Kind;

pub const KNOWN_R: f64 = 0.90;
pub const STALE_R: f64 = 0.70;

pub fn retrievability(stability: f64, last_reviewed: chrono::NaiveDate) -> f64 {
    let today = Local::now().date_naive();
    let elapsed = (today - last_reviewed).num_days();
    if elapsed <= 0 {
        return 1.0;
    }
    let r = (1.0 + elapsed as f64 / (9.0 * stability)).powi(-1);
    (r * 10000.0).round() / 10000.0
}

pub fn classify(r: f64) -> &'static str {
    if r >= KNOWN_R {
        "known"
    } else if r >= STALE_R {
        "stale"
    } else {
        "new"
    }
}

pub fn initial_stability(kind: &Kind) -> f64 {
    match kind {
        Kind::Concept => 4.0,
        Kind::Pattern => 2.5,
        Kind::Implementation => 1.5,
    }
}

/// Returns (new_stability, new_difficulty) after a review.
/// score: 0.0 (skipped/failed) → 1.0 (perfect)
pub fn update_after_review(stability: f64, difficulty: f64, r: f64, score: f64) -> (f64, f64) {
    if score >= 0.65 {
        let gain = 1.0 + (1.0 - r) * 2.0 * (1.0 - difficulty);
        let new_s = (stability * gain * 100.0).round() / 100.0;
        let new_d = ((difficulty - score * 0.05).max(0.1) * 1000.0).round() / 1000.0;
        (new_s, new_d)
    } else if score >= 0.4 {
        let gain = 1.0 + (1.0 - r) * 0.5 * (1.0 - difficulty);
        let new_s = (stability * gain * 100.0).round() / 100.0;
        let new_d = ((difficulty + (1.0 - score) * 0.05).min(0.9) * 1000.0).round() / 1000.0;
        (new_s, new_d)
    } else {
        let new_s = ((stability * 0.3).max(1.0) * 100.0).round() / 100.0;
        let new_d = ((difficulty + 0.1).min(0.9) * 1000.0).round() / 1000.0;
        (new_s, new_d)
    }
}
