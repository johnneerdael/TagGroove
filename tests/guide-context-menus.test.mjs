import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { insertion } = await load('helper-menu');
const { coverMenu } = await load('cover-model');
const page = await readFile(new URL('../docs/user-guide/reference/menus/context-menus.md', import.meta.url), 'utf8');
const table = marker => page.split(`<!-- ${marker}:start -->`)[1]
  .split(`<!-- ${marker}:end -->`)[0].split(/\r?\n/).filter(line => line.startsWith('| `'))
  .map(line => line.split('|').slice(1, -1).map(cell => cell.trim().replace(/^`|`$/g, '')));

test('helper insertion executes each written caret example, including selected function argument', () => {
  const examples = table('helper-insertion-examples');
  assert.equal(examples.length, 3);
  for (const [marked, added, expected] of examples) {
    const caret = marked.indexOf('▮');
    assert.notEqual(caret, -1);
    const result = insertion(marked.replace('▮', ''), caret, caret, added);
    assert.equal(result.value, expected);
    if (added.startsWith('$')) assert.equal(result.value.slice(result.start, result.end), 'x');
    else assert.equal(result.start, result.end);
  }
});

test('cover menu availability matches all written none/common/varying cases', () => {
  const examples = table('cover-menu-cases');
  assert.equal(examples.length, 3);
  const commands = ['add', 'remove', 'replace', 'extract', 'copy', 'next'];
  for (const [state, ...expected] of examples) {
    const items = coverMenu({selected:2, state, count:state==='same'?2:0, index:0,
      canEdit:true, canClipboard:true, aspect:true, mac:false});
    const actual = commands.map(command => items.find(item => item.command===command).enabled ? 'Yes' : 'No');
    assert.deepEqual(actual, expected, state);
  }
});
