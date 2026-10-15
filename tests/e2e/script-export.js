// Native e2e for scripting and export parity: working-folder and measured
// placeholders, $regexp case handling, ANSI exports with %_md5audio%, .mte
// files dropped into the export folder, .mta round trips of every action type
// and Convert → Text File → Tag with a format-string source.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); document.querySelectorAll('dialog[open]').forEach(d => d.close()); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const preview = async (path, template) => (await inv('preview_format', { paths: [path], template, columns: null }))[0].value;
  // Write a text file through File → Export: the export renders `text` once.
  const write = async (target, text) => {
    const library = await inv('load_export_library');
    library.configurations = library.configurations.filter(c => c.id !== 'e2e-writer');
    library.configurations.push({ id: 'e2e-writer', name: 'E2E writer', template: `$filename(txt,utf-8)'${text.replace(/'/g, "''")}'` });
    library.preferences = { ...library.preferences, configuration: 'e2e-writer', filename: target, append: false, partition: null };
    await inv('save_export_library', { library });
    const result = await inv('run_export', { paths: [rows()[0].dataset.path], columns: null, perDirectory: false });
    if (result.errors?.length) throw new Error(JSON.stringify(result.errors));
  };

  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    const a = row('sine-a.mp3').dataset.path, b = row('sine-b.mp3').dataset.path;
    const album = a.replace(/[\\/][^\\/]+$/, '');

    await step('%_workingdir% and %_workingpath% name the folder the list was opened from', async () => {
      // The window reports the folder right after loading it, spelled as it was
      // opened (macOS lists /var/folders files under /private/var/folders).
      const sep = album.includes('\\') ? '\\' : '/', opened = document.querySelector('.location .path')?.textContent || album;
      const value = await waitFor(async () => { const v = await preview(a, '%_workingdir%|%_workingpath%'); return [`album|${album}${sep}`, `album|${opened}${sep}`].includes(v) && v; }, 5000, 'working folder')
        .catch(async e => { throw new Error(`${e.message}: ${await preview(a, '%_workingdir%|%_workingpath%')}`); });
      return value;
    });

    await step('MP3 bitrate type, encoder and tag sizes are measured', async () => {
      const value = await preview(a, '%_vbr%|%_tag_size_prepended%|%_tag_size%|[%_tool%]|%_length%');
      const [vbr, prepended, total] = value.split('|');
      if (vbr !== 'CBR' || !(Number(prepended) > 10) || Number(total) < Number(prepended)) throw new Error(value);
      return value;
    });

    await step('$regexp ignores case unless its fourth argument is 0', async () => {
      const value = await preview(a, "$regexp(%title%,sine,X)|$regexp(%title%,sine,X,0)|$regexp(%title%,'(?-i)sine',X)");
      if (value !== 'X A|Sine A|Sine A') throw new Error(value);
      return value;
    });

    await step('ANSI export with %_md5audio% writes Windows-1252 and audio-only hashes', async () => {
      const library = await inv('load_export_library');
      library.configurations = library.configurations.filter(c => c.id !== 'e2e-hashes');
      library.configurations.push({ id: 'e2e-hashes', name: 'E2E hashes', template: '$filename(txt,ansi)$loop(%_filename%)%_filename%:%_md5audio%:%_md5%$char(13)$char(10)$loopend()Café' });
      library.preferences = { ...library.preferences, configuration: 'e2e-hashes', filename: album + '/hashes.txt', append: false, partition: null };
      await inv('save_export_library', { library });
      const result = await inv('run_export', { paths: [a, b], columns: null, perDirectory: false });
      if (result.errors?.length) throw new Error(JSON.stringify(result.errors));
      const text = (await inv('import_export_configuration', { path: album + '/hashes.txt', legacyEncoding: 'windows-1252' })).template;
      const lines = text.split('\r\n');
      for (const line of lines.slice(0, 2)) {
        const [, audio, file] = line.split(':');
        if (!/^[0-9a-f]{32}$/.test(audio) || audio === file) throw new Error(JSON.stringify(text));
      }
      if (lines[2] !== 'Café') throw new Error(JSON.stringify(text));
      return lines.slice(0, 2).join(' | ');
    });

    await step('an .mte file dropped into the export folder is listed under its name', async () => {
      const config = await inv('plugin:path|resolve_directory', { directory: 13 });
      await write(`${config.replace(/\/$/, '')}/export/Dropped List.mte`, '$filename(csv,utf-8)$loop(%_filename%)%title%$loopend()');
      const library = await inv('load_export_library');
      const dropped = library.configurations.find(c => c.name === 'Dropped List');
      if (!dropped || dropped.file !== 'Dropped List.mte' || !dropped.template.includes('%title%')) throw new Error(JSON.stringify(library.configurations.map(c => [c.name, c.file])));
      // Saving keeps it under its own file name.
      await inv('save_export_library', { library: { ...library, preferences: { ...library.preferences, configuration: dropped.id } } });
      const again = await inv('load_export_library');
      if (again.configurations.filter(c => c.name === 'Dropped List').length !== 1 || again.preferences.configuration !== dropped.id) throw new Error(JSON.stringify(again.configurations.map(c => [c.id, c.name, c.file])));
      return dropped.id;
    });

    await step('every action type survives an .mta export and import', async () => {
      const group = { name: 'Every type', enabled: true, actions: [
        { type: 'case_conversion', field: 'TITLE', mode: 'upper', word_separators: '' },
        { type: 'regex_replace', field: 'TITLE', pattern: '\\s+$', replacement: '', case_sensitive: true },
        { type: 'remove_duplicate_fields', field: '_TAG', only_same_content: true },
        { type: 'convert_codepage', field: '_ALL', codepage: '1251' },
        { type: 'import_cover', filename_template: '%_folderpath%folder.jpg', picture_type: 'CoverBack', delete_existing: true },
        { type: 'export_cover', filename_template: '%album%', picture_type: null, export_duplicates: false },
        { type: 'import_text_file', field: 'LYRICS', filename_template: '%_filename%.txt' },
        { type: 'export', template: 'Text', filename_template: '%_folderpath%list.txt', append: true, partition_template: '%album%' },
        { type: 'adjust_cover', format: 'jpeg', quality: 85, resize: { pixels: 500, size_mode: 'maximum', scale_mode: 'crop_square' } },
        { type: 'set_cover_properties', picture_type: 'CoverFront', description: '', only_picture_type: null },
        { type: 'generate_playlists', filename_template: '%album%.m3u', partition_template: '%_folderpath%' },
      ] };
      await inv('export_action_group', { path: album + '/Every type.mta', group });
      const imported = await inv('import_action_group', { path: album + '/Every type.mta', legacyEncoding: null });
      if (imported.issues.length || JSON.stringify(imported.group.actions) !== JSON.stringify(group.actions)) throw new Error(JSON.stringify(imported));
      return `${imported.group.actions.length} actions`;
    });

    await step('Text File → Tag reads a format-string source for the first selected file', async () => {
      await write(album + '/tracks.txt', 'Line One\r\nLine Two\r\n');
      op('select-all').click();
      await waitFor(() => document.querySelectorAll('tr[data-path].selected').length === 2 || document.querySelectorAll('tr[data-path][aria-selected="true"]').length === 2, 3000, 'selection');
      await waitFor(() => op('text-tags') && !op('text-tags').disabled, 5000, 'Text File → Tag enabled');
      op('text-tags').click();
      const dlg = await waitFor(() => [...document.querySelectorAll('dialog[open]')].find(d => d.querySelector('#text-source-path')), 5000, 'converter dialog');
      input(dlg.querySelector('#text-source-kind'), 'file');
      input(dlg.querySelector('#text-source-path'), '%_folderpath%tracks.txt');
      input(dlg.querySelector('#text-mask'), '%comment%');
      dlg.querySelector('#reviewed-preview-button').click();
      const status = await waitFor(() => { const text = dlg.querySelector('.converter-status').textContent; return /from .*tracks\.txt/.test(text) && text; }, 8000, 'preview status');
      const shown = dlg.querySelector('#reviewed-preview').textContent;
      if (!shown.includes('Line One') || !shown.includes('Line Two')) throw new Error(shown);
      dlg.close();
      return status;
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
