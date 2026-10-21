//! Table-driven rules: fixed expressions that are wrong, redundant or improvable.

use super::{Ctx, Found, RuleDef};
use crate::text::{match_case, tokenize, TokenKind};
use crate::types::{Category, Language, Severity};
use std::collections::HashMap;
use std::sync::OnceLock;

/// (pattern, replacements separated by '|' or "-" for none, explanation)
type Entry = (&'static str, &'static str, &'static str);

struct PhraseSet {
    id: &'static str,
    language: Language,
    category: Category,
    severity: Severity,
    description: &'static str,
    default: bool,
    entries: &'static [Entry],
}

const FR_COMMON_ERRORS: &[Entry] = &[
    ("a cause de", "à cause de", "Préposition « à » (avec accent)."),
    ("a partir de", "à partir de", "Préposition « à » (avec accent)."),
    ("a travers", "à travers", "Préposition « à » (avec accent)."),
    ("a peu près", "à peu près", "Préposition « à » (avec accent)."),
    ("a côté de", "à côté de", "Préposition « à » (avec accent)."),
    ("grâce a", "grâce à", "Préposition « à » (avec accent)."),
    ("jusqu'a", "jusqu'à", "Préposition « à » (avec accent)."),
    ("face a", "face à", "Préposition « à » (avec accent)."),
    ("quant a", "quant à", "Préposition « à » (avec accent)."),
    ("par rapport a", "par rapport à", "Préposition « à » (avec accent)."),
    ("a bientôt", "à bientôt", "Préposition « à » (avec accent)."),
    ("a demain", "à demain", "Préposition « à » (avec accent)."),
    ("a propos de", "à propos de", "Préposition « à » (avec accent)."),
    ("a l'instar de", "à l'instar de", "Préposition « à » (avec accent)."),
    ("a moins que", "à moins que", "Préposition « à » (avec accent)."),
    ("tout a fait", "tout à fait", "Préposition « à » (avec accent)."),
    ("pallier à", "pallier", "« Pallier » est transitif direct : on pallie quelque chose."),
    ("pallier au", "pallier le", "« Pallier » est transitif direct : on pallie quelque chose."),
    ("pallier aux", "pallier les", "« Pallier » est transitif direct : on pallie quelque chose."),
    ("palier à", "pallier", "« Pallier » (deux l) est transitif direct."),
    ("malgré que", "bien que|quoique", "« Malgré que » est critiqué ; préférez « bien que »."),
    ("quelques temps", "quelque temps", "« Quelque temps » : « quelque » est ici invariable."),
    ("comme même", "quand même", "L’expression correcte est « quand même »."),
    ("en terme de", "en termes de", "« En termes de » s’écrit au pluriel."),
    ("un espèce de", "une espèce de", "« Espèce » est féminin : « une espèce de »."),
    ("rendus compte", "rendu compte", "Dans « se rendre compte », le participe reste invariable."),
    ("rendue compte", "rendu compte", "Dans « se rendre compte », le participe reste invariable."),
    ("rendues compte", "rendu compte", "Dans « se rendre compte », le participe reste invariable."),
    ("sensé être", "censé être", "« Censé » (supposé) s’écrit avec un c."),
    ("sensée être", "censée être", "« Censé » (supposé) s’écrit avec un c."),
    ("sensés être", "censés être", "« Censé » (supposé) s’écrit avec un c."),
    ("sensées être", "censées être", "« Censé » (supposé) s’écrit avec un c."),
    ("en tous cas", "en tout cas", "On écrit « en tout cas »."),
    ("va de soit", "va de soi", "On écrit « cela va de soi »."),
    ("sans dessus dessous", "sens dessus dessous", "On écrit « sens dessus dessous »."),
    ("soit-disant", "soi-disant", "« Soi-disant » s’écrit avec « soi »."),
    ("soit disant", "soi-disant", "« Soi-disant » s’écrit avec « soi »."),
    ("c'est de cela dont", "c'est de cela que", "« Dont » fait doublon avec « de » : « c’est de cela que »."),
    ("c'est de ça dont", "c'est de ça que", "« Dont » fait doublon avec « de » : « c’est de ça que »."),
    ("après qu'il soit", "après qu'il est|après qu'il a", "« Après que » est suivi de l’indicatif."),
    ("après qu'elle soit", "après qu'elle est|après qu'elle a", "« Après que » est suivi de l’indicatif."),
    ("après qu'il ait", "après qu'il a|après qu'il eut", "« Après que » est suivi de l’indicatif."),
    ("après qu'elle ait", "après qu'elle a|après qu'elle eut", "« Après que » est suivi de l’indicatif."),
    ("après qu'ils aient", "après qu'ils ont|après qu'ils eurent", "« Après que » est suivi de l’indicatif."),
    ("après qu'ils soient", "après qu'ils sont", "« Après que » est suivi de l’indicatif."),
    ("pour pas que", "-", "Tournure familière ; à l’écrit : « pour que … ne … pas »."),
    ("solutionner", "résoudre", "« Solutionner » est critiqué ; préférez « résoudre »."),
    ("au jour d'aujourd'hui", "aujourd'hui", "Pléonasme : « aujourd’hui » suffit."),
    ("de manière à ce que", "de manière que|de sorte que", "Tournure lourde : « de manière que »."),
    ("de façon à ce que", "de façon que|de sorte que", "Tournure lourde : « de façon que »."),
    ("se rappeler de ça", "se rappeler cela", "On se rappelle quelque chose (transitif direct)."),
    ("je m'en rappelle", "je m'en souviens|je me le rappelle", "« Se rappeler » est transitif direct."),
    ("tu t'en rappelles", "tu t'en souviens|tu te le rappelles", "« Se rappeler » est transitif direct."),
    ("aréoport", "aéroport", "Le mot s’écrit « aéroport »."),
    ("infractus", "infarctus", "Le mot s’écrit « infarctus »."),
    ("dilemne", "dilemme", "Le mot s’écrit « dilemme »."),
    ("rénumération", "rémunération", "Le mot s’écrit « rémunération »."),
    ("hormis que", "hormis le fait que", "« Hormis » ne se construit pas avec « que » seul."),
];

const FR_PLEONASMS: &[Entry] = &[
    ("monter en haut", "monter", "Pléonasme : monter, c’est aller en haut."),
    ("descendre en bas", "descendre", "Pléonasme : descendre, c’est aller en bas."),
    ("sortir dehors", "sortir", "Pléonasme : sortir, c’est aller dehors."),
    ("entrer dedans", "entrer", "Pléonasme : « entrer » suffit."),
    ("reculer en arrière", "reculer", "Pléonasme : reculer, c’est aller en arrière."),
    ("prévoir à l'avance", "prévoir", "Pléonasme : prévoir, c’est déjà à l’avance."),
    ("prévoir d'avance", "prévoir", "Pléonasme : prévoir, c’est déjà à l’avance."),
    ("préparer à l'avance", "préparer", "Pléonasme : « préparer » suffit."),
    ("prévenir à l'avance", "prévenir", "Pléonasme : « prévenir » suffit."),
    ("voire même", "voire|même", "Pléonasme : « voire » ou « même », pas les deux."),
    ("car en effet", "car|en effet", "Pléonasme : « car » ou « en effet », pas les deux."),
    ("comme par exemple", "par exemple|comme", "Pléonasme : « comme » ou « par exemple »."),
    ("puis ensuite", "puis|ensuite", "Pléonasme : « puis » ou « ensuite »."),
    ("mais cependant", "mais|cependant", "Pléonasme : « mais » ou « cependant »."),
    ("mais pourtant", "mais|pourtant", "Pléonasme : « mais » ou « pourtant »."),
    ("mais néanmoins", "mais|néanmoins", "Pléonasme : « mais » ou « néanmoins »."),
    ("enfin finalement", "enfin|finalement", "Pléonasme."),
    ("tous unanimes", "unanimes", "Pléonasme : « unanimes » implique « tous »."),
    ("collaborer ensemble", "collaborer", "Pléonasme : collaborer, c’est travailler ensemble."),
    ("réunir ensemble", "réunir", "Pléonasme : « réunir » suffit."),
    ("fusionner ensemble", "fusionner", "Pléonasme : « fusionner » suffit."),
    ("joindre ensemble", "joindre", "Pléonasme : « joindre » suffit."),
    ("ajouter en plus", "ajouter", "Pléonasme : « ajouter » suffit."),
    ("hasard imprévu", "hasard", "Pléonasme : un hasard est imprévu."),
    ("petite maisonnette", "maisonnette", "Pléonasme : une maisonnette est petite."),
    ("panacée universelle", "panacée", "Pléonasme : une panacée est universelle."),
    ("dune de sable", "dune", "Pléonasme : une dune est faite de sable."),
    ("marcher à pied", "marcher", "Pléonasme : « marcher » suffit."),
    ("applaudir des deux mains", "applaudir", "Pléonasme : « applaudir » suffit."),
    ("s'avérer vrai", "s'avérer|se révéler vrai", "Pléonasme : « avérer » signifie déjà « reconnaître pour vrai »."),
    ("répéter de nouveau", "répéter", "Pléonasme : « répéter » suffit."),
    ("répéter à nouveau", "répéter", "Pléonasme : « répéter » suffit."),
    ("suffisamment assez", "assez|suffisamment", "Pléonasme."),
    ("s'entraider mutuellement", "s'entraider", "Pléonasme : « s’entraider » est déjà mutuel."),
    ("monopole exclusif", "monopole", "Pléonasme : un monopole est exclusif."),
    ("bref résumé", "résumé", "Pléonasme : un résumé est bref."),
    ("prédire l'avenir", "prédire", "Pléonasme : « prédire » suffit."),
    ("erreur involontaire", "erreur", "Pléonasme : une erreur est involontaire."),
    ("reporter à plus tard", "reporter", "Pléonasme : « reporter » suffit."),
    ("ajourner à plus tard", "ajourner", "Pléonasme : « ajourner » suffit."),
    ("consensus général", "consensus", "Pléonasme : un consensus est général."),
    ("exporter à l'étranger", "exporter", "Pléonasme : « exporter » suffit."),
    ("au grand maximum", "au maximum", "Pléonasme."),
    ("deux jumeaux", "des jumeaux", "Pléonasme : des jumeaux sont deux."),
    ("faux prétexte", "prétexte", "Pléonasme : un prétexte est faux."),
    ("commencer d'abord", "commencer", "Pléonasme : « commencer » suffit."),
    ("comparer entre eux", "comparer", "Pléonasme : « comparer » suffit."),
    ("comparer ensemble", "comparer", "Pléonasme : « comparer » suffit."),
];

const FR_ANGLICISMS: &[Entry] = &[
    ("faire du sens", "avoir du sens", "Calque de l’anglais « to make sense »."),
    ("fait du sens", "a du sens", "Calque de l’anglais « to make sense »."),
    ("font du sens", "ont du sens", "Calque de l’anglais « to make sense »."),
    ("être en charge de", "être chargé de|être responsable de", "Calque de l’anglais « in charge of »."),
    ("est en charge de", "est chargé de|est responsable de", "Calque de l’anglais « in charge of »."),
    ("suis en charge de", "suis chargé de|suis responsable de", "Calque de l’anglais « in charge of »."),
    ("impacter", "toucher|affecter|influer sur", "Anglicisme critiqué."),
    ("impacte", "touche|affecte", "Anglicisme critiqué."),
    ("impactent", "touchent|affectent", "Anglicisme critiqué."),
    ("impacté", "touché|affecté", "Anglicisme critiqué."),
    ("impactée", "touchée|affectée", "Anglicisme critiqué."),
    ("impactés", "touchés|affectés", "Anglicisme critiqué."),
    ("impactées", "touchées|affectées", "Anglicisme critiqué."),
    ("deadline", "date limite|échéance", "Anglicisme."),
    ("deadlines", "dates limites|échéances", "Anglicisme."),
    ("feedback", "retour|avis", "Anglicisme."),
    ("feedbacks", "retours|avis", "Anglicisme."),
    ("meeting", "réunion", "Anglicisme."),
    ("meetings", "réunions", "Anglicisme."),
    ("challenge", "défi", "Anglicisme."),
    ("challenges", "défis", "Anglicisme."),
    ("booster", "stimuler|dynamiser", "Anglicisme."),
    ("checker", "vérifier", "Anglicisme."),
    ("forwarder", "transférer", "Anglicisme."),
    ("updater", "mettre à jour", "Anglicisme."),
    ("mettre l'emphase sur", "mettre l'accent sur", "Calque de l’anglais « to put emphasis on »."),
    ("best practices", "bonnes pratiques", "Anglicisme."),
    ("adresser le problème", "traiter le problème|aborder le problème", "Calque de l’anglais « to address »."),
    ("adresser ce problème", "traiter ce problème|aborder ce problème", "Calque de l’anglais « to address »."),
    ("adresser la question", "aborder la question|traiter la question", "Calque de l’anglais « to address »."),
    ("brainstorming", "remue-méninges", "Anglicisme."),
    ("follow-up", "suivi", "Anglicisme."),
    ("asap", "dès que possible", "Anglicisme."),
    ("e-mail", "courriel", "Anglicisme (recommandation officielle : « courriel »)."),
    ("email", "courriel", "Anglicisme (recommandation officielle : « courriel »)."),
    ("à date", "à ce jour|jusqu'à présent", "Calque de l’anglais « to date »."),
    ("initier un projet", "lancer un projet", "Calque de l’anglais « to initiate »."),
];

const FR_FAMILIAR: &[Entry] = &[
    ("du coup", "donc|par conséquent|alors", "Tournure orale ; à l’écrit, préférez « donc »."),
    (
        "au niveau de",
        "en ce qui concerne|pour",
        "« Au niveau de » est souvent employé à tort au sens de « en ce qui concerne ».",
    ),
    ("ceci dit", "cela dit", "On écrit plutôt « cela dit »."),
    ("pas mal de", "beaucoup de|de nombreux", "Tournure familière."),
    ("un max de", "beaucoup de", "Tournure familière."),
    ("trop bien", "très bien|excellent", "Tournure familière."),
    ("au final", "finalement|en fin de compte", "Tournure critiquée ; préférez « finalement »."),
    ("dans le but de", "afin de|pour", "Tournure lourde ; préférez « afin de »."),
    ("suite à", "à la suite de|après", "« Suite à » est critiqué hors du style administratif."),
    ("baser sur", "fonder sur", "On fonde un raisonnement sur quelque chose."),
    ("basé sur", "fondé sur", "On fonde un raisonnement sur quelque chose."),
    ("basée sur", "fondée sur", "On fonde un raisonnement sur quelque chose."),
];

const FR_INTENSIFIERS: &[Entry] = &[
    ("très grand", "immense|gigantesque", "Un mot plus fort remplace « très + adjectif »."),
    ("très grande", "immense|gigantesque", "Un mot plus fort remplace « très + adjectif »."),
    ("très petit", "minuscule|infime", "Un mot plus fort remplace « très + adjectif »."),
    ("très petite", "minuscule|infime", "Un mot plus fort remplace « très + adjectif »."),
    ("très bon", "excellent", "Un mot plus fort remplace « très + adjectif »."),
    ("très bonne", "excellente", "Un mot plus fort remplace « très + adjectif »."),
    ("très mauvais", "exécrable|déplorable", "Un mot plus fort remplace « très + adjectif »."),
    ("très mauvaise", "exécrable|déplorable", "Un mot plus fort remplace « très + adjectif »."),
    ("très beau", "magnifique|splendide", "Un mot plus fort remplace « très + adjectif »."),
    ("très belle", "magnifique|splendide", "Un mot plus fort remplace « très + adjectif »."),
    ("très content", "ravi|enchanté", "Un mot plus fort remplace « très + adjectif »."),
    ("très contente", "ravie|enchantée", "Un mot plus fort remplace « très + adjectif »."),
    ("très fatigué", "épuisé|exténué", "Un mot plus fort remplace « très + adjectif »."),
    ("très fatiguée", "épuisée|exténuée", "Un mot plus fort remplace « très + adjectif »."),
    ("très important", "essentiel|crucial", "Un mot plus fort remplace « très + adjectif »."),
    ("très importante", "essentielle|cruciale", "Un mot plus fort remplace « très + adjectif »."),
    ("très intéressant", "passionnant|captivant", "Un mot plus fort remplace « très + adjectif »."),
    ("très intéressante", "passionnante|captivante", "Un mot plus fort remplace « très + adjectif »."),
    ("très difficile", "ardu|redoutable", "Un mot plus fort remplace « très + adjectif »."),
    ("très froid", "glacial", "Un mot plus fort remplace « très + adjectif »."),
    ("très chaud", "brûlant|torride", "Un mot plus fort remplace « très + adjectif »."),
    ("très rapide", "fulgurant", "Un mot plus fort remplace « très + adjectif »."),
    ("très drôle", "hilarant|désopilant", "Un mot plus fort remplace « très + adjectif »."),
    ("très triste", "affligé|navré", "Un mot plus fort remplace « très + adjectif »."),
    ("très heureux", "comblé|radieux", "Un mot plus fort remplace « très + adjectif »."),
    ("très heureuse", "comblée|radieuse", "Un mot plus fort remplace « très + adjectif »."),
    ("très sale", "crasseux|répugnant", "Un mot plus fort remplace « très + adjectif »."),
    ("très riche", "fortuné|opulent", "Un mot plus fort remplace « très + adjectif »."),
];

const EN_COMMON_ERRORS: &[Entry] = &[
    ("could of", "could have", "“Could of” is a mishearing of “could have”."),
    ("would of", "would have", "“Would of” is a mishearing of “would have”."),
    ("should of", "should have", "“Should of” is a mishearing of “should have”."),
    ("must of", "must have", "“Must of” is a mishearing of “must have”."),
    ("might of", "might have", "“Might of” is a mishearing of “might have”."),
    ("more then", "more than", "Use “than” for comparisons."),
    ("less then", "less than", "Use “than” for comparisons."),
    ("better then", "better than", "Use “than” for comparisons."),
    ("worse then", "worse than", "Use “than” for comparisons."),
    ("rather then", "rather than", "Use “than” for comparisons."),
    ("other then", "other than", "Use “than” for comparisons."),
    ("greater then", "greater than", "Use “than” for comparisons."),
    ("your welcome", "you're welcome", "“You're” is the contraction of “you are”."),
    ("your going", "you're going", "“You're” is the contraction of “you are”."),
    ("your not", "you're not", "“You're” is the contraction of “you are”."),
    ("your being", "you're being", "“You're” is the contraction of “you are”."),
    ("their is", "there is", "Use “there” to state that something exists."),
    ("their are", "there are", "Use “there” to state that something exists."),
    ("their was", "there was", "Use “there” to state that something exists."),
    ("their were", "there were", "Use “there” to state that something exists."),
    ("its a", "it's a", "“It's” is the contraction of “it is”."),
    ("its been", "it's been", "“It's” is the contraction of “it has”."),
    ("its not", "it's not", "“It's” is the contraction of “it is”."),
    ("its going", "it's going", "“It's” is the contraction of “it is”."),
    ("it's own", "its own", "The possessive “its” has no apostrophe."),
    ("irregardless", "regardless", "“Irregardless” is nonstandard."),
    ("could care less", "couldn't care less", "The idiom is “couldn't care less”."),
    ("for all intensive purposes", "for all intents and purposes", "The idiom is “for all intents and purposes”."),
    ("one in the same", "one and the same", "The idiom is “one and the same”."),
    ("nip it in the butt", "nip it in the bud", "The idiom is “nip it in the bud”."),
    ("on accident", "by accident", "The standard phrase is “by accident”."),
    ("supposably", "supposedly", "Did you mean “supposedly”?"),
    ("between you and i", "between you and me", "After a preposition, use “me”."),
    ("loose weight", "lose weight", "“Lose” means to stop having; “loose” means not tight."),
    ("alot", "a lot", "“A lot” is two words."),
    ("a lot of times", "often", "Consider “often”."),
    ("less people", "fewer people", "Use “fewer” with countable nouns."),
    ("there car", "their car", "Possessive: “their”."),
];

const EN_WORDY: &[Entry] = &[
    ("in order to", "to", "Wordy: “to” is enough."),
    ("due to the fact that", "because", "Wordy phrase."),
    ("at this point in time", "now", "Wordy phrase."),
    ("at the present time", "now|currently", "Wordy phrase."),
    ("in the event that", "if", "Wordy phrase."),
    ("a large number of", "many", "Wordy phrase."),
    ("has the ability to", "can", "Wordy phrase."),
    ("have the ability to", "can", "Wordy phrase."),
    ("is able to", "can", "Wordy phrase."),
    ("for the purpose of", "for|to", "Wordy phrase."),
    ("in spite of the fact that", "although", "Wordy phrase."),
    ("despite the fact that", "although", "Wordy phrase."),
    ("with regard to", "about|regarding", "Wordy phrase."),
    ("in regard to", "about|regarding", "Wordy phrase."),
    ("each and every", "each|every", "Redundant pair."),
    ("first and foremost", "first", "Redundant pair."),
    ("in the near future", "soon", "Wordy phrase."),
    ("the majority of", "most", "Wordy phrase."),
    ("in close proximity to", "near", "Wordy phrase."),
    ("until such time as", "until", "Wordy phrase."),
    ("make a decision", "decide", "Prefer the verb."),
    ("take into consideration", "consider", "Prefer the verb."),
    ("end result", "result", "Redundant: a result is an end."),
    ("past history", "history", "Redundant."),
    ("free gift", "gift", "Redundant: gifts are free."),
    ("added bonus", "bonus", "Redundant."),
    ("basic fundamentals", "fundamentals", "Redundant."),
    ("close proximity", "proximity", "Redundant."),
    ("future plans", "plans", "Redundant."),
    ("unexpected surprise", "surprise", "Redundant."),
    ("revert back", "revert", "Redundant."),
    ("return back", "return", "Redundant."),
    ("repeat again", "repeat", "Redundant."),
    ("absolutely essential", "essential", "Redundant."),
    ("completely finished", "finished", "Redundant."),
    ("it is important to note that", "", "Filler phrase: state the point directly."),
];

const EN_INTENSIFIERS: &[Entry] = &[
    ("very good", "excellent", "A stronger word can replace “very + adjective”."),
    ("very bad", "terrible|awful", "A stronger word can replace “very + adjective”."),
    ("very big", "huge|enormous", "A stronger word can replace “very + adjective”."),
    ("very large", "enormous|vast", "A stronger word can replace “very + adjective”."),
    ("very small", "tiny|minute", "A stronger word can replace “very + adjective”."),
    ("very happy", "delighted|thrilled", "A stronger word can replace “very + adjective”."),
    ("very sad", "miserable|heartbroken", "A stronger word can replace “very + adjective”."),
    ("very tired", "exhausted", "A stronger word can replace “very + adjective”."),
    ("very important", "crucial|essential", "A stronger word can replace “very + adjective”."),
    ("very interesting", "fascinating", "A stronger word can replace “very + adjective”."),
    ("very difficult", "arduous|daunting", "A stronger word can replace “very + adjective”."),
    ("very cold", "freezing", "A stronger word can replace “very + adjective”."),
    ("very hot", "scorching", "A stronger word can replace “very + adjective”."),
    ("very fast", "rapid|swift", "A stronger word can replace “very + adjective”."),
    ("very old", "ancient", "A stronger word can replace “very + adjective”."),
    ("very beautiful", "gorgeous|stunning", "A stronger word can replace “very + adjective”."),
    ("very angry", "furious", "A stronger word can replace “very + adjective”."),
    ("very scared", "terrified", "A stronger word can replace “very + adjective”."),
    ("very hungry", "starving", "A stronger word can replace “very + adjective”."),
    ("very smart", "brilliant", "A stronger word can replace “very + adjective”."),
    ("very funny", "hilarious", "A stronger word can replace “very + adjective”."),
    ("very clean", "spotless", "A stronger word can replace “very + adjective”."),
    ("very dirty", "filthy", "A stronger word can replace “very + adjective”."),
    ("very quiet", "silent", "A stronger word can replace “very + adjective”."),
    ("very loud", "deafening", "A stronger word can replace “very + adjective”."),
    ("very rich", "wealthy", "A stronger word can replace “very + adjective”."),
    ("very easy", "effortless", "A stronger word can replace “very + adjective”."),
];

static SETS: &[PhraseSet] = &[
    PhraseSet {
        id: "FR_COMMON_ERRORS",
        language: Language::Fr,
        category: Category::Grammar,
        severity: Severity::Error,
        description: "Fautes courantes et expressions figées (« malgré que », « pallier à », « a cause de »…)",
        default: true,
        entries: FR_COMMON_ERRORS,
    },
    PhraseSet {
        id: "FR_PLEONASMS",
        language: Language::Fr,
        category: Category::Style,
        severity: Severity::Hint,
        description: "Pléonasmes (« monter en haut », « voire même »…)",
        default: true,
        entries: FR_PLEONASMS,
    },
    PhraseSet {
        id: "FR_ANGLICISMS",
        language: Language::Fr,
        category: Category::Style,
        severity: Severity::Hint,
        description: "Anglicismes et calques (« faire du sens », « impacter »…)",
        default: true,
        entries: FR_ANGLICISMS,
    },
    PhraseSet {
        id: "FR_FAMILIAR",
        language: Language::Fr,
        category: Category::Style,
        severity: Severity::Hint,
        description: "Tournures familières ou critiquées (« du coup », « au niveau de »…)",
        default: true,
        entries: FR_FAMILIAR,
    },
    PhraseSet {
        id: "FR_INTENSIFIERS",
        language: Language::Fr,
        category: Category::Style,
        severity: Severity::Hint,
        description: "« très + adjectif » remplaçable par un mot plus fort",
        default: false,
        entries: FR_INTENSIFIERS,
    },
    PhraseSet {
        id: "EN_COMMON_ERRORS",
        language: Language::En,
        category: Category::Grammar,
        severity: Severity::Error,
        description: "Common mistakes and confused words (“could of”, “more then”…)",
        default: true,
        entries: EN_COMMON_ERRORS,
    },
    PhraseSet {
        id: "EN_WORDY",
        language: Language::En,
        category: Category::Style,
        severity: Severity::Hint,
        description: "Wordy or redundant phrases (“in order to”, “end result”…)",
        default: true,
        entries: EN_WORDY,
    },
    PhraseSet {
        id: "EN_INTENSIFIERS",
        language: Language::En,
        category: Category::Style,
        severity: Severity::Hint,
        description: "“very + adjective” that a stronger word can replace",
        default: false,
        entries: EN_INTENSIFIERS,
    },
];

/// Compiled patterns per set: first word → list of (word sequence, entry index).
type Compiled = HashMap<String, Vec<(Vec<String>, usize)>>;

fn compiled(set_index: usize) -> &'static Compiled {
    static CACHE: OnceLock<Vec<Compiled>> = OnceLock::new();
    &CACHE.get_or_init(|| {
        SETS.iter()
            .map(|set| {
                let mut map: Compiled = HashMap::new();
                for (i, (pattern, _, _)) in set.entries.iter().enumerate() {
                    let words: Vec<String> = tokenize(pattern, set.language)
                        .into_iter()
                        .filter(|t| t.kind == TokenKind::Word)
                        .map(|t| t.lower.replace('’', "'"))
                        .collect();
                    if let Some(first) = words.first().cloned() {
                        map.entry(first).or_default().push((words, i));
                    }
                }
                map
            })
            .collect()
    })[set_index]
}

