import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const toolbar = await load('toolbar');
const { lastSource } = await load('tag-sources');
const { shuffleOrder, dropAdds } = await load('list-behaviour');
const { automaticDefaults, typedDirectory, defaultLayout: panelLayout, KEEP, BLANK } = await load('tag-panel');
const { pasteFields, fieldClipText } = await load('tag-edits');
const { headerMarkup, defaultLayout } = await load('columns');
const { PlaylistUI } = await load('playlist-ui');
const { listShortcut } = await load('list-commands');

const key = (k, o = {}) => ({ key: k, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...o });

test('toolbar customization keys: F4, Space, Ctrl+E / Ctrl+D (⌘ on macOS) and Alt+arrows', () => {
  assert.equal(toolbar.toolbarKey(key('F4'), false), 'edit');
  assert.equal(toolbar.toolbarKey(key(' '), false), 'toggle');
  assert.equal(toolbar.toolbarKey(key('e', { ctrlKey: true }), false), 'show-all');
  assert.equal(toolbar.toolbarKey(key('d', { ctrlKey: true }), false), 'hide-all');
  assert.equal(toolbar.toolbarKey(key('e', { metaKey: true }), true), 'show-all');
  assert.equal(toolbar.toolbarKey(key('e', { ctrlKey: true }), true), undefined);
  assert.equal(toolbar.toolbarKey(key('ArrowUp', { altKey: true }), false), 'up');
  assert.equal(toolbar.toolbarKey(key('ArrowDown', { altKey: true }), true), 'down');
  assert.equal(toolbar.toolbarKey(key('PageUp', { altKey: true }), false), 'top');
  assert.equal(toolbar.toolbarKey(key('PageDown', { altKey: true }), false), 'bottom');
  assert.equal(toolbar.toolbarKey(key('ArrowUp'), false), undefined, 'plain arrows move the selection');
  assert.equal(toolbar.toolbarKey(key(' ', { shiftKey: true }), false), undefined);
});

test('toolbar configuration: defaults, normalization, moves and separators', () => {
  const defaults = toolbar.defaultToolbar();
  assert.equal(defaults.shown, true);
  assert.deepEqual(toolbar.visibleItems(defaults.items).filter(item => !toolbar.isSeparator(item.id)).map(item => item.id), ['folder', 'files', 'save', 'reload', 'filename-tags', 'tags-filename', 'groups', 'number']);
  for (const item of defaults.items) if (!toolbar.isSeparator(item.id)) assert.ok(toolbar.TOOLBAR_COMMANDS[item.id].hint, item.id);
  const saved = toolbar.normalizeToolbar({ shown: false, items: [{ id: 'save', visible: true, label: '  Write ' }, { id: 'save', visible: false }, { id: 'nonsense', visible: true }, { id: 'separator-9', visible: true, label: 'x' }] });
  assert.equal(saved.shown, false);
  assert.deepEqual(saved.items.slice(0, 2), [{ id: 'save', visible: true, label: 'Write' }, { id: 'separator-9', visible: true }]);
  assert.ok(saved.items.find(item => item.id === 'folder' && !item.visible), 'commands missing from saved settings are appended hidden');
  assert.equal(toolbar.itemLabel(saved.items[0]), 'Write');
  assert.equal(toolbar.normalizeToolbar(undefined).items.length, defaults.items.length);
  const items = [{ id: 'a', visible: true }, { id: 'b', visible: true }, { id: 'c', visible: true }];
  assert.deepEqual(toolbar.moveToolbarItem(items, 2, 'top'), { items: [items[2], items[0], items[1]], index: 0 });
  assert.deepEqual(toolbar.moveToolbarItem(items, 0, 'down').items.map(item => item.id), ['b', 'a', 'c']);
  assert.equal(toolbar.moveToolbarItem(items, 0, 'up').items, items, 'nothing moves past the edge');
  const sep = id => ({ id, visible: true });
  assert.deepEqual(toolbar.visibleItems([sep('separator-1'), { id: 'save', visible: true }, sep('separator-2'), sep('separator-3'), { id: 'reload', visible: false }, sep('separator-4')]).map(item => item.id), ['save']);
});

test('Ctrl+I remembers any tag source, tags or covers only', () => {
  assert.deepEqual(lastSource(null), { provider: 'musicbrainz', coversOnly: false });
  assert.deepEqual(lastSource('discogs'), { provider: 'discogs', coversOnly: false }, 'older builds stored a provider name');
  assert.deepEqual(lastSource(JSON.stringify({ provider: 'local_freedb', coversOnly: false })), { provider: 'local_freedb', coversOnly: false });
  assert.deepEqual(lastSource(JSON.stringify({ provider: 'discogs', coversOnly: true })), { provider: 'discogs', coversOnly: true });
  assert.deepEqual(lastSource(JSON.stringify({ provider: 'freedb', coversOnly: true })), { provider: 'freedb', coversOnly: false }, 'freedb has no covers');
  assert.deepEqual(lastSource(JSON.stringify({ provider: 'unknown' })), { provider: 'musicbrainz', coversOnly: false });
});

