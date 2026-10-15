// Native e2e for Options → General, Appearance, Networking, Updates and the
// remaining Messages, plus Utils → Remove unsupported fields and Optimize.
// Fixtures: general-options-fixtures.sh (FLAC, MP4, a cover image). Two page
// loads: the first stores startup options and sets the splitter; the second
// checks the startup behaviour and drives every option through the UI.
// Steps marked "live" need the network (GitHub, MusicBrainz, Cover Art Archive).
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const pathOf = name => row(name).dataset.path;
  const clickRow = async (name, o = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...o })); await sleep(250); };
  const scan = async path => (await inv('scan_files', { paths: [path], recursive: false })).entries[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 20000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const results = [];
  const step = async (name, fn) => { if (name.startsWith('live:') && window.OPENTAG_E2E_OFFLINE) { await report(`SKIP ${name} — offline run`); return; } try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const stage = sessionStorage.getItem('general-options-stage');
  // Shortcuts use ⌘ on macOS and Ctrl elsewhere.
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const windows = /^Win/.test(navigator.platform);
  const openOptions = async () => {
    for (let attempt = 0; attempt < 3; attempt++) {
      document.body.focus(); key('o', mod, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null);
      if (dlg) return dlg;
      await sleep(300);
    }
    throw new Error(`options dialog did not open; dialogs: ${[...document.querySelectorAll('dialog')].map(d => d.className + (d.open ? ' (open)' : '')).join(' | ')}; options button disabled=${op('options')?.disabled}; busy=${!!document.querySelector('footer.status.busy')}`);
  };
  // Change options through the dialog, so the frontend applies them as a user would.
  const setOptions = async (page, changes) => {
    const dlg = await openOptions();
    dlg.querySelector(`[data-page="${page}"]`).click(); await sleep(150);
    for (const [path, value] of changes) {
      const inputs = [...dlg.querySelectorAll(`[data-option="${path}"]`)];
      const el = inputs.find(e => e.type !== 'radio' || e.value === value);
      if (!el) throw new Error('no control for ' + path);
      if (el.type === 'checkbox') el.checked = value; else if (el.type === 'radio') el.checked = true; else el.value = String(value);
      el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); await sleep(60);
    }
    dlg.querySelector('#options-ok').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
    await idle();
  };

  try {
    if (!stage) {
      await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
      const album = pathOf('sine-a.mp3').replace(/[\\/][^\\/]+$/, '');
      await step('fixtures: FLAC and MP4 files are listed', async () => {
        await waitFor(async () => (await inv('scan_files', { paths: [album], recursive: false })).entries.length === 4, 15000, 'four audio files');
        return (await inv('scan_files', { paths: [album], recursive: false })).entries.map(e => `${e.filename} ${e.format}`).join(', ');
      });
      await step('startup options and an action group are stored for the second load', async () => {
        const settings = (await inv('load_settings')).settings;
        settings.pages = { ...settings.pages, general: { select_added: true }, 'directories-startup': { startup: 'previous' } };
        await inv('save_settings', { settings });
        await inv('remember_directories', { paths: [album] });
        await inv('save_action_library', { library: { groups: [{ name: 'Mark comment', enabled: true, actions: [{ type: 'format_value', field: 'COMMENT', template: 'marked' }] }], presets: [] } });
        return 'select added files, previous directory';
      });
      await step('the tag panel splitter resizes with the keyboard', async () => {
        const editor = document.querySelector('.editor'), splitter = document.querySelector('.panel-splitter'), before = editor.offsetWidth;
        splitter.focus(); key('ArrowRight', {}, splitter); key('ArrowRight', {}, splitter); await sleep(150);
        const after = editor.offsetWidth;
        if (after !== before + 32) throw new Error(`${before} → ${after}`);
        sessionStorage.setItem('general-options-width', String(after));
        return `${before}px → ${after}px, saved ${localStorage.getItem('opentag.panel-width')}`;
      });
      if (results.some(r => r[0] === 'FAIL')) return await finish();
      sessionStorage.setItem('general-options-stage', '2');
      sessionStorage.setItem('general-options-results', JSON.stringify(results));
      location.reload();
      return;
    }
    results.push(...JSON.parse(sessionStorage.getItem('general-options-results') || '[]'));
    const album = await waitFor(() => rows().length >= 4 && pathOf('sine-a.mp3').replace(/[\\/][^\\/]+$/, ''), 20000, 'four rows after reload');
    await idle();

    await step('Select files automatically selects the files loaded at startup', async () => {
      const selected = rows().filter(r => r.getAttribute('aria-selected') === 'true').length;
      if (selected !== 4) throw new Error(`${selected} of ${rows().length} selected`);
      return `${selected}/${rows().length} rows selected; status "${status()}"`;
    });
    await step('the splitter width is remembered across loads', async () => {
      const expected = Number(sessionStorage.getItem('general-options-width')), width = document.querySelector('.editor').offsetWidth;
      if (width !== expected) throw new Error(`${width} != ${expected}`);
      return `${width}px`;
    });
    await step('Options shows the General, Appearance, Messages, Networking and Updates controls', async () => {
      const dlg = await openOptions();
      const checked = path => dlg.querySelector(`[data-option="${path}"]`)?.checked;
      const seen = [];
      dlg.querySelector('[data-page="general"]').click(); await sleep(100);
      if (!checked('pages.general.autocomplete') || !checked('pages.general.keep_moved') || checked('pages.general.auto_select_next') || !checked('pages.general.select_added')) throw new Error('general defaults');
      seen.push('General');
      dlg.querySelector('[data-page="appearance"]').click(); await sleep(100);
      if (dlg.querySelector('[data-option="pages.appearance.ui_scale"]').value !== '100' || !dlg.querySelector('[data-option="pages.appearance.grid"][value="off"]').checked) throw new Error('appearance defaults');
      seen.push('Appearance');
      dlg.querySelector('[data-page="messages"]').click(); await sleep(100);
      if (!checked('pages.messages.action_warnings') || !checked('pages.messages.confirm_import_covers') || !dlg.querySelector('[data-option="pages.messages.import_covers_choice"]').matches(':disabled')) throw new Error('messages defaults');
      seen.push('Messages');
      dlg.querySelector('[data-page="networking"]').click(); await sleep(100);
      if (dlg.querySelector('[data-option="pages.networking.timeout"]').value !== '30' || !dlg.querySelector('[data-option="pages.networking.proxy"][value="system"]').checked || !dlg.querySelector('[data-option="pages.networking.proxy_url"]').matches(':disabled')) throw new Error('networking defaults');
      seen.push('Networking');
      dlg.querySelector('[data-page="updates"]').click(); await sleep(100);
      if (checked('pages.updates.check_on_startup') || checked('pages.updates.include_beta') || !checked('pages.updates.release_notes')) throw new Error('updates defaults');
      seen.push('Updates');
      const timeout = () => { dlg.querySelector('[data-page="networking"]').click(); };
      timeout(); await sleep(100);
      const field = dlg.querySelector('[data-option="pages.networking.timeout"]'); field.value = '2'; field.dispatchEvent(new Event('input', { bubbles: true }));
      dlg.querySelector('#options-ok').click(); await sleep(200);
      const problem = dlg.querySelector('.dialog-error')?.textContent || '';
      if (!/between 5 and 300/.test(problem)) throw new Error('timeout accepted: ' + problem);
      dlg.querySelector('#options-cancel').click(); await waitFor(() => !dlg.open, 3000, 'closed');
      return seen.join('|') + `; invalid timeout: "${problem}"`;
    });
    await step('Appearance: alternate rows, grid lines, toolbar and interface scaling', async () => {
      const width = window.innerWidth;
      await setOptions('appearance', [['pages.appearance.grid', 'alternate'], ['pages.appearance.toolbar_scale', '150'], ['pages.appearance.ui_scale', '125']]);
      await waitFor(() => Math.abs(window.innerWidth - width / 1.25) < 3, 4000, `zoom (innerWidth ${window.innerWidth} from ${width})`);
      await clickRow('sine-a.mp3');
      const odd = rows().find(r => r.classList.contains('odd') && !r.classList.contains('selected')), even = rows().find(r => !r.classList.contains('odd') && !r.classList.contains('selected'));
      const oddColor = getComputedStyle(odd).backgroundColor, evenColor = getComputedStyle(even).backgroundColor;
      if (document.documentElement.dataset.grid !== 'alternate' || oddColor === evenColor) throw new Error(`grid ${document.documentElement.dataset.grid} ${oddColor} ${evenColor}`);
      const zoom = getComputedStyle(document.querySelector('.toolbar')).zoom;
      if (zoom !== '1.5') throw new Error('toolbar zoom ' + zoom);
      const zoomed = window.innerWidth;
      await setOptions('appearance', [['pages.appearance.grid', 'lines'], ['pages.appearance.toolbar_scale', '100'], ['pages.appearance.ui_scale', '100']]);
      await waitFor(() => Math.abs(window.innerWidth - width) < 3, 4000, 'zoom reset');
      const border = getComputedStyle(rows()[0].querySelector('td')).borderRightWidth;
      if (border !== '1px') throw new Error('grid lines border ' + border);
      await setOptions('appearance', [['pages.appearance.grid', 'off']]);
      return `innerWidth ${width} → ${zoomed} at 125% → ${window.innerWidth}; odd rows ${oddColor} vs ${evenColor}; toolbar zoom ${zoom}; grid lines ${border}`;
    });
    await step('the tag panel completes typed values from its drop-down list', async () => {
      await clickRow('sine-a.mp3');
      const artist = document.querySelector('.fields input[data-field="ARTIST"]');
      artist.focus(); artist.value = 'Te'; artist.setSelectionRange(2, 2);
      artist.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: 'e' }));
      const completed = [artist.value, artist.selectionStart, artist.selectionEnd];
      if (completed.join() !== 'Test Artist,2,11') throw new Error('completed ' + completed.join());
      if (artist.getAttribute('aria-autocomplete') !== 'inline') throw new Error('aria-autocomplete');
      if (document.querySelector('.dirty-count')) throw new Error('the completed value equals the saved one; nothing should be pending');
      await setOptions('general', [['pages.general.autocomplete', false]]);
      const plain = document.querySelector('.fields input[data-field="ARTIST"]');
      plain.focus(); plain.value = 'Te'; plain.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: 'e' }));
      const off = plain.value;
      plain.value = 'Test Artist'; plain.dispatchEvent(new Event('input', { bubbles: true }));
      if (off !== 'Te') throw new Error('completed while off: ' + off);
      await setOptions('general', [['pages.general.autocomplete', true]]);
      return `"Te" → "${completed[0]}" (selection ${completed[1]}–${completed[2]}); off: "${off}"`;
    });
    await step('File List editing completes values and Enter moves to the next file', async () => {
      await setOptions('general', [['pages.general.auto_select_next', true]]);
      await clickRow('sine-a.mp3');
      const cell = () => row('sine-a.mp3').querySelector('td[data-col="title"]');
      cell().dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      const editor = await waitFor(() => document.querySelector('.cell-editor'), 3000, 'cell editor');
      editor.value = 'Sine'; editor.setSelectionRange(4, 4);
      editor.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: 'e' }));
      const completed = editor.value;
      if (!/^Sine [AB]$/.test(completed)) throw new Error('completion ' + completed);
      editor.value = 'Sine A'; key('Enter', {}, editor);
      const next = await waitFor(() => { const e = document.querySelector('.cell-editor'); return e && e !== editor && e; }, 3000, 'editor on the next file');
      const selectedRow = rows().find(r => r.getAttribute('aria-selected') === 'true')?.dataset.path.split(/[\\/]/).pop();
      const value = next.value;
      key('Escape', {}, next); await sleep(150);
      await setOptions('general', [['pages.general.auto_select_next', false]]);
      if (selectedRow !== 'sine-b.mp3' || value !== 'Sine B') throw new Error(`next row ${selectedRow} value ${value}`);
      return `"Sine" → "${completed}"; Enter opened ${selectedRow} with "${value}"`;
    });
    const utilRun = async (name, menuOp) => {
      await clickRow(name); const before = await scan(pathOf(name));
      op(menuOp).click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].util-dialog'), 3000, 'util dialog');
      const text = dlg.querySelector('.dialog-content p').textContent;
      dlg.querySelector('#run-util').click();
      await waitFor(() => !dlg.open, 20000, 'util finished'); await idle();
      return { before, after: await scan(pathOf(name)), text, status: status() };
    };
    await step('Optimize FLAC leaves 4 KiB of padding and Undo restores the file', async () => {
      const { before, after, text, status: s } = await utilRun('song.flac', 'util-optimize');
      if (before.size - after.size !== 4096) throw new Error(`size ${before.size} → ${after.size}`);
      key('z', mod, document.body); await idle();
      const undone = await scan(pathOf('song.flac'));
      if (undone.size !== before.size) throw new Error('undo size ' + undone.size);
      return `${before.size} → ${after.size} bytes ("${s}"); undo → ${undone.size}; prompt: ${text.slice(0, 60)}…`;
    });
    await step('Optimize MP4 drops free space and moves moov before mdat', async () => {
      const { before, after, status: s } = await utilRun('song.m4a', 'util-optimize');
      if (!(after.size < before.size) || after.duration !== before.duration) throw new Error(`size ${before.size} → ${after.size}, duration ${before.duration} → ${after.duration}`);
      const again = await utilRun('song.m4a', 'util-optimize');
      if (again.after.size !== after.size) throw new Error('second run changed the file');
      return `${before.size} → ${after.size} bytes ("${s}"), second run unchanged`;
    });
    await step('Utils skip formats they do not apply to', async () => {
      await clickRow('sine-a.mp3');
      op('util-optimize').click(); await sleep(200);
      const error = document.querySelector('.errors')?.textContent || '';
      if (document.querySelector('dialog[open].util-dialog') || !/applies to FLAC and MP4 files/.test(error)) throw new Error('error ' + error);
      document.querySelector('#dismiss-errors')?.click();
      const { before, after, status: s } = await utilRun('sine-a.mp3', 'util-unsupported');
      if (after.size !== before.size) throw new Error('an MP3 without unsupported frames was rewritten');
      return `Optimize on MP3: "${error.replace(/\s+/g, ' ').slice(0, 80)}"; Remove unsupported fields: "${s}", size unchanged`;
    });
    await step('Messages → at warnings from Actions shows or suppresses action errors', async () => {
      await inv('run_tool', { tool: windows ? { name: 'Lock', path: 'C:\\Windows\\System32\\attrib.exe', parameters: '+R "%_path%"', for_each_file: true } : { name: 'Lock', path: '/bin/chmod', parameters: '444 "%_path%"', for_each_file: true }, paths: [pathOf('sine-b.mp3')], root: null });
      await sleep(400);
      await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', mod);
      const runGroup = async () => { const group = await waitFor(() => [...document.querySelectorAll('[data-action-group]')].find(b => /Mark comment/.test(b.textContent) && !b.disabled), 5000, 'action group'); group.click(); await sleep(300); await idle(); await sleep(300); };
      await runGroup();
      const shown = [...document.querySelectorAll('dialog[open]')].find(d => /read-only/i.test(d.textContent));
      if (!shown) throw new Error('no action warning dialog');
      shown.querySelector('[data-close]')?.click(); shown.open && shown.close(); await sleep(150);
      await setOptions('messages', [['pages.messages.action_warnings', false]]);
      await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', mod);
      await runGroup();
      const dialogs = [...document.querySelectorAll('dialog[open]')].length, listed = document.querySelector('.errors')?.textContent || '';
      await inv('run_tool', { tool: windows ? { name: 'Unlock', path: 'C:\\Windows\\System32\\attrib.exe', parameters: '-R "%_path%"', for_each_file: true } : { name: 'Unlock', path: '/bin/chmod', parameters: '644 "%_path%"', for_each_file: true }, paths: [pathOf('sine-b.mp3')], root: null });
      await sleep(400);
      await setOptions('messages', [['pages.messages.action_warnings', true]]);
      if (dialogs || !/read-only/i.test(listed)) throw new Error(`dialogs=${dialogs} errors=${listed}`);
      return `on: "${shown.textContent.replace(/\s+/g, ' ').slice(0, 70)}"; off: listed only (${listed.replace(/\s+/g, ' ').slice(0, 60)})`;
    });
    await step('Keep moved files: on keeps a file moved out of the folder, off removes it', async () => {
      const sep = album.includes('\\') ? '\\' : '/', outside = album.replace(/[\\/][^\\/]+$/, '') + sep + 'moved-out';
      const moveOut = async () => {
        await clickRow('sine-a.mp3'); op('tags-filename').click();
        const dlg = await waitFor(() => document.querySelector('dialog[open] #template')?.closest('dialog'), 3000, 'Tag → Filename');
        const template = dlg.querySelector('#template'); template.value = `${outside}${sep}%title%`; template.dispatchEvent(new Event('input', { bubbles: true }));
        dlg.querySelector('#preview-button').click();
        const apply = await waitFor(() => !dlg.querySelector('#apply-converter').disabled && dlg.querySelector('#apply-converter'), 5000, 'preview');
        apply.click(); await waitFor(() => !dlg.open, 5000, 'rename'); await idle();
      };
      const undo = async () => { key('z', mod, document.body); await idle(); };
      await moveOut();
      const kept = rows().find(r => /^[\\/]/.test(r.dataset.path.slice(outside.length)) && r.dataset.path.startsWith(outside));
      if (!kept) throw new Error('moved file left the list while keep is on');
      await undo();
      await setOptions('general', [['pages.general.keep_moved', false]]);
      if (!row('sine-a.mp3')) { op('refresh').click(); await idle(); }
      await moveOut();
      const listed = rows().some(r => /^[\\/]/.test(r.dataset.path.slice(outside.length)) && r.dataset.path.startsWith(outside)), count = rows().length;
      const seen = `rows ${rows().map(r => r.dataset.path).join(' | ')}; location ${document.querySelector('.location .path')?.textContent}; status ${status()}`;
      await undo();
      await setOptions('general', [['pages.general.keep_moved', true]]);
      op('refresh').click(); await waitFor(() => row('sine-a.mp3'), 8000, 'sine-a back after refresh');
      if (listed || count !== 3) throw new Error(`listed=${listed} rows=${count}; ${seen}`);
      return `on: kept ${kept.dataset.path.split(/[\\/]/).slice(-2).join('/')}; off: removed (rows ${count}); undo and refresh restore sine-a.mp3`;
    });
    await step('Networking: a manual proxy carries tag source and update requests', async () => {
      await setOptions('networking', [['pages.networking.proxy', 'manual'], ['pages.networking.proxy_url', '127.0.0.1:9'], ['pages.networking.timeout', '10']]);
      let source = '', update = '';
      try { await inv('tag_source_search', { provider: 'musicbrainz', query: { mode: 'artist_album', artist: 'Daft Punk', album: 'Discovery', text: '' }, ticket: Date.now() }); } catch (e) { source = String(e); }
      try { await inv('check_for_updates', { includePrereleases: false }); } catch (e) { update = String(e); }
      await setOptions('networking', [['pages.networking.proxy', 'system'], ['pages.networking.timeout', '30']]);
      if (!/Could not connect to MusicBrainz/.test(source) || !/Could not connect to GitHub/.test(update)) throw new Error(`source="${source}" update="${update}"`);
      return `through 127.0.0.1:9: "${source}" / "${update}"`;
    });
    await step('live: Help → Check for updates reads the GitHub releases', async () => {
      op('help-updates').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].updates-dialog'), 30000, 'update dialog');
      const text = dlg.textContent.replace(/\s+/g, ' ');
      const failed = dlg.querySelector('.dialog-error')?.textContent;
      dlg.querySelector('[data-close]').click(); await sleep(150);
      if (failed) throw new Error(failed);
      // With development builds included, Options → Updates → Check now compares against betas too.
      await setOptions('updates', [['pages.updates.include_beta', true]]);
      await sleep(300);
      const options = await openOptions();
      options.querySelector('[data-page="updates"]').click(); await sleep(100);
      options.querySelector('#option-button-check-updates').click();
      const again = await waitFor(() => document.querySelector('dialog[open].updates-dialog'), 30000, 'update dialog over Options');
      const beta = again.textContent.replace(/\s+/g, ' ');
      again.querySelector('[data-close]').click(); await sleep(150); options.querySelector('#options-cancel').click();
      await setOptions('updates', [['pages.updates.include_beta', false]]);
      return `stable: ${text.slice(0, 150)} | with betas: ${beta.slice(0, 110)}`;
    });
    await step('live: at importing covers asks, remembers the choice and keeps existing artwork', async () => {
      await clickRow('sine-a.mp3');
      await inv('add_cover', { paths: [pathOf('sine-a.mp3')], imagePath: album.replace(/[\\/][^\\/]+$/, '') + '/cover.png' });
      op('refresh').click(); await idle();
      const coverBefore = (await scan(pathOf('sine-a.mp3'))).covers;
      if (coverBefore.length !== 1) throw new Error('fixture cover missing');
      await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', mod);
      op('source-musicbrainz').click();
      const d = await waitFor(() => document.querySelector('dialog[open] #ts-artist')?.closest('dialog'), 5000, 'MusicBrainz dialog');
      d.querySelector('#ts-artist').value = 'Daft Punk'; d.querySelector('#ts-album').value = 'Discovery';
      d.querySelector('#ts-artist').dispatchEvent(new Event('input', { bubbles: true })); d.querySelector('#ts-album').dispatchEvent(new Event('input', { bubbles: true }));
      d.querySelector('#ts-search').click();
      await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 40000, 'results');
      await waitFor(() => !d.querySelector('#ts-next').disabled, 5000, 'next'); d.querySelector('#ts-next').click();
      const apply = await waitFor(() => d.querySelector('#ts-apply'), 40000, 'review'); await waitFor(() => !apply.disabled, 40000, 'apply enabled');
      const write = await waitFor(() => d.querySelector('#ts-cover-write'), 40000, 'artwork available');
      if (!write.checked) { write.checked = true; write.dispatchEvent(new Event('change', { bubbles: true })); }
      apply.click();
      const ask = await waitFor(() => document.querySelector('dialog[open].import-covers-dialog'), 10000, 'import covers question');
      const question = ask.querySelector('p').textContent;
      ask.querySelector('#covers-remember').click(); ask.querySelector('#covers-keep').click();
      await waitFor(() => !d.open, 90000, 'import finished'); await idle();
      const a = await scan(pathOf('sine-a.mp3')), b = await scan(pathOf('sine-b.mp3'));
      const saved = (await inv('load_settings')).settings.pages.messages || {};
      if (a.covers.length !== 1 || a.covers[0].size !== coverBefore[0].size || b.covers.length !== 1) throw new Error(`a=${JSON.stringify(a.covers.map(c => c.size))} b=${b.covers.length}`);
      if (saved.confirm_import_covers !== false || saved.import_covers_choice !== 'preserve') throw new Error('choice not remembered ' + JSON.stringify(saved));
      return `"${question.slice(0, 70)}…"; kept A's ${coverBefore[0].size}-byte cover, B got a ${b.covers[0].size}-byte cover; remembered ${saved.import_covers_choice}; ALBUM=${a.fields.ALBUM}`;
    });
  } catch (e) {
    await report('FATAL ' + (e && e.message || e));
  }
  await finish();
})();
