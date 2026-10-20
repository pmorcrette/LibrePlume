//! French grammar, conjugation and typography rules.

use super::{cased, Ctx, Found, RuleDef};
use crate::affix::class;
use crate::conjugation::fr::SimpleTense;
use crate::resources::{FormKind, FrenchResources};
use crate::text::{starts_with_vowel, TokenKind};
use crate::types::{Category, Language, Severity};

fn def(
    id: &'static str,
    category: Category,
    description: &'static str,
    default: bool,
    check: super::CheckFn,
) -> RuleDef {
    RuleDef { id, language: Some(Language::Fr), category, description, enabled_by_default: default, check }
}

pub fn rules() -> Vec<RuleDef> {
    vec![
        def(
            "FR_SUBJECT_VERB",
            Category::Conjugation,
            "Accord du verbe avec le pronom sujet (« tu mange » → « tu manges »)",
            true,
            subject_verb,
        ),
        def(
            "FR_PARTICIPLE_INFINITIVE",
            Category::Conjugation,
            "Confusion infinitif / participe passé (« j’ai manger », « pour mangé »)",
            true,
            participle_infinitive,
        ),
        def(
            "FR_SI_CONDITIONAL",
            Category::Conjugation,
            "Pas de conditionnel après « si » (« si j’aurais » → « si j’avais »)",
            true,
            si_conditional,
        ),
        def(
            "FR_DET_NOUN_AGREEMENT",
            Category::Grammar,
            "Accord en nombre du déterminant et du nom (« les chat » → « les chats »)",
            true,
            det_noun_agreement,
        ),
        def(
            "FR_ELISION",
            Category::Grammar,
            "Élision obligatoire (« le arbre » → « l’arbre », « si il » → « s’il »)",
            true,
            elision,
        ),
        def(
            "FR_CONTRACTION",
            Category::Grammar,
            "Articles contractés (« de le » → « du », « à les » → « aux »)",
            true,
            contraction,
        ),
        def("FR_A_ACCENT", Category::Grammar, "Confusion « a » / « à »", true, a_accent),
        def("FR_ET_EST", Category::Grammar, "Confusion « et » / « est »", true, et_est),
        def("FR_SON_SONT", Category::Grammar, "Confusion « son » / « sont »", true, son_sont),
        def("FR_ON_ONT", Category::Grammar, "Confusion « on » / « ont »", true, on_ont),
        def("FR_CE_SE", Category::Grammar, "Confusion « ce » / « se »", true, ce_se),
        def("FR_CA_SA", Category::Grammar, "Confusion « sa » / « ça »", true, ca_sa),
        def("FR_CEST_SEST", Category::Grammar, "Confusion « c’est » / « s’est »", true, cest_sest),
        def("FR_PEU_PEUT", Category::Grammar, "Confusion « peu » / « peut »", true, peu_peut),
        def("FR_QUAND_QUANT", Category::Grammar, "Confusion « quand » / « quant »", true, quand_quant),
        def("FR_OU_OU", Category::Grammar, "Confusion « ou » / « où »", true, ou_ou),
        def(
            "FR_LEUR_LEURS",
            Category::Grammar,
            "« leur » pronom est invariable (« je leurs dis » → « je leur dis »)",
            true,
            leur_leurs,
        ),
        def("FR_NBSP", Category::Typography, "Espace avant « ; : ! ? » en français", true, nbsp_missing),
        def(
            "FR_NBSP_STRICT",
            Category::Typography,
            "Espace insécable (et fine) avant « ; : ! ? » et dans les guillemets",
            false,
            nbsp_strict,
        ),
        def("FR_GUILLEMETS", Category::Typography, "Guillemets français « … » plutôt que \"…\"", true, guillemets),
    ]
}

