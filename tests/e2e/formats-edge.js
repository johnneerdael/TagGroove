// Native e2e for Options → Tags → Advanced (single MP4 atom, reuse unmodified
// MP4 atoms and ID3v2 frames), AAC tagging under the Mpeg policy and Utils →
// Remove tag types on AAC and on WavPack with a prepended ID3v2 tag. Fixtures
// come from tests/e2e/formats-edge-fixtures.sh via OPENTAG_E2E_FIXTURES.
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const scan = async name => (await inv('scan_files', { paths: [row(name).dataset.path], recursive: false })).entries[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const ready = name => waitFor(() => op(name) && !op(name).disabled && op(name), 8000, name + ' enabled');
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const value = (fields, name) => Object.entries(fields || {}).find(([k]) => k.toUpperCase() === name)?.[1];
  const tags = entry => entry.technical._TAG?.[0] || '';
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const openOptions = async () => {
    for (let attempt = 0; attempt < 3; attempt++) {
      document.body.focus(); key('o', mod, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null);
      if (dlg) return dlg;
      await sleep(300);
    }
    throw new Error('options dialog did not open');
  };
  const removeTagTypes = async (name, kinds) => {
    await clickRow(name);
    (await ready('remove-tag-types')).click();
    const dlg = await waitFor(() => document.querySelector('dialog[open].remove-tag-types-dialog'), 4000, 'remove dialog');
    const enabled = [...dlg.querySelectorAll('[data-tag-kind]')].filter(i => !i.disabled).map(i => i.dataset.tagKind);
    for (const kind of kinds) {
      const input = dlg.querySelector(`[data-tag-kind="${kind}"]`);
      if (input.disabled) throw new Error(`${kind} is disabled; enabled: ${enabled.join(',')}`);
      input.click();
    }
    dlg.querySelector('#remove-tag-types').click();
    await waitFor(() => /Removed tag types/.test(status()), 8000, 'removed status'); await idle();
    return enabled;
  };

  try {
    await waitFor(() => row('stream.aac') && row('prefixed.wv'), 15000, 'fixture rows');
    await idle();

    await step('Options → Tags → Advanced lists the MP4 and ID3v2 write options', async () => {
      const dlg = await openOptions();
      dlg.querySelector('[data-page="tags-advanced"]').click(); await sleep(150);
      const box = path => dlg.querySelector(`[data-option="tags.advanced.${path}"]`);
      const labels = ['mp4_single_atom', 'reuse_mp4_atoms', 'reuse_id3v2_frames'].map(path => {
        const input = box(path);
        if (!input || input.type !== 'checkbox') throw new Error('no checkbox for ' + path);
        return `${input.closest('label')?.textContent.trim()}=${input.checked}`;
      });
      if (!box('mp4_single_atom').checked || box('reuse_mp4_atoms').checked || box('reuse_id3v2_frames').checked) throw new Error(labels.join(' | '));
      // Change all three, confirm, and check they are stored.
      for (const path of ['mp4_single_atom', 'reuse_mp4_atoms', 'reuse_id3v2_frames']) {
        const input = box(path); input.checked = !input.checked;
        input.dispatchEvent(new Event('input', { bubbles: true })); input.dispatchEvent(new Event('change', { bubbles: true }));
      }
      dlg.querySelector('#options-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed'); await idle();
      const advanced = (await inv('load_settings')).settings.tags.advanced;
      if (advanced.mp4_single_atom || !advanced.reuse_mp4_atoms || !advanced.reuse_id3v2_frames) throw new Error(JSON.stringify(advanced));
      // Back to the defaults for the rest of the run.
      const settings = (await inv('load_settings')).settings;
      settings.tags.advanced = { ...settings.tags.advanced, mp4_single_atom: true, reuse_mp4_atoms: false, reuse_id3v2_frames: false };
      await inv('save_settings', { settings });
      return labels.join(' | ');
    });

    await step('AAC shows its APEv2 tag and saves ID3v2 and ID3v1 under the Mpeg policy', async () => {
      const before = await scan('stream.aac');
      if (before.format !== 'AAC' || value(before.fields, 'TITLE')?.[0] !== 'From APE' || before.technical._TAG_READ?.[0] !== 'APEv2') throw new Error(`${before.format} ${JSON.stringify(before.fields)} ${tags(before)}`);
      await clickRow('stream.aac');
      const f = await waitFor(() => field('TITLE'), 4000, 'TITLE field');
      f.focus(); f.value = 'AAC edited'; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
      f.blur(); document.body.focus(); key('s', mod);
      await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status'); await idle();
      const after = await scan('stream.aac');
      if (value(after.fields, 'TITLE')?.[0] !== 'AAC edited' || value(after.fields, 'ARTIST')?.[0] !== 'Band') throw new Error(JSON.stringify(after.fields));
      if (!/ID3v2\.3/.test(tags(after)) || !/ID3v1/.test(tags(after)) || /APE/.test(tags(after))) throw new Error(tags(after));
      return `${tags(before)} → ${tags(after)}`;
    });

    await step('Remove tag types accepts AAC', async () => {
      const enabled = await removeTagTypes('stream.aac', ['id3v1', 'id3v2']);
      const after = await scan('stream.aac');
      if (tags(after) || after.format !== 'AAC') throw new Error(`${after.format} tags=${tags(after)}`);
      return `enabled ${enabled.join(',')} → no tags left`;
    });

    await step('WavPack lists its prepended ID3v2 tag and Remove tag types removes it', async () => {
      const before = await scan('prefixed.wv');
      if (before.format !== 'WavPack' || !/ID3v2/.test(tags(before)) || value(before.fields, 'TITLE')?.[0] !== 'From APE wv') throw new Error(`${before.format} ${tags(before)} ${JSON.stringify(before.fields)}`);
      await removeTagTypes('prefixed.wv', ['id3v2']);
      const after = await scan('prefixed.wv');
      if (/ID3v2/.test(tags(after)) || value(after.fields, 'TITLE')?.[0] !== 'From APE wv' || after.duration < 1.5) throw new Error(`${tags(after)} ${after.duration}`);
      return `${tags(before)} → ${tags(after)}`;
    });
  } catch (e) {
    await report(`FAIL setup — ${e && e.message || e}`);
  }
  await finish();
})();
