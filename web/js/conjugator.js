// The conjugator view.

import { api } from './api.js';
import { h } from './app.js';
import { t } from './i18n.js';

const EXAMPLES = {
  fr: ['être', 'avoir', 'aller', 'faire', 'venir', 'prendre', 'finir', 's’asseoir', 'appeler', 'résoudre'],
  en: ['be', 'have', 'go', 'write', 'study', 'stop'],
};

let form, field, langSelect, result;

export function init() {
  form = document.getElementById('conj-form');
  field = document.getElementById('conj-input');
  langSelect = document.getElementById('conj-lang');
  result = document.getElementById('conj-result');
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const verb = field.value.trim();
    if (verb) location.hash = `#/conjugate/${encodeURIComponent(verb)}?lang=${langSelect.value}`;
  });
  langSelect.addEventListener('change', renderExamples);
  renderExamples();
}

function renderExamples() {
  const lang = langSelect.value;
  document.getElementById('conj-examples').replaceChildren(...EXAMPLES[lang].map((v) => h('a', {
    class: 'chip', href: `#/conjugate/${encodeURIComponent(v)}?lang=${lang}`,
  }, v)));
}

export async function show(verb, params) {
  const lang = params.get('lang') || 'fr';
  langSelect.value = lang;
  renderExamples();
  if (!verb) {
    result.replaceChildren();
    field.focus();
    return;
  }
  field.value = verb;
  try {
    const conj = await api.get(`/api/conjugate/${encodeURIComponent(verb)}?language=${lang}`);
    render(conj);
  } catch (e) {
    result.replaceChildren(h('div', { class: 'card empty-state' }, e.message));
  }
}

function render(c) {
  const meta = [
    c.group ? h('span', { class: 'badge' }, t('conj.group', c.group)) : null,
    c.language === 'fr' ? h('span', { class: 'badge' }, t('conj.aux', c.auxiliary)) : null,
    c.model && c.model !== c.infinitive ? h('span', { class: 'badge' }, t('conj.model', c.model)) : null,
    c.pronominal ? h('span', { class: 'badge' }, t('conj.pronominal')) : null,
    !c.known ? h('span', { class: 'badge warn', title: t('conj.unknown') }, `⚠ ${t('conj.unknown')}`) : null,
  ];
  const sep = c.language === 'fr' ? '\u00a0: ' : ': ';
  const participles = c.participles.map((p) => h('span', {}, `${p.name}${sep}`, h('b', {}, p.form)));
  const tenseCard = (tense) => h('article', { class: 'card tense' },
    h('h3', {}, tense.name, tense.compound ? h('small', {}, t('conj.compound')) : null),
    h('ol', {}, ...tense.forms.map((f) => h('li', {}, ...splitPronoun(f)))));
  const moods = c.moods.map((mood) => h('section', { class: 'mood' },
    h('h2', {}, mood.name),
    h('div', { class: 'tenses' }, ...mood.tenses.map(tenseCard))));
  result.replaceChildren(
    h('header', { class: 'conj-header' }, h('h1', {}, c.infinitive), h('div', { class: 'conj-meta' }, ...meta)),
    h('div', { class: 'participles' }, ...participles),
    ...moods,
  );
}

/** Splits "j'ai aimé" into a muted pronoun and the verb form. */
function splitPronoun(f) {
  if (!f.pronoun) return [h('span', { class: 'vb' }, f.text)];
  const idx = f.text.lastIndexOf(f.form);
  if (idx <= 0) return [h('span', { class: 'vb' }, f.text)];
  return [h('span', { class: 'pro' }, f.text.slice(0, idx)), h('span', { class: 'vb' }, f.form)];
}
