// Small client for the LibrePlume REST API.

import { t } from './i18n.js';

function token() {
  try { return localStorage.getItem('lp.token') || ''; } catch (_) { return ''; }
}

/** Asks the user for the server token. Resolves to true when one was entered. */
function askToken() {
  const dialog = document.getElementById('token-dialog');
  const input = document.getElementById('token-input');
  input.value = token();
  return new Promise((resolve) => {
    dialog.addEventListener('close', () => {
      if (dialog.returnValue === 'ok' && input.value.trim()) {
        try { localStorage.setItem('lp.token', input.value.trim()); } catch (_) { /* ignore */ }
        resolve(true);
      } else {
        resolve(false);
      }
    }, { once: true });
    dialog.showModal();
  });
}

export async function request(method, path, body, { signal, retry = true } = {}) {
  const headers = { Accept: 'application/json' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  const tok = token();
  if (tok) headers.Authorization = `Bearer ${tok}`;
  const res = await fetch(path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body), signal });
  if (res.status === 401 && retry && await askToken()) {
    return request(method, path, body, { signal, retry: false });
  }
  if (res.status === 204) return null;
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error || `${t('error')} ${res.status}`);
  return data;
}

export const api = {
  get: (path, opts) => request('GET', path, undefined, opts),
  post: (path, body, opts) => request('POST', path, body, opts),
  put: (path, body, opts) => request('PUT', path, body, opts),
  patch: (path, body, opts) => request('PATCH', path, body, opts),
  del: (path, body, opts) => request('DELETE', path, body, opts),
};
