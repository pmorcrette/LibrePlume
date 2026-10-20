//! Synonyms and antonyms.
//!
//! The thesauri are LibreOffice's French thesaurus and the WordNet-based English one,
//! converted by `scripts/build-thesaurus.py` into sorted tab-separated lines (one per
//! meaning). They are embedded as text and searched by binary search over line offsets,
//! so nothing is parsed until a word is looked up.

use crate::conjugation::en::IRREGULAR;
use crate::resources::FormKind;
use crate::types::Language;
use serde::Serialize;
use std::sync::OnceLock;

const FR_THESAURUS: &str = include_str!("../thesaurus/fr.tsv");
const EN_THESAURUS: &str = include_str!("../thesaurus/en.tsv");

/// One sense of a word.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Meaning {
    /// Part of speech in English ("noun", "verb", "adjective noun"…); empty when unknown.
    pub part_of_speech: String,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Synonyms {
    pub language: Language,
    /// The word that was asked for.
    pub word: String,
    /// The thesaurus entry that was found: the word itself or its base form
    /// ("mangeaient" → "manger", "running" → "run").
    pub lemma: String,
    pub meanings: Vec<Meaning>,
}

impl Synonyms {
    /// Every distinct synonym, in order of relevance.
    pub fn all_synonyms(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for s in self.meanings.iter().flat_map(|m| &m.synonyms) {
            if !out.contains(&s.as_str()) {
                out.push(s);
            }
        }
        out
    }
}

struct Thesaurus {
    data: &'static str,
    /// Byte offset of the start of every line.
    lines: Vec<u32>,
}

impl Thesaurus {
    fn new(data: &'static str) -> Thesaurus {
        let mut lines = vec![0u32];
        lines.extend(data.match_indices('\n').map(|(i, _)| i as u32 + 1).filter(|&i| (i as usize) < data.len()));
        Thesaurus { data, lines }
    }

    fn line(&self, i: usize) -> &'static str {
        let start = self.lines[i] as usize;
        let end = self.lines.get(i + 1).map_or(self.data.len(), |&e| e as usize);
        self.data[start..end].trim_end_matches('\n')
    }

    fn head(&self, i: usize) -> &'static str {
        self.line(i).split('\t').next().unwrap_or_default()
    }

    /// Meanings of the entry whose headword equals `word`, ignoring case.
    fn lookup(&self, word: &str) -> Option<(String, Vec<Meaning>)> {
        let key = word.to_lowercase();
        // Lines are sorted by lowercased headword: find the first one not below the key.
        let first = self.lines.partition_point(|&start| {
            let line = &self.data[start as usize..];
            let head = line.split(['\t', '\n']).next().unwrap_or_default();
            head.to_lowercase() < key
        });
        let mut meanings = Vec::new();
        let mut lemma = None;
        let mut exact_case = false;
        for i in first..self.lines.len() {
            let head = self.head(i);
            if head.to_lowercase() != key {
                break;
            }
            // "Bach" and "bach" may both exist: prefer the spelling that was asked for.
            if head == word && !exact_case {
                exact_case = true;
                meanings.clear();
            } else if exact_case && head != word {
                continue;
            }
            lemma.get_or_insert_with(|| head.to_string());
            let mut cols = self.line(i).split('\t').skip(1);
            let split = |s: Option<&str>| -> Vec<String> {
                s.unwrap_or_default().split('|').filter(|w| !w.is_empty()).map(str::to_string).collect()
            };
            meanings.push(Meaning {
                part_of_speech: cols.next().unwrap_or_default().to_string(),
                synonyms: split(cols.next()),
                antonyms: split(cols.next()),
            });
        }
        if exact_case {
            lemma = Some(word.to_string());
        }
        lemma.map(|l| (l, meanings))
    }

    fn contains(&self, word: &str) -> bool {
        self.lookup(word).is_some()
    }
}

fn thesaurus(lang: Language) -> &'static Thesaurus {
    static FR: OnceLock<Thesaurus> = OnceLock::new();
    static EN: OnceLock<Thesaurus> = OnceLock::new();
    match lang {
        Language::Fr => FR.get_or_init(|| Thesaurus::new(FR_THESAURUS)),
        Language::En => EN.get_or_init(|| Thesaurus::new(EN_THESAURUS)),
    }
}

/// Possible base forms of an inflected French word, most likely first.
fn french_lemmas(word: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Past participles are often adjectives ("ravies" → "ravi" before "ravir"),
    // so their infinitive only comes after the nominal forms.
    let mut participle_of = Vec::new();
    let fr = crate::resources::french();
    for hit in fr.verbs.lookup(word) {
        let inf = fr.verbs.infinitive(hit.verb).to_string();
        match hit.kind {
            FormKind::Infinitive => {}
            FormKind::PastParticiple(_) => participle_of.push(inf),
            _ => out.push(inf),
        }
    }
    // Plural, then feminine.
    let mut singular = vec![word.to_string()];
    if let Some(stem) = word.strip_suffix("aux") {
        singular.push(format!("{stem}al"));
        singular.push(format!("{stem}ail"));
    }
    if let Some(stem) = word.strip_suffix('s').or_else(|| word.strip_suffix('x')) {
        singular.push(stem.to_string());
    }
    const FEMININE: &[(&str, &str)] = &[
        ("euse", "eux"),
        ("trice", "teur"),
        ("ive", "if"),
        ("ienne", "ien"),
        ("enne", "en"),
        ("onne", "on"),
        ("elle", "el"),
        ("ère", "er"),
        ("ète", "et"),
        ("sse", "s"),
        ("e", ""),
    ];
    for s in &singular {
        if s != word {
            out.push(s.clone());
        }
        if let Some((suffix, masc)) = FEMININE.iter().find(|(f, _)| s.ends_with(f)) {
            out.push(format!("{}{masc}", &s[..s.len() - suffix.len()]));
        }
    }
    out.extend(participle_of);
    out
}

