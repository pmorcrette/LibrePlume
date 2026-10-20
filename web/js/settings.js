// Settings view: general settings, custom dictionary and rules.

import { api } from './api.js';
import { h, toast } from './app.js';
import { t } from './i18n.js';

const CATEGORIES = ['spelling', 'grammar', 'conjugation', 'typography', 'punctuation', 'style'];

let settings = null;
let rules = [];

export function init() {
  document.getElementById('set-lang').addEventListener('change', (e) => save({ default_language: e.target.value === 'auto' ? null : e.target.value }));
  const range = document.getElementById('set-maxwords');
  range.addEventListener('input', () => showMaxWords(Number(range.value)));
  range.addEventListener('change', () => save({ max_sentence_words: Number(range.value) }));
  document.getElementById('dict-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const field = document.getElementById('dict-input');
    const words = field.value.split(/[\s,;]+/).filter(Boolean);
    if (!words.length) return;
    try {
      const res = await api.post('/api/dictionary', { words });
      field.value = '';
      renderWords(res.words);
    } catch (err) {
      toast(err.message);
    }
  });
  document.getElementById('rules-filter').addEventListener('input', renderRules);
  document.getElementById('rules-lang').addEventListener('change', renderRules);
  document.addEventListener('lp:lang', () => { if (settings) render(); });
}

export async function show() {
  try {
    const [s, r, d] = await Promise.all([api.get('/api/settings'), api.get('/api/rules'), api.get('/api/dictionary')]);
    settings = s;
    rules = r;
    render();
    renderWords(d.words);
  } catch (e) {
    toast(e.message);
  }
}

function render() {
  document.getElementById('set-lang').value = settings.default_language || 'auto';
  document.getElementById('set-maxwords').value = settings.max_sentence_words;
  showMaxWords(settings.max_sentence_words);
  document.getElementById('set-categories').replaceChildren(...CATEGORIES.map((c) => toggleRow({
    color: `var(--${c})`,
    label: t(`cat.${c}`),
    checked: !settings.disabled_categories.includes(c),
    onchange: (on) => {
      const set = new Set(settings.disabled_categories);
      if (on) set.delete(c); else set.add(c);
      save({ disabled_categories: [...set] });
    },
  })));
  renderRules();
}

function showMaxWords(n) {
  document.getElementById('set-maxwords-out').textContent = n === 0 ? t('settings.off') : t('settings.words', n);
}

async function save(patch) {
  try {
    settings = await api.patch('/api/settings', patch);
    rules = await api.get('/api/rules');
    render();
    toast(t('settings.saved'));
  } catch (e) {
    toast(e.message);
    if (settings) render();
  }
}

function toggleRow({ color, label, sub, tag, checked, onchange }) {
  const box = h('input', { type: 'checkbox', role: 'switch', 'aria-label': label });
  box.checked = checked;
  box.addEventListener('change', () => onchange(box.checked));
  return h('label', { class: 'toggle-row' },
    color ? h('span', { class: 'dot', style: { '--c': color } }) : null,
    h('span', { class: 'desc' }, label, sub ? h('small', {}, sub) : null),
    tag ? h('span', { class: 'lang-tag' }, tag) : null,
    h('span', { class: 'switch' }, box, h('span')));
}

function renderRules() {
  const q = document.getElementById('rules-filter').value.trim().toLowerCase();
  const lang = document.getElementById('rules-lang').value;
  const shown = rules.filter((r) => (!lang || !r.language || r.language === lang)
    && (!q || r.id.toLowerCase().includes(q) || r.description.toLowerCase().includes(q)));
  const groups = CATEGORIES.map((c) => [c, shown.filter((r) => r.category === c)]).filter(([, rs]) => rs.length);
  document.getElementById('rules-list').replaceChildren(...groups.map(([c, rs]) => h('div', { class: 'rule-group' },
    h('h3', {}, h('span', { class: 'dot', style: { background: `var(--${c})`, width: '.6rem', height: '.6rem', borderRadius: '50%', display: 'inline-block' } }), t(`cat.${c}`)),
    h('div', { class: 'toggle-list' }, ...rs.map((r) => toggleRow({
      label: r.description,
      sub: r.id,
      tag: r.language || 'fr · en',
      checked: r.enabled,
      onchange: async (on) => {
        try {
          settings = await api.put(`/api/rules/${encodeURIComponent(r.id)}`, { enabled: on });
          r.enabled = on;
          toast(t('settings.saved'));
        } catch (e) {
          toast(e.message);
          renderRules();
        }
      },
    }))))));
}

function renderWords(words) {
  const ul = document.getElementById('dict-words');
  if (!words.length) {
    ul.replaceChildren(h('li', { class: 'muted', style: { background: 'none' } }, t('dict.empty')));
    return;
  }
  ul.replaceChildren(...words.map((w) => h('li', {}, w, h('button', {
    type: 'button', 'aria-label': `${t('themes.delete')} ${w}`,
    onclick: async () => {
      try {
        const res = await api.del(`/api/dictionary/${encodeURIComponent(w)}`);
        renderWords(res.words);
      } catch (e) {
        toast(e.message);
      }
    },
  }, '×'))));
}
