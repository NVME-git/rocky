use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A single pre-generated question with its ideal answer and clue.
/// Stored in `question_bank` JSON on each node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionBankItem {
    pub question: String,
    pub answer: String,
    pub clue: String,
    /// Number of times this specific question has been asked (used for rotation)
    #[serde(default)]
    pub asked_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Concept,
    Pattern,
    Implementation,
    /// Taxonomy skeleton node — never quizzed, excluded from counts and ls.
    Domain,
}

impl Kind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "pattern" => Self::Pattern,
            "implementation" => Self::Implementation,
            "domain" => Self::Domain,
            _ => Self::Concept,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Concept => "concept",
            Self::Pattern => "pattern",
            Self::Implementation => "implementation",
            Self::Domain => "domain",
        }
    }

    pub fn is_domain(&self) -> bool {
        matches!(self, Self::Domain)
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_round_trips() {
        for s in ["concept", "pattern", "implementation"] {
            assert_eq!(Kind::from_str(s).as_str(), s);
        }
    }

    #[test]
    fn kind_from_str_defaults_to_concept() {
        assert_eq!(Kind::from_str("unknown"), Kind::Concept);
        assert_eq!(Kind::from_str(""),        Kind::Concept);
    }

    #[test]
    fn kind_display_matches_as_str() {
        for kind in [Kind::Concept, Kind::Pattern, Kind::Implementation] {
            assert_eq!(format!("{kind}"), kind.as_str());
        }
    }
}

/// The 13 fixed taxonomy domains Rocky uses to group topics.
#[allow(dead_code)]
pub const DOMAINS: &[&str] = &[
    "Language", "Database", "Auth", "API", "Frontend",
    "DevOps", "Architecture", "Performance", "Security",
    "Testing", "Tooling", "Data", "Other",
];

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Node {
    pub id: String,
    pub topic: String,
    pub kind: Kind,
    pub domain: String,
    pub description: String,
    pub difficulty: f64,
    pub stability: f64,
    pub last_reviewed: NaiveDate,
    pub last_encountered: NaiveDate,
    pub review_count: i64,
    pub contexts: Vec<String>,
    pub created_at: NaiveDate,
    /// Pre-generated question from diff context (empty if not yet generated)
    pub canonical_question: String,
    /// Pre-generated ideal answer (empty if not yet generated)
    pub canonical_answer: String,
    /// Short clue generated alongside canonical Q&A (empty if not yet generated)
    pub canonical_clue: String,
    /// Git repo/project name this node originated from
    pub repo: String,
    /// How many times this topic has been encountered across commits/sessions.
    /// Different from review_count (which counts quiz reviews).
    pub encounter_count: i64,
    /// Commit SHAs where this topic appeared. Audit trail for dedup decisions.
    pub source_commits: Vec<String>,
    /// Pre-generated bank of 3-5 questions (rotated through during reviews).
    pub question_bank: Vec<QuestionBankItem>,
    /// Distinct repo names this topic has been encountered in. Cross-project
    /// audit trail — `repo` is where it was first seen, `repos` accumulates.
    pub repos: Vec<String>,
    /// Soft-delete timestamp. None = active; Some(ts) = sitting in the
    /// Recycle Bin awaiting restore or permanent delete.
    pub discarded_at: Option<String>,
}