/// Possible base forms of an inflected English word, most likely first.
fn english_lemmas(word: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (base, past, pp) in IRREGULAR {
        if word == *past || word == *pp {
            out.push(base.to_string());
        }
    }
    let rules: &[(&str, &[&str])] = &[
        ("ies", &["y"]),
        ("ied", &["y"]),
        ("ier", &["y"]),
        ("iest", &["y"]),
        ("ves", &["f", "fe"]),
        ("es", &["", "e"]),
        ("s", &[""]),
        ("ing", &["", "e"]),
        ("ed", &["", "e"]),
        ("est", &["", "e"]),
        ("er", &["", "e"]),
    ];
    for (suffix, replacements) in rules {
        if let Some(stem) = word.strip_suffix(suffix) {
            if stem.len() < 2 {
                continue;
            }
            for r in *replacements {
                out.push(format!("{stem}{r}"));
            }
            // "stopped" → "stop", "bigger" → "big".
            let b = stem.as_bytes();
            if b.len() >= 3 && b[b.len() - 1] == b[b.len() - 2] {
                out.push(stem[..stem.len() - 1].to_string());
            }
        }
    }
    out
}

/// Strips a French elision ("l'arbre" → "arbre") and normalises apostrophes.
fn normalise(word: &str, lang: Language) -> String {
    let w = word.trim().replace('’', "'");
    if lang == Language::Fr {
        if let Some((head, rest)) = w.split_once('\'') {
            if head.chars().count() <= 6 && !rest.is_empty() && !w.contains(' ') && w.to_lowercase() != "aujourd'hui" {
                return rest.to_string();
            }
        }
    }
    w
}

/// Looks up synonyms of `word`, falling back on its base form.
pub fn synonyms(word: &str, lang: Language) -> Option<Synonyms> {
    let word = normalise(word, lang);
    if word.is_empty() || word.chars().count() > 80 {
        return None;
    }
    let thes = thesaurus(lang);
    let found = thes.lookup(&word).or_else(|| {
        let lower = word.to_lowercase();
        let candidates = match lang {
            Language::Fr => french_lemmas(&lower),
            Language::En => english_lemmas(&lower),
        };
        candidates.into_iter().find(|c| thes.contains(c)).and_then(|c| thes.lookup(&c))
    });
    found.map(|(lemma, meanings)| Synonyms { language: lang, word, lemma, meanings })
}

/// Synonyms usable as direct replacements of `word` in a text: only when the word itself
/// is an entry (so the synonyms have the same form), single words, keeping the case.
pub fn replacements(word: &str, lang: Language, max: usize) -> Vec<String> {
    let Some((_, meanings)) = thesaurus(lang).lookup(&normalise(word, lang)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for s in meanings.iter().flat_map(|m| m.synonyms.iter()) {
        if out.len() >= max {
            break;
        }
        let single = !s.contains(' ') && s.to_lowercase() != word.to_lowercase();
        let cased = crate::text::match_case(word, s);
        if single && !out.contains(&cased) {
            out.push(cased);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_lookup_and_lemmas() {
        let s = synonyms("beau", Language::Fr).unwrap();
        assert_eq!(s.lemma, "beau");
        assert!(s.all_synonyms().contains(&"magnifique"));

        let s = synonyms("mangeaient", Language::Fr).unwrap();
        assert_eq!(s.lemma, "manger");
        assert_eq!(synonyms("maisons", Language::Fr).unwrap().lemma, "maison");
        // The closest entry wins: the feminine form exists, so only the plural is removed.
        assert_eq!(synonyms("heureuses", Language::Fr).unwrap().lemma, "heureuse");
        assert_eq!(synonyms("ravies", Language::Fr).unwrap().lemma, "ravi");
        assert_eq!(synonyms("l’arbre", Language::Fr).unwrap().lemma, "arbre");
        assert_eq!(synonyms("Beau", Language::Fr).unwrap().lemma, "beau");
        assert!(synonyms("zxqwv", Language::Fr).is_none());
    }

    #[test]
    fn english_lookup_and_lemmas() {
        let s = synonyms("happy", Language::En).unwrap();
        assert!(s.all_synonyms().contains(&"glad"));
        assert!(s.meanings.iter().any(|m| m.antonyms.contains(&"unhappy".to_string())));
        assert_eq!(synonyms("ran", Language::En).unwrap().lemma, "run");
        assert_eq!(synonyms("walked", Language::En).unwrap().lemma, "walk");
        assert_eq!(synonyms("went", Language::En).unwrap().lemma, "go");
        assert_eq!(synonyms("houses", Language::En).unwrap().lemma, "house");
        assert_eq!(synonyms("begged", Language::En).unwrap().lemma, "beg");
        assert_eq!(synonyms("chatting", Language::En).unwrap().lemma, "chat");
    }

    #[test]
    fn replacements_keep_case_and_form() {
        let r = replacements("Magnifique", Language::Fr, 4);
        assert!(!r.is_empty());
        assert!(r.iter().all(|w| w.chars().next().unwrap().is_uppercase() && !w.contains(' ')));
        // An inflected word has no direct replacements (they would not agree).
        assert!(replacements("maisons", Language::Fr, 4).is_empty());
    }
}