fn res(ctx: &Ctx<'_>) -> &'static FrenchResources {
    ctx.fr.expect("French resources")
}

/// Lowercased word with straight apostrophes.
fn norm(ctx: &Ctx<'_>, i: usize) -> String {
    ctx.tokens[i].lower.replace('’', "'")
}

fn subject_person(w: &str) -> Option<usize> {
    Some(match w {
        "je" | "j'" => 0,
        "tu" => 1,
        "il" | "elle" | "on" => 2,
        "nous" => 3,
        "vous" => 4,
        "ils" | "elles" => 5,
        _ => return None,
    })
}

/// Pronouns that may also be objects or stressed forms and need a clear subject position.
fn ambiguous_subject(w: &str) -> bool {
    matches!(w, "tu" | "elle" | "elles" | "nous" | "vous")
}

/// Words after which a new clause (and thus a subject) can start.
const CLAUSE_START: &[&str] = &[
    "et",
    "mais",
    "ou",
    "donc",
    "or",
    "ni",
    "car",
    "que",
    "qu'",
    "quand",
    "lorsque",
    "lorsqu'",
    "si",
    "s'",
    "puisque",
    "puisqu'",
    "comme",
    "où",
    "alors",
    "puis",
    "ensuite",
    "parce",
    "bien",
    "afin",
    "pendant",
    "tandis",
    "dès",
    "aussitôt",
    "sinon",
    "enfin",
    "aujourd'hui",
    "demain",
    "hier",
    "maintenant",
    "ainsi",
    "pourtant",
    "cependant",
    "quoique",
    "quoiqu'",
];

/// Object/negation clitics that can sit between the subject and the verb.
const CLITICS: &[&str] = &[
    "ne", "n'", "me", "m'", "te", "t'", "se", "s'", "le", "la", "l'", "les", "lui", "leur", "nous", "vous", "y", "en",
];

fn is_clause_start(ctx: &Ctx<'_>, chunk: &[usize], k: usize) -> bool {
    k == 0 || CLAUSE_START.contains(&norm(ctx, chunk[k - 1]).as_str())
}

fn finite_hits(fr: &FrenchResources, word: &str) -> Vec<(u32, SimpleTense, usize)> {
    fr.verbs
        .lookup(word)
        .into_iter()
        .filter_map(|h| match h.kind {
            FormKind::Finite(t, p) => Some((h.verb, t, p as usize)),
            _ => None,
        })
        .collect()
}

fn person_label(p: usize) -> &'static str {
    ["je", "tu", "il/elle/on", "nous", "vous", "ils/elles"][p]
}

fn subject_verb(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        let mut k = 0;
        while k < chunk.len() {
            let w = norm(ctx, chunk[k]);
            let Some(person) = subject_person(&w) else {
                k += 1;
                continue;
            };
            if ambiguous_subject(&w) && !is_clause_start(ctx, chunk, k) {
                k += 1;
                continue;
            }
            let mut j = k + 1;
            while j < chunk.len() && CLITICS.contains(&norm(ctx, chunk[j]).as_str()) {
                j += 1;
            }
            if j >= chunk.len() {
                break;
            }
            let subject = ctx.tok(chunk[k]).text;
            let v_tok = ctx.tok(chunk[j]);
            let v = norm(ctx, chunk[j]);
            k = j;
            if v.contains('-') || v_tok.starts_upper() && !ctx.starts_sentence(chunk[j]) {
                continue;
            }
            let hits = finite_hits(fr, &v);
            if hits.is_empty() || hits.iter().any(|&(_, _, p)| p == person) {
                continue;
            }
            // Only flag words that are verbs and nothing else ("tu chat" is a different problem).
            if fr.lexicon.contains(&v) && !fr.verbs.lookup(&v).is_empty() && fr.lexicon.is_nominal(&v) && hits.len() < 2
            {
                continue;
            }
            let mut suggestions: Vec<String> = Vec::new();
            for &(verb, tense, _) in &hits {
                if let Some(forms) = fr.verbs.simple_forms(verb) {
                    if let Some(f) = forms.get(tense, person) {
                        let s = crate::text::match_case(v_tok.text, f);
                        if !suggestions.contains(&s) {
                            suggestions.push(s);
                        }
                    }
                }
            }
            if suggestions.is_empty() {
                continue;
            }
            suggestions.truncate(4);
            let infinitive = fr.verbs.infinitive(hits[0].0);
            out.push(Found::new(
                Severity::Error,
                format!(
                    "Le verbe « {infinitive} » ne s’accorde pas avec le sujet « {} » ({}). Forme attendue : « {} ».",
                    subject.trim_end_matches(['\'', '’']),
                    person_label(person),
                    suggestions[0]
                ),
                v_tok.start..v_tok.end,
                suggestions,
            ));
        }
    }
}

const AUX_FORMS: &[&str] = &[
    "ai", "as", "a", "avons", "avez", "ont", "avais", "avait", "avions", "aviez", "avaient", "aurai", "auras", "aura",
    "aurons", "aurez", "auront", "aurais", "aurait", "aurions", "auriez", "auraient", "aie", "aies", "ait", "ayons",
    "ayez", "aient", "eus", "eut", "eûmes", "eurent", "avoir", "ayant", "suis", "es", "est", "sommes", "êtes", "sont",
    "étais", "était", "étions", "étiez", "étaient", "serai", "seras", "sera", "serons", "serez", "seront", "serais",
    "serait", "serions", "seriez", "seraient", "sois", "soit", "soyons", "soyez", "soient", "fus", "fut", "furent",
    "être", "étant",
];

const ADVERBS_BETWEEN: &[&str] = &[
    "pas",
    "jamais",
    "plus",
    "rien",
    "bien",
    "déjà",
    "beaucoup",
    "toujours",
    "souvent",
    "trop",
    "tout",
    "encore",
    "peu",
    "vraiment",
    "mal",
    "enfin",
    "aussi",
    "également",
    "point",
    "guère",
    "presque",
    "longtemps",
    "assez",
    "même",
    "sûrement",
    "certainement",
];

