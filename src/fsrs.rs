/// FSRS-inspired DSR model with a mastery overlay.
///
/// D (difficulty)     — how hard this topic has been for this user (0–1)
/// S (stability)      — days until retrievability drops to 90%
/// R (retrievability) — time-decay-since-last-review probability of recall
/// M (mastery)        — recent score history (did you actually know it, regardless of freshness)
/// recall_now = R × M — what we sort and classify by
///
/// Why both: R alone is "did you forget" (assumes you ever knew). After a wrong
/// review R bounces back to 1.0 (reviewed today) but mastery stays low — so the
/// topic is correctly surfaced as a gap, not hidden as "fresh."
use chrono::Local;

use crate::node::Kind;

/// Thresholds applied to recall_now (= retrievability × mastery), not raw R.
/// Tuned for multiplied-metric ranges: two halfway-decent factors product to
/// ~0.5, which we want to land in the middle "stale" bucket.
pub const KNOWN_RECALL: f64 = 0.60;
pub const STALE_RECALL: f64 = 0.30;

pub fn retrievability(stability: f64, last_reviewed: chrono::NaiveDate) -> f64 {
    let today = Local::now().date_naive();
    let elapsed = (today - last_reviewed).num_days();
    if elapsed <= 0 {
        return 1.0;
    }
    let r = (1.0 + elapsed as f64 / (9.0 * stability)).powi(-1);
    (r * 10000.0).round() / 10000.0
}

/// Mean of the last N review scores (most-recent-first weighting kept simple).
/// Returns 0.5 when there is no review history — neutral, mid-urgency.
pub fn mastery(recent_scores: &[f64]) -> f64 {
    if recent_scores.is_empty() {
        return 0.5;
    }
    let n = recent_scores.len().min(3);
    let sum: f64 = recent_scores.iter().rev().take(n).sum();
    let m = sum / n as f64;
    (m * 10000.0).round() / 10000.0
}

/// Effective recall combining time decay (retrievability) with score history (mastery).
/// Multiplicative — one wrong recent answer dominates a high freshness number.
pub fn recall_now(retrievability: f64, mastery: f64) -> f64 {
    let r = (retrievability * mastery * 10000.0).round() / 10000.0;
    r.clamp(0.0, 1.0)
}

pub fn classify(recall: f64) -> &'static str {
    if recall >= KNOWN_RECALL {
        "known"
    } else if recall >= STALE_RECALL {
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
        Kind::Domain => 999.0, // skeleton nodes never decay
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Duration};

    fn today() -> chrono::NaiveDate { Local::now().date_naive() }

    #[test]
    fn retrievability_is_one_when_reviewed_today() {
        assert_eq!(retrievability(4.0, today()), 1.0);
    }

    #[test]
    fn retrievability_decays_over_time() {
        let r_1d = retrievability(4.0, today() - Duration::days(1));
        let r_7d = retrievability(4.0, today() - Duration::days(7));
        assert!(r_1d < 1.0, "should decay after 1 day");
        assert!(r_7d < r_1d, "should decay faster at 7 days");
    }

    #[test]
    fn retrievability_is_one_for_future_date() {
        // elapsed <= 0 branch
        let r = retrievability(4.0, today() + Duration::days(1));
        assert_eq!(r, 1.0);
    }

    #[test]
    fn classify_thresholds() {
        // Now classifies recall_now (= R × M), not raw R.
        assert_eq!(classify(0.80), "known");   // 0.9 × 0.9
        assert_eq!(classify(0.60), "known");   // boundary
        assert_eq!(classify(0.50), "stale");   // ~0.7 × 0.7
        assert_eq!(classify(0.30), "stale");   // boundary
        assert_eq!(classify(0.20), "new");
        assert_eq!(classify(0.0),  "new");
    }

    #[test]
    fn mastery_default_when_empty() {
        assert_eq!(mastery(&[]), 0.5);
    }

    #[test]
    fn mastery_uses_last_three_scores() {
        // Old reviews shouldn't drag if there are >=3 newer ones.
        let scores = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        assert_eq!(mastery(&scores), 1.0);
    }

    #[test]
    fn recall_now_is_multiplicative() {
        // High freshness × low mastery = low recall (the bug fix).
        let r = recall_now(1.0, 0.0);
        assert_eq!(r, 0.0);
        // Both high → high.
        assert!((recall_now(0.9, 0.9) - 0.81).abs() < 1e-6);
    }

    #[test]
    fn initial_stability_differs_by_kind() {
        assert!(initial_stability(&Kind::Concept) > initial_stability(&Kind::Pattern));
        assert!(initial_stability(&Kind::Pattern) > initial_stability(&Kind::Implementation));
    }

    #[test]
    fn perfect_score_increases_stability() {
        let (new_s, new_d) = update_after_review(4.0, 0.3, 0.8, 1.0);
        assert!(new_s > 4.0, "stability should increase on perfect score");
        assert!(new_d < 0.3, "difficulty should decrease on perfect score");
    }

    #[test]
    fn failed_score_reduces_stability() {
        let (new_s, _) = update_after_review(4.0, 0.3, 0.8, 0.0);
        assert!(new_s < 4.0, "stability should decrease on failed score");
    }

    #[test]
    fn failed_score_never_drops_below_one() {
        let (new_s, _) = update_after_review(1.0, 0.5, 0.5, 0.0);
        assert!(new_s >= 1.0, "stability floor is 1.0");
    }

    #[test]
    fn difficulty_clamped_between_bounds() {
        // Max difficulty clamp — keep failing should not exceed 0.9
        let mut d = 0.85;
        for _ in 0..20 {
            let (_, new_d) = update_after_review(1.0, d, 0.5, 0.0);
            d = new_d;
        }
        assert!(d <= 0.9, "difficulty capped at 0.9, got {d}");

        // Min difficulty clamp — keep acing should not drop below 0.1
        let mut d = 0.15;
        for _ in 0..20 {
            let (_, new_d) = update_after_review(4.0, d, 0.5, 1.0);
            d = new_d;
        }
        assert!(d >= 0.1, "difficulty floored at 0.1, got {d}");
    }
}
