(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const status = () => document.querySelector('.status-message')?.textContent || '';
  // Diagnostics for timeouts: a timeline of window state changes (busy, status,
  // open dialogs, pending edits, selection, focus) since the script started.
  const t0 = performance.now(), ms = () => Math.round(performance.now() - t0);
  const describe = el => !el ? '-' : el.tagName + (el.id ? '#' + el.id : '') + (el.dataset?.field ? ':' + el.dataset.field : '') + (el.className && typeof el.className === 'string' ? '.' + el.className.trim().split(/\s+/).join('.') : '') + (el.getAttribute?.('aria-label') ? `[${el.getAttribute('aria-label')}]` : '') + (el.isConnected === false ? '(detached)' : '') + (el.parentElement ? ' in ' + (el.parentElement.className || el.parentElement.tagName) : '');
  const snapshot = () => {
    const opsState = ['save', 'undo', 'redo'].map(op => { const b = document.querySelector(`button[data-op="${op}"]`); return `${op}:${b ? (b.disabled ? 'off' : 'on') : '-'}`; }).join(',');
    const dialogs = [...document.querySelectorAll('dialog[open]')].map(d => d.getAttribute('aria-label') || d.className || d.querySelector('header')?.textContent || 'dialog').join('|');
    const sel = [...document.querySelectorAll('tr[data-path].selected')].map(r => r.dataset.path.split(/[\\/]/).pop()).join('+');
    const focus = describe(document.activeElement);
    return `busy=${!!document.querySelector('footer.status.busy')} status="${status()}" dialogs=[${dialogs}] dirty="${document.querySelector('.dirty-count')?.textContent || ''}" rows=${document.querySelectorAll('tr[data-path]').length} sel=${sel} ${opsState} focus=${focus}`;
  };
  const timeline = []; let lastSnap = '';
  const watch = setInterval(() => { const s = snapshot(); if (s !== lastSnap) { lastSnap = s; timeline.push(`${ms()}ms ${s}`); if (timeline.length > 80) timeline.shift(); } }, 20);
  const waitFor = async (fn, wait = 8000, what = 'condition') => { const t = Date.now(); while (Date.now() - t < wait) { try { const v = fn(); if (v) return v; } catch {} await sleep(100); } throw new Error(`timeout waiting for ${what} at ${ms()}ms; ${snapshot()}`); };
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const row = name => [...document.querySelectorAll('tr[data-path]')].find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const read = async name => { const p = row(name)?.dataset.path; const b = await inv('scan_files', { paths: [p], recursive: false }); return b.entries[0]?.fields || {}; };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const undoReady = kind => waitFor(() => { const b = document.querySelector(`button[data-op="${kind}"]`); return b && !b.disabled; }, 8000, kind + ' available');
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); for (const line of timeline.splice(0)) await report('  state ' + line); } };
  try {
    await waitFor(() => document.querySelectorAll('tr[data-path]').length >= 2, 15000, 'fixture rows');
    await report(`loaded at ${ms()}ms; ${snapshot()}`);
    await report(`loaded rows=${document.querySelectorAll('tr[data-path]').length} headers=${[...document.querySelectorAll('thead th')].map(t => t.textContent.trim()).join('|')}`);
    await report(`menus=${[...document.querySelectorAll('.menubar > .menu > summary')].map(s => s.textContent).join('|')}`);

    await step('select row updates tag panel', async () => { await clickRow('sine-a.mp3'); const v = field('TITLE')?.value; if (v !== 'Sine A') throw new Error('TITLE field=' + v); return 'TITLE=' + v; });

    await step('clicking into the tag panel cancels a pending slow-click edit', async () => {
      // sine-a is the only selected file, so this click arms the delayed File List editor.
      await clickRow('sine-a.mp3'); const f = field('TITLE'); f.focus(); await sleep(800);
      if (document.querySelector('.cell-editor')) throw new Error('cell editor opened; focus=' + document.activeElement?.className);
      if (document.activeElement !== field('TITLE')) throw new Error('focus left the tag panel');
      return 'focus stays in TITLE';
    });

    await step('edit title + Cmd+S writes file', async () => {
      const f = field('TITLE'); f.focus(); f.value = 'Sine A Edited'; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
      if (!row('sine-a.mp3')?.querySelector('.pending,.rowmarker') && !document.querySelector('.dirty-count')) throw new Error('no pending marker');
      f.blur(); document.body.focus(); key('s', { metaKey: true }); await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status');
      const t = (await read('sine-a.mp3')).TITLE; if (t?.[0] !== 'Sine A Edited') throw new Error('disk TITLE=' + t); return status();
    });

    await step('Cmd+Z undoes the save on disk', async () => {
      await idle(); await undoReady('undo'); row('sine-a.mp3').focus(); await report('focus=' + (document.activeElement?.tagName) + ' ' + (document.activeElement?.dataset?.field || '')); key('z', { metaKey: true }); await waitFor(() => /Undid/.test(status()), 8000, 'undo status');
      const t = (await read('sine-a.mp3')).TITLE; if (t?.[0] !== 'Sine A') throw new Error('disk TITLE=' + t); return status();
    });

    await step('Cmd+Y redoes', async () => {
      await idle(); await undoReady('redo'); row('sine-a.mp3').focus(); key('y', { metaKey: true }); await waitFor(() => /Redid/.test(status()), 8000, 'redo status');
      const t = (await read('sine-a.mp3')).TITLE; if (t?.[0] !== 'Sine A Edited') throw new Error('disk TITLE=' + t); return status();
    });

    await step('copy tags A -> paste onto B stages drafts, save writes', async () => {
      await clickRow('sine-a.mp3'); key('c', { metaKey: true }); await sleep(300);
      await clickRow('sine-b.mp3'); key('v', { metaKey: true }); await sleep(400);
      const v = field('TITLE')?.value; if (v !== 'Sine A Edited') throw new Error('panel after paste=' + v + ' status=' + status());
      key('s', { metaKey: true }); await waitFor(() => /Saved tags/.test(status()), 8000, 'saved');
      const f = await read('sine-b.mp3'); if (f.TITLE?.[0] !== 'Sine A Edited') throw new Error('disk B TITLE=' + f.TITLE); return 'B track=' + f.TRACK;
    });
    await step('undo paste', async () => { await idle(); await undoReady('undo'); key('z', { metaKey: true }); await waitFor(() => /Undid/.test(status()), 8000, 'undo'); const t = (await read('sine-b.mp3')).TITLE; if (t?.[0] !== 'Sine B') throw new Error('disk B TITLE=' + t); return status(); });

    await step('Convert Tag->Tag (Alt+5) writes composed value', async () => {
      await clickRow('sine-b.mp3'); key('5', { altKey: true, code: 'Digit5' });
      const dlg = await waitFor(() => document.querySelector('dialog[open] #tag-tag-template') && document.querySelector('dialog[open]'), 5000, 'tag-tag dialog');
      const fld = dlg.querySelector('#tag-tag-field'); fld.value = 'COMMENT'; fld.dispatchEvent(new Event('input', { bubbles: true }));
      const tpl = dlg.querySelector('#tag-tag-template'); tpl.value = '%artist% / %title%'; tpl.dispatchEvent(new Event('input', { bubbles: true }));
      await sleep(1200); const preview = dlg.querySelector('#tag-tag-preview')?.textContent.trim().slice(0, 80);
      const apply = dlg.querySelector('#tag-tag-apply'); await waitFor(() => !apply.disabled, 5000, 'apply enabled'); apply.click();
      await waitFor(() => !document.querySelector('dialog[open]'), 8000, 'dialog close'); await sleep(500);
      const c = (await read('sine-b.mp3')).COMMENT; if (!String(c).includes('Test Artist / Sine B')) throw new Error('disk COMMENT=' + JSON.stringify(c) + ' preview=' + preview); return 'COMMENT=' + c;
    });

    await step('Filename -> Tag (Alt+2) skips a file that does not match and writes the rest', async () => {
      await clickRow('sine-a.mp3'); document.body.focus(); key('a', { metaKey: true }); await sleep(300);
      key('2', { altKey: true, code: 'Digit2' });
      const dlg = await waitFor(() => document.querySelector('dialog[open] #template')?.closest('dialog'), 5000, 'Filename -> Tag dialog');
      const input = dlg.querySelector('#template'); input.value = '%composer%-a'; input.dispatchEvent(new Event('input', { bubbles: true }));
      dlg.querySelector('#preview-button').click();
      const apply = await waitFor(() => !dlg.querySelector('#apply-converter').disabled && dlg.querySelector('#apply-converter'), 5000, 'preview');
      const rowsShown = dlg.querySelectorAll('.preview > div').length, skipped = dlg.querySelectorAll('.preview > .no-match').length;
      if (rowsShown !== 2 || skipped !== 1 || !/COMPOSER: sine/.test(dlg.querySelector('.preview').textContent) || apply.textContent !== 'Apply to 1 file') throw new Error(`preview rows=${rowsShown} skipped=${skipped} label=${apply.textContent} text=${dlg.querySelector('.preview').textContent}`);
      apply.click(); await waitFor(() => /Converted filenames to tags/.test(status()) && !document.querySelector('footer.status.busy'), 8000, 'converted');
      if (!/: 1 file, 1 didn't match/.test(status()) || document.querySelector('.errors')) throw new Error('status=' + status() + ' errors=' + document.querySelector('.errors')?.textContent);
      const a = await read('sine-a.mp3'), b = await read('sine-b.mp3');
      if (a.COMPOSER?.[0] !== 'sine' || b.COMPOSER) throw new Error(`disk A COMPOSER=${a.COMPOSER} B COMPOSER=${b.COMPOSER}`);
      const done = status();
      await undoReady('undo'); document.body.focus(); key('z', { metaKey: true }); await waitFor(() => /Undid/.test(status()), 8000, 'undo');
      if ((await read('sine-a.mp3')).COMPOSER) throw new Error('undo left COMPOSER on A');
      return done;
    });

    await step('Options dialog opens with Cmd+O and lists pages', async () => {
      document.body.focus(); key('o', { metaKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog[open] #options-ok') && document.querySelector('dialog[open]'), 5000, 'options dialog');
      const pages = [...dlg.querySelectorAll('[role=treeitem]')].map(e => e.textContent.trim()).join('|');
      dlg.querySelector('#options-cancel').click(); await sleep(300); return pages;
    });

    await step('Find regex uses Boost through the native command and reports invalid patterns', async () => {
      document.body.focus(); key('f', { metaKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog.find-dialog[open]'), 5000, 'Find dialog');
      try {
        const criteria = dlg.querySelector('#find-criteria'), strategy = dlg.querySelector('#find-strategy');
        strategy.value = 'regex'; strategy.dispatchEvent(new Event('change', { bubbles: true }));
        criteria.value = String.raw`(?i)\Asine-[ab]\.mp3\z`; criteria.dispatchEvent(new Event('input', { bubbles: true }));
        dlg.querySelector('#find-run').click();
        await waitFor(() => /2 matches in 2 files/.test(dlg.querySelector('#find-status').textContent), 8000, 'Boost Find filename results');
        const matches = [...dlg.querySelectorAll('tbody tr')].map(row => row.children[0].textContent).sort();
        if (matches.join('|') !== 'sine-a.mp3|sine-b.mp3') throw new Error('Find results: ' + matches);
        criteria.value = '('; criteria.dispatchEvent(new Event('input', { bubbles: true }));
        if (dlg.querySelectorAll('tbody tr').length) throw new Error('old results survived an edit');
        dlg.querySelector('#find-run').click();
        await waitFor(() => /Invalid regular expression:/.test(dlg.querySelector('#find-status').textContent), 8000, 'native syntax error');
        if (dlg.querySelectorAll('tbody tr').length || !dlg.querySelector('#find-select').disabled) throw new Error('failed Find exposed partial results');
        return 'two filename matches; edited criteria clear results; invalid pattern reports an error';
      } finally { dlg.close(); }
    });

    await step('Remove from list (Del) and Refresh (F5)', async () => {
      await clickRow('sine-b.mp3'); const before = document.querySelectorAll('tr[data-path]').length;
      row('sine-b.mp3').focus(); key('Delete'); await sleep(600);
      const after = document.querySelectorAll('tr[data-path]').length; if (after !== before - 1) throw new Error(`rows ${before} -> ${after}`);
      key('F5'); await waitFor(() => document.querySelectorAll('tr[data-path]').length === before, 10000, 'refresh restore'); return `${before} -> ${after} -> ${before}`;
    });

    await step('Tag Sources MusicBrainz dialog opens prefilled', async () => {
      await clickRow('sine-a.mp3'); const btn = document.querySelector('button[data-op="source-musicbrainz"]'); btn.click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].tag-source-dialog') || document.querySelector('dialog[open]'), 5000, 'source dialog');
      const vals = [...dlg.querySelectorAll('input')].map(i => i.value).filter(Boolean).join('|'); dlg.close(); return vals;
    });

    await step('View > Customize columns dialog opens', async () => { const b = document.querySelector('button[data-op="columns"]'); b.click(); const dlg = await waitFor(() => document.querySelector('dialog[open]'), 5000, 'columns dialog'); const n = dlg.querySelectorAll('li,option,[role=option],tr').length; dlg.close(); return 'items=' + n; });

    await step('Light/Dark appearance switch', async () => { const s = document.querySelector('#appearance'); s.value = 'light'; s.dispatchEvent(new Event('change', { bubbles: true })); await sleep(300); const l = document.documentElement.dataset.theme; s.value = 'system'; s.dispatchEvent(new Event('change', { bubbles: true })); return 'light->' + l; });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  clearInterval(watch);
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