macro_rules! set_checks {
    ($($i:literal),*) => {
        [$(|ctx: &Ctx<'_>, out: &mut Vec<Found>| check_set($i, ctx, out)),*]
    };
}

static CHECKS: [super::CheckFn; 8] = set_checks!(0, 1, 2, 3, 4, 5, 6, 7);

pub fn rules() -> Vec<RuleDef> {
    SETS.iter()
        .enumerate()
        .map(|(i, s)| RuleDef {
            id: s.id,
            language: Some(s.language),
            category: s.category,
            description: s.description,
            enabled_by_default: s.default,
            check: CHECKS[i],
        })
        .collect()
}

fn check_set(index: usize, ctx: &Ctx<'_>, out: &mut Vec<Found>) {
    let set = &SETS[index];
    let patterns = compiled(index);
    for chunk in &ctx.chunks {
        let words: Vec<String> = chunk.iter().map(|&i| ctx.tokens[i].lower.replace('’', "'")).collect();
        let mut k = 0;
        while k < words.len() {
            let mut matched = 0;
            if let Some(candidates) = patterns.get(&words[k]) {
                for (seq, entry) in candidates {
                    if k + seq.len() <= words.len() && words[k..k + seq.len()] == seq[..] {
                        let (_, repl, msg) = set.entries[*entry];
                        let first = ctx.tokens[chunk[k]].text;
                        let span = ctx.span(chunk[k], chunk[k + seq.len() - 1]);
                        let apostrophe = ctx.apostrophe();
                        let replacements = repl
                            .split('|')
                            .filter(|r| *r != "-")
                            .map(|r| match_case(first, &r.replace('\'', &apostrophe.to_string())))
                            .collect();
                        out.push(Found::new(set.severity, msg, span, replacements));
                        matched = seq.len();
                        break;
                    }
                }
            }
            k += matched.max(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_table_matches_sets() {
        assert_eq!(CHECKS.len(), SETS.len());
    }
}
