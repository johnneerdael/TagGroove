// Native e2e for Options → Tags → Advanced → List chapters as separate
// files: MP4 and ID3v2 chapters listed as rows, chapter titles edited in the
// tag panel and saved into the file, Undo, renaming refused for rows while
// Remove tag stays available, and a loaded cue sheet listed per track with edits written into the
// sheet. Fixtures come from tests/e2e/chapters-fixtures.sh. The first page
// load turns the option on and reloads into the same folder.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const SEP = '\u0001';
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const rowPaths = () => rows().map(r => r.dataset.path);
  const chapterRow = (name, n) => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name + SEP + n);
  const fileRow = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const click = async r => { r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const scan = async path => (await inv('scan_files', { paths: [path], recursive: false })).entries;
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const relaunch = async (file, any) => {
    const resources = await inv('plugin:path|resolve_directory', { directory: 11 });
    const binary = /\/Resources\/?$/.test(resources) ? resources.replace(/\/Resources\/?$/, '/MacOS/opentag')
      : /^Win/.test(navigator.platform) ? resources.replace(/\\?$/, '\\opentag.exe') : resources.replace(/\/?$/, '/opentag');
    await inv('run_tool', { tool: { name: 'Relaunch', path: binary, parameters: `"/fn:${file}"`, for_each_file: false }, paths: [any], root: null });
  };
  const saveTitle = async (target, title) => {
    await click(target);
    const f = await waitFor(() => field('TITLE'), 4000, 'TITLE field');
    f.focus(); f.value = title; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
    f.blur(); document.body.focus(); key('s', { metaKey: true });
    await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status (' + status() + ')'); await idle();
  };

  try {
    if (!sessionStorage.getItem('chapters-stage')) {
      await waitFor(() => rows().length >= 5, 15000, 'fixture rows');
      const album = fileRow('image.flac').dataset.path.replace(/[\\/][^\\/]+$/, '');
      const settings = (await inv('load_settings')).settings;
      settings.tags.advanced.list_chapters = true;
      settings.pages = { ...settings.pages, 'directories-startup': { startup: 'previous' } };
      await inv('save_settings', { settings });
      await inv('remember_directories', { paths: [album] });
      sessionStorage.setItem('chapters-stage', '2');
      location.reload();
      return;
    }
    await waitFor(() => chapterRow('book.m4b', 2), 15000, 'chapter rows');
    await idle();
    const album = fileRow('image.flac').dataset.path.replace(/[\\/][^\\/]+$/, '');
    const sep = album.includes('\\') ? '\\' : '/';
    const book = album + sep + 'book.m4b', podcast = album + sep + 'podcast.mp3';

    await step('files with chapters are listed as one row per chapter', async () => {
      const paths = rowPaths();
      for (const expected of [book + SEP + '1', book + SEP + '2', podcast + SEP + '1', podcast + SEP + '2', album + sep + 'image.flac'])
        if (!paths.includes(expected)) throw new Error(paths.map(p => p.replace(SEP, '#')).join(', '));
      if (paths.includes(book) || paths.includes(podcast)) throw new Error('the files are listed as well');
      const [second] = (await scan(book + SEP + '2'));
      if (second.fields.TITLE?.[0] !== 'Second part' || second.fields.ARTIST?.[0] !== 'Narrator' || second.technical._SUBSONG?.[0] !== '1' || second.technical._SUBSONG_COUNT?.[0] !== '2')
        throw new Error(JSON.stringify([second.fields, second.technical._SUBSONG]));
      return `${paths.length} rows; row 2 "${second.fields.TITLE[0]}", ${second.technical._LENGTH?.[0]}`;
    });

    await step('a chapter title edited in the tag panel is saved into the MP4 chapter', async () => {
      await saveTitle(chapterRow('book.m4b', 2), 'Middle part');
      const listed = await scan(book);
      const titles = listed.map(e => e.fields.TITLE?.[0]).join('|');
      if (titles !== 'Opening|Middle part') throw new Error(titles);
      if (listed.some(e => e.fields.ARTIST?.[0] !== 'Narrator')) throw new Error('artist changed');
      if (field('TITLE').value !== 'Middle part') throw new Error('panel shows ' + field('TITLE').value);
      return titles;
    });

    await step('Undo restores the chapter title', async () => {
      chapterRow('book.m4b', 2).focus(); key('z', { metaKey: true });
      await waitFor(() => /Undid/.test(status()), 8000, 'undo status (' + status() + ')'); await idle();
      const titles = (await scan(book)).map(e => e.fields.TITLE?.[0]).join('|');
      if (titles !== 'Opening|Second part') throw new Error(titles);
      await waitFor(() => chapterRow('book.m4b', 2), 4000, 'row after undo');
      return titles;
    });

    await step('ID3v2 CHAP titles are edited as rows', async () => {
      await saveTitle(chapterRow('podcast.mp3', 1), 'Cold open');
      const titles = (await scan(podcast)).map(e => e.fields.TITLE?.[0]).join('|');
      if (titles !== 'Cold open|Second part') throw new Error(titles);
      return titles;
    });

    await step('renaming is refused for chapter rows; Remove tag works on their file', async () => {
      await click(chapterRow('book.m4b', 1));
      const refused = ['tags-filename', 'filename-filename'].filter(name => op(name));
      for (const name of refused) if (!op(name).disabled || !/not files of their own/.test(op(name).title)) throw new Error(`${name} enabled: ${op(name).title}`);
      for (const name of ['remove-tags', 'strip-mp4-chapters', 'extended']) if (op(name)?.disabled) throw new Error(`${name} disabled: ${op(name).title}`);
      await click(fileRow('image.flac'));
      if (op('remove-tags')?.disabled) throw new Error('Remove tag disabled for an ordinary file');
      return refused.join(', ') + ' disabled';
    });

    await step('a loaded cue sheet lists its tracks and row edits are written into the sheet', async () => {
      await relaunch(album + sep + 'live.cue', album + sep + 'image.flac');
      await waitFor(() => /live\.cue$/.test(document.querySelector('.location .path')?.textContent || ''), 15000, 'cue location');
      await idle();
      const cue = album + sep + 'live.cue';
      await waitFor(() => rowPaths().length === 2 && rowPaths()[1] === cue + SEP + '2', 8000, 'cue rows ' + rowPaths().map(p => p.replace(SEP, '#')).join());
      await saveTitle(rows()[1], 'Slow one');
      const [track] = await scan(cue + SEP + '2');
      if (track.fields.TITLE?.[0] !== 'Slow one' || track.fields.ALBUM?.[0] !== 'Live' || track.fields.ARTIST?.[0] !== 'The Trio' || track.filename !== 'image.flac')
        throw new Error(JSON.stringify(track.fields));
      const [first] = await scan(cue + SEP + '1');
      if (first.fields.TITLE?.[0] !== 'Opening') throw new Error(JSON.stringify(first.fields));
      const image = (await scan(album + sep + 'image.flac')).find(e => e.path === album + sep + 'image.flac');
      if (image?.fields.TITLE) throw new Error('the image file was tagged: ' + JSON.stringify(image.fields));
      return `track 2 "${track.fields.TITLE[0]}", ${track.duration.toFixed(1)} s`;
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
