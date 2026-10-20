//! Public entry point: checking, correcting and conjugating.

use crate::conjugation::{self, Conjugation, ConjugationError};
use crate::rules::{self, Ctx};
use crate::text::Utf16Index;
use crate::types::{Category, Issue, Language, RuleInfo, Severity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckOptions {
    /// Language of the text; detected automatically when absent.
    pub language: Option<Language>,
    /// Rules to turn off.
    pub disabled_rules: HashSet<String>,
    /// Rules that are off by default and should run.
    pub enabled_rules: HashSet<String>,
    /// Whole categories to skip.
    pub disabled_categories: HashSet<Category>,
    /// Words accepted by the spell checker.
    pub user_words: HashSet<String>,
    /// Sentences longer than this many words are reported (0 disables).
    pub max_sentence_words: Option<usize>,
}

pub const DEFAULT_MAX_SENTENCE_WORDS: usize = 40;

/// An issue as returned to clients. Offsets are UTF-16 code units, like JavaScript strings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Match {
    pub rule_id: String,
    pub category: Category,
    pub severity: Severity,
    pub message: String,
    pub offset: usize,
    pub length: usize,
    /// The flagged text.
    pub text: String,
    pub replacements: Vec<String>,
    /// The surrounding sentence, to help readers without the full text.
    pub sentence: String,
    #[serde(skip)]
    pub byte_start: usize,
    #[serde(skip)]
    pub byte_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    pub characters: usize,
    pub words: usize,
    pub sentences: usize,
    /// Estimated reading time in seconds (230 words per minute).
    pub reading_time_seconds: usize,
    pub issues_by_category: BTreeMap<Category, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub language: Language,
    pub language_detected: bool,
    pub matches: Vec<Match>,
    pub stats: Stats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub language: Language,
    pub original: String,
    pub corrected: String,
    pub applied: Vec<Match>,
    pub remaining: Vec<Match>,
}

/// Guesses the language from frequent function words.
pub fn detect_language(text: &str) -> Language {
    const FR: &[&str] = &[
        "le", "la", "les", "de", "des", "du", "un", "une", "et", "est", "en", "que", "qui", "pas", "pour", "dans",
        "ce", "il", "elle", "je", "nous", "vous", "sur", "avec", "au", "aux", "mais", "ou", "sont", "être", "a", "à",
        "ne", "se", "son", "sa", "ses", "cette", "plus", "très",
    ];
    const EN: &[&str] = &[
        "the", "and", "of", "to", "is", "in", "that", "it", "you", "for", "on", "with", "as", "are", "this", "be",
        "was", "have", "not", "but", "they", "he", "she", "we", "at", "by", "from", "or", "an", "will", "my", "your",
        "what", "there", "can", "would", "i",
    ];
    let (mut fr, mut en) = (0i32, 0i32);
    for word in text.split(|c: char| !c.is_alphabetic() && c != '\'').take(400) {
        let w = word.to_lowercase();
        if w.is_empty() {
            continue;
        }
        if FR.contains(&w.as_str()) {
            fr += 1;
        }
        if EN.contains(&w.as_str()) {
            en += 1;
        }
        if w.chars().any(|c| "éèêàùçôîœ".contains(c)) {
            fr += 1;
        }
    }
    if en > fr {
        Language::En
    } else {
        Language::Fr
    }
}

/// Lists every rule, optionally for one language.
pub fn rules(lang: Option<Language>) -> Vec<RuleInfo> {
    rules::registry()
        .iter()
        .filter(|r| lang.is_none() || r.language.is_none() || r.language == lang)
        .map(|r| r.info())
        .collect()
}

fn category_priority(c: Category) -> u8 {
    match c {
        Category::Conjugation => 0,
        Category::Grammar => 1,
        Category::Spelling => 2,
        Category::Punctuation => 3,
        Category::Typography => 4,
        Category::Style => 5,
    }
}

