//! Rule framework and registry.

mod common;
mod en;
mod fr;
mod phrases;
mod spelling;
mod style;

use crate::resources::{EnglishResources, FrenchResources};
use crate::text::{Token, TokenKind};
use crate::types::{Category, Language, RuleInfo, Severity};
use std::ops::Range;

/// A finding produced by a rule before it is tagged with the rule id.
#[derive(Debug, Clone)]
pub struct Found {
    pub severity: Severity,
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub replacements: Vec<String>,
}

impl Found {
    pub fn new(severity: Severity, message: impl Into<String>, span: Range<usize>, replacements: Vec<String>) -> Found {
        Found { severity, message: message.into(), start: span.start, end: span.end, replacements }
    }
}

pub type CheckFn = fn(&Ctx<'_>, &mut Vec<Found>);

pub struct RuleDef {
    pub id: &'static str,
    pub language: Option<Language>,
    pub category: Category,
    pub description: &'static str,
    pub enabled_by_default: bool,
    pub check: CheckFn,
}

impl RuleDef {
    pub fn info(&self) -> RuleInfo {
        RuleInfo {
            id: self.id,
            language: self.language,
            category: self.category,
            description: self.description,
            enabled_by_default: self.enabled_by_default,
        }
    }
}

/// Everything a rule can look at.
pub struct Ctx<'a> {
    pub text: &'a str,
    pub lang: Language,
    pub tokens: Vec<Token<'a>>,
    pub sentences: Vec<Range<usize>>,
    /// Runs of word tokens separated only by spaces (punctuation breaks a run).
    pub chunks: Vec<Vec<usize>>,
    pub user_words: &'a std::collections::HashSet<String>,
    pub max_sentence_words: usize,
    pub fr: Option<&'static FrenchResources>,
    pub en: Option<&'static EnglishResources>,
}

impl<'a> Ctx<'a> {
    pub fn new(
        text: &'a str,
        lang: Language,
        user_words: &'a std::collections::HashSet<String>,
        max_sentence_words: usize,
    ) -> Ctx<'a> {
        let tokens = crate::text::tokenize(text, lang);
        let sentences = crate::text::sentences(&tokens);
        let mut chunks = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        for (i, t) in tokens.iter().enumerate() {
            match t.kind {
                TokenKind::Word => current.push(i),
                TokenKind::Space => {}
                _ => {
                    if !current.is_empty() {
                        chunks.push(std::mem::take(&mut current));
                    }
                }
            }
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        Ctx {
            text,
            lang,
            tokens,
            sentences,
            chunks,
            user_words,
            max_sentence_words,
            fr: (lang == Language::Fr).then(crate::resources::french),
            en: (lang == Language::En).then(crate::resources::english),
        }
    }

    pub fn tok(&self, i: usize) -> &Token<'a> {
        &self.tokens[i]
    }

    /// Byte span covering tokens `a..=b`.
    pub fn span(&self, a: usize, b: usize) -> Range<usize> {
        self.tokens[a].start..self.tokens[b].end
    }

    /// Source text between the start of token `a` and the end of token `b`.
    pub fn slice(&self, a: usize, b: usize) -> &'a str {
        &self.text[self.tokens[a].start..self.tokens[b].end]
    }

    /// Index of the previous non-space token.
    pub fn prev_non_space(&self, i: usize) -> Option<usize> {
        (0..i).rev().find(|&j| self.tokens[j].kind != TokenKind::Space)
    }

    /// Index of the next non-space token.
    pub fn next_non_space(&self, i: usize) -> Option<usize> {
        (i + 1..self.tokens.len()).find(|&j| self.tokens[j].kind != TokenKind::Space)
    }

    /// True if the word token at `i` starts a sentence.
    pub fn starts_sentence(&self, i: usize) -> bool {
        match self.prev_non_space(i) {
            None => true,
            Some(j) => {
                let t = &self.tokens[j];
                t.kind == TokenKind::Newline
                    || (t.kind == TokenKind::Punct && matches!(t.text, "." | "!" | "?" | "…" | "..."))
            }
        }
    }

    /// The apostrophe style used in the text (typographic or straight).
    pub fn apostrophe(&self) -> char {
        if self.text.contains('’') {
            '’'
        } else {
            '\''
        }
    }
}

/// All rules, across languages.
pub fn registry() -> &'static [RuleDef] {
    static RULES: std::sync::OnceLock<Vec<RuleDef>> = std::sync::OnceLock::new();
    RULES.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(spelling::rules());
        v.extend(common::rules());
        v.extend(fr::rules());
        v.extend(en::rules());
        v.extend(phrases::rules());
        v.extend(style::rules());
        v
    })
}

pub fn rules_for(lang: Language) -> impl Iterator<Item = &'static RuleDef> {
    registry().iter().filter(move |r| r.language.is_none() || r.language == Some(lang))
}

/// Replaces the text of `a..=b` keeping the capitalisation of the first token.
pub fn cased(ctx: &Ctx<'_>, a: usize, replacement: &str) -> String {
    crate::text::match_case(ctx.tokens[a].text, replacement)
}
