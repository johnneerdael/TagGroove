import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/file-ops.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {parentDirectory, DirectoryHistory, fileShortcut, nameList, transferStatus, launchDirectory, FileOpsUI, defaultDirectoryState, startupChoice, startupOptionsPage} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

test('parent directories stop at filesystem and drive roots', () => {
  assert.equal(parentDirectory('/music/rock/album'), '/music/rock');
  assert.equal(parentDirectory('/music/rock/'), '/music');
  assert.equal(parentDirectory('/music'), '/');
  assert.equal(parentDirectory('/'), undefined);
  assert.equal(parentDirectory('C:\\Music\\Rock'), 'C:\\Music');
  assert.equal(parentDirectory('C:\\Music'), 'C:\\');
  assert.equal(parentDirectory('C:\\'), undefined);
  assert.equal(parentDirectory('relative'), undefined);
});

test('directory history moves back and forward and drops forward entries on visit', () => {
  const history = new DirectoryHistory();
  history.visit('/a'); history.visit('/b'); history.visit('/b'); history.visit('/c');
  assert.equal(history.canForward, false);
  assert.equal(history.move('back'), '/b');
  assert.equal(history.move('back'), '/a');
  assert.equal(history.canBack, false);
  assert.equal(history.move('back'), undefined);
  assert.equal(history.current, '/a');
  assert.equal(history.peek('forward'), '/b');
  assert.equal(history.move('forward'), '/b');
  history.visit('/d');
  assert.equal(history.canForward, false);
  assert.equal(history.move('back'), '/b');
});

const key = (key, modifiers = {}) => ({key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...modifiers});
test('shortcuts follow Mp3tag on Windows/Linux and avoid reserved macOS commands', () => {
  assert.equal(fileShortcut(key('d', {ctrlKey: true}), false, false), 'folder');
  assert.equal(fileShortcut(key('D', {ctrlKey: true, shiftKey: true}), false, false), 'dir-up');
  assert.equal(fileShortcut(key('h', {ctrlKey: true}), false, false), 'dir-add');
  assert.equal(fileShortcut(key('m', {ctrlKey: true}), false, false), 'dir-favorite');
  assert.equal(fileShortcut(key('Delete', {ctrlKey: true}), false, false), 'file-delete');
  assert.equal(fileShortcut(key('C', {ctrlKey: true, shiftKey: true}), false, false), 'file-copy');
  assert.equal(fileShortcut(key('V', {ctrlKey: true, shiftKey: true}), false, false), 'file-move');
  assert.equal(fileShortcut(key('T', {ctrlKey: true, shiftKey: true}), false, false), 'file-play');
  assert.equal(fileShortcut(key('F4'), false, false), 'file-play');
  assert.equal(fileShortcut(key('ArrowLeft', {altKey: true}), false, false), 'dir-back');
  assert.equal(fileShortcut(key('ArrowRight', {altKey: true}), false, false), 'dir-forward');
  // Text editing keeps its own copy, paste, delete and word-navigation keys.
  for (const event of [key('C', {ctrlKey: true, shiftKey: true}), key('V', {ctrlKey: true, shiftKey: true}), key('Delete', {ctrlKey: true}), key('ArrowLeft', {altKey: true})])
    assert.equal(fileShortcut(event, true, false), undefined);
  assert.equal(fileShortcut(key('d', {ctrlKey: true}), true, false), 'folder');
  // Plain keys and unrelated modifiers do nothing.
  assert.equal(fileShortcut(key('Delete'), false, false), undefined);
  assert.equal(fileShortcut(key('d', {metaKey: true}), false, false), undefined);
  assert.equal(fileShortcut(key('s', {ctrlKey: true}), false, false), undefined);
});

test('macOS uses Command except for system-reserved Hide and Minimize', () => {
  assert.equal(fileShortcut(key('d', {metaKey: true}), false, true), 'folder');
  assert.equal(fileShortcut(key('d', {ctrlKey: true}), false, true), undefined);
  assert.equal(fileShortcut(key('Backspace', {metaKey: true}), false, true), 'file-delete');
  assert.equal(fileShortcut(key('h', {ctrlKey: true}), false, true), 'dir-add');
  assert.equal(fileShortcut(key('m', {ctrlKey: true}), false, true), 'dir-favorite');
  assert.equal(fileShortcut(key('h', {ctrlKey: true}), true, true), undefined);
  assert.equal(fileShortcut(key('h', {metaKey: true}), false, true), undefined);
});