const MODALS: &[&str] = &[
    "vais",
    "vas",
    "va",
    "allons",
    "allez",
    "vont",
    "allais",
    "allait",
    "allions",
    "alliez",
    "allaient",
    "peux",
    "peut",
    "pouvons",
    "pouvez",
    "peuvent",
    "pouvais",
    "pouvait",
    "pourrait",
    "pourrais",
    "pourraient",
    "pourrons",
    "pourrez",
    "dois",
    "doit",
    "devons",
    "devez",
    "doivent",
    "devais",
    "devait",
    "devrait",
    "devrais",
    "devraient",
    "veux",
    "veut",
    "voulons",
    "voulez",
    "veulent",
    "voulais",
    "voulait",
    "voudrais",
    "voudrait",
    "faut",
    "fallait",
    "faudra",
    "faudrait",
    "sais",
    "sait",
    "savons",
    "savez",
    "savent",
    "aller",
    "pouvoir",
    "devoir",
    "vouloir",
    "souhaite",
    "souhaites",
    "souhaitent",
    "aime",
    "aimes",
    "aiment",
    "préfère",
    "préfères",
    "préfèrent",
    "ose",
    "oses",
    "osent",
];

const PREPOSITIONS_INF: &[&str] = &["pour", "de", "d'", "sans", "à", "afin"];

fn participle_infinitive(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        for k in 0..chunk.len() {
            let w = norm(ctx, chunk[k]);
            let is_aux = AUX_FORMS.contains(&w.as_str());
            let after_subject =
                k > 0 && matches!(norm(ctx, chunk[k - 1]).as_str(), "il" | "elle" | "on" | "ça" | "cela" | "y");
            let is_trigger =
                MODALS.contains(&w.as_str()) || PREPOSITIONS_INF.contains(&w.as_str()) && !(w == "à" && after_subject);
            if !is_aux && !is_trigger {
                continue;
            }
            let mut j = k + 1;
            if is_aux {
                while j < chunk.len() && ADVERBS_BETWEEN.contains(&norm(ctx, chunk[j]).as_str()) {
                    j += 1;
                }
            } else {
                while j < chunk.len() {
                    let n = norm(ctx, chunk[j]);
                    if CLITICS.contains(&n.as_str()) || matches!(n.as_str(), "pas" | "plus" | "jamais" | "rien") {
                        j += 1;
                    } else {
                        break;
                    }
                }
            }
            if j >= chunk.len() {
                continue;
            }
            let v_tok = ctx.tok(chunk[j]);
            let v = norm(ctx, chunk[j]);
            let hits = fr.verbs.lookup(&v);
            if is_aux {
                // "j'ai manger" → "j'ai mangé"
                if !v.ends_with("er") || !hits.iter().any(|h| h.kind == FormKind::Infinitive) {
                    continue;
                }
                let pp = format!("{}é", &v[..v.len() - 2]);
                let pp = crate::text::match_case(v_tok.text, &pp);
                if w == "a" && !is_subject_before(ctx, chunk, k) {
                    // "machine a laver" is more likely « à laver ».
                    let aux_tok = ctx.tok(chunk[k]);
                    out.push(Found::new(
                        Severity::Error,
                        "Confusion possible : participe passé après l’auxiliaire (« a mangé ») ou préposition « à » suivie de l’infinitif (« à laver »).",
                        aux_tok.start..v_tok.end,
                        vec![format!("{} {pp}", aux_tok.text), format!("{} {}", cased(ctx, chunk[k], "à"), v_tok.text)],
                    ));
                } else {
                    out.push(Found::new(
                        Severity::Error,
                        format!(
                            "Après l’auxiliaire « {} », on attend le participe passé « {pp} », pas l’infinitif.",
                            ctx.tok(chunk[k]).text
                        ),
                        v_tok.start..v_tok.end,
                        vec![pp],
                    ));
                }
            } else {
                // "pour mangé", "je vais mangé" → infinitive.
                if !(v.ends_with('é') || v.ends_with("ée") || v.ends_with("és") || v.ends_with("ées")) {
                    continue;
                }
                if fr.lexicon.contains(&v) {
                    continue;
                }
                let Some(inf) = hits.iter().find_map(|h| match h.kind {
                    FormKind::PastParticiple(_) => {
                        let inf = fr.verbs.infinitive(h.verb);
                        inf.ends_with("er").then(|| inf.to_string())
                    }
                    _ => None,
                }) else {
                    continue;
                };
                if w == "afin" && j != k + 2 {
                    continue;
                }
                let inf = crate::text::match_case(v_tok.text, &inf);
                out.push(Found::new(
                    Severity::Error,
                    format!(
                        "Après « {} », on attend l’infinitif « {inf} », pas le participe passé.",
                        ctx.tok(chunk[k]).text
                    ),
                    v_tok.start..v_tok.end,
                    vec![inf],
                ));
            }
        }
    }
}

