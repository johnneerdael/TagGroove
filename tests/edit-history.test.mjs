import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { historyLabel, editShortcut } = await load('edit-history');
const { copyTags, clipboardText, pastePlan, cutPlan } = await load('tag-clipboard');
const { tagTagFieldOptions, validateTagTarget, tagTagOutcome, tagTagKey } = await load('tag-tag-ui');

const key = (key, modifiers = {}) => ({ key, code: /^[0-9]$/.test(key) ? `Digit${key}` : `Key${key.toUpperCase()}`, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...modifiers });
const idle = { typing: false, textSelected: false, mac: false };

test('history labels describe the operation and file count', () => {
  assert.equal(historyLabel('Undo', { label: 'Save tags', files: 3 }), 'Undo Save tags (3 files)');
  assert.equal(historyLabel('Redo', { label: 'Tag → Filename', files: 1 }), 'Redo Tag → Filename (1 file)');
  assert.equal(historyLabel('Undo', null), 'Undo');
});

test('undo, redo and tag clipboard shortcuts leave text fields and selections native', () => {
  assert.equal(editShortcut(key('z', { ctrlKey: true }), idle), 'undo');
  assert.equal(editShortcut(key('z', { metaKey: true, shiftKey: true }), idle), 'redo');
  assert.equal(editShortcut(key('y', { ctrlKey: true }), idle), 'redo');
  assert.equal(editShortcut(key('c', { ctrlKey: true }), idle), 'copy-tags');
  assert.equal(editShortcut(key('x', { metaKey: true }), idle), 'cut-tags');
  assert.equal(editShortcut(key('v', { ctrlKey: true }), idle), 'paste-tags');
  for (const name of ['z', 'y', 'c', 'x', 'v']) assert.equal(editShortcut(key(name, { ctrlKey: true }), { ...idle, typing: true }), undefined);
  assert.equal(editShortcut(key('c', { ctrlKey: true }), { ...idle, textSelected: true }), undefined);
  assert.equal(editShortcut(key('z', { ctrlKey: true }), { ...idle, textSelected: true }), 'undo');
  assert.equal(editShortcut(key('c'), idle), undefined);
  assert.equal(editShortcut(key('c', { ctrlKey: true, shiftKey: true }), idle), undefined);
});

test('Alt+1 to Alt+5 open the converters in Mp3tag order', () => {
  assert.deepEqual(['1', '2', '3', '4', '5'].map(digit => editShortcut(key(digit, { altKey: true }), idle)), ['tags-filename', 'filename-tags', 'filename-filename', 'text-tags', 'tag-tag']);
  // macOS Option+digit produces characters such as ¡; the physical key still matches.
  assert.equal(editShortcut({ ...key('¡', { altKey: true }), code: 'Digit1' }, { ...idle, mac: true }), 'tags-filename');
  assert.equal(editShortcut({ ...key('¡', { altKey: true }), code: 'Digit1' }, { ...idle, typing: true, mac: true }), undefined);
  assert.equal(editShortcut(key('5', { altKey: true }), { ...idle, typing: true }), 'tag-tag');
  assert.equal(editShortcut(key('6', { altKey: true }), idle), undefined);
  assert.equal(editShortcut(key('1', { altKey: true, shiftKey: true }), idle), undefined);
});

const files = () => [
  { name: 'a.mp3', covers: 1, fields: { TITLE: ['One'], ARTIST: ['A', 'B'], COMMENT: [] } },
  { name: 'b.flac', covers: 0, fields: { title: ['Two'], ALBUM: ['Line\nbreak'] } },
];

test('copy keeps table order, drops removed fields and is independent of the source', () => {
  const sources = files(), clipboard = copyTags(sources);
  sources[0].fields.TITLE[0] = 'Changed';
  assert.deepEqual(clipboard.items.map(item => item.fields), [{ TITLE: ['One'], ARTIST: ['A', 'B'] }, { title: ['Two'], ALBUM: ['Line\nbreak'] }]);
  assert.equal(clipboard.items[0].covers, 1);
  assert.equal(clipboardText(clipboard), 'a.mp3\tARTIST=A \\\\ B\tTITLE=One\nb.flac\tALBUM=Line break\ttitle=Two');
  assert.throws(() => copyTags([]), /Select files/);
});

test('paste replaces tags one to one, or one source to every target, keeping native names', () => {
  const clipboard = copyTags(files());
  const targets = [
    { path: '/x.flac', fields: { title: ['Old'], GENRE: ['Rock'], ARTIST: ['A', 'B'] } },
    { path: '/y.mp3', fields: { TITLE: ['Two'] } },
  ];
  assert.deepEqual(pastePlan(clipboard, targets), [
    { path: '/x.flac', updates: { GENRE: [], title: ['One'] } },
    { path: '/y.mp3', updates: { ALBUM: ['Line\nbreak'] } },
  ]);
  const single = copyTags([files()[0]]);
  assert.deepEqual(pastePlan(single, [{ path: '/1', fields: {} }, { path: '/2', fields: { TITLE: ['One'], ARTIST: ['A', 'B'] } }, { path: '/3', fields: { COMMENT: ['x'] } }]), [
    { path: '/1', updates: { TITLE: ['One'], ARTIST: ['A', 'B'] } },
    { path: '/3', updates: { COMMENT: [], TITLE: ['One'], ARTIST: ['A', 'B'] } },
  ]);
  assert.throws(() => pastePlan(clipboard, [targets[0], targets[1], { path: '/z', fields: {} }]), /holds tags of 2 files/);
  assert.throws(() => pastePlan(undefined, targets), /Copy tags first/);
  assert.throws(() => pastePlan(clipboard, []), /Select files/);
});

test('cut stages removal of every present field', () => {
  assert.deepEqual(cutPlan([{ path: '/a', fields: { TITLE: ['x'], COMMENT: [] } }, { path: '/b', fields: {} }]), [{ path: '/a', updates: { TITLE: [] } }]);
});

test('Tag → Tag field choices, targets and previews follow Format Value rules', () => {
  const options = tagTagFieldOptions(['title', 'MOOD', 'mood', '_PATH', 'BPM']);
  assert.deepEqual(options.slice(-2), ['BPM', 'MOOD']);
  assert.equal(options.filter(name => name.toUpperCase() === 'TITLE').length, 1);
  assert.ok(options.includes('_FILENAME') && options.includes('_DIRECTORY'));
  assert.equal(validateTagTarget(' albumartist '), 'albumartist');
  assert.equal(validateTagTarget('_filename'), '_FILENAME');
  for (const bad of ['', '  ', 'A=B', '_PATH', 'X\nY']) assert.throws(() => validateTagTarget(bad));
  assert.deepEqual(tagTagOutcome('ALBUM', 'Hits'), { label: 'ALBUM', value: 'Hits', removed: false });
  assert.equal(tagTagOutcome('ALBUM', '').removed, true);
  assert.equal(tagTagOutcome('_FILENAME', '01 Song', 'old.name.mp3').value, '01 Song.mp3');
  assert.equal(tagTagOutcome('_FILENAME', '01 Song.MP3', 'old.mp3').value, '01 Song.MP3');
  assert.equal(tagTagOutcome('_FILENAME', '').removed, true);
  assert.equal(tagTagOutcome('_DIRECTORY', 'Album').label, 'New folder name');
  assert.notEqual(tagTagKey({ field: 'A', template: '%b%', paths: ['/1'] }), tagTagKey({ field: 'A', template: '%b%', paths: ['/2'] }));
});
