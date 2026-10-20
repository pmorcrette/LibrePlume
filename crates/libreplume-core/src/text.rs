//! Tokenisation, sentence segmentation and offset helpers.

use crate::types::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// Letters, possibly with inner hyphens, apostrophes and digits.
    Word,
    Number,
    /// Horizontal whitespace (including non-breaking spaces).
    Space,
    Newline,
    Punct,
    /// URLs, e-mail addresses and similar: never checked.
    Opaque,
    Symbol,
}

#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
    /// Lowercased text, only filled for words.
    pub lower: String,
}

impl<'a> Token<'a> {
    pub fn is_word(&self) -> bool {
        self.kind == TokenKind::Word
    }

    pub fn is_space(&self) -> bool {
        self.kind == TokenKind::Space
    }

    pub fn is(&self, s: &str) -> bool {
        self.kind == TokenKind::Word && self.lower == s
    }

    pub fn is_any(&self, list: &[&str]) -> bool {
        self.kind == TokenKind::Word && list.contains(&self.lower.as_str())
    }

    pub fn is_punct(&self, s: &str) -> bool {
        self.kind == TokenKind::Punct && self.text == s
    }

    pub fn starts_upper(&self) -> bool {
        self.text.chars().next().is_some_and(char::is_uppercase)
    }

    pub fn is_all_upper(&self) -> bool {
        let mut letters = self.text.chars().filter(|c| c.is_alphabetic()).peekable();
        letters.peek().is_some() && letters.all(char::is_uppercase)
    }
}

pub fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '’' || c == 'ʼ'
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || is_combining(c)
}

fn is_combining(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}')
}

fn is_hspace(c: char) -> bool {
    c != '\n' && c != '\r' && c.is_whitespace()
}

/// French elided forms that are split from the word they attach to.
const FR_ELISIONS: &[&str] = &[
    "l", "d", "j", "m", "t", "s", "n", "c", "ç", "qu", "jusqu", "lorsqu", "puisqu", "quoiqu",
];

/// Words containing an apostrophe that must stay whole.
const FR_APOSTROPHE_WORDS: &[&str] = &[
    "aujourd", "quelqu", "presqu", "prud", "entr", "grand", "main-d",
];

/// Splits `text` into tokens. French elisions (`l'`, `qu'`…) become separate tokens.
pub fn tokenize(text: &str, lang: Language) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let bytes_len = text.len();
    while i < bytes_len {
        let rest = &text[i..];
        let c = rest.chars().next().unwrap();
        let (kind, len) = if c == '\n' || c == '\r' {
            let len = if rest.starts_with("\r\n") { 2 } else { 1 };
            (TokenKind::Newline, len)
        } else if is_hspace(c) {
            let len = rest
                .char_indices()
                .find(|&(_, ch)| !is_hspace(ch))
                .map_or(rest.len(), |(idx, _)| idx);
            (TokenKind::Space, len)
        } else if let Some(len) = opaque_len(rest) {
            (TokenKind::Opaque, len)
        } else if c.is_ascii_digit() {
            let len = number_len(rest);
            // Things like "2D" or "MP3" are words.
            let after = &rest[len..];
            if after.chars().next().is_some_and(is_word_char) {
                (TokenKind::Word, word_len(rest))
            } else {
                (TokenKind::Number, len)
            }
        } else if is_word_char(c) {
            (TokenKind::Word, word_len(rest))
        } else if c.is_ascii_punctuation() || is_unicode_punct(c) {
            let len = if rest.starts_with("...") {
                3
            } else {
                c.len_utf8()
            };
            (TokenKind::Punct, len)
        } else {
            (TokenKind::Symbol, c.len_utf8())
        };
        let piece = &text[i..i + len];
        if kind == TokenKind::Word && lang == Language::Fr {
            push_french_word(&mut tokens, piece, i);
        } else {
            tokens.push(make_token(kind, piece, i));
        }
        i += len;
    }
    tokens
}

fn make_token(kind: TokenKind, text: &str, start: usize) -> Token<'_> {
    Token {
        kind,
        text,
        start,
        end: start + text.len(),
        lower: if kind == TokenKind::Word {
            text.to_lowercase()
        } else {
            String::new()
        },
    }
}

