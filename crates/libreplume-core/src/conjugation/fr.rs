//! French conjugation engine.
//!
//! Regular verbs (1st and 2nd group) are generated from rules, including the usual
//! spelling alternations (ç/ge, è, -eler/-eter, -yer). Third-group verbs follow a
//! model ("venir", "prendre", "peindre"…) described by a few principal parts from
//! which every simple tense is derived, in the spirit of the Bescherelle tables.

use super::{Conjugation, ConjugationError, Mood, NamedForm, PersonForm, Tense};
use crate::text::starts_with_vowel;
use crate::types::Language;

pub const PRONOUNS: [&str; 6] = ["je", "tu", "il/elle", "nous", "vous", "ils/elles"];
const REFLEXIVE: [&str; 6] = ["me", "te", "se", "nous", "vous", "se"];
const IMPERATIVE_PERSONS: [usize; 3] = [1, 3, 4];

/// Simple (non-compound) tenses, indexing [`SimpleForms::tenses`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum SimpleTense {
    IndPresent = 0,
    IndImparfait = 1,
    IndPasseSimple = 2,
    IndFutur = 3,
    CondPresent = 4,
    SubjPresent = 5,
    SubjImparfait = 6,
}

impl SimpleTense {
    pub const ALL: [SimpleTense; 7] = [
        SimpleTense::IndPresent,
        SimpleTense::IndImparfait,
        SimpleTense::IndPasseSimple,
        SimpleTense::IndFutur,
        SimpleTense::CondPresent,
        SimpleTense::SubjPresent,
        SimpleTense::SubjImparfait,
    ];

    pub fn from_index(i: u8) -> Option<SimpleTense> {
        SimpleTense::ALL.get(i as usize).copied()
    }

    pub fn label(self) -> &'static str {
        match self {
            SimpleTense::IndPresent => "présent de l’indicatif",
            SimpleTense::IndImparfait => "imparfait de l’indicatif",
            SimpleTense::IndPasseSimple => "passé simple",
            SimpleTense::IndFutur => "futur simple",
            SimpleTense::CondPresent => "conditionnel présent",
            SimpleTense::SubjPresent => "subjonctif présent",
            SimpleTense::SubjImparfait => "subjonctif imparfait",
        }
    }
}

/// Every simple form of a verb.
#[derive(Debug, Clone)]
pub struct SimpleForms {
    pub infinitive: String,
    pub group: u8,
    pub model: &'static str,
    pub tenses: [[Option<String>; 6]; 7],
    /// 2nd singular, 1st plural, 2nd plural.
    pub imperative: [Option<String>; 3],
    pub present_participle: Option<String>,
    /// Masculine singular, feminine singular, masculine plural, feminine plural.
    pub past_participle: [String; 4],
    pub pp_invariable: bool,
    pub auxiliary_etre: bool,
    pub impersonal: bool,
}

