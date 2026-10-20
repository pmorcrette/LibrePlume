//! Minimal Hunspell affix expander used to build a word-class lexicon.
//!
//! Spell checking itself is delegated to `spellbook`; this module only reads the
//! dictionary to learn which words are nouns/adjectives (and their number) and which
//! entries are verbs. Only suffix rules are expanded, which is all the French
//! dictionary needs for that purpose.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
enum CondPart {
    Any,
    Char(char),
    Set(Vec<char>, bool),
}

#[derive(Debug, Clone)]
struct SuffixRule {
    strip: String,
    add: String,
    cond: Vec<CondPart>,
}

impl SuffixRule {
    fn applies(&self, word: &str) -> bool {
        if !word.ends_with(&self.strip) {
            return false;
        }
        let chars: Vec<char> = word.chars().collect();
        if self.cond.len() > chars.len() {
            return false;
        }
        let tail = &chars[chars.len() - self.cond.len()..];
        self.cond.iter().zip(tail).all(|(part, &c)| match part {
            CondPart::Any => true,
            CondPart::Char(x) => *x == c,
            CondPart::Set(set, negated) => set.contains(&c) != *negated,
        })
    }

    fn apply(&self, word: &str) -> String {
        let mut out = word[..word.len() - self.strip.len()].to_string();
        out.push_str(&self.add);
        out
    }
}

fn parse_condition(cond: &str) -> Vec<CondPart> {
    if cond == "." {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut chars = cond.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => parts.push(CondPart::Any),
            '[' => {
                let negated = chars.peek() == Some(&'^');
                if negated {
                    chars.next();
                }
                let mut set = Vec::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                    set.push(c);
                }
                parts.push(CondPart::Set(set, negated));
            }
            c => parts.push(CondPart::Char(c)),
        }
    }
    parts
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlagMode {
    Single,
    Long,
    Num,
    Utf8,
}

fn split_flags(flags: &str, mode: FlagMode) -> Vec<String> {
    match mode {
        FlagMode::Single | FlagMode::Utf8 => flags.chars().map(String::from).collect(),
        FlagMode::Long => {
            let chars: Vec<char> = flags.chars().collect();
            chars.chunks(2).map(|c| c.iter().collect()).collect()
        }
        FlagMode::Num => flags.split(',').map(str::to_string).collect(),
    }
}

/// Word classes stored in the lexicon (bit set).
pub mod class {
    /// Noun or adjective, singular.
    pub const SINGULAR: u8 = 1;
    /// Noun or adjective, plural.
    pub const PLURAL: u8 = 2;
    /// Comes from a capitalised entry (proper noun, acronym).
    pub const PROPER: u8 = 4;
    /// Invariable word or other non-verbal entry without inflection.
    pub const OTHER: u8 = 8;
}

#[derive(Debug, Default)]
pub struct Lexicon {
    /// Lowercased form → class bits. Contains non-verbal words only.
    pub forms: HashMap<Box<str>, u8>,
    /// Infinitives found in the dictionary.
    pub verbs: Vec<String>,
    /// Infinitives in -ir that conjugate like "finir".
    pub second_group: HashSet<String>,
}

impl Lexicon {
    pub fn class(&self, word: &str) -> u8 {
        self.forms.get(word).copied().unwrap_or(0)
    }

    pub fn is_nominal(&self, word: &str) -> bool {
        self.class(word) & (class::SINGULAR | class::PLURAL) != 0
    }

    pub fn contains(&self, word: &str) -> bool {
        self.forms.contains_key(word)
    }

