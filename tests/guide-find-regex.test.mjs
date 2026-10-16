import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/find.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {
  compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022},
}).outputText;
const { findMatcher } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

test('written Find regex examples use its actual JavaScript matcher and case control', async () => {
  const page = await readFile(new URL('../docs/user-guide/reference/regular-expressions.md', import.meta.url), 'utf8');
  const section = page.split('<!-- find-regex-examples:start -->')[1]?.split('<!-- find-regex-examples:end -->')[0];
  assert.ok(section, 'Find dialect examples must be present in the reference');
  const rows = section.split(/\r?\n/).filter(line => line.startsWith('| `'));
  assert.equal(rows.length, 3);
  for (const row of rows) {
    const [pattern, sensitive, value, expected] = row.split('|').slice(1, -1)
      .map(cell => cell.trim().replace(/^`|`$/g, ''));
    if (expected === 'Invalid pattern') {
      assert.throws(() => findMatcher(pattern, 'regex', sensitive === 'On'), /Invalid regular expression/);
    } else {
      assert.equal(findMatcher(pattern, 'regex', sensitive === 'On')(value), expected === 'Match', row);
    }
  }
});