fn run_rules(text: &str, options: &CheckOptions) -> (Language, bool, Vec<Issue>, Stats) {
    let (lang, detected) = match options.language {
        Some(l) => (l, false),
        None => (detect_language(text), true),
    };
    let max_words = options.max_sentence_words.unwrap_or(DEFAULT_MAX_SENTENCE_WORDS);
    let ctx = Ctx::new(text, lang, &options.user_words, max_words);
    let mut issues = Vec::new();
    for rule in rules::rules_for(lang) {
        let enabled =
            if options.disabled_rules.contains(rule.id) || options.disabled_categories.contains(&rule.category) {
                false
            } else {
                rule.enabled_by_default || options.enabled_rules.contains(rule.id)
            };
        if !enabled {
            continue;
        }
        let mut found = Vec::new();
        (rule.check)(&ctx, &mut found);
        issues.extend(found.into_iter().map(|f| Issue {
            rule_id: rule.id,
            category: rule.category,
            severity: f.severity,
            message: f.message,
            start: f.start,
            end: f.end,
            replacements: f.replacements,
        }));
    }

    // Overlapping issues: keep the most important one.
    issues.sort_by_key(|i| (category_priority(i.category), i.severity, i.start));
    let mut kept: Vec<Issue> = Vec::new();
    for issue in issues {
        let overlaps = kept
            .iter()
            .any(|k| issue.start < k.end && k.start < issue.end && !(issue.start == issue.end || k.start == k.end));
        // Style hints spanning a whole sentence never hide or get hidden.
        let wide = issue.rule_id == "LONG_SENTENCE";
        if !overlaps || wide {
            kept.push(issue);
        }
    }
    kept.sort_by_key(|i| (i.start, i.end));

    let words = ctx.tokens.iter().filter(|t| t.is_word() && !t.text.ends_with(['\'', '’'])).count();
    let mut stats = Stats {
        characters: text.chars().count(),
        words,
        sentences: ctx.sentences.len(),
        reading_time_seconds: (words * 60).div_ceil(230),
        issues_by_category: BTreeMap::new(),
    };
    for i in &kept {
        *stats.issues_by_category.entry(i.category).or_default() += 1;
    }
    (lang, detected, kept, stats)
}

fn to_matches(text: &str, issues: Vec<Issue>) -> Vec<Match> {
    let index = Utf16Index::new(text);
    let lang_sentences = |start: usize, end: usize| -> String {
        let s = text[..start].rfind(['.', '!', '?', '\n']).map_or(0, |p| p + 1);
        let e = text[end..].find(['.', '!', '?', '\n']).map_or(text.len(), |p| end + p + 1);
        let s = text.floor_char_boundary_compat(s);
        let e = text.floor_char_boundary_compat(e.min(text.len()));
        text[s..e].trim().chars().take(300).collect()
    };
    issues
        .into_iter()
        .map(|i| {
            let offset = index.to_utf16(i.start);
            let end16 = index.to_utf16(i.end);
            Match {
                rule_id: i.rule_id.to_string(),
                category: i.category,
                severity: i.severity,
                message: i.message,
                offset,
                length: end16 - offset,
                text: text[i.start..i.end].to_string(),
                replacements: i.replacements,
                sentence: lang_sentences(i.start, i.end),
                byte_start: i.start,
                byte_end: i.end,
            }
        })
        .collect()
}

trait CharBoundary {
    fn floor_char_boundary_compat(&self, i: usize) -> usize;
}

impl CharBoundary for str {
    fn floor_char_boundary_compat(&self, mut i: usize) -> usize {
        while i > 0 && !self.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}

/// Checks `text` and returns every issue found.
pub fn check(text: &str, options: &CheckOptions) -> CheckResult {
    let (language, language_detected, issues, stats) = run_rules(text, options);
    CheckResult { language, language_detected, matches: to_matches(text, issues), stats }
}

/// Applies the first suggestion of each issue (optionally limited to some categories).
pub fn correct(text: &str, options: &CheckOptions, categories: Option<&HashSet<Category>>) -> Correction {
    let result = check(text, options);
    let mut applied = Vec::new();
    let mut remaining = Vec::new();
    let mut corrected = String::with_capacity(text.len());
    let mut cursor = 0;
    for m in result.matches {
        let allowed = categories.is_none_or(|c| c.contains(&m.category));
        // Style hints are suggestions, not corrections: never applied automatically.
        let automatic = m.severity != Severity::Hint || categories.is_some();
        if allowed && automatic && !m.replacements.is_empty() && m.byte_start >= cursor {
            corrected.push_str(&text[cursor..m.byte_start]);
            corrected.push_str(&m.replacements[0]);
            cursor = m.byte_end;
            applied.push(m);
        } else {
            remaining.push(m);
        }
    }
    corrected.push_str(&text[cursor..]);
    Correction { language: result.language, original: text.to_string(), corrected, applied, remaining }
}

/// Conjugates a verb in the given language (French by default).
pub fn conjugate(verb: &str, lang: Language) -> Result<Conjugation, ConjugationError> {
    match lang {
        Language::Fr => {
            let fr = crate::resources::french();
            let (inf, _) = conjugation::fr::parse_infinitive(verb);
            let second = fr.verbs.is_second_group(&inf);
            if second.is_none() && fr.speller.check(&inf) {
                // A real word, but not a verb ("table", "hier").
                return Err(ConjugationError::NotAVerb(inf));
            }
            conjugation::fr::conjugate(verb, second, second.is_some())
        }
        Language::En => conjugation::en::conjugate(verb),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_language() {
        assert_eq!(detect_language("Le chat est sur la table et il dort."), Language::Fr);
        assert_eq!(detect_language("The cat is on the table and it sleeps."), Language::En);
    }
}
