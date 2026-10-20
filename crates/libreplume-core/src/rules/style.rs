//! Style rules: sentence length and close repetitions.

use super::{Ctx, Found, RuleDef};
use crate::text::TokenKind;
use crate::types::{Category, Language, Severity};

pub fn rules() -> Vec<RuleDef> {
    vec![
        RuleDef {
            id: "LONG_SENTENCE",
            language: None,
            category: Category::Style,
            description: "Phrase trop longue / Sentence too long",
            enabled_by_default: true,
            check: long_sentence,
        },
        RuleDef {
            id: "WORD_REPETITION",
            language: None,
            category: Category::Style,
            description: "Même mot répété à faible distance / Same word repeated nearby",
            enabled_by_default: true,
            check: word_repetition,
        },
    ]
}

fn long_sentence(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let max = ctx.max_sentence_words;
    if max == 0 {
        return;
    }
    for s in &ctx.sentences {
        let words: Vec<usize> =
            s.clone().filter(|&i| ctx.tokens[i].is_word() && !ctx.tokens[i].text.ends_with(['\'', '’'])).collect();
        if words.len() <= max {
            continue;
        }
        let (first, last) = (words[0], *words.last().unwrap());
        let msg = match ctx.lang {
            Language::Fr => {
                format!("Phrase longue ({} mots) : envisagez de la scinder pour la rendre plus lisible.", words.len())
            }
            Language::En => {
                format!("Long sentence ({} words): consider splitting it to improve readability.", words.len())
            }
        };
        out.push(Found::new(Severity::Hint, msg, ctx.span(first, last), vec![]));
    }
}

const FR_IGNORE: &[&str] = &[
    "avoir", "être", "faire", "comme", "aussi", "alors", "après", "avant", "autre", "autres", "toujours", "encore",
    "entre", "depuis", "pendant", "cette", "elles", "leurs", "notre", "votre", "nous", "vous", "dans", "pour", "avec",
    "sans", "sous", "mais", "donc", "quand", "parce", "plus", "moins", "très", "tout", "tous", "toute", "toutes",
    "même", "mêmes", "quelque", "quelques", "celui", "celle", "ceux", "celles", "ainsi", "était", "étaient", "avait",
    "avaient", "sont", "peut", "faut", "comment", "pourquoi", "lorsque", "puisque",
];

const EN_IGNORE: &[&str] = &[
    "about", "after", "again", "also", "because", "been", "before", "being", "between", "could", "does", "doing",
    "from", "have", "having", "here", "into", "more", "most", "only", "other", "should", "some", "such", "than",
    "that", "their", "them", "then", "there", "these", "they", "this", "those", "through", "under", "very", "were",
    "what", "when", "where", "which", "while", "with", "would", "your", "will", "just", "like", "over",
];

/// Window (in words) inside which a repeated word is reported.
const WINDOW: usize = 12;

fn word_repetition(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let ignore = match ctx.lang {
        Language::Fr => FR_IGNORE,
        Language::En => EN_IGNORE,
    };
    let mut recent: Vec<(String, usize)> = Vec::new();
    let mut position = 0usize;
    for (i, t) in ctx.tokens.iter().enumerate() {
        if t.kind == TokenKind::Newline && ctx.tokens.get(i + 1).is_some_and(|n| n.kind == TokenKind::Newline) {
            recent.clear();
        }
        if !t.is_word() {
            continue;
        }
        position += 1;
        let w = &t.lower;
        if w.chars().count() < 5 || ignore.contains(&w.as_str()) || t.starts_upper() && !ctx.starts_sentence(i) {
            continue;
        }
        recent.retain(|(_, pos)| position - pos <= WINDOW);
        if recent.iter().any(|(r, _)| r == w) {
            let msg = match ctx.lang {
                Language::Fr => {
                    format!("« {} » est répété à courte distance : un synonyme rendrait le texte plus fluide.", t.text)
                }
                Language::En => format!("“{}” is repeated nearby: a synonym would make the text flow better.", t.text),
            };
            let synonyms = crate::thesaurus::replacements(t.text, ctx.lang, 5);
            out.push(Found::new(Severity::Hint, msg, t.start..t.end, synonyms));
        }
        recent.push((w.clone(), position));
    }
}
