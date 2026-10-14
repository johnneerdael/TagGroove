import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/helper-menu.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { FieldList, FormatHistory, defaultFieldList, helperEntries, insertion, extendedFields, informationFields, scriptFunctions, placeholder } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
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
