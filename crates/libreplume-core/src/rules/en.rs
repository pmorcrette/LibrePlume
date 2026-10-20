//! English grammar rules.

use super::{cased, Ctx, Found, RuleDef};
use crate::conjugation::en::{is_known_verb, third_person, COMMON_REGULAR, IRREGULAR};
use crate::types::{Category, Language, Severity};
use std::collections::HashMap;
use std::sync::OnceLock;

fn def(
    id: &'static str,
    category: Category,
    description: &'static str,
    default: bool,
    check: super::CheckFn,
) -> RuleDef {
    RuleDef { id, language: Some(Language::En), category, description, enabled_by_default: default, check }
}

pub fn rules() -> Vec<RuleDef> {
    vec![
        def("EN_A_AN", Category::Grammar, "Use “an” before a vowel sound and “a” before a consonant sound", true, a_an),
        def(
            "EN_SUBJECT_VERB",
            Category::Conjugation,
            "Subject–verb agreement (“he go” → “he goes”, “they was” → “they were”)",
            true,
            subject_verb,
        ),
        def("EN_I_LOWERCASE", Category::Typography, "The pronoun “I” is always capitalised", true, i_lowercase),
        def("EN_PASSIVE_VOICE", Category::Style, "Passive voice (“was written by”)", false, passive_voice),
    ]
}

fn norm(ctx: &Ctx<'_>, i: usize) -> String {
    ctx.tokens[i].lower.replace('’', "'")
}

fn vowel_sound(word: &str) -> Option<bool> {
    let lower = word.to_lowercase();
    let first = lower.chars().next()?;
    if !first.is_alphabetic() {
        return None;
    }
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    // Acronyms are read letter by letter: "an FBI agent", "a UFO".
    if letters.len() >= 2 && letters.len() <= 5 && letters.iter().all(|c| c.is_uppercase()) {
        return Some("aefhilmnorsx".contains(first));
    }
    const VOWEL_H: &[&str] = &["hour", "honest", "honor", "honour", "heir"];
    if VOWEL_H.iter().any(|p| lower.starts_with(p)) {
        return Some(true);
    }
    const CONSONANT_VOWEL: &[&str] = &[
        "uni", "use", "usu", "uti", "ura", "ure", "uro", "eu", "one", "once", "ewe", "ubiq", "ufo", "uganda",
        "ukulele", "ukrain", "unanim", "uvula",
    ];
    if CONSONANT_VOWEL.iter().any(|p| lower.starts_with(p)) && lower != "unimportant" && !lower.starts_with("unin") {
        return Some(false);
    }
    Some(matches!(first, 'a' | 'e' | 'i' | 'o' | 'u'))
}

fn a_an(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            let w = norm(ctx, chunk[k]);
            if w != "a" && w != "an" {
                continue;
            }
            let next = ctx.tok(chunk[k + 1]);
            let Some(vowel) = vowel_sound(next.text) else { continue };
            let expected = if vowel { "an" } else { "a" };
            if w == expected {
                continue;
            }
            let t = ctx.tok(chunk[k]);
            out.push(Found::new(
                Severity::Error,
                format!("Use “{expected}” before “{}”.", next.text),
                t.start..t.end,
                vec![cased(ctx, chunk[k], expected)],
            ));
        }
    }
}

/// Words after which a pronoun is the subject of a new clause.
const CLAUSE_START: &[&str] = &[
    "and",
    "but",
    "or",
    "so",
    "because",
    "when",
    "if",
    "that",
    "then",
    "while",
    "although",
    "though",
    "since",
    "as",
    "where",
    "once",
    "until",
    "unless",
    "whereas",
    "after",
    "before",
    "yesterday",
    "today",
    "now",
    "also",
    "still",
    "often",
    "always",
    "never",
    "usually",
];

fn person(w: &str) -> Option<usize> {
    Some(match w {
        "i" => 0,
        "you" => 1,
        "he" | "she" | "it" => 2,
        "we" => 3,
        "they" => 5,
        _ => return None,
    })
}

