// "API & MCP" page: how to connect clients, AI agents and the browser extension.

import { h, toast } from './app.js';
import { lang } from './i18n.js';
import { api } from './api.js';

const TEXT = {
  fr: {
    restTitle: 'API REST',
    restIntro: 'Toutes les routes échangent du JSON. Les positions (offset, length) sont exprimées en unités UTF-16, comme les chaînes JavaScript.',
    mcpTitle: 'Serveur MCP pour agents IA',
    mcpIntro: 'LibrePlume parle le Model Context Protocol : un agent peut vérifier et corriger des textes, conjuguer, et administrer les règles, le dictionnaire, les réglages et les thèmes.',
    mcpHttp: 'Transport HTTP (recommandé)',
    mcpStdio: 'Transport stdio (agent local, via Docker)',
    tools: 'Outils disponibles',
    ltTitle: 'Compatibilité LanguageTool',
    ltIntro: 'Les clients LanguageTool (extensions, LibreOffice, Obsidian…) peuvent pointer vers ce serveur : il répond sur /v2/check.',
    extTitle: 'Extension Firefox',
    extIntro: 'Installez l’extension depuis le dossier extension/firefox du dépôt, puis indiquez l’adresse de ce serveur dans ses options :',
    authTitle: 'Authentification',
    authOn: 'Ce serveur exige un jeton pour les modifications et pour /mcp. Il est conservé uniquement dans ce navigateur.',
    authOff: 'Aucun jeton n’est configuré : toute personne qui accède au serveur peut modifier ses réglages. Définissez LIBREPLUME_TOKEN sur un serveur public.',
    setToken: 'Enregistrer le jeton', clearToken: 'Oublier le jeton', copied: 'Copié', copy: 'Copier',
  },
  en: {
    restTitle: 'REST API',
    restIntro: 'Every route speaks JSON. Positions (offset, length) are UTF-16 code units, like JavaScript strings.',
    mcpTitle: 'MCP server for AI agents',
    mcpIntro: 'LibrePlume speaks the Model Context Protocol: an agent can check and correct texts, conjugate verbs, and manage rules, dictionary, settings and themes.',
    mcpHttp: 'HTTP transport (recommended)',
    mcpStdio: 'stdio transport (local agent, through Docker)',
    tools: 'Available tools',
    ltTitle: 'LanguageTool compatibility',
    ltIntro: 'LanguageTool clients (browser add-ons, LibreOffice, Obsidian…) can point to this server: it answers on /v2/check.',
    extTitle: 'Firefox extension',
    extIntro: 'Install the add-on from the extension/firefox folder of the repository, then enter this server address in its options:',
    authTitle: 'Authentication',
    authOn: 'This server requires a token for changes and for /mcp. It is only stored in this browser.',
    authOff: 'No token is configured: anyone who can reach the server can change its settings. Set LIBREPLUME_TOKEN on a public server.',
    setToken: 'Save token', clearToken: 'Forget token', copied: 'Copied', copy: 'Copy',
  },
};

const ENDPOINTS = [
  ['POST /api/check', '{ text, language?, categories?, disabled_rules?, enabled_rules? }'],
  ['POST /api/correct', '{ text, language?, categories? }'],
  ['GET /api/conjugate/{verb}?language=fr', ''],
  ['GET /api/synonyms/{word}?language=fr', ''],
  ['GET /api/rules', '?language=fr&category=grammar'],
  ['PUT /api/rules/{id}', '{ enabled }'],
  ['GET | PATCH /api/settings', '{ default_language, max_sentence_words, disabled_categories, default_theme }'],
  ['GET | POST | DELETE /api/dictionary', '{ word } | { words: [] }'],
  ['GET /api/themes', ''],
  ['PUT | DELETE /api/themes/{id}', '{ name, dark, variables }'],
  ['GET /api/info', ''],
  ['POST /v2/check', 'LanguageTool (form: text, language)'],
  ['POST /mcp', 'Model Context Protocol (JSON-RPC)'],
];

