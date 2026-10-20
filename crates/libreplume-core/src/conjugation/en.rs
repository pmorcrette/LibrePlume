//! English conjugation: irregular verb table plus regular spelling rules.

use super::{Conjugation, ConjugationError, Mood, NamedForm, PersonForm, Tense};
use crate::types::Language;

pub const PRONOUNS: [&str; 6] = ["I", "you", "he/she/it", "we", "you", "they"];

/// (base, past simple, past participle)
pub const IRREGULAR: &[(&str, &str, &str)] = &[
    ("arise", "arose", "arisen"),
    ("awake", "awoke", "awoken"),
    ("be", "was", "been"),
    ("bear", "bore", "borne"),
    ("beat", "beat", "beaten"),
    ("become", "became", "become"),
    ("begin", "began", "begun"),
    ("bend", "bent", "bent"),
    ("bet", "bet", "bet"),
    ("bind", "bound", "bound"),
    ("bite", "bit", "bitten"),
    ("bleed", "bled", "bled"),
    ("blow", "blew", "blown"),
    ("break", "broke", "broken"),
    ("breed", "bred", "bred"),
    ("bring", "brought", "brought"),
    ("build", "built", "built"),
    ("burn", "burnt", "burnt"),
    ("burst", "burst", "burst"),
    ("buy", "bought", "bought"),
    ("catch", "caught", "caught"),
    ("choose", "chose", "chosen"),
    ("cling", "clung", "clung"),
    ("come", "came", "come"),
    ("cost", "cost", "cost"),
    ("creep", "crept", "crept"),
    ("cut", "cut", "cut"),
    ("deal", "dealt", "dealt"),
    ("dig", "dug", "dug"),
    ("do", "did", "done"),
    ("draw", "drew", "drawn"),
    ("dream", "dreamt", "dreamt"),
    ("drink", "drank", "drunk"),
    ("drive", "drove", "driven"),
    ("eat", "ate", "eaten"),
    ("fall", "fell", "fallen"),
    ("feed", "fed", "fed"),
    ("feel", "felt", "felt"),
    ("fight", "fought", "fought"),
    ("find", "found", "found"),
    ("flee", "fled", "fled"),
    ("fly", "flew", "flown"),
    ("forbid", "forbade", "forbidden"),
    ("forget", "forgot", "forgotten"),
    ("forgive", "forgave", "forgiven"),
    ("freeze", "froze", "frozen"),
    ("get", "got", "got"),
    ("give", "gave", "given"),
    ("go", "went", "gone"),
    ("grind", "ground", "ground"),
    ("grow", "grew", "grown"),
    ("hang", "hung", "hung"),
    ("have", "had", "had"),
    ("hear", "heard", "heard"),
    ("hide", "hid", "hidden"),
    ("hit", "hit", "hit"),
    ("hold", "held", "held"),
    ("hurt", "hurt", "hurt"),
    ("keep", "kept", "kept"),
    ("kneel", "knelt", "knelt"),
    ("know", "knew", "known"),
    ("lay", "laid", "laid"),
    ("lead", "led", "led"),
    ("lean", "leant", "leant"),
    ("leap", "leapt", "leapt"),
    ("learn", "learnt", "learnt"),
    ("leave", "left", "left"),
    ("lend", "lent", "lent"),
    ("let", "let", "let"),
    ("lie", "lay", "lain"),
    ("light", "lit", "lit"),
    ("lose", "lost", "lost"),
    ("make", "made", "made"),
    ("mean", "meant", "meant"),
    ("meet", "met", "met"),
    ("pay", "paid", "paid"),
    ("put", "put", "put"),
    ("quit", "quit", "quit"),
    ("read", "read", "read"),
    ("ride", "rode", "ridden"),
    ("ring", "rang", "rung"),
    ("rise", "rose", "risen"),
    ("run", "ran", "run"),
    ("say", "said", "said"),
    ("see", "saw", "seen"),
    ("seek", "sought", "sought"),
    ("sell", "sold", "sold"),
    ("send", "sent", "sent"),
    ("set", "set", "set"),
    ("sew", "sewed", "sewn"),
    ("shake", "shook", "shaken"),
    ("shine", "shone", "shone"),
    ("shoot", "shot", "shot"),
    ("show", "showed", "shown"),
    ("shrink", "shrank", "shrunk"),
    ("shut", "shut", "shut"),
    ("sing", "sang", "sung"),
    ("sink", "sank", "sunk"),
    ("sit", "sat", "sat"),
    ("sleep", "slept", "slept"),
    ("slide", "slid", "slid"),
    ("speak", "spoke", "spoken"),
    ("speed", "sped", "sped"),
    ("spend", "spent", "spent"),
    ("spin", "spun", "spun"),
    ("spit", "spat", "spat"),
    ("split", "split", "split"),
    ("spread", "spread", "spread"),
    ("spring", "sprang", "sprung"),
    ("stand", "stood", "stood"),
    ("steal", "stole", "stolen"),
    ("stick", "stuck", "stuck"),
    ("sting", "stung", "stung"),
    ("stink", "stank", "stunk"),
    ("strike", "struck", "struck"),
    ("swear", "swore", "sworn"),
    ("sweep", "swept", "swept"),
    ("swim", "swam", "swum"),
    ("swing", "swung", "swung"),
    ("take", "took", "taken"),
    ("teach", "taught", "taught"),
    ("tear", "tore", "torn"),
    ("tell", "told", "told"),
    ("think", "thought", "thought"),
    ("throw", "threw", "thrown"),
    ("understand", "understood", "understood"),
    ("undertake", "undertook", "undertaken"),
    ("upset", "upset", "upset"),
    ("wake", "woke", "woken"),
    ("wear", "wore", "worn"),
    ("weave", "wove", "woven"),
    ("weep", "wept", "wept"),
    ("win", "won", "won"),
    ("wind", "wound", "wound"),
    ("withdraw", "withdrew", "withdrawn"),
    ("write", "wrote", "written"),
    ("overcome", "overcame", "overcome"),
    ("mistake", "mistook", "mistaken"),
    ("misunderstand", "misunderstood", "misunderstood"),
    ("rewrite", "rewrote", "rewritten"),
    ("broadcast", "broadcast", "broadcast"),
    ("forecast", "forecast", "forecast"),
    ("outdo", "outdid", "outdone"),
    ("overtake", "overtook", "overtaken"),
];

