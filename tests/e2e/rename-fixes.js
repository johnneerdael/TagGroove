// Case-only renames and filename-mode targets through the real app (see
// docs/verification/rename-fixes.md). Linux CI volumes are case-sensitive;
// tests/e2e/rename-fixes-fixtures.sh adds a casefolded ext4 folder as
// ../casefold when the runner allows it, and those steps are skipped otherwise.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(100); } throw new Error('timeout waiting for ' + what); };
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const names = async folder => (await inv('scan_files', { paths: [folder], recursive: true })).entries.map(e => e.path.slice(folder.length + 1)).sort();
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); document.querySelectorAll('dialog[open]').forEach(d => d.close()); } };
  const tagToFilename = async (name, template) => {
    await clickRow(name); op('tags-filename').click();
    const dlg = await waitFor(() => document.querySelector('dialog[open] #template')?.closest('dialog'), 3000, 'Tag → Filename');
    const input = dlg.querySelector('#template'); input.value = template; input.dispatchEvent(new Event('input', { bubbles: true }));
    dlg.querySelector('#preview-button').click();
    const apply = await waitFor(() => !dlg.querySelector('#apply-converter').disabled && dlg.querySelector('#apply-converter'), 5000, 'preview');
    apply.click(); await waitFor(() => !dlg.open, 5000, 'rename'); await idle();
  };
  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    const album = row('sine-a.mp3').dataset.path.replace(/\/[^/]+$/, '');

    await step('Tag → Filename changes only the letter case and undo restores it', async () => {
      await tagToFilename('sine-a.mp3', '$upper(%_filename%)');
      await waitFor(() => row('SINE-A.mp3') && !row('sine-a.mp3'), 5000, 'renamed row');
      const renamed = (await names(album)).join('|');
      if (!renamed.includes('SINE-A.mp3') || renamed.includes('sine-a.mp3')) throw new Error('on disk: ' + renamed);
      document.body.focus(); key('z', mod); await idle();
      await waitFor(() => row('sine-a.mp3'), 5000, 'undone row');
      return renamed;
    });

    await step('Tag → Tag on _FILENAME uses filename mode without doubling the extension', async () => {
      const path = row('sine-b.mp3').dataset.path;
      // A separator produced from a value is replaced; the typed extension is kept once.
      const result = await inv('convert_tag_to_tag', { paths: [path], field: '_FILENAME', template: "$replace(%title%,' ','/').MP3", columns: null });
      if (result.errors.length) throw new Error(JSON.stringify(result.errors));
      const target = result.renamed?.[0]?.to || '';
      if (!target.endsWith(album.split('/').pop() + '/Sine_B.MP3')) throw new Error('renamed to ' + target);
      await inv('undo_last');
      const back = (await names(album)).join('|');
      if (!back.includes('sine-b.mp3')) throw new Error('after undo: ' + back);
      return target.split('/').pop();
    });

    const casefold = album.replace(/\/[^/]+$/, '') + '/casefold';
    let folded = [];
    try { folded = await names(casefold); } catch { /* No casefolded volume on this runner. */ }
    if (!folded.length) {
      await report('PASS case-insensitive volume steps skipped — no casefolded folder (local macOS/Windows volumes are covered by cargo tests)');
    } else {
      const song = (await inv('scan_files', { paths: [casefold], recursive: true })).entries.find(e => e.path.endsWith('/song.mp3')).path;
      const folder = song.replace(/\/[^/]+$/, '');
      await step('case-insensitive volume: case-only file rename, never replacing another file', async () => {
        const first = await inv('rename_files', { paths: [song], template: "'Song'", planToken: null });
        if (first.errors.length) throw new Error(JSON.stringify(first.errors));
        const renamed = first.renamed[0].to;
        const clash = await inv('rename_files', { paths: [renamed], template: "'other'", planToken: null });
        if (!clash.errors.length) throw new Error('replaced Other.mp3');
        const listed = (await names(folder)).join('|');
        if (listed !== 'Other.mp3|Song.mp3') throw new Error('listed ' + listed);
        return listed;
      });
      await step('case-insensitive volume: Case Conversion renames the folder and undo restores it', async () => {
        const file = (await inv('scan_files', { paths: [folder], recursive: false })).entries.find(e => /song\.mp3$/i.test(e.path)).path;
        const result = await inv('apply_action', { paths: [file], action: { type: 'case', field: '_DIRECTORY', mode: 'upper' } });
        if (result.errors.length) throw new Error(JSON.stringify(result.errors));
        const parent = folder.replace(/\/[^/]+$/, '');
        const after = (await names(parent)).join('|');
        if (!after.startsWith('ALBUM/')) throw new Error('after ' + after);
        await inv('undo_last');
        const undone = (await names(parent)).join('|');
        if (!undone.startsWith('album/')) throw new Error('after undo ' + undone);
        return `${after} → ${undone}`;
      });
    }
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
