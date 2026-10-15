// Actions, action groups and helper menus driven through the real UI (see docs/verification/native-e2e.md).
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(100); } throw new Error('timeout waiting for ' + what); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const row = name => [...document.querySelectorAll('tr[data-path]')].find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const read = async name => { const p = row(name)?.dataset.path; const b = await inv('scan_files', { paths: [p], recursive: false }); return b.entries[0]?.fields || {}; };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const top = () => [...document.querySelectorAll('dialog[open]')].at(-1);
  const topWith = (selector, what) => waitFor(() => top()?.querySelector(selector) && top(), 5000, what);
  const set = (el, value) => { el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const helperFor = el => el.parentElement.querySelector('.helper-button');
  const openHelper = async el => { const b = await waitFor(() => helperFor(el), 3000, 'helper button'); b.click(); return waitFor(() => document.querySelector('.helper-menu'), 3000, 'helper menu'); };
  const item = (menu, label) => [...menu.querySelectorAll('button')].find(b => b.textContent.trim() === label);
  const topLabels = menu => [...menu.children].map(c => c.tagName === 'HR' ? '—' : c.classList.contains('helper-group') ? c.querySelector('button').textContent.trim() : c.textContent.trim());
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); document.querySelectorAll('.helper-menu').forEach(m => m.remove()); } };
  try {
    await waitFor(() => document.querySelectorAll('tr[data-path]').length >= 2, 15000, 'fixture rows');
    await clickRow('sine-a.mp3');

    await step('filter helper menu lists sections and toggles Auto-apply filter', async () => {
      const filter = document.querySelector('#filter'); const menu = await openHelper(filter);
      const labels = topLabels(menu).join('|');
      for (const expected of ['TITLE', 'Information fields', 'Functions', 'Manage history…', 'Auto-apply filter']) if (!labels.includes(expected)) throw new Error('menu=' + labels);
      if (item(menu, 'Auto-apply filter').getAttribute('aria-checked') !== 'true') throw new Error('Auto-apply filter should start enabled (clear the e2e profile)');
      item(menu, 'Auto-apply filter').click(); await sleep(300);
      const input = document.querySelector('#filter'); input.focus(); input.value = 'sine-b'; input.dispatchEvent(new Event('input', { bubbles: true })); await sleep(500);
      const before = document.querySelectorAll('tr[data-path]').length; if (before !== 2) throw new Error('filtered without Return: rows=' + before);
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); await waitFor(() => document.querySelectorAll('tr[data-path]').length === 1, 5000, 'Return applies filter');
      const again = await openHelper(document.querySelector('#filter')); const auto = item(again, 'Auto-apply filter'); if (auto.getAttribute('aria-checked') !== 'false') throw new Error('auto-apply still checked');
      auto.click(); await sleep(200); const f = document.querySelector('#filter'); set(f, ''); await waitFor(() => document.querySelectorAll('tr[data-path]').length === 2, 5000, 'cleared filter');
      return labels;
    });

    await step('Tag → Filename helper inserts placeholders and Manage history… stores a named entry', async () => {
      await clickRow('sine-a.mp3'); document.querySelector('button[data-op="tags-filename"]').click();
      const dlg = await topWith('#template', 'Tag → Filename'); const tpl = dlg.querySelector('#template'); tpl.focus(); tpl.setSelectionRange(tpl.value.length, tpl.value.length);
      let menu = await openHelper(tpl); item(menu, 'ALBUM').click(); await sleep(100);
      if (tpl.value !== '%artist% - %title%%album%') throw new Error('value=' + tpl.value);
      menu = await openHelper(tpl); item(menu, 'Functions').click(); await sleep(100); item(menu, '$num(x,y)').click(); await sleep(100);
      if (!tpl.value.endsWith('$num(x,y)') || tpl.value.slice(tpl.selectionStart, tpl.selectionEnd) !== 'x') throw new Error('function insert value=' + tpl.value);
      menu = await openHelper(tpl); item(menu, 'Manage history…').click();
      const manage = await topWith('.history-list', 'Manage History'); manage.querySelector('[data-add]').click(); await sleep(100);
      manage.querySelector('[data-name]').value = 'Numbered'; manage.querySelector('[data-value]').value = '%track% %title%'; manage.querySelector('[data-keep]').click(); await sleep(100);
      const entries = [...manage.querySelectorAll('.history-list li')].map(li => li.textContent).join('|');
      manage.querySelector('[data-use]').click(); await sleep(200);
      if (tpl.value !== '%track% %title%') throw new Error('used value=' + tpl.value);
      const options = [...document.getElementById(tpl.getAttribute('list')).options].map(o => `${o.label}=${o.value}`).join('|');
      dlg.close(); await sleep(200); return `history=${entries}; datalist=${options}`;
    });

    await step('Actions dialog: create group with helper-built Format Value, Apply writes, ⌘Z undoes', async () => {
      await clickRow('sine-a.mp3'); document.querySelector('button[data-op="groups"]').click();
      let groups = await topWith('#apply-groups', 'Action groups'); groups.querySelector('[data-command="new"]').click();
      let name = await topWith('[data-key="name"]', 'name dialog'); name.querySelector('[data-key="name"]').value = '&Comment fill'; name.querySelector('[data-keep]').click(); await sleep(200);
      groups.querySelector('[data-command="edit"]').click();
      const editor = await topWith('.action-edit-list', 'group editor'); editor.querySelector('[data-command="new"]').click();
      const action = await topWith('#full-action-type', 'action dialog'); const type = action.querySelector('#full-action-type'); type.value = 'format_value'; type.dispatchEvent(new Event('change', { bubbles: true })); await sleep(150);
      const field = action.querySelector('[data-key="field"]'); const fmenu = await openHelper(field);
      const flabels = topLabels(fmenu).join('|'); if (!/Remove field\|Remove all fields…\|Reset fields…/.test(flabels)) throw new Error('fields menu=' + flabels);
      item(fmenu, 'Choose field').click(); await sleep(100); item(fmenu, 'COMMENT').click(); await sleep(100);
      if (field.value !== 'COMMENT') throw new Error('field=' + field.value);
      const tpl = action.querySelector('[data-key="template"]'); set(tpl, '%artist% ('); tpl.setSelectionRange(tpl.value.length, tpl.value.length);
      const menu = await openHelper(tpl); item(menu, 'ALBUM').click(); await sleep(100); tpl.value += ')';
      action.querySelector('[data-keep]').click(); await sleep(200);
      editor.querySelector('[data-keep]').click(); await sleep(200);
      groups = top(); const summary = groups.querySelector('.group-summary')?.textContent;
      const apply = groups.querySelector('#apply-groups'); await waitFor(() => !apply.disabled, 5000, 'apply enabled'); apply.click();
      await waitFor(() => !document.querySelector('dialog[open]'), 10000, 'groups dialog closed'); await idle(); await sleep(500);
      const c = (await read('sine-a.mp3')).COMMENT; if (c?.[0] !== 'Test Artist (Test Album)') throw new Error('disk COMMENT=' + JSON.stringify(c));
      const other = (await read('sine-b.mp3')).COMMENT; if (other) throw new Error('unselected file changed: ' + other);
      await waitFor(() => { const b = document.querySelector('button[data-op="undo"]'); return b && !b.disabled; }, 8000, 'undo available');
      row('sine-a.mp3').focus(); key('z', { metaKey: true }); await waitFor(() => /Undid/.test(status()), 8000, 'undo status');
      const after = (await read('sine-a.mp3')).COMMENT; if (after?.length) throw new Error('COMMENT after undo=' + after);
      return `${summary}; ${status()}`;
    });

    await step('Actions menu underlines the group mnemonic and Alt+A, C runs it', async () => {
      await idle(); await clickRow('sine-b.mp3');
      const menuButton = await waitFor(() => document.querySelector('#actions-menu [data-action-group]'), 5000, 'group in Actions menu');
      if (!menuButton.innerHTML.includes('<u>C</u>')) throw new Error('menu html=' + menuButton.innerHTML);
      document.body.focus(); key('a', { altKey: true, code: 'KeyA' }); await sleep(200); key('c'); await sleep(300); await idle(); await sleep(500);
      const c = (await read('sine-b.mp3')).COMMENT; if (c?.[0] !== 'Test Artist (Test Album)') throw new Error('disk COMMENT=' + JSON.stringify(c));
      document.querySelectorAll('details[open]').forEach(d => d.open = false); return menuButton.innerHTML;
    });

    await step('Groups dialog sorts by name/state and duplicates as "Copy of"', async () => {
      document.querySelector('button[data-op="groups"]').click();
      const groups = await topWith('#apply-groups', 'Action groups'); groups.querySelector('[data-command="duplicate"]').click();
      const name = await topWith('[data-key="name"]', 'duplicate name'); const proposed = name.querySelector('[data-key="name"]').value; name.querySelector('[data-keep]').click(); await sleep(200);
      if (proposed !== 'Copy of &Comment fill') throw new Error('proposed=' + proposed);
      groups.querySelector('[data-enable="0"]').click(); await sleep(100);
      groups.querySelector('[data-command="sort-state"]').click(); await sleep(100);
      const byState = [...groups.querySelectorAll('.group-row span')].map(s => s.textContent).join('|');
      groups.querySelector('[data-command="sort-name"]').click(); await sleep(100);
      const byName = [...groups.querySelectorAll('.group-row span')].map(s => s.textContent).join('|');
      const summary = groups.querySelector('.group-summary').textContent; const exportButton = !!groups.querySelector('[data-command="export"]');
      if (byState !== 'Copy of &Comment fill|&Comment fill' || byName !== '&Comment fill|Copy of &Comment fill' || summary !== '1 action group enabled' || !exportButton) throw new Error(`state=${byState} name=${byName} summary=${summary}`);
      groups.querySelector('#save-groups').click(); await waitFor(() => groups.querySelector('#save-groups').disabled && !groups.querySelector('[data-close]').disabled, 5000, 'saved groups');
      groups.querySelector('[data-close]').click(); await waitFor(() => !document.querySelector('dialog[open]'), 5000, 'groups closed');
      return `${byState} / ${byName}`;
    });

    await step('Quick action editor offers import cover type, delete option, quality presets and codepages', async () => {
      await clickRow('sine-a.mp3'); document.querySelector('button[data-op="action"]').click();
      const dlg = await topWith('#full-action-type', 'quick action'); const type = dlg.querySelector('#full-action-type');
      type.value = 'import_cover'; type.dispatchEvent(new Event('change', { bubbles: true })); await sleep(100);
      const coverType = dlg.querySelector('[data-key="picture_type"]').value, del = !!dlg.querySelector('[data-key="delete_existing"]');
      type.value = 'adjust_cover'; type.dispatchEvent(new Event('change', { bubbles: true })); await sleep(100);
      const preset = dlg.querySelector('[data-key="quality_preset"]'); preset.value = '75'; preset.dispatchEvent(new Event('change', { bubbles: true }));
      const quality = dlg.querySelector('[data-key="quality"]').value;
      type.value = 'convert_codepage'; type.dispatchEvent(new Event('change', { bubbles: true })); await sleep(100);
      const codepages = dlg.querySelector('[data-key="codepage"]').options.length;
      type.value = 'regex_replace'; type.dispatchEvent(new Event('change', { bubbles: true })); await sleep(150);
      const rmenu = await openHelper(dlg.querySelector('[data-key="pattern"]')); const rlabels = topLabels(rmenu).join('|'); rmenu.remove();
      type.value = 'replace'; type.dispatchEvent(new Event('change', { bubbles: true })); await sleep(100);
      set(dlg.querySelector('[data-key="field"]'), 'TITLE'); dlg.querySelector('[data-key="from"]').value = 'Sine'; dlg.querySelector('[data-key="to"]').value = 'Tone';
      dlg.querySelector('[data-keep]').click(); await waitFor(() => !document.querySelector('dialog[open]'), 8000, 'quick closed'); await idle(); await sleep(400);
      const t = (await read('sine-a.mp3')).TITLE?.[0]; if (t !== 'Tone A' || coverType !== 'CoverFront' || !del || quality !== '75' || codepages < 30 || !rlabels.startsWith('Regular expression')) throw new Error(`title=${t} type=${coverType} delete=${del} quality=${quality} codepages=${codepages} regex=${rlabels}`);
      return `TITLE=${t}; codepages=${codepages}; regex menu=${rlabels}`;
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