impl SimpleForms {
    pub fn get(&self, tense: SimpleTense, person: usize) -> Option<&str> {
        self.tenses[tense as usize][person].as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ps {
    A,
    I,
    U,
    In,
    None,
}

impl Ps {
    fn endings(self) -> [&'static str; 6] {
        match self {
            Ps::A => ["ai", "as", "a", "âmes", "âtes", "èrent"],
            Ps::I => ["is", "is", "it", "îmes", "îtes", "irent"],
            Ps::U => ["us", "us", "ut", "ûmes", "ûtes", "urent"],
            Ps::In => ["ins", "ins", "int", "înmes", "întes", "inrent"],
            Ps::None => [""; 6],
        }
    }

    fn subj_endings(self) -> [&'static str; 6] {
        match self {
            Ps::A => ["asse", "asses", "ât", "assions", "assiez", "assent"],
            Ps::I => ["isse", "isses", "ît", "issions", "issiez", "issent"],
            Ps::U => ["usse", "usses", "ût", "ussions", "ussiez", "ussent"],
            Ps::In => ["insse", "insses", "înt", "inssions", "inssiez", "inssent"],
            Ps::None => [""; 6],
        }
    }
}

/// A third-group (or irregular) conjugation model. All strings are suffixes appended
/// to the radical, i.e. the infinitive minus `ending`.
struct Model {
    name: &'static str,
    ending: &'static str,
    present: [&'static str; 6],
    imparfait: Option<&'static str>,
    ps: Ps,
    ps_stem: &'static str,
    futur: &'static str,
    subj: Option<[&'static str; 6]>,
    imper: Option<[&'static str; 3]>,
    ppr: Option<&'static str>,
    pp: &'static str,
    pp_invariable: bool,
    impersonal: bool,
}

const fn model(
    name: &'static str,
    ending: &'static str,
    present: [&'static str; 6],
    ps: Ps,
    ps_stem: &'static str,
    futur: &'static str,
    pp: &'static str,
) -> Model {
    Model {
        name,
        ending,
        present,
        imparfait: None,
        ps,
        ps_stem,
        futur,
        subj: None,
        imper: None,
        ppr: None,
        pp,
        pp_invariable: false,
        impersonal: false,
    }
}

static MODELS: &[Model] = &[
    Model {
        subj: Some(["sois", "sois", "soit", "soyons", "soyez", "soient"]),
        imper: Some(["sois", "soyons", "soyez"]),
        imparfait: Some("ét"),
        ppr: Some("étant"),
        pp_invariable: true,
        ..model("être", "être", ["suis", "es", "est", "sommes", "êtes", "sont"], Ps::U, "f", "ser", "été")
    },
    Model {
        subj: Some(["aie", "aies", "ait", "ayons", "ayez", "aient"]),
        imper: Some(["aie", "ayons", "ayez"]),
        ppr: Some("ayant"),
        ..model("avoir", "avoir", ["ai", "as", "a", "avons", "avez", "ont"], Ps::U, "e", "aur", "eu")
    },
    Model {
        subj: Some(["aille", "ailles", "aille", "allions", "alliez", "aillent"]),
        imper: Some(["va", "allons", "allez"]),
        ..model("aller", "aller", ["vais", "vas", "va", "allons", "allez", "vont"], Ps::A, "all", "ir", "allé")
    },
    Model {
        subj: Some(["fasse", "fasses", "fasse", "fassions", "fassiez", "fassent"]),
        ..model("faire", "faire", ["fais", "fais", "fait", "faisons", "faites", "font"], Ps::I, "f", "fer", "fait")
    },
    model("dire", "dire", ["dis", "dis", "dit", "disons", "dites", "disent"], Ps::I, "d", "dir", "dit"),
    model("prédire", "dire", ["dis", "dis", "dit", "disons", "disez", "disent"], Ps::I, "d", "dir", "dit"),
    model("maudire", "dire", ["dis", "dis", "dit", "dissons", "dissez", "dissent"], Ps::I, "d", "dir", "dit"),
    Model {
        subj: Some(["puisse", "puisses", "puisse", "puissions", "puissiez", "puissent"]),
        imper: Some(["", "", ""]),
        pp_invariable: true,
        ..model(
            "pouvoir",
            "pouvoir",
            ["peux", "peux", "peut", "pouvons", "pouvez", "peuvent"],
            Ps::U,
            "p",
            "pourr",
            "pu",
        )
    },
    Model {
        subj: Some(["veuille", "veuilles", "veuille", "voulions", "vouliez", "veuillent"]),
        imper: Some(["veuille", "veuillons", "veuillez"]),
        ..model(
            "vouloir",
            "vouloir",
            ["veux", "veux", "veut", "voulons", "voulez", "veulent"],
            Ps::U,
            "voul",
            "voudr",
            "voulu",
        )
    },
    Model {
        subj: Some(["sache", "saches", "sache", "sachions", "sachiez", "sachent"]),
        imper: Some(["sache", "sachons", "sachez"]),
        ppr: Some("sachant"),
        ..model("savoir", "savoir", ["sais", "sais", "sait", "savons", "savez", "savent"], Ps::U, "s", "saur", "su")
    },
    Model {
        subj: Some(["aille", "ailles", "aille", "alions", "aliez", "aillent"]),
        ..model("valoir", "aloir", ["aux", "aux", "aut", "alons", "alez", "alent"], Ps::U, "al", "audr", "alu")
    },
    Model {
        imparfait: Some("fall"),
        subj: Some(["", "", "faille", "", "", ""]),
        ppr: Some(""),
        pp_invariable: true,
        impersonal: true,
        ..model("falloir", "falloir", ["", "", "faut", "", "", ""], Ps::U, "fall", "faudr", "fallu")
    },
    Model {
        imparfait: Some("pleuv"),
        subj: Some(["", "", "pleuve", "", "", ""]),
        pp_invariable: true,
        impersonal: true,
        ..model("pleuvoir", "pleuvoir", ["", "", "pleut", "", "", ""], Ps::U, "pl", "pleuvr", "plu")
    },
    model("voir", "voir", ["vois", "vois", "voit", "voyons", "voyez", "voient"], Ps::I, "v", "verr", "vu"),
    model("prévoir", "voir", ["vois", "vois", "voit", "voyons", "voyez", "voient"], Ps::I, "v", "voir", "vu"),
    model("pourvoir", "voir", ["vois", "vois", "voit", "voyons", "voyez", "voient"], Ps::U, "v", "voir", "vu"),
    model("venir", "enir", ["iens", "iens", "ient", "enons", "enez", "iennent"], Ps::In, "", "iendr", "enu"),
    model(
        "prendre",
        "prendre",
        ["prends", "prends", "prend", "prenons", "prenez", "prennent"],
        Ps::I,
        "pr",
        "prendr",
        "pris",
    ),
    model("mettre", "mettre", ["mets", "mets", "met", "mettons", "mettez", "mettent"], Ps::I, "m", "mettr", "mis"),
    model("battre", "battre", ["bats", "bats", "bat", "battons", "battez", "battent"], Ps::I, "batt", "battr", "battu"),
    model("rendre", "re", ["s", "s", "", "ons", "ez", "ent"], Ps::I, "", "r", "u"),
    model("rompre", "re", ["s", "s", "t", "ons", "ez", "ent"], Ps::I, "", "r", "u"),
    model("peindre", "ndre", ["ns", "ns", "nt", "gnons", "gnez", "gnent"], Ps::I, "gn", "ndr", "nt"),
    model("conduire", "re", ["s", "s", "t", "sons", "sez", "sent"], Ps::I, "s", "r", "t"),
    Model { pp_invariable: true, ..model("luire", "re", ["s", "s", "t", "sons", "sez", "sent"], Ps::I, "s", "r", "") },
    model("connaître", "aître", ["ais", "ais", "aît", "aissons", "aissez", "aissent"], Ps::U, "", "aîtr", "u"),
    model("naître", "aître", ["ais", "ais", "aît", "aissons", "aissez", "aissent"], Ps::I, "aqu", "aîtr", "é"),
    model("croire", "oire", ["ois", "ois", "oit", "oyons", "oyez", "oient"], Ps::U, "", "oir", "u"),
    model("boire", "oire", ["ois", "ois", "oit", "uvons", "uvez", "oivent"], Ps::U, "", "oir", "u"),
    model("lire", "ire", ["is", "is", "it", "isons", "isez", "isent"], Ps::U, "", "ir", "u"),
    model("écrire", "ire", ["is", "is", "it", "ivons", "ivez", "ivent"], Ps::I, "iv", "ir", "it"),
    model("vivre", "ivre", ["is", "is", "it", "ivons", "ivez", "ivent"], Ps::U, "éc", "ivr", "écu"),
    model("suivre", "ivre", ["is", "is", "it", "ivons", "ivez", "ivent"], Ps::I, "iv", "ivr", "ivi"),
    Model {
        pp_invariable: true,
        ..model("rire", "ire", ["is", "is", "it", "ions", "iez", "ient"], Ps::I, "", "ir", "i")
    },
    Model {
        pp_invariable: true,
        ..model("plaire", "aire", ["ais", "ais", "aît", "aisons", "aisez", "aisent"], Ps::U, "", "air", "u")
    },
    model("taire", "aire", ["ais", "ais", "ait", "aisons", "aisez", "aisent"], Ps::U, "", "air", "u"),
    Model {
        pp_invariable: true,
        ..model("suffire", "ire", ["is", "is", "it", "isons", "isez", "isent"], Ps::I, "", "ir", "i")
    },
    model("partir", "tir", ["s", "s", "t", "tons", "tez", "tent"], Ps::I, "t", "tir", "ti"),
    model("dormir", "mir", ["s", "s", "t", "mons", "mez", "ment"], Ps::I, "m", "mir", "mi"),
    model("servir", "vir", ["s", "s", "t", "vons", "vez", "vent"], Ps::I, "v", "vir", "vi"),
    model("ouvrir", "rir", ["re", "res", "re", "rons", "rez", "rent"], Ps::I, "r", "rir", "ert"),
    model("cueillir", "ir", ["e", "es", "e", "ons", "ez", "ent"], Ps::I, "", "er", "i"),
    model("assaillir", "ir", ["e", "es", "e", "ons", "ez", "ent"], Ps::I, "", "ir", "i"),
    model("courir", "ir", ["s", "s", "t", "ons", "ez", "ent"], Ps::U, "", "r", "u"),
    model("mourir", "ourir", ["eurs", "eurs", "eurt", "ourons", "ourez", "eurent"], Ps::U, "our", "ourr", "ort"),
    model("recevoir", "cevoir", ["çois", "çois", "çoit", "cevons", "cevez", "çoivent"], Ps::U, "ç", "cevr", "çu"),
    model("devoir", "evoir", ["ois", "ois", "oit", "evons", "evez", "oivent"], Ps::U, "", "evr", "û"),
    model("asseoir", "eoir", ["ieds", "ieds", "ied", "eyons", "eyez", "eyent"], Ps::I, "", "iér", "is"),
    model("mouvoir", "ouvoir", ["eus", "eus", "eut", "ouvons", "ouvez", "euvent"], Ps::U, "", "ouvr", "u"),
    model("acquérir", "érir", ["iers", "iers", "iert", "érons", "érez", "ièrent"], Ps::I, "", "err", "is"),
    model("fuir", "ir", ["is", "is", "it", "yons", "yez", "ient"], Ps::I, "", "ir", "i"),
    model("vaincre", "cre", ["cs", "cs", "c", "quons", "quez", "quent"], Ps::I, "qu", "cr", "cu"),
    model("coudre", "dre", ["ds", "ds", "d", "sons", "sez", "sent"], Ps::I, "s", "dr", "su"),
    model("moudre", "dre", ["ds", "ds", "d", "lons", "lez", "lent"], Ps::U, "l", "dr", "lu"),
    model("résoudre", "oudre", ["ous", "ous", "out", "olvons", "olvez", "olvent"], Ps::U, "olv", "oudr", "olu"),
    model("conclure", "ure", ["us", "us", "ut", "uons", "uez", "uent"], Ps::U, "", "ur", "u"),
    model("inclure", "ure", ["us", "us", "ut", "uons", "uez", "uent"], Ps::U, "", "ur", "us"),
    model(
        "bouillir",
        "ouillir",
        ["ous", "ous", "out", "ouillons", "ouillez", "ouillent"],
        Ps::I,
        "ouill",
        "ouillir",
        "ouilli",
    ),
    model("vêtir", "ir", ["s", "s", "", "ons", "ez", "ent"], Ps::I, "", "ir", "u"),
    model("traire", "aire", ["ais", "ais", "ait", "ayons", "ayez", "aient"], Ps::None, "", "air", "ait"),
    model("accroître", "oître", ["ois", "ois", "oît", "oissons", "oissez", "oissent"], Ps::U, "", "oîtr", "u"),
];

fn find_model(name: &str) -> &'static Model {
    MODELS.iter().find(|m| m.name == name).unwrap_or_else(|| panic!("unknown conjugation model {name}"))
}

/// Verbs that take a specific model regardless of their ending.
const EXACT_MODELS: &[(&str, &str)] = &[
    ("être", "être"),
    ("avoir", "avoir"),
    ("aller", "aller"),
    ("dire", "dire"),
    ("redire", "dire"),
    ("contredire", "prédire"),
    ("dédire", "prédire"),
    ("interdire", "prédire"),
    ("médire", "prédire"),
    ("prédire", "prédire"),
    ("maudire", "maudire"),
    ("pouvoir", "pouvoir"),
    ("vouloir", "vouloir"),
    ("savoir", "savoir"),
    ("falloir", "falloir"),
    ("pleuvoir", "pleuvoir"),
    ("voir", "voir"),
    ("revoir", "voir"),
    ("entrevoir", "voir"),
    ("prévoir", "prévoir"),
    ("pourvoir", "pourvoir"),
    ("naître", "naître"),
    ("renaître", "naître"),
    ("croire", "croire"),
    ("boire", "boire"),
    ("lire", "lire"),
    ("élire", "lire"),
    ("relire", "lire"),
    ("réélire", "lire"),
    ("rire", "rire"),
    ("sourire", "rire"),
    ("plaire", "plaire"),
    ("déplaire", "plaire"),
    ("complaire", "plaire"),
    ("taire", "taire"),
    ("suffire", "suffire"),
    ("luire", "luire"),
    ("reluire", "luire"),
    ("nuire", "luire"),
    ("partir", "partir"),
    ("repartir", "partir"),
    ("départir", "partir"),
    ("sortir", "partir"),
    ("ressortir", "partir"),
    ("mentir", "partir"),
    ("démentir", "partir"),
    ("sentir", "partir"),
    ("consentir", "partir"),
    ("ressentir", "partir"),
    ("pressentir", "partir"),
    ("repentir", "partir"),
    ("dormir", "dormir"),
    ("endormir", "dormir"),
    ("rendormir", "dormir"),
    ("servir", "servir"),
    ("desservir", "servir"),
    ("resservir", "servir"),
    ("cueillir", "cueillir"),
    ("accueillir", "cueillir"),
    ("recueillir", "cueillir"),
    ("assaillir", "assaillir"),
    ("tressaillir", "assaillir"),
    ("défaillir", "assaillir"),
    ("mourir", "mourir"),
    ("devoir", "devoir"),
    ("redevoir", "devoir"),
    ("asseoir", "asseoir"),
    ("rasseoir", "asseoir"),
    ("mouvoir", "mouvoir"),
    ("émouvoir", "mouvoir"),
    ("promouvoir", "mouvoir"),
    ("fuir", "fuir"),
    ("enfuir", "fuir"),
    ("coudre", "coudre"),
    ("découdre", "coudre"),
    ("recoudre", "coudre"),
    ("moudre", "moudre"),
    ("résoudre", "résoudre"),
    ("absoudre", "résoudre"),
    ("dissoudre", "résoudre"),
    ("conclure", "conclure"),
    ("exclure", "conclure"),
    ("inclure", "inclure"),
    ("occlure", "inclure"),
    ("bouillir", "bouillir"),
    ("vêtir", "vêtir"),
    ("revêtir", "vêtir"),
    ("dévêtir", "vêtir"),
    ("traire", "traire"),
    ("extraire", "traire"),
    ("distraire", "traire"),
    ("soustraire", "traire"),
    ("abstraire", "traire"),
    ("accroître", "accroître"),
    ("décroître", "accroître"),
    ("valoir", "valoir"),
    ("équivaloir", "valoir"),
    ("revaloir", "valoir"),
    ("prévaloir", "valoir"),
];

/// Verbs matched by their ending, longest first.
const SUFFIX_MODELS: &[(&str, &str)] = &[
    ("prendre", "prendre"),
    ("mettre", "mettre"),
    ("battre", "battre"),
    ("cevoir", "recevoir"),
    ("courir", "courir"),
    ("quérir", "acquérir"),
    ("venir", "venir"),
    ("tenir", "venir"),
    ("faire", "faire"),
    ("vivre", "vivre"),
    ("suivre", "suivre"),
    ("rompre", "rompre"),
    ("aître", "connaître"),
    ("crire", "écrire"),
    ("indre", "peindre"),
    ("vaincre", "vaincre"),
    ("ouvrir", "ouvrir"),
    ("couvrir", "ouvrir"),
    ("offrir", "ouvrir"),
    ("souffrir", "ouvrir"),
    ("uire", "conduire"),
    ("andre", "rendre"),
    ("endre", "rendre"),
    ("ondre", "rendre"),
    ("erdre", "rendre"),
    ("ordre", "rendre"),
];

/// Verbs conjugated with « être » in compound tenses.
const ETRE_VERBS: &[&str] = &[
    "aller",
    "venir",
    "devenir",
    "revenir",
    "redevenir",
    "parvenir",
    "survenir",
    "intervenir",
    "advenir",
    "provenir",
    "arriver",
    "partir",
    "repartir",
    "entrer",
    "rentrer",
    "monter",
    "remonter",
    "descendre",
    "redescendre",
    "tomber",
    "retomber",
    "rester",
    "naître",
    "renaître",
    "mourir",
    "décéder",
    "retourner",
    "sortir",
    "ressortir",
    "passer",
    "repasser",
    "apparaître",
    "éclore",
    "accourir",
];

/// Verbs in -eler/-eter that take « è » instead of doubling the consonant.
const E_GRAVE_ELER: &[&str] = &[
    "acheter",
    "racheter",
    "geler",
    "dégeler",
    "congeler",
    "surgeler",
    "regeler",
    "peler",
    "modeler",
    "remodeler",
    "harceler",
    "celer",
    "receler",
    "déceler",
    "ciseler",
    "démanteler",
    "écarteler",
    "marteler",
    "crocheter",
    "fureter",
    "haleter",
    "corseter",
    "bégueter",
];

/// Verbs beginning with an aspirated h (no elision: « je hais »).
const H_ASPIRE_VERBS: &[&str] = &[
    "haïr",
    "hacher",
    "haleter",
    "hanter",
    "happer",
    "harceler",
    "harasser",
    "harponner",
    "hasarder",
    "hâter",
    "hausser",
    "heurter",
    "hisser",
    "hocher",
    "honnir",
    "hurler",
    "huer",
    "hennir",
    "hérisser",
    "hacker",
    "handicaper",
    "hiérarchiser",
    "houspiller",
    "hululer",
];

fn elides_before(word: &str, verb_infinitive: &str) -> bool {
    if starts_with_vowel(word) {
        return true;
    }
    word.starts_with('h') && !H_ASPIRE_VERBS.contains(&verb_infinitive)
}

enum Kind {
    First,
    Second,
    Hair,
    Model(&'static Model),
}

/// Determines how `verb` conjugates. `second_group` comes from the dictionary when known.
fn resolve(verb: &str, second_group: Option<bool>) -> Option<Kind> {
    if let Some((_, name)) = EXACT_MODELS.iter().find(|(v, _)| *v == verb) {
        return Some(Kind::Model(find_model(name)));
    }
    if verb == "haïr" || verb == "entrhaïr" {
        return Some(Kind::Hair);
    }
    if verb.ends_with("er") && verb.chars().count() > 2 {
        return Some(Kind::First);
    }
    if second_group == Some(true) && verb.ends_with("ir") {
        return Some(Kind::Second);
    }
    if let Some((_, name)) = SUFFIX_MODELS.iter().find(|(s, _)| verb.ends_with(s)) {
        let m = find_model(name);
        if verb.len() > m.ending.len() || verb == m.name {
            return Some(Kind::Model(m));
        }
    }
    if verb.ends_with("ir") && second_group.is_none() && verb.chars().count() > 3 {
        return Some(Kind::Second);
    }
    None
}

fn starts_with_ao(ending: &str) -> bool {
    ending.starts_with(['a', 'â', 'o'])
}

/// Applies the "mute e" stem alternations of first-group verbs.
fn first_group_mute_stem(radical: &str, verb: &str, future: bool) -> String {
    if radical.ends_with("oy") || radical.ends_with("uy") || radical.ends_with("ay") {
        let mut s = radical[..radical.len() - 1].to_string();
        s.push('i');
        return s;
    }
    let chars: Vec<char> = radical.chars().collect();
    let n = chars.len();
    if n >= 3 && (verb.ends_with("eler") || verb.ends_with("eter")) && chars[n - 2] == 'e' && chars[n - 3] != 'e' {
        if E_GRAVE_ELER.contains(&verb) {
            let mut c = chars.clone();
            c[n - 2] = 'è';
            return c.into_iter().collect();
        }
        let mut s = radical.to_string();
        s.push(chars[n - 1]);
        return s;
    }
    // Find the last vowel and the consonant cluster after it.
    let is_vowel = |c: char| "aeiouyéèêëàâîïôöûüù".contains(c);
    // The u of "gu" and "qu" is part of the consonant ("alléguer" → "allègue").
    let is_consonant_at =
        |i: usize| !is_vowel(chars[i]) || (chars[i] == 'u' && i > 0 && matches!(chars[i - 1], 'g' | 'q'));
    let mut i = n;
    while i > 0 && is_consonant_at(i - 1) {
        i -= 1;
    }
    if i == 0 || i == n {
        return radical.to_string();
    }
    let vowel_idx = i - 1;
    let cluster: String = chars[i..].iter().collect();
    let cl: Vec<char> = cluster.chars().collect();
    let cluster_ok = (cl.len() == 1 && cl[0] != 'x')
        || (cl.len() == 2
            && matches!(cl[0], 'b' | 'c' | 'd' | 'f' | 'g' | 'p' | 't' | 'v')
            && matches!(cl[1], 'r' | 'l'))
        || matches!(cluster.as_str(), "gn" | "ch" | "gu" | "qu");
    // "ee" in borrowed words ("geeker", "squeezer") never alternates.
    let doubled_vowel = vowel_idx > 0 && matches!(chars[vowel_idx - 1], 'e' | 'é');
    if !cluster_ok || doubled_vowel {
        return radical.to_string();
    }
    let v = chars[vowel_idx];
    if v == 'e' || (v == 'é' && !future) {
        let mut c = chars;
        c[vowel_idx] = 'è';
        return c.into_iter().collect();
    }
    radical.to_string()
}

fn first_group_form(radical: &str, verb: &str, ending: &str, mute: bool) -> String {
    let mut stem = if mute { first_group_mute_stem(radical, verb, false) } else { radical.to_string() };
    if starts_with_ao(ending) {
        if stem.ends_with('c') {
            stem.pop();
            stem.push('ç');
        } else if stem.ends_with('g') {
            stem.push('e');
        }
    }
    stem + ending
}

fn arr6(f: impl Fn(usize) -> Option<String>) -> [Option<String>; 6] {
    std::array::from_fn(f)
}

fn pp_variants(ms: &str, invariable: bool) -> [String; 4] {
    if invariable {
        return [ms.to_string(), ms.to_string(), ms.to_string(), ms.to_string()];
    }
    if let Some(stem) = ms.strip_suffix('û') {
        let base = format!("{stem}u");
        return [ms.to_string(), format!("{base}e"), format!("{base}s"), format!("{base}es")];
    }
    if ms.ends_with('s') || ms.ends_with('x') {
        return [ms.to_string(), format!("{ms}e"), ms.to_string(), format!("{ms}es")];
    }
    [ms.to_string(), format!("{ms}e"), format!("{ms}s"), format!("{ms}es")]
}

fn first_group(verb: &str) -> SimpleForms {
    let radical = &verb[..verb.len() - 2];
    let f = |ending: &str, mute: bool| first_group_form(radical, verb, ending, mute);
    const MUTE: [bool; 6] = [true, true, true, false, false, true];
    let present = ["e", "es", "e", "ons", "ez", "ent"];
    let imparfait = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let ps = Ps::A.endings();
    let subj = ["e", "es", "e", "ions", "iez", "ent"];
    let subj_imp = Ps::A.subj_endings();
    let futur_stem = match verb {
        "envoyer" => "enverr".to_string(),
        "renvoyer" => "renverr".to_string(),
        _ => first_group_mute_stem(radical, verb, true) + "er",
    };
    let fut = ["ai", "as", "a", "ons", "ez", "ont"];
    let cond = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let pp = format!("{radical}é");
    SimpleForms {
        infinitive: verb.to_string(),
        group: 1,
        model: "chanter",
        tenses: [
            arr6(|i| Some(f(present[i], MUTE[i]))),
            arr6(|i| Some(f(imparfait[i], false))),
            arr6(|i| Some(f(ps[i], false))),
            arr6(|i| Some(format!("{futur_stem}{}", fut[i]))),
            arr6(|i| Some(format!("{futur_stem}{}", cond[i]))),
            arr6(|i| Some(f(subj[i], MUTE[i]))),
            arr6(|i| Some(f(subj_imp[i], false))),
        ],
        imperative: [Some(f("e", true)), Some(f("ons", false)), Some(f("ez", false))],
        present_participle: Some(f("ant", false)),
        past_participle: pp_variants(&pp, false),
        pp_invariable: false,
        auxiliary_etre: ETRE_VERBS.contains(&verb),
        impersonal: false,
    }
}

fn second_group_table(verb: &str, hair: bool) -> SimpleForms {
    let radical = verb.strip_suffix(if hair { "ïr" } else { "ir" }).unwrap_or(verb);
    let r = |s: &str| Some(format!("{radical}{s}"));
    let (present, ps, subj_imp): ([&str; 6], [&str; 6], [&str; 6]) = if hair {
        // « haïr » keeps its diaeresis and has no circumflex in the passé simple.
        (
            ["is", "is", "it", "ïssons", "ïssez", "ïssent"],
            ["ïs", "ïs", "ït", "ïmes", "ïtes", "ïrent"],
            ["ïsse", "ïsses", "ït", "ïssions", "ïssiez", "ïssent"],
        )
    } else {
        (["is", "is", "it", "issons", "issez", "issent"], Ps::I.endings(), Ps::I.subj_endings())
    };
    // For haïr the radical is "ha" and the diaeresis is written in the endings.
    let radical_imp = if hair { "ïss" } else { "iss" };
    let imparfait = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let subj = ["e", "es", "e", "ions", "iez", "ent"];
    let fut = ["ai", "as", "a", "ons", "ez", "ont"];
    let cond = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let pp = if hair { format!("{radical}ï") } else { format!("{radical}i") };
    SimpleForms {
        infinitive: verb.to_string(),
        group: if hair { 3 } else { 2 },
        model: if hair { "haïr" } else { "finir" },
        tenses: [
            arr6(|i| r(present[i])),
            arr6(|i| r(&format!("{radical_imp}{}", imparfait[i]))),
            arr6(|i| r(ps[i])),
            arr6(|i| Some(format!("{verb}{}", fut[i]))),
            arr6(|i| Some(format!("{verb}{}", cond[i]))),
            arr6(|i| r(&format!("{radical_imp}{}", subj[i]))),
            arr6(|i| r(subj_imp[i])),
        ],
        imperative: [r(present[1]), r(present[3]), r(present[4])],
        present_participle: r(&format!("{radical_imp}ant")),
        past_participle: pp_variants(&pp, false),
        pp_invariable: false,
        auxiliary_etre: false,
        impersonal: false,
    }
}

fn from_model(verb: &str, m: &'static Model) -> SimpleForms {
    let radical = &verb[..verb.len() - m.ending.len()];
    let r = |s: &str| -> Option<String> {
        if s.is_empty() && m.impersonal {
            None
        } else {
            Some(format!("{radical}{s}"))
        }
    };
    let present: [Option<String>; 6] = arr6(|i| if m.impersonal && i != 2 { None } else { r(m.present[i]) });
    let imp_stem = m.imparfait.map(str::to_string).unwrap_or_else(|| m.present[3].trim_end_matches("ons").to_string());
    let imparfait_end = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let only3 = |i: usize| !m.impersonal || i == 2;
    let imparfait = arr6(|i| only3(i).then(|| format!("{radical}{imp_stem}{}", imparfait_end[i])));
    let ps = arr6(|i| (only3(i) && m.ps != Ps::None).then(|| format!("{radical}{}{}", m.ps_stem, m.ps.endings()[i])));
    let subj_imp =
        arr6(|i| (only3(i) && m.ps != Ps::None).then(|| format!("{radical}{}{}", m.ps_stem, m.ps.subj_endings()[i])));
    let fut_end = ["ai", "as", "a", "ons", "ez", "ont"];
    let cond_end = ["ais", "ais", "ait", "ions", "iez", "aient"];
    let futur = arr6(|i| only3(i).then(|| format!("{radical}{}{}", m.futur, fut_end[i])));
    let cond = arr6(|i| only3(i).then(|| format!("{radical}{}{}", m.futur, cond_end[i])));
    let subj = match m.subj {
        Some(s) => arr6(|i| (!s[i].is_empty()).then(|| format!("{radical}{}", s[i]))),
        None => {
            let third_plural = m.present[5].trim_end_matches("ent");
            let end = ["e", "es", "e", "ions", "iez", "ent"];
            arr6(|i| {
                Some(if i == 3 || i == 4 {
                    format!("{radical}{imp_stem}{}", end[i])
                } else {
                    format!("{radical}{third_plural}{}", end[i])
                })
            })
        }
    };
    let imperative: [Option<String>; 3] = if m.impersonal {
        [None, None, None]
    } else {
        match m.imper {
            Some(s) => std::array::from_fn(|i| (!s[i].is_empty()).then(|| format!("{radical}{}", s[i]))),
            None => {
                let mut sg = m.present[1].to_string();
                if sg.ends_with("es") {
                    sg.pop();
                }
                [
                    Some(format!("{radical}{sg}")),
                    Some(format!("{radical}{}", m.present[3])),
                    Some(format!("{radical}{}", m.present[4])),
                ]
            }
        }
    };
    let present_participle = match m.ppr {
        Some("") => None,
        Some(p) => Some(format!("{radical}{p}")),
        None => Some(format!("{radical}{imp_stem}ant")),
    };
    let pp = format!("{radical}{}", m.pp);
    let group = if m.name == "être" || m.name == "avoir" { 0 } else { 3 };
    SimpleForms {
        infinitive: verb.to_string(),
        group,
        model: m.name,
        tenses: [present, imparfait, ps, futur, cond, subj, subj_imp],
        imperative,
        present_participle,
        past_participle: pp_variants(&pp, m.pp_invariable),
        pp_invariable: m.pp_invariable,
        auxiliary_etre: ETRE_VERBS.contains(&verb),
        impersonal: m.impersonal,
    }
}

/// Conjugates the simple tenses of an infinitive (without « se »).
pub fn simple_forms(verb: &str, second_group: Option<bool>) -> Option<SimpleForms> {
    Some(match resolve(verb, second_group)? {
        Kind::First => first_group(verb),
        Kind::Second => second_group_table(verb, false),
        Kind::Hair => second_group_table(verb, true),
        Kind::Model(m) => from_model(verb, m),
    })
}

/// Joins words, eliding « je, me, te, se, que, ne, de » before a vowel or mute h.
fn join_elide(words: &[&str], infinitive: &str) -> String {
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        if w.is_empty() {
            continue;
        }
        if i + 1 < words.len() {
            let next = words[i + 1];
            let elidable = matches!(*w, "je" | "me" | "te" | "se" | "que" | "ne" | "de");
            if elidable && elides_before(next, infinitive) {
                out.push_str(&w[..w.len() - 1]);
                out.push('\'');
                continue;
            }
        }
        out.push_str(w);
        if i + 1 < words.len() {
            out.push(' ');
        }
    }
    out
}

fn subjunctive_pronoun(p: usize, impersonal: bool) -> String {
    match p {
        2 if impersonal => "qu’il".into(),
        2 => "qu’il/elle".into(),
        5 => "qu’ils/elles".into(),
        _ => format!("que {}", PRONOUNS[p]),
    }
}

/// Normalises an input such as "Se laver", "s’asseoir" into (infinitive, pronominal).
pub fn parse_infinitive(input: &str) -> (String, bool) {
    let lower = input.trim().to_lowercase().replace('’', "'");
    for prefix in ["se ", "s'"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return (rest.trim().to_string(), true);
        }
    }
    (lower, false)
}

/// Display form of the past participle for être agreement: "allé(e)", "allé(e)s".
fn agreed_participle(forms: &SimpleForms, plural: bool) -> String {
    let [ms, fs, mp, fp] = &forms.past_participle;
    if forms.pp_invariable {
        return ms.clone();
    }
    if !plural {
        if *fs == format!("{ms}e") {
            format!("{ms}(e)")
        } else {
            format!("{ms}/{fs}")
        }
    } else if *mp == format!("{ms}s") && *fp == format!("{ms}es") {
        format!("{ms}(e)s")
    } else if mp == ms && *fp == format!("{ms}es") {
        format!("{ms}(es)")
    } else {
        format!("{mp}/{fp}")
    }
}

/// Builds the full conjugation table of a French verb.
pub fn conjugate(input: &str, second_group: Option<bool>, known: bool) -> Result<Conjugation, ConjugationError> {
    let (verb, pronominal) = parse_infinitive(input);
    if verb.is_empty() {
        return Err(ConjugationError::Empty);
    }
    let forms = simple_forms(&verb, second_group).ok_or_else(|| ConjugationError::NotAVerb(verb.clone()))?;
    let etre = pronominal || forms.auxiliary_etre;
    let aux = simple_forms(if etre { "être" } else { "avoir" }, None).expect("auxiliary");

    let persons: Vec<usize> = if forms.impersonal { vec![2] } else { (0..6).collect() };
    let pronoun_for = |p: usize| if forms.impersonal { "il".to_string() } else { PRONOUNS[p].to_string() };

    let simple_tense = |t: SimpleTense, subjunctive: bool, id: &str, name: &str| -> Tense {
        let mut out = Vec::new();
        for &p in &persons {
            let Some(form) = forms.get(t, p) else { continue };
            let pronoun = if subjunctive { subjunctive_pronoun(p, forms.impersonal) } else { pronoun_for(p) };
            let form_words: Vec<&str> = if pronominal { vec![REFLEXIVE[p], form] } else { vec![form] };
            let form_text = join_elide(&form_words, &verb);
            let mut all: Vec<&str> = pronoun.split(' ').collect();
            all.extend(form_words.iter());
            let text = join_elide(&all, &verb);
            out.push(PersonForm { pronoun, form: form_text, text });
        }
        Tense { id: id.into(), name: name.into(), compound: false, forms: out }
    };

    let compound_tense = |t: SimpleTense, subjunctive: bool, id: &str, name: &str| -> Tense {
        let mut out = Vec::new();
        for &p in &persons {
            let Some(aux_form) = aux.get(t, p) else { continue };
            let pp = if etre { agreed_participle(&forms, p >= 3) } else { forms.past_participle[0].clone() };
            let pronoun = if subjunctive { subjunctive_pronoun(p, forms.impersonal) } else { pronoun_for(p) };
            let mut form_words: Vec<&str> = Vec::new();
            if pronominal {
                form_words.push(REFLEXIVE[p]);
            }
            form_words.push(aux_form);
            form_words.push(&pp);
            let form_text = join_elide(&form_words, "");
            let mut all: Vec<&str> = pronoun.split(' ').collect();
            all.extend(form_words.iter());
            let text = join_elide(&all, "");
            out.push(PersonForm { pronoun, form: form_text, text });
        }
        Tense { id: id.into(), name: name.into(), compound: true, forms: out }
    };

    use SimpleTense::*;
    let mut indicatif = vec![
        simple_tense(IndPresent, false, "present", "Présent"),
        compound_tense(IndPresent, false, "passe_compose", "Passé composé"),
        simple_tense(IndImparfait, false, "imparfait", "Imparfait"),
        compound_tense(IndImparfait, false, "plus_que_parfait", "Plus-que-parfait"),
        simple_tense(IndPasseSimple, false, "passe_simple", "Passé simple"),
        compound_tense(IndPasseSimple, false, "passe_anterieur", "Passé antérieur"),
        simple_tense(IndFutur, false, "futur", "Futur simple"),
        compound_tense(IndFutur, false, "futur_anterieur", "Futur antérieur"),
    ];
    indicatif.retain(|t| !t.forms.is_empty());
    let conditionnel = vec![
        simple_tense(CondPresent, false, "present", "Présent"),
        compound_tense(CondPresent, false, "passe", "Passé"),
    ];
    let mut subjonctif = vec![
        simple_tense(SubjPresent, true, "present", "Présent"),
        compound_tense(SubjPresent, true, "passe", "Passé"),
        simple_tense(SubjImparfait, true, "imparfait", "Imparfait"),
        compound_tense(SubjImparfait, true, "plus_que_parfait", "Plus-que-parfait"),
    ];
    subjonctif.retain(|t| !t.forms.is_empty());

    let mut moods = vec![
        Mood { id: "indicatif".into(), name: "Indicatif".into(), tenses: indicatif },
        Mood { id: "conditionnel".into(), name: "Conditionnel".into(), tenses: conditionnel },
        Mood { id: "subjonctif".into(), name: "Subjonctif".into(), tenses: subjonctif },
    ];

    // Imperative.
    let mut imp_present = Vec::new();
    let mut imp_passe = Vec::new();
    const IMP_REFLEXIVE: [&str; 3] = ["toi", "nous", "vous"];
    for (k, form) in forms.imperative.iter().enumerate() {
        let Some(form) = form else { continue };
        let text = if pronominal { format!("{form}-{}", IMP_REFLEXIVE[k]) } else { form.clone() };
        imp_present.push(PersonForm { pronoun: String::new(), form: text.clone(), text });
        if !pronominal {
            if let Some(a) = &aux.imperative[k] {
                let pp = if etre {
                    agreed_participle(&forms, IMPERATIVE_PERSONS[k] >= 3)
                } else {
                    forms.past_participle[0].clone()
                };
                let text = format!("{a} {pp}");
                imp_passe.push(PersonForm { pronoun: String::new(), form: text.clone(), text });
            }
        }
    }
    if !imp_present.is_empty() {
        let mut tenses =
            vec![Tense { id: "present".into(), name: "Présent".into(), compound: false, forms: imp_present }];
        if !imp_passe.is_empty() {
            tenses.push(Tense { id: "passe".into(), name: "Passé".into(), compound: true, forms: imp_passe });
        }
        moods.push(Mood { id: "imperatif".into(), name: "Impératif".into(), tenses });
    }

    let aux_inf = if etre { "être" } else { "avoir" };
    let pp_display = if etre { agreed_participle(&forms, false) } else { forms.past_participle[0].clone() };
    let refl_inf = |w: &str| if pronominal { join_elide(&["se", w], &verb) } else { w.to_string() };
    let infinitive_display = refl_inf(&verb);
    moods.push(Mood {
        id: "infinitif".into(),
        name: "Infinitif".into(),
        tenses: vec![
            Tense {
                id: "present".into(),
                name: "Présent".into(),
                compound: false,
                forms: vec![PersonForm {
                    pronoun: String::new(),
                    form: infinitive_display.clone(),
                    text: infinitive_display.clone(),
                }],
            },
            Tense {
                id: "passe".into(),
                name: "Passé".into(),
                compound: true,
                forms: {
                    let t = format!("{} {pp_display}", refl_inf(aux_inf));
                    vec![PersonForm { pronoun: String::new(), form: t.clone(), text: t }]
                },
            },
        ],
    });

    let mut participles = Vec::new();
    if let Some(ppr) = &forms.present_participle {
        participles.push(NamedForm { id: "present".into(), name: "Participe présent".into(), form: refl_inf(ppr) });
    }
    let pp_all = {
        let mut v: Vec<&str> = forms.past_participle.iter().map(String::as_str).collect();
        v.dedup();
        let mut uniq: Vec<&str> = Vec::new();
        for f in v {
            if !uniq.contains(&f) {
                uniq.push(f);
            }
        }
        uniq.join(", ")
    };
    participles.push(NamedForm { id: "past".into(), name: "Participe passé".into(), form: pp_all });
    let aux_ppr = if etre { "étant" } else { "ayant" };
    participles.push(NamedForm {
        id: "past_compound".into(),
        name: "Participe passé composé".into(),
        form: format!("{} {pp_display}", refl_inf(aux_ppr)),
    });

    Ok(Conjugation {
        language: Language::Fr,
        infinitive: infinitive_display,
        group: (forms.group > 0).then_some(forms.group),
        model: forms.model.to_string(),
        auxiliary: aux_inf.to_string(),
        pronominal,
        known,
        participles,
        moods,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn present(verb: &str) -> Vec<String> {
        let f = simple_forms(verb, None).unwrap();
        (0..6).map(|p| f.get(SimpleTense::IndPresent, p).unwrap_or("-").to_string()).collect()
    }

    fn tense(verb: &str, t: SimpleTense) -> Vec<String> {
        let f = simple_forms(verb, None).unwrap();
        (0..6).map(|p| f.get(t, p).unwrap_or("-").to_string()).collect()
    }

    #[test]
    fn first_group_alternations() {
        assert_eq!(present("chanter"), ["chante", "chantes", "chante", "chantons", "chantez", "chantent"]);
        assert_eq!(present("manger")[3], "mangeons");
        assert_eq!(present("commencer")[3], "commençons");
        assert_eq!(present("appeler")[0], "appelle");
        assert_eq!(present("acheter")[0], "achète");
        assert_eq!(present("lever")[0], "lève");
        assert_eq!(present("céder")[0], "cède");
        assert_eq!(present("nettoyer")[0], "nettoie");
        assert_eq!(present("créer")[0], "crée");
        assert_eq!(present("alléguer")[0], "allègue");
        assert_eq!(present("complexer")[0], "complexe");
        assert_eq!(present("célébrer")[0], "célèbre");
        assert_eq!(present("déferler")[0], "déferle");
        assert_eq!(tense("appeler", SimpleTense::IndFutur)[0], "appellerai");
        assert_eq!(tense("céder", SimpleTense::IndFutur)[0], "céderai");
        assert_eq!(tense("lever", SimpleTense::IndFutur)[0], "lèverai");
        assert_eq!(tense("envoyer", SimpleTense::IndFutur)[0], "enverrai");
        assert_eq!(tense("manger", SimpleTense::IndPasseSimple)[0], "mangeai");
        assert_eq!(tense("étudier", SimpleTense::IndImparfait)[3], "étudiions");
    }

    #[test]
    fn second_group() {
        assert_eq!(present("finir"), ["finis", "finis", "finit", "finissons", "finissez", "finissent"]);
        assert_eq!(tense("finir", SimpleTense::SubjPresent)[0], "finisse");
        assert_eq!(present("haïr")[0], "hais");
        assert_eq!(present("haïr")[3], "haïssons");
    }

    #[test]
    fn irregular_verbs() {
        assert_eq!(present("être"), ["suis", "es", "est", "sommes", "êtes", "sont"]);
        assert_eq!(tense("être", SimpleTense::IndImparfait)[0], "étais");
        assert_eq!(tense("avoir", SimpleTense::IndPasseSimple)[2], "eut");
        assert_eq!(present("aller")[5], "vont");
        assert_eq!(tense("aller", SimpleTense::IndFutur)[0], "irai");
        assert_eq!(present("faire")[4], "faites");
        assert_eq!(tense("faire", SimpleTense::SubjPresent)[0], "fasse");
        assert_eq!(present("venir"), ["viens", "viens", "vient", "venons", "venez", "viennent"]);
        assert_eq!(tense("venir", SimpleTense::IndPasseSimple)[3], "vînmes");
        assert_eq!(tense("devenir", SimpleTense::IndFutur)[0], "deviendrai");
        assert_eq!(tense("prendre", SimpleTense::SubjPresent)[0], "prenne");
        assert_eq!(present("peindre")[3], "peignons");
        assert_eq!(present("conduire")[3], "conduisons");
        assert_eq!(present("connaître")[2], "connaît");
        assert_eq!(present("recevoir")[0], "reçois");
        assert_eq!(tense("recevoir", SimpleTense::IndPasseSimple)[0], "reçus");
        assert_eq!(present("ouvrir")[0], "ouvre");
        assert_eq!(present("partir")[0], "pars");
        assert_eq!(present("rendre")[2], "rend");
        assert_eq!(present("rompre")[2], "rompt");
        assert_eq!(present("vivre")[0], "vis");
        assert_eq!(tense("vivre", SimpleTense::IndPasseSimple)[0], "vécus");
        assert_eq!(present("mourir")[0], "meurs");
        assert_eq!(present("résoudre")[3], "résolvons");
        assert_eq!(present("dire")[4], "dites");
        assert_eq!(present("interdire")[4], "interdisez");
        assert_eq!(present("falloir")[2], "faut");
        assert_eq!(present("falloir")[0], "-");
        assert_eq!(present("vouloir")[5], "veulent");
        assert_eq!(tense("pouvoir", SimpleTense::SubjPresent)[3], "puissions");
        assert_eq!(simple_forms("devoir", None).unwrap().past_participle[1], "due");
        assert_eq!(simple_forms("prendre", None).unwrap().past_participle[3], "prises");
    }

    #[test]
    fn dictionary_hint_selects_group() {
        // "asservir" is a 2nd group verb even though it ends in -servir.
        let f = simple_forms("asservir", Some(true)).unwrap();
        assert_eq!(f.get(SimpleTense::IndPresent, 3), Some("asservissons"));
        assert!(simple_forms("gésir", Some(false)).is_none());
    }

    #[test]
    fn full_table() {
        let c = conjugate("aller", None, true).unwrap();
        assert_eq!(c.auxiliary, "être");
        let pc = &c.moods[0].tenses[1];
        assert_eq!(pc.forms[0].text, "je suis allé(e)");
        assert_eq!(pc.forms[3].text, "nous sommes allé(e)s");

        let c = conjugate("aimer", None, true).unwrap();
        assert_eq!(c.moods[0].tenses[0].forms[0].text, "j'aime");
        assert_eq!(c.moods[0].tenses[1].forms[0].text, "j'ai aimé");
        let subj = &c.moods[2].tenses[0];
        assert_eq!(subj.forms[2].text, "qu’il/elle aime");

        let c = conjugate("s’asseoir", None, true).unwrap();
        assert!(c.pronominal);
        assert_eq!(c.moods[0].tenses[0].forms[0].text, "je m'assieds");
        assert_eq!(c.moods[0].tenses[1].forms[2].text, "il/elle s'est assis(e)");

        let c = conjugate("haïr", None, true).unwrap();
        assert_eq!(c.moods[0].tenses[0].forms[0].text, "je hais");

        assert!(conjugate("table", Some(false), false).is_err());
    }
}