test('summaries bound long selections and report errors', () => {
  assert.equal(nameList(['a', 'b']), 'a, b');
  assert.equal(nameList(['1', '2', '3', '4', '5', '6', '7']), '1, 2, 3, 4, 5 and 2 more');
  assert.equal(transferStatus({entries: [], errors: [], renamed: [], copied: [{from: 'a', to: 'b'}]}, 'copy', '/x'), 'Copied 1 file to /x');
  assert.equal(transferStatus({entries: [], errors: [{path: 'a', error: 'e'}], renamed: [], copied: []}, 'move', '/x'), 'Moved 0 files to /x, 1 error');
  assert.equal(launchDirectory({directories: [], files: ['/m/a.mp3'], add: false}), '/m');
  assert.equal(launchDirectory({directories: ['/d'], files: ['/m/a.mp3'], add: false}), '/d');
});

function harness(overrides = {}) {
  const calls = [], state = {...defaultDirectoryState(), favorites: ['/fav']};
  const host = {
    mac: false, selected: [{path: '/m/a.mp3', name: 'a.mp3'}, {path: '/m/b.mp3', name: 'b.mp3'}], pending: 0, answers: [], picks: [], loads: [],
    invoke: async (command, args) => {
      calls.push([command, args]);
      if (command === 'load_directory_state') return structuredClone(state);
      if (command === 'remember_directories') return {...state, recent: args.paths};
      if (command === 'save_directory_state') return args.state;
      if (command === 'take_launch_requests') return host.launches || [];
      if (command === 'trash_files') return {removed: ['/m/a.mp3'], errors: [{path: '/m/b.mp3', error: 'locked'}]};
      if (command === 'transfer_files') return {entries: [], errors: [], renamed: args.mode === 'move' ? args.paths.map(from => ({from, to: `/dest/${from.split('/').pop()}`})) : [], copied: args.mode === 'copy' ? [{from: '/m/a.mp3', to: '/dest/a.mp3'}] : []};
    },
    pickDirectories: async () => host.picks.shift() || [],
    confirm: async message => { host.confirmed = message; return host.answers.length ? host.answers.shift() : true; },
    selection: () => host.selected, pendingCount: () => host.pending,
    load: async (paths, options) => { host.loads.push([paths, options]); return host.loadResult ?? true; },
    removeEntries: paths => { host.removed = paths; }, moved: (result, status) => { host.movedResult = result; host.status = status; },
    run: async (_message, job) => job(), report: (errors, status) => { host.errors = errors; host.status = status; },
    openReason: () => '', changed: () => {}, prefs: {subfolders: true, startup: 'empty'}, preferences: () => host.prefs,
    ...overrides,
  };
  return {ui: new FileOpsUI(host), host, calls};
}

test('change directory replaces the list, records history and asks before discarding drafts', async () => {
  const {ui, host, calls} = harness();
  host.picks.push(['/music/rock']);
  await ui.operate('folder');
  assert.deepEqual(host.loads[0], [['/music/rock'], {replace: true, recursive: true, location: '/music/rock'}]);
  assert.equal(ui.history.current, '/music/rock');
  assert.deepEqual(calls.at(-1), ['remember_directories', {paths: ['/music/rock']}]);
  host.pending = 2; host.answers.push(false);
  await ui.operate('folder');
  assert.match(host.confirmed, /discard pending tag changes in 2 files/);
  assert.equal(host.loads.length, 1);
  await ui.operate('dir-up');
  assert.equal(host.loads[1][0][0], '/music');
  assert.equal(ui.history.current, '/music');
  await ui.operate('dir-back');
  assert.equal(ui.history.current, '/music/rock');
  assert.equal(ui.reason('dir-forward'), '');
});

