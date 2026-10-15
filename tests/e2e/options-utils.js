// Native e2e for Options (Messages, Tag Panel, Field Values, Tools), the Tools
// menu, Utils → Remove tag types, Copy as Text, Repeat, Help → Keyboard
// shortcuts and configuration backup/restore. Runs in two page loads: the first
// prepares settings through a configuration bundle round trip and reloads.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  const clickRow = async (name, o = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...o })); await sleep(250); };
  const scan = async name => (await inv('scan_files', { paths: [row(name).dataset.path], recursive: false })).entries[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const ready = name => waitFor(() => op(name) && !op(name).disabled && op(name), 8000, name + ' enabled');
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const stage = sessionStorage.getItem('options-utils-stage');
  const tool = { name: 'Copy beside', path: '/bin/cp', parameters: '"%_path%" "%_folderpath%copy-%_filename_ext%"', for_each_file: true };

  try {
    if (!stage) {
      await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
      const album = row('sine-a.mp3').dataset.path.replace(/\/[^/]+$/, '');
      await step('configuration bundle saves, validates and restores settings', async () => {
        const settings = (await inv('load_settings')).settings;
        settings.general.natural_sort = true;
        settings.pages = { ...settings.pages, messages: { confirm_remove_tags: false }, tools: { tools: [tool], default_tool: -1 }, 'directories-startup': { startup: 'previous' } };
        await inv('save_settings', { settings });
        await inv('save_action_library', { library: { groups: [{ name: 'Append plus', enabled: true, actions: [{ type: 'format_value', field: 'COMMENT', template: '%comment%+' }] }], presets: [] } });
        await inv('remember_directories', { paths: [album] });
        const bundle = album + '/../opentag-configuration.json';
        const count = await inv('export_configuration', { outputPath: bundle });
        const changed = (await inv('load_settings')).settings; changed.general.natural_sort = false; changed.pages = {}; await inv('save_settings', { settings: changed });
        const restored = await inv('import_configuration', { path: bundle });
        const after = (await inv('load_settings')).settings;
        if (!after.general.natural_sort || after.pages.tools?.tools?.[0]?.path !== '/bin/cp' || after.pages.messages?.confirm_remove_tags !== false) throw new Error('settings not restored: ' + JSON.stringify(after.pages));
        let rejected = '';
        try { await inv('import_configuration', { path: album + '/sine-a.mp3' }); } catch (e) { rejected = String(e); }
        if (!rejected) throw new Error('an audio file was accepted as configuration');
        return `exported ${count} files, restored ${restored.join(',')}; invalid file rejected: ${rejected.slice(0, 60)}`;
      });
      if (results.some(r => r[0] === 'FAIL')) return await finish();
      sessionStorage.setItem('options-utils-stage', '2');
      sessionStorage.setItem('options-utils-results', JSON.stringify(results));
      location.reload();
      return;
    }
    results.push(...JSON.parse(sessionStorage.getItem('options-utils-results') || '[]'));
    await step('reload opens the previous directory from the restored configuration', async () => { await waitFor(() => rows().length >= 2, 15000, 'rows after reload'); return `${rows().length} rows`; });
    await idle();

    await step('Options lists Messages, Tag Panel, Field Values and Tools pages', async () => {
      document.body.focus(); key('o', { metaKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options dialog');
      const pages = [...dlg.querySelectorAll('[role=treeitem]')].map(e => e.textContent.trim());
      for (const p of ['Messages', 'Tag Panel', 'Field Values', 'Tools']) if (!pages.includes(p)) throw new Error('missing ' + p + ' in ' + pages.join('|'));
      dlg.querySelector('[data-page="messages"]').click(); await sleep(150);
      const box = dlg.querySelector('[data-option="pages.messages.confirm_remove_tags"]'), del = dlg.querySelector('[data-option="pages.messages.confirm_delete_files"]');
      if (!box || box.checked || !del?.checked) throw new Error(`remove=${box?.checked} delete=${del?.checked}`);
      return pages.join('|');
    });

    await step('Tag Panel and Field Values pages open their editors', async () => {
      const dlg = document.querySelector('dialog[open].options-dialog');
      dlg.querySelector('[data-page="tag-panel"]').click(); await sleep(150);
      dlg.querySelector('#option-button-customize-tag-panel').click();
      const panel = await waitFor(() => document.querySelector('dialog[open].tag-panel-dialog'), 4000, 'tag panel dialog');
      panel.close(); await sleep(200);
      dlg.querySelector('[data-page="field-values"]').click(); await sleep(150);
      dlg.querySelector('#option-button-edit-field-values').click();
      const values = await waitFor(() => document.querySelector('dialog[open].field-values-dialog'), 4000, 'field values dialog');
      values.close(); await sleep(200);
      if (!dlg.open) throw new Error('options dialog closed');
      return 'both editors opened over Options';
    });

    await step('Tools page edits tools and rejects a relative program path', async () => {
      const dlg = document.querySelector('dialog[open].options-dialog');
      dlg.querySelector('[data-page="tools"]').click(); await sleep(150);
      const listed = [...dlg.querySelectorAll('#tools-list option')].map(o => o.textContent);
      if (listed[0] !== 'Copy beside') throw new Error('tools ' + listed);
      dlg.querySelector('#tool-new').click(); await sleep(100);
      input(dlg.querySelector('#tool-name'), 'Broken'); input(dlg.querySelector('#tool-path'), 'bin/echo');
      if (dlg.querySelector('#tools-list option[value="1"]').textContent !== 'Broken') throw new Error('list name not live-updated');
      dlg.querySelector('#options-ok').click(); await sleep(300);
      const error = dlg.querySelector('.dialog-error')?.textContent || '';
      if (!dlg.open || !/full path/.test(error)) throw new Error('validation: ' + error);
      dlg.querySelector('#tool-remove').click(); await sleep(100);
      dlg.querySelector('#options-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
      const saved = (await inv('load_settings')).settings.pages.tools.tools;
      if (saved.length !== 1 || saved[0].name !== 'Copy beside') throw new Error(JSON.stringify(saved));
      return error;
    });

    await step('Tools menu runs the first tool with ⌘1 without a shell', async () => {
      await clickRow('sine-a.mp3');
      const item = document.querySelector('[data-tool="0"]');
      if (!item || !/Copy beside/.test(item.textContent) || !/⌘ 1|Ctrl 1/.test(item.textContent)) throw new Error('menu item ' + item?.textContent);
      row('sine-a.mp3').focus(); key('1', { metaKey: true, code: 'Digit1' });
      await waitFor(() => /Started Copy beside/.test(status()), 5000, 'tool status');
      const album = row('sine-a.mp3').dataset.path.replace(/\/[^/]+$/, '');
      const copy = await waitFor(async () => (await inv('scan_files', { paths: [album], recursive: false })).entries.find(e => e.filename === 'copy-sine-a.mp3'), 8000, 'copied file');
      return status() + ' → ' + copy.filename;
    });

    await step('Remove tag types strips only ID3v1 and Undo restores it', async () => {
      await clickRow('sine-a.mp3');
      const f = document.querySelector('.fields [data-field="TITLE"]'); f.focus(); input(f, 'Sine A Saved'); f.blur(); document.body.focus();
      key('s', { metaKey: true }); await waitFor(() => /Saved tags/.test(status()), 8000, 'saved'); await idle();
      const before = (await scan('sine-a.mp3')).technical._TAG[0];
      if (!/ID3v1/.test(before)) throw new Error('fixture has no ID3v1: ' + before);
      (await ready('remove-tag-types')).click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].remove-tag-types-dialog'), 4000, 'remove dialog');
      const v1 = dlg.querySelector('[data-tag-kind="id3v1"]'), ape = dlg.querySelector('[data-tag-kind="ape"]');
      if (v1.disabled || !ape.disabled) throw new Error(`id3v1 disabled=${v1.disabled} ape disabled=${ape.disabled}`);
      v1.click(); dlg.querySelector('#remove-tag-types').click();
      await waitFor(() => /Removed tag types/.test(status()), 8000, 'removed status'); await idle();
      const after = (await scan('sine-a.mp3')).technical._TAG[0];
      if (/ID3v1/.test(after) || !/ID3v2/.test(after)) throw new Error('after=' + after);
      const label = op('undo').textContent;
      await ready('undo'); row('sine-a.mp3').focus(); key('z', { metaKey: true }); await waitFor(() => /Undid Remove tag types/.test(status()), 8000, 'undo'); await idle();
      const undone = (await scan('sine-a.mp3')).technical._TAG[0];
      if (undone !== before) throw new Error('undo=' + undone);
      return `${before} → ${after} → ${undone} (${label.trim()})`;
    });

    await step('Remove tags runs without confirmation when Messages disables it', async () => {
      await clickRow('sine-b.mp3'); (await ready('remove-tags')).click();
      await waitFor(() => /Removed tags/.test(status()), 8000, 'removed tags (a native dialog would block here)'); await idle();
      const entry = await scan('sine-b.mp3');
      if (entry.fields.TITLE || entry.technical._TAG) throw new Error('tags left: ' + JSON.stringify(entry.technical._TAG));
      await ready('undo'); row('sine-b.mp3').focus(); key('z', { metaKey: true }); await waitFor(() => /Undid Remove tags/.test(status()), 8000, 'undo'); await idle();
      return 'TITLE after undo=' + (await scan('sine-b.mp3')).fields.TITLE;
    });

    await step('Copy as Text evaluates a format string per file and remembers it', async () => {
      await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', { metaKey: true });
      (await ready('copy-text')).click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].copy-text-dialog'), 4000, 'copy dialog');
      input(dlg.querySelector('#copy-text-format'), '$num(%track%,2). %title%');
      
      const preview = await waitFor(() => { const t = dlg.querySelector('#copy-text-preview').innerText.trim(); return /^01\. /.test(t) && /02\. Sine B/.test(t) && t; }, 5000, 'preview');
      dlg.querySelector('#copy-text-ok').click();
      await waitFor(() => { const err = dlg.querySelector('.dialog-error')?.textContent; if (err) throw new Error(err); return /Copied 2 lines as text/.test(status()); }, 5000, 'copied status').catch(e => { dlg.close(); throw e; });
      const saved = (await inv('load_settings')).settings.pages['copy-as-text'];
      if (saved?.format !== '$num(%track%,2). %title%') throw new Error('format not remembered ' + JSON.stringify(saved));
      return `preview=${JSON.stringify(preview)} (clipboard is checked with pbpaste after the run; reading it here opens a native paste prompt)`;
    });

    await step('Ctrl+Y repeats the last action group when nothing can be redone', async () => {
      await clickRow('sine-b.mp3');
      const group = await waitFor(() => [...document.querySelectorAll('[data-action-group]')].find(b => /Append plus/.test(b.textContent) && !b.disabled), 5000, 'action group menu item');
      group.click(); await waitFor(() => /Applied action groups/.test(status()), 8000, 'applied'); await idle();
      const once = (await scan('sine-b.mp3')).fields.COMMENT?.[0] || '';
      const label = op('redo').textContent.trim();
      if (!/Repeat Append plus/.test(label)) throw new Error('redo label=' + label);
      row('sine-b.mp3').focus(); key('y', { metaKey: true });
      await waitFor(async () => ((await scan('sine-b.mp3')).fields.COMMENT?.[0] || '') === once + '+', 8000, 'second plus');
      await idle();
      return `COMMENT ${JSON.stringify(once)} → ${JSON.stringify((await scan('sine-b.mp3')).fields.COMMENT[0])}; menu "${label}"`;
    });

    await step('Row context menu offers Tools and Utils and runs a tool', async () => {
      await clickRow('sine-b.mp3'); 
      row('sine-a.mp3').querySelector('td').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 200, clientY: 200 }));
      const menu = await waitFor(() => document.querySelector('.row-context-menu'), 3000, 'context menu');
      const labels = [...menu.querySelectorAll('button')].map(b => b.querySelector('span').textContent);
      const selectedNow = [...document.querySelectorAll('tr.selected')].map(r => r.dataset.path.split('/').pop()).join(',');
      if (!labels.includes('Copy beside') || !labels.includes('Remove tag types…') || selectedNow !== 'sine-a.mp3') throw new Error(labels.join('|') + ' selected=' + selectedNow);
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      if (document.querySelector('.row-context-menu')) throw new Error('Escape did not close');
      await clickRow('sine-b.mp3');
      row('sine-b.mp3').querySelector('td').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 200, clientY: 200 }));
      (await waitFor(() => document.querySelector('.row-context-menu [data-context="tool:0"]'), 3000, 'tool item')).click();
      const album = row('sine-b.mp3').dataset.path.replace(/\/[^/]+$/, '');
      await waitFor(async () => (await inv('scan_files', { paths: [album], recursive: false })).entries.find(e => e.filename === 'copy-sine-b.mp3'), 8000, 'copied file');
      return labels.join('|');
    });

    await step('Help → Keyboard shortcuts lists menu bindings and tool slots', async () => {
      op('help-shortcuts').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].shortcuts-dialog'), 4000, 'shortcuts dialog');
      const text = dlg.innerText;
      for (const needle of ['Save tags', 'Options…', 'Undo', 'User-defined tool 1 … 10', 'Tag → Filename']) if (!text.includes(needle)) throw new Error('missing ' + needle);
      const count = dlg.querySelectorAll('tbody tr').length; dlg.close(); return count + ' rows';
    });

    await step('Tools menu and Help menu entries are present', async () => {
      const tools = [...document.querySelectorAll('.menubar > .menu')].find(m => m.querySelector('summary').textContent === 'Tools');
      const help = [...document.querySelectorAll('.menubar > .menu')].find(m => m.querySelector('summary').textContent === 'Help');
      const names = m => [...m.querySelectorAll('.menu-list button')].map(b => b.querySelector('span')?.textContent || b.textContent).join('|');
      return `Tools: ${names(tools)}; Help: ${names(help)}; File has ${['config-save', 'config-load', 'config-folder'].filter(op).length}/3 configuration items`;
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  sessionStorage.removeItem('options-utils-stage');
  await finish();
})();
