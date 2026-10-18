import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { listShortcut, invertSelection, nextFile, similarFiles, nextSimilarFiles, formatDuration, formatSize, listSummary } = await load('list-commands');
const { findInList, findMatcher, sortFindResults } = await load('find');
const { adjacentCell, slowEditStillWanted } = await load('cell-editor');
const key = (key, modifiers = {}) => ({ key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...modifiers });

test('documented File List shortcuts map to list commands', () => {
  assert.equal(listShortcut(key('A', { ctrlKey: true, shiftKey: true }), false), 'invert-selection');
  assert.equal(listShortcut(key('a', { metaKey: true }), false), undefined, 'Select all stays with the existing handler');
  assert.equal(listShortcut(key('u', { ctrlKey: true }), false), 'unselect-all');
  assert.equal(listShortcut(key('n', { ctrlKey: true }), false), 'select-next');
  assert.equal(listShortcut(key('g', { ctrlKey: true }), false), 'select-similar');
  assert.equal(listShortcut(key('G', { ctrlKey: true, shiftKey: true }), false), 'select-next-similar');
  assert.equal(listShortcut(key('f', { ctrlKey: true }), true), 'find');
  assert.equal(listShortcut(key('F2'), false), 'edit-cell');
  assert.equal(listShortcut(key('F2'), true), undefined);
  assert.equal(listShortcut(key('Delete'), false), 'remove-from-list');
  assert.equal(listShortcut(key('Delete'), true), undefined, 'Delete inside text fields edits text');
  assert.equal(listShortcut(key('Backspace'), false, true), 'remove-from-list', 'Mac keyboards: ⌫ removes from the list');
  assert.equal(listShortcut(key('Backspace'), true, true), undefined, '⌫ inside text fields edits text');
  assert.equal(listShortcut(key('Backspace'), false, false), undefined, 'Backspace is not Remove on PC keyboards');
  assert.equal(listShortcut(key('Backspace', { metaKey: true }), false, true), undefined, '⌘⌫ stays Delete files');
  assert.equal(listShortcut(key('F5'), true), 'refresh');
  assert.equal(listShortcut(key('F3'), false), 'filter-focus');
  assert.equal(listShortcut(key('u', { ctrlKey: true }), true), undefined);
  assert.equal(listShortcut(key('g', { ctrlKey: true, altKey: true }), false), undefined);
});

test('selection commands follow list order', () => {
  const list = ['a', 'b', 'c', 'd', 'e'];
  assert.deepEqual(invertSelection(list, new Set(['b', 'd', 'x'])), ['a', 'c', 'e']);
  assert.equal(nextFile(list, new Set(['b', 'c'])), 'd');
  assert.equal(nextFile(list, new Set()), 'a');
  assert.equal(nextFile(list, new Set(['e'])), 'e');
  const album = { a: 'X', b: 'X', c: 'Y', d: 'X', e: 'Z' };
  assert.deepEqual(similarFiles(list, path => album[path], 'b'), ['a', 'b', 'd']);
  assert.deepEqual(similarFiles(list, path => album[path], 'missing'), []);
  assert.deepEqual(nextSimilarFiles(list, path => album[path], new Set(['a', 'b'])), ['c']);
  assert.deepEqual(nextSimilarFiles(list, path => album[path], new Set(['c'])), ['a', 'b', 'd']);
  assert.deepEqual(nextSimilarFiles(list, path => album[path], new Set(['e'])), []);
});

test('status summary reports selection, listed totals and active file properties', () => {
  const entry = (duration, size, extra = {}) => ({ duration, size, format: 'MPEG', bitrate: 320, sample_rate: 44100, channels: 2, ...extra });
  assert.equal(formatDuration(65), '1:05');
  assert.equal(formatDuration(3725), '1:02:05');
  assert.equal(formatDuration(90061), '1d 1:01:01');
  assert.deepEqual([formatSize(512), formatSize(1536), formatSize(5 * 1048576), formatSize(3 * 1073741824)], ['512 B', '1.5 KB', '5.0 MB', '3.0 GB']);
  const listed = [entry(60, 1048576, { technical: { _CODEC: ['MPEG 1 Layer 3'] } }), entry(120, 1048576)];
  assert.deepEqual(listSummary([listed[0]], listed, 3, listed[0]), ['1 selected (1:00 | 1.0 MB)', '2 of 3 files (3:00 | 2.0 MB)', 'MPEG 1 Layer 3 · 320 kbps · 44,100 Hz · 2 channels']);
  assert.deepEqual(listSummary([], [], 0), ['Select a file to view its tags']);
});

