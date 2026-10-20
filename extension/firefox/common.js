// Shared by the background script, the popup and the options page.

const DEFAULTS = {
  server: 'http://localhost:8080',
  token: '',
  language: 'auto',
  autoCheck: true,
  showStyle: true,
  excluded: [],
};

async function loadSettings() {
  const stored = await browser.storage.sync.get(DEFAULTS);
  return { ...DEFAULTS, ...stored, server: String(stored.server || DEFAULTS.server).replace(/\/+$/, '') };
}

async function apiRequest(method, path, body) {
  const settings = await loadSettings();
  const headers = { Accept: 'application/json' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  if (settings.token) headers.Authorization = `Bearer ${settings.token}`;
  const res = await fetch(`${settings.server}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
    credentials: 'omit',
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data;
}

/** Checks a text with the user's settings. */
async function checkText(text, language) {
  const settings = await loadSettings();
  const body = { text, language: language || settings.language };
  if (!settings.showStyle) body.disabled_categories = ['style'];
  return apiRequest('POST', '/api/check', body);
}

/** Applies the server's default theme to an extension page. */
async function applyServerTheme() {
  try {
    const cached = (await browser.storage.local.get('theme')).theme;
    if (cached) setThemeVariables(cached);
    const res = await apiRequest('GET', '/api/themes');
    let theme = res.themes.find((t) => t.id === res.default) || res.themes[0];
    if (res.default === 'plume' && matchMedia('(prefers-color-scheme: dark)').matches) {
      theme = res.themes.find((t) => t.id === 'encre') || theme;
    }
    setThemeVariables(theme);
    await browser.storage.local.set({ theme });
  } catch (_) {
    // Keep the built-in colours when the server is unreachable.
  }
}

function setThemeVariables(theme) {
  for (const [k, v] of Object.entries(theme.variables || {})) document.documentElement.style.setProperty(`--${k}`, v);
  document.documentElement.style.colorScheme = theme.dark ? 'dark' : 'light';
}

function msg(key, ...subs) {
  return browser.i18n.getMessage(key, subs) || key;
}

function localize(root = document) {
  root.querySelectorAll('[data-msg]').forEach((el) => { el.textContent = msg(el.dataset.msg); });
  root.querySelectorAll('[data-msg-placeholder]').forEach((el) => { el.placeholder = msg(el.dataset.msgPlaceholder); });
  document.documentElement.lang = browser.i18n.getUILanguage().split('-')[0];
}
