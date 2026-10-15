import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const dock = await load('panel-dock');
const toolbar = await load('toolbar');
const { TagPanel, defaultSettings, SEPARATOR } = await load('tag-panel');

test('saved layouts are completed and clamped; an older device width fills a missing one', () => {
  assert.deepEqual(dock.normalizeMainLayout(undefined), dock.defaultMainLayout());
  assert.equal(dock.normalizeMainLayout(undefined, 333).width, 333);
  assert.equal(dock.normalizeMainLayout({ width: 300 }, 333).width, 300);
  const layout = dock.normalizeMainLayout({ dock: 'bottom', width: 5000, height: 20, filter: 'toolbar-right', float: { x: 10.4, y: 'top', width: 50 } });
  assert.deepEqual([layout.dock, layout.width, layout.height, layout.filter, layout.last_dock], ['bottom', 640, 120, 'toolbar-right', 'bottom']);
  assert.deepEqual(layout.float, { x: 10, y: 120, width: 220, height: 520 });
  const unknown = dock.normalizeMainLayout({ dock: 'diagonal', filter: 'status-bar', last_dock: 'float' });
  assert.deepEqual([unknown.dock, unknown.filter, unknown.last_dock], ['left', 'list', 'left']);
  assert.equal(dock.normalizeMainLayout({ dock: 'float', last_dock: 'right' }).last_dock, 'right');
});

test('floating remembers the docked side; double-clicking the title bar toggles between them', () => {
  let layout = dock.withDock(dock.defaultMainLayout(), 'top');
  assert.deepEqual([layout.dock, layout.last_dock], ['top', 'top']);
  layout = dock.withDock(layout, 'float');
  assert.deepEqual([layout.dock, layout.last_dock], ['float', 'top']);
  assert.equal(dock.toggledDock(layout), 'top');
  assert.equal(dock.withDock(layout, 'float').last_dock, 'top');
  assert.equal(dock.toggledDock(dock.withDock(layout, 'right')), 'float');
  assert.ok(dock.horizontalDock('top') && dock.horizontalDock('bottom') && !dock.horizontalDock('left') && !dock.horizontalDock('float'));
});

test('a floating panel stays reachable inside the window', () => {
  const view = { width: 1000, height: 700 };
  assert.deepEqual(dock.fitFloat({ x: 100, y: 100, width: 300, height: 400 }, view), { x: 100, y: 100, width: 300, height: 400 });
  assert.deepEqual(dock.fitFloat({ x: 5000, y: -50, width: 300, height: 400 }, view), { x: 880, y: 0, width: 300, height: 400 });
  assert.deepEqual(dock.fitFloat({ x: 0, y: 690, width: 3000, height: 3000 }, view), { x: 0, y: 660, width: 1000, height: 700 });
  assert.deepEqual(dock.fitFloat({ x: 0, y: 0, width: 10, height: 10 }, view), { x: 0, y: 0, width: 220, height: 160 });
});

test('dropping near an edge docks there; elsewhere the panel floats', () => {
  const area = { left: 100, top: 50, width: 800, height: 600 };
  assert.equal(dock.dropTarget({ x: 120, y: 300 }, area, 120), 'left');
  assert.equal(dock.dropTarget({ x: 880, y: 300 }, area, 120), 'right');
  assert.equal(dock.dropTarget({ x: 500, y: 60 }, area, 120), 'top');
  assert.equal(dock.dropTarget({ x: 500, y: 640 }, area, 120), 'bottom');
  assert.equal(dock.dropTarget({ x: 500, y: 350 }, area, 120), 'float');
  assert.equal(dock.dropTarget({ x: 20, y: 300 }, area, 120), 'float', 'outside the workspace');
  assert.equal(dock.dropTarget({ x: 130, y: 70 }, area, 120), 'top', 'the nearest edge wins in a corner');
});

test('splitters grow the panel toward the file list and stay within limits', () => {
  const base = dock.defaultMainLayout();
  assert.equal(dock.resized({ ...base, dock: 'left' }, 264, 40).width, 304);
  assert.equal(dock.resized({ ...base, dock: 'right' }, 264, 40).width, 224);
  assert.equal(dock.resized({ ...base, dock: 'right' }, 264, -1000).width, 640);
  assert.equal(dock.resized({ ...base, dock: 'top' }, 230, 30).height, 260);
  assert.equal(dock.resized({ ...base, dock: 'bottom' }, 230, 30).height, 200);
  assert.equal(dock.resized({ ...base, dock: 'bottom' }, 230, 900).height, 120);
  assert.deepEqual(dock.resized({ ...base, dock: 'float' }, 264, 40), { width: base.width, height: base.height });
});

