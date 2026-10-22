// Drives the debug app through named states for the user guide's screenshots.
// scripts/docs-screenshots.sh runs it in CI (Linux, xvfb) against the album
// from tests/docs-screens/fixtures.sh. Each `SHOT <name>` line asks the runner
// to capture the window; the state is held until the runner has done so.
// Run it only through a runner: the Linux CI job (scripts/docs-screenshots.sh) or the
// owner-approved macOS runner (scripts/docs-screenshots-macos.sh); both clean up after it.
//
// Scenes: the core scenes below, plus one file per writer workstream in
// tests/docs-screens/scenes/*.js (see the README there), which the runner
// loads before this script. `name` becomes
// docs/user-guide/assets/screens/<name>-light.png and <name>-dark.png.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 15000, 'idle');
  const op = name => { const b = document.querySelector(`.menubar button[data-op="${name}"]`) || document.querySelector(`button[data-op="${name}"]`); if (!b) throw new Error('no command ' + name); b.click(); };
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const dialog = () => document.querySelector('dialog[open]');
  const clickRow = async index => { rows()[index].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const selectAll = async () => { op('select-all'); await waitFor(() => rows().every(r => r.classList.contains('selected')), 3000, 'all selected'); };
  const menu = label => [...document.querySelectorAll('.menubar > .menu')].find(d => d.querySelector('summary')?.textContent.trim() === label);
  const reset = async () => {
    for (let i = 0; i < 5 && dialog(); i++) { dialog().close(); await sleep(150); }
    document.querySelectorAll('details[open]').forEach(d => { d.open = false; });
    document.activeElement?.blur?.();
    await idle();
    // A failed command leaves an error banner and a status message that would show up in the next
    // capture. Dismiss the banner, and re-read the tags when the status is not the plain "Loaded" text.
    document.querySelector('#dismiss-errors')?.click();
    const filterBox = document.querySelector('#filter'); if (filterBox && filterBox.value) { input(filterBox, ''); await sleep(500); }
    const status = document.querySelector('.status-message')?.textContent || '';
    if (status && !/^Loaded/i.test(status)) { try { op('reload'); await sleep(600); await idle(); } catch { /* no reload command */ } }
  };
  // Hold each state until the runner captures it. Captures take well under a
  // second under xvfb; the margin keeps slow runners from mixing scenes.
  // Focus is dropped first: WebKitGTK paints an empty popup for focused
  // inputs with suggestions when no window manager runs.
  // A scene may set `crop`: 'dialog' (the top open dialog), 'menu' (menu bar + open menu) or a CSS selector.
  // The rectangle is reported before the SHOT line for runners that crop
  // (CSS pixels plus the page's inner size, so a runner can scale them).
  const cropRect = crop => {
    if (!crop) return null;
    if (crop === 'menu') {
      // The menu bar plus the open menu's panel (the panel hangs outside its <details> box).
      const open = document.querySelector('.menubar > .menu[open]');
      const boxes = [document.querySelector('.menubar'), open, open?.querySelector(':scope > :not(summary)')].filter(Boolean).map(e => e.getBoundingClientRect());
      if (!open || boxes.length < 3) return null;
      const left = Math.min(...boxes.map(b => b.left)), top = Math.min(...boxes.map(b => b.top));
      const right = Math.max(boxes[2].right + 12, boxes[1].right + 300), bottom = Math.max(...boxes.map(b => b.bottom)) + 8;
      return [left, top, right - left, bottom - top, innerWidth, innerHeight].map(n => Math.round(n)).join(',');
    }
    const el = crop === 'dialog' ? [...document.querySelectorAll('dialog[open]')].at(-1) : document.querySelector(crop);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return [r.left, r.top, r.width, r.height, innerWidth, innerHeight].map(n => Math.round(n)).join(',');
  };
  let activeScene = null;
  const shot = async (name, crop) => {
    document.activeElement?.blur?.(); await sleep(900);
    const banner = document.querySelector('.errors p');
    if (banner && !activeScene?.allowError) throw new Error('error banner on screen: ' + banner.textContent.slice(0, 120));
    const rect = cropRect(crop);
    if (rect) await report('CROP ' + rect);
    await report('SHOT ' + name); await sleep(2500);
  };

  // Native sheets (confirmations, file pickers) block the page and cannot be dismissed from here, and
  // Tauri's invoke cannot be wrapped. The macOS runner therefore turns off Options → Messages →
  // "at unsaved changes" for the run (and restores the setting afterwards), and scenes avoid commands
  // that open a picker or a destructive confirmation.

  // The reader's real mouse pointer must not leave hover highlights in a capture
  // (a desktop run). Synthetic clicks dispatched below do not need hit-testing.
  const noHover = document.createElement('style');
  noHover.textContent = 'html { pointer-events: none !important; }';
  document.head.append(noHover);

  const scenes = [
    { name: 'main-window', run: async () => { await clickRow(0); } },
    { name: 'tag-panel-keep', run: async () => { await selectAll(); } },
    { name: 'menu-convert', crop: 'menu', run: async () => { await clickRow(0); const m = menu('Convert'); if (!m) throw new Error('no Convert menu'); m.open = true; } },
    {
      name: 'tag-to-filename', crop: 'dialog', run: async () => {
        await selectAll(); op('tags-filename');
        const d = await waitFor(() => dialog()?.querySelector('#template') && dialog(), 5000, 'Tag → Filename');
        input(d.querySelector('#template'), '$num(%track%,2) - %artist% - %title%');
        d.querySelector('#preview-button').click();
        await waitFor(() => d.querySelector('#preview .preview'), 8000, 'preview');
      }
    },
    {
      name: 'extended-tags', crop: 'dialog', run: async () => {
        await clickRow(1); op('extended');
        await waitFor(() => document.querySelector('dialog[open].extended-dialog #extended-rows .extended-row'), 5000, 'Extended tags');
      }
    },
  ];
  // Scene files register factories that receive these helpers.
  // Select the rows at the given indices (first plain click, the rest Ctrl/Cmd-click).
  const selectRows = async indices => {
    await clickRow(indices[0]);
    for (const i of indices.slice(1)) rows()[i].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ctrlKey: true, metaKey: true }));
    await waitFor(() => rows().filter(r => r.classList.contains('selected')).length === indices.length, 3000, indices.length + ' rows selected');
  };
  // Run a command and wait for its dialog (optionally until `selector` exists inside it).
  const openDialog = async (name, selector, ms = 8000) => {
    const button = document.querySelector(`button[data-op="${name}"]`);
    const why = () => ` [command ${button?.disabled ? 'disabled: ' + (button.title || '') : 'enabled'}; status "${document.querySelector('.status-message')?.textContent || ''}"; ${document.querySelectorAll('dialog[open]').length} dialog(s) open; ${rows().filter(r => r.classList.contains('selected')).length} rows selected]`;
    op(name);
    try { return await waitFor(() => { const d = dialog(); return d && (!selector || d.querySelector(selector)) && d; }, ms, 'dialog for ' + name + (selector ? ' (' + selector + ')' : '')); }
    catch (e) { throw new Error(e.message + why()); }
  };
  const openMenu = label => { const m = menu(label); if (!m) throw new Error('no ' + label + ' menu'); m.open = true; return m; };
  // Options dialog: switch to the page with this id (see src/options-ui.ts).
  const optionsPage = async (d, id) => {
    const b = d.querySelector(`[data-page="${id}"]`); if (!b) throw new Error('no options page ' + id);
    b.click(); await sleep(500);
    // Clicking a page scrolls the dialog so the tree item is visible, which also scrolls the page itself
    // out of view. Start every page at its top (a lower tree item may then sit below the fold).
    d.querySelectorAll('*').forEach(e => { if (e.scrollTop > 0) e.scrollTop = 0; });
    await sleep(400);
    return d;
  };
  // Real-music runs (scripts/docs-screenshots-macos.sh) point OPENTAG_DOCS_MUSIC at
  // a working copy of the owner's albums/ and songs/ folders. openAlbum('albums/R.E.M. - …')
  // replaces the list with that folder (or file) through the same path Finder's "Open With" uses.
  const currentFolder = () => document.querySelector('.location .path')?.textContent || '';
  const openAlbum = async relative => {
    const root = window.OPENTAG_DOCS_MUSIC;
    if (!root) throw new Error('OPENTAG_DOCS_MUSIC is not set; this scene needs the real-music runner');
    const path = root.replace(/\/+$/, '') + '/' + relative;
    if (currentFolder() === path) return;
    const url = 'file://' + encodeURI(path).replace(/#/g, '%23').replace(/\?/g, '%3F') + (/\.[A-Za-z0-9]{2,4}$/.test(relative) ? '' : '/');
    await inv('e2e_open_urls', { urls: [url] });
    await waitFor(() => rows().length > 0 && rows().every(r => r.dataset.path.startsWith(path)), 20000, 'rows of ' + relative);
    await idle(); await sleep(400);
  };
  // A scene can take extra images while it runs: snap('name', crop) saves name-<mode>.png.
  // List them in the scene's `also` array so a scene filter can select them.
  let currentMode = 'light';
  const snap = (name, crop) => shot(`${name}-${currentMode}`, crop);
  // music('albums', /Anyma/) -> 'albums/<folder name>' of the first match in the working copy.
  const music = (kind, pattern) => {
    const found = ((window.OPENTAG_DOCS_LIBRARY || {})[kind] || []).find(n => pattern.test(n));
    if (!found) throw new Error(`no ${kind} entry matching ${pattern}`);
    return kind + '/' + found;
  };
  const helpers = { inv, sleep, waitFor, rows, idle, op, input, dialog, clickRow, selectAll, menu, selectRows, openDialog, openMenu, optionsPage, report, openAlbum, snap, music };
  for (const factory of window.OPENTAG_DOCS_SCENE_FILES || []) scenes.push(...factory(helpers));
  const wanted = window.OPENTAG_DOCS_SCENES;
  let failures = 0, taken = 0;
  try {
    await waitFor(() => rows().length >= 5, 20000, 'fixture rows');
    await idle();
    const originalAppearance = document.querySelector('#appearance')?.value;
    for (const mode of ['light', 'dark']) {
      const select = document.querySelector('#appearance');
      input(select, mode);
      await waitFor(() => document.documentElement.dataset.theme === mode, 3000, mode + ' theme');
      currentMode = mode;
      for (const scene of scenes) {
        const names = [scene.name, ...(scene.also || [])];
        if ((window.OPENTAG_DOCS_SKIP || []).includes(scene.name)) continue;
        if (Array.isArray(wanted) && wanted.length && !names.some(n => wanted.includes(n))) continue;
        try {
          await reset();
          activeScene = scene;
          // Real-music runs: every scene starts on its own album (default: the first one).
          if (window.OPENTAG_DOCS_MUSIC) await openAlbum(scene.album || window.OPENTAG_DOCS_FIRST);
          await scene.run();
          if (!scene.noFinalShot) { await shot(`${scene.name}-${mode}`, scene.crop); taken++; }
        } catch (e) {
          failures++;
          await report(`FAIL ${scene.name}-${mode} — ${e && e.message || e}`);
        }
      }
    }
    // Leave the reader's own colour mode as it was (matters on a desktop run).
    if (originalAppearance) { await reset(); input(document.querySelector('#appearance'), originalAppearance); }
  } catch (e) {
    failures++;
    await report(`FATAL ${e && e.message || e}`);
  }
  await report(`SUMMARY ${taken} screenshots, ${failures} failed`);
  await inv('e2e_exit', { code: failures ? 1 : 0 });
})();
