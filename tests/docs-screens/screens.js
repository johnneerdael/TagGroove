// Drives the debug app through named states for the user guide's screenshots.
// scripts/docs-screenshots.sh runs it in CI (Linux, xvfb) against the album
// from tests/docs-screens/fixtures.sh. Each `SHOT <name>` line asks the runner
// to capture the window; the state is held until the runner has done so.
// Never run on a working desktop.
//
// To add a screen: append a scene below. `name` becomes
// docs/user-guide/assets/screens/<name>-light.png and <name>-dark.png.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + last.message + ')' : '')); };
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 15000, 'idle');
  const op = name => { const b = document.querySelector(`.menubar button[data-op="${name}"]`) || document.querySelector(`button[data-op="${name}"]`); if (!b) throw new Error('no command ' + name); b.click(); };
  const input = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const dialog = () => document.querySelector('dialog[open]');
  const clickRow = async index => { rows()[index].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const selectAll = async () => { op('select-all'); await waitFor(() => rows().every(r => r.classList.contains('selected')), 3000, 'all selected'); };
  const menu = label => [...document.querySelectorAll('.menubar > .menu')].find(d => d.querySelector('summary')?.textContent.trim() === label);
  const reset = async () => {
    for (let i = 0; i < 5 && dialog(); i++) { dialog().close(); await sleep(150); }
    document.querySelectorAll('details[open]').forEach(d => { d.open = false; });
    document.activeElement?.blur?.();
    await idle();
  };
  // Hold each state until the runner captures it. Captures take well under a
  // second under xvfb; the margin keeps slow runners from mixing scenes.
  const shot = async name => { await sleep(900); await report('SHOT ' + name); await sleep(2500); };

  const scenes = [
    { name: 'main-window', run: async () => { await clickRow(0); } },
    { name: 'tag-panel-keep', run: async () => { await selectAll(); } },
    { name: 'menu-convert', run: async () => { await clickRow(0); const m = menu('Convert'); if (!m) throw new Error('no Convert menu'); m.open = true; } },
    {
      name: 'tag-to-filename', run: async () => {
        await selectAll(); op('tags-filename');
        const d = await waitFor(() => dialog()?.querySelector('#template') && dialog(), 5000, 'Tag → Filename');
        input(d.querySelector('#template'), '$num(%track%,2) - %artist% - %title%');
        d.querySelector('#preview-button').click();
        await waitFor(() => d.querySelector('#preview .preview'), 8000, 'preview');
      }
    },
    {
      name: 'extended-tags', run: async () => {
        await clickRow(1); op('extended');
        await waitFor(() => document.querySelector('dialog[open].extended-dialog #extended-rows .extended-row'), 5000, 'Extended tags');
      }
    },
  ];
  const wanted = window.OPENTAG_DOCS_SCENES;
  let failures = 0, taken = 0;
  try {
    await waitFor(() => rows().length >= 8, 20000, 'fixture rows');
    await idle();
    for (const mode of ['light', 'dark']) {
      const select = document.querySelector('#appearance');
      input(select, mode);
      await waitFor(() => document.documentElement.dataset.theme === mode, 3000, mode + ' theme');
      for (const scene of scenes) {
        if (Array.isArray(wanted) && wanted.length && !wanted.includes(scene.name)) continue;
        try {
          await reset();
          await scene.run();
          await shot(`${scene.name}-${mode}`);
          taken++;
        } catch (e) {
          failures++;
          await report(`FAIL ${scene.name}-${mode} — ${e && e.message || e}`);
        }
      }
    }
  } catch (e) {
    failures++;
    await report(`FATAL ${e && e.message || e}`);
  }
  await report(`SUMMARY ${taken} screenshots, ${failures} failed`);
  await inv('e2e_exit', { code: failures ? 1 : 0 });
})();