/// Frequent regular verbs, used by the grammar rules to recognise base forms.
pub const COMMON_REGULAR: &[&str] = &[
    "accept", "add", "agree", "allow", "answer", "appear", "arrive", "ask", "believe", "belong", "call", "change",
    "check", "clean", "close", "complain", "contain", "cook", "cry", "dance", "decide", "depend", "describe", "die",
    "enjoy", "explain", "fail", "fill", "finish", "follow", "happen", "hate", "help", "hope", "improve", "include",
    "invite", "join", "jump", "kill", "kiss", "laugh", "like", "listen", "live", "look", "love", "manage", "matter",
    "miss", "move", "need", "offer", "open", "order", "own", "play", "prefer", "prepare", "promise", "pull", "push",
    "reach", "realize", "receive", "remember", "rain", "seem", "share", "smile", "start", "stay", "stop", "study",
    "suggest", "talk", "thank", "touch", "travel", "try", "turn", "use", "visit", "wait", "walk", "want", "wash",
    "watch", "wish", "work", "worry", "carry", "create", "develop", "provide", "require", "support", "learn", "mean",
    "seem", "plan", "agree", "admit",
];

/// Verbs that double their final consonant despite having several syllables.
const DOUBLING: &[&str] = &[
    "admit", "begin", "commit", "control", "occur", "prefer", "refer", "regret", "forget", "permit", "submit",
    "transfer", "omit", "equip", "upset", "compel", "expel", "propel", "patrol", "rebel", "acquit", "deter", "incur",
    "recur", "infer", "confer", "defer",
];

pub fn irregular(base: &str) -> Option<(&'static str, &'static str)> {
    IRREGULAR.iter().find(|(b, _, _)| *b == base).map(|&(_, p, pp)| (p, pp))
}

