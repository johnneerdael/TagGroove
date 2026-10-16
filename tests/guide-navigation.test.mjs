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
const { windowShortcut } = await load('keyboard');
const { editShortcut } = await load('edit-history');
const { fileShortcut } = await load('file-ops');
const { listShortcut } = await load('list-commands');

test('written keyboard examples route through the actual focus/platform helpers', async () => {
  const page = await readFile(new URL('../docs/user-guide/reference/keyboard-shortcuts.md', import.meta.url), 'utf8');
  const exampleTable = page.split('<!-- keyboard-routing-examples:start -->')[1]
    .split('<!-- keyboard-routing-examples:end -->')[0];
  const rows = exampleTable.split(/\r?\n/).filter(line => line.startsWith('| `'));
  assert.equal(rows.length, 9);
  const labels = new Map([
    ['extended', 'Extended tags'], ['tag-tag', 'Tag → Tag'],
    ['number-folders', 'Numbering per folder'], ['pane-next', 'Next pane'],
  ]);
  for (const row of rows) {
    const [chord, contextLabel, expected] = row.split('|').slice(1, -1).map(cell => cell.trim());
    const keys = chord.slice(1, -1).split('+');
    const key = keys.pop();
    const event = {
      key: key.length === 1 ? key.toLowerCase() : key,
      code: /^\d$/.test(key) ? `Digit${key}` : key.length === 1 ? `Key${key.toUpperCase()}` : key,
      ctrlKey: keys.includes('Control') || keys.includes('Ctrl'),
      metaKey: keys.includes('Command'), altKey: keys.includes('Alt') || keys.includes('Option'),
      shiftKey: keys.includes('Shift'),
    };
    const context = { mac: contextLabel.startsWith('macOS'), typing: contextLabel.includes('text field') };
    const routed = windowShortcut(event, context)
      ?? editShortcut(event, { ...context, textSelected: false })
      ?? fileShortcut(event, context.typing, context.mac)
      ?? listShortcut(event, context.typing, context.mac);
    assert.equal(routed === undefined ? 'Text entry' : labels.get(routed), expected, row);
  }
});