test('random order and drop modes', () => {
  const items = ['a', 'b', 'c', 'd'];
  const shuffled = shuffleOrder(items, () => 0);
  assert.deepEqual(shuffled, ['b', 'c', 'd', 'a']);
  assert.deepEqual([...shuffled].sort(), items);
  assert.deepEqual(items, ['a', 'b', 'c', 'd']);
  assert.deepEqual(shuffleOrder(items, () => 0.999), items);
  assert.equal(dropAdds({ control: false, command: false }, false), false);
  assert.equal(dropAdds({ control: true, command: false }, false), true);
  assert.equal(dropAdds({ control: false, command: true }, false), false, 'the Windows key does not add');
  assert.equal(dropAdds({ control: false, command: true }, true), true, '⌘ on macOS');
  assert.equal(dropAdds({ control: true, command: false }, true), true);
  assert.match(headerMarkup(defaultLayout().columns.slice(0, 2), [], true), /^<th[^>]*data-col="filename"[^>]*><button type="button" class="shuffle-list" data-shuffle/);
  assert.equal(headerMarkup(defaultLayout().columns.slice(0, 2), []).includes('data-shuffle'), false);
  assert.equal(headerMarkup(defaultLayout().columns.slice(0, 2), [], true).match(/data-shuffle/g).length, 1);
});

test('F3 toggles the filter', () => {
  assert.equal(listShortcut(key('F3'), false), 'filter-focus');
  assert.equal(listShortcut(key('F3'), true), 'filter-focus', 'also from the filter itself, to hide it');
});

test('tag panel: automatic default values on multi-select and typed directories', () => {
  const fields = panelLayout().fields.map(field => ({ ...field }));
  const comment = fields.find(field => field.field === 'COMMENT'), genre = fields.find(field => field.field === 'GENRE');
  comment.default_value = BLANK; genre.default_value = 'Rock';
  const title = fields.find(field => field.field === 'TITLE'); title.default_value = KEEP;
  assert.deepEqual(automaticDefaults(fields, () => false), [['GENRE', 'Rock'], ['COMMENT', '']]);
  assert.deepEqual(automaticDefaults(fields, field => field === 'GENRE'), [['COMMENT', '']], 'pending edits are kept');
  genre.visible = false;
  assert.deepEqual(automaticDefaults(fields, () => false), [['COMMENT', '']], 'hidden fields are not filled');
  assert.equal(typedDirectory('  /music/new ', '/music/old'), '/music/new');
  assert.equal(typedDirectory('"C:\\Music"', ''), 'C:\\Music');
  assert.equal(typedDirectory('/music/old', '/music/old'), undefined);
  assert.equal(typedDirectory('   ', '/music/old'), undefined);
});

test('Extended Tags field clipboard: paste replaces listed fields and adds new ones', () => {
  const clip = [{ name: 'ARTIST', values: ['A', 'B'] }, { name: 'MOOD', values: ['calm'] }];
  assert.equal(fieldClipText(clip), 'ARTIST=A\nARTIST=B\nMOOD=calm');
  assert.equal(fieldClipText([{ name: 'EMPTY', values: [] }]), 'EMPTY=');
  assert.deepEqual(pasteFields([{ name: 'title', removed: false }, { name: 'artist', removed: false }, { name: 'MOOD', removed: true }], clip), [{ index: 1, field: clip[0] }, { index: -1, field: clip[1] }]);
});

test('Ctrl+Shift+P writes one playlist of all files into each folder', async () => {
  const calls = [];
  const host = {
    invoke: async (command, args) => { calls.push([command, args]); return { playlists: [{ path: '/a/a.m3u8', files: 1 }, { path: '/b/b.m3u8', files: 1 }], errors: [] }; },
    values: () => ({ filename: '%_directory%.m3u8', confirm_filename: true }), remember: async () => {},
    files: scope => scope === 'all' ? ['/a/1.mp3', '/b/2.mp3'] : ['/a/1.mp3'],
    workingDirectory: () => '/', loadedPlaylist: () => undefined, columns: () => [],
    modal: () => { throw new Error('no dialog expected'); }, modalError: () => {}, run: async (_text, job) => job(),
    done: (plan, interactive) => { host.plan = plan; host.interactive = interactive; },
  };
  await new PlaylistUI(host).createPerFolder();
  assert.equal(calls.length, 1);
  const [command, { request }] = calls[0];
  assert.equal(command, 'create_playlists');
  assert.deepEqual(request.paths, ['/a/1.mp3', '/b/2.mp3'], 'all files, not the selection');
  assert.equal(request.partition_template, '%_folderpath%');
  assert.equal(request.filename_template, '%_directory%.m3u8');
  assert.equal(request.base_directory, null, 'relative names resolve in each folder');
  assert.equal(host.plan.playlists.length, 2);
});
