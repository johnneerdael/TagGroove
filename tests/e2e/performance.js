// Interaction timings for a real folder. Run through scripts/e2e-perf.sh, which
// launches the app with `/fp:<folder>` and prepends `window.__OPENTAG_PERF__`
// ({launch: epoch ms of the launch, write: true only for a disposable copy}).
// The app records IPC and render timings into `window.__OPENTAG_TIMINGS__`
// while that array exists (see `timing()` in src/main.ts).
(async () => {
  const config = window.__OPENTAG_PERF__ || {};
  const timings = window.__OPENTAG_TIMINGS__ = window.__OPENTAG_TIMINGS__ || [];
  const rawInvoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const calls = { filter: (fn) => timings.filter(t => t.name.startsWith('ipc ')).map(t => ({ cmd: t.name.slice(4), started: t.at, ms: t.ms, in: t.in || 0, out: t.out || 0, pending: t.pending })).filter(fn), some: fn => calls.filter(() => true).some(fn) };
  const pendingOf = cmd => timings.some(t => t.name === 'ipc ' + cmd && t.pending);
  const pending = new Proxy({}, { get: (_, cmd) => pendingOf(cmd) });
  const inv = (cmd, args) => rawInvoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  // Next painted frame; falls back after 1 s if frames are suspended (window hidden).
  let suspended = 0;
  const frame = () => new Promise(r => { let done = false; const finish = late => { if (!done) { done = true; if (late) suspended++; r(); } }; requestAnimationFrame(() => setTimeout(() => finish(false), 0)); setTimeout(() => finish(true), 1000); });
  // Forces style and layout, so sync timings include them (not paint).
  const layout = () => document.querySelector('.app')?.getBoundingClientRect();
  // Re-checks on every DOM mutation as well as on a timer: timers are throttled while the window is hidden.
  const waitFor = (fn, ms = 30000, what = 'condition') => new Promise((resolve, reject) => {
    const started = Date.now(); let done = false;
    const finish = (ok, value) => { done = true; observer.disconnect(); clearInterval(timer); ok ? resolve(value) : reject(new Error('timeout waiting for ' + what)); };
    const check = () => { if (done) return; try { const value = fn(); if (value) return finish(true, value); } catch {} if (Date.now() - started > ms) finish(false); };
    const observer = new MutationObserver(check); observer.observe(document, { subtree: true, childList: true, characterData: true, attributes: true });
    const timer = setInterval(check, 5); check();
  });
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const busy = () => !!document.querySelector('footer.status.busy');
  const idle = (ms = 600000) => waitFor(() => !busy(), ms, 'idle');
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => document.querySelectorAll('tr[data-path]');
  const panel = name => document.querySelector(`.fields [data-field="${name}"]`);
  const round = ms => Math.round(ms * 10) / 10;
  const stats = list => { const s = [...list].sort((a, b) => a - b); return `n=${s.length} median=${round(s[Math.floor(s.length / 2)])} p95=${round(s[Math.min(s.length - 1, Math.floor(s.length * 0.95))])} max=${round(s[s.length - 1])}`; };
  const perf = (name, detail) => report(`PERF ${name} ${detail}`);
  const ipc = (cmd, since) => { const list = calls.filter(c => c.cmd === cmd && c.started >= since); return list.length ? `${cmd}: calls=${list.length} total_ms=${round(list.reduce((s, c) => s + c.ms, 0))} in_bytes=${list.reduce((s, c) => s + c.in, 0)} out_bytes=${list.reduce((s, c) => s + c.out, 0)}` : `${cmd}: none`; };
  const ipcSince = since => [...new Set(calls.filter(c => c.started >= since && c.cmd !== 'e2e_report').map(c => c.cmd))].map(cmd => ipc(cmd, since)).join('; ');
  // Time an interaction: `sync` covers handlers, style and layout; `frame` the next painted frame.
  const measure = async act => { const t = performance.now(); act(); layout(); const sync = performance.now() - t; await frame(); return { sync, frame: performance.now() - t }; };
  const both = (name, list) => perf(name, `sync ${stats(list.map(m => m.sync))} | frame ${stats(list.map(m => m.frame))}`);
  const settled = async () => { await waitFor(() => !pending.evaluate_columns && !pending.filter_matches && !busy(), 120000, 'settled'); await frame(); };
  try {
    // 1. Launch → rows visible → all column values evaluated.
    await perf('visibility', document.visibilityState);
    const total = await waitFor(() => { const n = Number(document.querySelector('.location strong')?.textContent.split(' ')[0]); return n > 0 && rows().length && !busy() && n; }, 300000, 'rows');
    const visibleAt = Date.now(), visiblePerf = performance.now();
    await perf('launch_to_rows', `ms=${config.launch ? visibleAt - config.launch : 'n/a'} page_ms=${round(visiblePerf)} files=${total} rendered_rows=${rows().length}`);
    await waitFor(() => calls.some(c => c.cmd === 'evaluate_columns') && !pending.evaluate_columns, 60000, 'column values').catch(() => {});
    await frame();
    await perf('launch_to_column_values', `ms=${config.launch ? Date.now() - config.launch : 'n/a'} page_ms=${round(performance.now())} ${ipcSince(0)}`);

    // 2. Selection change → tag panel populated (single click, then arrowing).
    const list = [...rows()];
    const clicks = [];
    for (let i = 1; i < Math.min(11, list.length); i++) {
      const target = rows()[i], path = target.dataset.path;
      clicks.push(await measure(() => target.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }))));
      if (!document.querySelector(`tr[data-path="${CSS.escape(path)}"].selected`)) throw new Error('click did not select');
      await sleep(30);
    }
    await both('select_click', clicks);
    const arrows = [], covers = [];
    rows()[0].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await frame();
    for (let i = 0; i < 30; i++) {
      const from = document.activeElement?.dataset?.path, t = performance.now();
      arrows.push(await measure(() => key('ArrowDown')));
      const now = document.querySelector('tr.selected')?.dataset.path; if (!now || now === from) throw new Error('arrow did not move');
      const loaded = await waitFor(() => document.querySelector('.cover-art img')?.src.startsWith('data:') || document.querySelector('.cover-info')?.textContent.includes('No artwork'), 5000, 'cover').catch(() => false);
      if (loaded) covers.push(performance.now() - t);
    }
    await both('select_arrow', arrows);
    for (const name of ['workspace render', 'selection refresh', 'list refresh', 'table window', 'table selection', 'sort', 'filter']) { const list = timings.filter(t => t.name === name).map(t => t.ms); if (list.length) await perf(`phase ${name.replace(/ /g, '_')}`, stats(list)); }
    await perf('cover_hydrate_after_arrow', `${stats(covers)} ${ipc('get_covers', 0)}`);

    // 3. Sort by a column (header click), then restore filename order.
    for (const column of ['title', 'artist', 'length', 'filename']) {
      const button = document.querySelector(`[data-sort="${column}"]`); if (!button) continue;
      const first = rows()[0]?.dataset.path, since = performance.now();
      const m = await measure(() => button.click());
      await settled();
      await perf(`sort_${column}`, `sync_ms=${round(m.sync)} first_frame_ms=${round(m.frame)} settled_ms=${round(performance.now() - since)} changed=${rows()[0]?.dataset.path !== first} ${ipcSince(since)}`);
    }

    // 4. Filter typing: one character at a time, then clear.
    const typed = [], word = 'remix';
    let input = document.querySelector('#filter'); input.focus();
    const filterStart = performance.now();
    for (let i = 1; i <= word.length; i++) {
      input = document.querySelector('#filter');
      typed.push(await measure(() => { input.value = word.slice(0, i); input.dispatchEvent(new Event('input', { bubbles: true })); }));
      await sleep(60);
    }
    await settled(); await waitFor(() => !document.querySelector('.filterbar .count')?.textContent.includes('Filtering'), 60000, 'filter result');
    await both('filter_keystroke', typed);
    await perf('filter_type_to_result', `ms=${round(performance.now() - filterStart)} visible=${rows().length} count="${document.querySelector('.filterbar .count')?.textContent}" ${ipcSince(filterStart)}`);
    // A second query over the same files (the backend already holds their filter fields).
    input = document.querySelector('#filter'); const requeryStart = performance.now();
    input.value = 'mix'; input.dispatchEvent(new Event('input', { bubbles: true }));
    await sleep(150); await settled(); await waitFor(() => !document.querySelector('.filterbar .count')?.textContent.includes('Filtering'), 60000, 'requery result');
    await perf('filter_requery', `ms=${round(performance.now() - requeryStart)} visible=${rows().length} ${ipcSince(requeryStart)}`);
    input = document.querySelector('#filter'); input.value = ''; const clearStart = performance.now(); input.dispatchEvent(new Event('input', { bubbles: true })); await settled();
    await perf('filter_clear', `ms=${round(performance.now() - clearStart)}`);
    input.blur?.();

    // 5. Select all + tag panel with every file.
    rows()[0].focus(); let since = performance.now();
    const all = await measure(() => key('a', { metaKey: true }));
    await perf('select_all', `sync_ms=${round(all.sync)} frame_ms=${round(all.frame)} selected="${document.querySelector('.filterbar .count')?.textContent}" title_mixed=${panel('TITLE')?.value === ''}`);
    const single = await measure(() => key('ArrowDown'));
    await perf('select_all_then_single', `sync_ms=${round(single.sync)} frame_ms=${round(single.frame)}`);

    // 6. Scroll the virtual table from top to bottom.
    const wrap = document.querySelector('.table-wrap'), scrolls = [];
    for (let top = 0; top < wrap.scrollHeight; top += wrap.clientHeight) { const t = performance.now(); wrap.scrollTop = top; wrap.dispatchEvent(new Event('scroll')); await frame(); scrolls.push(performance.now() - t); }
    await perf('scroll_page', stats(scrolls));
    wrap.scrollTop = 0; await frame();

    if (config.write) {
      // 7. Edit one field across every file, save, then undo it.
      rows()[0].focus(); key('a', { metaKey: true }); await frame();
      const value = `TagGroove perf ${Date.now()}`, comment = panel('COMMENT');
      comment.focus();
      const edit = await measure(() => { comment.value = value; comment.dispatchEvent(new Event('input', { bubbles: true })); });
      await perf('edit_field_all', `sync_ms=${round(edit.sync)} frame_ms=${round(edit.frame)} pending="${document.querySelector('.dirty-count')?.textContent}"`);
      comment.blur(); rows()[0].focus();
      since = performance.now(); let progressSeen = 0;
      key('s', { metaKey: true });
      await waitFor(() => { if (/\d+ of \d+/.test(document.querySelector('.job-progress-text')?.textContent || '')) progressSeen++; return /Saved tags/.test(status()) && !busy(); }, 1800000, 'saved');
      const savedAll = performance.now() - since; await frame();
      await perf('save_all', `ms=${round(savedAll)} status="${status()}" progress_updates=${progressSeen} ${ipcSince(since)}`);
      const sample = rows()[rows().length - 1].dataset.path, disk = await inv('scan_files', { paths: [sample], recursive: false });
      await perf('save_all_verify', `comment_ok=${disk.entries[0]?.fields.COMMENT?.[0] === value}`);
      await waitFor(() => !document.querySelector('button[data-op="undo"]')?.disabled, 30000, 'undo available').catch(() => {});
      const undo = document.querySelector('button[data-op="undo"]');
      if (undo && !undo.disabled) {
        rows()[0].focus(); since = performance.now(); key('z', { metaKey: true });
        await waitFor(() => /Undid|cannot/.test(status()) && !busy(), 1800000, 'undo');
        const after = await inv('scan_files', { paths: [sample], recursive: false });
        await perf('undo_save_all', `ms=${round(performance.now() - since)} status="${status()}" restored=${after.entries[0]?.fields.COMMENT?.[0] !== value} ${ipcSince(since)}`);
      } else await perf('undo_save_all', `unavailable title="${undo?.title}" status="${status()}"`);

      // 8. Save one file.
      rows()[1].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await frame();
      const single = panel('COMMENT'); single.focus(); single.value = 'TagGroove single'; single.dispatchEvent(new Event('input', { bubbles: true })); single.blur(); rows()[1].focus();
      since = performance.now(); key('s', { metaKey: true });
      await waitFor(() => /Saved tags/.test(status()) && !busy(), 60000, 'saved one'); const saved = performance.now() - since; await frame();
      await perf('save_one', `ms=${round(saved)} frame_ms=${round(performance.now() - since)} ${ipcSince(since)}`);

      // 9. Cancel a multi-file save after the first progress update.
      rows()[0].focus(); key('a', { metaKey: true }); await frame();
      const again = panel('COMMENT'); again.focus(); again.value = 'TagGroove cancelled'; again.dispatchEvent(new Event('input', { bubbles: true })); again.blur(); rows()[0].focus();
      since = performance.now(); key('s', { metaKey: true });
      const cancel = await waitFor(() => /^[1-9]\d* of /.test(document.querySelector('.job-progress-text')?.textContent || '') && document.querySelector('#cancel-job'), 60000, 'progress').catch(() => null);
      const clicked = performance.now(); cancel?.click();
      await waitFor(() => /Saved tags/.test(status()) && !busy(), 600000, 'cancelled save');
      await perf('save_cancel', `cancel_found=${!!cancel} stop_ms=${round(performance.now() - clicked)} status="${status()}" pending="${document.querySelector('.dirty-count')?.textContent || 'none'}"`);
    }
    await report(`SUMMARY done visibility=${document.visibilityState} suspended_frames=${suspended}`);
  } catch (e) {
    await report('FATAL ' + (e && e.stack || e));
  }
  await inv('e2e_exit', { code: 0 });
})();
