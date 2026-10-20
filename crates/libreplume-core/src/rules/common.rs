//! Language-independent typography and punctuation rules.

use super::{Ctx, Found, RuleDef};
use crate::text::{capitalize, TokenKind};
use crate::types::{Category, Language, Severity};

pub fn rules() -> Vec<RuleDef> {
    vec![
        RuleDef {
            id: "DOUBLE_SPACE",
            language: None,
            category: Category::Typography,
            description: "Espaces multiples entre deux mots / Repeated spaces between words",
            enabled_by_default: true,
            check: double_space,
        },
        RuleDef {
            id: "SPACE_BEFORE_PUNCT",
            language: None,
            category: Category::Punctuation,
            description: "Espace superflue avant une virgule ou un point / Space before a comma or period",
            enabled_by_default: true,
            check: space_before_punct,
        },
        RuleDef {
            id: "MISSING_SPACE_AFTER_PUNCT",
            language: None,
            category: Category::Punctuation,
            description: "Espace manquante après la ponctuation / Missing space after punctuation",
            enabled_by_default: true,
            check: missing_space_after,
        },
        RuleDef {
            id: "REPEATED_WORD",
            language: None,
            category: Category::Grammar,
            description: "Mot répété deux fois de suite / Word repeated twice in a row",
            enabled_by_default: true,
            check: repeated_word,
        },
        RuleDef {
            id: "SENTENCE_CAPITAL",
            language: None,
            category: Category::Typography,
            description: "Majuscule en début de phrase / Capital letter at the start of a sentence",
            enabled_by_default: true,
            check: sentence_capital,
        },
        RuleDef {
            id: "MULTIPLE_PUNCT",
            language: None,
            category: Category::Punctuation,
            description: "Ponctuation redoublée (!!, ,,) / Doubled punctuation",
            enabled_by_default: true,
            check: multiple_punct,
        },
        RuleDef {
            id: "UNPAIRED_BRACKETS",
            language: None,
            category: Category::Punctuation,
            description: "Parenthèses ou crochets non appariés / Unpaired brackets",
            enabled_by_default: true,
            check: unpaired_brackets,
        },
        RuleDef {
            id: "ELLIPSIS",
            language: None,
            category: Category::Typography,
            description: "Points de suspension typographiques (…) / Typographic ellipsis (…)",
            enabled_by_default: false,
            check: ellipsis,
        },
        RuleDef {
            id: "TYPOGRAPHIC_APOSTROPHE",
            language: None,
            category: Category::Typography,
            description: "Apostrophe typographique (’) / Typographic apostrophe (’)",
            enabled_by_default: false,
            check: typographic_apostrophe,
        },
    ]
}

fn fr(ctx: &Ctx<'_>) -> bool {
    ctx.lang == Language::Fr
}

fn double_space(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for (i, t) in ctx.tokens.iter().enumerate() {
        if t.kind != TokenKind::Space || t.text.chars().count() < 2 || !t.text.chars().all(|c| c == ' ') {
            continue;
        }
        let between_words = i > 0
            && i + 1 < ctx.tokens.len()
            && !matches!(ctx.tokens[i - 1].kind, TokenKind::Newline)
            && !matches!(ctx.tokens[i + 1].kind, TokenKind::Newline);
        if between_words {
            let msg =
                if fr(ctx) { "Espaces multiples : une seule suffit." } else { "Multiple spaces: use a single one." };
            out.push(Found::new(Severity::Hint, msg, t.start..t.end, vec![" ".into()]));
        }
    }
}

fn space_before_punct(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for i in 1..ctx.tokens.len().saturating_sub(1) {
        let (sp, p) = (&ctx.tokens[i], &ctx.tokens[i + 1]);
        if sp.kind != TokenKind::Space || p.kind != TokenKind::Punct || ctx.tokens[i - 1].kind == TokenKind::Newline {
            continue;
        }
        let bad = match ctx.lang {
            Language::Fr => matches!(p.text, "," | "." | "…" | "..." | ")" | "]"),
            Language::En => matches!(p.text, "," | "." | ";" | ":" | "!" | "?" | "…" | "..." | ")" | "]"),
        };
        if !bad || !ctx.tokens[i - 1].is_word() && ctx.tokens[i - 1].kind != TokenKind::Number {
            continue;
        }
        let msg = if fr(ctx) {
            format!("Pas d’espace avant « {} ».", p.text)
        } else {
            format!("No space before “{}”.", p.text)
        };
        out.push(Found::new(Severity::Warning, msg, sp.start..p.end, vec![p.text.to_string()]));
    }
}

fn missing_space_after(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for i in 1..ctx.tokens.len().saturating_sub(1) {
        let (p, next) = (&ctx.tokens[i], &ctx.tokens[i + 1]);
        if p.kind != TokenKind::Punct || !next.is_word() || !ctx.tokens[i - 1].is_word() {
            continue;
        }
        let missing = match p.text {
            "," | ";" => true,
            "!" | "?" => true,
            // "fin.Début" but not "e.g." or "U.S." or "node.js".
            "." => {
                let prev = &ctx.tokens[i - 1];
                next.starts_upper() && prev.text.chars().count() > 2 && !prev.is_all_upper()
            }
            ":" => ctx.lang == Language::En && next.text.chars().count() > 1,
            _ => false,
        };
        if !missing {
            continue;
        }
        let replacement = if fr(ctx) && matches!(p.text, "!" | "?" | ";") {
            format!("\u{202F}{} ", p.text)
        } else {
            format!("{} ", p.text)
        };
        let msg = if fr(ctx) {
            format!("Il manque une espace après « {} ».", p.text)
        } else {
            format!("Missing space after “{}”.", p.text)
        };
        out.push(Found::new(Severity::Warning, msg, p.start..p.end, vec![replacement]));
    }
}