fn is_subject_before(ctx: &Ctx<'_>, chunk: &[usize], k: usize) -> bool {
    let mut i = k;
    while i > 0 {
        i -= 1;
        let w = norm(ctx, chunk[i]);
        if CLITICS.contains(&w.as_str()) {
            continue;
        }
        return subject_person(&w).is_some() || matches!(w.as_str(), "qui" | "ça" | "cela" | "ceci" | "on");
    }
    false
}

fn si_conditional(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        for k in 0..chunk.len() {
            let w = norm(ctx, chunk[k]);
            if w != "si" && w != "s'" {
                continue;
            }
            if k > 0
                && !matches!(
                    norm(ctx, chunk[k - 1]).as_str(),
                    "même" | "et" | "mais" | "ou" | "car" | "donc" | "alors" | "sauf" | "comme"
                )
            {
                continue;
            }
            let Some(&subj) = chunk.get(k + 1) else { continue };
            let Some(person) = subject_person(&norm(ctx, subj)) else { continue };
            let mut j = k + 2;
            while j < chunk.len() && CLITICS.contains(&norm(ctx, chunk[j]).as_str()) {
                j += 1;
            }
            let Some(&vi) = chunk.get(j) else { continue };
            let v_tok = ctx.tok(vi);
            let hits = finite_hits(fr, &norm(ctx, vi));
            let Some(&(verb, _, _)) = hits.iter().find(|&&(_, t, p)| t == SimpleTense::CondPresent && p == person)
            else {
                continue;
            };
            if hits.iter().any(|&(_, t, _)| t != SimpleTense::CondPresent) {
                continue;
            }
            let Some(imp) =
                fr.verbs.simple_forms(verb).and_then(|f| f.get(SimpleTense::IndImparfait, person).map(str::to_string))
            else {
                continue;
            };
            out.push(Found::new(
                Severity::Error,
                format!(
                    "Après « si » exprimant une condition, on emploie l’imparfait (« {imp} ») et non le conditionnel."
                ),
                v_tok.start..v_tok.end,
                vec![crate::text::match_case(v_tok.text, &imp)],
            ));
        }
    }
}

const PLURAL_DETS: &[&str] = &[
    "les",
    "des",
    "ces",
    "mes",
    "tes",
    "ses",
    "nos",
    "vos",
    "leurs",
    "plusieurs",
    "quelques",
    "certains",
    "certaines",
    "aux",
    "divers",
    "diverses",
    "différents",
    "différentes",
    "deux",
    "trois",
    "quatre",
    "cinq",
    "six",
    "sept",
    "huit",
    "neuf",
    "dix",
    "douze",
    "quinze",
    "vingt",
    "trente",
    "quarante",
    "cinquante",
    "soixante",
    "maints",
    "maintes",
    "nombreux",
    "nombreuses",
];

const SINGULAR_DETS: &[&str] = &[
    "un", "une", "le", "la", "l'", "ce", "cet", "cette", "mon", "ton", "son", "ma", "ta", "sa", "notre", "votre",
    "leur", "chaque", "du", "au", "quel", "quelle",
];

/// Determiners that are also object pronouns ("je les vois", "il le prend").
const DET_PRONOUNS: &[&str] = &["les", "le", "la", "l'", "leur"];

const SUBJECTS_AND_CLITICS: &[&str] = &[
    "je", "j'", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles", "ne", "n'", "me", "m'", "te", "t'", "se",
    "s'", "qui", "ça", "cela", "à", "de", "d'", "pour", "sans", "faut",
];

const PREPOSITIONS: &[&str] = &[
    "dans",
    "sur",
    "sous",
    "avec",
    "pour",
    "par",
    "chez",
    "vers",
    "entre",
    "contre",
    "devant",
    "derrière",
    "après",
    "avant",
    "depuis",
    "pendant",
    "selon",
    "sans",
    "de",
    "à",
    "en",
    "malgré",
    "parmi",
    "durant",
];

const NUMERAL_WORDS: &[&str] = &["cent", "mille", "vingt", "quatre", "un", "une", "deux", "trois"];

fn pluralize(fr: &FrenchResources, w: &str) -> Option<String> {
    let mut candidates = vec![format!("{w}s"), format!("{w}x")];
    if let Some(stem) = w.strip_suffix("al") {
        candidates.insert(0, format!("{stem}aux"));
    }
    if let Some(stem) = w.strip_suffix("ail") {
        candidates.push(format!("{stem}aux"));
    }
    candidates.into_iter().find(|c| fr.lexicon.class(c) & class::PLURAL != 0)
}

fn singularize(fr: &FrenchResources, w: &str) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(stem) = w.strip_suffix("aux") {
        candidates.push(format!("{stem}al"));
        candidates.push(format!("{stem}ail"));
    }
    if let Some(stem) = w.strip_suffix('s').or_else(|| w.strip_suffix('x')) {
        candidates.push(stem.to_string());
    }
    candidates.into_iter().find(|c| fr.lexicon.class(c) & class::SINGULAR != 0)
}

