// Native e2e for Options → Language: English by default, German and Dutch
// switched live through the Options dialog (menus, toolbar, Options pages and
// dialogs), no catalog text left untranslated, no labels that overflow in
// German or Dutch where English fits (light and dark), a catalog added to the
// languages folder of the configuration folder, and English restored live.
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 20000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const i18n = () => window.__OPENTAG_I18N__;
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const openDialog = () => document.querySelector('dialog[open]');
  const closeDialogs = async () => { for (let i = 0; i < 5 && openDialog(); i++) { openDialog().close(); await sleep(120); } };
  const openOptions = async () => {
    for (let attempt = 0; attempt < 3; attempt++) {
      document.body.focus(); key('o', mod, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null);
      if (dlg) return dlg;
      await sleep(300);
    }
    throw new Error('options dialog did not open');
  };
  const setLanguage = async value => {
    const dlg = await openOptions();
    dlg.querySelector('[data-page="language"]').click(); await sleep(150);
    const select = dlg.querySelector('[data-option="pages.language.language"]');
    if (value !== undefined) {
      if (![...select.options].some(o => o.value === value)) throw new Error(`no ${value} in ${[...select.options].map(o => o.value)}`);
      select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); await sleep(80);
    }
    dlg.querySelector('#options-ok').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
    await idle();
  };
  const menus = () => [...document.querySelectorAll('.menubar > details.menu')];
  const summaries = () => menus().map(m => m.querySelector('summary').textContent.trim());
  const sameList = (actual, expected, what) => { if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${what}: ${JSON.stringify(actual)}`); return actual.join(', '); };

  // Text people read: text nodes and the attributes the translator covers, outside translate="no".
  const ATTRIBUTES = ['title', 'aria-label', 'placeholder', 'alt'];
  const readable = root => {
    const out = [];
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, { acceptNode: n => n instanceof Element && n.matches('script,style,textarea,[translate="no"]') ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT });
    for (let n = walker.currentNode; n; n = walker.nextNode()) {
      if (n.nodeType === Node.TEXT_NODE) { if (/\p{L}/u.test(n.data)) out.push(n.data.replace(/\s+/g, ' ').trim()); }
      else for (const a of ATTRIBUTES) { const v = n.getAttribute(a); if (v && /\p{L}/u.test(v)) out.push(v.replace(/\s+/g, ' ').trim()); }
    }
    return out;
  };
  // Catalog text still shown in English: translating it again would change it.
  const leftovers = root => [...new Set(readable(root).filter(text => i18n().translate(text) !== text))];
  // English-looking text the catalogs do not know yet; reported for coverage, not a failure.
  const unknown = new Set();
  const collectUnknown = root => { for (const text of readable(root)) if (text.length > 3 && /\b(the|and|of|to|with|for|files?|tags?|select|save|selected|field|value|cover|not|no)\b/i.test(text) && /^[\x20-\x7e…→—–·’“”‘]+$/.test(text)) unknown.add(text); };

  // Labels whose text no longer fits: compared with the same view in English.
  const SELECTORS = 'button, summary, label, legend, h2, th, header, [role="treeitem"], .menu-list > *, .field > span, p.menu-note';
  const measure = root => {
    const over = [];
    [...root.querySelectorAll(SELECTORS)].forEach((el, index) => {
      if (!el.getClientRects().length || !el.clientWidth) return;
      if (el.scrollWidth > el.clientWidth + 2) over.push(`${index}:${el.tagName.toLowerCase()}${el.dataset.op ? '#' + el.dataset.op : ''} “${el.textContent.trim().slice(0, 40)}”`);
    });
    const box = root.getBoundingClientRect();
    if (root.matches('dialog') && (box.right > innerWidth + 1 || box.left < -1)) over.push(`dialog ${Math.round(box.width)}px wider than the window`);
    const content = root.querySelector('.dialog-content');
    if (content && content.scrollWidth > content.clientWidth + 2) over.push(`dialog content scrolls sideways (${content.scrollWidth} > ${content.clientWidth})`);
    return over;
  };
  const baseline = new Map();
  const views = [];
  // Every view: open it, check it, close it. `language` is '' for the English baseline.
  const check = async (language, name, root, problems) => {
    const over = measure(root);
    if (!language) { baseline.set(name, new Set(over.map(o => o.split(' ')[0]))); return; }
    const base = baseline.get(name) || new Set();
    const added = over.filter(o => !base.has(o.split(' ')[0]));
    if (added.length) problems.push(`${name}: ${added.join('; ')}`);
    const left = leftovers(root);
    if (left.length) problems.push(`${name} untranslated: ${left.slice(0, 8).join(' | ')}`);
    collectUnknown(root);
  };
  const DIALOG_OPS = ['tags-filename', 'filename-tags', 'filename-filename', 'text-tags', 'tag-tag', 'number', 'groups', 'action', 'extended', 'columns', 'panel-options', 'field-values', 'copy-text', 'remove-tag-types', 'help-shortcuts', 'cover-manage', 'about', 'find', 'playlist-generate', 'dir-manage'];
  const sweep = async (language, { dialogs = true } = {}) => {
    const problems = [];
    // Main window: toolbar, tag panel, status bar.
    await check(language, 'window', document.querySelector('.app'), problems);
    for (const [i, menu] of menus().entries()) {
      menu.open = true; await sleep(60);
      await check(language, `menu ${i}`, menu, problems);
      menu.querySelectorAll('details').forEach(d => { d.open = true; });
      await sleep(30);
      await check(language, `menu ${i} submenus`, menu, problems);
      menu.querySelectorAll('details').forEach(d => { d.open = false; });
      menu.open = false;
    }
    const dlg = await openOptions();
    for (const item of [...dlg.querySelectorAll('[role="treeitem"]')]) {
      item.click(); await sleep(80);
      await check(language, `options ${item.dataset.page}`, dlg, problems);
    }
    dlg.querySelector('#options-cancel').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
    if (dialogs) {
      for (const name of DIALOG_OPS) {
        const button = op(name);
        if (!button || button.disabled) { if (!language) views.push(`${name} (unavailable)`); continue; }
        button.click();
        const dialog = await waitFor(() => openDialog(), 3000, name).catch(() => null);
        if (!dialog) { if (!language) views.push(`${name} (no dialog)`); continue; }
        await sleep(250);
        await check(language, `dialog ${name}`, dialog, problems);
        if (!language) views.push(name);
        await closeDialogs(); await idle();
      }
    }
    return problems;
  };
  const selectAll = async () => { document.body.focus(); key('a', mod, document.body); await waitFor(() => rows().length && rows().every(r => r.classList.contains('selected')), 3000, 'all selected'); };

  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    await waitFor(() => i18n(), 5000, 'translation layer');
    await selectAll();

    await step('English is shown on an English system and catalog text is complete', async () => {
      const language = i18n().language();
      if (language !== 'en') throw new Error(`language ${language}, system ${navigator.languages}`);
      if (document.documentElement.lang !== 'en') throw new Error('html lang ' + document.documentElement.lang);
      return sameList(summaries(), ['File', 'Edit', 'View', 'Convert', 'Actions', 'Tag Sources', 'Tools', 'Help'], 'menus');
    });
    await step('Options → Language lists the system default, English, Deutsch and Nederlands', async () => {
      const dlg = await openOptions();
      dlg.querySelector('[data-page="language"]').click(); await sleep(150);
      const select = dlg.querySelector('[data-option="pages.language.language"]');
      const options = [...select.options].map(o => `${o.value}=${o.textContent}`);
      dlg.querySelector('#options-cancel').click(); await sleep(150);
      return sameList(options, ['system=System default (English)', 'en=English', 'de=Deutsch', 'nl=Nederlands'], 'choices');
    });
    await step('English baseline of menus, Options pages and dialogs', async () => { await sweep(''); return `${baseline.size} views; dialogs: ${views.join(', ')}`; });

    await step('German applies live to menus, toolbar, tag panel and status bar', async () => {
      await setLanguage('de');
      await waitFor(() => i18n().language() === 'de', 5000, 'German');
      if (document.documentElement.lang !== 'de') throw new Error('html lang ' + document.documentElement.lang);
      const save = op('save'), shortcut = menus()[0].querySelector('button[data-op="save"] small')?.textContent;
      if (!/Tags speichern/.test(save.textContent)) throw new Error('save button: ' + save.textContent);
      if (shortcut !== '⌘/Strg S') throw new Error('shortcut: ' + shortcut);
      if (!/Tag-Panel/.test(document.querySelector('.panel-title').textContent)) throw new Error('panel title: ' + document.querySelector('.panel-title').textContent);
      return sameList(summaries(), ['Datei', 'Bearbeiten', 'Ansicht', 'Konvertieren', 'Aktionen', 'Tag-Quellen', 'Werkzeuge', 'Hilfe'], 'menus');
    });
    await step('German: every view is translated and no label overflows where English fits', async () => {
      const problems = await sweep('de');
      if (problems.length) throw new Error(problems.join(' || '));
      return `${baseline.size} views`;
    });
    await step('German in dark mode: menus and Options pages still fit', async () => {
      const appearance = document.querySelector('#appearance');
      appearance.value = 'dark'; appearance.dispatchEvent(new Event('change', { bubbles: true }));
      await waitFor(() => document.documentElement.dataset.theme === 'dark', 3000, 'dark mode');
      const problems = await sweep('de', { dialogs: false });
      const restore = document.querySelector('#appearance'); restore.value = 'light'; restore.dispatchEvent(new Event('change', { bubbles: true }));
      await waitFor(() => document.documentElement.dataset.theme === 'light', 3000, 'light mode');
      if (problems.length) throw new Error(problems.join(' || '));
      return 'dark and back to light';
    });

    await step('Dutch applies live and every view is translated and fits', async () => {
      await setLanguage('nl');
      await waitFor(() => i18n().language() === 'nl', 5000, 'Dutch');
      const menusShown = sameList(summaries(), ['Bestand', 'Bewerken', 'Weergave', 'Converteren', 'Acties', 'Tagbronnen', 'Tools', 'Help'], 'menus');
      const problems = await sweep('nl');
      if (problems.length) throw new Error(problems.join(' || '));
      const dlg = await openOptions();
      const header = dlg.querySelector('#options-title').textContent;
      dlg.querySelector('#options-cancel').click(); await sleep(150);
      if (header !== 'Opties') throw new Error('options header: ' + header);
      return menusShown;
    });

    await step('a catalog in the languages folder adds a language, with English for what it lacks', async () => {
      const folder = await inv('prepare_languages_folder', { template: JSON.stringify({ '@meta': { name: 'English', locale: 'en' } }) });
      const separator = folder.includes('\\') ? '\\' : '/';
      const catalog = { '@meta': { name: 'Esperanto (test)', locale: 'eo' }, File: 'Dosiero', 'Save tags': 'Konservi etikedojn', Edit: 'Redakti {wrong}' };
      await inv('export_value_list', { path: folder + separator + 'eo.json', values: [JSON.stringify(catalog)] });
      await setLanguage();
      await setLanguage('eo');
      await waitFor(() => i18n().language() === 'eo', 5000, 'Esperanto');
      const shown = summaries();
      if (shown[0] !== 'Dosiero' || shown[1] !== 'Edit' || !/Konservi etikedojn/.test(op('save').textContent)) throw new Error(JSON.stringify(shown));
      const dlg = await openOptions();
      dlg.querySelector('[data-page="language"]').click(); await sleep(150);
      const note = dlg.querySelector('.options-page').textContent;
      dlg.querySelector('#options-cancel').click(); await sleep(150);
      if (!/eo\.json: \d+ missing, 1 invalid/.test(note)) throw new Error('warning not shown: ' + note.slice(0, 300));
      return `${folder}: ${shown.slice(0, 3).join(', ')}`;
    });

    await step('English is restored live', async () => {
      await setLanguage('en');
      await waitFor(() => i18n().language() === 'en', 5000, 'English');
      if (!/Save tags/.test(op('save').textContent)) throw new Error('save button: ' + op('save').textContent);
      const left = readable(document.body).filter(text => /\b(Datei|Bearbeiten|Bestand|Dosiero|speichern|opslaan)\b/.test(text));
      if (left.length) throw new Error('still translated: ' + left.slice(0, 5).join(' | '));
      return sameList(summaries(), ['File', 'Edit', 'View', 'Convert', 'Actions', 'Tag Sources', 'Tools', 'Help'], 'menus');
    });
    if (unknown.size) await report(`INFO text without a catalog entry (${unknown.size}): ${[...unknown].slice(0, 60).join(' | ')}`);
  } catch (e) {
    await report(`FATAL ${e && e.stack || e}`);
  }
  await finish();
})();
