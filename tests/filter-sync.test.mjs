import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { FilterSync } = await load('filter-sync');
const { updateSortHeader, headerMarkup, defaultLayout } = await load('columns');

function host() {
  const state = { paths: ['a', 'b', 'c'], entries: { a: { v: 1 }, b: { v: 1 }, c: { v: 1 } }, drafts: {}, columns: [{ name: 'Title', template: '%title%' }], values: {} };
  return {
    state,
    paths: () => state.paths,
    columns: () => state.columns,
    token: (path, columns) => [state.entries[path], state.drafts[path], ...columns.map(c => state.values[path]?.[c.template])],
    file: (path, columns) => ({ path, fields: { TITLE: [path] }, raw_fields: { TITLE: [path] }, columns: columns.map(c => state.values[path]?.[c.template] ?? null) }),
  };
}

test('filter queries send each file and the order only when they change', () => {
  const h = host(), sync = new FilterSync(h, 'session');
  const first = sync.request();
  assert.equal(first.session, 'session');
  assert.deepEqual(first.order, ['a', 'b', 'c']);
  assert.deepEqual(first.updates.map(file => file.path), ['a', 'b', 'c']);
  // Typing another character sends only the query.
  const second = sync.request();
  assert.equal(second.order, null);
  assert.deepEqual(second.updates, []);
  // A saved file, a pending edit and a new column value each resend only that file.
  h.state.entries.b = { v: 2 };
  h.state.drafts.c = { TITLE: ['edited'] };
  h.state.values.a = { '%title%': 'A' };
  const third = sync.request();
  assert.deepEqual(third.updates.map(file => file.path), ['a', 'b', 'c']);
  assert.deepEqual(third.updates[0].columns, ['A']);
  assert.deepEqual(sync.request().updates, []);
  // A new sort order is sent without resending fields.
  h.state.paths = ['c', 'b', 'a'];
  const sorted = sync.request();
  assert.deepEqual(sorted.order, ['c', 'b', 'a']);
  assert.deepEqual(sorted.updates, []);
});

test('changed columns, removed files and resets resend fields', () => {
  const h = host(), sync = new FilterSync(h, 'session');
  sync.request();
  h.state.columns = [{ name: 'Artist', template: '%artist%' }];
  assert.equal(sync.request().updates.length, 3);
  // A file that leaves and returns is sent again: the backend dropped it.
  h.state.paths = ['a', 'b'];
  assert.deepEqual(sync.request().updates, []);
  h.state.paths = ['a', 'b', 'c'];
  assert.deepEqual(sync.request().updates.map(file => file.path), ['c']);
  sync.reset();
  const resent = sync.request();
  assert.deepEqual(resent.order, ['a', 'b', 'c']);
  assert.equal(resent.updates.length, 3);
});

test('sort header updates keep the header cells', () => {
  const layout = defaultLayout(), columns = layout.columns.slice(0, 2);
  const cell = column => {
    const attributes = {}, button = { innerHTML: '' };
    return { dataset: { col: column.id }, setAttribute: (name, value) => { attributes[name] = value; }, querySelector: () => button, attributes, button };
  };
  const cells = columns.map(cell);
  updateSortHeader(cells, columns, [{ id: 'title', ascending: false }]);
  assert.equal(cells[0].attributes['aria-sort'], 'none');
  assert.equal(cells[1].attributes['aria-sort'], 'descending');
  assert.equal(cells[0].button.innerHTML, 'Filename');
  assert.equal(cells[1].button.innerHTML, 'Title ↓');
  // The same labels as a full header render.
  assert.ok(headerMarkup(columns, [{ id: 'title', ascending: false }]).includes('>Title ↓</button>'));
});