/// 3rd person singular form → base form, for the verbs the rules know.
fn third_person_index() -> &'static HashMap<String, &'static str> {
    static INDEX: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    INDEX.get_or_init(|| {
        IRREGULAR
            .iter()
            .map(|(b, _, _)| *b)
            .chain(COMMON_REGULAR.iter().copied())
            .filter(|b| *b != "be")
            .map(|b| (third_person(b), b))
            .collect()
    })
}

fn subject_verb(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            let w = norm(ctx, chunk[k]);
            let Some(p) = person(&w) else { continue };
            let clause_start = k == 0 || CLAUSE_START.contains(&norm(ctx, chunk[k - 1]).as_str());
            if !clause_start {
                continue;
            }
            let prev = (k > 0).then(|| norm(ctx, chunk[k - 1]));
            let v_tok = ctx.tok(chunk[k + 1]);
            let v = norm(ctx, chunk[k + 1]);
            let singular3 = p == 2;
            let fix: Option<String> = match v.as_str() {
                "is" | "am" | "are" => {
                    let want = match p {
                        0 => "am",
                        2 => "is",
                        _ => "are",
                    };
                    (v != want).then(|| want.to_string())
                }
                "was" if matches!(p, 1 | 3 | 5) => Some("were".into()),
                "were" if matches!(p, 0 | 2) && !matches!(prev.as_deref(), Some("if" | "though" | "as")) => {
                    Some("was".into())
                }
                "has" if !singular3 => Some("have".into()),
                "have" if singular3 => Some("has".into()),
                "don't" | "dont" if singular3 => Some("doesn't".into()),
                "doesn't" if !singular3 => Some("don't".into()),
                "do" if singular3 => Some("does".into()),
                "does" if !singular3 => Some("do".into()),
                _ if singular3 && is_known_verb(&v) && v != "be" => {
                    // "he go" → "he goes" (only for frequent verbs that are not also nouns after "it").
                    Some(third_person(&v))
                }
                _ if !singular3 => third_person_index().get(&v).map(|b| b.to_string()),
                _ => None,
            };
            let Some(fix) = fix else { continue };
            // "it" + base verb is often an object in imperatives ("make it work"): already excluded
            // by requiring a clause start. Skip modal-like "need"/"dare" constructions.
            if matches!(v.as_str(), "need" | "dare" | "used") {
                continue;
            }
            out.push(Found::new(
                Severity::Error,
                format!("The verb does not agree with the subject “{}”: use “{fix}”.", ctx.tok(chunk[k]).text),
                v_tok.start..v_tok.end,
                vec![crate::text::match_case(v_tok.text, &fix)],
            ));
        }
    }
}

fn i_lowercase(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for t in &ctx.tokens {
        if !t.is_word() {
            continue;
        }
        let fixed = match t.text {
            "i" => "I",
            "i'm" => "I'm",
            "i've" => "I've",
            "i'll" => "I'll",
            "i'd" => "I'd",
            "i’m" => "I’m",
            "i’ve" => "I’ve",
            "i’ll" => "I’ll",
            "i’d" => "I’d",
            _ => continue,
        };
        out.push(Found::new(
            Severity::Error,
            "The pronoun “I” is always written in capitals.",
            t.start..t.end,
            vec![fixed.into()],
        ));
    }
}

fn passive_voice(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    const BE: &[&str] = &["am", "is", "are", "was", "were", "be", "been", "being"];
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            if !BE.contains(&norm(ctx, chunk[k]).as_str()) {
                continue;
            }
            let mut j = k + 1;
            if j < chunk.len() - 1 && norm(ctx, chunk[j]).ends_with("ly") {
                j += 1;
            }
            let v = norm(ctx, chunk[j]);
            let participle = IRREGULAR.iter().any(|(_, _, pp)| *pp == v && v.len() > 3)
                || COMMON_REGULAR.iter().any(|b| crate::conjugation::en::regular_past(b) == v);
            let followed_by_by = chunk.get(j + 1).is_some_and(|&n| norm(ctx, n) == "by");
            if participle && (followed_by_by || IRREGULAR.iter().any(|(_, _, pp)| *pp == v)) {
                out.push(Found::new(
                    Severity::Hint,
                    "Passive voice: consider rewriting in the active voice.",
                    ctx.span(chunk[k], chunk[j]),
                    vec![],
                ));
            }
        }
    }
}
