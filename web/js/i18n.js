// Interface translations (French and English).

const STRINGS = {
  fr: {
    skip: 'Aller au contenu',
    'nav.check': 'Correcteur', 'nav.conjugate': 'Conjugueur', 'nav.rules': 'Réglages', 'nav.themes': 'Thèmes', 'nav.api': 'API & MCP',
    'theme.quick': 'Changer de thème',
    'lang.auto': 'Détection auto',
    'check.language': 'Langue du texte', 'check.copy': 'Copier', 'check.clear': 'Effacer', 'check.fixAll': 'Tout corriger',
    'check.placeholder': 'Écrivez ou collez votre texte ici…',
    'check.issues': 'Suggestions',
    'check.empty': 'Votre texte apparaîtra ici, analysé au fil de la frappe.',
    'check.perfect': 'Aucune faute détectée. Bravo !',
    'check.count': (n) => n === 1 ? '1 point à revoir' : `${n} points à revoir`,
    'check.checking': 'Analyse…', 'check.checked': 'À jour', 'check.offline': 'Serveur injoignable',
    'check.detected': (l) => `Détecté : ${l}`,
    'check.words': (n) => n <= 1 ? `${n} mot` : `${n} mots`,
    'check.sentences': (n) => n <= 1 ? `${n} phrase` : `${n} phrases`,
    'check.reading': (s) => s < 60 ? `${s} s de lecture` : `${Math.round(s / 60)} min de lecture`,
    'check.copied': 'Texte copié', 'check.fixed': (n) => n === 1 ? '1 correction appliquée' : `${n} corrections appliquées`,
    'check.ignore': 'Ignorer', 'check.addWord': 'Ajouter au dictionnaire', 'check.disableRule': 'Désactiver la règle',
    'check.remove': 'supprimer', 'check.wordAdded': (w) => `« ${w} » ajouté au dictionnaire`, 'check.ruleDisabled': 'Règle désactivée',
    'check.score.great': 'Texte très soigné', 'check.score.good': 'Quelques retouches', 'check.score.work': 'À relire attentivement',
    'cat.spelling': 'Orthographe', 'cat.grammar': 'Grammaire', 'cat.conjugation': 'Conjugaison',
    'cat.typography': 'Typographie', 'cat.punctuation': 'Ponctuation', 'cat.style': 'Style',
    'sev.error': 'Erreur', 'sev.warning': 'Avertissement', 'sev.hint': 'Suggestion',
    'conj.placeholder': 'Un verbe à l’infinitif : venir, s’asseoir, to go…', 'conj.submit': 'Conjuguer',
    'conj.group': (g) => `${g}${g === 1 ? 'er' : 'e'} groupe`, 'conj.aux': (a) => `auxiliaire ${a}`, 'conj.model': (m) => `modèle : ${m}`,
    'conj.unknown': 'Verbe absent du dictionnaire : conjugaison déduite de sa terminaison.',
    'conj.pronominal': 'pronominal', 'conj.compound': 'composé',
    'settings.title': 'Réglages généraux', 'settings.shared': 'Ces réglages s’appliquent à tous les utilisateurs de ce serveur.',
    'settings.defaultLang': 'Langue par défaut', 'settings.maxWords': 'Longueur maximale d’une phrase',
    'settings.off': 'désactivé', 'settings.words': (n) => `${n} mots`, 'settings.categories': 'Catégories vérifiées', 'settings.saved': 'Réglages enregistrés',
    'dict.title': 'Dictionnaire personnel', 'dict.help': 'Noms propres, jargon, néologismes : ces mots ne seront plus signalés.',
    'dict.placeholder': 'Ajouter un mot…', 'dict.add': 'Ajouter', 'dict.empty': 'Aucun mot pour l’instant.',
    'rules.title': 'Règles', 'rules.filter': 'Filtrer les règles…', 'rules.allLangs': 'Toutes les langues',
    'themes.title': 'Thèmes', 'themes.auto': 'Suivre le mode clair/sombre du système', 'themes.editor': 'Créer un thème',
    'themes.import': 'Importer', 'themes.export': 'Exporter', 'themes.name': 'Nom', 'themes.base': 'À partir de',
    'themes.fontText': 'Police du texte', 'themes.fontUi': 'Police de l’interface', 'themes.size': 'Taille du texte', 'themes.radius': 'Arrondis',
    'themes.dark': 'Thème sombre', 'themes.try': 'Essayer', 'themes.save': 'Enregistrer',
    'themes.saved': 'Thème enregistré', 'themes.deleted': 'Thème supprimé', 'themes.default': 'Par défaut', 'themes.makeDefault': 'Définir par défaut',
    'themes.custom': 'personnalisé', 'themes.edit': 'Modifier', 'themes.delete': 'Supprimer', 'themes.light': 'clair', 'themes.darkShort': 'sombre',
    'themes.previewText': 'Le chat dort sur le <u>canapé</u>.', 'themes.invalid': 'Fichier de thème invalide',
    'token.title': 'Jeton d’accès requis', 'token.help': 'Ce serveur protège les modifications par un jeton (LIBREPLUME_TOKEN).', 'token.save': 'Enregistrer',
    cancel: 'Annuler', error: 'Erreur',
  },
  en: {
    skip: 'Skip to content',
    'nav.check': 'Checker', 'nav.conjugate': 'Conjugator', 'nav.rules': 'Settings', 'nav.themes': 'Themes', 'nav.api': 'API & MCP',
    'theme.quick': 'Switch theme',
    'lang.auto': 'Auto-detect',
    'check.language': 'Text language', 'check.copy': 'Copy', 'check.clear': 'Clear', 'check.fixAll': 'Fix all',
    'check.placeholder': 'Write or paste your text here…',
    'check.issues': 'Suggestions',
    'check.empty': 'Your text will be analysed here as you type.',
    'check.perfect': 'No mistakes found. Well done!',
    'check.count': (n) => n === 1 ? '1 thing to review' : `${n} things to review`,
    'check.checking': 'Checking…', 'check.checked': 'Up to date', 'check.offline': 'Server unreachable',
    'check.detected': (l) => `Detected: ${l}`,
    'check.words': (n) => n === 1 ? '1 word' : `${n} words`,
    'check.sentences': (n) => n === 1 ? '1 sentence' : `${n} sentences`,
    'check.reading': (s) => s < 60 ? `${s} s read` : `${Math.round(s / 60)} min read`,
    'check.copied': 'Text copied', 'check.fixed': (n) => n === 1 ? '1 fix applied' : `${n} fixes applied`,
    'check.ignore': 'Ignore', 'check.addWord': 'Add to dictionary', 'check.disableRule': 'Disable rule',
    'check.remove': 'remove', 'check.wordAdded': (w) => `“${w}” added to the dictionary`, 'check.ruleDisabled': 'Rule disabled',
    'check.score.great': 'Polished text', 'check.score.good': 'A few touch-ups', 'check.score.work': 'Needs a careful review',
    'cat.spelling': 'Spelling', 'cat.grammar': 'Grammar', 'cat.conjugation': 'Conjugation',
    'cat.typography': 'Typography', 'cat.punctuation': 'Punctuation', 'cat.style': 'Style',
    'sev.error': 'Error', 'sev.warning': 'Warning', 'sev.hint': 'Suggestion',
    'conj.placeholder': 'An infinitive: venir, s’asseoir, to go…', 'conj.submit': 'Conjugate',
    'conj.group': (g) => `group ${g}`, 'conj.aux': (a) => `auxiliary ${a}`, 'conj.model': (m) => `model: ${m}`,
    'conj.unknown': 'Verb missing from the dictionary: conjugation inferred from its ending.',
    'conj.pronominal': 'pronominal', 'conj.compound': 'compound',
    'settings.title': 'General settings', 'settings.shared': 'These settings apply to every user of this server.',
    'settings.defaultLang': 'Default language', 'settings.maxWords': 'Maximum sentence length',
    'settings.off': 'off', 'settings.words': (n) => `${n} words`, 'settings.categories': 'Checked categories', 'settings.saved': 'Settings saved',
    'dict.title': 'Custom dictionary', 'dict.help': 'Names, jargon, neologisms: these words will no longer be flagged.',
    'dict.placeholder': 'Add a word…', 'dict.add': 'Add', 'dict.empty': 'No words yet.',
    'rules.title': 'Rules', 'rules.filter': 'Filter rules…', 'rules.allLangs': 'All languages',
    'themes.title': 'Themes', 'themes.auto': 'Follow the system light/dark mode', 'themes.editor': 'Create a theme',
    'themes.import': 'Import', 'themes.export': 'Export', 'themes.name': 'Name', 'themes.base': 'Based on',
    'themes.fontText': 'Text font', 'themes.fontUi': 'Interface font', 'themes.size': 'Text size', 'themes.radius': 'Corner radius',
    'themes.dark': 'Dark theme', 'themes.try': 'Try it', 'themes.save': 'Save',
    'themes.saved': 'Theme saved', 'themes.deleted': 'Theme deleted', 'themes.default': 'Default', 'themes.makeDefault': 'Make default',
    'themes.custom': 'custom', 'themes.edit': 'Edit', 'themes.delete': 'Delete', 'themes.light': 'light', 'themes.darkShort': 'dark',
    'themes.previewText': 'The cat sleeps on the <u>sofa</u>.', 'themes.invalid': 'Invalid theme file',
    'token.title': 'Access token required', 'token.help': 'This server protects changes with a token (LIBREPLUME_TOKEN).', 'token.save': 'Save',
    cancel: 'Cancel', error: 'Error',
  },
};

