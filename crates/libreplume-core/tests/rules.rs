//! End-to-end checks of the rule set.

use libreplume_core::{check, correct, CheckOptions, Language, Severity};

fn opts(lang: Language) -> CheckOptions {
    CheckOptions { language: Some(lang), ..Default::default() }
}

/// Asserts that `text` triggers `rule` on `flagged` with `fix` as first suggestion.
fn expect(lang: Language, text: &str, rule: &str, flagged: &str, fix: &str) {
    let r = check(text, &opts(lang));
    let hit = r.matches.iter().find(|m| m.rule_id == rule && m.text == flagged);
    let Some(hit) = hit else {
        panic!("{rule} not triggered on {flagged:?} in {text:?}; got {:#?}", r.matches);
    };
    assert_eq!(hit.replacements.first().map(String::as_str), Some(fix), "in {text:?}");
}

#[test]
fn french_errors() {
    use Language::Fr;
    expect(Fr, "Il à mangé une pomme.", "FR_A_ACCENT", "à", "a");
    expect(Fr, "Tu mange trop vite.", "FR_SUBJECT_VERB", "mange", "manges");
    expect(Fr, "Ils mangeons ensemble.", "FR_SUBJECT_VERB", "mangeons", "mangent");
    expect(Fr, "J'ai manger une pomme.", "FR_PARTICIPLE_INFINITIVE", "manger", "mangé");
    expect(Fr, "Je vais mangé.", "FR_PARTICIPLE_INFINITIVE", "mangé", "manger");
    expect(Fr, "Si j'aurais su, je serais venu.", "FR_SI_CONDITIONAL", "aurais", "avais");
    expect(Fr, "Les chat dorment.", "FR_DET_NOUN_AGREEMENT", "chat", "chats");
    expect(Fr, "Il a mangé les pomme.", "FR_DET_NOUN_AGREEMENT", "pomme", "pommes");
    expect(Fr, "Je pense aux porte.", "FR_DET_NOUN_AGREEMENT", "porte", "portes");
    expect(Fr, "Le arbre est grand.", "FR_ELISION", "Le arbre", "L'arbre");
    expect(Fr, "Je pars si il pleut.", "FR_ELISION", "si il", "s'il");
    expect(Fr, "Je parle de le livre.", "FR_CONTRACTION", "de le", "du");
    expect(Fr, "Ils son partis.", "FR_SON_SONT", "son", "sont");
    expect(Fr, "Ils on oublié.", "FR_ON_ONT", "on", "ont");
    expect(Fr, "Il ce lave.", "FR_CE_SE", "ce", "se");
    expect(Fr, "Sa va bien ?", "FR_CA_SA", "Sa", "Ça");
    expect(Fr, "Il c'est trompé.", "FR_CEST_SEST", "c'est", "s'est");
    expect(Fr, "Il peu venir.", "FR_PEU_PEUT", "peu", "peut");
    expect(Fr, "Quand à moi, je reste.", "FR_QUAND_QUANT", "Quand", "Quant");
    expect(Fr, "Le jour ou il est venu.", "FR_OU_OU", "ou", "où");
    expect(Fr, "Je leurs ai dit.", "FR_LEUR_LEURS", "leurs", "leur");
    expect(Fr, "Il faut pallier à ce problème.", "FR_COMMON_ERRORS", "pallier à", "pallier");
    expect(Fr, "Il faut monter en haut.", "FR_PLEONASMS", "monter en haut", "monter");
    expect(Fr, "Cela fait du sens.", "FR_ANGLICISMS", "fait du sens", "a du sens");
    expect(Fr, "Bonjour!", "FR_NBSP", "!", "\u{202f}!");
    expect(Fr, "Une grammaire sans fautes d'ortographe.", "SPELLING", "ortographe", "orthographe");
    expect(Fr, "il fait beau. elle dort.", "SENTENCE_CAPITAL", "elle", "Elle");
}

#[test]
fn english_errors() {
    use Language::En;
    expect(En, "He go to school.", "EN_SUBJECT_VERB", "go", "goes");
    expect(En, "They was happy.", "EN_SUBJECT_VERB", "was", "were");
    expect(En, "It don't matter.", "EN_SUBJECT_VERB", "don't", "doesn't");
    expect(En, "This is a apple.", "EN_A_AN", "a", "an");
    expect(En, "An university.", "EN_A_AN", "An", "A");
    expect(En, "I could of done it.", "EN_COMMON_ERRORS", "could of", "could have");
    expect(En, "i think so.", "EN_I_LOWERCASE", "i", "I");
    expect(En, "We left in order to win.", "EN_WORDY", "in order to", "to");
    expect(En, "The teh cat.", "SPELLING", "teh", "the");
}

fn assert_clean(lang: Language, corpus: &str) {
    for line in corpus.lines().filter(|l| !l.trim().is_empty()) {
        let r = check(line, &opts(lang));
        let bad: Vec<_> = r.matches.iter().filter(|m| m.severity != Severity::Hint).collect();
        assert!(bad.is_empty(), "false positives in {line:?}: {bad:#?}");
    }
}

#[test]
fn correct_french_text_has_no_errors() {
    assert_clean(Language::Fr, include_str!("data/fr_correct.txt"));
}

#[test]
fn correct_english_text_has_no_errors() {
    assert_clean(Language::En, include_str!("data/en_correct.txt"));
}

#[test]
fn user_dictionary_and_disabled_rules() {
    let mut o = opts(Language::Fr);
    assert!(check("Le zorglub arrive.", &o).matches.iter().any(|m| m.rule_id == "SPELLING"));
    o.user_words.insert("zorglub".into());
    assert!(check("Le zorglub arrive.", &o).matches.is_empty());
    o.disabled_rules.insert("FR_A_ACCENT".into());
    assert!(check("Il à faim.", &o).matches.is_empty());
}

#[test]
fn utf16_offsets_and_correction() {
    let text = "😀 Il à mangé, les chat dorment.";
    let r = check(text, &opts(Language::Fr));
    let m = r.matches.iter().find(|m| m.rule_id == "FR_A_ACCENT").unwrap();
    let utf16: Vec<u16> = text.encode_utf16().collect();
    assert_eq!(String::from_utf16(&utf16[m.offset..m.offset + m.length]).unwrap(), "à");

    let c = correct(text, &opts(Language::Fr), None);
    assert_eq!(c.corrected, "😀 Il a mangé, les chats dorment.");
}

#[test]
fn conjugation_api() {
    let c = libreplume_core::conjugate("finir", Language::Fr).unwrap();
    assert_eq!(c.group, Some(2));
    assert!(libreplume_core::conjugate("table", Language::Fr).is_err());
    let c = libreplume_core::conjugate("asservir", Language::Fr).unwrap();
    assert_eq!(c.moods[0].tenses[0].forms[3].form, "asservissons");
    let c = libreplume_core::conjugate("go", Language::En).unwrap();
    assert_eq!(c.participles[1].form, "gone");
}