pub fn is_known_verb(base: &str) -> bool {
    irregular(base).is_some() || COMMON_REGULAR.contains(&base)
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn doubles_final(base: &str) -> bool {
    if DOUBLING.contains(&base) {
        return true;
    }
    let c: Vec<char> = base.chars().collect();
    let n = c.len();
    if n < 3 {
        return false;
    }
    let vowel_groups = c.iter().enumerate().filter(|&(i, &ch)| is_vowel(ch) && (i == 0 || !is_vowel(c[i - 1]))).count();
    vowel_groups == 1
        && !is_vowel(c[n - 1])
        && !matches!(c[n - 1], 'w' | 'x' | 'y')
        && is_vowel(c[n - 2])
        && !is_vowel(c[n - 3])
}

pub fn third_person(base: &str) -> String {
    match base {
        "be" => return "is".into(),
        "have" => return "has".into(),
        "do" => return "does".into(),
        "go" => return "goes".into(),
        _ => {}
    }
    let chars: Vec<char> = base.chars().collect();
    let n = chars.len();
    if base.ends_with('s')
        || base.ends_with('x')
        || base.ends_with('z')
        || base.ends_with("ch")
        || base.ends_with("sh")
        || base.ends_with('o')
    {
        format!("{base}es")
    } else if n >= 2 && chars[n - 1] == 'y' && !is_vowel(chars[n - 2]) {
        format!("{}ies", &base[..base.len() - 1])
    } else {
        format!("{base}s")
    }
}

pub fn present_participle(base: &str) -> String {
    if base == "be" {
        return "being".into();
    }
    if let Some(stem) = base.strip_suffix("ie") {
        return format!("{stem}ying");
    }
    if base.ends_with('e') && !base.ends_with("ee") && !base.ends_with("ye") && !base.ends_with("oe") && base.len() > 2
    {
        return format!("{}ing", &base[..base.len() - 1]);
    }
    if doubles_final(base) {
        let last = base.chars().last().unwrap();
        return format!("{base}{last}ing");
    }
    format!("{base}ing")
}

pub fn regular_past(base: &str) -> String {
    let chars: Vec<char> = base.chars().collect();
    let n = chars.len();
    if base.ends_with('e') {
        format!("{base}d")
    } else if n >= 2 && chars[n - 1] == 'y' && !is_vowel(chars[n - 2]) {
        format!("{}ied", &base[..base.len() - 1])
    } else if doubles_final(base) {
        format!("{base}{}ed", chars[n - 1])
    } else {
        format!("{base}ed")
    }
}

pub fn past(base: &str) -> String {
    irregular(base).map_or_else(|| regular_past(base), |(p, _)| p.to_string())
}

pub fn past_participle(base: &str) -> String {
    irregular(base).map_or_else(|| regular_past(base), |(_, pp)| pp.to_string())
}

fn be_present(p: usize) -> &'static str {
    match p {
        0 => "am",
        2 => "is",
        _ => "are",
    }
}

fn be_past(p: usize) -> &'static str {
    if p == 0 || p == 2 {
        "was"
    } else {
        "were"
    }
}

fn have_present(p: usize) -> &'static str {
    if p == 2 {
        "has"
    } else {
        "have"
    }
}

