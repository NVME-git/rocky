/// Rocky's personality layer.
///
/// All user-facing flavour text lives here. Set `personality = false` in
/// [ui] config to suppress everything and get plain output.
use colored::Colorize;

// ── ASCII Rocky ───────────────────────────────────────────────────────────────

const ROCKY_CALM: &str = "
   __|__
  /◉   ◉\\
  \\ ─── /
   \\_↑_/
  /|||||\\
";

const ROCKY_HAPPY: &str = "
   __|__
  /^   ^\\
  \\ ─── /
   \\_↑_/
  /|||||\\
";

const ROCKY_EXCITE: &str = "
  \\(◉ · ◉)/
    \\─────/
     |||||
";

const ROCKY_BANNER: &str = "
     __|__
    /◉   ◉\\         R  O  C  K  Y
    \\ ─── /         Personal Knowledge Graph
     \\_↑_/
    /|||||\\          You observe. Question?
";

const ASCII_VARIANTS: &[&str] = &[ROCKY_CALM, ROCKY_HAPPY, ROCKY_EXCITE];

// ── Phrase banks ──────────────────────────────────────────────────────────────

const CORRECT: &[&str] = &[
    "Fist my bump, friend! Is correct!",
    "Excite excite excite! Friend get it!",
    "Yes! Friend brain work excellent!",
    "Is good! Rocky is very happy!",
    "You are genius human!",
    "Amazing! Very good science!",
    "Rocky knew friend could do!",
    "Friend learn fast. Rocky is proud.",
    "Fist my bump! Is exactly right!",
    "Excite! Friend understand consequence!",
];

const PARTIAL: &[&str] = &[
    "Is close! Brain almost have it!",
    "Good think! More practice, more know!",
    "Is okay. Science is hard. We figure out.",
    "Friend try hard. Rocky respect.",
    "Almost! Keep going, friend.",
    "Right direction! Just need more depth.",
];

const FAILED: &[&str] = &[
    "Is okay! Rocky also not know at first!",
    "Now friend know. Is good gift.",
    "Hard topic. But friend will remember.",
    "We are crew. We solve together.",
    "Brain need time. Is normal.",
    "Rocky fail many thing before succeed. Is process.",
];

const SKIPPED: &[&str] = &[
    "Is okay! Rocky skip hard question sometimes too.",
    "Next time maybe. No problem.",
    "Rocky save question for later. Is wise.",
];

const SESSION_DONE_FEW: &[&str] = &[
    "Good session! Friend brain stronger now.",
    "Rocky is pleased. Science happened today.",
    "We are good crew, friend.",
];

const SESSION_DONE_MANY: &[&str] = &[
    "Many topic! Rocky is very impress!",
    "Excite excite excite! Much learning today!",
    "All done! Friend brain is strong today!",
    "Science win today. Is good.",
];

// ── Streak phrases ────────────────────────────────────────────────────────────

fn streak_phrase(days: u32) -> String {
    match days {
        1 => "First day! Good start, friend.".into(),
        2 => "Two day in row! Consistency is best science.".into(),
        3..=6 => format!("{days} day streak! Friend very dedicated!"),
        7..=13 => format!("{days} day streak! Excite excite! Rocky tell other Eridian!"),
        14..=29 => format!("{days} day streak!! Friend brain grow very large!"),
        _ => format!("{days} day streak!!! Rocky is maximum excite! Fist my bump!"),
    }
}

// ── Milestone phrases ─────────────────────────────────────────────────────────

pub fn milestone_phrase(known: usize) -> Option<&'static str> {
    match known {
        1 => Some("First topic known! Rocky mark beginning of great friendship!"),
        5 => Some("Five topic! Friend brain filling up! Excite!"),
        10 => Some("Ten topic known! Excite excite excite!"),
        25 => Some("Twenty-five! Friend brain grow very large!"),
        50 => Some("Fifty topic! Rocky tell other Eridian about friend!"),
        100 => Some("One hundred topic!! Friend is now expert human! Rocky maximum proud!"),
        _ => None,
    }
}

// ── PKG mood (for stats) ──────────────────────────────────────────────────────

