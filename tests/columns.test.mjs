import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { defaultLayout, editKey, normalizeLayout, layoutProblem, toggleColumn, moveColumn, resizeColumn, sortClick, sortIndicator, numericValue, sortRows, columnFields, quickValue, ColumnValueCache, headerMarkup, uniqueColumnId, visibleColumns } = await load('columns');
const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

test('default layout approximates the Mp3tag column set with a visible subset', () => {
  const layout = defaultLayout();
  assert.equal(layoutProblem(layout), '');
  const names = layout.columns.map(column => column.name);
  for (const name of ['Filename', 'Path', 'Title', 'Artist', 'Album', 'Track', 'Year', 'Genre', 'Comment', 'Album Artist', 'Composer', 'Discnumber', 'Codec', 'Bitrate', 'Length', 'Tag']) assert.ok(names.includes(name), name);
  assert.deepEqual(visibleColumns(layout).slice(0, 3).map(column => column.id), ['filename', 'title', 'artist']);
  assert.ok(layout.columns.some(column => !column.visible));
  assert.equal(layout.columns.find(column => column.id === 'length').sort_by, '%_length_seconds%');
  assert.ok(layout.columns.filter(column => column.numeric).every(column => column.align === 'right'));
});

test('only tag fields and filename placeholders are editable', () => {
  assert.equal(editKey({ field: '%artist%' }), 'ARTIST');
  assert.equal(editKey({ field: ' %Album Artist% ' }), 'ALBUM ARTIST');
  assert.equal(editKey({ field: '%_filename_ext%' }), '_FILENAME_EXT');
  assert.equal(editKey({ field: '%_FILENAME%' }), '_FILENAME');
  for (const field of ['', '%_length%', 'artist', '$upper(%artist%)', '%artist%%title%']) assert.equal(editKey({ field }), '', field);
});

test('normalizing fills omitted properties, drops unknown sort keys and rejects unusable layouts', () => {
  const layout = normalizeLayout({ version: 1, columns: [{ id: 'a', name: 'A', value: '%a%', width: 5 }, { id: 'b', name: 'B', value: '%b%', width: 99999, align: 'diagonal', visible: false }], sort: [{ id: 'b', ascending: false }, { id: 'missing', ascending: true }, { id: 'b', ascending: true }] });
  assert.deepEqual(layout.columns.map(column => [column.width, column.align, column.visible, column.field, column.sort_by]), [[24, 'left', true, '', ''], [4000, 'left', false, '', '']]);
  assert.deepEqual(layout.sort, [{ id: 'b', ascending: false }]);
  assert.throws(() => normalizeLayout({ columns: [{ id: 'a', name: 'A', value: '%a%', visible: false }] }), /visible/);
  assert.throws(() => normalizeLayout({ columns: [{ id: 'a', name: 'A', value: '%a%' }, { id: 'A', name: 'B', value: '%b%' }] }), /unique/);
  assert.throws(() => normalizeLayout({ columns: [{ id: 'a', name: 'A', value: '%a%', field: '%_bitrate%' }] }), /field/);
  assert.throws(() => normalizeLayout({ columns: [{ id: 'a', name: ' ', value: '%a%' }] }), /name/);
  assert.throws(() => normalizeLayout(null), /not valid/);
});

test('column edits keep the source layout and the last visible column', () => {
  const layout = defaultLayout();
  const hidden = toggleColumn(layout, 'title');
  assert.equal(hidden.columns.find(column => column.id === 'title').visible, false);
  assert.equal(layout.columns.find(column => column.id === 'title').visible, true);
  assert.equal(toggleColumn(hidden, 'title', true).columns.find(column => column.id === 'title').visible, true);
  const single = normalizeLayout({ columns: [{ id: 'a', name: 'A', value: '%a%' }, { id: 'b', name: 'B', value: '%b%', visible: false }] });
  assert.throws(() => toggleColumn(single, 'a'), /visible/);
  const moved = moveColumn(layout, 'filename', 'artist', true);
  assert.deepEqual(moved.columns.slice(0, 3).map(column => column.id), ['title', 'artist', 'filename']);
  assert.deepEqual(moveColumn(layout, 'artist', 'filename', false).columns.slice(0, 3).map(column => column.id), ['artist', 'filename', 'title']);
  assert.equal(resizeColumn(layout, 'title', 3).columns[1].width, 24);
  assert.equal(resizeColumn(layout, 'title', 301.6).columns[1].width, 302);
  assert.equal(uniqueColumnId(layout, 'title'), 'title-2');
});

test('header clicks replace, reverse and extend sort criteria', () => {
  let sort = [{ id: 'filename', ascending: true }];
  sort = sortClick(sort, 'artist', false);
  assert.deepEqual(sort, [{ id: 'artist', ascending: true }]);
  sort = sortClick(sort, 'artist', false);
  assert.deepEqual(sort, [{ id: 'artist', ascending: false }]);
  sort = sortClick(sort, 'track', true);
  assert.deepEqual(sort, [{ id: 'artist', ascending: false }, { id: 'track', ascending: true }]);
  sort = sortClick(sort, 'track', true);
  assert.deepEqual(sort[1], { id: 'track', ascending: false });
  assert.deepEqual(sortClick(sort, 'track', false), [{ id: 'track', ascending: true }]);
  assert.equal(sortIndicator(sort, 'track'), ' ↓<sup>2</sup>');
  assert.equal(sortIndicator([{ id: 'a', ascending: true }], 'a'), ' ↑');
  assert.match(headerMarkup(defaultLayout().columns.slice(0, 2), sort), /data-sort="filename".*data-resize="filename"/);
});

