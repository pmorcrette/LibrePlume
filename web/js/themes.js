// Themes: applying, choosing and creating interface themes.

import { api } from './api.js';
import { h, toast } from './app.js';
import { t } from './i18n.js';

const COLOR_VARS = ['bg', 'surface', 'surface-2', 'border', 'text', 'text-muted', 'accent', 'accent-contrast',
  'spelling', 'grammar', 'conjugation', 'typography', 'punctuation', 'style'];
const COLOR_LABELS = {
  fr: { bg: 'Fond', surface: 'Cartes', 'surface-2': 'Surfaces secondaires', border: 'Bordures', text: 'Texte', 'text-muted': 'Texte discret',
    accent: 'Accent', 'accent-contrast': 'Texte sur accent', spelling: 'Orthographe', grammar: 'Grammaire', conjugation: 'Conjugaison',
    typography: 'Typographie', punctuation: 'Ponctuation', style: 'Style' },
  en: { bg: 'Background', surface: 'Cards', 'surface-2': 'Secondary surfaces', border: 'Borders', text: 'Text', 'text-muted': 'Muted text',
    accent: 'Accent', 'accent-contrast': 'Text on accent', spelling: 'Spelling', grammar: 'Grammar', conjugation: 'Conjugation',
    typography: 'Typography', punctuation: 'Punctuation', style: 'Style' },
};
const TEXT_FONTS = [
  ['Literata', '"Literata", "Iowan Old Style", "Charter", Georgia, serif'],
  ['Garamond', '"EB Garamond", Garamond, "Times New Roman", serif'],
  ['Atkinson Hyperlegible', '"Atkinson Hyperlegible", Verdana, sans-serif'],
  ['Inter', '"Inter", "Segoe UI", system-ui, -apple-system, sans-serif'],
  ['OpenDyslexic', '"OpenDyslexic", "Comic Sans MS", sans-serif'],
  ['Mono', '"iA Writer Quattro", "IBM Plex Mono", "JetBrains Mono", "Courier New", monospace'],
];
const UI_FONTS = [
  ['Inter', '"Inter", "Segoe UI", system-ui, -apple-system, sans-serif'],
  ['IBM Plex Sans', '"IBM Plex Sans", system-ui, sans-serif'],
  ['Atkinson Hyperlegible', '"Atkinson Hyperlegible", Verdana, sans-serif'],
  ['Système', 'system-ui, -apple-system, "Segoe UI", Roboto, sans-serif'],
  ['Sérif', '"Literata", Georgia, serif'],
];

let themes = [];
let serverDefault = 'plume';
let editing = null; // id of the custom theme being edited