const TOOLS = ['check_text', 'correct_text', 'conjugate_verb', 'find_synonyms', 'list_rules', 'set_rule_enabled', 'get_settings', 'update_settings',
  'list_dictionary', 'add_dictionary_words', 'remove_dictionary_words', 'list_themes', 'save_theme', 'delete_theme', 'server_info'];

function code(text) {
  const btn = h('button', { class: 'btn ghost small copy', type: 'button' }, TEXT[lang()].copy);
  btn.addEventListener('click', async () => {
    await navigator.clipboard.writeText(text);
    toast(TEXT[lang()].copied);
  });
  return h('pre', {}, h('code', {}, text), btn);
}

let info = null;

export async function show() {
  if (!info) {
    try { info = await api.get('/api/info'); } catch (_) { info = {}; }
  }
  render();
}

export function init() {
  document.addEventListener('lp:lang', () => { if (info) render(); });
}

function render() {
  const T = TEXT[lang()];
  const origin = location.origin;
  const tokenHeader = info.auth_required_for_changes ? ' \\\n  -H "Authorization: Bearer $LIBREPLUME_TOKEN"' : '';
  const tokenField = h('input', { type: 'password', 'aria-label': 'Token', autocomplete: 'off' });
  try { tokenField.value = localStorage.getItem('lp.token') || ''; } catch (_) { /* ignore */ }

  document.getElementById('docs').replaceChildren(
    h('div', { class: 'card' },
      h('h2', {}, T.mcpTitle),
      h('p', { class: 'muted' }, T.mcpIntro),
      h('h3', {}, T.mcpHttp),
      code(`claude mcp add --transport http libreplume ${origin}/mcp${info.auth_required_for_changes ? ' \\\n  --header "Authorization: Bearer $LIBREPLUME_TOKEN"' : ''}`),
      code(JSON.stringify({ mcpServers: { libreplume: { type: 'http', url: `${origin}/mcp`, ...(info.auth_required_for_changes ? { headers: { Authorization: 'Bearer <token>' } } : {}) } } }, null, 2)),
      h('h3', {}, T.mcpStdio),
      code(JSON.stringify({ mcpServers: { libreplume: { command: 'docker', args: ['exec', '-i', 'libreplume', 'libreplume', 'mcp'] } } }, null, 2)),
      h('h3', {}, T.tools),
      h('p', {}, ...TOOLS.flatMap((tool, i) => [i ? ' · ' : '', h('code', {}, tool)]))),
    h('div', { class: 'card' },
      h('h2', {}, T.restTitle),
      h('p', { class: 'muted' }, T.restIntro),
      h('table', {}, h('tbody', {}, ...ENDPOINTS.map(([route, body]) => h('tr', {}, h('td', {}, route), h('td', {}, body))))),
      code(`curl -s ${origin}/api/check -H 'Content-Type: application/json'${tokenHeader} \\\n  -d '{"text": "Il à mangé les pomme.", "language": "fr"}'`),
      code(`curl -s ${origin}/api/conjugate/prendre`)),
    h('div', { class: 'card' },
      h('h2', {}, T.ltTitle),
      h('p', { class: 'muted' }, T.ltIntro),
      code(`curl -s ${origin}/v2/check -d language=fr --data-urlencode "text=Ils mangeons."`)),
    h('div', { class: 'card' },
      h('h2', {}, T.extTitle),
      h('p', { class: 'muted' }, T.extIntro),
      code(origin)),
    h('div', { class: 'card' },
      h('h2', {}, T.authTitle),
      h('p', { class: 'muted' }, info.auth_required_for_changes ? T.authOn : T.authOff),
      h('form', { class: 'inline-form', onsubmit: (e) => {
        e.preventDefault();
        try { localStorage.setItem('lp.token', tokenField.value.trim()); } catch (_) { /* ignore */ }
        toast('✓');
      } }, tokenField,
      h('button', { class: 'btn primary', type: 'submit' }, T.setToken),
      h('button', { class: 'btn ghost', type: 'button', onclick: () => {
        try { localStorage.removeItem('lp.token'); } catch (_) { /* ignore */ }
        tokenField.value = '';
        toast('✓');
      } }, T.clearToken))),
  );
}