fn det_noun_agreement(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            let det = norm(ctx, chunk[k]);
            let plural_det = PLURAL_DETS.contains(&det.as_str());
            let singular_det = SINGULAR_DETS.contains(&det.as_str());
            if !plural_det && !singular_det {
                continue;
            }
            let prev = (k > 0).then(|| norm(ctx, chunk[k - 1]));
            if prev.as_deref().is_some_and(|p| subject_person(p).is_some())
                || DET_PRONOUNS.contains(&det.as_str())
                    && prev.as_deref().is_some_and(|p| SUBJECTS_AND_CLITICS.contains(&p))
            {
                continue;
            }
            let n_tok = ctx.tok(chunk[k + 1]);
            if n_tok.starts_upper() && !ctx.starts_sentence(chunk[k + 1]) || n_tok.text.contains('-') {
                continue;
            }
            let n = norm(ctx, chunk[k + 1]);
            if NUMERAL_WORDS.contains(&n.as_str()) {
                continue;
            }
            let c = fr.lexicon.class(&n);
            if c & class::PROPER != 0 || c & (class::SINGULAR | class::PLURAL) == 0 {
                continue;
            }
            // "le/la/les" may be object pronouns: "Pierre les porte", "il veut les manger".
            if DET_PRONOUNS.contains(&det.as_str()) {
                let hits = fr.verbs.lookup(&n);
                if hits.iter().any(|h| h.kind == FormKind::Infinitive) {
                    continue;
                }
                if !hits.is_empty() {
                    // As a determiner, it follows a verb, a preposition or starts the group.
                    let after_verb_or_prep = prev.as_deref().is_none_or(|p| {
                        PREPOSITIONS.contains(&p) || fr.verbs.is_verb_form(p) && !fr.lexicon.is_nominal(p)
                    });
                    if !after_verb_or_prep {
                        continue;
                    }
                }
            }
            if plural_det && c & class::PLURAL == 0 {
                let Some(p) = pluralize(fr, &n) else { continue };
                let p = crate::text::match_case(n_tok.text, &p);
                out.push(Found::new(
                    Severity::Warning,
                    format!(
                        "Après le déterminant pluriel « {} », le nom devrait être au pluriel : « {p} ».",
                        ctx.tok(chunk[k]).text
                    ),
                    n_tok.start..n_tok.end,
                    vec![p],
                ));
            } else if singular_det && c & class::SINGULAR == 0 {
                let Some(s) = singularize(fr, &n) else { continue };
                let s = crate::text::match_case(n_tok.text, &s);
                out.push(Found::new(
                    Severity::Warning,
                    format!(
                        "Après le déterminant singulier « {} », le nom devrait être au singulier : « {s} ».",
                        ctx.tok(chunk[k]).text
                    ),
                    n_tok.start..n_tok.end,
                    vec![s],
                ));
            }
        }
    }
}

/// Words starting with a mute h (elision required) — most frequent ones.
const H_MUET: &[&str] = &[
    "homme",
    "hommes",
    "heure",
    "heures",
    "hôtel",
    "hôtels",
    "histoire",
    "histoires",
    "habitude",
    "habitudes",
    "herbe",
    "hiver",
    "hôpital",
    "huile",
    "humain",
    "humains",
    "honneur",
    "hôte",
    "harmonie",
    "héritage",
    "hélicoptère",
    "hypothèse",
    "habit",
    "habitant",
    "habitants",
    "hirondelle",
    "horizon",
    "horloge",
    "horreur",
    "humeur",
    "humour",
    "hygiène",
    "habitation",
    "hébergement",
    "héroïne",
    "hésitation",
    "homme-orchestre",
    "honnêteté",
    "hommage",
    "horaire",
    "horaires",
    "hospitalité",
    "humanité",
    "humidité",
    "humilité",
    "hydrogène",
    "hymne",
    "habiter",
    "habiller",
    "hésiter",
    "honorer",
    "hériter",
    "habite",
    "habitent",
    "hésite",
    "hérite",
    "habille",
    "humilier",
];

/// Vowel-initial words that do not take elision.
const NO_ELISION: &[&str] =
    &["onze", "onzième", "oui", "ouate", "ouistiti", "uhlan", "un", "une", "ululement", "yacht", "yaourt", "yoga"];

