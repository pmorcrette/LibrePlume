use super::{Ctx, Found, RuleDef};
use crate::text::{is_apostrophe, match_case, TokenKind};
use crate::types::{Category, Language, Severity};

pub fn rules() -> Vec<RuleDef> {
    vec![RuleDef {
        id: "SPELLING",
        language: None,
        category: Category::Spelling,
        description: "Mots absents du dictionnaire / Words missing from the dictionary",
        enabled_by_default: true,
        check: spelling,
    }]
}

/// Clitics that may follow a hyphen in French ("dit-il", "donne-moi", "a-t-elle").
const FR_HYPHEN_CLITICS: &[&str] = &[
    "je", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles", "moi", "toi", "lui", "leur", "en", "y", "le", "la",
    "les", "ce", "t", "là", "ci", "même", "mêmes",
];

/// At most this many distinct misspellings get suggestions per request (suggesting is slow).
const MAX_SUGGESTED: usize = 40;

fn spelling(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let speller = crate::resources::speller(ctx.lang);
    let mut wrong: Vec<(usize, bool)> = Vec::new();
    for (i, t) in ctx.tokens.iter().enumerate() {
        if t.kind != TokenKind::Word {
            continue;
        }
        let word = t.text;
        if word.chars().any(|c| c.is_ascii_digit()) || t.is_all_upper() && word.chars().count() > 1 {
            continue;
        }
        // Elided French tokens ("l'", "qu'") are closed-class and always valid.
        if ctx.lang == Language::Fr && word.chars().last().is_some_and(is_apostrophe) {
            continue;
        }
        if ctx.user_words.contains(word) || ctx.user_words.contains(&t.lower) {
            continue;
        }
        if speller.check(word) {
            continue;
        }
        if word.contains('-') && hyphen_parts_ok(ctx, word) {
            continue;
        }
        let proper = t.starts_upper() && !ctx.starts_sentence(i);
        wrong.push((i, proper));
    }

    // Suggestions are computed in parallel: each lookup can take tens of milliseconds.
    let mut uniq: Vec<&str> = Vec::new();
    for &(i, _) in &wrong {
        let w = ctx.tokens[i].text;
        if !uniq.contains(&w) && uniq.len() < MAX_SUGGESTED {
            uniq.push(w);
        }
    }
    let suggestions: Vec<(&str, Vec<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = uniq
            .chunks(uniq.len().div_ceil(available_threads()).max(1))
            .map(|chunk| s.spawn(move || chunk.iter().map(|w| (*w, speller.suggest(w))).collect::<Vec<_>>()))
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });

    for (i, proper) in wrong {
        let t = &ctx.tokens[i];
        let sugg = suggestions
            .iter()
            .find(|(w, _)| *w == t.text)
            .map(|(_, s)| s.iter().map(|s| match_case(t.text, s)).collect::<Vec<_>>())
            .unwrap_or_default();
        let (severity, message) = match (ctx.lang, proper) {
            (Language::Fr, false) => {
                (Severity::Error, format!("Mot inconnu : « {} ». Faute d’orthographe possible.", t.text))
            }
            (Language::Fr, true) => (
                Severity::Warning,
                format!("Mot inconnu : « {} ». S’il s’agit d’un nom propre, ajoutez-le au dictionnaire.", t.text),
            ),
            (Language::En, false) => (Severity::Error, format!("Possible spelling mistake: “{}”.", t.text)),
            (Language::En, true) => (
                Severity::Warning,
                format!("Unknown word “{}”. If it is a proper noun, add it to the dictionary.", t.text),
            ),
        };
        let mut dedup = Vec::new();
        for s in sugg {
            if !dedup.contains(&s) && s != t.text {
                dedup.push(s);
            }
        }
        out.push(Found::new(severity, message, t.start..t.end, dedup));
    }
}

fn available_threads() -> usize {
    std::thread::available_parallelism().map_or(2, |n| n.get()).min(8)
}

fn hyphen_parts_ok(ctx: &Ctx<'_>, word: &str) -> bool {
    let speller = crate::resources::speller(ctx.lang);
    word.split('-').filter(|p| !p.is_empty()).all(|part| {
        let lower = part.to_lowercase();
        speller.check(part)
            || ctx.user_words.contains(part)
            || (ctx.lang == Language::Fr && FR_HYPHEN_CLITICS.contains(&lower.as_str()))
    })
}