pub fn conjugate(input: &str) -> Result<Conjugation, ConjugationError> {
    let mut base = input.trim().to_lowercase();
    if let Some(rest) = base.strip_prefix("to ") {
        base = rest.trim().to_string();
    }
    if base.is_empty() {
        return Err(ConjugationError::Empty);
    }
    if !base.chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
        return Err(ConjugationError::NotAVerb(base));
    }
    let b = base.as_str();
    let s3 = third_person(b);
    let pst = past(b);
    let pp = past_participle(b);
    let ing = present_participle(b);

    let tense = |id: &str, name: &str, compound: bool, f: &dyn Fn(usize) -> String| -> Tense {
        Tense {
            id: id.into(),
            name: name.into(),
            compound,
            forms: (0..6)
                .map(|p| {
                    let form = f(p);
                    PersonForm { pronoun: PRONOUNS[p].into(), text: format!("{} {form}", PRONOUNS[p]), form }
                })
                .collect(),
        }
    };

    let present_simple = |p: usize| -> String {
        if b == "be" {
            be_present(p).into()
        } else if p == 2 {
            s3.clone()
        } else {
            b.to_string()
        }
    };
    let past_simple = |p: usize| -> String {
        if b == "be" {
            be_past(p).into()
        } else {
            pst.clone()
        }
    };

    let indicative = vec![
        tense("present", "Present simple", false, &present_simple),
        tense("present_continuous", "Present continuous", true, &|p| format!("{} {ing}", be_present(p))),
        tense("present_perfect", "Present perfect", true, &|p| format!("{} {pp}", have_present(p))),
        tense("present_perfect_continuous", "Present perfect continuous", true, &|p| {
            format!("{} been {ing}", have_present(p))
        }),
        tense("past", "Past simple", false, &past_simple),
        tense("past_continuous", "Past continuous", true, &|p| format!("{} {ing}", be_past(p))),
        tense("past_perfect", "Past perfect", true, &|_| format!("had {pp}")),
        tense("past_perfect_continuous", "Past perfect continuous", true, &|_| format!("had been {ing}")),
        tense("future", "Future simple", true, &|_| format!("will {b}")),
        tense("future_continuous", "Future continuous", true, &|_| format!("will be {ing}")),
        tense("future_perfect", "Future perfect", true, &|_| format!("will have {pp}")),
        tense("future_perfect_continuous", "Future perfect continuous", true, &|_| format!("will have been {ing}")),
    ];
    let conditional = vec![
        tense("present", "Present", true, &|_| format!("would {b}")),
        tense("continuous", "Continuous", true, &|_| format!("would be {ing}")),
        tense("perfect", "Perfect", true, &|_| format!("would have {pp}")),
        tense("perfect_continuous", "Perfect continuous", true, &|_| format!("would have been {ing}")),
    ];
    let subjunctive = vec![
        tense("present", "Present", false, &|_| b.to_string()),
        tense("past", "Past", false, &|_| if b == "be" { "were".into() } else { pst.clone() }),
    ];
    let imperative = Tense {
        id: "present".into(),
        name: "Present".into(),
        compound: false,
        forms: vec![
            PersonForm { pronoun: String::new(), form: b.to_string(), text: b.to_string() },
            PersonForm { pronoun: String::new(), form: format!("let's {b}"), text: format!("let's {b}") },
            PersonForm { pronoun: String::new(), form: format!("do not {b}"), text: format!("do not {b}") },
        ],
    };
    let one = |id: &str, name: &str, f: String| Tense {
        id: id.into(),
        name: name.into(),
        compound: false,
        forms: vec![PersonForm { pronoun: String::new(), form: f.clone(), text: f }],
    };
    let infinitive = vec![
        one("present", "Present", format!("to {b}")),
        one("perfect", "Perfect", format!("to have {pp}")),
        one("continuous", "Continuous", format!("to be {ing}")),
    ];

    Ok(Conjugation {
        language: Language::En,
        infinitive: format!("to {b}"),
        group: None,
        model: if irregular(b).is_some() { "irregular".into() } else { "regular".into() },
        auxiliary: "have".into(),
        pronominal: false,
        known: is_known_verb(b),
        participles: vec![
            NamedForm { id: "present".into(), name: "Present participle".into(), form: ing.clone() },
            NamedForm { id: "past".into(), name: "Past participle".into(), form: pp.clone() },
        ],
        moods: vec![
            Mood { id: "indicative".into(), name: "Indicative".into(), tenses: indicative },
            Mood { id: "conditional".into(), name: "Conditional".into(), tenses: conditional },
            Mood { id: "subjunctive".into(), name: "Subjunctive".into(), tenses: subjunctive },
            Mood { id: "imperative".into(), name: "Imperative".into(), tenses: vec![imperative] },
            Mood { id: "infinitive".into(), name: "Infinitive".into(), tenses: infinitive },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spelling_rules() {
        assert_eq!(third_person("watch"), "watches");
        assert_eq!(third_person("study"), "studies");
        assert_eq!(third_person("play"), "plays");
        assert_eq!(present_participle("stop"), "stopping");
        assert_eq!(present_participle("make"), "making");
        assert_eq!(present_participle("die"), "dying");
        assert_eq!(present_participle("begin"), "beginning");
        assert_eq!(present_participle("open"), "opening");
        assert_eq!(past("try"), "tried");
        assert_eq!(past("go"), "went");
        assert_eq!(past("stop"), "stopped");
        assert_eq!(past("visit"), "visited");
        assert_eq!(past_participle("write"), "written");
    }

    #[test]
    fn table() {
        let c = conjugate("to be").unwrap();
        assert_eq!(c.moods[0].tenses[0].forms[0].text, "I am");
        assert_eq!(c.moods[0].tenses[4].forms[3].text, "we were");
        let c = conjugate("walk").unwrap();
        assert_eq!(c.moods[0].tenses[2].forms[2].text, "he/she/it has walked");
    }
}
