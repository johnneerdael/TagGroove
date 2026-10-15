import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

const load = async (name, replace = {}) => {
  let javascript = ts.transpileModule(await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8'), {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
  for (const [from, to] of Object.entries(replace)) javascript = javascript.replace(from, to);
  return `data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`;
};
const actionUI = await load('action-ui');
const {menuTree, menuLabel, localized, readSettings, settingsMarkup} = await import(await load('web-sources', {"'./action-ui'": `'${actionUI}'`}));

test('web sources form # submenus with & access keys in file name order', () => {
  const leaves = ['Shop#&Albums', 'Shop#&Singles', '&Lyrics', 'Covers#Shop#Front'].map(menu => ({menu, markup: raw => `<b data-menu="${menu}">${raw}</b>`}));
  const markup = menuTree(leaves);
  assert.equal(markup,
    '<details class="group-submenu web-submenu"><summary>Shop</summary><div><b data-menu="Shop#&Albums">&Albums</b><b data-menu="Shop#&Singles">&Singles</b></div></details>'
    + '<b data-menu="&Lyrics">&Lyrics</b>'
    + '<details class="group-submenu web-submenu"><summary>Covers</summary><div><details class="group-submenu web-submenu"><summary>Shop</summary><div><b data-menu="Covers#Shop#Front">Front</b></div></details></div></details>');
  // Submenu names carry their own access keys.
  assert.match(menuTree([{menu: '&Shop#Albums', markup: () => 'x'}]), /<summary><u>S<\/u>hop<\/summary>/);
  assert.equal(menuLabel('Shop#&Albums && More'), 'Shop › Albums & More');
});

const schema = {
  key: 'fixture', title: 'Fixture Settings',
  settings: [
    {type: 'heading', key: 'h', title: 'Import'},
    {type: 'bool', key: 'comment', title: 'Write a comment', description: 'Adds COMMENT', default: false},
    {type: 'string', key: 'lang', title: 'Language', choices: ['en', 'de'], default: 'en'},
    {type: 'string', key: 'note', title: 'Note', default: 'x'},
    {type: 'number', key: 'limit', title: 'Limit', default: 3},
    {type: 'separator', key: 's', title: 'Below the line'},
  ],
  localizations: {de: [{key: 'comment', title: 'Kommentar schreiben', description: 'Fügt COMMENT hinzu'}]},
};

test('settings dialogs are generated from the schema with localized titles', () => {
  const markup = settingsMarkup({schema, values: {comment: true, lang: 'de', note: '<b>', limit: 5}}, 'de-DE');
  assert.match(markup, /<h3 class="ws-heading">Import<\/h3>/);
  assert.match(markup, /<input type="checkbox" data-setting="1" checked> Kommentar schreiben<\/label><small>Fügt COMMENT hinzu<\/small>/);
  assert.match(markup, /<option selected>de<\/option>/);
  assert.match(markup, /data-setting="3" value="&lt;b&gt;"/);
  assert.match(markup, /type="number" data-setting="4" value="5"/);
  assert.match(markup, /<hr class="ws-separator"><p class="ws-note">Below the line<\/p>/);
  assert.match(settingsMarkup({schema, values: {}}, 'en'), /> Write a comment<\/label>/);
  assert.equal(localized(schema, 'comment', 'fr')?.title, undefined);
});

test('settings are read back with their types', () => {
  const controls = {1: {checked: true, value: 'on'}, 2: {value: 'de'}, 3: {value: 'text'}, 4: {value: '7'}};
  const root = {querySelector: selector => controls[/data-setting="(\d+)"/.exec(selector)[1]] || null};
  assert.deepEqual(readSettings(schema, root), {comment: true, lang: 'de', note: 'text', limit: 7});
  controls[4].value = 'many';
  assert.throws(() => readSettings(schema, root), /Limit needs a number/);
});
