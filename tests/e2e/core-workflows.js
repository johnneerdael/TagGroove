(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(100); } throw new Error('timeout waiting for ' + what); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const row = name => document.querySelector(`tr[data-path$="/${CSS.escape(name)}"]`) || [...document.querySelectorAll('tr[data-path]')].find(r => r.dataset.path.endsWith('/' + name));
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const read = async name => { const p = row(name)?.dataset.path; const b = await inv('scan_files', { paths: [p], recursive: false }); return b.entries[0]?.fields || {}; };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const undoReady = kind => waitFor(() => { const b = document.querySelector(`button[data-op="${kind}"]`); return b && !b.disabled; }, 8000, kind + ' available');
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  try {
    await waitFor(() => document.querySelectorAll('tr[data-path]').length >= 2, 15000, 'fixture rows');
    await report(`loaded rows=${document.querySelectorAll('tr[data-path]').length} headers=${[...document.querySelectorAll('thead th')].map(t => t.textContent.trim()).join('|')}`);
    await report(`menus=${[...document.querySelectorAll('.menubar > .menu > summary')].map(s => s.textContent).join('|')}`);

    await step('select row updates tag panel', async () => { await clickRow('sine-a.mp3'); const v = field('TITLE')?.value; if (v !== 'Sine A') throw new Error('TITLE field=' + v); return 'TITLE=' + v; });

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

    await step('Options dialog opens with Cmd+O and lists pages', async () => {
      document.body.focus(); key('o', { metaKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog[open] #options-ok') && document.querySelector('dialog[open]'), 5000, 'options dialog');
      const pages = [...dlg.querySelectorAll('[role=treeitem]')].map(e => e.textContent.trim()).join('|');
      dlg.querySelector('#options-cancel').click(); await sleep(300); return pages;
    });

    await step('Find (Cmd+F) opens', async () => { key('f', { metaKey: true }); await sleep(600); const d = [...document.querySelectorAll('dialog[open], .find-window, [role=dialog]')].map(e => e.getAttribute('aria-label') || e.className).join('|'); if (!d) throw new Error('no find dialog'); document.querySelectorAll('dialog[open]').forEach(x => x.close()); key('Escape'); return d; });

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
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
