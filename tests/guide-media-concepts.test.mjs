import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
async function load(path) {
  const source = await readFile(new URL(path, import.meta.url), 'utf8');
  const js = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
}
const covers = await load('../src/cover-model.ts');
const columns = await load('../src/columns.ts');
const coverDoc = await readFile(new URL('../docs/user-guide/concepts/cover-art.md', import.meta.url), 'utf8');
const sortDoc = await readFile(new URL('../docs/user-guide/concepts/sort-fields.md', import.meta.url), 'utf8');
const forms = text => {const lf = text.replace(/\r\n/g, '\n'); return [lf, lf.replace(/\n/g, '\r\n')];};
function rows(text, marker) {
  return text.split(`<!-- ${marker}:start -->`)[1].split(`<!-- ${marker}:end -->`)[0]
    .split(/\r?\n/).filter(line => line.startsWith('|')).slice(2)
    .map(line => line.split('|').slice(1, -1).map(cell => cell.trim()));
}
const code = text => {assert.ok(text.startsWith('`') && text.endsWith('`')); return text.slice(1, -1);};
test('written cover identity/order examples use actual shared-selection model', () => {
  for (const text of forms(coverDoc)) {
    const definitions = rows(text, 'cover-concept-definitions');assert.equal(definitions.length, 4);
    const entries = Object.fromEntries(definitions.map(([label, image, role, description]) => {
      const kind = covers.PICTURE_TYPES.find(([, display]) => display === role)?.[0];assert.ok(kind);
      return [code(label), {index: 0, digest: code(image), picture_type: kind, description: code(description), mime: 'image/png', data_base64: '', width: 300, height: 300}];
    }));
    const list = value => value === 'None' ? [] : code(value).split(',').map((key, index) => ({...entries[key], index}));
    const cases = rows(text, 'cover-concept-selections');assert.equal(cases.length, 6);
    for (const [first, second, expected] of cases) {
      const state = covers.selectionCovers([list(first), list(second)]);
      assert.equal(state.state, {'Same': 'same', 'Covers vary': 'varying', 'No embedded artwork': 'none'}[expected]);
    }
  }
});
test('written column display/key distinction matches actual default and custom sort selection', () => {
  for (const text of forms(sortDoc)) {
    const cases = rows(text, 'sort-concept-expressions');assert.equal(cases.length, 3);
    const artist = columns.defaultLayout().columns.find(column => column.id === 'artist');assert.ok(artist);
    assert.equal(artist.sort_by, '');
    assert.equal(columns.sortTemplate(artist), artist.value);
    assert.equal(artist.value.includes('artistsort'), false);
    const custom = {...artist, value: code(cases[0][1]), sort_by: code(cases[1][1])};
    assert.equal(columns.sortTemplate(custom), '$if2(%artistsort%,%artist%)');
    assert.equal(custom.value, '%artist%');
    assert.equal(columns.sortTemplate({...custom, sort_by: '  '}), '%artist%');
  }
});
