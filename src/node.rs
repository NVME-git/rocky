use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Concept,
    Pattern,
    Implementation,
}

impl Kind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "pattern" => Self::Pattern,
            "implementation" => Self::Implementation,
            _ => Self::Concept,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Concept => "concept",
            Self::Pattern => "pattern",
            Self::Implementation => "implementation",
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
}
