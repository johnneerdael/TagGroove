// Native e2e for the remaining keyboard reference rows and main-window
// customization: Ctrl+Shift+Del, Ctrl+P / Ctrl+Shift+P, Ctrl+I / Ctrl+Shift+I,
// multi-selection in the Actions and Tag Sources dialogs, F3, Sort by, random
// order, toolbar hints, View → Toolbars and Customize toolbar, the Tag Panel
// context menu, automatic default values and the Directory field, the Extended
// Tags save mode and field clipboard, drop vs. Ctrl+drop and the Mp3tag
// columns.ini import. Fixtures: tests/e2e/keyboard-ui-fixtures.sh.
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const errorsText = () => [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' | ');
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const names = () => rows().map(r => r.dataset.path.split('/').pop());
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  // A plain click on the only selected row would start the slow-click cell editor (after 550 ms), which takes the keyboard; such a row is only focused.
  const clickRow = async (name, init = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); if (!Object.keys(init).length && r.classList.contains('selected') && rows().filter(x => x.classList.contains('selected')).length === 1) { r.focus(); await sleep(50); return; } r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...init })); await sleep(250); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 15000, 'idle');
  const press = (target, k, o = {}) => { const event = new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }); target.dispatchEvent(event); return event; };
  const ctrl = { ctrlKey: true };
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const openDialog = () => document.querySelector('dialog[open]');
  const closeDialogs = async () => { for (let i = 0; i < 5 && openDialog(); i++) { openDialog().close(); await sleep(150); } };
  const menuItem = (menu, text) => [...document.querySelectorAll('.menubar > details.menu')].find(d => d.querySelector('summary').textContent === menu)?.querySelector('.menu-list');
  const shortcutOf = (menu, opName) => menuItem(menu)?.querySelector(`button[data-op="${opName}"] small`)?.textContent || '';
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } finally { await closeDialogs(); document.querySelectorAll('details[open]').forEach(d => d.open = false); } };
  const setOptions = async (page, changes) => {
    op('options').click();
    const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options dialog');
    dlg.querySelector(`[data-page="${page}"]`).click(); await sleep(150);
    for (const [path, value] of changes) {
      const el = dlg.querySelector(`[data-option="${path}"]`) || dlg.querySelector(`#${path}`);
      if (!el) throw new Error('no control for ' + path);
      if (el.type === 'checkbox') el.checked = value; else el.value = String(value);
      el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); await sleep(60);
    }
    dlg.querySelector('#options-ok').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
    await idle();
  };
  const selectAll = async () => { document.body.focus(); press(document.body, 'a', ctrl); await waitFor(() => rows().every(r => r.classList.contains('selected')), 3000, 'all selected'); };
  const discardAll = async () => { await selectAll(); press(document.body, 'b', ctrl); await waitFor(() => !document.querySelector('.dirty-count'), 3000, 'drafts discarded'); };

  try {
    await waitFor(() => rows().length >= 4, 15000, 'fixture rows');
    const album = row('sine-a.mp3').dataset.path.replace(/\/[^/]+$/, ''), work = album.replace(/\/[^/]+$/, '');
    // Native confirmations cannot be answered here; Options → Messages turns the ones this suite meets off.
    await setOptions('messages', [['pages.messages.confirm_delete_files', false], ['pages.messages.unsaved_changes', false]]);

    await step('menus show the remaining reference shortcuts', async () => {
      const expected = [['Edit', 'file-delete-permanent', 'Ctrl ⇧ Del'], ['File', 'playlist-folders', '⌘/Ctrl ⇧ P'], ['File', 'playlist-all', '⌘/Ctrl P'], ['Tag Sources', 'source-last', '⌘/Ctrl I'], ['Tag Sources', 'source-last-quick', '⌘/Ctrl ⇧ I'], ['View', 'toggle-filter', 'F3']];
      for (const [menu, name, keys] of expected) if (shortcutOf(menu, name) !== keys) throw new Error(`${menu} ${name}: "${shortcutOf(menu, name)}"`);
      const view = menuItem('View');
      if (!view.querySelector('.view-sort-menu [data-sort-menu="title"]') || !view.querySelector('.view-toolbars-menu [data-op="toggle-toolbar"]') || !view.querySelector('[data-op="toolbar-customize"]')) throw new Error('Sort by or Toolbars submenu missing');
      if (shortcutOf('File', 'playlist-generate')) throw new Error('Generate playlists keeps a shortcut');
      return expected.length + ' shortcuts';
    });

    await step('F3 hides and shows the filter; a hidden filter shows every file; Ctrl+Tab skips it', async () => {
      const filter = document.querySelector('#filter'); filter.focus(); input(filter, 'sine-a');
      await waitFor(() => rows().length === 1, 5000, 'filtered');
      press(document.querySelector('#filter'), 'F3');
      await waitFor(() => document.querySelector('.filterbar').hidden && rows().length >= 4, 3000, 'filter hidden, all files');
      if (!/Filter hidden/.test(status())) throw new Error('status ' + status());
      await clickRow('sine-a.mp3'); row('sine-a.mp3').focus();
      const seen = [];
      for (let i = 0; i < 2; i++) { press(document.activeElement, 'Tab', ctrl); await sleep(150); seen.push(document.activeElement.closest('.editor') ? 'panel' : document.activeElement.id === 'filter' ? 'filter' : document.activeElement.closest('.table-wrap') ? 'list' : document.activeElement.tagName); }
      if (seen.join() !== 'panel,list') throw new Error('Ctrl+Tab order ' + seen);
      press(document.activeElement, 'F3');
      const shown = await waitFor(() => !document.querySelector('.filterbar').hidden && document.querySelector('#filter'), 3000, 'filter shown');
      await waitFor(() => document.activeElement === document.querySelector('#filter') && rows().length === 1, 3000, 'filter restored and focused');
      if (shown.value !== 'sine-a') throw new Error('filter text ' + shown.value);
      input(document.querySelector('#filter'), ''); await waitFor(() => rows().length >= 4, 3000, 'cleared');
      return `Ctrl+Tab while hidden: ${seen.join(' → ')}`;
    });

    await step('View → Sort by sorts by a column; choosing it again reverses the order', async () => {
      const item = () => document.querySelector('.view-sort-menu [data-sort-menu="title"]');
      item().click();
      await waitFor(() => document.querySelector('th[data-col="title"]').getAttribute('aria-sort') === 'ascending', 3000, 'title ascending');
      item().click();
      await waitFor(() => document.querySelector('th[data-col="title"]').getAttribute('aria-sort') === 'descending', 3000, 'title descending');
      if (item().getAttribute('aria-checked') !== 'true') throw new Error('menu does not mark the sort column');
      const order = names().join(',');
      document.querySelector('.view-sort-menu [data-sort-menu="filename"]').click();
      await waitFor(() => document.querySelector('th[data-col="filename"]').getAttribute('aria-sort') === 'ascending', 3000, 'filename ascending');
      return 'title ↓: ' + order;
    });

    await step('the icon at the start of the column headers puts the files in random order', async () => {
      const button = document.querySelector('th[data-col] [data-shuffle]');
      if (!button || button.closest('th') !== document.querySelector('thead th')) throw new Error('shuffle button missing from the first column');
      const before = names().slice().sort().join(',');
      button.click();
      await waitFor(() => /Shuffled \d+ files/.test(status()), 3000, 'shuffled status');
      if (document.querySelector('th[aria-sort="ascending"],th[aria-sort="descending"]')) throw new Error('sort criteria kept');
      if (names().slice().sort().join(',') !== before) throw new Error('files changed');
      document.querySelector('button[data-sort="filename"]').click();
      await waitFor(() => document.querySelector('th[data-col="filename"]').getAttribute('aria-sort') === 'ascending', 3000, 'sorted again');
      return status();
    });

    await step('toolbar buttons show their description in the status bar', async () => {
      const save = document.querySelector('.toolbar button[data-op="save"]'), before = status();
      save.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
      await waitFor(() => status().startsWith('Save the pending tag changes'), 2000, 'hint shown');
      const hint = status();
      document.querySelector('.toolbar').dispatchEvent(new PointerEvent('pointerleave'));
      await waitFor(() => status() === before, 2000, 'status restored');
      return hint;
    });

    await step('View → Toolbars → Main toolbar hides and shows the toolbar and remembers it', async () => {
      document.querySelector('.view-toolbars-menu [data-op="toggle-toolbar"]').click();
      await waitFor(() => !document.querySelector('.toolbar'), 3000, 'toolbar hidden');
      if ((await inv('load_settings')).settings.pages.toolbar?.shown !== false) throw new Error('not saved');
      if (document.querySelector('.view-toolbars-menu [data-op="toggle-toolbar"]').getAttribute('aria-checked') !== 'false') throw new Error('menu check');
      document.querySelector('.view-toolbars-menu [data-op="toggle-toolbar"]').click();
      await waitFor(() => document.querySelector('.toolbar'), 3000, 'toolbar shown');
      return 'hidden, then shown';
    });

    await step('Customize toolbar: Ctrl+D / Ctrl+E, Space, F4 rename and Alt+↑ with the keyboard', async () => {
      document.querySelector('[data-op="toolbar-customize"]').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].toolbar-dialog'), 3000, 'customize dialog');
      const list = dlg.querySelector('.toolbar-items'), checks = () => [...list.querySelectorAll('input[data-show]')].map(box => box.checked);
      press(list, 'd', ctrl); await sleep(100);
      if (checks().some(Boolean)) throw new Error('Ctrl+D left items shown');
      press(list, 'e', ctrl); await sleep(100);
      if (!checks().every(Boolean)) throw new Error('Ctrl+E left items hidden');
      press(list, 'Home'); press(list, ' '); await sleep(100);
      if (checks()[0]) throw new Error('Space did not hide the first item');
      press(list, 'ArrowDown'); press(list, 'F4');
      const rename = await waitFor(() => dlg.querySelector('.toolbar-rename'), 2000, 'rename field');
      rename.value = 'Add songs'; press(rename, 'Enter');
      press(dlg.querySelector('.toolbar-items'), 'ArrowUp', { altKey: true }); await sleep(100);
      const first = dlg.querySelector('.toolbar-items [data-item="0"] span').textContent;
      if (first !== 'Add songs') throw new Error('first item ' + first);
      dlg.querySelector('#toolbar-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].toolbar-dialog'), 3000, 'dialog closed');
      const buttons = [...document.querySelectorAll('.toolbar button[data-op]')];
      if (buttons[0].dataset.op !== 'files' || !buttons[0].textContent.includes('Add songs')) throw new Error('toolbar starts with ' + buttons[0].textContent);
      if (document.querySelector('.toolbar button[data-op="folder"]')) throw new Error('hidden item still shown');
      if (!document.querySelector('.toolbar button[data-op="undo"]')) throw new Error('Ctrl+E did not show Undo');
      const saved = (await inv('load_settings')).settings.pages.toolbar;
      if (saved.items[0].label !== 'Add songs') throw new Error('not saved ' + JSON.stringify(saved.items[0]));
      document.querySelector('[data-op="toolbar-customize"]').click();
      const again = await waitFor(() => document.querySelector('dialog[open].toolbar-dialog'), 3000, 'customize again');
      again.querySelector('[data-tb="reset"]').click(); again.querySelector('#toolbar-ok').click();
      await waitFor(() => document.querySelector('.toolbar button[data-op]')?.dataset.op === 'folder', 3000, 'reset toolbar');
      return `${buttons.length} buttons, then reset`;
    });

    await step('Actions dialog: Insert, multi-selection with ⌘/Ctrl+click, Ctrl+A, Ctrl+Shift+A, Space, Alt+↑ and Delete', async () => {
      document.body.focus(); press(document.body, '6', { code: 'Digit6', altKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog[open].action-groups-dialog'), 3000, 'actions dialog');
      for (const name of ['Alpha', 'Beta', 'Gamma']) {
        press(dlg.querySelector('.group-list'), 'Insert');
        const child = await waitFor(() => [...document.querySelectorAll('dialog[open]')].find(d => d !== dlg && d.querySelector('[data-key="name"]')), 3000, 'name dialog');
        input(child.querySelector('[data-key="name"]'), name); child.querySelector('[data-keep]').click();
        await waitFor(() => !child.open, 2000, 'name kept');
      }
      const groupRows = () => [...dlg.querySelectorAll('[data-group-row]')], marked = () => groupRows().filter(r => r.classList.contains('marked')).map(r => r.querySelector('span').textContent);
      if (groupRows().length !== 3) throw new Error('groups ' + groupRows().length);
      groupRows()[0].querySelector('span').dispatchEvent(new MouseEvent('click', { bubbles: true }));
      groupRows()[2].querySelector('span').dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
      if (marked().join() !== 'Alpha,Gamma') throw new Error('Ctrl+click marked ' + marked());
      press(groupRows()[2], 'A', { ctrlKey: true, shiftKey: true });
      if (marked().join() !== 'Beta') throw new Error('Ctrl+Shift+A marked ' + marked());
      press(dlg.querySelector('[data-group-row].current'), 'a', ctrl);
      if (marked().join() !== 'Alpha,Beta,Gamma') throw new Error('Ctrl+A marked ' + marked());
      press(dlg.querySelector('[data-group-row].current'), ' ');
      if (groupRows().some(r => r.querySelector('input').checked)) throw new Error('Space did not disable every selected group');
      groupRows()[0].querySelector('span').dispatchEvent(new MouseEvent('click', { bubbles: true }));
      groupRows()[2].querySelector('span').dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
      press(dlg.querySelector('[data-group-row].current'), 'ArrowUp', { altKey: true });
      const order = groupRows().map(r => r.querySelector('span').textContent).join();
      if (order !== 'Alpha,Gamma,Beta') throw new Error('Alt+↑ order ' + order);
      press(dlg.querySelector('[data-group-row].current'), 'Delete');
      if (groupRows().map(r => r.querySelector('span').textContent).join() !== 'Beta') throw new Error('Delete left ' + groupRows().map(r => r.textContent));
      // The group editor's action list: two actions, Ctrl+A, Delete.
      press(groupRows()[0], 'F4');
      const editor = await waitFor(() => document.querySelector('dialog[open].group-editor-dialog'), 3000, 'group editor');
      for (let i = 0; i < 2; i++) {
        editor.querySelector('[data-command="new"]').click();
        const action = await waitFor(() => [...document.querySelectorAll('dialog[open]')].find(d => d !== editor && d !== dlg && d.querySelector('#full-action-type')), 3000, 'action dialog');
        action.querySelector('[data-keep]').click(); await waitFor(() => !action.open, 2000, 'action kept');
      }
      const actions = () => [...editor.querySelectorAll('[data-action-index]')];
      if (actions().length !== 2) throw new Error('actions ' + actions().length);
      press(actions()[0], 'a', ctrl);
      if (actions().filter(a => a.classList.contains('marked')).length !== 2) throw new Error('Ctrl+A in the action list');
      press(editor.querySelector('[data-action-index]'), 'Delete');
      if (actions().length) throw new Error('Delete left actions');
      return 'groups and actions';
    });

    await step('Ctrl+Shift+P writes one playlist per folder of all files; Ctrl+P asks for the file name', async () => {
      await clickRow('sine-a.mp3');
      press(document.body, 'P', { ctrlKey: true, shiftKey: true });
      await waitFor(() => /Created 2 playlists with 4 files/.test(status()), 8000, 'per-folder playlists (status "' + status() + '" errors "' + errorsText() + '")');
      const top = await inv('read_playlist', { path: album + '/album.m3u8' }), sub = await inv('read_playlist', { path: album + '/sub/sub.m3u8' });
      if (top.files.length !== 3 || sub.files.length !== 1 || !sub.files[0].endsWith('sine-sub.mp3')) throw new Error(`playlists ${top.files.length}/${sub.files.length}`);
      press(document.body, 'p', ctrl);
      const dlg = await waitFor(() => document.querySelector('dialog[open].playlist-dialog'), 3000, 'playlist dialog');
      return `album.m3u8 ${top.files.length} files, sub/sub.m3u8 ${sub.files.length}; Ctrl+P: ${dlg.querySelector('#playlist-filename').value.split('/').pop()}`;
    });

    await step('Tag Sources: field multi-selection with Ctrl+A, Ctrl+Shift+A and Space; Ctrl+I and Ctrl+Shift+I reopen the last source', async () => {
      await setOptions('tag-sources-local-freedb', [['local-freedb-directory', work + '/freedb']]);
      await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', { ctrlKey: true });
      document.body.focus(); press(document.body, 'l', ctrl);
      const d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, 'freedb dialog');
      const mode = d.querySelector('#ts-mode'); mode.value = 'disc_id'; mode.dispatchEvent(new Event('change', { bubbles: true }));
      d.querySelector('#ts-text').value = '0A000802'; d.querySelector('#ts-search').click();
      await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 10000, 'results');
      d.querySelector('#ts-next').click();
      await waitFor(() => d.querySelector('.ts-fields tr[data-row]'), 10000, 'review');
      const fieldRows = () => [...d.querySelectorAll('.ts-fields tr[data-row]')], selectedRows = () => fieldRows().filter(r => r.classList.contains('selected'));
      fieldRows()[0].querySelector('th').dispatchEvent(new MouseEvent('click', { bubbles: true }));
      fieldRows()[2].querySelector('th').dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
      if (selectedRows().length !== 2) throw new Error('⌘/Ctrl+click selected ' + selectedRows().length);
      press(fieldRows()[0], 'A', { ctrlKey: true, shiftKey: true });
      if (selectedRows().length !== fieldRows().length - 2) throw new Error('Ctrl+Shift+A selected ' + selectedRows().length);
      press(fieldRows()[0], 'a', ctrl);
      if (selectedRows().length !== fieldRows().length) throw new Error('Ctrl+A selected ' + selectedRows().length);
      press(fieldRows()[1].querySelector('input[type=checkbox]'), ' ');
      const boxes = fieldRows().map(r => r.querySelector('input[type=checkbox]').checked);
      if (boxes.some(Boolean)) throw new Error('Space left fields enabled');
      if (!/Choose at least one field/.test(d.querySelector('.ts-status').textContent) && !d.querySelector('#ts-apply').disabled) throw new Error('apply state');
      d.close(); await sleep(200);
      press(document.body, 'i', ctrl);
      const again = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, 'Ctrl+I dialog');
      if (again.getAttribute('aria-label') !== 'freedb (local)') throw new Error('Ctrl+I opened ' + again.getAttribute('aria-label'));
      const idleText = again.querySelector('.ts-status').textContent;
      again.close(); await sleep(200);
      press(document.body, 'I', { ctrlKey: true, shiftKey: true });
      const quick = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, 'Ctrl+Shift+I dialog');
      await waitFor(() => quick.querySelector('.ts-status').textContent !== idleText, 8000, 'quick search ran without pressing Search');
      const menu = menuItem('Tag Sources').querySelector('button[data-op="source-last"] span').textContent;
      if (!menu.includes('freedb (local)')) throw new Error('menu ' + menu);
      return `${fieldRows().length} fields; quick: ${quick.querySelector('.ts-status').textContent}`;
    });

    await step('Extended Tags save mode: Save writes, Ignore discards, Ask offers both, when moving with << >>', async () => {
      const edit = async (text) => { const dlg = await waitFor(() => document.querySelector('dialog.extended-dialog[open]'), 3000, 'extended'); const row = [...dlg.querySelectorAll('.extended-row')].find(r => r.querySelector('input').value === 'TITLE'); input(row.querySelector('textarea'), text); return dlg; };
      const titleOf = async name => (await inv('scan_files', { paths: [row(name).dataset.path], recursive: false })).entries[0].fields.TITLE?.[0];
      await setOptions('tags', [['pages.tags.extended_save_mode', 'save']]);
      await clickRow('sine-a.mp3'); press(document.body, 't', { code: 'KeyT', altKey: true });
      let dlg = await edit('Saved By Mode');
      dlg.querySelector('[data-nav="1"]').click();
      await waitFor(() => /sine-b\.mp3/.test(dlg.querySelector('header').textContent), 5000, 'moved to sine-b');
      await idle();
      if (await titleOf('sine-a.mp3') !== 'Saved By Mode') throw new Error('Save mode did not write');
      if (document.querySelector('.dirty-count')) throw new Error('draft left after saving');
      dlg.close(); await sleep(200);
      await setOptions('tags', [['pages.tags.extended_save_mode', 'ignore']]);
      await clickRow('sine-b.mp3'); press(document.body, 't', { code: 'KeyT', altKey: true });
      dlg = await edit('Ignored');
      dlg.querySelector('[data-nav="-1"]').click();
      await waitFor(() => /sine-a\.mp3/.test(dlg.querySelector('header').textContent), 5000, 'moved to sine-a');
      if (document.querySelector('.dirty-count') || await titleOf('sine-b.mp3') !== 'Sine B') throw new Error('Ignore kept the edit');
      dlg.close(); await sleep(200);
      await setOptions('tags', [['pages.tags.extended_save_mode', 'ask']]);
      await clickRow('sine-a.mp3'); press(document.body, 't', { code: 'KeyT', altKey: true });
      dlg = await edit('Asked');
      dlg.querySelector('[data-nav="1"]').click();
      const ask = await waitFor(() => document.querySelector('dialog[open].extended-ask-dialog'), 3000, 'ask dialog');
      ask.querySelector('#extended-discard').click();
      await waitFor(() => /sine-b\.mp3/.test(dlg.querySelector('header').textContent), 5000, 'moved after Don\'t save');
      if (document.querySelector('.dirty-count') || await titleOf('sine-a.mp3') !== 'Saved By Mode') throw new Error('Don\'t save changed the file');
      dlg.close(); await sleep(200);
      await setOptions('tags', [['pages.tags.extended_save_mode', 'pending']]);
      return 'save, ignore, ask';
    });

    await step('Extended Tags field list: Cut, Copy and Paste fields from the context menu', async () => {
      await clickRow('sine-a.mp3'); press(document.body, 't', { code: 'KeyT', altKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog.extended-dialog[open]'), 3000, 'extended');
      const field = name => [...dlg.querySelectorAll('.extended-row')].filter(r => !r.hidden).find(r => r.querySelector('input').value === name);
      const menu = target => { target.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 100, clientY: 100 })); return dlg.querySelector('.extended-menu'); };
      let m = menu(field('ARTIST'));
      if (!m || m.querySelector('[data-context="paste"]').disabled === false) throw new Error('menu or empty clipboard');
      m.querySelector('[data-context="cut"]').click();
      if (field('ARTIST')) throw new Error('Cut left the field');
      m = menu(dlg.querySelector('#extended-rows'));
      m.querySelector('[data-context="paste"]').click();
      const pasted = field('ARTIST');
      if (!pasted || pasted.querySelector('textarea').value !== 'Test Artist') throw new Error('paste ' + (pasted && pasted.querySelector('textarea').value));
      m = menu(field('ALBUM')); m.querySelector('[data-context="copy"]').click();
      m = menu(field('TITLE')); m.querySelector('[data-context="paste"]').click();
      if (field('ALBUM').querySelector('textarea').value !== 'Test Album') throw new Error('copy changed the source');
      dlg.querySelector('#apply-extended').click(); await sleep(200);
      if (document.querySelector('.dirty-count')) throw new Error('unchanged fields left a draft');
      return 'cut + paste restores ARTIST, copy + paste';
    });

    await step('Tag Panel: right-click opens the panel menu at the pointer; it hides and shows fields', async () => {
      await clickRow('sine-a.mp3');
      const title = document.querySelector('.editor .panel-title');
      title.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 60, clientY: 140 }));
      const menu = await waitFor(() => document.querySelector('.editor .panel-menu[open]'), 2000, 'panel menu');
      const composer = [...menu.querySelectorAll('[data-panel-command^="field:"]')].find(b => b.textContent.includes('Composer'));
      composer.click();
      await waitFor(() => !document.querySelector('.fields [data-field="COMPOSER"]'), 3000, 'Composer hidden');
      document.querySelector('.editor .fields').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 60, clientY: 200 }));
      const again = await waitFor(() => document.querySelector('.editor .panel-menu[open]'), 2000, 'panel menu again');
      [...again.querySelectorAll('[data-panel-command^="field:"]')].find(b => b.textContent.includes('Composer')).click();
      await waitFor(() => document.querySelector('.fields [data-field="COMPOSER"]'), 3000, 'Composer shown');
      const field = document.querySelector('.fields [data-field="TITLE"]');
      const native = field.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
      if (!native) throw new Error('text fields lost the system menu');
      return 'menu at pointer';
    });

    await step('Tag Panel: default values fill in automatically when several files are selected', async () => {
      op('panel-options').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].tag-panel-dialog'), 3000, 'customize tag panel');
      const comment = [...dlg.querySelectorAll('.panel-config-row')].find(r => r.querySelector('small').textContent === 'COMMENT');
      comment.click(); input(dlg.querySelector('#tp-default'), 'Auto default'); dlg.querySelector('#tp-ok').click();
      await waitFor(() => !dlg.open, 3000, 'saved');
      await clickRow('sine-a.mp3');
      if (document.querySelector('.dirty-count')) throw new Error('one file got defaults');
      await clickRow('sine-b.mp3', { shiftKey: true });
      await waitFor(() => document.querySelector('.dirty-count'), 3000, 'defaults pending');
      const value = document.querySelector('.fields [data-field="COMMENT"]').value, text = status();
      await discardAll();
      op('panel-options').click();
      const reset = await waitFor(() => document.querySelector('dialog[open].tag-panel-dialog'), 3000, 'customize again');
      [...reset.querySelectorAll('.panel-config-row')].find(r => r.querySelector('small').textContent === 'COMMENT').click();
      input(reset.querySelector('#tp-default'), '< keep >'); reset.querySelector('#tp-ok').click();
      await waitFor(() => !reset.open, 3000, 'reset');
      if (value !== 'Auto default') throw new Error('panel shows ' + value);
      return text;
    });

    await step('Ctrl+Shift+Del deletes the selected file permanently', async () => {
      const path = row('zz-delete.mp3').dataset.path;
      await clickRow('zz-delete.mp3'); row('zz-delete.mp3').focus();
      press(row('zz-delete.mp3'), 'Delete', { ctrlKey: true, shiftKey: true });
      await waitFor(() => !row('zz-delete.mp3'), 5000, 'row removed (status "' + status() + '" errors "' + errorsText() + '")');
      const scan = await inv('scan_files', { paths: [path], recursive: false });
      if (scan.entries.length) throw new Error('file still on disk');
      return status();
    });

    await step('Mp3tag columns.ini import reads the user-supplied file', async () => {
      const imported = await inv('import_mp3tag_columns', { path: work + '/mp3tag-columns.ini' });
      const columns = imported.layout.columns;
      if (columns.map(c => c.name).join() !== 'File,Künstler,Track') throw new Error('columns ' + columns.map(c => c.name));
      const track = columns[2];
      if (track.visible || !track.numeric || track.align !== 'right' || track.sort_by !== '%track%' || columns[1].field !== '%ARTIST%') throw new Error(JSON.stringify(columns));
      return `${columns.length} columns from UTF-16 columns.ini`;
    });

    await step('typing a folder into the Directory field and pressing Enter opens it', async () => {
      await clickRow('sine-a.mp3');
      const dir = document.querySelector('.fields [data-directory]');
      if (dir.readOnly) throw new Error('Directory field is read-only');
      dir.focus(); input(dir, album + '/sub'); press(dir, 'Enter');
      await waitFor(() => rows().length === 1 && row('sine-sub.mp3'), 8000, 'sub folder listed (errors "' + errorsText() + '")');
      return names().join(',');
    });

    await step('a drop replaces the File List; with Ctrl held it adds the dropped files', async () => {
      // Over the File List, away from the cover window (positions are physical pixels).
      const r = document.querySelector('.table-wrap').getBoundingClientRect(), dpr = window.devicePixelRatio || 1;
      const position = { x: (r.left + r.width / 2) * dpr, y: (r.top + r.height / 2) * dpr };
      const drop = async paths => { await idle(); await inv('plugin:event|emit', { event: 'tauri://drag-drop', payload: { paths, position } }); };
      // Earlier synthetic shortcuts never sent key-up events; release Control first.
      window.dispatchEvent(new KeyboardEvent('keyup', { key: 'Control' }));
      await drop([album]);
      await waitFor(() => rows().length === 3 && row('sine-sub.mp3') && row('sine-a.mp3'), 8000, 'replaced by the album');
      await drop([album + '/sub']);
      await waitFor(() => rows().length === 1 && row('sine-sub.mp3'), 8000, 'replaced by sub');
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true }));
      await drop([album + '/sine-a.mp3']);
      await waitFor(() => rows().length === 2 && row('sine-a.mp3') && row('sine-sub.mp3'), 8000, 'Ctrl+drop added');
      window.dispatchEvent(new KeyboardEvent('keyup', { key: 'Control' }));
      const modifiers = await inv('modifier_keys');
      if (typeof modifiers.control !== 'boolean' || typeof modifiers.command !== 'boolean') throw new Error('modifier_keys ' + JSON.stringify(modifiers));
      return `${names().join(',')}; native modifiers ${JSON.stringify(modifiers)}`;
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
