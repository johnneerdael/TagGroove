import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/helper-menu.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { helperKey, FieldList, FormatHistory, defaultFieldList, helperEntries, insertion, extendedFields, informationFields, scriptFunctions, placeholder, ConfigListStorage, configListKey, duplicateEntry } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const memory = () => { const data = new Map(); return { getItem: key => data.get(key) ?? null, setItem: (key, value) => data.set(key, value), data }; };
const labels = entries => entries.map(entry => entry.separator ? '—' : entry.label);
const submenu = (entries, label) => entries.find(entry => entry.label === label)?.submenu;

test('field list starts from defaults, grows with used names, and supports remove/clear/reset', () => {
  const storage = memory(), list = new FieldList(storage);
  assert.deepEqual(list.list(), defaultFieldList);
  assert.equal(list.add('mood'), false, 'existing names are not duplicated case-insensitively');
  assert.equal(list.add('Custom Field'), true);
  assert.ok(list.has('CUSTOM FIELD'));
  assert.equal(list.add('_FILENAME'), false, 'special names are offered per input, not stored');
  assert.equal(list.add('BAD;NAME'), false);
  assert.equal(list.remove('custom field'), true);
  assert.equal(list.has('Custom Field'), false);
  list.clear(); assert.deepEqual(list.list(), []);
  list.reset(); assert.deepEqual(list.list(), defaultFieldList);
  storage.setItem('opentag.field-list', '{broken'); assert.deepEqual(list.list(), defaultFieldList);
});

test('history is most-recent-first, deduplicated, bounded and keeps named entries', () => {
  const history = new FormatHistory(memory());
  history.add('tag-filename', '%artist% - %title%');
  history.add('tag-filename', '%track% %title%');
  history.set('tag-filename', [{ value: '%track% %title%', name: 'Numbered', description: 'Track first' }, ...history.list('tag-filename').slice(1)]);
  history.add('tag-filename', '%artist% - %title%');
  history.add('tag-filename', '%track% %title%');
  assert.deepEqual(history.list('tag-filename'), [{ value: '%track% %title%', name: 'Numbered', description: 'Track first' }, { value: '%artist% - %title%' }]);
  history.add('tag-filename', '   ');
  assert.equal(history.list('tag-filename').length, 2);
  assert.equal(history.remove('tag-filename', '%artist% - %title%'), true);
  for (let i = 0; i < 40; i++) history.add('filter', `value ${i}`);
  assert.equal(history.list('filter').length, FormatHistory.limit);
  assert.equal(history.list('filter')[0].value, 'value 39');
  history.clear('filter'); assert.deepEqual(history.list('filter'), []);
  assert.deepEqual(history.list('tag-filename').map(entry => entry.value), ['%track% %title%'], 'histories are independent per input');
});

test('insertion replaces the selection and selects the first function parameter', () => {
  assert.deepEqual(insertion('a  b', 2, 2, '%album%'), { value: 'a %album% b', start: 9, end: 9 });
  assert.deepEqual(insertion('x OLD y', 2, 5, '$num(x,y)'), { value: 'x $num(x,y) y', start: 7, end: 8 });
  assert.deepEqual(insertion('', 0, 0, '$rand()'), { value: '$rand()', start: 7, end: 7 });
  assert.deepEqual(insertion('', 0, 0, '$fmtDate(fmt,x)'), { value: '$fmtDate(fmt,x)', start: 9, end: 12 });
  assert.equal(placeholder('ALBUMARTIST'), '%albumartist%');
});

test('format helper follows the documented section order and context restrictions', () => {
  const panel = ['TITLE', 'ARTIST', 'ALBUM', '_DIRECTORY'], known = ['MOOD', 'title', 'BPM', '_PATH'];
  const format = helperEntries({ kind: 'format', panel, known, history: true, manage: true });
  assert.deepEqual(labels(format), ['TITLE', 'ARTIST', 'ALBUM', 'Extended fields', 'Information fields', 'Functions', '—', 'Manage history…']);
  assert.equal(format[0].insert, '%title%');
  assert.deepEqual(submenu(format, 'Extended fields').map(entry => entry.insert), ['%bpm%', '%mood%']);
  const info = submenu(format, 'Information fields');
  assert.ok(info.some(entry => entry.insert === '%_filename%'));
  assert.ok(!info.some(entry => entry.insert === '%_total_files%'), 'export totals only where exports evaluate them');
  assert.ok(submenu(helperEntries({ kind: 'format-export', panel, known }), 'Information fields').some(entry => entry.insert === '%_total_files%'));
  const functions = submenu(format, 'Functions');
  assert.deepEqual(functions.filter(entry => entry.heading).map(entry => entry.label), ['String functions', 'Boolean functions', 'Arithmetic functions', 'Variable functions', 'Metadata functions']);
  assert.equal(functions.filter(entry => !entry.heading).length, 65, 'every documented signature, including both $meta forms');
  const history = helperEntries({ kind: 'format', panel, known, history: true, historySelected: false });
  assert.deepEqual(labels(history).slice(-2), ['Remove from history', 'Remove all from history…']);
  assert.equal(history.at(-2).disabled, true);
});