let current = (() => {
  try {
    const saved = localStorage.getItem('lp.uiLang');
    if (saved && STRINGS[saved]) return saved;
  } catch (_) { /* ignore */ }
  return (navigator.language || 'fr').toLowerCase().startsWith('fr') ? 'fr' : 'en';
})();

export function lang() {
  return current;
}

export function setLang(l) {
  current = STRINGS[l] ? l : 'fr';
  try { localStorage.setItem('lp.uiLang', current); } catch (_) { /* ignore */ }
  apply();
}

/** Translates a key; function entries receive the extra arguments. */
export function t(key, ...args) {
  const entry = STRINGS[current][key] ?? STRINGS.fr[key] ?? key;
  return typeof entry === 'function' ? entry(...args) : entry;
}

/** Translates every element carrying data-i18n attributes. */
export function apply(root = document) {
  document.documentElement.lang = current;
  root.querySelectorAll('[data-i18n]').forEach((el) => { el.textContent = t(el.dataset.i18n); });
  root.querySelectorAll('[data-i18n-placeholder]').forEach((el) => { el.placeholder = t(el.dataset.i18nPlaceholder); });
  root.querySelectorAll('[data-i18n-title]').forEach((el) => { el.title = t(el.dataset.i18nTitle); el.setAttribute('aria-label', el.title); });
  document.dispatchEvent(new CustomEvent('lp:lang'));
}
