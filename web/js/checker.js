// The checker view: editor with inline highlights and the list of suggestions.

import { api } from './api.js';
import { h, escapeHtml, toast } from './app.js';
import { t } from './i18n.js';
import { link as synonymsLink, wordLinks } from './synonyms.js';

const CATEGORIES = ['spelling', 'grammar', 'conjugation', 'typography', 'punctuation', 'style'];
const DEBOUNCE_MS = 450;
const LANG_NAMES = { fr: 'Français', en: 'English' };

const state = {
  text: '',
  matches: [],
  ignored: new Set(),
  hidden: new Set(),
  active: -1,
  stats: null,
  language: null,
  pending: null,
  timer: 0,
  checkedText: null,
};

let input, backdrop, list, chips;

const store = {
  get(k, d) { try { return localStorage.getItem(k) ?? d; } catch (_) { return d; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch (_) { /* ignore */ } },
};

export function init() {
  input = document.getElementById('input');
  backdrop = document.getElementById('backdrop');
  list = document.getElementById('issues');
  chips = document.getElementById('category-filter');
  const langSelect = document.getElementById('check-lang');
  langSelect.value = store.get('lp.checkLang', 'auto');
  try { state.hidden = new Set(JSON.parse(store.get('lp.hiddenCats', '[]'))); } catch (_) { /* ignore */ }

  input.value = store.get('lp.draft', '');
  state.text = input.value;

  input.addEventListener('input', onInput);
  input.addEventListener('click', syncActiveFromCaret);
  input.addEventListener('dblclick', showSynonymsForSelection);
  input.addEventListener('keyup', (e) => { if (e.shiftKey && e.key.startsWith('Arrow')) showSynonymsForSelection(); });
  input.addEventListener('keyup', (e) => { if (e.key.startsWith('Arrow') || e.key === 'Home' || e.key === 'End') syncActiveFromCaret(); });
  window.addEventListener('resize', autosize);
  langSelect.addEventListener('change', () => { store.set('lp.checkLang', langSelect.value); runCheck(); });
  document.getElementById('btn-copy').addEventListener('click', async () => {
    await navigator.clipboard.writeText(input.value);
    toast(t('check.copied'));
  });
  document.getElementById('btn-clear').addEventListener('click', () => {
    input.focus();
    input.select();
    if (!document.execCommand('insertText', false, '')) input.value = '';
    onInput();
  });
  document.getElementById('btn-fix-all').addEventListener('click', fixAll);
  document.addEventListener('lp:lang', render);
  autosize();
  render();
  if (state.text.trim()) runCheck();
}

export function show() {
  requestAnimationFrame(autosize);
}

function autosize() {
  if (!input) return;
  input.style.height = 'auto';
  input.style.height = `${input.scrollHeight}px`;
}

/** Shifts known matches after an edit so highlights follow the text until the next check. */
function onInput() {
  const old = state.text;
  const now = input.value;
  let start = 0;
  while (start < old.length && start < now.length && old[start] === now[start]) start++;
  let endOld = old.length;
  let endNew = now.length;
  while (endOld > start && endNew > start && old[endOld - 1] === now[endNew - 1]) { endOld--; endNew--; }
  const delta = endNew - endOld;
  state.matches = state.matches.filter((m) => m.offset + m.length <= start || m.offset >= endOld)
    .map((m) => (m.offset >= endOld ? { ...m, offset: m.offset + delta } : m));
  state.text = now;
  store.set('lp.draft', now);
  autosize();
  render();
  clearTimeout(state.timer);
  state.timer = setTimeout(runCheck, DEBOUNCE_MS);
}

async function runCheck() {
  const text = input.value;
  state.pending?.abort();
  const status = document.getElementById('check-status');
  if (!text.trim()) {
    state.matches = [];
    state.stats = null;
    state.checkedText = text;
    status.textContent = '';
    status.classList.remove('busy');
    render();
    return;
  }
  const controller = new AbortController();
  state.pending = controller;
  status.textContent = t('check.checking');
  status.classList.add('busy');
  const language = document.getElementById('check-lang').value;
  try {
    const result = await api.post('/api/check', { text, language }, { signal: controller.signal });
    if (input.value !== text) return; // a newer check is on its way
    state.matches = result.matches;
    state.stats = result.stats;
    state.language = result.language_detected ? result.language : null;
    state.checkedText = text;
    status.textContent = t('check.checked');
  } catch (e) {
    if (e.name === 'AbortError') return;
    status.textContent = t('check.offline');
  } finally {
    if (state.pending === controller) {
      status.classList.remove('busy');
      state.pending = null;
    }
  }
  render();
}

function key(m) {
  return `${m.rule_id}\u0000${m.text}`;
}

function visibleMatches() {
  return state.matches.filter((m) => !state.ignored.has(key(m)) && !state.hidden.has(m.category));
}

function render() {
  renderBackdrop();
  renderList();
  renderChips();
  renderStats();
}

function renderBackdrop() {
  const text = state.text;
  const matches = visibleMatches().slice().sort((a, b) => a.offset - b.offset || b.length - a.length);
  let html = '';
  let cursor = 0;
  for (const m of matches) {
    if (m.offset < cursor || m.length === 0) continue;
    const idx = state.matches.indexOf(m);
    html += escapeHtml(text.slice(cursor, m.offset));
    const cls = [m.severity === 'hint' ? 'hint' : '', idx === state.active ? 'active' : ''].join(' ').trim();
    html += `<mark data-i="${idx}" class="${cls}" style="--c: var(--${m.category})">${escapeHtml(text.slice(m.offset, m.offset + m.length))}</mark>`;
    cursor = m.offset + m.length;
  }
  html += escapeHtml(text.slice(cursor));
  // A trailing newline needs content after it to take up space.
  backdrop.innerHTML = `${html}​`;
}

function renderChips() {
  const counts = Object.fromEntries(CATEGORIES.map((c) => [c, 0]));
  for (const m of state.matches) if (!state.ignored.has(key(m))) counts[m.category]++;
  chips.replaceChildren(...CATEGORIES.filter((c) => counts[c] > 0 || state.hidden.has(c)).map((c) => h('button', {
    class: 'chip', type: 'button', style: { '--c': `var(--${c})` }, 'aria-pressed': String(!state.hidden.has(c)),
    onclick: () => {
      if (state.hidden.has(c)) state.hidden.delete(c); else state.hidden.add(c);
      store.set('lp.hiddenCats', JSON.stringify([...state.hidden]));
      render();
    },
  }, h('span', { class: 'dot' }), t(`cat.${c}`), h('span', { class: 'count' }, counts[c]))));
}

function renderStats() {
  const s = state.stats;
  const words = s ? s.words : (state.text.match(/[\p{L}\p{N}’'-]+/gu) || []).length;
  document.getElementById('stat-words').textContent = t('check.words', words);
  document.getElementById('stat-sentences').textContent = s ? t('check.sentences', s.sentences) : '';
  document.getElementById('stat-reading').textContent = s && s.words ? t('check.reading', s.reading_time_seconds) : '';
  document.getElementById('detected-lang').textContent = state.language ? t('check.detected', LANG_NAMES[state.language]) : '';

  const visible = visibleMatches();
  const fixable = visible.filter((m) => m.severity !== 'hint' && m.replacements.length);
  document.getElementById('btn-fix-all').disabled = fixable.length === 0;

  const ring = document.getElementById('score-ring');
  const label = document.getElementById('score-label');
  if (!state.text.trim()) {
    ring.style.strokeDashoffset = '119.4';
    label.textContent = t('check.empty');
    return;
  }
  const weights = { error: 3, warning: 2, hint: 0.5 };
  const penalty = visible.reduce((acc, m) => acc + weights[m.severity], 0) / Math.max(words, 1) * 100;
  // Weighted issues per 100 words, mapped to a 0–100 score that never quite reaches 0.
  const score = Math.round(100 * Math.exp(-penalty / 40));
  ring.style.strokeDashoffset = String(119.4 * (1 - score / 100));
  ring.style.stroke = score >= 85 ? 'var(--punctuation)' : score >= 60 ? 'var(--grammar)' : 'var(--spelling)';
  const verdict = score >= 85 ? t('check.score.great') : score >= 60 ? t('check.score.good') : t('check.score.work');
  label.textContent = visible.length ? `${t('check.count', visible.length)} · ${verdict}` : t('check.perfect');
}

function renderList() {
  const matches = visibleMatches();
  if (!matches.length) {
    list.replaceChildren(state.text.trim() && state.checkedText === state.text
      ? h('li', { class: 'empty-state' }, svgIcon('icon-check'), h('p', {}, t('check.perfect')), h('p', { class: 'muted' }, t('syn.hint')))
      : h('li', { class: 'empty-state' }, h('p', { class: 'muted' }, t('syn.hint'))));
    return;
  }
  list.replaceChildren(...matches.map((m) => {
    const idx = state.matches.indexOf(m);
    const suggestions = m.replacements.slice(0, 5).map((r) => h('button', {
      class: `suggestion${r === '' ? ' empty' : ''}`, type: 'button',
      onclick: (e) => { e.stopPropagation(); applyReplacement(m, r); },
    }, r === '' ? t('check.remove') : visibleSpaces(r)));
    const actions = [
      h('button', { class: 'link-btn', type: 'button', onclick: (e) => { e.stopPropagation(); state.ignored.add(key(m)); render(); } }, t('check.ignore')),
      m.category === 'spelling' ? h('button', { class: 'link-btn', type: 'button', onclick: (e) => { e.stopPropagation(); addWord(m.text); } }, t('check.addWord')) : null,
      h('button', { class: 'link-btn', type: 'button', onclick: (e) => { e.stopPropagation(); disableRule(m.rule_id); } }, t('check.disableRule')),
    ];
    return h('li', {
      class: `issue${idx === state.active ? ' active' : ''}`, 'data-i': idx, tabindex: 0,
      style: { '--c': `var(--${m.category})` },
      onclick: () => focusMatch(idx),
      onkeydown: (e) => { if (e.key === 'Enter') focusMatch(idx); },
    },
    h('div', { class: 'issue-head' }, t(`cat.${m.category}`), h('span', { class: 'rule' }, m.rule_id)),
    h('p', { class: 'issue-msg' }, m.message),
    h('div', { class: 'issue-fix' }, m.text.trim() && m.replacements.length ? h('del', {}, m.text) : null, ...suggestions),
    h('div', { class: 'issue-actions' }, ...actions));
  }));
}

function visibleSpaces(s) {
  // Make non-breaking spaces visible in suggestions.
  return s.replace(/ /g, '⍽').replace(/ /g, '⍽');
}

function svgIcon(id) {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
  use.setAttribute('href', `#${id}`);
  svg.append(use);
  return svg;
}

function setActive(idx, scrollList = true) {
  state.active = idx;
  backdrop.querySelectorAll('mark').forEach((el) => el.classList.toggle('active', Number(el.dataset.i) === idx));
  list.querySelectorAll('.issue').forEach((el) => el.classList.toggle('active', Number(el.dataset.i) === idx));
  if (scrollList) list.querySelector(`.issue[data-i="${idx}"]`)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
}

function focusMatch(idx) {
  const m = state.matches[idx];
  if (!m) return;
  setActive(idx, false);
  input.focus({ preventScroll: true });
  input.setSelectionRange(m.offset, m.offset + m.length);
  const mark = backdrop.querySelector(`mark[data-i="${idx}"]`);
  if (mark) {
    const r = mark.getBoundingClientRect();
    if (r.top < 90 || r.bottom > window.innerHeight - 40) window.scrollBy({ top: r.top - window.innerHeight / 3, behavior: 'smooth' });
  }
}

function syncActiveFromCaret() {
  const pos = input.selectionStart;
  const idx = state.matches.findIndex((m) => visibleMatches().includes(m) && pos >= m.offset && pos <= m.offset + m.length);
  if (idx !== state.active) setActive(idx);
}

/** Replaces a range through the editing commands so that undo keeps working. */
// ------------------------------------------------------------------ synonyms of the selected word

const WORD = /^[\p{L}][\p{L}'’-]{0,39}$/u;
let synonymRequest = 0;

/** Shows the synonyms of the selected word above the suggestions, to replace it in one click. */
async function showSynonymsForSelection() {
  const start = input.selectionStart;
  const end = input.selectionEnd;
  const word = input.value.slice(start, end).trim();
  const card = document.getElementById('syn-card');
  if (!WORD.test(word) || word.length < 2) return;
  const offset = start + input.value.slice(start, end).indexOf(word);
  const chosen = document.getElementById('check-lang').value;
  const lang = chosen !== 'auto' ? chosen : state.language || 'fr';
  const request = ++synonymRequest;
  let result;
  try {
    result = await api.get(`/api/synonyms/${encodeURIComponent(word)}?language=${lang}`);
  } catch (_) {
    result = null;
  }
  if (request !== synonymRequest) return;
  const close = h('button', { class: 'link-btn syn-close', type: 'button', 'aria-label': '×', onclick: () => card.replaceChildren() }, '×');
  if (!result) {
    card.replaceChildren(h('div', { class: 'syn-card' }, h('div', { class: 'syn-card-head' }, h('strong', {}, t('syn.of', word)), close),
      h('p', { class: 'muted' }, t('syn.none', word))));
    return;
  }
  const words = [];
  for (const m of result.meanings) for (const s of m.synonyms) if (!words.includes(s)) words.push(s);
  const pick = (synonym) => {
    if (input.value.slice(offset, offset + word.length) !== word) return;
    const replacement = word[0] === word[0].toUpperCase() ? synonym[0].toUpperCase() + synonym.slice(1) : synonym;
    replaceRange(offset, offset + word.length, replacement);
    input.setSelectionRange(offset, offset + replacement.length);
    toast(t('syn.replaced', word, replacement));
    card.replaceChildren();
  };
  card.replaceChildren(h('div', { class: 'syn-card' },
    h('div', { class: 'syn-card-head' }, h('strong', {}, t('syn.of', word)), close),
    result.lemma.toLowerCase() !== word.toLowerCase() ? h('p', { class: 'muted' }, t('syn.lemma', result.lemma)) : null,
    wordLinks(words, result.language, { shown: 14, onPick: pick }),
    h('a', { class: 'link-btn', href: synonymsLink(result.lemma, result.language) }, t('syn.lookup'))));
}

function replaceRange(start, end, text) {
  input.focus({ preventScroll: true });
  input.setSelectionRange(start, end);
  if (!document.execCommand('insertText', false, text)) {
    input.setRangeText(text, start, end, 'end');
    input.dispatchEvent(new Event('input'));
  }
}

function applyReplacement(m, replacement) {
  if (input.value.slice(m.offset, m.offset + m.length) !== m.text) return runCheck();
  state.active = -1;
  replaceRange(m.offset, m.offset + m.length, replacement);
}

function fixAll() {
  const fixable = visibleMatches()
    .filter((m) => m.severity !== 'hint' && m.replacements.length)
    .sort((a, b) => a.offset - b.offset);
  let out = '';
  let cursor = 0;
  let count = 0;
  for (const m of fixable) {
    if (m.offset < cursor || input.value.slice(m.offset, m.offset + m.length) !== m.text) continue;
    out += input.value.slice(cursor, m.offset) + m.replacements[0];
    cursor = m.offset + m.length;
    count++;
  }
  out += input.value.slice(cursor);
  replaceRange(0, input.value.length, out);
  toast(t('check.fixed', count));
}

async function addWord(word) {
  try {
    await api.post('/api/dictionary', { word });
    toast(t('check.wordAdded', word));
    state.matches = state.matches.filter((m) => !(m.category === 'spelling' && m.text === word));
    render();
  } catch (e) {
    toast(e.message);
  }
}

async function disableRule(id) {
  try {
    await api.put(`/api/rules/${encodeURIComponent(id)}`, { enabled: false });
    toast(t('check.ruleDisabled'));
    state.matches = state.matches.filter((m) => m.rule_id !== id);
    render();
  } catch (e) {
    toast(e.message);
  }
}
