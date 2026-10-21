// Options page.

const $ = (id) => document.getElementById(id);

async function init() {
  localize();
  applyServerTheme();
  const s = await loadSettings();
  $('server').value = s.server;
  $('token').value = s.token;
  $('language').value = s.language;
  $('autoCheck').checked = s.autoCheck;
  $('showStyle').checked = s.showStyle;
  $('excluded').value = s.excluded.join('\n');

  $('form').addEventListener('submit', async (e) => {
    e.preventDefault();
    await save();
    $('result').textContent = msg('optSaved');
  });
  $('test').addEventListener('click', async () => {
    await save();
    try {
      const info = await apiRequest('GET', '/api/health');
      $('result').textContent = msg('optOk', info.version);
      applyServerTheme();
    } catch (_) {
      $('result').textContent = msg('serverError');
    }
  });
  $('permission').addEventListener('click', () => browser.permissions.request({ origins: ['<all_urls>'] }));
}

async function save() {
  let server = $('server').value.trim().replace(/\/+$/, '');
  if (server && !/^https?:\/\//.test(server)) server = `http://${server}`;
  await browser.storage.sync.set({
    server,
    token: $('token').value.trim(),
    language: $('language').value,
    autoCheck: $('autoCheck').checked,
    showStyle: $('showStyle').checked,
    excluded: $('excluded').value.split('\n').map((d) => d.trim().toLowerCase()).filter(Boolean),
  });
}

init();
