import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

const root = new URL('../', import.meta.url);
const source = await readFile(new URL('src/utils-ui.ts', root), 'utf8');
const javascript = ts.transpileModule(source, {
  compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022},
}).outputText;
const {utilTargets} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

test('written mixed-format utility counts use the actual selection model under LF and CRLF', async () => {
  const written = (await readFile(new URL('docs/user-guide/guides/utilities.md', root), 'utf8')).replace(/\r\n/g, '\n');
  for (const page of [written, written.replace(/\n/g, '\r\n')]) {
    const section = page.split('<!-- utility-scope-cases:start -->')[1].split('<!-- utility-scope-cases:end -->')[0];
    const rows = section.split(/\r?\n/).filter(line => line.startsWith('|')).slice(2);
    assert.equal(rows.length, 4, 'Read every written scope example');
    for (const row of rows) {
      const [formats, unsupported, optimize] = row.split('|').slice(1, -1).map(cell => cell.trim().slice(1, -1));
      const selected = formats.split(', ');
      assert.equal(utilTargets('remove_unsupported_fields', selected), Number(unsupported), formats);
      assert.equal(utilTargets('optimize', selected), Number(optimize), formats);
    }
  }
});
