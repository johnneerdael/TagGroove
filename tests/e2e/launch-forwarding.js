// Native e2e for launch forwarding: a real second launch with `/add /fn:` is
// forwarded to the running instance over the single-instance channel (Linux
// D-Bus in CI; the fixtures script starts it), and macOS open-documents events
// (Finder "Open With", Dock drops, `open -a OpenTag <paths>`) are fed through
// the same `RunEvent::Opened` handler with `e2e_open_urls`.
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const errorsText = () => [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' | ');
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const names = () => rows().map(r => r.dataset.path.split(/[\\/]/).pop()).sort();
  const location = () => document.querySelector('.location .path')?.textContent || '';
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const showsOnly = async (expected, what) => {
    const want = [...expected].sort().join(', ');
    await waitFor(() => names().join(', ') === want, 15000, `${what}: rows ${want} (have ${names().join(', ')}; errors "${errorsText()}")`);
    await idle();
  };
  const dirname = path => path.replace(/[\\/][^\\/]*$/, '');
  const fileUrl = (path, folder = false) => 'file://' + encodeURI(path.replace(/\\/g, '/')).replace(/#/g, '%23').replace(/\?/g, '%3F') + (folder ? '/' : '');
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  try {
    await waitFor(() => rows().some(r => r.dataset.path.endsWith('sine-a.mp3')), 15000, 'fixture rows');
    const album = dirname(rows().find(r => r.dataset.path.endsWith('sine-a.mp3')).dataset.path), work = dirname(album);
    const sep = album.includes('\\') ? '\\' : '/';

    await step('A second launch with /add /fn: appends to the running instance', async () => {
      if (!/Linux/.test(navigator.platform)) return 'skipped: the fixtures start the second launch on Linux only';
      await showsOnly(['forward-c.mp3', 'sine-a.mp3', 'sine-b.mp3'], 'forwarded /add');
      if (location() !== album) throw new Error(`/add changed the working directory to ${location()}`);
      return `3 rows, working directory ${location()}`;
    });

    await step('Finder Open With replaces the list with the opened files', async () => {
      const open = work + sep + 'Open With';
      await inv('e2e_open_urls', { urls: [fileUrl(open + sep + 'open-d.mp3'), fileUrl(open + sep + 'open #e.mp3'), 'https://example.org/ignored.mp3'] });
      await showsOnly(['open #e.mp3', 'open-d.mp3'], 'Open With');
      if (location() !== open) throw new Error(`location ${location()}`);
      return names().join(', ');
    });

    await step('A folder dropped on the Dock icon opens as the working directory', async () => {
      const folder = work + sep + 'Opened Folder';
      await inv('e2e_open_urls', { urls: [fileUrl(folder, true)] });
      await showsOnly(['folder-f.mp3'], 'folder drop');
      if (location() !== folder) throw new Error(`location ${location()} (trailing slash kept?)`);
      return location();
    });

    await step('Back-to-back open events load in order without being dropped', async () => {
      await inv('e2e_open_urls', { urls: [fileUrl(album, true)] });
      await inv('e2e_open_urls', { urls: [fileUrl(work + sep + 'Open With' + sep + 'open-d.mp3')] });
      await showsOnly(['open-d.mp3'], 'last open wins');
      return 'second request replaced the first after it finished';
    });

    await step('Non-file URLs are ignored', async () => {
      await inv('e2e_open_urls', { urls: ['https://example.org/a.mp3', 'opentag://launch'] });
      await sleep(800);
      await showsOnly(['open-d.mp3'], 'unchanged list');
      return 'list unchanged';
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