test('a horizontal panel fills columns as tall as the panel; separators start a new column', () => {
  const field = (span, height = 52) => ({ span, height, separator: false });
  const separator = { span: 12, height: 0, separator: true };
  // Title, Artist, Album full width; Year and Track share a row; Genre; Comment (multiline).
  const items = [field(12), field(12), field(12), field(6), field(6), field(12), field(12, 81)];
  assert.deepEqual(dock.panelColumns(items, 160), [[0, 1, 2], [3, 4, 5], [6]]);
  assert.deepEqual(dock.panelColumns(items, 1000), [[0, 1, 2, 3, 4, 5, 6]]);
  assert.deepEqual(dock.panelColumns([field(12), separator, field(6), field(6), separator, separator, field(12)], 1000), [[0], [2, 3], [6]]);
  assert.deepEqual(dock.panelColumns([separator, field(12)], 100), [[1]]);
  // A field taller than the panel still gets a column of its own.
  assert.deepEqual(dock.panelColumns([field(12, 400), field(12)], 100), [[0], [1]]);
});

test('the tag panel renders columns when its host asks for them', () => {
  const settings = defaultSettings();
  settings.panel.fields.splice(3, 0, { name: '', field: SEPARATOR, type: 'text', size: 'full', rows: 3, default_value: '< keep >', visible: true });
  let horizontal = false;
  const host = { values: () => [], disabled: () => false, directories: () => ({ selected: [], all: [] }), columns: items => horizontal ? dock.panelColumns(items, 1000) : undefined };
  const panel = new TagPanel(host);
  panel.settings = settings;
  const vertical = panel.fieldsHTML();
  assert.match(vertical, /^<div class="panel-grid">/);
  assert.match(vertical, /panel-separator/);
  horizontal = true;
  const columns = panel.fieldsHTML();
  assert.match(columns, /^<div class="panel-columns"><div class="panel-grid">/);
  assert.equal(columns.match(/class="panel-grid"/g).length, 2);
  assert.doesNotMatch(columns, /panel-separator/);
  // Field indexes stay those of the configuration, so Alt shortcuts and drafts still match.
  assert.match(columns, /data-panel-index="4"/);
});

test('the dock choices and the Mp3tag import appear in the panel menu when the host offers them', () => {
  const host = { values: () => [], disabled: () => false, directories: () => ({ selected: [], all: [] }) };
  const plain = new TagPanel(host).titleHTML();
  assert.doesNotMatch(plain, /dock:|import-mp3tag/);
  const menu = new TagPanel({ ...host, importMp3tag: async () => ({}), dock: { choices: dock.DOCKS, current: () => 'right', set: () => {} } }).titleHTML();
  assert.match(menu, /data-panel-command="import-mp3tag"/);
  for (const [side] of dock.DOCKS) assert.match(menu, new RegExp(`data-panel-command="dock:${side}"`));
  assert.match(menu, /data-panel-command="dock:right" role="menuitemcheckbox" aria-checked="true"/);
});

test('main toolbar keeps spacers; the custom toolbar takes commands, tools and action groups by name', () => {
  const main = toolbar.normalizeToolbar({ items: [{ id: 'spacer-1', visible: true, label: 'x' }, { id: 'save', visible: true }, { id: 'tool:Foo', visible: true }] });
  assert.deepEqual(main.items.slice(0, 2), [{ id: 'spacer-1', visible: true }, { id: 'save', visible: true }]);
  assert.ok(!main.items.some(item => item.id === 'tool:Foo'), 'tools belong on the custom toolbar');
  assert.deepEqual(toolbar.normalizeCustomToolbar(undefined), { shown: true, items: [] });
  const custom = toolbar.normalizeCustomToolbar({ shown: false, items: [{ id: 'tool:Foobar2000', visible: true }, { id: 'action:Clean#Case', visible: false, label: ' Case ' }, { id: 'save', visible: true }, { id: 'tool:', visible: true }, { id: 'nonsense', visible: true }, { id: 'tool:Foobar2000', visible: true }, { id: 'separator-1', visible: true }] });
  assert.equal(custom.shown, false);
  assert.deepEqual(custom.items.map(item => item.id), ['tool:Foobar2000', 'action:Clean#Case', 'save', 'separator-1']);
  assert.equal(custom.items[1].label, 'Case');
  assert.equal(toolbar.itemLabel({ id: 'tool:Foobar2000', visible: true }), 'Foobar2000');
  assert.equal(toolbar.itemLabel({ id: 'spacer-2', visible: true }), 'Spacer');
  assert.match(toolbar.itemHint('action:Clean'), /Action group: apply Clean/);
  assert.match(toolbar.itemHint('tool:Foobar2000'), /Tool: start Foobar2000/);
  assert.equal(toolbar.itemHint('save'), toolbar.TOOLBAR_COMMANDS.save.hint);
  assert.equal(toolbar.dividerId([{ id: 'spacer-1', visible: true }], 'spacer'), 'spacer-2');
  assert.deepEqual(toolbar.visibleItems([{ id: 'spacer-1', visible: true }, { id: 'save', visible: true }, { id: 'spacer-2', visible: true }]).map(item => item.id), ['spacer-1', 'save', 'spacer-2']);
});
