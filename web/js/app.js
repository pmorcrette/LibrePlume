// LibrePlume web interface: routing and shared helpers.

import { apply, lang, setLang } from './i18n.js';
import * as checker from './checker.js';
import * as conjugator from './conjugator.js';
import * as settings from './settings.js';
import * as themes from './themes.js';
import * as docs from './docs.js';

const views = { check: checker, conjugate: conjugator, rules: settings, themes, api: docs };
const routes = { '': 'check', conjugate: 'conjugate', conjugueur: 'conjugate', rules: 'rules', themes: 'themes', api: 'api' };
const started = new Set();

let toastTimer;
export function toast(message) {
  const el = document.getElementById('toast');
  el.textContent = message;
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 2600);
}

export function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

/** Creates an element: h('div', { class: 'x', onclick }, child1, 'text'). */
export function h(tag, attrs = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === undefined || v === null || v === false) continue;
    if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else if (k === 'style' && typeof v === 'object') {
      for (const [prop, val] of Object.entries(v)) {
        if (prop.startsWith('--')) el.style.setProperty(prop, val);
        else el.style[prop] = val;
      }
    }
    else if (k === 'html') el.innerHTML = v;
    else el.setAttribute(k, v === true ? '' : v);
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

function route() {
  const [path, query] = location.hash.replace(/^#\/?/, '').split('?');
  const [first, ...rest] = path.split('/');
  const name = routes[first] ?? 'check';
  document.querySelectorAll('.view').forEach((v) => { v.hidden = v.dataset.view !== name; });
  document.querySelectorAll('.tabs a').forEach((a) => {
    if (a.dataset.route === name) a.setAttribute('aria-current', 'page');
    else a.removeAttribute('aria-current');
  });
  const view = views[name];
  if (!started.has(name)) {
    started.add(name);
    view.init?.();
  }
  view.show?.(decodeURIComponent(rest.join('/')), new URLSearchParams(query || ''));
}

function init() {
  apply();
  const uiLang = document.getElementById('ui-lang');
  uiLang.textContent = lang().toUpperCase();
  uiLang.addEventListener('click', () => {
    setLang(lang() === 'fr' ? 'en' : 'fr');
    uiLang.textContent = lang().toUpperCase();
  });
  document.getElementById('theme-toggle').addEventListener('click', () => themes.cycle());
  themes.boot();
  window.addEventListener('hashchange', route);
  route();
}

init();
