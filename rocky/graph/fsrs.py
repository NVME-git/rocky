"""
FSRS-inspired DSR model for the PKG.

Each node has three values:
  D (difficulty)   — how hard this topic has been for this user (0–1)
  S (stability)    — days until retrievability drops to 90%
  R (retrievability) — current probability of recall, computed from S and elapsed time

Key insight from FSRS: reviewing right as something is fading (low R) gives the
biggest stability boost. Reviewing immediately after learning gives almost nothing.
This is why Rocky's work-context trigger outperforms scheduled review.
"""

from datetime import date

# Classification thresholds
KNOWN_R = 0.90    # R ≥ 0.90 → silent pass
STALE_R = 0.70    # 0.70 ≤ R < 0.90 → reminder
# R < 0.70 → Socratic Q&A

# Initial stability per kind (days to 90% retrievability on first successful review)
INITIAL_STABILITY: dict[str, float] = {
    "concept": 4.0,         # abstract ideas decay slowly
    "pattern": 2.5,         # patterns need moderate reinforcement
    "implementation": 1.5,  # implementation details fade fastest
}


def retrievability(stability: float, last_reviewed: date) -> float:
    """
    Probability of recall given stability (S) and days elapsed since last review.
    Formula: R = (1 + t / (9 * S)) ^ -1
    """
    elapsed = (date.today() - last_reviewed).days
    if elapsed <= 0:
        return 1.0
    return round((1 + elapsed / (9 * stability)) ** -1, 4)


def classify(r: float) -> str:
    """Classify a topic based on its current retrievability."""
    if r >= KNOWN_R:
        return "known"
    if r >= STALE_R:
        return "stale"
    return "new"


def initial_stability(kind: str) -> float:
    return INITIAL_STABILITY.get(kind, 2.0)


def update_after_review(
    stability: float,
    difficulty: float,
    r: float,
    score: float,
) -> tuple[float, float]:
    """
    Compute new (stability, difficulty) after a review.

    score: 0.0 (skipped/failed) → 1.0 (perfect understanding)
    r:     retrievability at the moment of review

    Stability gains are largest when:
      - R was low (reviewing at the edge of forgetting)
      - difficulty is low (easier topics reinforce faster)

    This mirrors the FSRS stability increase formula while staying simple.
    """
    if score >= 0.65:  # understood
        # Bigger gain the closer to forgetting the review was
        gain = 1.0 + (1.0 - r) * 2.0 * (1.0 - difficulty)
        new_stability = round(stability * gain, 2)
        new_difficulty = round(max(0.1, difficulty - score * 0.05), 3)

    elif score >= 0.4:  # partial understanding
        gain = 1.0 + (1.0 - r) * 0.5 * (1.0 - difficulty)
        new_stability = round(stability * gain, 2)
        new_difficulty = round(min(0.9, difficulty + (1.0 - score) * 0.05), 3)

    else:  # skipped or failed
        # Stability resets toward the initial value — not a full reset, just pulled down
        new_stability = round(max(1.0, stability * 0.3), 2)
        new_difficulty = round(min(0.9, difficulty + 0.1), 3)

    return new_stability, new_difficulty
