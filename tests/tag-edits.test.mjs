import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/tag-edits.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { resolveTagEdits } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const row = (original, name, values = [], valueChanged = false, removed = false) => ({ original, name, values, valueChanged, removed, changed: true });

test('simultaneous field swaps preserve both values', () => {
  const source = { A: ['first'], B: ['second'] };
  assert.deepEqual(resolveTagEdits(source, [row('A', 'B'), row('B', 'A')]), { A: ['second'], B: ['first'] });
  assert.deepEqual(source, { A: ['first'], B: ['second'] });
});

test('rename into a removed field keeps the renamed value in either row order', () => {
  const source = { A: ['first'], B: ['second'] };
  const edits = [row('A', 'B'), row('B', 'B', [], false, true)];
  assert.deepEqual(resolveTagEdits(source, edits), { A: [], B: ['first'] });
  assert.deepEqual(resolveTagEdits(source, edits.toReversed()), { A: [], B: ['first'] });
});

test('mixed field rename resolves each file independently and preserves multivalues', () => {
  const edits = [row('ARTIST', 'PERFORMER')];
  assert.deepEqual(resolveTagEdits({ ARTIST: ['Björk', 'Guest'] }, edits), { ARTIST: [], PERFORMER: ['Björk', 'Guest'] });
  assert.deepEqual(resolveTagEdits({ ARTIST: ['Other'] }, edits), { ARTIST: [], PERFORMER: ['Other'] });
  assert.deepEqual(resolveTagEdits({}, edits), { ARTIST: [], PERFORMER: [] });
});

test('explicit value edits override per-file values while untouched rows do nothing', () => {
  assert.deepEqual(resolveTagEdits({ ARTIST: ['Original'] }, [row('ARTIST', 'PERFORMER', ['New'], true)]), { ARTIST: [], PERFORMER: ['New'] });
  assert.deepEqual(resolveTagEdits({ ARTIST: ['Original'] }, [{ ...row('ARTIST', 'ARTIST'), changed: false }]), {});
  assert.deepEqual(resolveTagEdits({ ARTIST: ['Original'] }, [row('ARTIST', 'ARTIST', [], true)]), { ARTIST: [] });
});

test('name-only changes retain multiline field values', () => {
  assert.deepEqual(resolveTagEdits({ COMMENT: ['first line\nsecond line'] }, [row('COMMENT', 'DESCRIPTION')]), { COMMENT: [], DESCRIPTION: ['first line\nsecond line'] });
});

test('validate the entire edit before changing fields', () => {
  assert.throws(() => resolveTagEdits({ A: ['first'], B: ['second'] }, [row('A', 'B'), { ...row('B', 'B'), changed: false }]), /Duplicate field/);
  assert.throws(() => resolveTagEdits({}, [row('', '')]), /field name/);
  assert.deepEqual(resolveTagEdits({}, [row('', '', [], false, true)]), {});
});
