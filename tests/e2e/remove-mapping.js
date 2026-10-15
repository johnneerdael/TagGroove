// Native e2e for File → Remove tag (Ctrl+R, following Options → Tags → Mpeg →
// Remove, journaled for undo) and Options → Tags → Mapping (a Vorbis Comments
// DATE → YEAR rule edited in the dialog, read and saved through the app).
// Fixtures: remove-mapping-fixtures.sh (a FLAC with DATE). Runs in CI
// (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  // Returns false when a handler called preventDefault (the reload is suppressed).
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, code: /^[a-z]$/i.test(k) ? 'Key' + k.toUpperCase() : k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  const pathOf = name => row(name).dataset.path;
  const clickRow = async (name, o = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...o })); await sleep(250); };
  const scan = async name => (await inv('scan_files', { paths: [pathOf(name)], recursive: false })).entries[0];
  const field = (entry, name) => Object.entries(entry.fields).find(([k]) => k.toUpperCase() === name)?.[1];
  const tagKinds = async name => ((await scan(name)).technical._TAG || [''])[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 20000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const ready = name => waitFor(() => op(name) && !op(name).disabled && op(name), 8000, name + ' enabled');
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  // Shortcuts use ⌘ on macOS and Ctrl elsewhere.
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const openOptions = async () => {
    for (let attempt = 0; attempt < 3; attempt++) {
      document.body.focus(); key('o', mod, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null);
      if (dlg) return dlg;
      await sleep(300);
    }
    throw new Error('options dialog did not open');
  };
  const updateSettings = async change => { const settings = (await inv('load_settings')).settings; change(settings); await inv('save_settings', { settings }); };
  const saveTitle = async (name, title) => {
    await clickRow(name);
    const f = document.querySelector('.fields [data-field="TITLE"]'); f.focus(); input(f, title); f.blur(); document.body.focus();
    key('s', mod, document.body); await waitFor(() => /Saved tags/.test(status()), 8000, 'saved'); await idle();
  };

  try {
    await waitFor(() => rows().length >= 3, 15000, 'fixture rows');
    // No native confirmation dialog during the run. Messages are read from the
    // frontend's copy of the options, so this goes through the dialog.
    await step('Options → Messages turns off the Remove tag confirmation', async () => {
      const dlg = await openOptions();
      dlg.querySelector('[data-page="messages"]').click(); await sleep(150);
      const box = dlg.querySelector('[data-option="pages.messages.confirm_remove_tags"]');
      box.checked = false; box.dispatchEvent(new Event('change', { bubbles: true })); await sleep(100);
      dlg.querySelector('#options-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed'); await idle();
      const saved = (await inv('load_settings')).settings.pages.messages?.confirm_remove_tags;
      if (saved !== false) throw new Error('saved=' + saved);
      return 'confirm_remove_tags=false';
    });

    await step('File menu lists Remove tag with ⌘/Ctrl R', async () => {
      const file = [...document.querySelectorAll('.menubar > .menu')].find(m => m.querySelector('summary').textContent === 'File');
      const item = file?.querySelector('button[data-op="remove-tags"]');
      if (!item || !/Remove tag/.test(item.textContent) || !/R/.test(item.querySelector('small')?.textContent || '')) throw new Error('item ' + item?.textContent);
      return item.textContent.trim();
    });

    await step('Ctrl+R removes only the kinds checked under Options → Tags → Mpeg → Remove', async () => {
      await saveTitle('sine-a.mp3', 'Sine A Saved');
      const before = await tagKinds('sine-a.mp3');
      if (!/ID3v1/.test(before) || !/ID3v2/.test(before)) throw new Error('fixture tags ' + before);
      await updateSettings(s => { s.tags.mpeg.remove_id3v1 = true; s.tags.mpeg.remove_id3v2 = false; s.tags.mpeg.remove_ape = false; });
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      const notPrevented = key('r', mod);
      if (notPrevented) throw new Error('Ctrl+R was not consumed, the WebView could reload');
      await waitFor(() => /Removed tags/.test(status()), 8000, 'removed status'); await idle();
      const after = await tagKinds('sine-a.mp3');
      if (/ID3v1/.test(after) || !/ID3v2/.test(after)) throw new Error('after=' + after);
      if ((await scan('sine-a.mp3')).fields.TITLE?.[0] !== 'Sine A Saved') throw new Error('ID3v2 title lost');
      (await ready('undo')).click(); await waitFor(() => /Undid Remove tags/.test(status()), 8000, 'undo'); await idle();
      const undone = await tagKinds('sine-a.mp3');
      if (undone !== before) throw new Error('undo=' + undone);
      return `${before} → ${after} → undo ${undone}`;
    });

    await step('with the default Remove choices Ctrl+R strips every MPEG tag kind', async () => {
      const defaults = (await inv('load_settings')).defaults.tags.mpeg;
      await updateSettings(s => { s.tags.mpeg = { ...s.tags.mpeg, remove_id3v1: defaults.remove_id3v1, remove_id3v2: defaults.remove_id3v2, remove_ape: defaults.remove_ape }; });
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus(); key('r', mod);
      await waitFor(() => /Removed tags/.test(status()), 8000, 'removed status'); await idle();
      const entry = await scan('sine-a.mp3');
      if (entry.technical._TAG || entry.fields.TITLE) throw new Error('left ' + JSON.stringify(entry.technical._TAG));
      return 'no tags left';
    });

    await step('Ctrl+R while typing is consumed without removing tags', async () => {
      const before = await tagKinds('sine-b.mp3');
      await clickRow('sine-b.mp3');
      const filter = document.querySelector('#filter'); filter.focus();
      const notPrevented = key('r', mod, filter);
      await sleep(500);
      if (notPrevented) throw new Error('not consumed');
      const after = await tagKinds('sine-b.mp3');
      if (after !== before || !after) throw new Error(`${before} → ${after}`);
      filter.blur();
      return after;
    });

    await step('Options → Tags → Mapping adds a rule, rejects a conflicting one and reloads files', async () => {
      const raw = await scan('song.flac');
      if (field(raw, 'DATE')?.[0] !== '2024') throw new Error('fixture fields ' + JSON.stringify(raw.fields));
      const dlg = await openOptions();
      const item = dlg.querySelector('[data-page="tags-mapping"]');
      if (!item || item.getAttribute('aria-level') !== '2') throw new Error('Mapping page not under Tags');
      item.click(); await sleep(150);
      dlg.querySelector('#mapping-new').click(); await sleep(100);
      const type = dlg.querySelector('#mapping-tag-type'); type.value = 'VorbisComment'; type.dispatchEvent(new Event('change', { bubbles: true }));
      input(dlg.querySelector('#mapping-source'), 'DATE'); input(dlg.querySelector('#mapping-target'), 'YEAR');
      if (!/DATE → YEAR/.test(dlg.querySelector('#mapping-list option[value="0"]').textContent)) throw new Error('list not updated');
      dlg.querySelector('#mapping-new').click(); await sleep(100);
      input(dlg.querySelector('#mapping-source'), 'date'); input(dlg.querySelector('#mapping-target'), 'OTHER');
      dlg.querySelector('#options-ok').click(); await sleep(300);
      const error = dlg.querySelector('.dialog-error')?.textContent || '';
      if (!dlg.open || !/several targets/.test(error)) throw new Error('validation: ' + error);
      dlg.querySelector('#mapping-remove').click(); await sleep(100);
      dlg.querySelector('#options-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed'); await idle();
      const saved = (await inv('load_settings')).settings.tags.mappings;
      if (saved.length !== 1 || saved[0].tag_type !== 'VorbisComment' || saved[0].source !== 'DATE' || saved[0].target !== 'YEAR') throw new Error(JSON.stringify(saved));
      const mapped = await scan('song.flac');
      if (field(mapped, 'YEAR')?.[0] !== '2024' || field(mapped, 'DATE')) throw new Error(JSON.stringify(mapped.fields));
      await clickRow('song.flac');
      const year = document.querySelector('.fields [data-field="YEAR"]');
      if (year?.value !== '2024') throw new Error('tag panel YEAR=' + year?.value);
      return `${error}; YEAR=${year.value}`;
    });
    // A failed step must not leave the dialog open: it blocks every shortcut.
    document.querySelector('dialog[open].options-dialog #options-cancel')?.click(); await sleep(200);

    await step('saving the mapped field writes the source field', async () => {
      await clickRow('song.flac');
      const year = document.querySelector('.fields [data-field="YEAR"]'); year.focus(); input(year, '2030'); year.blur(); document.body.focus();
      key('s', mod, document.body); await waitFor(() => /Saved tags/.test(status()), 8000, 'saved'); await idle();
      const mapped = await scan('song.flac');
      await updateSettings(s => { s.tags.mappings = []; });
      const raw = await scan('song.flac');
      if (field(mapped, 'YEAR')?.[0] !== '2030') throw new Error('mapped ' + JSON.stringify(mapped.fields));
      if (field(raw, 'DATE')?.[0] !== '2030' || field(raw, 'YEAR')) throw new Error('raw ' + JSON.stringify(raw.fields));
      return 'DATE=' + field(raw, 'DATE')[0];
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await finish();
})();
