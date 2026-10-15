// Native e2e for the main window layout: docking the Tag Panel left, right,
// above and below the File List and floating it (View → Tag panel position,
// dragging the title bar, double-click), splitter sizes saved in the options,
// the horizontal panel's columns, the Filter beside the toolbars, the Custom
// Toolbar, the Mp3tag usrfields.ini import, Options → General → Notification
// area, and both color modes. Fixtures: tests/e2e/panel-dock-fixtures.sh.
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
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 15000, 'idle');
  const press = (target, k, o = {}) => { const event = new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }); target.dispatchEvent(event); return event; };
  const mac = /Mac/.test(navigator.platform), primary = mac ? { metaKey: true } : { ctrlKey: true };
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const openDialog = () => document.querySelector('dialog[open]');
  const closeDialogs = async () => { for (let i = 0; i < 5 && openDialog(); i++) { openDialog().close(); await sleep(150); } };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } finally { await closeDialogs(); document.querySelectorAll('details[open]').forEach(d => d.open = false); } };
  const setOptions = async (page, changes) => {
    document.querySelector('button[data-op="options"]').click();
    const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options dialog');
    dlg.querySelector(`[data-page="${page}"]`).click(); await sleep(150);
    for (const [path, value] of changes) {
      const el = dlg.querySelector(`[data-option="${path}"]`);
      if (!el) throw new Error('no control for ' + path);
      if (el.type === 'checkbox') el.checked = value; else el.value = String(value);
      el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); await sleep(60);
    }
    dlg.querySelector('#options-ok').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
    await idle();
  };
  const app = () => document.querySelector('.app');
  const rect = selector => document.querySelector(selector).getBoundingClientRect();
  const dockTo = async side => {
    const item = document.querySelector(`.view-dock-menu [data-panel-dock="${side}"]`);
    if (!item) throw new Error('no View → Tag panel position → ' + side);
    item.click();
    await waitFor(() => app().dataset.dock === side, 3000, 'docked ' + side);
    await sleep(150);
  };
  // The layout is saved shortly after it changes (dragging changes it often).
  const savedLayout = async expect => {
    const t = Date.now();
    while (Date.now() - t < 4000) { const layout = (await inv('load_settings')).settings.pages.layout; if (layout && expect(layout)) return layout; await sleep(200); }
    throw new Error('layout not saved: ' + JSON.stringify((await inv('load_settings')).settings.pages.layout));
  };
  const pointer = (target, type, x, y) => target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, button: 0, buttons: type === 'pointerup' ? 0 : 1, pointerId: 1, pointerType: 'mouse', isPrimary: true }));
  const drag = async to => {
    const title = document.querySelector('.editor .panel-title'), r = title.getBoundingClientRect(), x = r.left + 30, y = r.top + r.height / 2;
    pointer(title, 'pointerdown', x, y); await sleep(30);
    for (let i = 1; i <= 4; i++) { pointer(title, 'pointermove', x + (to.x - x) * i / 4, y + (to.y - y) * i / 4); await sleep(30); }
    if (!document.querySelector('.dock-overlay')) throw new Error('no dock preview while dragging');
    const target = document.querySelector('.dock-overlay').dataset.target;
    pointer(title, 'pointerup', to.x, to.y); await sleep(200);
    if (document.querySelector('.dock-overlay')) throw new Error('dock preview left behind');
    return target;
  };

  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    const album = rows()[0].dataset.path.replace(/[\\/][^\\/]+$/, ''), work = album.replace(/[\\/][^\\/]+$/, '');
    rows()[0].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(250);

    await step('the tag panel starts docked on the left of the file list', async () => {
      if (app().dataset.dock !== 'left') throw new Error('dock ' + app().dataset.dock);
      const editor = rect('.editor'), files = rect('.files');
      if (!(editor.right <= files.left + 1 && editor.width > 150)) throw new Error(`editor ${editor.left}-${editor.right}, files ${files.left}`);
      const menu = document.querySelector('.view-dock-menu'), filterMenu = document.querySelector('.view-filter-dock-menu');
      if (!menu || menu.querySelectorAll('[data-panel-dock]').length !== 5 || !filterMenu) throw new Error('View menu lacks the position submenus');
      if (menu.querySelector('[data-panel-dock="left"]').getAttribute('aria-checked') !== 'true') throw new Error('Left not checked');
      return `panel ${Math.round(editor.width)}px wide`;
    });

    await step('docked right: the panel follows the file list and its splitter grows it with ←; the layout is saved', async () => {
      await dockTo('right');
      const editor = rect('.editor'), files = rect('.files'), splitter = document.querySelector('.panel-splitter');
      if (!(editor.left >= files.right - 1)) throw new Error(`editor ${editor.left}, files right ${files.right}`);
      if (Math.abs(splitter.getBoundingClientRect().left + 4 - editor.left) > 3) throw new Error('splitter not at the panel edge');
      const before = Number(splitter.getAttribute('aria-valuenow'));
      splitter.focus(); press(splitter, 'ArrowLeft'); press(splitter, 'ArrowLeft'); await sleep(150);
      const after = Number(document.querySelector('.panel-splitter').getAttribute('aria-valuenow'));
      if (after !== before + 32) throw new Error(`width ${before} → ${after}`);
      if (Math.abs(rect('.editor').width - after) > 2) throw new Error('editor width ' + rect('.editor').width);
      const saved = await savedLayout(layout => layout.dock === 'right' && layout.width === after);
      // The panel menu offers the same positions.
      const menuItem = document.querySelector('.panel-menu [data-panel-command="dock:right"]');
      if (!menuItem || menuItem.getAttribute('aria-checked') !== 'true') throw new Error('panel menu position');
      return `width ${before} → ${after}, saved ${JSON.stringify({ dock: saved.dock, width: saved.width })}`;
    });

    await step('docked above: a horizontal panel with field columns and the cover on the right; ↓ grows it', async () => {
      await dockTo('top');
      const editor = rect('.editor'), files = rect('.files');
      if (!(editor.bottom <= files.top + 1 && editor.width > files.width - 2)) throw new Error(`editor ${editor.top}-${editor.bottom}, files ${files.top}`);
      const grids = document.querySelectorAll('.fields .panel-columns > .panel-grid');
      if (grids.length < 2) throw new Error('columns ' + grids.length);
      if (!(grids[1].getBoundingClientRect().left > grids[0].getBoundingClientRect().right - 1)) throw new Error('columns are not side by side');
      const cover = document.querySelector('.editor .cover-section');
      if (cover && !(cover.getBoundingClientRect().left >= rect('.fields').right - 1)) throw new Error('cover not right of the fields');
      // Editing still works: the title field drafts a change.
      const title = document.querySelector('.editor [data-field="TITLE"]'); title.focus(); input(title, 'Docked Title');
      await waitFor(() => document.querySelector('.dirty-count'), 3000, 'pending change');
      const splitter = document.querySelector('.panel-splitter');
      if (splitter.getAttribute('aria-orientation') !== 'horizontal') throw new Error('splitter orientation');
      const before = Number(splitter.getAttribute('aria-valuenow'));
      splitter.focus(); press(splitter, 'ArrowDown'); await sleep(200);
      const after = Number(document.querySelector('.panel-splitter').getAttribute('aria-valuenow'));
      if (after !== before + 16) throw new Error(`height ${before} → ${after}`);
      await savedLayout(layout => layout.dock === 'top' && layout.height === after);
      press(document.body, 'b', primary);
      await waitFor(() => !document.querySelector('.dirty-count'), 3000, 'change discarded');
      return `${grids.length} columns, height ${before} → ${after}`;
    });

    await step('docked below the file list', async () => {
      await dockTo('bottom');
      const editor = rect('.editor'), files = rect('.files');
      if (!(editor.top >= files.bottom - 1)) throw new Error(`editor top ${editor.top}, files bottom ${files.bottom}`);
      if (!document.querySelector('.fields .panel-columns')) throw new Error('no columns');
      return `panel ${Math.round(editor.height)}px high`;
    });

    await step('dragging the title bar into the file list floats the panel; double-click docks it again', async () => {
      const files = rect('.files'), target = await drag({ x: files.left + files.width / 2, y: files.top + files.height / 2 });
      if (target !== 'float') throw new Error('preview ' + target);
      await waitFor(() => app().dataset.dock === 'float', 3000, 'floating');
      const editor = document.querySelector('.editor');
      if (getComputedStyle(editor).position !== 'fixed') throw new Error('position ' + getComputedStyle(editor).position);
      if (getComputedStyle(editor).resize !== 'both') throw new Error('not resizable');
      if (rect('.files').width < window.innerWidth - 40) throw new Error('file list did not take the whole width');
      if (document.activeElement && document.activeElement !== document.body && !document.querySelector('.app').contains(document.activeElement)) throw new Error('focus left the window');
      await savedLayout(layout => layout.dock === 'float' && layout.last_dock === 'bottom');
      // Moving a floating panel by its title bar.
      const start = rect('.editor');
      const bar = rect('.editor .panel-title');
      await drag({ x: bar.left + 30 + 60, y: bar.top + bar.height / 2 + 40 });
      const moved = rect('.editor');
      if (Math.abs(moved.left - start.left - 60) > 3 || Math.abs(moved.top - start.top - 40) > 3) throw new Error(`moved ${start.left},${start.top} → ${moved.left},${moved.top}`);
      await savedLayout(layout => Math.abs(layout.float.x - moved.left) <= 2);
      document.querySelector('.editor .panel-title').dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true }));
      await waitFor(() => app().dataset.dock === 'bottom', 3000, 'docked again at the bottom');
      return `floated at ${Math.round(moved.left)},${Math.round(moved.top)}, then docked below again`;
    });

    await step('dragging the title bar to the left edge docks the panel there', async () => {
      const workspace = rect('.workspace');
      const target = await drag({ x: workspace.left + 12, y: workspace.top + workspace.height / 2 });
      if (target !== 'left') throw new Error('preview ' + target);
      await waitFor(() => app().dataset.dock === 'left', 3000, 'docked left');
      if (!(rect('.editor').right <= rect('.files').left + 1)) throw new Error('not left of the file list');
      await savedLayout(layout => layout.dock === 'left');
      return 'docked left';
    });

    await step('the filter docks right of the toolbars and keeps filtering; F3 still hides it', async () => {
      document.querySelector('.view-filter-dock-menu [data-filter-dock="toolbar-right"]').click();
      await waitFor(() => document.querySelector('.toolbar-row[data-filter="toolbar-right"] > .filterbar'), 3000, 'filter in the toolbar row');
      const filterbar = rect('.toolbar-row > .filterbar'), toolbar = rect('.toolbar-row > .toolbar');
      if (!(filterbar.left >= toolbar.left)) throw new Error('filter is not right of the toolbar');
      if (document.querySelector('.files .filterbar')) throw new Error('filter still above the file list');
      const filter = document.querySelector('#filter'); filter.focus(); input(filter, 'sine-a');
      await waitFor(() => rows().length === 1, 5000, 'filtered');
      press(document.querySelector('#filter'), 'F3');
      await waitFor(() => document.querySelector('.filterbar').hidden && rows().length >= 2, 3000, 'filter hidden');
      press(document.body, 'F3');
      await waitFor(() => !document.querySelector('.filterbar').hidden, 3000, 'filter shown');
      input(document.querySelector('#filter'), ''); await waitFor(() => rows().length >= 2, 3000, 'cleared');
      await savedLayout(layout => layout.filter === 'toolbar-right');
      document.querySelector('.view-filter-dock-menu [data-filter-dock="toolbar-left"]').click();
      await waitFor(() => document.querySelector('.toolbar-row[data-filter="toolbar-left"] > .filterbar:first-child'), 3000, 'filter left of the toolbars');
      document.querySelector('.view-filter-dock-menu [data-filter-dock="list"]').click();
      await waitFor(() => document.querySelector('.files > .filterbar'), 3000, 'filter above the file list again');
      return 'right, left, then above the list';
    });

    await step('Custom toolbar: add a command and a tool, shown next to the main toolbar and saved', async () => {
      const tool = mac || !/Win/.test(navigator.platform) ? '/bin/echo' : 'C:\\Windows\\System32\\whoami.exe';
      // Options → Tools: one tool to offer on the custom toolbar.
      document.querySelector('button[data-op="options"]').click();
      const options = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options');
      options.querySelector('[data-page="tools"]').click(); await sleep(150);
      options.querySelector('#tool-new').click(); await sleep(150);
      input(options.querySelector('#tool-name'), 'Echo path'); input(options.querySelector('#tool-path'), tool); await sleep(100);
      options.querySelector('#options-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed'); await idle();
      if (document.querySelector('.custom-toolbar')) throw new Error('an empty custom toolbar is shown');
      document.querySelector('[data-op="custom-toolbar-customize"]').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].toolbar-dialog'), 3000, 'customize custom toolbar');
      const choice = dlg.querySelector('#toolbar-choice');
      if (!choice.querySelector('option[value="tool:Echo path"]')) throw new Error('tool not offered');
      choice.value = 'reload'; dlg.querySelector('[data-tb="add"]').click();
      dlg.querySelector('[data-tb="spacer"]').click();
      choice.value = 'tool:Echo path'; dlg.querySelector('[data-tb="add"]').click();
      const listed = [...dlg.querySelectorAll('.toolbar-items [data-item] span')].map(s => s.textContent);
      if (listed.join() !== 'Reload,Spacer,Echo path') throw new Error('items ' + listed);
      dlg.querySelector('#toolbar-ok').click();
      await waitFor(() => document.querySelector('.custom-toolbar button[data-op="reload"]'), 3000, 'custom toolbar shown');
      const toolButton = document.querySelector('.custom-toolbar [data-custom-item="tool:Echo path"]');
      if (!toolButton || toolButton.disabled) throw new Error('tool button missing or disabled with a selection');
      if (!document.querySelector('.custom-toolbar .spacer')) throw new Error('spacer missing');
      toolButton.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
      if (!/Tool: start Echo path/.test(status())) throw new Error('hint ' + status());
      document.querySelector('.custom-toolbar').dispatchEvent(new PointerEvent('pointerleave'));
      const saved = (await inv('load_settings')).settings.pages['custom-toolbar'];
      if (saved.items.map(item => item.id.replace(/-\d+$/, '')).join() !== 'reload,spacer,tool:Echo path') throw new Error('saved ' + JSON.stringify(saved));
      document.querySelector('.view-toolbars-menu [data-op="toggle-custom-toolbar"]').click();
      await waitFor(() => !document.querySelector('.custom-toolbar'), 3000, 'custom toolbar hidden');
      document.querySelector('.view-toolbars-menu [data-op="toggle-custom-toolbar"]').click();
      await waitFor(() => document.querySelector('.custom-toolbar'), 3000, 'custom toolbar shown again');
      return 'Reload, spacer, Echo path';
    });

    await step('Mp3tag usrfields.ini import maps fields, labels, sizes, types and line breaks and reports the rest', async () => {
      const imported = await inv('import_mp3tag_tag_panel', { path: work + '/usrfields.ini' });
      const fields = imported.panel.fields;
      const summary = fields.map(f => `${f.field}:${f.size}:${f.type}${f.visible ? '' : ':hidden'}`).join(' ');
      if (summary !== 'TITLE:full:text ARTIST:full:text _SEPARATOR:full:text YEAR:normal:text TRACK:normal:text MOOD:full:multiline COMPILATION:small:check:hidden') throw new Error(summary);
      if (fields[1].name !== 'K&ünstler' || fields[5].rows !== 4) throw new Error('label or rows ' + JSON.stringify(fields[1]) + JSON.stringify(fields[5]));
      if (imported.skipped.length !== 1 || !/Path/.test(imported.skipped[0])) throw new Error('skipped ' + JSON.stringify(imported.skipped));
      // The panel menu and the Customize dialog offer the import.
      if (!document.querySelector('.panel-menu [data-panel-command="import-mp3tag"]')) throw new Error('panel menu entry missing');
      document.querySelector('button[data-op="panel-options"]').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].tag-panel-dialog'), 3000, 'customize tag panel');
      if (!dlg.querySelector('#tp-import-mp3tag')) throw new Error('dialog button missing');
      return `${fields.length} entries, skipped: ${imported.skipped[0]}`;
    });

    await step('Options → General → Notification area: off by default, opt-in on Windows and Linux', async () => {
      document.querySelector('button[data-op="options"]').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options');
      dlg.querySelector('[data-page="general"]').click(); await sleep(150);
      const icon = dlg.querySelector('[data-option="pages.general.tray_icon"]'), minimize = dlg.querySelector('[data-option="pages.general.minimize_to_tray"]');
      if (!icon || !minimize) throw new Error('notification area options missing');
      if (icon.checked || minimize.checked) throw new Error('not off by default');
      if (icon.disabled !== mac) throw new Error(`enabled state on ${navigator.platform}: disabled=${icon.disabled}`);
      dlg.close(); await sleep(150);
      if (mac) {
        const state = await inv('set_tray_options', { showIcon: true, minimizeToTray: true });
        if (state.supported) throw new Error('macOS reports a notification area');
        await inv('set_tray_options', { showIcon: false, minimizeToTray: false });
        return 'macOS: options disabled, no tray';
      }
      await setOptions('general', [['pages.general.tray_icon', true], ['pages.general.minimize_to_tray', true]]);
      await sleep(300);
      if (/Notification area/.test(errorsText())) throw new Error(errorsText());
      const general = (await inv('load_settings')).settings.pages.general;
      if (!general.tray_icon || !general.minimize_to_tray) throw new Error('not saved ' + JSON.stringify(general));
      const state = await inv('set_tray_options', { showIcon: true, minimizeToTray: true });
      if (!state.supported || !state.active) throw new Error('tray ' + JSON.stringify(state));
      await setOptions('general', [['pages.general.tray_icon', false], ['pages.general.minimize_to_tray', false]]);
      const off = await inv('set_tray_options', { showIcon: false, minimizeToTray: false });
      if (off.active) throw new Error('icon kept ' + JSON.stringify(off));
      return 'icon shown, then removed';
    });

    await step('light and dark color modes style every dock position', async () => {
      const luminance = color => { const [r, g, b] = color.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number).map(v => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
      const contrast = (a, b) => { const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
      const select = document.querySelector('#appearance'), seen = [];
      for (const mode of ['light', 'dark']) {
        select.value = mode; select.dispatchEvent(new Event('change', { bubbles: true }));
        await waitFor(() => document.documentElement.dataset.theme === mode, 3000, mode);
        for (const side of ['right', 'top', 'float']) {
          await dockTo(side);
          const editor = document.querySelector('.editor'), label = editor.querySelector('.panel-field label') || editor.querySelector('.panel-title');
          const background = getComputedStyle(editor).backgroundColor, text = getComputedStyle(label).color, ratio = contrast(background, text);
          if (ratio < 4.5) throw new Error(`${mode} ${side}: contrast ${ratio.toFixed(2)} (${text} on ${background})`);
          if ((mode === 'dark') !== (luminance(background) < 0.2)) throw new Error(`${mode} ${side}: background ${background}`);
          seen.push(`${mode}/${side} ${ratio.toFixed(1)}`);
        }
      }
      select.value = 'system'; select.dispatchEvent(new Event('change', { bubbles: true }));
      await dockTo('left');
      return seen.join(', ');
    });

    await step('Ctrl+Q hides the docked panel and the file list takes the space; the position is kept', async () => {
      await dockTo('top');
      press(document.body, 'q', { ctrlKey: true, code: 'KeyQ' });
      await waitFor(() => app().dataset.panel === 'hidden', 3000, 'panel hidden');
      if (rect('.files').top - rect('.workspace').top > 2) throw new Error('file list does not fill the workspace');
      document.querySelector('.view-dock-menu [data-panel-dock="right"]').click();
      await waitFor(() => app().dataset.panel === 'shown' && app().dataset.dock === 'right', 3000, 'choosing a position shows the panel');
      await dockTo('left');
      return 'hidden, then shown on the right';
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
