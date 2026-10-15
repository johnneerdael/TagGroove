// Native e2e for keyboard routing and the Extended Tags dialog: macOS Option
// keys type characters in text fields instead of triggering Alt shortcuts,
// AltGr chords stay text on Windows/Linux, Alt+T, << / >> navigation with
// pending edits, field-name list, value auto-completion, the dialog's cover
// window, Ctrl+Tab, Ctrl+Space, Ctrl+Shift+K, ⌘/Ctrl+T, column auto-size and
// ⌫ on Mac keyboards. The macOS rules are exercised by overriding
// navigator.platform, which the app reads on every key press.
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const errorsText = () => [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' | ');
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const pathOf = name => row(name).dataset.path;
  // A second click on the only selected row starts the slow-click rename (the cell editor opens after 550 ms and takes focus), so an already selected row is only focused.
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); if (r.classList.contains('selected') && rows().filter(x => x.classList.contains('selected')).length === 1) { r.focus(); await sleep(50); return; } r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(250); };
  const scan = async name => (await inv('scan_files', { paths: [pathOf(name)], recursive: false })).entries[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const done = async (pattern, what) => { await waitFor(() => pattern.test(status()) && !document.querySelector('footer.status.busy'), 10000, what + ' (status "' + status() + '" errors "' + errorsText() + '")'); };
  // Returns the dispatched event so a step can check whether the app consumed it.
  const press = (target, k, o = {}) => { const event = new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }); target.dispatchEvent(event); return event; };
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const setPlatform = value => Object.defineProperty(navigator, 'platform', { value, configurable: true });
  const restorePlatform = () => { delete navigator.platform; };
  const openDialog = () => document.querySelector('dialog[open]');
  const extended = () => document.querySelector('dialog.extended-dialog[open]');
  const caption = () => extended()?.querySelector('header')?.textContent || '';
  const fieldRow = (dlg, name) => [...dlg.querySelectorAll('.extended-row')].find(r => r.querySelector('input').value === name);
  const panel = name => document.querySelector(`.fields [data-field="${name}"]`);
  const closeDialogs = async () => { for (let i = 0; i < 4 && openDialog(); i++) { openDialog().close(); await sleep(150); } };
  const png = (w, h, color) => new Promise(resolve => { const c = document.createElement('canvas'); c.width = w; c.height = h; const g = c.getContext('2d'); g.fillStyle = color; g.fillRect(0, 0, w, h); c.toBlob(b => resolve(new File([b], 'canvas.png', { type: 'image/png' })), 'image/png'); });
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } finally { restorePlatform(); await closeDialogs(); } };
  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');

    await step('macOS: Option+letter and Option+digit type in text fields instead of triggering shortcuts', async () => {
      setPlatform('MacIntel');
      await clickRow('sine-a.mp3');
      const title = await waitFor(() => panel('TITLE'), 3000, 'Title field'); title.focus();
      const typed = [['Dead', 'KeyI'], ['å', 'KeyA'], ['©', 'KeyG'], ['®', 'KeyR'], ['µ', 'KeyM'], ['§', 'Digit6'], ['¡', 'Digit1'], ['†', 'KeyT']];
      for (const [k, code] of typed) {
        const event = press(title, k, { code, altKey: true });
        await sleep(120);
        if (event.defaultPrevented) throw new Error(`Option+${code} (${k}) was swallowed`);
        if (document.activeElement !== title) throw new Error(`Option+${code} moved focus to ${document.activeElement?.dataset?.field || document.activeElement?.tagName}`);
        if (openDialog()) throw new Error(`Option+${code} opened ${openDialog().querySelector('header')?.textContent}`);
      }
      const shift6 = press(title, 'ﬂ', { code: 'Digit6', altKey: true, shiftKey: true }); await sleep(120);
      if (shift6.defaultPrevented || openDialog()) throw new Error('Option+Shift+6 opened Actions (Quick)');
      return `${typed.length + 1} Option chords left to the text field`;
    });

    await step('macOS: Control+Option triggers Alt shortcuts while typing; Option alone works outside text fields', async () => {
      setPlatform('MacIntel');
      await clickRow('sine-a.mp3');
      panel('TITLE').focus();
      const jump = press(panel('TITLE'), 'i', { code: 'KeyI', altKey: true, ctrlKey: true });
      await waitFor(() => document.activeElement === panel('ARTIST'), 2000, 'Control+Option+I focuses Artist');
      if (!jump.defaultPrevented) throw new Error('mnemonic not consumed');
      row('sine-a.mp3').focus();
      press(row('sine-a.mp3'), '¬', { code: 'KeyL', altKey: true });
      await waitFor(() => document.activeElement === panel('TITLE'), 2000, 'Option+L outside text fields focuses Title');
      panel('TITLE').focus();
      press(panel('TITLE'), 't', { code: 'KeyT', altKey: true, ctrlKey: true });
      await waitFor(() => extended(), 3000, 'Control+Option+T opens Extended tags');
      return caption();
    });

    await step('Windows/Linux: AltGr chords stay text; Alt mnemonics work while typing', async () => {
      setPlatform('Win32');
      await clickRow('sine-a.mp3');
      const title = panel('TITLE'); title.focus();
      const altGr = press(title, 'µ', { code: 'KeyM', altKey: true, ctrlKey: true }); await sleep(200);
      if (altGr.defaultPrevented || openDialog() || document.activeElement !== title) throw new Error('AltGr+M was taken as Manage covers or a mnemonic');
      const polish = press(title, 'ą', { code: 'KeyA', altKey: true, ctrlKey: true }); await sleep(150);
      if (polish.defaultPrevented || document.activeElement !== title) throw new Error('AltGr+A was swallowed');
      press(title, 'i', { code: 'KeyI', altKey: true });
      await waitFor(() => document.activeElement === panel('ARTIST'), 2000, 'Alt+I focuses Artist');
      return 'AltGr+M, AltGr+A typed; Alt+I jumped';
    });

    await step('Alt+T opens Extended tags with caption, navigation and field-name list', async () => {
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      press(row('sine-a.mp3'), 't', { code: 'KeyT', altKey: true });
      const dlg = await waitFor(() => extended(), 3000, 'Extended tags');
      if (!/sine-a\.mp3/.test(caption())) throw new Error('caption ' + caption());
      const prev = dlg.querySelector('[data-nav="-1"]'), next = dlg.querySelector('[data-nav="1"]');
      if (!prev.disabled || next.disabled || prev.hidden) throw new Error(`<< disabled=${prev.disabled} >> disabled=${next.disabled}`);
      const name = fieldRow(dlg, 'TITLE').querySelector('input');
      const list = await waitFor(() => name.list, 2000, 'field-name drop-down');
      const options = [...list.options].map(o => o.value);
      if (!options.includes('ALBUMARTIST') || !options.includes('TITLE')) throw new Error('field list ' + options.slice(0, 8).join(','));
      if (!name.parentElement.querySelector('.helper-button')) throw new Error('no Fields helper button');
      if (!dlg.querySelector('.extended-cover .cover-art')) throw new Error('no cover window');
      return `${caption()} · ${options.length} known fields`;
    });

    await step('<< and >> move through the File List and keep edits pending; Save writes them', async () => {
      await clickRow('sine-a.mp3'); press(row('sine-a.mp3'), 't', { code: 'KeyT', altKey: true });
      const dlg = await waitFor(() => extended(), 3000, 'Extended tags');
      const value = fieldRow(dlg, 'TITLE').querySelector('textarea');
      value.value = 'Sine A edited'; value.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertFromPaste' }));
      dlg.querySelector('[data-nav="1"]').click();
      await waitFor(() => /sine-b\.mp3/.test(caption()), 3000, 'caption of the next file');
      if (!row('sine-b.mp3').classList.contains('selected') || row('sine-a.mp3').classList.contains('selected')) throw new Error('File List selection did not follow');
      if (fieldRow(dlg, 'TITLE').querySelector('textarea').value !== 'Sine B') throw new Error('next file shows ' + fieldRow(dlg, 'TITLE').querySelector('textarea').value);
      if (!dlg.querySelector('[data-nav="1"]').disabled && rows().length === 2) throw new Error('>> enabled on the last file');
      dlg.querySelector('[data-nav="-1"]').click();
      await waitFor(() => /sine-a\.mp3/.test(caption()), 3000, 'caption of the previous file');
      const back = fieldRow(dlg, 'TITLE').querySelector('textarea').value;
      if (back !== 'Sine A edited') throw new Error('returned to ' + back);
      dlg.querySelector('[data-close]').click();
      await waitFor(() => /Extended tag changes for 1 file are pending/.test(status()), 3000, 'pending status');
      document.body.focus(); press(document.body, 's', mod);
      await done(/Saved tags/, 'saved'); await idle();
      const entry = await scan('sine-a.mp3');
      if (entry.fields.TITLE?.[0] !== 'Sine A edited') throw new Error(JSON.stringify(entry.fields.TITLE));
      return 'TITLE on disk: ' + entry.fields.TITLE[0];
    });

    await step('field values auto-complete from loaded values', async () => {
      await clickRow('sine-b.mp3'); press(row('sine-b.mp3'), 't', { code: 'KeyT', altKey: true });
      const dlg = await waitFor(() => extended(), 3000, 'Extended tags');
      const value = fieldRow(dlg, 'ARTIST').querySelector('textarea');
      value.focus(); value.value = 'Test'; value.setSelectionRange(4, 4);
      value.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: 't' }));
      if (value.value !== 'Test Artist' || value.selectionStart !== 4 || value.selectionEnd !== 11) throw new Error(`"${value.value}" ${value.selectionStart}-${value.selectionEnd}`);
      return 'Test → Test Artist (completion selected)';
    });

    await step('cover window in Extended tags: paste embeds, Remove cover removes', async () => {
      await clickRow('sine-b.mp3'); press(row('sine-b.mp3'), 't', { code: 'KeyT', altKey: true });
      const dlg = await waitFor(() => extended(), 3000, 'Extended tags');
      const art = dlg.querySelector('.extended-cover .cover-art');
      const data = new DataTransfer(); data.items.add(await png(120, 90, '#36c'));
      art.focus(); art.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
      await done(/Added cover/, 'added cover');
      if ((await scan('sine-b.mp3')).covers.length !== 1) throw new Error('cover not embedded');
      await waitFor(() => extended()?.querySelector('.extended-cover img[data-cover-image]')?.src.startsWith('data:image/'), 5000, 'dialog shows the cover');
      const remove = await waitFor(() => { const b = extended().querySelector('.extended-cover [data-cover="remove"]'); return b && !b.disabled && b; }, 3000, 'Remove cover enabled');
      if (extended().querySelector('.extended-cover [data-cover="extract"]').disabled) throw new Error('Extract cover disabled with a cover shown');
      remove.click();
      await done(/Removed cover/, 'removed cover');
      if ((await scan('sine-b.mp3')).covers.length !== 0) throw new Error('cover not removed');
      if (!extended()) throw new Error('dialog closed by the cover command');
      return 'pasted 120×90 PNG, then removed';
    });

    await step('Ctrl+Tab cycles Tag Panel, Filter and File List', async () => {
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      const where = () => document.activeElement?.closest('.editor') ? 'panel' : document.activeElement?.id === 'filter' ? 'filter' : document.activeElement?.closest('.table-wrap') ? 'list' : document.activeElement?.tagName;
      const seen = [];
      for (let i = 0; i < 3; i++) { press(document.activeElement, 'Tab', { code: 'Tab', ctrlKey: true }); await sleep(120); seen.push(where()); }
      press(document.activeElement, 'Tab', { code: 'Tab', ctrlKey: true, shiftKey: true }); await sleep(120); seen.push(where());
      if (seen.join(',') !== 'panel,filter,list,filter') throw new Error(seen.join(','));
      return seen.join(' → ');
    });

    await step('Ctrl+Space opens the format string helper in a converter', async () => {
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      press(row('sine-a.mp3'), '1', { code: 'Digit1', altKey: true });
      const input = await waitFor(() => openDialog()?.querySelector('#template[data-helper-bound]'), 3000, 'Tag → Filename with its helper');
      input.focus(); press(input, ' ', { code: 'Space', ctrlKey: true });
      const menu = await waitFor(() => openDialog().querySelector('.helper-menu'), 2000, 'helper menu');
      press(menu, 'Escape');
      await waitFor(() => !openDialog().querySelector('.helper-menu'), 2000, 'menu closed');
      return [...menu.querySelectorAll(':scope > button, :scope > .helper-group > button')].slice(0, 4).map(b => b.textContent).join(', ');
    });

    await step('Ctrl+Shift+K opens Auto-numbering with the counter restarting per folder', async () => {
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      press(row('sine-a.mp3'), 'K', { code: 'KeyK', shiftKey: true, ...mod });
      const dlg = await waitFor(() => openDialog()?.querySelector('#number-directory') && openDialog(), 3000, 'numbering');
      if (!dlg.querySelector('#number-directory').checked || !dlg.querySelector('#number-reset-track').checked) throw new Error('per-folder options not set');
      dlg.close(); await sleep(150);
      row('sine-a.mp3').focus(); press(row('sine-a.mp3'), 'k', { code: 'KeyK', ...mod });
      const plain = await waitFor(() => openDialog()?.querySelector('#number-directory') && openDialog(), 3000, 'numbering');
      if (plain.querySelector('#number-directory').checked) throw new Error('Ctrl+K preset the folder option');
      return 'Ctrl+Shift+K: next disc on folder change + reset track; Ctrl+K: plain';
    });

    await step('⌘/Ctrl+T reads the tags again', async () => {
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      press(row('sine-a.mp3'), 't', { code: 'KeyT', ...mod });
      await done(/Reloaded/, 'reloaded');
      return status();
    });

    await step('menus and Help → Keyboard shortcuts show the bindings', async () => {
      const ext = document.querySelector('.menubar button[data-op="extended"] small')?.textContent;
      const reload = document.querySelector('.menubar button[data-op="reload"] small')?.textContent;
      const discard = document.querySelector('.menubar button[data-op="discard"] small')?.textContent;
      if (ext !== 'Alt T' || !/T$/.test(reload || '') || !/B$/.test(discard || '')) throw new Error(`${ext} / ${reload} / ${discard}`);
      document.querySelector('.menubar button[data-op="help-shortcuts"]').click();
      const dlg = await waitFor(() => document.querySelector('dialog.shortcuts-dialog[open]'), 3000, 'shortcuts dialog');
      const text = dlg.textContent;
      for (const expected of ['Extended tags', 'Ctrl Tab', 'Ctrl Space', 'Auto-size columns', '⌘/Ctrl ⇧ K', 'Control+Option']) if (!text.includes(expected)) throw new Error('missing ' + expected);
      return `${dlg.querySelectorAll('tr').length} rows`;
    });

    await step('View → Auto-size columns fits the File List columns', async () => {
      const th = () => document.querySelector('th[data-col="filename"]');
      const before = th().offsetWidth;
      const button = document.querySelector('.menubar button[data-op="autosize-columns"]');
      if (!button || button.disabled) throw new Error('menu item missing or disabled');
      button.click();
      await waitFor(() => th() && th().offsetWidth !== before, 3000, 'width change');
      const after = th().offsetWidth, text = row('sine-a.mp3').querySelector('td').scrollWidth <= row('sine-a.mp3').querySelector('td').clientWidth;
      if (!text) throw new Error('filename still truncated');
      return `Filename ${before}px → ${after}px`;
    });

    await step('Mac keyboards: ⌫ removes the file from the list (not the disk)', async () => {
      setPlatform('MacIntel');
      const count = rows().length;
      await clickRow('sine-b.mp3'); row('sine-b.mp3').focus();
      press(row('sine-b.mp3'), 'Backspace', { code: 'Backspace' });
      await waitFor(() => rows().length === count - 1 && !row('sine-b.mp3'), 3000, 'row removed');
      const path = rows()[0].dataset.path.replace(/sine-a\.mp3$/, 'sine-b.mp3');
      if (!(await inv('scan_files', { paths: [path], recursive: false })).entries.length) throw new Error('file deleted from disk');
      return `${count} → ${rows().length} rows; file still on disk`;
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