fn elision(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let ap = ctx.apostrophe();
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            let (a, b) = (ctx.tok(chunk[k]), ctx.tok(chunk[k + 1]));
            if a.end == b.start {
                continue;
            }
            let w = norm(ctx, chunk[k]);
            let next = norm(ctx, chunk[k + 1]);
            if NO_ELISION.contains(&next.as_str()) || next.starts_with("oua") {
                continue;
            }
            let vowel = starts_with_vowel(&next) || H_MUET.contains(&next.as_str());
            if !vowel {
                continue;
            }
            let replacement = match w.as_str() {
                "le" | "la" | "de" | "je" | "me" | "te" | "se" | "ne" => {
                    format!("{}{ap}{}", &a.text[..a.text.len() - 1], b.text)
                }
                "que" | "jusque" | "lorsque" | "puisque" | "quoique" => {
                    format!("{}{ap}{}", &a.text[..a.text.len() - 1], b.text)
                }
                "si" if matches!(next.as_str(), "il" | "ils") => format!("{}{ap}{}", &a.text[..1], b.text),
                "ce" if matches!(next.as_str(), "est" | "était" | "étaient" | "en") => {
                    format!("{}{ap}{}", &a.text[..1], b.text)
                }
                "ce" => {
                    let fr = res(ctx);
                    if !fr.lexicon.is_nominal(&next) || fr.verbs.is_verb_form(&next) {
                        continue;
                    }
                    let cet = cased(ctx, chunk[k], "cet");
                    let cette = cased(ctx, chunk[k], "cette");
                    out.push(Found::new(
                        Severity::Error,
                        "Devant une voyelle, « ce » devient « cet » (masculin) ou « cette » (féminin).",
                        a.start..b.end,
                        vec![format!("{cet} {}", b.text), format!("{cette} {}", b.text)],
                    ));
                    continue;
                }
                _ => continue,
            };
            out.push(Found::new(
                Severity::Error,
                format!("Élision obligatoire devant une voyelle ou un h muet : « {replacement} »."),
                a.start..b.end,
                vec![replacement],
            ));
        }
    }
}

fn contraction(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(2) {
            let a = norm(ctx, chunk[k]);
            let b = norm(ctx, chunk[k + 1]);
            let repl = match (a.as_str(), b.as_str()) {
                ("de", "le") => "du",
                ("de", "les") => "des",
                ("à", "le") => "au",
                ("à", "les") => "aux",
                _ => continue,
            };
            let next = norm(ctx, chunk[k + 2]);
            // "de le faire": here "le" is a pronoun before an infinitive.
            let infinitive = fr.verbs.lookup(&next).iter().any(|h| h.kind == FormKind::Infinitive);
            if !fr.lexicon.is_nominal(&next) || infinitive {
                continue;
            }
            let r = cased(ctx, chunk[k], repl);
            out.push(Found::new(
                Severity::Error,
                format!("« {} {} » se contracte en « {r} ».", ctx.tok(chunk[k]).text, ctx.tok(chunk[k + 1]).text),
                ctx.span(chunk[k], chunk[k + 1]),
                vec![r],
            ));
        }
    }
}

/// Generic helper: `prev` (one of `before`) followed by `word` → `replacement`.
fn pair_rule(
    ctx: &Ctx<'_>,
    out: &mut Vec<Found>,
    before: &[&str],
    word: &str,
    replacement: &str,
    message: &str,
    extra: impl Fn(&Ctx<'_>, &[usize], usize) -> bool,
) {
    for chunk in &ctx.chunks {
        for k in 1..chunk.len() {
            if norm(ctx, chunk[k]) != word || !before.contains(&norm(ctx, chunk[k - 1]).as_str()) {
                continue;
            }
            if !extra(ctx, chunk, k) {
                continue;
            }
            let t = ctx.tok(chunk[k]);
            out.push(Found::new(Severity::Error, message, t.start..t.end, vec![cased(ctx, chunk[k], replacement)]));
        }
    }
}

fn always(_: &Ctx<'_>, _: &[usize], _: usize) -> bool {
    true
}

fn a_accent(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    pair_rule(
        ctx,
        out,
        &["il", "elle", "on", "ça", "cela"],
        "à",
        "a",
        "Après un sujet, il faut le verbe avoir « a » (sans accent).",
        always,
    );
    pair_rule(ctx, out, &["y"], "à", "a", "« il y a » s’écrit sans accent.", |ctx, chunk, k| {
        k >= 2 && norm(ctx, chunk[k - 2]) == "il"
    });
}

fn et_est(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    pair_rule(
        ctx,
        out,
        &["il", "elle", "on", "c'", "ce", "ça", "cela"],
        "et",
        "est",
        "Confusion probable entre « et » (conjonction) et « est » (verbe être).",
        |ctx, chunk, k| {
            // "elle et moi" is fine.
            chunk.get(k + 1).is_none_or(|&n| {
                !matches!(norm(ctx, n).as_str(), "moi" | "toi" | "lui" | "elle" | "eux" | "nous" | "vous")
            })
        },
    );
}

fn son_sont(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    pair_rule(
        ctx,
        out,
        &["ils", "elles", "ne", "n'"],
        "son",
        "sont",
        "Confusion probable entre « son » (déterminant) et « sont » (verbe être).",
        |ctx, chunk, k| {
            norm(ctx, chunk[k - 1]).starts_with('n')
                && k >= 2
                && matches!(norm(ctx, chunk[k - 2]).as_str(), "ils" | "elles")
                || matches!(norm(ctx, chunk[k - 1]).as_str(), "ils" | "elles")
        },
    );
}

fn on_ont(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    pair_rule(
        ctx,
        out,
        &["ils", "elles", "n'"],
        "on",
        "ont",
        "Confusion probable entre « on » (pronom) et « ont » (verbe avoir).",
        |ctx, chunk, k| {
            matches!(norm(ctx, chunk[k - 1]).as_str(), "ils" | "elles")
                || k >= 2 && matches!(norm(ctx, chunk[k - 2]).as_str(), "ils" | "elles")
        },
    );
}

fn ce_se(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    pair_rule(
        ctx,
        out,
        &["il", "elle", "on", "ils", "elles", "ne"],
        "ce",
        "se",
        "Devant un verbe pronominal, on écrit « se » (pronom), pas « ce » (démonstratif).",
        |ctx, chunk, k| {
            let Some(&n) = chunk.get(k + 1) else { return false };
            !finite_hits(fr, &norm(ctx, n)).is_empty()
        },
    );
}

fn ca_sa(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            if norm(ctx, chunk[k]) != "sa" {
                continue;
            }
            let next = norm(ctx, chunk[k + 1]);
            let hits = finite_hits(fr, &next);
            if fr.lexicon.is_nominal(&next) || !hits.iter().any(|&(_, _, p)| p == 2) {
                continue;
            }
            let t = ctx.tok(chunk[k]);
            out.push(Found::new(
                Severity::Error,
                "Devant un verbe, on écrit « ça » (pronom), pas « sa » (déterminant).",
                t.start..t.end,
                vec![cased(ctx, chunk[k], "ça")],
            ));
        }
    }
}