test('add directory appends without changing the working directory', async () => {
  const {ui, host} = harness();
  host.picks.push(['/a', '/b']);
  await ui.operate('dir-add');
  assert.deepEqual(host.loads[0][1], {replace: false, recursive: true, location: '2 added locations'});
  assert.equal(ui.history.current, undefined);
  assert.equal(ui.reason('dir-up'), 'Change to a directory first.');
});

test('a failed scan keeps the previous working directory', async () => {
  const {ui, host} = harness({loadResult: false});
  host.loadResult = false; host.picks.push(['/broken']);
  await ui.operate('folder');
  assert.equal(ui.history.current, undefined);
});

test('delete confirms, trashes and reports per-file failures', async () => {
  const {ui, host, calls} = harness();
  host.answers.push(false);
  await ui.operate('file-delete');
  assert.equal(calls.length, 0);
  assert.match(host.confirmed, /Move 2 files to the Recycle Bin or trash\?\n\na\.mp3, b\.mp3/);
  await ui.operate('file-delete');
  assert.deepEqual(calls[0], ['trash_files', {paths: ['/m/a.mp3', '/m/b.mp3']}]);
  assert.deepEqual(host.removed, ['/m/a.mp3']);
  assert.equal(host.errors.length, 1);
  assert.match(host.status, /Moved 1 file to the trash, 1 error/);
});

test('copy and move ask for a destination and hand moves to the re-keying hook', async () => {
  const {ui, host, calls} = harness();
  await ui.operate('file-copy');
  assert.equal(calls.length, 0, 'cancelled destination picker does nothing');
  host.picks.push(['/dest']);
  await ui.operate('file-copy');
  assert.deepEqual(calls[0], ['transfer_files', {paths: ['/m/a.mp3', '/m/b.mp3'], destination: '/dest', mode: 'copy'}]);
  assert.equal(host.status, 'Copied 1 file to /dest');
  host.picks.push(['/dest']);
  await ui.operate('file-move');
  assert.match(host.confirmed, /Existing files are never replaced/);
  assert.equal(host.movedResult.renamed.length, 2);
  assert.equal(host.status, 'Moved 2 files to /dest');
});

test('selection-dependent operations explain why they are unavailable', () => {
  const {ui, host} = harness();
  host.selected = [];
  for (const op of ['file-delete', 'file-copy', 'file-move', 'file-play']) assert.equal(ui.reason(op), 'Select files first.');
  assert.match(ui.reason('file-reveal'), /change to a directory/);
  ui.history.visit('/m');
  assert.equal(ui.reason('file-reveal'), '');
});

test('startup opens queued launch requests before the startup directory', async () => {
  const {ui, host} = harness();
  host.launches = [{directories: [], files: ['/m/a.mp3'], add: false}, {directories: ['/x'], files: [], add: true}];
  await ui.initialize();
  assert.deepEqual(host.loads.map(([paths, options]) => [paths, options.replace, options.location]), [[['/m/a.mp3'], true, '/m'], [['/x'], false, '/x']]);
  assert.equal(ui.history.current, '/m');

  const second = harness();
  await second.ui.initialize();
  assert.equal(second.host.loads.length, 0, 'empty startup keeps the list empty');
  const third = harness();
  third.host.prefs = {subfolders: true, startup: 'favorite'};
  await third.ui.initialize();
  assert.deepEqual(third.host.loads[0][0], ['/fav']);
});

test('the Options read-subfolders setting feeds directory scans', async () => {
  const {ui, host} = harness();
  host.prefs = {subfolders: false, startup: 'empty'};
  host.picks.push(['/m']);
  await ui.operate('folder');
  assert.equal(host.loads[0][1].recursive, false);
});

test('the startup choice is an Options page under Directories with a safe default', () => {
  assert.equal(startupOptionsPage.parent, 'directories');
  assert.equal(startupOptionsPage.controls[0].path, 'pages.directories-startup.startup');
  assert.deepEqual(startupOptionsPage.controls[0].choices.map(([value]) => value), ['empty', 'previous', 'favorite']);
  assert.equal(startupChoice(undefined), 'empty');
  assert.equal(startupChoice('bogus'), 'empty');
  assert.equal(startupChoice('previous'), 'previous');
});