test('numeric sorting uses leading numbers and keeps ties stable', () => {
  assert.equal(numericValue('03/12'), 3);
  assert.equal(numericValue('1,5'), 1.5);
  assert.equal(numericValue(''), Number.NEGATIVE_INFINITY);
  const rows = [['b', '10', 'x'], ['a', '9', 'x'], ['c', '', 'y'], ['d', '9', 'x']].map(([item, track, album]) => ({ item, keys: [album, track] }));
  assert.deepEqual(sortRows(rows, [{ ascending: true, numeric: false }, { ascending: true, numeric: true }], collator), ['a', 'd', 'b', 'c']);
  assert.deepEqual(sortRows(rows, [{ ascending: true, numeric: false }, { ascending: false, numeric: true }], collator), ['b', 'a', 'd', 'c']);
  const decimals = [['x', '1.5'], ['y', '1.25']].map(([item, value]) => ({ item, keys: [value] }));
  assert.deepEqual(sortRows(decimals, [{ ascending: true, numeric: true }], collator), ['y', 'x']);
  assert.deepEqual(sortRows(decimals, [{ ascending: true, numeric: false }], collator), ['x', 'y'], 'text sorting compares digit runs');
});

test('column fields answer Vorbis track and year placeholders like the tag panel', () => {
  const fields = columnFields({ TRACKNUMBER: ['4'], DATE: ['1999'], Title: ['Song'] });
  assert.deepEqual([fields.TRACK, fields.YEAR], [['4'], ['1999']]);
  assert.deepEqual(columnFields({ TRACK: ['1'], TRACKNUMBER: ['2'] }).TRACK, ['1']);
  assert.equal(quickValue('%title%', fields), 'Song');
  assert.equal(quickValue('%missing%', fields), '');
  assert.equal(quickValue('$num(%track%,2)', fields), undefined);
});

test('value cache batches evaluation, keeps stale values while refreshing and reevaluates changed rows', async () => {
  const tokens = new Map([['a', [1]], ['b', [1]]]), calls = [];
  let updates = 0, value = 'one';
  const cache = new ColumnValueCache({
    token: path => tokens.get(path), templates: () => ['%t%', '%u%'],
    evaluate: async (paths, templates) => { calls.push([paths, templates]); return paths.map(path => templates.map(template => `${path}${template}${value}`)); },
    updated: () => updates++,
  });
  assert.equal(cache.get('a', '%t%'), undefined);
  assert.equal(cache.get('b', '%u%'), undefined);
  assert.equal(cache.get('gone', '%t%'), undefined);
  await new Promise(resolve => setTimeout(resolve));
  assert.deepEqual(calls, [[['a', 'b'], ['%t%', '%u%']]]);
  assert.equal(cache.get('a', '%t%'), 'a%t%one');
  assert.equal(cache.fresh('b', '%u%'), 'b%u%one');
  assert.equal(updates, 1);
  value = 'two'; tokens.set('a', [2]);
  assert.equal(cache.fresh('a', '%t%'), undefined, 'changed rows are not fresh');
  assert.equal(cache.get('a', '%t%'), 'a%t%one', 'stale value remains visible');
  assert.equal(cache.get('a', '%other%'), undefined);
  await new Promise(resolve => setTimeout(resolve));
  assert.deepEqual(calls[1], [['a'], ['%t%', '%u%']]);
  assert.equal(cache.get('a', '%t%'), 'a%t%two');
  assert.equal(calls.length, 2, 'inactive templates are not requested repeatedly');
  cache.retain(path => path === 'b');
  assert.equal(cache.fresh('a', '%t%'), undefined);
});

test('value cache reports failures without retrying forever', async () => {
  let calls = 0, failure;
  const cache = new ColumnValueCache({ token: () => [1], templates: () => ['%t%'], evaluate: async () => { calls++; throw new Error('backend down'); }, updated: () => {}, failed: error => { failure = error; } });
  cache.get('a', '%t%');
  await new Promise(resolve => setTimeout(resolve));
  assert.equal(cache.get('a', '%t%'), '');
  await new Promise(resolve => setTimeout(resolve));
  assert.equal(calls, 1);
  assert.match(String(failure), /backend down/);
});

test('sorting 1000 rows with cached values stays well within a frame budget', () => {
  const rows = Array.from({ length: 1000 }, (_, index) => ({ item: index, keys: [`Artist ${index % 37}`, String(index % 12 + 1)] }));
  const started = performance.now();
  for (let run = 0; run < 10; run++) sortRows(rows, [{ ascending: true, numeric: false }, { ascending: true, numeric: true }], collator);
  const average = (performance.now() - started) / 10;
  assert.ok(average < 50, `${average}ms`);
});
