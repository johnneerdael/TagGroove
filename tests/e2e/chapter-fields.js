// Native e2e for chapter rows beyond titles (Options → Tags → Advanced →
// List chapters as separate files): fields typed for one chapter row are
// stored with that chapter (ID3v2 CHAP sub-frames, Nero tags in MP4,
// chapter-targeted Matroska tags) while the same value on every row becomes
// the file's; Remove tag and Strip chapters MP4 run on the file of a row;
// %_subsong% and %_subsong_count%; and a cue sheet whose second track starts
// in one FILE and plays from the next. Fixtures come from
// tests/e2e/chapter-fields-fixtures.sh. The first page load turns the option
// on and reloads into the same folder.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const SEP = '\u0001';
  const base = path => path.split(/[\\/]/).pop();
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const rowPaths = () => rows().map(r => r.dataset.path);
  const chapterRow = (name, n) => rows().find(r => base(r.dataset.path) === name + SEP + n);
  const fileRow = name => rows().find(r => base(r.dataset.path) === name);
  const click = async r => { r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const ready = name => waitFor(() => op(name) && !op(name).disabled && op(name), 8000, name + ' enabled (' + (op(name)?.title || 'no button') + ')');
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const scan = async path => (await inv('scan_files', { paths: [path], recursive: false })).entries;
  const preview = async (path, template) => (await inv('preview_format', { paths: [path], template, columns: null }))[0].value;
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const relaunch = async (file, any) => {
    const binary = window.OPENTAG_E2E_BINARY;
    if (typeof binary !== 'string' || !binary) throw new Error('Native runner did not supply its executable');
    await inv('run_tool', { tool: { name: 'Relaunch', path: binary, parameters: `"/fn:${file}"`, for_each_file: false }, paths: [any], root: null });
  };
  // Type a value into a tag panel field of one row and save with the toolbar.
  const saveField = async (target, name, value) => {
    await click(target);
    const f = await waitFor(() => field(name), 4000, name + ' field');
    f.focus(); f.value = value; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
    f.blur(); (await ready('save')).click();
    await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status (' + status() + ')'); await idle();
  };
  const values = (entries, name) => entries.map(e => e.fields[name]?.join(';') || '').join('|');

  try {
    if (!sessionStorage.getItem('chapter-fields-stage')) {
      await waitFor(() => fileRow('podcast.mp3') && fileRow('film.mka'), 15000, 'fixture rows');
      const album = fileRow('podcast.mp3').dataset.path.replace(/[\\/][^\\/]+$/, '');
      const settings = (await inv('load_settings')).settings;
      settings.tags.advanced.list_chapters = true;
      settings.pages = { ...settings.pages, messages: { confirm_remove_tags: false }, 'directories-startup': { startup: 'previous' } };
      await inv('save_settings', { settings });
      await inv('remember_directories', { paths: [album] });
      sessionStorage.setItem('chapter-fields-stage', '2');
      location.reload();
      return;
    }
    // Ordinary restart restores exact saved rows. Rescan the folder explicitly
    // after this fixture changes List chapters through the backend settings API.
    await waitFor(() => rows().length >= 5, 15000, 'saved fixture rows');
    await idle();
    await report('SETUP saved rows before explicit chapter refresh: ' + rowPaths().map(path => path.replace(SEP, '#')).join(', '));
    const refresh = await waitFor(() => { const button = op('refresh'); return button && !button.disabled && button; }, 3000, 'Refresh enabled');
    refresh.click();
    await waitFor(() => chapterRow('film.mka', 2) && chapterRow('book.m4b', 2), 15000, 'chapter rows');
    await idle();
    const album = fileRow('sine-a.mp3').dataset.path.replace(/[\\/][^\\/]+$/, '');
    const sep = album.includes('\\') ? '\\' : '/';
    const book = album + sep + 'book.m4b', podcast = album + sep + 'podcast.mp3', film = album + sep + 'film.mka';

    await step('%_subsong% counts chapter rows from 0 and a plain file is subsong 0 of 1', async () => {
      const second = await preview(podcast + SEP + '2', '%_subsong%/%_subsong_count%');
      const plain = await preview(album + sep + 'sine-a.mp3', '%_subsong%/%_subsong_count%');
      if (second !== '1/2' || plain !== '0/1') throw new Error(`${second}; ${plain}`);
      return `chapter 2: ${second}; sine-a.mp3: ${plain}`;
    });

    await step('an ARTIST typed for one ID3v2 chapter row stays with that chapter', async () => {
      await saveField(chapterRow('podcast.mp3', 2), 'ARTIST', 'Guest');
      const listed = await scan(podcast);
      if (values(listed, 'ARTIST') !== 'Host|Guest' || values(listed, 'TITLE') !== 'Opening|Second part') throw new Error(values(listed, 'ARTIST') + ' / ' + values(listed, 'TITLE'));
      return values(listed, 'ARTIST');
    });

    await step('an MP4 field differs per chapter (Nero tags) until both rows agree', async () => {
      await saveField(chapterRow('book.m4b', 1), 'ALBUM', 'Collected');
      let listed = await scan(book);
      if (values(listed, 'ALBUM') !== 'Collected|') throw new Error('after row 1: ' + values(listed, 'ALBUM'));
      await saveField(chapterRow('book.m4b', 2), 'ALBUM', 'Collected');
      listed = await scan(book);
      if (values(listed, 'ALBUM') !== 'Collected|Collected' || values(listed, 'ARTIST') !== 'Narrator|Narrator') throw new Error(values(listed, 'ALBUM') + ' / ' + values(listed, 'ARTIST'));
      return values(listed, 'ALBUM');
    });

    await step('a Matroska chapter row keeps a COMMENT of its own', async () => {
      await saveField(chapterRow('film.mka', 1), 'COMMENT', 'Intro scene');
      const listed = await scan(film);
      if (values(listed, 'COMMENT') !== 'Intro scene|' || values(listed, 'TITLE') !== 'Opening|Second part') throw new Error(values(listed, 'COMMENT') + ' / ' + values(listed, 'TITLE'));
      return values(listed, 'COMMENT');
    });

    await step('Remove tag on a chapter row clears the file and keeps its chapter rows', async () => {
      await click(chapterRow('book.m4b', 1));
      (await ready('remove-tags')).click();
      await waitFor(() => /Removed tags/.test(status()), 8000, 'remove status (' + status() + ')'); await idle();
      const listed = await scan(book);
      if (listed.length !== 2 || values(listed, 'ARTIST') !== '|' || values(listed, 'ALBUM') !== '|' || values(listed, 'TITLE') !== 'Opening|Second part') throw new Error(JSON.stringify(listed.map(e => e.fields)));
      if (!chapterRow('book.m4b', 1) || !chapterRow('book.m4b', 2) || fileRow('book.m4b')) throw new Error(rowPaths().map(p => base(p).replace(SEP, '#')).join());
      return status();
    });

    await step('Strip chapters MP4 on a row lists the file again and Undo brings the rows back', async () => {
      const before = rowPaths().map(base);
      await click(chapterRow('book.m4b', 2));
      (await ready('strip-mp4-chapters')).click();
      await waitFor(() => /Stripped MP4 chapters/.test(status()), 8000, 'strip status (' + status() + ')'); await idle();
      await waitFor(() => fileRow('book.m4b') && !chapterRow('book.m4b', 1), 4000, 'file row ' + rowPaths().map(p => base(p).replace(SEP, '#')).join());
      if (rowPaths().indexOf(fileRow('book.m4b').dataset.path) !== before.indexOf('book.m4b' + SEP + '1')) throw new Error('the file moved: ' + rowPaths().map(base).join());
      (await ready('undo')).click();
      await waitFor(() => /Undid/.test(status()), 8000, 'undo status (' + status() + ')'); await idle();
      await waitFor(() => chapterRow('book.m4b', 1) && chapterRow('book.m4b', 2) && !fileRow('book.m4b'), 4000, 'rows after undo ' + rowPaths().map(p => base(p).replace(SEP, '#')).join());
      const titles = values(await scan(book), 'TITLE');
      if (titles !== 'Opening|Second part') throw new Error(titles);
      return titles;
    });

    await step('renaming is still refused for chapter rows', async () => {
      await click(chapterRow('film.mka', 2));
      const rename = op('tags-filename');
      if (rename && (!rename.disabled || !/not files of their own/.test(rename.title))) throw new Error(`tags-filename: ${rename.disabled} ${rename.title}`);
      if (op('remove-tags')?.disabled) throw new Error('Remove tag disabled: ' + op('remove-tags').title);
      return rename ? rename.title : 'no Tag → Filename button';
    });

    await step('a cue sheet track whose INDEX 01 sits in the next FILE plays from that file', async () => {
      await relaunch(album + sep + 'gaps.cue', album + sep + 'sine-a.mp3');
      await waitFor(() => /gaps\.cue$/.test(document.querySelector('.location .path')?.textContent || ''), 15000, 'cue location');
      await idle();
      const cue = album + sep + 'gaps.cue';
      await waitFor(() => rowPaths().length === 2 && rowPaths()[1] === cue + SEP + '2', 8000, 'cue rows ' + rowPaths().map(p => base(p).replace(SEP, '#')).join());
      const [first] = await scan(cue + SEP + '1'), [second] = await scan(cue + SEP + '2');
      if (first.filename !== 'disc-1.flac' || second.filename !== 'disc-2.flac') throw new Error(`${first.filename}, ${second.filename}`);
      // Track 1 keeps the pregap at the end of disc-1.flac; track 2 starts at 0 in disc-2.flac.
      if (Math.abs(first.duration - 2) > 0.05 || Math.abs(second.duration - 2) > 0.05) throw new Error(`${first.duration}, ${second.duration}`);
      if (second.fields.TITLE?.[0] !== 'Second' || second.fields.ARTIST?.[0] !== 'The Duo') throw new Error(JSON.stringify(second.fields));
      const sub = await preview(cue + SEP + '2', '%_subsong%/%_subsong_count%');
      if (sub !== '1/2') throw new Error(sub);
      await saveField(rows()[1], 'TITLE', 'Second (live)');
      const [edited] = await scan(cue + SEP + '2');
      if (edited.fields.TITLE?.[0] !== 'Second (live)') throw new Error(JSON.stringify(edited.fields));
      return `${first.duration.toFixed(2)} s + ${second.duration.toFixed(2)} s; ${sub}`;
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
