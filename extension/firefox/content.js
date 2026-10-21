// Content script: checks the text field being edited and offers fixes in a small panel.

(() => {
  if (window.__libreplume) return;
  window.__libreplume = true;

  const DEBOUNCE_MS = 800;
  const MIN_LENGTH = 3;
  const CATEGORIES = ['spelling', 'grammar', 'conjugation', 'typography', 'punctuation', 'style'];
  const BLOCKS = new Set(['P', 'DIV', 'LI', 'UL', 'OL', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6', 'BLOCKQUOTE', 'PRE', 'TR', 'SECTION', 'ARTICLE', 'TABLE']);
  const msg = (k, ...s) => browser.i18n.getMessage(k, s) || k;

  const state = new WeakMap(); // field -> { matches, text, timer, version }
  const ignored = new Set();
  let settings = null;
  let active = null;
  let badge = null;
  let panel = null;
  let theme = null;

  // ------------------------------------------------------------------ setup

  browser.runtime.sendMessage({ type: 'settings' }).then((s) => {
    settings = s;
    const host = location.hostname;
    if (s.excluded.some((d) => d && (host === d || host.endsWith(`.${d}`)))) return;
    browser.storage.local.get('theme').then((r) => { theme = r.theme || null; });
    document.addEventListener('focusin', onFocus, true);
    document.addEventListener('input', onInput, true);
    // A field may already be focused when the script starts (e.g. a compose window opened with the cursor in it).
    const focused = editableRoot(deepActiveElement());
    if (focused) onFocus({ composedPath: () => [focused], target: focused });
    window.addEventListener('scroll', schedulePosition, true);
    window.addEventListener('resize', schedulePosition);
  }).catch(() => {});

  browser.runtime.onMessage.addListener((message) => {
    if (message?.type === 'checkActiveField') {
      // Explicit request: check the focused field even if heuristics would skip it.
      const field = editableRoot(deepActiveElement(), { force: true });
      if (field) {
        active = field;
        watch(field);
        check(field).then(() => openPanel());
      }
    } else if (message?.type === 'checkSelection') {
      const text = String(window.getSelection() || '').trim();
      if (text) browser.runtime.sendMessage({ type: 'check', text }).then((r) => openReadOnlyPanel(text, r));
    }
  });

  /** Code editors, where prose checking makes no sense. */
  const CODE_EDITORS = '.cm-editor, .CodeMirror, .monaco-editor, .ace_editor, .code-editor, [role="code"]';

  /** The focused element, looking inside shadow roots. */
  function deepActiveElement() {
    let el = document.activeElement;
    while (el?.shadowRoot?.activeElement) el = el.shadowRoot.activeElement;
    return el;
  }

  /** The element that really received an event, inside open shadow roots. */
  function eventTarget(e) {
    return (e.composedPath && e.composedPath()[0]) || e.target;
  }

  /**
   * The editable element to check for a focused node, or null.
   *
   * Rich mail editors (Outlook, …) often set spellcheck="false" because they ship their own
   * proofing, so that attribute is only honoured on plain text areas and code editors.
   */
  function editableRoot(node, { force = false } = {}) {
    // nodeType rather than instanceof: elements of other frames come from another realm.
    if (!node || node.nodeType !== Node.ELEMENT_NODE) return null;
    if (node.tagName === 'TEXTAREA') {
      if (node.readOnly || node.disabled) return null;
      return force || node.getAttribute('spellcheck') !== 'false' ? node : null;
    }
    if (node.isContentEditable) {
      let root = node;
      while (root.parentElement && root.parentElement.isContentEditable) root = root.parentElement;
      if (!force && root.closest(CODE_EDITORS)) return null;
      return root;
    }
    return null;
  }

  /** Some editors insert text themselves without firing input events: watch the DOM too. */
  const watched = new WeakSet();
  function watch(field) {
    if (field.tagName === 'TEXTAREA' || watched.has(field)) return;
    watched.add(field);
    new MutationObserver(() => changed(field, false)).observe(field, { childList: true, subtree: true, characterData: true });
  }

  function onFocus(e) {
    const field = editableRoot(eventTarget(e));
    if (!field) return;
    watch(field);
    if (field === active) return;
    active = field;
    closePanel();
    const st = state.get(field);
    if (st) updateBadge(field);
    else if (settings.autoCheck && textOf(field).text.trim().length >= MIN_LENGTH) check(field);
    else hideBadge();
  }

  function onInput(e) {
    const field = editableRoot(eventTarget(e));
    if (field) changed(field, true);
  }

  /** Called whenever the text of a field may have changed (typed, or modified by the page). */
  function changed(field, typed) {
    if (!settings) return;
    if (typed) active = field;
    const st = ensureState(field);
    if (textOf(field).text === st.text) return;
    shiftMatches(field, st);
    if (!settings.autoCheck) return;
    clearTimeout(st.timer);
    st.timer = setTimeout(() => check(field), DEBOUNCE_MS);
  }

  function ensureState(field) {
    let st = state.get(field);
    if (!st) {
      st = { matches: [], text: '', timer: 0, version: 0 };
      state.set(field, st);
    }
    return st;
  }

  // ------------------------------------------------------------------ text extraction

  /** Text of a field and, for rich editors, the text nodes it is made of. */
  function textOf(field) {
    if (field.tagName === 'TEXTAREA') return { text: field.value, segments: null };
    const segments = [];
    let text = '';
    const newline = () => { if (text && !text.endsWith('\n')) text += '\n'; };
    const walk = (node) => {
      for (const child of node.childNodes) {
        if (child.nodeType === Node.TEXT_NODE) {
          segments.push({ node: child, start: text.length });
          text += child.data;
        } else if (child.nodeType === Node.ELEMENT_NODE) {
          if (child.tagName === 'BR') { text += '\n'; continue; }
          if (['SCRIPT', 'STYLE', 'TEMPLATE'].includes(child.tagName)) continue;
          const block = BLOCKS.has(child.tagName);
          if (block) newline();
          walk(child);
          if (block) newline();
        }
      }
    };
    walk(field);
    return { text, segments };
  }

  function domPoint(segments, offset) {
    for (let i = segments.length - 1; i >= 0; i--) {
      const s = segments[i];
      if (offset >= s.start) return { node: s.node, offset: Math.min(offset - s.start, s.node.data.length) };
    }
    return null;
  }

  function rangeFor(field, offset, length) {
    const { segments } = textOf(field);
    const a = domPoint(segments, offset);
    const b = domPoint(segments, offset + length);
    if (!a || !b) return null;
    const r = document.createRange();
    r.setStart(a.node, a.offset);
    r.setEnd(b.node, b.offset);
    return r;
  }

  /** Keeps matches aligned with the text while the user types. */
  function shiftMatches(field, st) {
    const now = textOf(field).text;
    const old = st.text;
    let start = 0;
    while (start < old.length && start < now.length && old[start] === now[start]) start++;
    let eo = old.length;
    let en = now.length;
    while (eo > start && en > start && old[eo - 1] === now[en - 1]) { eo--; en--; }
    const delta = en - eo;
    st.matches = st.matches.filter((m) => m.offset + m.length <= start || m.offset >= eo)
      .map((m) => (m.offset >= eo ? { ...m, offset: m.offset + delta } : m));
    st.text = now;
    paintHighlights(field, st);
    if (field === active) updateBadge(field);
    if (panel && field === active) renderPanel();
  }

  // ------------------------------------------------------------------ checking

  async function check(field) {
    const st = ensureState(field);
    const { text } = textOf(field);
    const version = ++st.version;
    if (text.trim().length < MIN_LENGTH) {
      st.matches = [];
      st.text = text;
      paintHighlights(field, st);
      if (field === active) hideBadge();
      return;
    }
    if (field === active) updateBadge(field, true);
    let result;
    try {
      result = await browser.runtime.sendMessage({ type: 'check', text });
    } catch (e) {
      result = { error: String(e?.message || e) };
    }
    if (version !== st.version || textOf(field).text !== text) return;
    if (result?.error) {
      st.error = result.error;
      st.matches = [];
    } else {
      st.error = null;
      st.matches = result.matches;
    }
    st.text = text;
    paintHighlights(field, st);
    if (field === active) {
      updateBadge(field);
      if (panel) renderPanel();
    }
  }

  const keyOf = (m) => `${m.rule_id}\u0000${m.text}`;
  const visible = (st) => st.matches.filter((m) => !ignored.has(keyOf(m)));

  // ------------------------------------------------------------------ highlights (rich editors)

  function paintHighlights(field, st) {
    if (!window.CSS?.highlights || field.tagName === 'TEXTAREA') return;
    const groups = Object.fromEntries(CATEGORIES.map((c) => [c, []]));
    for (const m of visible(st)) {
      const r = rangeFor(field, m.offset, m.length);
      if (r) groups[m.category].push(r);
    }
    for (const c of CATEGORIES) {
      // eslint-disable-next-line no-undef
      CSS.highlights.set(`libreplume-${c}`, new Highlight(...groups[c]));
    }
  }

  // ------------------------------------------------------------------ badge

  function themed(el) {
    if (theme?.variables) {
      for (const [k, v] of Object.entries(theme.variables)) el.style.setProperty(`--lp-${k}`, v);
    }
    return el;
  }

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }

  function updateBadge(field, busy = false) {
    const st = state.get(field);
    if (!st) return;
    if (!badge) {
      badge = themed(el('div', 'libreplume-badge'));
      badge.setAttribute('role', 'button');
      badge.setAttribute('tabindex', '0');
      badge.addEventListener('mousedown', (e) => e.preventDefault()); // keep focus in the field
      badge.addEventListener('click', () => (panel ? closePanel() : openPanel()));
      badge.addEventListener('keydown', (e) => { if (e.key === 'Enter') openPanel(); });
      document.documentElement.append(badge);
    }
    const list = visible(st);
    badge.className = 'libreplume-badge';
    if (busy) badge.classList.add('lp-busy');
    if (st.error) {
      badge.classList.add('lp-error');
      badge.textContent = '!';
      badge.title = msg('serverError');
    } else if (list.length) {
      const worst = list.some((m) => m.severity === 'error') ? 'lp-error' : list.some((m) => m.severity === 'warning') ? 'lp-warning' : 'lp-hint';
      badge.classList.add(worst);
      badge.textContent = String(list.length);
      badge.title = `LibrePlume : ${list.length} ${msg('issues')}`;
    } else {
      badge.classList.add('lp-ok');
      badge.textContent = '✓';
      badge.title = msg('noIssues');
    }
    badge.hidden = false;
    position();
  }

  function hideBadge() {
    if (badge) badge.hidden = true;
    closePanel();
  }

  let positionQueued = false;
  function schedulePosition() {
    if (positionQueued) return;
    positionQueued = true;
    requestAnimationFrame(() => { positionQueued = false; position(); });
  }

  function position() {
    if (!badge || badge.hidden || !active) return;
    if (!active.isConnected) { hideBadge(); return; }
    const r = active.getBoundingClientRect();
    const visibleField = r.width > 40 && r.height > 16 && r.bottom > 0 && r.top < innerHeight;
    badge.style.setProperty('display', visibleField ? 'flex' : 'none', 'important');
    const x = Math.min(r.right, innerWidth) - 30;
    const y = Math.min(r.bottom, innerHeight) - 30;
    badge.style.setProperty('left', `${Math.max(4, x)}px`, 'important');
    badge.style.setProperty('top', `${Math.max(4, y)}px`, 'important');
    if (panel && !panel.classList.contains('lp-readonly')) placePanel(x, y);
  }

  // ------------------------------------------------------------------ panel

  function placePanel(x, y) {
    const w = Math.min(380, innerWidth - 16);
    panel.style.setProperty('width', `${w}px`, 'important');
    panel.style.setProperty('left', `${Math.max(8, Math.min(x + 26 - w, innerWidth - w - 8))}px`, 'important');
    const below = y + 34;
    const h = panel.offsetHeight || 300;
    const top = below + h < innerHeight ? below : Math.max(8, y - h - 8);
    panel.style.setProperty('top', `${top}px`, 'important');
  }

  function openPanel() {
    if (!active) return;
    if (!panel) {
      panel = themed(el('div', 'libreplume-panel'));
      panel.addEventListener('mousedown', (e) => {
        if (!(e.target instanceof HTMLButtonElement)) return;
        e.preventDefault(); // do not steal focus from the field
      });
      document.documentElement.append(panel);
      document.addEventListener('keydown', onEscape, true);
    }
    renderPanel();
    position();
  }

  function closePanel() {
    panel?.remove();
    panel = null;
    document.removeEventListener('keydown', onEscape, true);
  }

  function onEscape(e) {
    if (e.key === 'Escape') closePanel();
  }

  function header(count, onFixAll) {
    const head = el('div', 'lp-head');
    head.append(el('strong', 'lp-title', 'LibrePlume'), el('span', 'lp-count', count ? `${count} ${msg('issues')}` : msg('noIssues')));
    if (onFixAll) {
      const fix = el('button', 'lp-btn lp-primary', msg('fixAll'));
      fix.addEventListener('click', onFixAll);
      head.append(fix);
    }
    const close = el('button', 'lp-btn lp-close', '×');
    close.title = msg('close');
    close.addEventListener('click', closePanel);
    head.append(close);
    return head;
  }

  function issueItem(m, { onApply, onIgnore, onAddWord }) {
    const item = el('div', `lp-issue lp-${m.category}`);
    item.append(el('div', 'lp-cat', msg(`cat_${m.category}`)), el('div', 'lp-msg', m.message));
    const fixes = el('div', 'lp-fixes');
    if (m.text.trim() && m.replacements.length) fixes.append(el('del', 'lp-del', m.text));
    for (const r of m.replacements.slice(0, 4)) {
      const b = el('button', 'lp-sugg', r === '' ? msg('remove') : r.replace(/[  ]/g, '⍽'));
      if (onApply) b.addEventListener('click', () => onApply(r));
      else b.disabled = true;
      fixes.append(b);
    }
    item.append(fixes);
    const actions = el('div', 'lp-actions');
    if (onIgnore) {
      const ig = el('button', 'lp-link', msg('ignore'));
      ig.addEventListener('click', onIgnore);
      actions.append(ig);
    }
    if (onAddWord && m.category === 'spelling') {
      const add = el('button', 'lp-link', msg('addWord'));
      add.addEventListener('click', onAddWord);
      actions.append(add);
    }
    item.append(actions);
    return item;
  }

  function renderPanel() {
    if (!panel || !active) return;
    const field = active;
    const st = state.get(field);
    panel.replaceChildren();
    if (st?.error) {
      panel.append(header(0), el('div', 'lp-empty', msg('serverError')));
      return;
    }
    const list = st ? visible(st) : [];
    const fixable = list.filter((m) => m.severity !== 'hint' && m.replacements.length);
    panel.append(header(list.length, fixable.length ? () => fixAll(field) : null));
    const body = el('div', 'lp-body');
    if (!list.length) body.append(el('div', 'lp-empty', `✓ ${msg('noIssues')}`));
    for (const m of list) {
      body.append(issueItem(m, {
        onApply: (r) => applyFix(field, m, r),
        onIgnore: () => { ignored.add(keyOf(m)); refresh(field); },
        onAddWord: async () => {
          await browser.runtime.sendMessage({ type: 'addWord', word: m.text });
          st.matches = st.matches.filter((x) => !(x.category === 'spelling' && x.text === m.text));
          refresh(field);
        },
      }));
    }
    panel.append(body);
  }

  function openReadOnlyPanel(text, result) {
    closePanel();
    panel = themed(el('div', 'libreplume-panel lp-readonly'));
    document.documentElement.append(panel);
    document.addEventListener('keydown', onEscape, true);
    const list = result?.matches || [];
    panel.append(header(list.length));
    const body = el('div', 'lp-body');
    if (result?.error) body.append(el('div', 'lp-empty', msg('serverError')));
    else if (!list.length) body.append(el('div', 'lp-empty', `✓ ${msg('noIssues')}`));
    for (const m of list) body.append(issueItem(m, {}));
    panel.append(body);
    const sel = window.getSelection();
    const rect = sel.rangeCount ? sel.getRangeAt(0).getBoundingClientRect() : { left: 20, bottom: 20 };
    panel.style.setProperty('width', `${Math.min(380, innerWidth - 16)}px`, 'important');
    panel.style.setProperty('left', `${Math.max(8, Math.min(rect.left, innerWidth - 396))}px`, 'important');
    panel.style.setProperty('top', `${Math.min(rect.bottom + 8, innerHeight - 320)}px`, 'important');
  }

  function refresh(field) {
    paintHighlights(field, state.get(field));
    updateBadge(field);
    renderPanel();
  }

  // ------------------------------------------------------------------ applying fixes

  /** Replaces [offset, offset+length) through editing commands so that undo works. */
  function replaceIn(field, offset, length, replacement) {
    field.focus({ preventScroll: true });
    if (field.tagName === 'TEXTAREA') {
      field.setSelectionRange(offset, offset + length);
      if (!document.execCommand('insertText', false, replacement)) {
        field.setRangeText(replacement, offset, offset + length, 'end');
        field.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertReplacementText', data: replacement }));
      }
      return true;
    }
    const range = rangeFor(field, offset, length);
    if (!range) return false;
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    if (!document.execCommand('insertText', false, replacement)) {
      range.deleteContents();
      range.insertNode(document.createTextNode(replacement));
      field.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertReplacementText', data: replacement }));
    }
    return true;
  }

  function applyFix(field, m, replacement) {
    const { text } = textOf(field);
    if (text.slice(m.offset, m.offset + m.length) !== m.text) {
      check(field);
      return;
    }
    replaceIn(field, m.offset, m.length, replacement);
  }

  function fixAll(field) {
    const st = state.get(field);
    const fixes = visible(st)
      .filter((m) => m.severity !== 'hint' && m.replacements.length)
      .sort((a, b) => b.offset - a.offset);
    let lastStart = Infinity;
    for (const m of fixes) {
      if (m.offset + m.length > lastStart) continue; // overlapping
      const { text } = textOf(field);
      if (text.slice(m.offset, m.offset + m.length) !== m.text) continue;
      replaceIn(field, m.offset, m.length, m.replacements[0]);
      lastStart = m.offset;
    }
  }
})();