fn cest_sest(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for chunk in &ctx.chunks {
        for k in 1..chunk.len().saturating_sub(1) {
            if norm(ctx, chunk[k]) != "c'" || norm(ctx, chunk[k + 1]) != "est" {
                continue;
            }
            if !matches!(norm(ctx, chunk[k - 1]).as_str(), "il" | "elle" | "on" | "qui") {
                continue;
            }
            let (a, b) = (ctx.tok(chunk[k]), ctx.tok(chunk[k + 1]));
            let ap = a.text.chars().last().unwrap_or('\'');
            out.push(Found::new(
                Severity::Error,
                "Après un sujet, il s’agit du verbe pronominal : « s’est ».",
                a.start..b.end,
                vec![format!("{}{ap}{}", cased(ctx, chunk[k], "s"), b.text)],
            ));
        }
    }
}

fn peu_peut(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    pair_rule(
        ctx,
        out,
        &["il", "elle", "on", "ça", "cela", "ne", "n'"],
        "peu",
        "peut",
        "Confusion probable : « peut » (verbe pouvoir) et non « peu » (adverbe).",
        |ctx, chunk, k| {
            let prev = norm(ctx, chunk[k - 1]);
            !prev.starts_with('n')
                || k >= 2 && matches!(norm(ctx, chunk[k - 2]).as_str(), "il" | "elle" | "on" | "ça" | "cela")
        },
    );
    pair_rule(
        ctx,
        out,
        &["un", "trop", "très", "si", "à"],
        "peut",
        "peu",
        "Confusion probable : « peu » (adverbe) et non « peut » (verbe pouvoir).",
        |ctx, chunk, k| {
            let prev = norm(ctx, chunk[k - 1]);
            prev != "à" || k >= 2 && norm(ctx, chunk[k - 2]) == "peu"
        },
    );
}

fn quand_quant(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for chunk in &ctx.chunks {
        for k in 0..chunk.len() {
            let w = norm(ctx, chunk[k]);
            let next = chunk.get(k + 1).map(|&n| norm(ctx, n));
            let before_a = next.as_deref().is_some_and(|n| matches!(n, "à" | "au" | "aux"));
            let t = ctx.tok(chunk[k]);
            if w == "quand" && before_a {
                out.push(Found::new(
                    Severity::Error,
                    "« Quant à » (en ce qui concerne) s’écrit avec un t.",
                    t.start..t.end,
                    vec![cased(ctx, chunk[k], "quant")],
                ));
            } else if w == "quant" && !before_a && next.is_some() {
                out.push(Found::new(
                    Severity::Error,
                    "« Quand » (conjonction de temps) s’écrit avec un d.",
                    t.start..t.end,
                    vec![cased(ctx, chunk[k], "quand")],
                ));
            }
        }
    }
}

fn ou_ou(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    const TIME_PLACE: &[&str] = &[
        "jour", "moment", "endroit", "instant", "époque", "année", "lieu", "ville", "pays", "période", "matin", "soir",
        "nuit", "minute", "seconde", "pièce", "maison",
    ];
    for chunk in &ctx.chunks {
        for k in 0..chunk.len() {
            if norm(ctx, chunk[k]) != "ou" {
                continue;
            }
            let prev = (k > 0).then(|| norm(ctx, chunk[k - 1]));
            let next = chunk.get(k + 1).map(|&n| norm(ctx, n));
            let after_d = prev.as_deref() == Some("d'");
            let relative = prev.as_deref().is_some_and(|p| TIME_PLACE.contains(&p))
                && next.as_deref().is_some_and(|n| subject_person(n).is_some());
            if after_d || relative {
                let t = ctx.tok(chunk[k]);
                out.push(Found::new(
                    Severity::Error,
                    "« Où » (lieu, temps) prend un accent grave ; « ou » sans accent signifie « ou bien ».",
                    t.start..t.end,
                    vec![cased(ctx, chunk[k], "où")],
                ));
            }
        }
    }
}