test('find strategies match tag values and filenames', () => {
  const items = [
    { path: '/a/One.mp3', filename: 'One.mp3', fields: { TITLE: ['Love Song'], ARTIST: ['Band', 'Lovely Guest'], _LENGTH: ['Love'] } },
    { path: '/a/Two.mp3', filename: 'Two love.mp3', fields: { TITLE: ['love'] } },
  ];
  const fields = (strategy, query, caseSensitive = false) => findInList(items, query, strategy, caseSensitive).map(result => `${result.filename}:${result.field}:${result.value}`);
  assert.deepEqual(fields('contains', 'love'), ['One.mp3:TITLE:Love Song', 'One.mp3:ARTIST:Lovely Guest', 'Two love.mp3:Filename:Two love.mp3', 'Two love.mp3:TITLE:love']);
  assert.deepEqual(fields('contains', 'love', true), ['Two love.mp3:Filename:Two love.mp3', 'Two love.mp3:TITLE:love']);
  assert.deepEqual(fields('word', 'love'), ['One.mp3:TITLE:Love Song', 'Two love.mp3:Filename:Two love.mp3', 'Two love.mp3:TITLE:love']);
  assert.deepEqual(fields('content', 'LOVE'), ['Two love.mp3:TITLE:love']);
  assert.deepEqual(fields('begins', 'lov'), ['One.mp3:TITLE:Love Song', 'One.mp3:ARTIST:Lovely Guest', 'Two love.mp3:TITLE:love']);
  assert.deepEqual(fields('ends', '.MP3'), ['One.mp3:Filename:One.mp3', 'Two love.mp3:Filename:Two love.mp3']);
  assert.throws(() => findMatcher('(', 'regex', false), /native matcher/);
  assert.throws(() => findMatcher('', 'contains', false), /Enter text/);
  const results = findInList(items, 'love', 'contains', false);
  assert.deepEqual(sortFindResults(results, { key: 'field', ascending: true }).map(result => result.field), ['ARTIST', 'Filename', 'TITLE', 'TITLE']);
  assert.deepEqual(sortFindResults(results, { key: 'filename', ascending: false }).map(result => result.filename)[0], 'Two love.mp3');
});

test('Tab navigation moves across editable cells and wraps to adjacent rows', () => {
  const rows = ['a', 'b'], columns = ['filename', 'title', 'artist'];
  assert.deepEqual(adjacentCell(rows, columns, { path: 'a', column: 'title' }, 1), { path: 'a', column: 'artist' });
  assert.deepEqual(adjacentCell(rows, columns, { path: 'a', column: 'artist' }, 1), { path: 'b', column: 'filename' });
  assert.deepEqual(adjacentCell(rows, columns, { path: 'b', column: 'filename' }, -1), { path: 'a', column: 'artist' });
  assert.equal(adjacentCell(rows, columns, { path: 'b', column: 'artist' }, 1), undefined);
  assert.equal(adjacentCell(rows, [], { path: 'a', column: 'title' }, 1), undefined);
  assert.equal(adjacentCell(rows, columns, { path: 'gone', column: 'title' }, 1), undefined);
});

test('a delayed second click edits only while the clicked row keeps the focus', () => {
  const target = { path: 'a', column: 'filename' }, selected = new Set(['a']);
  assert.equal(slowEditStillWanted(target, { selected, busy: false, focusedRow: 'a' }), true);
  // Focus moved to the tag panel (or anything outside the row) in between.
  assert.equal(slowEditStillWanted(target, { selected, busy: false, focusedRow: undefined }), false);
  assert.equal(slowEditStillWanted(target, { selected, busy: false, focusedRow: 'b' }), false);
  assert.equal(slowEditStillWanted(target, { selected, busy: true, focusedRow: 'a' }), false);
  assert.equal(slowEditStillWanted(target, { selected: new Set(['a', 'b']), busy: false, focusedRow: 'a' }), false);
  assert.equal(slowEditStillWanted(target, { selected: new Set(['b']), busy: false, focusedRow: 'a' }), false);
});
