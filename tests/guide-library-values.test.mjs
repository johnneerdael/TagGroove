import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const root = new URL('../', import.meta.url);
async function load(name) {
  const text = await readFile(new URL(`src/${name}.ts`, root), 'utf8');
  const js = ts.transpileModule(text, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
}
const {parseValueList, withLists, defaultSettings, comboOptions} = await load('tag-panel');
const {ToolsUI} = await load('tools');
const section = (page, name) => page.split(`<!-- ${name}:start -->`)[1].split(`<!-- ${name}:end -->`)[0];
function block(page, name) {
  const lines = section(page, name).split(/\r?\n/), start = lines.findIndex(line => /^ *```text$/.test(line));
  assert(start >= 0, name);
  const indentation = lines[start].match(/^ */)[0], values = [];
  for (let n = start + 1; n < lines.length && lines[n].trim() !== '```'; n++) values.push(lines[n].startsWith(indentation) ? lines[n].slice(indentation.length) : lines[n]);
  return values.join('\n');
}
test('written field lists and preference examples use actual saved-list and dropdown models under LF/CRLF', async () => {
  const written = (await readFile(new URL('docs/user-guide/customize/field-values.md', root), 'utf8')).replace(/\r\n/g, '\n');
  for (const page of [written, written.replace(/\n/g, '\r\n')]) {
    assert.deepEqual(parseValueList(block(page, 'field-values-normalization')), block(page, 'field-values-normalized').split('\n'));
    const lists = withLists(defaultSettings(), {mood: parseValueList(block(page, 'field-values-practice'))});
    assert.deepEqual(Object.keys(lists.field_values.lists), ['MOOD']);
    const rows = section(page, 'field-values-preference').split(/\r?\n/).filter(line => line.startsWith('|')).slice(2);
    assert.equal(rows.length, 3);
    for (const line of rows) {
      const [selected, prefer, expected] = line.split('|').slice(1, -1).map(cell => cell.trim());
      const values = selected === 'Both empty' ? ['', ''] : [selected.slice(1, -1)];
      const options = comboOptions({field: 'MOOD', values, custom: lists.field_values.lists.MOOD, preferTagValues: prefer === 'On'});
      assert.deepEqual(options.filter(option => option.kind === 'value').map(option => option.value), expected.slice(1, -1).split(', '));
    }
  }
});
test('documented fixed Tools inventory matches actual menu assembly; saved names remain literal dynamic labels', async () => {
  const text = await readFile(new URL('src/main.ts', root), 'utf8');
  const ast = ts.createSourceFile('main.ts', text, ts.ScriptTarget.Latest, true);
  let fixed;
  function visit(node) {
    if (ts.isCallExpression(node) && node.expression.getText(ast) === 'menu' && node.arguments[0]?.text === 'Tools') fixed = node.arguments[1].elements.map(entry => entry.elements[1].text);
    ts.forEachChild(node, visit);
  }
  visit(ast);assert.equal(fixed.length, 5);
  const page = await readFile(new URL('docs/user-guide/reference/menus/tools.md', root), 'utf8');
  const written = [...page.matchAll(/^\| \*\*([^*]+)\*\* \|/gm)].map(match => match[1]);
  assert.deepEqual(written, fixed);
  const ui = new ToolsUI({values: () => ({tools: [{name:'Inspect & keep',path:'/never-launched',parameters:'',for_each_file:false}]}), reason: () => '', mac: false});
  assert.match(ui.menuHTML(), /Inspect &amp; keep/);
  assert.match(ui.menuHTML(), /Ctrl 1/);
});