const store = {
  get(k, d) { try { return localStorage.getItem(k) ?? d; } catch (_) { return d; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch (_) { /* ignore */ } },
};

const systemDark = window.matchMedia('(prefers-color-scheme: dark)');

function choice() {
  return store.get('lp.theme', 'auto');
}

/** The theme to display for the current choice. */
function resolve() {
  const c = choice();
  if (c !== 'auto') return themes.find((x) => x.id === c) || themes[0];
  if (serverDefault !== 'plume') return themes.find((x) => x.id === serverDefault) || themes[0];
  return themes.find((x) => x.id === (systemDark.matches ? 'encre' : 'plume')) || themes[0];
}

export function applyVariables(theme, target = document.documentElement) {
  for (const [k, v] of Object.entries(theme.variables)) target.style.setProperty(`--${k}`, v);
  if (target === document.documentElement) {
    target.style.colorScheme = theme.dark ? 'dark' : 'light';
    let meta = document.querySelector('meta[name="theme-color"]');
    if (!meta) meta = document.head.appendChild(h('meta', { name: 'theme-color' }));
    meta.content = theme.variables.bg;
  }
}

function applyCurrent() {
  const theme = resolve();
  if (!theme) return;
  applyVariables(theme);
  store.set('lp.themeVars', JSON.stringify({ variables: theme.variables, dark: theme.dark }));
  if (document.getElementById('theme-grid')?.childElementCount) renderGrid();
}

export async function boot() {
  systemDark.addEventListener('change', () => { if (choice() === 'auto') applyCurrent(); });
  try {
    const res = await api.get('/api/themes');
    themes = res.themes;
    serverDefault = res.default;
    applyCurrent();
  } catch (_) { /* keep the cached theme */ }
}

/** Quick switch from the top bar: next theme in the list. */
export function cycle() {
  if (!themes.length) return;
  const cur = resolve();
  const next = themes[(themes.indexOf(cur) + 1) % themes.length];
  store.set('lp.theme', next.id);
  applyCurrent();
  toast(next.name);
}

function select(id) {
  store.set('lp.theme', id);
  document.getElementById('theme-auto').checked = false;
  applyCurrent();
}

export function init() {
  const auto = document.getElementById('theme-auto');
  auto.addEventListener('change', () => {
    store.set('lp.theme', auto.checked ? 'auto' : resolve().id);
    applyCurrent();
  });
  const fill = (sel, list) => sel.replaceChildren(...list.map(([name, stack]) => h('option', { value: stack }, name)));
  fill(document.getElementById('theme-font-text'), TEXT_FONTS);
  fill(document.getElementById('theme-font-ui'), UI_FONTS);

  const form = document.getElementById('theme-form');
  form.addEventListener('input', updateLive);
  form.addEventListener('submit', (e) => { e.preventDefault(); saveTheme(); });
  document.getElementById('theme-base').addEventListener('change', (e) => {
    const base = themes.find((x) => x.id === e.target.value);
    editing = null;
    if (base) loadIntoEditor(base, false);
  });
  document.getElementById('theme-preview-btn').addEventListener('click', () => applyVariables(readEditor()));
  document.getElementById('theme-export').addEventListener('click', exportTheme);
  document.getElementById('theme-import').addEventListener('click', importTheme);
  document.addEventListener('lp:lang', () => { renderGrid(); renderColorFields(readEditor().variables); });
}

export async function show() {
  try {
    const res = await api.get('/api/themes');
    themes = res.themes;
    serverDefault = res.default;
  } catch (e) {
    toast(e.message);
  }
  document.getElementById('theme-auto').checked = choice() === 'auto';
  renderGrid();
  const baseSel = document.getElementById('theme-base');
  baseSel.replaceChildren(...themes.map((x) => h('option', { value: x.id }, x.name)));
  if (!editing) {
    const cur = resolve();
    baseSel.value = cur.id;
    loadIntoEditor(cur, false);
  }
}

function previewOf(theme) {
  const v = theme.variables;
  return h('div', { class: 'preview', style: { background: v.bg, color: v.text, fontFamily: v['font-ui'] } },
    h('div', { class: 'bar', style: { background: v.accent, width: '40%' } }),
    h('div', { style: { background: v.surface, border: `1px solid ${v.border}`, borderRadius: '8px', padding: '.5rem .6rem' } },
      h('div', { class: 'line', style: { fontFamily: v['font-text'] }, html: t('themes.previewText').replace('<u>', `<u style="text-decoration-color:${v.spelling}">`) }),
      h('div', { style: { display: 'flex', gap: '4px', marginTop: '.45rem' } },
        ...['spelling', 'grammar', 'conjugation', 'typography', 'style'].map((c) => h('span', { style: { width: '14px', height: '6px', borderRadius: '3px', background: v[c] } }))),
    ));
}

function renderGrid() {
  const grid = document.getElementById('theme-grid');
  const current = choice() === 'auto' ? null : resolve().id;
  grid.replaceChildren(...themes.map((theme) => {
    const isDefault = theme.id === serverDefault;
    return h('div', { class: 'theme-card', 'aria-pressed': String(theme.id === current) },
      h('button', { type: 'button', class: 'theme-select', 'aria-label': theme.name, onclick: () => select(theme.id) },
        previewOf(theme)),
      h('span', { class: 'tick' }, (() => { const s = document.createElementNS('http://www.w3.org/2000/svg', 'svg'); s.setAttribute('viewBox', '0 0 24 24'); s.innerHTML = '<use href="#icon-check"/>'; return s; })()),
      h('div', { class: 'meta' },
        theme.name,
        h('small', {}, isDefault ? `★ ${t('themes.default')}` : (theme.builtin ? (theme.dark ? t('themes.darkShort') : t('themes.light')) : t('themes.custom'))),
        h('span', { class: 'actions' },
          !isDefault ? h('button', { class: 'link-btn', type: 'button', title: t('themes.makeDefault'), onclick: () => makeDefault(theme.id) }, '★') : null,
          !theme.builtin ? h('button', { class: 'link-btn', type: 'button', onclick: () => { editing = theme.id; loadIntoEditor(theme, true); document.querySelector('.theme-editor').scrollIntoView({ behavior: 'smooth' }); } }, t('themes.edit')) : null,
          !theme.builtin ? h('button', { class: 'link-btn', type: 'button', onclick: () => deleteTheme(theme.id) }, t('themes.delete')) : null)));
  }));
}

async function makeDefault(id) {
  try {
    await api.patch('/api/settings', { default_theme: id });
    serverDefault = id;
    renderGrid();
    toast(t('settings.saved'));
  } catch (e) {
    toast(e.message);
  }
}

async function deleteTheme(id) {
  try {
    await api.del(`/api/themes/${encodeURIComponent(id)}`);
    if (choice() === id) store.set('lp.theme', 'auto');
    if (editing === id) editing = null;
    toast(t('themes.deleted'));
    await show();
    applyCurrent();
  } catch (e) {
    toast(e.message);
  }
}

function renderColorFields(vars) {
  const labels = COLOR_LABELS[document.documentElement.lang] || COLOR_LABELS.fr;
  document.getElementById('theme-colors').replaceChildren(...COLOR_VARS.map((k) => h('label', { class: 'color-field' },
    h('input', { type: 'color', name: k, value: toHex(vars[k]) }), labels[k])));
}

function loadIntoEditor(theme, keepName) {
  const v = theme.variables;
  renderColorFields(v);
  if (keepName) document.getElementById('theme-name').value = theme.name;
  document.getElementById('theme-dark').checked = !!theme.dark;
  setSelect('theme-font-text', v['font-text'], TEXT_FONTS);
  setSelect('theme-font-ui', v['font-ui'], UI_FONTS);
  document.getElementById('theme-size').value = parseFloat(v['text-size']) || 1.125;
  document.getElementById('theme-radius').value = parseFloat(v.radius) || 14;
  updateLive();
}

function setSelect(id, value, list) {
  const sel = document.getElementById(id);
  if (!list.some(([, stack]) => stack === value)) sel.append(h('option', { value }, value.split(',')[0].replace(/"/g, '')));
  sel.value = value;
}

function toHex(color) {
  if (/^#[0-9a-f]{6}$/i.test(color)) return color;
  if (/^#[0-9a-f]{3}$/i.test(color)) return `#${[...color.slice(1)].map((c) => c + c).join('')}`;
  const probe = h('span', { style: { color } });
  document.body.append(probe);
  const rgb = getComputedStyle(probe).color.match(/\d+/g) || [0, 0, 0];
  probe.remove();
  return `#${rgb.slice(0, 3).map((n) => Number(n).toString(16).padStart(2, '0')).join('')}`;
}

function readEditor() {
  const variables = {};
  document.querySelectorAll('#theme-colors input[type="color"]').forEach((i) => { variables[i.name] = i.value; });
  variables['font-text'] = document.getElementById('theme-font-text').value;
  variables['font-ui'] = document.getElementById('theme-font-ui').value;
  variables['text-size'] = `${document.getElementById('theme-size').value}rem`;
  variables.radius = `${document.getElementById('theme-radius').value}px`;
  const name = document.getElementById('theme-name').value.trim() || 'Mon thème';
  return { id: editing || slug(name), name, dark: document.getElementById('theme-dark').checked, variables };
}

function slug(name) {
  const s = name.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  return (s || 'theme').slice(0, 40);
}

function updateLive() {
  const theme = readEditor();
  const v = theme.variables;
  document.getElementById('theme-size-out').textContent = v['text-size'];
  document.getElementById('theme-radius-out').textContent = v.radius;
  const live = document.getElementById('theme-live');
  live.replaceChildren(h('div', { style: { background: v.bg, color: v.text, padding: '1.25rem', fontFamily: v['font-ui'] } },
    h('div', { style: { display: 'flex', gap: '.5rem', marginBottom: '1rem' } },
      h('span', { style: { background: v.accent, color: v['accent-contrast'], padding: '.3rem .8rem', borderRadius: '999px', fontWeight: 600, fontSize: '.8rem' } }, t('nav.check')),
      h('span', { style: { color: v['text-muted'], padding: '.3rem .5rem', fontSize: '.8rem' } }, t('nav.conjugate'))),
    h('div', { style: { background: v.surface, border: `1px solid ${v.border}`, borderRadius: v.radius, padding: '1rem 1.2rem' } },
      h('p', { style: { fontFamily: v['font-text'], fontSize: v['text-size'], lineHeight: 1.7, margin: 0 }, html:
        `Les <u style="text-decoration:underline wavy ${v.grammar}">chat</u> dorment sur le <u style="text-decoration:underline wavy ${v.spelling}">canapè</u>, <u style="text-decoration:underline dotted ${v.style};text-decoration-thickness:2px">très très</u> paisiblement.` }),
      h('div', { style: { marginTop: '.9rem', background: v['surface-2'], borderRadius: `calc(${v.radius} * .6)`, padding: '.6rem .75rem', fontSize: '.85rem', borderLeft: `3px solid ${v.spelling}` } },
        h('strong', { style: { color: v.spelling, fontSize: '.7rem', letterSpacing: '.06em', textTransform: 'uppercase' } }, t('cat.spelling')),
        h('div', { style: { margin: '.25rem 0 .4rem' } }, 'canapè → '),
        h('span', { style: { background: v.surface, padding: '.15rem .55rem', borderRadius: '6px', fontFamily: v['font-text'], fontWeight: 600 } }, 'canapé'))),
    h('p', { style: { color: v['text-muted'], fontSize: '.8rem', margin: '.8rem 0 0' } }, 'Aa Bb Cc — 0123')));
}

async function saveTheme() {
  const theme = readEditor();
  try {
    const saved = await api.put(`/api/themes/${encodeURIComponent(theme.id)}`, theme);
    editing = saved.id;
    toast(t('themes.saved'));
    await show();
    select(saved.id);
  } catch (e) {
    toast(e.message);
  }
}

function exportTheme() {
  const theme = readEditor();
  const blob = new Blob([JSON.stringify(theme, null, 2)], { type: 'application/json' });
  const a = h('a', { href: URL.createObjectURL(blob), download: `libreplume-theme-${theme.id}.json` });
  document.body.append(a);
  a.click();
  setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
}

function importTheme() {
  const picker = h('input', { type: 'file', accept: 'application/json,.json' });
  picker.addEventListener('change', async () => {
    try {
      const theme = JSON.parse(await picker.files[0].text());
      if (!theme || typeof theme.variables !== 'object') throw new Error();
      editing = null;
      document.getElementById('theme-name').value = theme.name || 'Import';
      loadIntoEditor({ ...resolve(), ...theme, variables: { ...resolve().variables, ...theme.variables } }, true);
    } catch (_) {
      toast(t('themes.invalid'));
    }
  });
  picker.click();
}
