// Toolbar popup: check a pasted or selected text and open the conjugator.

const $ = (id) => document.getElementById(id);
let corrected = '';

async function init() {
  localize();
  applyServerTheme();
  const settings = await loadSettings();
  const status = $('status');
  status.textContent = new URL(settings.server).host;
  apiRequest('GET', '/api/health')
    .then(() => status.classList.add('ok'))
    .catch(() => { status.classList.add('err'); status.title = msg('serverError'); });

  const pending = (await browser.storage.session.get('pendingText')).pendingText;
  if (pending) {
    $('text').value = pending;
    await browser.storage.session.remove('pendingText');
    run();
  }
  $('text').focus();
  $('check').addEventListener('click', run);
  $('text').addEventListener('keydown', (e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) run(); });
  $('copy').addEventListener('click', async () => {
    await navigator.clipboard.writeText(corrected);
    $('copy').textContent = msg('copied');
  });
  $('conj').addEventListener('submit', (e) => {
    e.preventDefault();
    const verb = $('verb').value.trim();
    if (verb) browser.runtime.sendMessage({ type: 'openApp', hash: `#/conjugate/${encodeURIComponent(verb)}` }).then(() => window.close());
  });
  $('check-field').addEventListener('click', async (e) => {
    e.preventDefault();
    const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
    if (tab) await browser.tabs.sendMessage(tab.id, { type: 'checkActiveField' }).catch(() => {});
    window.close();
  });
  $('open-app').addEventListener('click', (e) => { e.preventDefault(); browser.runtime.sendMessage({ type: 'openApp' }).then(() => window.close()); });
  $('options').addEventListener('click', (e) => { e.preventDefault(); browser.runtime.openOptionsPage(); window.close(); });
}

async function run() {
  const text = $('text').value;
  if (!text.trim()) return;
  $('count').textContent = msg('checking');
  try {
    const result = await checkText(text);
    render(text, result.matches);
  } catch (_) {
    $('count').textContent = msg('serverError');
  }
}

function render(text, matches) {
  $('count').textContent = matches.length ? `${matches.length} ${msg('issues')}` : `✓ ${msg('noIssues')}`;
  // Corrected version: first suggestion of every error and warning.
  let out = '';
  let cursor = 0;
  for (const m of matches) {
    if (m.severity === 'hint' || !m.replacements.length || m.offset < cursor) continue;
    out += text.slice(cursor, m.offset) + m.replacements[0];
    cursor = m.offset + m.length;
  }
  corrected = out + text.slice(cursor);
  $('copy').hidden = corrected === text;
  $('copy').textContent = msg('copy');

  $('issues').replaceChildren(...matches.map((m) => {
    const item = document.createElement('div');
    item.className = 'issue';
    item.style.setProperty('--c', `var(--${m.category})`);
    const cat = Object.assign(document.createElement('div'), { className: 'cat', textContent: msg(`cat_${m.category}`) });
    const message = Object.assign(document.createElement('div'), { className: 'msg', textContent: m.message });
    const fixes = document.createElement('div');
    if (m.replacements.length) fixes.append(Object.assign(document.createElement('del'), { textContent: m.text }));
    for (const r of m.replacements.slice(0, 4)) {
      const b = Object.assign(document.createElement('button'), { className: 'sugg', textContent: r || msg('remove') });
      b.addEventListener('click', () => {
        const value = $('text').value;
        if (value.slice(m.offset, m.offset + m.length) !== m.text) return run();
        $('text').value = value.slice(0, m.offset) + r + value.slice(m.offset + m.length);
        run();
      });
      fixes.append(b);
    }
    item.append(cat, message, fixes);
    return item;
  }));
}

init();
