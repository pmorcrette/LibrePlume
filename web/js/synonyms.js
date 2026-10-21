// The synonyms view.

import { api } from './api.js';
import { h } from './app.js';
import { t } from './i18n.js';

const EXAMPLES = {
  fr: ['beau', 'rapide', 'difficile', 'travail', 'dire', 'content', 'souvent', 'problème'],
  en: ['happy', 'fast', 'big', 'work', 'say', 'important'],
};
const SHOWN = 24;

let form, field, langSelect, result;

export function init() {
  form = document.getElementById('syn-form');
  field = document.getElementById('syn-input');
  langSelect = document.getElementById('syn-lang');
  result = document.getElementById('syn-result');
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const word = field.value.trim();
    if (word) location.hash = link(word, langSelect.value);
  });
  langSelect.addEventListener('change', renderExamples);
  renderExamples();
}

export function link(word, lang) {
  return `#/synonyms/${encodeURIComponent(word)}?lang=${lang}`;
}

function renderExamples() {
  const lang = langSelect.value;
  document.getElementById('syn-examples').replaceChildren(...EXAMPLES[lang].map((w) => h('a', { class: 'chip', href: link(w, lang) }, w)));
}

export async function show(word, params) {
  const lang = params.get('lang') || 'fr';
  langSelect.value = lang;
  renderExamples();
  if (!word) {
    result.replaceChildren();
    field.focus();
    return;
  }
  field.value = word;
  try {
    render(await api.get(`/api/synonyms/${encodeURIComponent(word)}?language=${lang}`));
  } catch (_) {
    result.replaceChildren(h('div', { class: 'card empty-state' }, t('syn.none', word)));
  }
}

/** Translated part-of-speech label ("adjective noun" → "adjectif · nom"). */
export function posLabel(pos) {
  return pos.split(' ').filter(Boolean).map((p) => t(`pos.${p}`)).join(' · ');
}

/** A list of words as links, collapsed after `shown` entries. */
export function wordLinks(words, lang, { cls = '', shown = SHOWN, onPick } = {}) {
  const make = (w) => (onPick
    ? h('button', { type: 'button', class: `syn-word ${cls}`, onclick: () => onPick(w) }, w)
    : h('a', { class: `syn-word ${cls}`, href: link(w, lang) }, w));
  const list = h('div', { class: 'syn-words' }, ...words.slice(0, shown).map(make));
  if (words.length > shown) {
    const more = h('button', { type: 'button', class: 'link-btn' }, t('syn.more', words.length - shown));
    more.addEventListener('click', () => { more.replaceWith(...words.slice(shown).map(make)); });
    list.append(more);
  }
  return list;
}

function render(s) {
  const header = h('header', { class: 'conj-header' },
    h('h1', {}, s.word),
    h('div', { class: 'conj-meta' }, s.lemma.toLowerCase() !== s.word.toLowerCase() ? h('span', { class: 'badge' }, t('syn.lemma', s.lemma)) : null));
  const meanings = s.meanings.map((m, i) => h('article', { class: 'card syn-meaning' },
    h('h3', {}, s.meanings.length > 1 ? t('syn.meaning', i + 1) : t('syn.synonyms'), m.part_of_speech ? h('small', {}, posLabel(m.part_of_speech)) : null),
    m.synonyms.length ? wordLinks(m.synonyms, s.language) : null,
    m.antonyms.length ? h('div', { class: 'syn-antonyms' }, h('span', { class: 'muted syn-label' }, t('syn.antonyms')), wordLinks(m.antonyms, s.language, { cls: 'antonym' })) : null));
  result.replaceChildren(header, h('div', { class: 'syn-grid' }, ...meanings));
}