fn push_french_word<'a>(tokens: &mut Vec<Token<'a>>, piece: &'a str, start: usize) {
    let mut offset = 0;
    let mut rest = piece;
    while let Some((idx, ap)) = rest.char_indices().find(|&(_, c)| is_apostrophe(c)) {
        let head = rest[..idx].to_lowercase();
        let keep_whole = FR_APOSTROPHE_WORDS.iter().any(|w| head == *w)
            || !FR_ELISIONS.contains(&head.as_str());
        if keep_whole {
            break;
        }
        let cut = idx + ap.len_utf8();
        tokens.push(make_token(TokenKind::Word, &rest[..cut], start + offset));
        offset += cut;
        rest = &rest[cut..];
    }
    if !rest.is_empty() {
        tokens.push(make_token(TokenKind::Word, rest, start + offset));
    }
}

fn is_unicode_punct(c: char) -> bool {
    matches!(
        c,
        '«' | '»' | '“' | '”' | '‘' | '’' | '…' | '–' | '—' | '¿' | '¡' | '‹' | '›' | '•' | '·'
    )
}

fn word_len(s: &str) -> usize {
    let mut end = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if is_word_char(c) || c.is_ascii_digit() {
            end = idx + c.len_utf8();
            continue;
        }
        // Inner hyphens and apostrophes join words: "peut-être", "aujourd'hui", "don't".
        if c == '-' || is_apostrophe(c) {
            if let Some(&(_, next)) = chars.peek() {
                if is_word_char(next) {
                    end = idx + c.len_utf8();
                    continue;
                }
            }
            // Keep a trailing elision apostrophe attached: "l'" before a space is rare
            // but "jusqu'" followed by a quote happens.
        }
        break;
    }
    end.max(s.chars().next().map_or(0, char::len_utf8))
}

fn number_len(s: &str) -> usize {
    let mut end = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if c.is_ascii_digit() {
            end = idx + 1;
        } else if matches!(c, '.' | ',') && chars.peek().is_some_and(|(_, n)| n.is_ascii_digit())
        {
            continue;
        } else {
            break;
        }
    }
    end
}

/// Length of a URL or e-mail address at the start of `s`, if any.
fn opaque_len(s: &str) -> Option<usize> {
    let lower_start: String = s.chars().take(12).collect::<String>().to_lowercase();
    let chunk_len = s
        .char_indices()
        .find(|&(_, c)| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '«' | '»'))
        .map_or(s.len(), |(i, _)| i);
    let chunk = &s[..chunk_len];
    let is_url = lower_start.starts_with("http://")
        || lower_start.starts_with("https://")
        || lower_start.starts_with("ftp://")
        || lower_start.starts_with("www.");
    let is_email = {
        let at = chunk.find('@');
        at.is_some_and(|a| {
            a > 0
                && chunk[a + 1..].contains('.')
                && chunk[..a]
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
        })
    };
    if !(is_url || is_email) {
        return None;
    }
    // Trailing punctuation belongs to the sentence, not the URL.
    let trimmed = chunk.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '\'', '’']);
    Some(trimmed.len().max(1))
}

const ABBREVIATIONS: &[&str] = &[
    "m", "mm", "mme", "mmes", "mlle", "dr", "pr", "st", "ste", "etc", "cf", "p", "pp", "env",
    "av", "apr", "j.-c", "mr", "mrs", "ms", "prof", "vs", "e.g", "i.e", "no", "vol", "fig", "ex",
    "art", "chap", "éd", "ed",
];

/// Splits the token list into sentences, returned as ranges of token indices.
pub fn sentences(tokens: &[Token<'_>]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        let mut boundary = false;
        if t.kind == TokenKind::Newline {
            // A blank line or a line break after which a new paragraph starts.
            boundary = true;
        } else if t.kind == TokenKind::Punct && matches!(t.text, "." | "!" | "?" | "…" | "...") {
            let prev_word = tokens[..i].iter().rev().find(|t| t.kind != TokenKind::Space);
            let is_abbrev = t.text == "."
                && prev_word.is_some_and(|w| {
                    w.is_word()
                        && (ABBREVIATIONS.contains(&w.lower.as_str())
                            || (w.text.chars().count() == 1 && w.starts_upper()))
                });
            // Consume trailing punctuation such as closing quotes or "?!".
            let mut j = i + 1;
            while j < tokens.len()
                && tokens[j].kind == TokenKind::Punct
                && matches!(
                    tokens[j].text,
                    "!" | "?" | "." | "»" | "”" | "\"" | ")" | "’" | "'" | "…"
                )
            {
                j += 1;
            }
            let followed_by_space = j >= tokens.len()
                || matches!(tokens[j].kind, TokenKind::Space | TokenKind::Newline);
            if !is_abbrev && followed_by_space {
                i = j - 1;
                boundary = true;
            }
        }
        if boundary {
            if start <= i {
                out.push(start..i + 1);
            }
            start = i + 1;
        }
        i += 1;
    }
    if start < tokens.len() {
        out.push(start..tokens.len());
    }
    out.retain(|r| tokens[r.clone()].iter().any(|t| t.is_word() || t.kind == TokenKind::Number));
    out
}

