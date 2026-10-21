use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Languages supported by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Fr,
    En,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::Fr, Language::En];

    pub fn code(self) -> &'static str {
        match self {
            Language::Fr => "fr",
            Language::En => "en",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Language::Fr => "Français",
            Language::En => "English",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.trim().to_ascii_lowercase();
        let primary = lower.split(['-', '_']).next().unwrap_or_default();
        match primary {
            "fr" | "french" | "français" | "francais" => Ok(Language::Fr),
            "en" | "english" | "anglais" => Ok(Language::En),
            _ => Err(format!("unsupported language: {s}")),
        }
    }
}

/// Broad family a rule belongs to. Used for filtering and colouring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Spelling,
    Grammar,
    Conjugation,
    Typography,
    Punctuation,
    Style,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Spelling,
        Category::Grammar,
        Category::Conjugation,
        Category::Typography,
        Category::Punctuation,
        Category::Style,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Category::Spelling => "spelling",
            Category::Grammar => "grammar",
            Category::Conjugation => "conjugation",
            Category::Typography => "typography",
            Category::Punctuation => "punctuation",
            Category::Style => "style",
        }
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Category::ALL
            .into_iter()
            .find(|c| c.code().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| format!("unknown category: {s}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Almost certainly wrong.
    Error,
    /// Probably wrong, depends on context.
    Warning,
    /// Suggestion to improve the text.
    Hint,
}

/// A problem found by a rule, expressed with byte offsets into the checked text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub rule_id: &'static str,
    pub category: Category,
    pub severity: Severity,
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub replacements: Vec<String>,
}

/// Public description of a rule.
#[derive(Debug, Clone, Serialize)]
pub struct RuleInfo {
    pub id: &'static str,
    pub language: Option<Language>,
    pub category: Category,
    pub description: &'static str,
    pub enabled_by_default: bool,
}