fn leur_leurs(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let fr = res(ctx);
    for chunk in &ctx.chunks {
        for k in 0..chunk.len().saturating_sub(1) {
            if norm(ctx, chunk[k]) != "leurs" {
                continue;
            }
            let next = norm(ctx, chunk[k + 1]);
            if !fr.is_verb_only(&next) || finite_hits(fr, &next).is_empty() {
                continue;
            }
            let t = ctx.tok(chunk[k]);
            out.push(Found::new(
                Severity::Error,
                "Devant un verbe, « leur » est un pronom personnel invariable.",
                t.start..t.end,
                vec![cased(ctx, chunk[k], "leur")],
            ));
        }
    }
}

const NNBSP: char = '\u{202F}';
const NBSP: char = '\u{00A0}';

fn nbsp_missing(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for (i, t) in ctx.tokens.iter().enumerate() {
        if t.kind != TokenKind::Punct || !matches!(t.text, ";" | ":" | "!" | "?") || i == 0 {
            continue;
        }
        let prev = &ctx.tokens[i - 1];
        let attached = matches!(prev.kind, TokenKind::Word | TokenKind::Number)
            || prev.kind == TokenKind::Punct && matches!(prev.text, "»" | ")" | "\"" | "”");
        if !attached {
            continue;
        }
        let next = ctx.tokens.get(i + 1);
        // Times (12:30), smileys (:-) ;)), paths and code are left alone.
        let next_ok = next.is_none_or(|n| {
            matches!(n.kind, TokenKind::Space | TokenKind::Newline)
                || n.kind == TokenKind::Punct && matches!(n.text, "!" | "?" | "»" | "\"" | "”" | ")" | "…")
        });
        if !next_ok {
            continue;
        }
        let space = if t.text == ":" { NBSP } else { NNBSP };
        out.push(Found::new(
            Severity::Warning,
            format!("En français, « {} » est précédé d’une espace (insécable).", t.text),
            t.start..t.end,
            vec![format!("{space}{}", t.text)],
        ));
    }
}

fn nbsp_strict(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    for i in 1..ctx.tokens.len() {
        let (sp, t) = (&ctx.tokens[i - 1], &ctx.tokens[i]);
        if sp.kind == TokenKind::Space
            && sp.text == " "
            && t.kind == TokenKind::Punct
            && matches!(t.text, ";" | ":" | "!" | "?" | "»")
        {
            let space = if matches!(t.text, ":" | "»") { NBSP } else { NNBSP };
            out.push(Found::new(
                Severity::Hint,
                "Utilisez une espace insécable pour éviter un retour à la ligne avant la ponctuation.",
                sp.start..sp.end,
                vec![space.to_string()],
            ));
        }
        if t.kind == TokenKind::Space && t.text == " " && sp.kind == TokenKind::Punct && sp.text == "«" {
            out.push(Found::new(
                Severity::Hint,
                "Utilisez une espace insécable après « « ».",
                t.start..t.end,
                vec![NBSP.to_string()],
            ));
        }
    }
}

fn guillemets(ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let mut open = true;
    for (i, t) in ctx.tokens.iter().enumerate() {
        if t.kind == TokenKind::Newline {
            open = true;
        }
        if t.kind != TokenKind::Punct || t.text != "\"" {
            continue;
        }
        // Decide orientation from the surroundings rather than only alternating.
        let prev_space = i == 0 || matches!(ctx.tokens[i - 1].kind, TokenKind::Space | TokenKind::Newline);
        let next_space = ctx
            .tokens
            .get(i + 1)
            .is_none_or(|n| matches!(n.kind, TokenKind::Space | TokenKind::Newline | TokenKind::Punct));
        let opening = if prev_space && !next_space {
            true
        } else if next_space && !prev_space {
            false
        } else {
            open
        };
        open = !opening;
        let (start, end, repl) = if opening {
            let end = match ctx.tokens.get(i + 1) {
                Some(n) if n.kind == TokenKind::Space => n.end,
                _ => t.end,
            };
            (t.start, end, format!("«{NBSP}"))
        } else {
            let start =
                if i > 0 && ctx.tokens[i - 1].kind == TokenKind::Space { ctx.tokens[i - 1].start } else { t.start };
            (start, t.end, format!("{NBSP}»"))
        };
        out.push(Found::new(Severity::Hint, "En français, on utilise les guillemets « … ».", start..end, vec![repl]));
    }
}