    /// Builds the lexicon from Hunspell `.aff` and `.dic` sources.
    pub fn build(aff: &str, dic: &str) -> Lexicon {
        let mut mode = FlagMode::Single;
        let mut needaffix = None;
        let mut forbidden = None;
        let mut rules: HashMap<String, Vec<SuffixRule>> = HashMap::new();
        for line in aff.lines() {
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("FLAG") => {
                    mode = match fields.next() {
                        Some("long") => FlagMode::Long,
                        Some("num") => FlagMode::Num,
                        Some("UTF-8") => FlagMode::Utf8,
                        _ => FlagMode::Single,
                    }
                }
                Some("NEEDAFFIX") => needaffix = fields.next().map(str::to_string),
                Some("FORBIDDENWORD") => forbidden = fields.next().map(str::to_string),
                Some("SFX") => {
                    let parts: Vec<&str> = fields.collect();
                    // Header lines have 3 fields (flag, cross, count).
                    if parts.len() < 4 {
                        continue;
                    }
                    let flag = parts[0].to_string();
                    let strip = if parts[1] == "0" { "" } else { parts[1] };
                    let add = parts[2].split('/').next().unwrap_or_default();
                    let add = if add == "0" { "" } else { add };
                    rules.entry(flag).or_default().push(SuffixRule {
                        strip: strip.to_string(),
                        add: add.to_string(),
                        cond: parse_condition(parts[3]),
                    });
                }
                _ => {}
            }
        }

        let verbal: HashSet<&str> = rules
            .iter()
            .filter(|(_, rs)| rs.iter().any(|r| r.add.ends_with("ons") || r.add.ends_with("ant")))
            .map(|(f, _)| f.as_str())
            .collect();
        let finir_like: HashSet<&str> = rules
            .iter()
            .filter(|(_, rs)| rs.iter().any(|r| r.add.ends_with("issons")))
            .map(|(f, _)| f.as_str())
            .collect();

        let mut lex = Lexicon::default();
        let mut seen_verbs = HashSet::new();
        for line in dic.lines().skip(1) {
            let entry = line.split(['\t', ' ']).next().unwrap_or_default();
            if entry.is_empty() {
                continue;
            }
            let (word, flags) = match entry.find('/') {
                Some(i) => (&entry[..i], split_flags(&entry[i + 1..], mode)),
                None => (entry, Vec::new()),
            };
            if forbidden.as_ref().is_some_and(|f| flags.contains(f)) {
                continue;
            }
            let is_verb = flags.iter().any(|f| verbal.contains(f.as_str()))
                && (word.ends_with("er") || word.ends_with("ir") || word.ends_with("re") || word.ends_with("oir"));
            if is_verb {
                if seen_verbs.insert(word.to_string()) {
                    if flags.iter().any(|f| finir_like.contains(f.as_str())) {
                        lex.second_group.insert(word.to_string());
                    }
                    lex.verbs.push(word.to_string());
                }
                continue;
            }
            let proper = word.chars().next().is_some_and(char::is_uppercase);
            let extra = if proper { class::PROPER } else { 0 };
            let lemma_ends_s = word.ends_with(['s', 'x', 'z']);
            let mut inflected = false;
            for flag in &flags {
                let Some(rs) = rules.get(flag) else { continue };
                if verbal.contains(flag.as_str()) {
                    continue;
                }
                for rule in rs.iter().filter(|r| r.applies(word)) {
                    inflected = true;
                    let form = rule.apply(word);
                    let number = if form.ends_with(['s', 'x', 'z']) {
                        if lemma_ends_s && form.len() == word.len() {
                            class::SINGULAR | class::PLURAL
                        } else {
                            class::PLURAL
                        }
                    } else {
                        class::SINGULAR
                    };
                    *lex.forms.entry(form.to_lowercase().into()).or_default() |= number | extra;
                }
            }
            let needs_affix = needaffix.as_ref().is_some_and(|f| flags.contains(f));
            if !inflected && !needs_affix {
                *lex.forms.entry(word.to_lowercase().into()).or_default() |= class::OTHER | extra;
            }
        }
        lex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_suffixes() {
        let aff = "FLAG long\nSFX S. Y 2\nSFX S. 0 0/L' [^sxz]\nSFX S. 0 s/D' [^sxz]\nSFX a0 Y 1\nSFX a0 er ons/n' er\nSFX f0 Y 1\nSFX f0 ir issons ir\n";
        let dic = "4\nchat/S.\nchanter/a0\nfinir/f0\navec\n";
        let lex = Lexicon::build(aff, dic);
        assert_eq!(lex.class("chat"), class::SINGULAR);
        assert_eq!(lex.class("chats"), class::PLURAL);
        assert_eq!(lex.class("avec"), class::OTHER);
        assert_eq!(lex.verbs, vec!["chanter", "finir"]);
        assert!(lex.second_group.contains("finir"));
        assert!(!lex.contains("chanter"));
    }
}