test('matching patterns, placeholders, regex inputs and the filter get only applicable sections', () => {
  const panel = ['TITLE'], known = [];
  assert.deepEqual(labels(helperEntries({ kind: 'pattern', panel, known })), ['TITLE', 'DUMMY (skip text)']);
  assert.deepEqual(labels(helperEntries({ kind: 'placeholder', panel, known: ['MOOD'] })), ['TITLE', 'Extended fields']);
  const regex = helperEntries({ kind: 'regex', panel, known, history: true, historySelected: true });
  assert.deepEqual(labels(regex), ['Regular expression', '—', 'Remove from history', 'Remove all from history…']);
  assert.ok(submenu(regex, 'Regular expression').some(entry => entry.insert === '(.*)'));
  assert.ok(submenu(helperEntries({ kind: 'regex-replacement', panel, known }), 'Regular expression').some(entry => entry.insert === '$1'));
  assert.deepEqual(labels(helperEntries({ kind: 'history', panel, known, history: true, manage: true })), ['Manage history…']);
  const filter = helperEntries({ kind: 'filter', panel, known, history: true, manage: true, autoApply: false });
  assert.deepEqual(labels(filter).slice(-3), ['Manage history…', '—', 'Auto-apply filter']);
  assert.equal(filter.at(-1).checked, false);
});

test('fields helper offers the documented field-list commands', () => {
  const entries = helperEntries({ kind: 'field', panel: [], known: ['TITLE', 'title', 'MOOD', '_FILENAME'], inFieldList: false });
  assert.deepEqual(labels(entries), ['Choose field', '—', 'Remove field', 'Remove all fields…', 'Reset fields…']);
  assert.deepEqual(submenu(entries, 'Choose field').map(entry => entry.label), ['MOOD', 'TITLE', '_FILENAME']);
  assert.equal(entries.find(entry => entry.label === 'Remove field').disabled, true);
  assert.equal(helperEntries({ kind: 'field', panel: [], known: [], inFieldList: true }).find(entry => entry.label === 'Remove field').disabled, false);
});

test('catalogs match the documented placeholder and function inventory', () => {
  const names = informationFields.flatMap(([, list]) => list);
  assert.equal(new Set(names).size, names.length);
  for (const name of ['_bitrate', '_cover_type', '_directory', '_file_mod_datetime_raw', '_counter', '_md5audio', '_playlist_folderpath']) assert.ok(names.includes(name), name);
  const functions = scriptFunctions.flatMap(([, list]) => list).map(signature => signature.slice(1, signature.indexOf('(')).toLowerCase());
  assert.equal(new Set(functions).size, 64, 'the 64 documented scripting functions');
  assert.deepEqual(extendedFields(['TITLE'], ['title', 'mood', 'MOOD', 'x=y']), ['MOOD']);
});

test('every helper function exists in the scripting engine catalog', async () => {
  const script = await readFile(new URL('../crates/tag-core/src/script.rs', import.meta.url), 'utf8');
  const catalog = script.slice(script.indexOf('pub const SUPPORTED_FUNCTIONS'), script.indexOf('];', script.indexOf('pub const SUPPORTED_FUNCTIONS')));
  const supported = new Set([...catalog.matchAll(/"([a-z_0-9]+)"/g)].map(match => match[1]));
  for (const [, list] of scriptFunctions) for (const signature of list) assert.ok(supported.has(signature.slice(1, signature.indexOf('(')).toLowerCase()), signature);
});

test('Ctrl+Space opens the helper menu and Shift+Del removes a history entry', () => {
  const key = (k, modifiers = {}) => ({ key: k, code: k === ' ' ? 'Space' : k, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...modifiers });
  assert.equal(helperKey(key(' ', { ctrlKey: true })), 'open');
  assert.equal(helperKey(key(' ', { ctrlKey: true, altKey: true })), 'open', 'Control+Option+Space on macOS');
  assert.equal(helperKey(key(' ', { metaKey: true })), undefined, '⌘Space belongs to Spotlight');
  assert.equal(helperKey(key(' ')), undefined);
  assert.equal(helperKey(key('Delete', { shiftKey: true })), 'remove-history');
  assert.equal(helperKey(key('Delete')), undefined);
  assert.equal(helperKey(key('Backspace', { shiftKey: true })), undefined, 'Shift+Backspace still deletes text');
});

test('favorites are offered in the ► menu and survive the history limit', () => {
  const history = new FormatHistory(memory());
  history.set('filter', [{ value: '%genre% IS rock', name: 'Rock', favorite: true }, { value: 'x' }]);
  for (let i = 0; i < 40; i++) history.add('filter', `value ${i}`);
  const list = history.list('filter');
  assert.equal(list.length, FormatHistory.limit);
  assert.ok(list.some(entry => entry.name === 'Rock' && entry.favorite), 'favorites are kept when old entries drop out');
  assert.ok(!list.some(entry => entry.value === 'x'));
  assert.deepEqual(history.favorites('filter').map(entry => entry.value), ['%genre% IS rock']);
  const menu = helperEntries({ kind: 'filter', panel: ['TITLE'], known: [], history: true, manage: true, favorites: history.favorites('filter') });
  const tail = labels(menu).slice(-3);
  assert.deepEqual(tail, ['Rock', '—', 'Manage history…']);
  assert.equal(menu.find(entry => entry.label === 'Rock').command, 'set:%genre% IS rock');
  const find = helperEntries({ kind: 'history', panel: [], known: [], history: true, historySelected: true, favorites: [{ value: 'Björk', favorite: true }] });
  assert.deepEqual(labels(find), ['Björk', '—', 'Remove from history', 'Remove all from history…']);
});

