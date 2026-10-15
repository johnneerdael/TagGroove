import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { tagsPagePrefs, autoSaveTargets, reorderKey, moveSelection, dropSelection, mergeOrder, dropIndex, folderImageFolder, autoSizeShortcut, fitWidth, AUTO_SIZE_ROWS } = await load('list-behaviour');
const { copyTags, pastePlan, pasteCoverPlan, cutCoverPlan, pendingDigests, copiedMessage } = await load('tag-clipboard');
const options = await load('options-ui');
const { headerMarkup, defaultLayout, minColumnWidth, maxColumnWidth } = await load('columns');

const key = (k, o = {}) => ({ key: k, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...o });

test('Options → Tags offers save-on-move and the folder-image switch, both off by default', () => {
  const page = options.pageTree().find(({ page }) => page.id === 'tags').page;
  const paths = page.controls.filter(control => control.kind === 'checkbox').map(control => control.path);
  assert.deepEqual(paths, ['pages.tags.save_on_move', 'pages.tags.no_folder_image']);
  const values = options.pageValues(options.defaultSettings(), 'tags');
  assert.deepEqual(tagsPagePrefs(values), { save_on_move: false, no_folder_image: false });
  assert.deepEqual(tagsPagePrefs({ save_on_move: true, no_folder_image: 'yes' }), { save_on_move: true, no_folder_image: false });
});

test('moving to another file saves only the files left behind that have pending edits', () => {
  const pending = path => path !== 'b';
  assert.deepEqual(autoSaveTargets(['a'], new Set(['b']), pending), ['a']);
  assert.deepEqual(autoSaveTargets(['a', 'b', 'c'], new Set(), pending), ['a', 'c'], 'clicking the empty area');
  assert.deepEqual(autoSaveTargets(['a'], new Set(['a']), pending), [], 'clicking the selected file again');
  assert.deepEqual(autoSaveTargets(['a'], new Set(['a', 'b']), pending), ['a'], 'extending the selection');
  assert.deepEqual(autoSaveTargets(['b'], new Set(['c']), pending), [], 'nothing pending');
  assert.deepEqual(autoSaveTargets([], new Set(['a']), pending), []);
});

test('Alt+arrows and Alt+Page keys are the manual reorder keys', () => {
  assert.equal(reorderKey(key('ArrowUp', { altKey: true })), 'up');
  assert.equal(reorderKey(key('ArrowDown', { altKey: true })), 'down');
  assert.equal(reorderKey(key('PageUp', { altKey: true })), 'top');
  assert.equal(reorderKey(key('PageDown', { altKey: true })), 'bottom');
  assert.equal(reorderKey(key('ArrowUp')), undefined);
  assert.equal(reorderKey(key('ArrowUp', { altKey: true, shiftKey: true })), undefined);
  assert.equal(reorderKey(key('ArrowUp', { altKey: true, ctrlKey: true })), undefined);
  assert.equal(reorderKey(key('ArrowLeft', { altKey: true })), undefined, 'Alt+Left stays directory history');
});

test('selected files move one row, keeping their order and stopping at the edges', () => {
  const order = ['a', 'b', 'c', 'd', 'e'];
  assert.deepEqual(moveSelection(order, new Set(['c']), 'up'), ['a', 'c', 'b', 'd', 'e']);
  assert.deepEqual(moveSelection(order, new Set(['b', 'c']), 'down'), ['a', 'd', 'b', 'c', 'e']);
  assert.deepEqual(moveSelection(order, new Set(['b', 'd']), 'up'), ['b', 'a', 'd', 'c', 'e']);
  assert.deepEqual(moveSelection(order, new Set(['a', 'c']), 'up'), ['a', 'c', 'b', 'd', 'e'], 'a blocked file stays, the others move');
  assert.deepEqual(moveSelection(order, new Set(['e']), 'down'), order);
  assert.deepEqual(moveSelection(order, new Set(['b', 'd']), 'top'), ['b', 'd', 'a', 'c', 'e']);
  assert.deepEqual(moveSelection(order, new Set(['b', 'd']), 'bottom'), ['a', 'c', 'e', 'b', 'd']);
  assert.deepEqual(order, ['a', 'b', 'c', 'd', 'e'], 'the input is not changed');
});

test('Alt+drag drops the dragged files before a row or at the end', () => {
  const order = ['a', 'b', 'c', 'd', 'e'];
  assert.deepEqual(dropSelection(order, new Set(['d', 'b']), 'a'), ['b', 'd', 'a', 'c', 'e']);
  assert.deepEqual(dropSelection(order, new Set(['a']), null), ['b', 'c', 'd', 'e', 'a']);
  assert.deepEqual(dropSelection(order, new Set(['a']), 'd'), ['b', 'c', 'a', 'd', 'e']);
  assert.deepEqual(dropSelection(order, new Set(['b', 'c']), 'c'), order, 'onto a dragged row');
  assert.deepEqual(dropSelection(order, new Set(['b']), 'missing'), order);
  assert.equal(dropIndex(44, 30, 5), 1);
  assert.equal(dropIndex(46, 30, 5), 2);
  assert.equal(dropIndex(-20, 30, 5), 0);
  assert.equal(dropIndex(9000, 30, 5), 5);
});

test('reordering the filtered rows keeps hidden files in their places', () => {
  const full = ['a', 'x', 'b', 'y', 'c'], shown = ['a', 'b', 'c'];
  assert.deepEqual(mergeOrder(full, shown, ['c', 'a', 'b']), ['c', 'x', 'a', 'y', 'b']);
  assert.deepEqual(mergeOrder(full, full, ['y', 'x', 'c', 'b', 'a']), ['y', 'x', 'c', 'b', 'a']);
});

