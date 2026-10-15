// Actions (Quick) memory, OEM codepages, ANSI Import Text File, Auto-numbering
// memory, the cover art directory for Add cover, Adjust inside Manage covers
// and Tag → Filename clashes, driven through the real UI. Fixtures:
// actions-polish-fixtures.sh. Runs headless in CI (see docs/verification/actions-polish.md).
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const errorsText = () => [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' | ');
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const pathOf = name => row(name).dataset.path;
  const clickRow = async (name, opts = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...opts })); await sleep(300); };
  const selectAll = async () => { await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', { shiftKey: true }); };
  const read = async name => (await inv('scan_files', { paths: [pathOf(name)], recursive: false })).entries[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const top = () => [...document.querySelectorAll('dialog[open]')].at(-1);
  const topWith = (selector, what) => waitFor(() => top()?.querySelector(selector) && top(), 5000, what);
  const set = (el, value) => { if (el.type === 'checkbox') el.checked = value; else el.value = String(value); el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const closed = what => waitFor(() => !document.querySelector('dialog[open]'), 10000, what + ' closed (' + (top()?.querySelector('.dialog-error')?.textContent || '') + ')');
  const quick = async () => { op('action').click(); return topWith('#full-action-type', 'Actions (Quick)'); };
  const chooseType = async (dlg, type) => { set(dlg.querySelector('#full-action-type'), type); await sleep(150); };
  const applyQuick = async (dlg, what) => { dlg.querySelector('[data-keep]').click(); await closed(what); await idle(); await sleep(300); };
  const openOptions = async () => {
    for (let attempt = 0; attempt < 3; attempt++) {
      document.body.focus(); key('o', mod, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null);
      if (dlg) return dlg;
      await sleep(300);
    }
    throw new Error('options dialog did not open');
  };
  const setCoverDirectory = async value => {
    const options = await openOptions();
    options.querySelector('[data-page="directories"]').click(); await sleep(150);
    set(options.querySelector('[data-option="pages.directories.cover_art"]'), value);
    options.querySelector('#options-ok').click(); await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed'); await idle();
  };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); document.querySelectorAll('dialog[open]').forEach(d => d.close()); } };
  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    const album = pathOf('sine-a.mp3').replace(/[\\/][^\\/]+$/, '');

    await step('Actions (Quick) reopens on the last applied type and its settings', async () => {
      await clickRow('sine-a.mp3');
      let dlg = await quick();
      const first = dlg.querySelector('#full-action-type').value;
      await chooseType(dlg, 'replace');
      set(dlg.querySelector('[data-key="field"]'), 'TITLE'); set(dlg.querySelector('[data-key="from"]'), 'Sine'); set(dlg.querySelector('[data-key="to"]'), 'Wave'); set(dlg.querySelector('[data-key="whole_word"]'), true);
      await applyQuick(dlg, 'quick replace');
      const title = (await read('sine-a.mp3')).fields.TITLE?.[0]; if (title !== 'Wave A') throw new Error('TITLE=' + title);
      dlg = await quick();
      const again = ['#full-action-type', '[data-key="field"]', '[data-key="from"]', '[data-key="to"]'].map(s => dlg.querySelector(s).value).join('|');
      const whole = dlg.querySelector('[data-key="whole_word"]').checked;
      dlg.querySelector('[data-close]').click(); await closed('quick');
      if (first !== 'format_value' || again !== 'replace|TITLE|Sine|Wave' || !whole) throw new Error(`first=${first} reopened=${again} whole=${whole}`);
      return `first=${first}; reopened ${again}, whole word kept`;
    });

    await step('Convert Codepage offers OEM 850 and converts DOS bytes', async () => {
      await clickRow('sine-a.mp3');
      let dlg = await quick(); await chooseType(dlg, 'format_value');
      set(dlg.querySelector('[data-key="field"]'), 'ALBUM'); set(dlg.querySelector('[data-key="template"]'), 'Gr$char(148)$char(225)e');
      await applyQuick(dlg, 'format value');
      dlg = await quick(); await chooseType(dlg, 'convert_codepage');
      const codepage = dlg.querySelector('[data-key="codepage"]');
      const offered = ['437', '850', '852'].filter(id => [...codepage.options].some(o => o.value === id));
      set(dlg.querySelector('[data-key="field"]'), 'ALBUM'); set(codepage, '850');
      await applyQuick(dlg, 'convert codepage');
      const value = (await read('sine-a.mp3')).fields.ALBUM?.[0];
      if (offered.length !== 3 || value !== 'Größe') throw new Error(`offered=${offered} ALBUM=${JSON.stringify(value)} errors=${errorsText()}`);
      return `ALBUM=${value}`;
    });

    await step('Import Text File reads a Windows-1252 file without a BOM', async () => {
      await clickRow('sine-b.mp3');
      const dlg = await quick(); await chooseType(dlg, 'import_text_file');
      set(dlg.querySelector('[data-key="field"]'), 'COMMENT'); set(dlg.querySelector('[data-key="filename_template"]'), '%_folderpath%ansi.txt');
      await applyQuick(dlg, 'import text');
      const value = (await read('sine-b.mp3')).fields.COMMENT?.[0];
      if (value !== 'Café € “quoted”') throw new Error(`COMMENT=${JSON.stringify(value)} errors=${errorsText()}`);
      return `COMMENT=${value}`;
    });

    await step('Auto-numbering remembers staged options; Ctrl+Shift+K presets per folder without changing them', async () => {
      await selectAll();
      op('number').click();
      let dlg = await topWith('#number-start', 'numbering');
      set(dlg.querySelector('#number-start'), 7); set(dlg.querySelector('#number-digits'), 3); set(dlg.querySelector('#number-total'), true);
      dlg.querySelector('#preview-number').click();
      await waitFor(() => !dlg.querySelector('#apply-number').disabled, 8000, 'numbering preview');
      dlg.querySelector('#apply-number').click(); await closed('numbering');
      op('save').click(); await idle(); await waitFor(async () => (await read('sine-b.mp3')).fields.TRACK?.[0] === '008/2', 8000, 'saved track numbers');
      await selectAll(); op('number').click();
      dlg = await topWith('#number-start', 'numbering again');
      const reopened = [dlg.querySelector('#number-start').value, dlg.querySelector('#number-digits').value, dlg.querySelector('#number-total').checked, dlg.querySelector('#number-total-value').value, dlg.querySelector('#number-directory').checked].join('|');
      dlg.close(); await sleep(150);
      row('sine-a.mp3').focus(); key('K', { code: 'KeyK', shiftKey: true, ...mod });
      dlg = await topWith('#number-directory', 'Ctrl+Shift+K');
      const perFolder = [dlg.querySelector('#number-start').value, dlg.querySelector('#number-directory').checked, dlg.querySelector('#number-reset-track').checked].join('|');
      dlg.close(); await sleep(150);
      row('sine-a.mp3').focus(); key('k', { code: 'KeyK', ...mod });
      dlg = await topWith('#number-directory', 'Ctrl+K');
      const plain = dlg.querySelector('#number-directory').checked; dlg.close(); await sleep(150);
      if (reopened !== '7|3|true|2|false' || perFolder !== '7|true|true' || plain) throw new Error(`reopened=${reopened} perFolder=${perFolder} plainDirectory=${plain}`);
      return `reopened ${reopened}; Ctrl+Shift+K ${perFolder}`;
    });

    await step('Options → Directories cover art directory is where Add cover starts', async () => {
      await setCoverDirectory('Artwork');
      window.__OPENTAG_E2E_DIALOGS__ = { requests: [], answers: [album + '/Artwork/front.png'] };
      await clickRow('sine-a.mp3'); op('add-cover').click();
      await waitFor(() => /Added cover/.test(status()), 8000, 'added cover (' + errorsText() + ')'); await idle();
      const request = window.__OPENTAG_E2E_DIALOGS__.requests[0];
      const covers = (await read('sine-a.mp3')).covers;
      if (!/^[\\/]Artwork$/.test(request?.defaultPath?.slice(album.length) ?? '') || !request.defaultPath.startsWith(album) || covers.length !== 1 || covers[0].width !== 400) throw new Error(`request=${JSON.stringify(request)} covers=${JSON.stringify(covers.map(c => [c.width, c.height]))}`);
      // Without the option the dialog starts in the working folder (the album has no image of its own).
      await setCoverDirectory('');
      window.__OPENTAG_E2E_DIALOGS__ = { requests: [], answers: [null] };
      await clickRow('sine-b.mp3'); op('add-cover').click();
      await waitFor(() => window.__OPENTAG_E2E_DIALOGS__.requests.length === 1, 5000, 'second dialog request');
      const fallback = String(window.__OPENTAG_E2E_DIALOGS__.requests[0].defaultPath).replace(/[\\/]$/, '');
      delete window.__OPENTAG_E2E_DIALOGS__;
      // The working folder is spelled as it was opened (macOS lists /var/folders files under /private/var).
      const opened = document.querySelector('.location .path')?.textContent;
      if (fallback !== album && fallback !== opened) throw new Error(`fallback=${fallback} album=${album} opened=${opened}`);
      return `start folder ${request.defaultPath.slice(album.length) || '.'}, then ${fallback === album || fallback === opened ? 'working folder' : fallback}`;
    });

    await step('Manage covers → Adjust… resizes and converts the selected cover, written on OK', async () => {
      await clickRow('sine-a.mp3');
      key('µ', { code: 'KeyM', ctrlKey: true, altKey: true }, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].manage-covers-dialog'), 5000, 'manage dialog');
      await waitFor(() => dlg.querySelectorAll('li[data-id]').length === 1, 3000, 'one cover');
      const listbox = dlg.querySelector('.manage-covers-list'); key('a', mod, listbox); await sleep(100);
      const adjustButton = dlg.querySelector('[data-manage="adjust"]'); if (adjustButton.disabled) throw new Error('Adjust… disabled');
      adjustButton.click();
      const adjust = await topWith('[data-key="format"]', 'Adjust cover dialog');
      const typeLocked = adjust.querySelector('#full-action-type').disabled;
      set(adjust.querySelector('[data-key="format"]'), 'jpeg'); set(adjust.querySelector('[data-key="resize_enabled"]'), true);
      set(adjust.querySelector('[data-key="pixels"]'), 100); set(adjust.querySelector('[data-key="size_mode"]'), 'maximum'); set(adjust.querySelector('[data-key="scale_mode"]'), 'proportional');
      adjust.querySelector('[data-keep]').click();
      await waitFor(() => top() === dlg && /100×50/.test(dlg.querySelector('li[data-id] small')?.textContent || ''), 8000, 'adjusted preview (' + (dlg.querySelector('.dialog-error')?.textContent || '') + ')');
      const label = dlg.querySelector('li[data-id] strong').textContent;
      dlg.querySelector('#manage-covers-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].manage-covers-dialog'), 10000, 'manage closes (' + (dlg.querySelector('.dialog-error')?.textContent || '') + ')');
      await waitFor(() => /Updated covers/.test(status()), 8000, 'updated covers'); await idle();
      const cover = (await read('sine-a.mp3')).covers[0];
      if (!typeLocked || cover.width !== 100 || cover.height !== 50 || cover.mime !== 'image/jpeg' || cover.picture_type !== 'CoverFront') throw new Error(`locked=${typeLocked} cover=${JSON.stringify({ w: cover.width, h: cover.height, mime: cover.mime, type: cover.picture_type })}`);
      return `${label}; written ${cover.width}×${cover.height} ${cover.mime}`;
    });

    await step('Tag → Filename renames the other files when one destination clashes', async () => {
      await selectAll(); op('tags-filename').click();
      const dlg = await topWith('#template', 'Tag → Filename');
      set(dlg.querySelector('#template'), 'same');
      dlg.querySelector('#preview-button').click();
      await waitFor(() => !dlg.querySelector('#apply-converter').disabled, 8000, 'rename preview');
      dlg.querySelector('#apply-converter').click(); await closed('Tag → Filename'); await idle();
      await waitFor(() => /Renamed: 1 file, 1 failed/.test(status()), 8000, 'partial rename status (' + status() + ')');
      const names = (await inv('scan_files', { paths: [album], recursive: false })).entries.map(e => e.path.slice(album.length + 1)).sort().join('|');
      if (names !== 'same.mp3|sine-b.mp3' || !/Destination exists/.test(errorsText())) throw new Error(`files=${names} errors=${errorsText()}`);
      return `${status()}; ${errorsText()}`;
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