/// Converts sorted byte offsets to UTF-16 code unit offsets in a single pass.
pub struct Utf16Index {
    /// (byte offset, utf16 offset) for every char boundary where the two differ in step.
    boundaries: Vec<(usize, usize)>,
    len: usize,
    len16: usize,
}

impl Utf16Index {
    pub fn new(text: &str) -> Self {
        let mut boundaries = Vec::new();
        let mut u16pos = 0;
        for (b, c) in text.char_indices() {
            if c.len_utf8() != c.len_utf16() || !boundaries.is_empty() {
                boundaries.push((b, u16pos));
            }
            u16pos += c.len_utf16();
        }
        Utf16Index {
            boundaries,
            len: text.len(),
            len16: u16pos,
        }
    }

    pub fn to_utf16(&self, byte: usize) -> usize {
        if byte >= self.len {
            return self.len16;
        }
        match self.boundaries.binary_search_by_key(&byte, |&(b, _)| b) {
            Ok(i) => self.boundaries[i].1,
            Err(0) => byte,
            Err(i) => {
                // Inside an ASCII run that follows boundary i-1.
                let (b, u) = self.boundaries[i - 1];
                u + (byte - b)
            }
        }
    }

    pub fn to_byte(&self, u16off: usize) -> usize {
        if u16off >= self.len16 {
            return self.len;
        }
        match self.boundaries.binary_search_by_key(&u16off, |&(_, u)| u) {
            Ok(i) => self.boundaries[i].0,
            Err(0) => u16off,
            Err(i) => {
                let (b, u) = self.boundaries[i - 1];
                b + (u16off - u)
            }
        }
    }
}

/// Copies the case pattern of `model` onto `word` (capitalised or all caps).
pub fn match_case(model: &str, word: &str) -> String {
    let letters: Vec<char> = model.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) {
        return word.to_uppercase();
    }
    if model.chars().next().is_some_and(char::is_uppercase) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            return first.to_uppercase().chain(chars).collect();
        }
    }
    word.to_string()
}

pub fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// True when the word begins with a vowel sound for elision purposes (ignores h).
pub fn starts_with_vowel(word: &str) -> bool {
    word.chars().next().is_some_and(|c| {
        matches!(
            c.to_lowercase().next().unwrap_or(c),
            'a' | 'e' | 'i' | 'o' | 'u' | 'à' | 'â' | 'ä' | 'é' | 'è' | 'ê' | 'ë' | 'î' | 'ï' | 'ô'
                | 'ö' | 'û' | 'ù' | 'ü' | 'œ' | 'æ'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str, lang: Language) -> Vec<&str> {
        tokenize(text, lang)
            .into_iter()
            .filter(|t| t.is_word())
            .map(|t| t.text)
            .collect()
    }

    #[test]
    fn splits_french_elisions() {
        assert_eq!(
            words("L'arbre qu'il voit aujourd'hui", Language::Fr),
            vec!["L'", "arbre", "qu'", "il", "voit", "aujourd'hui"]
        );
        assert_eq!(words("jusqu'à l’école", Language::Fr), vec!["jusqu'", "à", "l’", "école"]);
        assert_eq!(words("quelqu'un", Language::Fr), vec!["quelqu'un"]);
        assert_eq!(words("don't stop", Language::En), vec!["don't", "stop"]);
    }

    #[test]
    fn keeps_hyphenated_words_and_urls() {
        let toks = tokenize("Voir https://example.org/a-b, peut-être.", Language::Fr);
        assert!(toks.iter().any(|t| t.kind == TokenKind::Opaque && t.text == "https://example.org/a-b"));
        assert!(toks.iter().any(|t| t.text == "peut-être"));
    }

    #[test]
    fn segments_sentences() {
        let text = "Bonjour M. Dupont. Comment allez-vous ? Très bien !";
        let toks = tokenize(text, Language::Fr);
        assert_eq!(sentences(&toks).len(), 3);
    }

    #[test]
    fn utf16_offsets() {
        let text = "a😀é b";
        let idx = Utf16Index::new(text);
        let b = text.find('b').unwrap();
        assert_eq!(idx.to_utf16(b), 5);
        assert_eq!(idx.to_byte(5), b);
        assert_eq!(idx.to_utf16(0), 0);
    }
}
