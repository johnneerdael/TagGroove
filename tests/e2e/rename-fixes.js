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
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const names = async folder => (await inv('scan_files', { paths: [folder], recursive: true })).entries.map(e => e.path.slice(folder.length + 1).replace(/\\/g, '/')).sort();
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
    const album = row('sine-a.mp3').dataset.path.replace(/[\\/][^\\/]+$/, '');

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
      if (target.split(/[\\/]/).slice(-2).join('/') !== album.split(/[\\/]/).pop() + '/Sine_B.MP3') throw new Error('renamed to ' + target);
      await inv('undo_last');
      const back = (await names(album)).join('|');
      if (!back.includes('sine-b.mp3')) throw new Error('after undo: ' + back);
      return target.split(/[\\/]/).pop();
    });

    await step('edited filename actions use platform cleanup, Undo and collision protection', async () => {
      const path = row('sine-a.mp3').dataset.path, windows = /Win/.test(navigator.platform);
      const execute = action => inv('execute_action_groups', { paths: [path], groups: [{ name: 'Edited filename regression', enabled: true, actions: [action] }], columns: null });
      const files = async () => (await inv('scan_files', { paths: [album], recursive: false })).entries;
      const changed = [
        [{ type: 'replace', field: '_FILENAME', from: 'sine-a', to: windows ? 'AC/DC: Live?' : 'AC\\DC: Live?', whole_word: false, case_sensitive: true }, windows ? 'ACDC Live.mp3' : 'AC\\DC: Live?.mp3'],
        [{ type: 'regex_replace', field: '_FILENAME', pattern: '^sine-a[.]mp3$', replacement: 'CON.MP3', case_sensitive: true }, windows ? 'Reserved-Windows-Filename.MP3' : 'CON.MP3'],
      ];
      for (const [action, expected] of changed) {
        const result = await execute(action);
        if (result.errors.length) throw new Error(JSON.stringify(result.errors));
        const target = result.renamed?.[0]?.to, expectedPath = album + (windows ? '\\' : '/') + expected;
        if (target !== expectedPath) throw new Error(`expected ${expectedPath}, got ${target}`);
        const after = await files(), renamed = after.find(entry => entry.filename === expected);
        if (!renamed || renamed.fields.TITLE?.[0] !== 'Sine A' || after.some(entry => entry.filename === 'sine-a.mp3')) throw new Error('renamed file or metadata mismatch');
        const undo = await inv('undo_last');
        if (undo.errors?.length) throw new Error(JSON.stringify(undo.errors));
        const restored = await files();
        if (!restored.some(entry => entry.filename === 'sine-a.mp3') || restored.some(entry => entry.filename === expected)) throw new Error('Undo did not restore the original filename');
      }
      const clash = await execute({ type: 'replace', field: '_FILENAME', from: 'sine-a', to: windows ? 'sine-b?' : 'sine-b', whole_word: false, case_sensitive: true });
      if (!clash.errors.length) throw new Error('cleanup replaced the existing destination');
      const after = await files();
      if (after.find(entry => entry.filename === 'sine-a.mp3')?.fields.TITLE?.[0] !== 'Sine A' || after.find(entry => entry.filename === 'sine-b.mp3')?.fields.TITLE?.[0] !== 'Sine B') throw new Error('collision changed a source or destination');
      return windows ? 'Windows cleanup and reserved name, Undo and collision refusal' : 'Unix literal characters retained, Undo and collision refusal';
    });

    await step('literal inline rename keeps native paths, audio suffix, canonical identity and Undo', async () => {
      const path = row('sine-a.mp3').dataset.path, windows = /Win/.test(navigator.platform), sep = windows ? '\\' : '/';
      const files = async () => (await inv('scan_files', { paths: [album], recursive: true })).entries;
      const cases = [
        ["Inline album/Disc 1/O'Brien %title%.flac", "Inline album/Disc 1/O'Brien %title%.flac.mp3"],
        windows ? ['Inline album\\Disc 2\\Track.mp3', 'Inline album/Disc 2/Track.mp3'] : ['AC\\DC.mp3', 'AC\\DC.mp3'],
        ['Inline album/../Moved.MP3', 'Moved.MP3'],
      ];
      for (const [name, relative] of cases) {
        const result = await inv('rename_file_from_list', { path, name });
        if (result.errors.length) throw new Error(JSON.stringify(result.errors));
        const expected = album + sep + (windows ? relative.replace(/\//g, '\\') : relative);
        if (result.renamed?.[0]?.to !== expected || result.entries?.[0]?.path !== expected) throw new Error('inline receipt/readback identity mismatch');
        const renamed = (await files()).find(entry => entry.path === expected);
        if (renamed?.fields.TITLE?.[0] !== 'Sine A') throw new Error('inline name or metadata mismatch');
        const undo = await inv('undo_last');
        if (undo.errors?.length) throw new Error(JSON.stringify(undo.errors));
        const restored = await files();
        if (restored.find(entry => entry.path === path)?.fields.TITLE?.[0] !== 'Sine A' || restored.some(entry => entry.path === expected)) throw new Error('inline Undo did not restore the original');
      }
      const clash = await inv('rename_file_from_list', { path, name: 'sine-b.mp3' });
      if (!clash.errors.length || clash.error_diagnostics?.[0]?.diagnostic?.kind !== 'destination_collision') throw new Error('inline collision was not reported');
      const unchanged = await files();
      if (unchanged.find(entry => entry.path === path)?.fields.TITLE?.[0] !== 'Sine A' || unchanged.find(entry => entry.filename === 'sine-b.mp3')?.fields.TITLE?.[0] !== 'Sine B') throw new Error('inline collision changed audio metadata');
      return 'registered literal command; folders, suffix retention, native backslash, canonical receipt, Undo and collision';
    });

    const casefold = album.replace(/[\\/][^\\/]+$/, '') + (album.includes('\\') ? '\\' : '/') + 'casefold';
    let folded = [];
    try { folded = await names(casefold); } catch { /* No casefolded volume on this runner. */ }
    if (!folded.length) {
      await report('PASS case-insensitive volume steps skipped — no casefolded folder (local macOS/Windows volumes are covered by cargo tests)');
    } else {
      const song = (await inv('scan_files', { paths: [casefold], recursive: true })).entries.find(e => /[\\/]song\.mp3$/.test(e.path)).path;
      const folder = song.replace(/[\\/][^\\/]+$/, '');
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
        const parent = folder.replace(/[\\/][^\\/]+$/, '');
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