test('the folder image is used only without covers, for files of one folder, unless switched off', () => {
  const none = { disabled: false, provider: false };
  assert.equal(folderImageFolder([{ path: '/music/album/a.mp3', covers: 0 }], '/music/album/a.mp3', none), '/music/album');
  assert.equal(folderImageFolder([{ path: 'C:\\Music\\a.mp3', covers: 0 }, { path: 'C:\\Music\\b.mp3', covers: 0 }], 'C:\\Music\\b.mp3', none), 'C:\\Music');
  assert.equal(folderImageFolder([{ path: '/m/a.mp3', covers: 1 }], '/m/a.mp3', none), undefined, 'embedded art wins');
  assert.equal(folderImageFolder([{ path: '/m/a.mp3', covers: 0 }, { path: '/n/b.mp3', covers: 0 }], '/m/a.mp3', none), undefined, 'mixed folders');
  assert.equal(folderImageFolder([{ path: '/m/a.mp3', covers: 0 }, { path: '/m/b.mp3', covers: 2 }], '/m/a.mp3', none), undefined);
  assert.equal(folderImageFolder([{ path: '/m/a.mp3', covers: 0 }], '/m/a.mp3', { disabled: true, provider: false }), undefined);
  assert.equal(folderImageFolder([{ path: 'content://x', covers: 0 }], 'content://x', { disabled: false, provider: true }), undefined);
  assert.equal(folderImageFolder([], undefined, none), undefined);
});

test('column auto-size: Ctrl/⌘ Num + and widths fitted to the widest value', () => {
  assert.ok(autoSizeShortcut({ key: '+', code: 'NumpadAdd', ctrlKey: true, metaKey: false, altKey: false }));
  assert.ok(autoSizeShortcut({ key: '+', ctrlKey: false, metaKey: true, altKey: false, location: 3 }));
  assert.ok(!autoSizeShortcut({ key: '+', code: 'Equal', ctrlKey: true, metaKey: false, altKey: false, location: 0 }), 'main keyboard plus is zoom');
  assert.ok(!autoSizeShortcut({ key: '+', code: 'NumpadAdd', ctrlKey: false, metaKey: false, altKey: false }));
  const limits = { min: minColumnWidth, max: maxColumnWidth };
  assert.equal(fitWidth([10, 120.2, 80], 18, limits), 139);
  assert.equal(fitWidth([], 4, limits), minColumnWidth);
  assert.equal(fitWidth([NaN, 1e9], 0, limits), maxColumnWidth);
  assert.ok(AUTO_SIZE_ROWS >= 1000);
  assert.match(headerMarkup(defaultLayout().columns.slice(0, 1), []), /data-resize="filename"[^>]*double-click to fit/);
});

const art = (digest, type = 'CoverFront', description = '') => ({ digest, picture_type: type, description, mime: 'image/png', size: 10, width: 1, height: 1 });
const ref = cover => ({ digest: cover.digest, picture_type: cover.picture_type, description: cover.description });

test('Copy tags carries the artwork and Paste tags replaces the covers with it', () => {
  const clipboard = copyTags([{ name: 'a.mp3', fields: { TITLE: ['A'] }, covers: 2, artwork: [art('f'), art('b', 'CoverBack')] }]);
  assert.deepEqual(clipboard.items[0].artwork.map(cover => cover.digest), ['f', 'b']);
  const targets = [{ path: 'x', covers: [] }, { path: 'y', covers: [art('old')] }, { path: 'z', covers: [art('f'), art('b', 'CoverBack')] }];
  assert.deepEqual(pasteCoverPlan(clipboard, targets), [
    { path: 'x', covers: [ref(art('f')), ref(art('b', 'CoverBack'))] },
    { path: 'y', covers: [ref(art('f')), ref(art('b', 'CoverBack'))] },
  ], 'z already has these covers');
  // Fields still paste as before.
  assert.deepEqual(pastePlan(clipboard, [{ path: 'x', fields: {} }]), [{ path: 'x', updates: { TITLE: ['A'] } }]);
});

test('per-file paste, files copied without covers and unknown artwork', () => {
  const clipboard = copyTags([
    { name: '1', fields: {}, covers: 1, artwork: [art('one')] },
    { name: '2', fields: {}, covers: 0, artwork: [] },
    { name: '3', fields: {}, covers: 1 },
  ]);
  const targets = ['p', 'q', 'r'].map(path => ({ path, covers: [art('mine')] }));
  assert.deepEqual(pasteCoverPlan(clipboard, targets), [
    { path: 'p', covers: [ref(art('one'))] },
    { path: 'q', covers: [] },
  ], 'a source without covers removes them; unknown artwork leaves r alone');
  assert.deepEqual(pasteCoverPlan(clipboard, targets.slice(0, 2)), [], 'count mismatch is reported by the field plan');
  assert.deepEqual(pasteCoverPlan(undefined, targets), []);
});

test('Cut tags removes covers; pending digests and status text', () => {
  assert.deepEqual(cutCoverPlan([{ path: 'a', covers: [art('x')] }, { path: 'b', covers: [] }]), [{ path: 'a', covers: [] }]);
  assert.deepEqual(pendingDigests([[ref(art('a')), ref(art('b'))], [ref(art('a'))], []]), ['a', 'b']);
  const clipboard = copyTags([{ name: '1', fields: {}, covers: 2, artwork: [art('a'), art('b')] }, { name: '2', fields: {}, covers: 0, artwork: [] }]);
  assert.equal(copiedMessage(clipboard, false, 0), 'Copied tags of 2 files with 2 covers.');
  assert.equal(copiedMessage(copyTags([{ name: '1', fields: {}, covers: 1 }]), true, 1), 'Cut tags of 1 file. Artwork of 1 file could not be copied.');
});