test('duplicates are named copies of the selected entry', () => {
  const entries = [{ value: 'a', name: 'First', description: 'd', favorite: true }, { value: 'b', name: 'First copy' }, { value: 'c' }];
  assert.deepEqual(duplicateEntry(entries, 0), { value: 'a', name: 'First copy 2', description: 'd', favorite: true });
  assert.deepEqual(duplicateEntry(entries, 2), { value: 'c' });
  assert.throws(() => duplicateEntry(entries, 5), /Select/);
});

const backend = saved => {
  const calls = [];
  return { calls, load: async () => saved === undefined ? { exists: false, lists: {} } : { exists: true, lists: saved }, save: async lists => { calls.push(structuredClone(lists)); } };
};
const legacy = entries => {
  const data = new Map(Object.entries(entries));
  return { data, get length() { return data.size; }, key: i => [...data.keys()][i] ?? null, getItem: key => data.get(key) ?? null, setItem: (key, value) => data.set(key, value), removeItem: key => data.delete(key) };
};

test('helper lists load from the configuration and save changes there', async () => {
  const store = backend({ 'opentag.history.filter': [{ value: 'one', favorite: true }] });
  const storage = new ConfigListStorage(store, legacy({ 'opentag.history.filter': '[{"value":"stale"}]' }), undefined, 0);
  await storage.ready;
  const history = new FormatHistory(storage);
  assert.deepEqual(history.list('filter'), [{ value: 'one', favorite: true }], 'the configuration wins over old local storage');
  history.add('filter', 'two');
  await new Promise(resolve => setTimeout(resolve, 5));
  assert.deepEqual(store.calls.at(-1), { 'opentag.history.filter': [{ value: 'two' }, { value: 'one', favorite: true }] });
  storage.pause(); history.add('filter', 'three');
  await new Promise(resolve => setTimeout(resolve, 5));
  assert.equal(store.calls.length, 1, 'paused while a configuration is restored');
});

test('lists in local storage move into the configuration once', async () => {
  const store = backend(undefined);
  const old = legacy({ 'opentag.history.tag-filename': '[{"value":"%artist%","name":"A"}]', 'opentag.field-list': '["MOOD"]', 'opentag.panel-width': '300', 'opentag.history.broken': '{x' });
  const storage = new ConfigListStorage(store, old, undefined, 0);
  await storage.ready;
  assert.deepEqual(store.calls, [{ 'opentag.field-list': ['MOOD'], 'opentag.history.tag-filename': [{ value: '%artist%', name: 'A' }] }]);
  assert.deepEqual([...old.data.keys()].sort(), ['opentag.history.broken', 'opentag.panel-width'], 'moved lists leave local storage; other preferences stay');
  assert.deepEqual(new FieldList(storage).list(), ['MOOD']);
  assert.ok(configListKey('opentag.history.find') && !configListKey('opentag.cover.aspect'));
});

test('changes made before the configuration has loaded are kept and saved', async () => {
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const calls = [];
  const storage = new ConfigListStorage({ load: async () => { await gate; return { exists: true, lists: { 'opentag.history.find': [{ value: 'old' }] } }; }, save: async lists => { calls.push(lists); } }, undefined, undefined, 0);
  new FormatHistory(storage).add('find', 'early');
  release(); await storage.ready;
  await new Promise(resolve => setTimeout(resolve, 5));
  assert.deepEqual(calls.at(-1), { 'opentag.history.find': [{ value: 'early' }] });
});

test('helper saves resume after a refused configuration change and retain changes made while paused', async () => {
  const calls=[],errors=[];let fail=true;
  const storage=new ConfigListStorage({load:async()=>({exists:true,lists:{}}),save:async lists=>{calls.push(structuredClone(lists));if(fail)throw new Error('controlled ordinary flush refusal');}},undefined,error=>errors.push(String(error)),0);
  await storage.ready;storage.pause();
  new FormatHistory(storage).add('find','retained');
  await new Promise(resolve=>setTimeout(resolve,5));assert.deepEqual(calls,[]);
  storage.resume();await new Promise(resolve=>setTimeout(resolve,5));assert.equal(errors.length,1);
  storage.pause();fail=false;new FormatHistory(storage).add('find','newer');
  storage.resume();await new Promise(resolve=>setTimeout(resolve,5));
  assert.deepEqual(calls.at(-1),{'opentag.history.find':[{value:'newer'},{value:'retained'}]});
});