pub fn pkg_mood(known: usize, total: usize) -> &'static str {
    if total == 0 {
        return "PKG is empty. Let us begin science, question?";
    }
    let pct = known * 100 / total;
    match pct {
        80..=100 => "PKG look strong. Rocky is very please.",
        50..=79  => "Good progress, friend. Keep science going.",
        _        => "Many gap. But is okay. We fix together.",
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Picks a phrase from a slice using subsecond time as a cheap random source.
fn pick(items: &[&str]) -> String {
    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize
        % items.len();
    items[idx].to_string()
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct Personality {
    pub enabled: bool,
}

impl Personality {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Show the full welcome banner (used on install).
    pub fn banner(&self) {
        if !self.enabled { return; }
        println!("{}", ROCKY_BANNER.truecolor(245, 158, 11));
    }

    /// Show ASCII Rocky. Pass `excite=true` after big milestones.
    pub fn print_rocky(&self, excite: bool) {
        if !self.enabled { return; }
        let art = if excite {
            ROCKY_EXCITE
        } else {
            let idx = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as usize
                % ASCII_VARIANTS.len();
            ASCII_VARIANTS[idx]
        };
        println!("{}", art.truecolor(245, 158, 11));
    }

    pub fn correct(&self) -> Option<String> {
        if !self.enabled { return None; }
        Some(pick(CORRECT).truecolor(29, 158, 117).to_string())
    }

    pub fn partial(&self) -> Option<String> {
        if !self.enabled { return None; }
        Some(pick(PARTIAL).truecolor(239, 159, 39).to_string())
    }

    pub fn failed(&self) -> Option<String> {
        if !self.enabled { return None; }
        Some(pick(FAILED).dimmed().to_string())
    }

    pub fn skipped(&self) -> Option<String> {
        if !self.enabled { return None; }
        Some(pick(SKIPPED).dimmed().to_string())
    }

    pub fn session_done(&self, topics_completed: u32) -> Option<String> {
        if !self.enabled { return None; }
        let phrase = if topics_completed >= 3 {
            pick(SESSION_DONE_MANY)
        } else {
            pick(SESSION_DONE_FEW)
        };
        Some(phrase.truecolor(29, 158, 117).to_string())
    }

    pub fn streak(&self, days: u32) -> Option<String> {
        if !self.enabled || days < 2 { return None; }
        Some(streak_phrase(days).truecolor(6, 182, 212).to_string())
    }

    pub fn milestone(&self, known: usize) -> Option<String> {
        if !self.enabled { return None; }
        milestone_phrase(known).map(|s| s.truecolor(6, 182, 212).bold().to_string())
    }

    pub fn pkg_mood(&self, known: usize, total: usize) -> Option<String> {
        if !self.enabled { return None; }
        Some(pkg_mood(known, total).dimmed().to_string())
    }

    /// Rewrites an LLM-generated question in Rocky's voice.
    /// Strips trailing "?" and appends ", question?"
    pub fn format_question(&self, q: &str) -> String {
        if !self.enabled { return q.to_string(); }
        let trimmed = q.trim().trim_end_matches('?').trim_end_matches('.');
        format!("{trimmed}, question?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_question_appends_rocky_suffix() {
        let p = Personality::new(true);
        let q = p.format_question("What is a database index?");
        assert!(q.ends_with(", question?"), "got: {q}");
        assert!(!q.contains("??"), "should not double punctuate: {q}");
    }

    #[test]
    fn format_question_passthrough_when_disabled() {
        let p = Personality::new(false);
        let original = "What is a database index?";
        assert_eq!(p.format_question(original), original);
    }

    #[test]
    fn pkg_mood_empty_pkg() {
        assert!(pkg_mood(0, 0).contains("empty"));
    }

    #[test]
    fn pkg_mood_strong_pkg() {
        assert!(pkg_mood(9, 10).contains("strong") || pkg_mood(9, 10).contains("please"));
    }

    #[test]
    fn milestone_phrase_known_values() {
        assert!(milestone_phrase(1).is_some());
        assert!(milestone_phrase(10).is_some());
        assert!(milestone_phrase(100).is_some());
        assert!(milestone_phrase(7).is_none());
    }
}