const REPEAT_OK: &[&str] = &["nous", "vous", "had", "that", "is", "bye", "très", "so", "very", "da", "ha", "là", "si"];

fn repeated_word(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for chunk in &ctx.chunks {
        for w in chunk.windows(2) {
            let (a, b) = (ctx.tok(w[0]), ctx.tok(w[1]));
            // Must be separated by whitespace (not "l'l'").
            if a.end == b.start || a.lower != b.lower || REPEAT_OK.contains(&a.lower.as_str()) {
                continue;
            }
            if a.text.chars().count() == 1 && !a.text.chars().all(char::is_alphabetic) {
                continue;
            }
            let msg = if fr(ctx) {
                format!("Le mot « {} » est répété.", b.text)
            } else {
                format!("The word “{}” is repeated.", b.text)
            };
            out.push(Found::new(Severity::Warning, msg, a.end..b.end, vec![String::new()]));
        }
    }
}

fn sentence_capital(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for s in &ctx.sentences {
        let Some(first) =
            s.clone().find(|&i| ctx.tokens[i].kind != TokenKind::Space && ctx.tokens[i].kind != TokenKind::Newline)
        else {
            continue;
        };
        let t = &ctx.tokens[first];
        if !t.is_word() {
            continue;
        }
        // Only when the previous sentence really ended (not after a line break, which may be a list).
        let after_terminal = match ctx.prev_non_space(first) {
            None => true,
            Some(j) => ctx.tokens[j].kind == TokenKind::Punct && matches!(ctx.tokens[j].text, "." | "!" | "?"),
        };
        let first_char = t.text.chars().next().unwrap_or(' ');
        // Skip camelCase words such as "iPhone" or "eBay".
        let camel = t.text.chars().skip(1).any(char::is_uppercase);
        if !after_terminal || !first_char.is_lowercase() || camel {
            continue;
        }
        let msg = if fr(ctx) {
            "Une phrase commence par une majuscule."
        } else {
            "A sentence should start with a capital letter."
        };
        out.push(Found::new(Severity::Warning, msg, t.start..t.end, vec![capitalize(t.text)]));
    }
}

fn multiple_punct(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let mut i = 0;
    while i < ctx.tokens.len() {
        let t = &ctx.tokens[i];
        if t.kind == TokenKind::Punct && matches!(t.text, "!" | "?" | ",") {
            let mut j = i + 1;
            while j < ctx.tokens.len() && ctx.tokens[j].kind == TokenKind::Punct && ctx.tokens[j].text == t.text {
                j += 1;
            }
            if j - i >= 2 {
                let msg = if fr(ctx) {
                    format!("Un seul « {} » suffit.", t.text)
                } else {
                    format!("A single “{}” is enough.", t.text)
                };
                let severity = if t.text == "," { Severity::Warning } else { Severity::Hint };
                out.push(Found::new(severity, msg, t.start..ctx.tokens[j - 1].end, vec![t.text.to_string()]));
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

fn unpaired_brackets(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let mut stack: Vec<usize> = Vec::new();
    let flush = |stack: &mut Vec<usize>, out: &mut Vec<Found>| {
        for &i in stack.iter() {
            let t = &ctx.tokens[i];
            let msg = if fr(ctx) {
                format!("« {} » n’est jamais refermé.", t.text)
            } else {
                format!("“{}” is never closed.", t.text)
            };
            out.push(Found::new(Severity::Warning, msg, t.start..t.end, vec![]));
        }
        stack.clear();
    };
    for (i, t) in ctx.tokens.iter().enumerate() {
        if t.kind == TokenKind::Newline && ctx.tokens.get(i + 1).is_some_and(|n| n.kind == TokenKind::Newline) {
            flush(&mut stack, out);
        }
        if t.kind != TokenKind::Punct {
            continue;
        }
        match t.text {
            "(" | "[" => stack.push(i),
            ")" | "]" => {
                let open = if t.text == ")" { "(" } else { "[" };
                if stack.last().is_some_and(|&o| ctx.tokens[o].text == open) {
                    stack.pop();
                } else {
                    // A lone ")" after a number or letter is a list marker: "1)", "a)".
                    let list_marker = i > 0
                        && (ctx.tokens[i - 1].kind == TokenKind::Number || ctx.tokens[i - 1].text.chars().count() == 1)
                        && (i < 2 || matches!(ctx.tokens[i - 2].kind, TokenKind::Newline | TokenKind::Space));
                    if !list_marker {
                        let msg = if fr(ctx) {
                            format!("« {} » ne ferme rien.", t.text)
                        } else {
                            format!("“{}” closes nothing.", t.text)
                        };
                        out.push(Found::new(Severity::Warning, msg, t.start..t.end, vec![]));
                    }
                }
            }
            _ => {}
        }
    }
    flush(&mut stack, out);
}

fn ellipsis(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for t in &ctx.tokens {
        if t.kind == TokenKind::Punct && t.text == "..." {
            let msg =
                if fr(ctx) { "Utilisez le caractère « … »." } else { "Use the ellipsis character “…”." };
            out.push(Found::new(Severity::Hint, msg, t.start..t.end, vec!["…".into()]));
        }
    }
}

fn typographic_apostrophe(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for t in &ctx.tokens {
        if t.kind != TokenKind::Word {
            continue;
        }
        for (idx, _) in t.text.match_indices('\'') {
            let start = t.start + idx;
            let msg =
                if fr(ctx) { "Apostrophe typographique recommandée." } else { "Typographic apostrophe recommended." };
            out.push(Found::new(Severity::Hint, msg, start..start + 1, vec!["’".into()]));
        }
    }
}
