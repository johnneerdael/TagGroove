// Native cover workflows: paste, context menu type/description, clipboard
// copy/paste between files, aggregated multi-selection, Manage covers (apply
// to all, reorder, undo), extract, drag and drop onto the cover window and the
// larger viewer. Run: OPENTAG_E2E_ID=org.opentag.editor.e2e.covers scripts/e2e-native.sh tests/e2e/covers.js
// Copy/Paste cover use the real system clipboard.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const errorsText = () => [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' | ');
  const row = name => [...document.querySelectorAll('tr[data-path]')].find(r => r.dataset.path.endsWith('/' + name));
  const pathOf = name => row(name).dataset.path;
  const clickRow = async (name, opts = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...opts })); await sleep(300); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const covers = async name => (await inv('scan_files', { paths: [pathOf(name)], recursive: false })).entries[0].covers;
  const art = () => document.querySelector('.cover-art');
  const info = () => document.querySelector('.cover-info')?.textContent || '';
  const key = (target, k, o = {}) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const done = async (pattern, what) => { await waitFor(() => pattern.test(status()) && !document.querySelector('footer.status.busy'), 10000, what + ' (status "' + status() + '" errors "' + errorsText() + '")'); };
  const menuItem = label => [...document.querySelectorAll('.cover-menu button')].find(b => b.textContent.replace('✓ ', '').trim().startsWith(label));
  const openMenu = async () => { document.querySelector('.cover-menu')?.remove(); const a = art(); const r = a.getBoundingClientRect(); a.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 10, clientY: r.top + 10 })); return waitFor(() => document.querySelector('.cover-menu'), 3000, 'cover menu'); };
  const png = (w, h, color) => new Promise(resolve => { const c = document.createElement('canvas'); c.width = w; c.height = h; const g = c.getContext('2d'); g.fillStyle = color; g.fillRect(0, 0, w, h); g.fillStyle = '#fff'; g.fillRect(0, 0, w / 2, h / 2); c.toBlob(b => resolve(new File([b], 'canvas.png', { type: 'image/png' })), 'image/png'); });
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  try {
    await waitFor(() => document.querySelectorAll('tr[data-path]').length >= 2, 15000, 'fixture rows');

    await step('empty cover window offers drop/paste hint and Edit > Manage covers', async () => {
      await clickRow('sine-a.mp3');
      if (!/No artwork/.test(info())) throw new Error('info=' + info());
      const item = document.querySelector('button[data-op="cover-manage"]'); if (!item) throw new Error('no Manage covers menu item');
      return info().trim() + ' | menu: ' + item.textContent.trim();
    });

    await step('paste image event on cover art embeds it (dimensions/size/format shown)', async () => {
      const file = await png(300, 200, '#c33');
      const data = new DataTransfer(); data.items.add(file);
      art().focus(); art().dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
      await done(/Added cover/, 'added cover');
      const c = await covers('sine-a.mp3');
      if (c.length !== 1 || c[0].width !== 300 || c[0].height !== 200 || c[0].mime !== 'image/png') throw new Error(JSON.stringify(c));
      await waitFor(() => /300×200, .* PNG/.test(info()) && document.querySelector('.cover-art img[data-cover-image]')?.complete, 5000, 'details in cover info');
      return info().replace(/\s+/g, ' ').trim();
    });

    await step('undo/redo of the added cover', async () => {
      row('sine-a.mp3').focus(); await waitFor(() => !document.querySelector('button[data-op="undo"]')?.disabled, 5000, 'undo');
      key(row('sine-a.mp3'), 'z', { metaKey: true }); await done(/Undid/, 'undo');
      if ((await covers('sine-a.mp3')).length !== 0) throw new Error('cover not removed by undo');
      row('sine-a.mp3').focus(); key(row('sine-a.mp3'), 'y', { metaKey: true }); await done(/Redid/, 'redo');
      if ((await covers('sine-a.mp3')).length !== 1) throw new Error('cover not restored by redo');
      return status();
    });

    await step('context menu lists the documented commands', async () => {
      await clickRow('sine-a.mp3'); const menu = await openMenu();
      const labels = [...menu.querySelectorAll(':scope > button, :scope > .cover-menu-parent > button')].map(b => b.querySelector('span').textContent.replace('✓ ', ''));
      for (const l of ['Add cover…', 'Remove cover', 'Replace cover…', 'Extract cover…', 'Adjust cover…', 'Manage covers…', 'Set cover type', 'Set cover description…', 'Cut cover', 'Copy cover', 'Paste cover', 'Correct aspect ratio']) if (!labels.includes(l)) throw new Error('missing ' + l + ' in ' + labels.join('|'));
      if (document.activeElement?.tagName === 'BUTTON' && menu.contains(document.activeElement)) throw new Error('mouse-opened menu should not focus an item');
      key(menu, 'Escape'); await sleep(100); if (document.querySelector('.cover-menu')) throw new Error('Escape did not close');
      return labels.length + ' items';
    });

    await step('Set cover type submenu (keyboard) writes Back Cover', async () => {
      const a = art(); a.focus(); key(a, 'F10', { shiftKey: true });
      const menu = await waitFor(() => document.querySelector('.cover-menu'), 3000, 'menu');
      const type = menuItem('Set cover type'); type.focus(); key(type, 'ArrowRight');
      const back = await waitFor(() => [...menu.querySelectorAll('.cover-submenu button')].find(b => b.textContent.includes('Back Cover')), 2000, 'submenu');
      back.click(); await done(/Set cover type to Back Cover/, 'set type');
      const c = await covers('sine-a.mp3'); if (c[0].picture_type !== 'CoverBack') throw new Error(JSON.stringify(c));
      return info().replace(/\s+/g, ' ').trim();
    });

    await step('Set cover description dialog', async () => {
      await openMenu(); menuItem('Set cover description').click();
      const input = await waitFor(() => document.querySelector('dialog[open] #cover-description-value'), 3000, 'description dialog');
      input.value = 'Rear sleeve'; document.querySelector('#cover-description-ok').click();
      await done(/Set cover description/, 'description');
      const c = await covers('sine-a.mp3'); if (c[0].description !== 'Rear sleeve') throw new Error(JSON.stringify(c));
      return c[0].description;
    });

    await step('Copy cover (⌘C) then Paste cover (⌘V) on another file via the system clipboard', async () => {
      await clickRow('sine-a.mp3'); await waitFor(() => document.querySelector('.cover-art img[data-cover-image]'), 5000, 'image');
      key(art(), 'c', { metaKey: true }); await waitFor(() => /Copied the Back Cover/.test(status()), 5000, 'copied (' + errorsText() + ')');
      await clickRow('sine-b.mp3'); key(art(), 'v', { metaKey: true }); await done(/Pasted cover/, 'pasted');
      const c = await covers('sine-b.mp3'); if (c.length !== 1 || c[0].width !== 300 || c[0].height !== 200) throw new Error(JSON.stringify(c));
      return `${c[0].picture_type} ${c[0].width}x${c[0].height} ${c[0].mime}`;
    });

    await step('multi-selection with different covers shows the varying hint', async () => {
      await clickRow('sine-a.mp3'); await clickRow('sine-b.mp3', { shiftKey: true });
      await waitFor(() => /Covers vary/.test(info()), 3000, 'varying hint, info=' + info());
      if (document.querySelector('.cover-art img[data-cover-image]')) throw new Error('a cover image is shown for varying covers');
      return info().replace(/\s+/g, ' ').trim();
    });

    await step('Manage covers (Ctrl+Alt+M): apply to all, reorder, OK writes both files', async () => {
      key(document.body, 'µ', { code: 'KeyM', ctrlKey: true, altKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog[open].manage-covers-dialog'), 5000, 'manage dialog');
      await waitFor(() => dlg.querySelectorAll('li[data-id]').length === 2, 3000, 'two covers');
      const presence = [...dlg.querySelectorAll('.cover-presence')].map(p => p.getAttribute('aria-label')).join('|');
      if (presence !== 'In 1 of 2 files|In 1 of 2 files') throw new Error('presence ' + presence);
      const rows = () => [...dlg.querySelectorAll('li[data-id]')];
      const pick = r => { r.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, clientY: 1 })); r.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, button: 0, clientY: 1 })); };
      const listbox = dlg.querySelector('.manage-covers-list');
      key(listbox, 'a', { metaKey: true }); await sleep(100);
      dlg.querySelector('[data-manage="all"]').click(); await sleep(100);
      const after = [...dlg.querySelectorAll('.cover-presence')].map(p => p.getAttribute('aria-label')).join('|');
      if (after !== 'In all 2 files|In all 2 files') throw new Error('after apply to all ' + after);
      pick(rows()[1]); await sleep(100); dlg.querySelector('[data-manage="up"]').click(); await sleep(100);
      const order = rows().map(r => r.querySelector('strong').textContent).join('|');
      if (!order.startsWith('Front Cover')) throw new Error('order ' + order);
      dlg.querySelector('#manage-covers-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].manage-covers-dialog'), 10000, 'dialog closes (' + (dlg.querySelector('.dialog-error')?.textContent || '') + ')');
      await done(/Updated covers/, 'updated');
      const a = await covers('sine-a.mp3'), b = await covers('sine-b.mp3');
      const sig = c => c.map(x => x.picture_type + ':' + x.description).join(',');
      if (sig(a) !== 'CoverFront:,CoverBack:Rear sleeve' || sig(b) !== sig(a)) throw new Error(sig(a) + ' / ' + sig(b));
      return order + ' → ' + sig(a);
    });

    await step('shared covers display with navigation (→) and the larger viewer', async () => {
      await waitFor(() => /Shared by 2 files/.test(info()) && /1 \/ 2/.test(info()), 5000, 'shared covers info=' + info());
      art().focus(); key(art(), 'ArrowRight'); await waitFor(() => /2 \/ 2/.test(info()), 3000, 'next cover');
      key(art(), 'Enter'); const viewer = await waitFor(() => document.querySelector('dialog.cover-viewer[open] img'), 5000, 'viewer image');
      viewer.closest('dialog').querySelector('[data-viewer="previous"]').click(); await waitFor(() => /1 \/ 2/.test(document.querySelector('dialog.cover-viewer header').textContent), 3000, 'viewer follows navigation');
      const text = document.querySelector('dialog.cover-viewer').textContent.replace(/\s+/g, ' ').trim();
      document.querySelector('dialog.cover-viewer [data-viewer="close"]').click(); await sleep(100);
      if (document.querySelector('dialog.cover-viewer')) throw new Error('viewer did not close');
      return text.slice(0, 80);
    });

    await step('undo Manage covers restores both files', async () => {
      row('sine-b.mp3').focus(); key(row('sine-b.mp3'), 'z', { metaKey: true }); await done(/Undid Manage covers/, 'undo manage');
      const a = await covers('sine-a.mp3'), b = await covers('sine-b.mp3');
      if (a.length !== 1 || b.length !== 1 || a[0].picture_type !== 'CoverBack') throw new Error(JSON.stringify([a.length, b.length, a[0]?.picture_type]));
      row('sine-b.mp3').focus(); key(row('sine-b.mp3'), 'y', { metaKey: true }); await done(/Redid/, 'redo manage');
      return status();
    });

    let extracted = '';
    await step('Extract cover saves a format-string name next to the file', async () => {
      await clickRow('sine-a.mp3'); await waitFor(() => document.querySelector('.cover-art img[data-cover-image]'), 5000, 'image');
      await openMenu(); menuItem('Extract cover').click();
      const input = await waitFor(() => document.querySelector('dialog[open] #cover-extract-format'), 3000, 'extract dialog');
      input.value = '%album% front'; input.dispatchEvent(new Event('input', { bubbles: true }));
      await waitFor(() => /Test Album front\.png/.test(document.querySelector('#cover-extract-preview').textContent), 5000, 'resolved name');
      document.querySelector('#cover-extract-save').click();
      await waitFor(() => /Saved cover to/.test(status()), 5000, 'saved status (' + (document.querySelector('dialog[open] .dialog-error')?.textContent || '') + ')');
      extracted = status().replace('Saved cover to ', '');
      return extracted;
    });

    await step('dropping the extracted image on the cover window embeds it', async () => {
      if (!extracted) throw new Error('nothing extracted');
      await clickRow('sine-a.mp3'); const before = (await covers('sine-a.mp3')).length;
      const r = document.querySelector('.cover-section').getBoundingClientRect(), dpr = window.devicePixelRatio || 1;
      const position = { x: (r.left + r.width / 2) * dpr, y: (r.top + r.height / 2) * dpr };
      await inv('plugin:event|emit', { event: 'tauri://drag-enter', payload: { paths: [extracted], position } });
      await waitFor(() => document.querySelector('.drop-overlay.cover-drop'), 3000, 'cover drop overlay');
      const overlay = document.querySelector('.drop-overlay').textContent;
      await inv('plugin:event|emit', { event: 'tauri://drag-drop', payload: { paths: [extracted], position } });
      await done(/Added cover/, 'drop added');
      const c = await covers('sine-a.mp3'); if (c.length !== before + 1) throw new Error('covers ' + before + ' -> ' + c.length);
      return overlay + ' → ' + c.map(x => x.picture_type + ':' + x.description).join(',');
    });

    await step('Remove cover (Del on cover art) removes only the displayed cover', async () => {
      await clickRow('sine-a.mp3'); const before = await covers('sine-a.mp3');
      art().focus(); key(art(), 'ArrowRight'); await sleep(150); key(art(), 'Delete'); await done(/Removed cover/, 'removed');
      const after = await covers('sine-a.mp3');
      if (after.length !== before.length - 1 || after[0].digest !== before[0].digest) throw new Error(before.length + ' -> ' + after.length);
      return `${before.length} → ${after.length}`;
    });

    await step('cover menu follows Light and Dark appearance', async () => {
      const s = document.querySelector('#appearance'), colors = [];
      for (const mode of ['light', 'dark']) { s.value = mode; s.dispatchEvent(new Event('change', { bubbles: true })); await sleep(250); const m = await openMenu(); colors.push(getComputedStyle(m).backgroundColor + '/' + getComputedStyle(m).color); key(m, 'Escape'); }
      s.value = 'system'; s.dispatchEvent(new Event('change', { bubbles: true }));
      if (colors[0] === colors[1]) throw new Error('same colors ' + colors.join(' '));
      return colors.join(' vs ');
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
