//! Language resources (dictionaries, lexicon, verb index), loaded once per process.

use crate::affix::Lexicon;
use crate::conjugation::fr::{simple_forms, SimpleForms, SimpleTense};
use crate::types::Language;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const FR_AFF: &str = include_str!("../dictionaries/fr/fr.aff");
const FR_DIC: &str = include_str!("../dictionaries/fr/fr.dic");
const EN_AFF: &str = include_str!("../dictionaries/en/en.aff");
const EN_DIC: &str = include_str!("../dictionaries/en/en.dic");

/// What a verb form is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    Finite(SimpleTense, u8),
    /// 0 = 2nd singular, 1 = 1st plural, 2 = 2nd plural.
    Imperative(u8),
    Infinitive,
    PresentParticiple,
    /// 0 = masc. sing., 1 = fem. sing., 2 = masc. plur., 3 = fem. plur.
    PastParticiple(u8),
}

impl FormKind {
    fn encode(self) -> u8 {
        match self {
            FormKind::Finite(t, p) => (t as u8) * 8 + p,
            FormKind::Imperative(k) => 56 + k,
            FormKind::Infinitive => 64,
            FormKind::PresentParticiple => 65,
            FormKind::PastParticiple(v) => 66 + v,
        }
    }

    fn decode(code: u8) -> FormKind {
        match code {
            0..=55 => FormKind::Finite(SimpleTense::from_index(code / 8).unwrap_or(SimpleTense::IndPresent), code % 8),
            56..=63 => FormKind::Imperative(code - 56),
            64 => FormKind::Infinitive,
            65 => FormKind::PresentParticiple,
            _ => FormKind::PastParticiple(code - 66),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerbHit {
    pub verb: u32,
    pub kind: FormKind,
}

/// Reverse index from every conjugated form to the verbs that produce it.
#[derive(Default)]
pub struct VerbIndex {
    pub verbs: Vec<String>,
    second_group: Vec<bool>,
    forms: HashMap<Box<str>, Vec<u32>>,
}

impl VerbIndex {
    pub fn build(lexicon: &Lexicon) -> VerbIndex {
        let mut index = VerbIndex::default();
        let mut forms: HashMap<Box<str>, Vec<u32>> = HashMap::with_capacity(lexicon.verbs.len() * 40);
        for verb in &lexicon.verbs {
            let second = lexicon.second_group.contains(verb);
            let Some(sf) = simple_forms(verb, Some(second)) else { continue };
            let id = index.verbs.len() as u32;
            index.verbs.push(verb.clone());
            index.second_group.push(second);
            let mut add = |form: &str, kind: FormKind| {
                let code = (id << 8) | kind.encode() as u32;
                let entry = forms.entry(form.into()).or_default();
                if !entry.contains(&code) {
                    entry.push(code);
                }
            };
            for t in SimpleTense::ALL {
                for p in 0..6 {
                    if let Some(f) = sf.get(t, p) {
                        add(f, FormKind::Finite(t, p as u8));
                    }
                }
            }
            for (k, f) in sf.imperative.iter().enumerate() {
                if let Some(f) = f {
                    add(f, FormKind::Imperative(k as u8));
                }
            }
            add(verb, FormKind::Infinitive);
            if let Some(f) = &sf.present_participle {
                add(f, FormKind::PresentParticiple);
            }
            for (v, f) in sf.past_participle.iter().enumerate() {
                add(f, FormKind::PastParticiple(v as u8));
            }
        }
        forms.shrink_to_fit();
        index.forms = forms;
        index
    }

    pub fn lookup(&self, form: &str) -> Vec<VerbHit> {
        self.forms
            .get(form)
            .map(|codes| {
                codes.iter().map(|&c| VerbHit { verb: c >> 8, kind: FormKind::decode((c & 0xff) as u8) }).collect()
            })
            .unwrap_or_default()
    }

    pub fn is_verb_form(&self, form: &str) -> bool {
        self.forms.contains_key(form)
    }

    pub fn verb_id(&self, infinitive: &str) -> Option<u32> {
        self.verbs.iter().position(|v| v == infinitive).map(|i| i as u32)
    }

    pub fn infinitive(&self, id: u32) -> &str {
        &self.verbs[id as usize]
    }

    pub fn is_second_group(&self, infinitive: &str) -> Option<bool> {
        self.verb_id(infinitive).map(|i| self.second_group[i as usize])
    }

    pub fn simple_forms(&self, id: u32) -> Option<SimpleForms> {
        simple_forms(&self.verbs[id as usize], Some(self.second_group[id as usize]))
    }
}

pub struct Speller {
    dict: spellbook::Dictionary,
    cache: Mutex<HashMap<String, Vec<String>>>,
}

impl Speller {
    fn new(aff: &str, dic: &str) -> Speller {
        Speller {
            dict: spellbook::Dictionary::new(aff, dic).expect("bundled dictionary is valid"),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn check(&self, word: &str) -> bool {
        if self.dict.check(word) {
            return true;
        }
        if word.contains('’') {
            return self.dict.check(&word.replace('’', "'"));
        }
        false
    }

    /// Spelling suggestions, best first, cached.
    pub fn suggest(&self, word: &str) -> Vec<String> {
        if let Some(hit) = self.cache.lock().unwrap().get(word) {
            return hit.clone();
        }
        let mut raw = Vec::new();
        self.dict.suggest(&word.replace('’', "'"), &mut raw);
        let (single, multi): (Vec<String>, Vec<String>) = raw.into_iter().partition(|s| !s.contains(' '));
        let mut out: Vec<String> = single.into_iter().chain(multi).take(6).collect();
        if word.contains('’') {
            out = out.into_iter().map(|s| s.replace('\'', "’")).collect();
        }
        let mut cache = self.cache.lock().unwrap();
        if cache.len() > 20_000 {
            cache.clear();
        }
        cache.insert(word.to_string(), out.clone());
        out
    }
}

pub struct FrenchResources {
    pub speller: Speller,
    pub lexicon: Lexicon,
    pub verbs: VerbIndex,
}

impl FrenchResources {
    /// True when `word` can be a noun or adjective (not only a verb form).
    pub fn is_nominal(&self, word: &str) -> bool {
        self.lexicon.is_nominal(word)
    }

    /// True when `word` is a verb form and nothing else in the dictionary.
    pub fn is_verb_only(&self, word: &str) -> bool {
        self.verbs.is_verb_form(word) && !self.lexicon.contains(word)
    }
}

pub struct EnglishResources {
    pub speller: Speller,
}

static FRENCH: OnceLock<FrenchResources> = OnceLock::new();
static ENGLISH: OnceLock<EnglishResources> = OnceLock::new();

pub fn french() -> &'static FrenchResources {
    FRENCH.get_or_init(|| {
        let (speller, lexicon) = std::thread::scope(|s| {
            let sp = s.spawn(|| Speller::new(FR_AFF, FR_DIC));
            let lexicon = Lexicon::build(FR_AFF, FR_DIC);
            (sp.join().expect("speller thread"), lexicon)
        });
        let verbs = VerbIndex::build(&lexicon);
        FrenchResources { speller, lexicon, verbs }
    })
}

pub fn english() -> &'static EnglishResources {
    ENGLISH.get_or_init(|| EnglishResources { speller: Speller::new(EN_AFF, EN_DIC) })
}

/// Loads every language eagerly (in parallel) so the first request is fast.
pub fn preload() {
    std::thread::scope(|s| {
        s.spawn(|| {
            french();
        });
        s.spawn(|| {
            english();
        });
    });
}

pub fn speller(lang: Language) -> &'static Speller {
    match lang {
        Language::Fr => &french().speller,
        Language::En => &english().speller,
    }
}
