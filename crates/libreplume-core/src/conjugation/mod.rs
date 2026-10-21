//! Verb conjugation for the supported languages.

pub mod en;
pub mod fr;

use crate::types::Language;
use serde::Serialize;

/// A fully conjugated verb, ready to be displayed.
#[derive(Debug, Clone, Serialize)]
pub struct Conjugation {
    pub language: Language,
    /// The infinitive as typed by the user, normalised (e.g. "se laver").
    pub infinitive: String,
    /// Conjugation group when the language has them (French 1/2/3).
    pub group: Option<u8>,
    /// Reference verb used as a model ("chanter", "venir"…).
    pub model: String,
    pub auxiliary: String,
    pub pronominal: bool,
    /// Whether the verb was found in the dictionary.
    pub known: bool,
    pub participles: Vec<NamedForm>,
    pub moods: Vec<Mood>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedForm {
    pub id: String,
    pub name: String,
    pub form: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Mood {
    pub id: String,
    pub name: String,
    pub tenses: Vec<Tense>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tense {
    pub id: String,
    pub name: String,
    pub compound: bool,
    pub forms: Vec<PersonForm>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonForm {
    /// Pronoun as displayed ("je", "que tu", "they"…). Empty for imperative.
    pub pronoun: String,
    /// The verb form alone, including auxiliaries ("ai chanté").
    pub form: String,
    /// Pronoun and form combined, with elision ("j'ai chanté").
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConjugationError {
    Empty,
    NotAVerb(String),
}

impl std::fmt::Display for ConjugationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConjugationError::Empty => f.write_str("no verb given"),
            ConjugationError::NotAVerb(v) => write!(f, "« {v} » is not a verb infinitive this conjugator knows"),
        }
    }
}

impl std::error::Error for ConjugationError {}
