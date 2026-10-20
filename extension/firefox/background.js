// Background script: talks to the LibrePlume server for content scripts and menus.

browser.runtime.onMessage.addListener((message) => {
  switch (message?.type) {
    case 'check':
      return checkText(message.text, message.language).catch((e) => ({ error: e.message }));
    case 'addWord':
      return apiRequest('POST', '/api/dictionary', { word: message.word }).catch((e) => ({ error: e.message }));
    case 'settings':
      return loadSettings();
    case 'openApp':
      return loadSettings().then((s) => browser.tabs.create({ url: `${s.server}/${message.hash || ''}` }));
    default:
      return undefined;
  }
});

function createMenus() {
  browser.contextMenus.removeAll().then(() => {
    browser.contextMenus.create({ id: 'lp-check', title: browser.i18n.getMessage('menuCheck', '%s'), contexts: ['selection'] });
    browser.contextMenus.create({ id: 'lp-conjugate', title: browser.i18n.getMessage('menuConjugate', '%s'), contexts: ['selection'] });
  });
}

browser.runtime.onInstalled.addListener(createMenus);
browser.runtime.onStartup.addListener(createMenus);

browser.contextMenus.onClicked.addListener(async (info, tab) => {
  const text = (info.selectionText || '').trim();
  if (!text) return;
  if (info.menuItemId === 'lp-conjugate') {
    const settings = await loadSettings();
    const verb = text.split(/\s+/)[0].toLowerCase();
    const lang = /^(to\s)?[a-z]+$/.test(text.toLowerCase()) && settings.language === 'en' ? 'en' : 'fr';
    browser.tabs.create({ url: `${settings.server}/#/conjugate/${encodeURIComponent(verb)}?lang=${lang}` });
    return;
  }
  // Hand the selection to the popup, then open it.
  await browser.storage.session.set({ pendingText: text });
  try {
    await browser.action.openPopup();
  } catch (_) {
    browser.tabs.sendMessage(tab.id, { type: 'checkSelection' }).catch(() => {});
  }
});

browser.commands.onCommand.addListener(async (command) => {
  if (command !== 'check-field') return;
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  if (tab) browser.tabs.sendMessage(tab.id, { type: 'checkActiveField' }).catch(() => {});
});
