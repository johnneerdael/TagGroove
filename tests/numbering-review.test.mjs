import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/numbering-review.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {numberingCounter, createNumberingReview} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const request = () => ({items: [{path: '/a.flac', fields: {ALBUM: ['A']}}], options: {disc_enabled: true, track_start: 1}});
const plan = () => [{path: '/a.flac', fields: {TRACK: ['01/12'], DISCNUMBER: ['1/2']}}];

test('numbering rejects empty, fractional and out-of-range counters', () => {
  for (const input of ['', ' ', '0', '-1', '1.5', 'Infinity', '4294967296']) {
    assert.throws(() => numberingCounter(input, 'Track start'), /Track start/);
  }
  assert.equal(numberingCounter('4294967295', 'Track start'), 4294967295);
  assert.equal(numberingCounter('02', 'Digits', 10), 2);
  assert.throws(() => numberingCounter('11', 'Digits', 10), /Digits/);
});

test('staging consumes the reviewed plan and preserves its captured values', () => {
  const state = createNumberingReview(), input = request(), attempt = state.begin(input), response = plan();
  assert.equal(state.complete(attempt, response, input), true);
  response[0].fields.TRACK[0] = '999';
  assert.deepEqual(state.take(input), plan());
  assert.throws(() => state.take(input), /Preview again/);
});

test('changed options, selection order or field values invalidate numbering', () => {
  for (const changed of [
    {...request(), options: {disc_enabled: true, track_start: 2}},
    {...request(), items: [{path: '/b.mp3', fields: {ALBUM: ['A']}}]},
    {...request(), items: [{path: '/a.flac', fields: {ALBUM: ['B']}}]},
  ]) {
    const state = createNumberingReview(), attempt = state.begin(request());
    assert.equal(state.complete(attempt, plan(), changed), false);
    assert.throws(() => state.take(changed), /Preview again/);
  }
  const state = createNumberingReview(), old = state.begin(request());
  state.invalidate();
  const current = state.begin(request());
  assert.equal(state.complete(current, plan(), request()), true);
  assert.equal(state.complete(old, plan(), request()), false);
  assert.deepEqual(state.take(request()), plan());
});

test('partial, reordered or unexpected patches cannot stage any fields', () => {
  for (const response of [[], [{...plan()[0], path: '/other.flac'}], [{...plan()[0], fields: {TRACK: ['01'], TITLE: ['Other']}}], [{...plan()[0], fields: {TRACK: ['01']}}]]) {
    const state = createNumberingReview(), attempt = state.begin(request());
    assert.throws(() => state.complete(attempt, response, request()), /preview/i);
    assert.throws(() => state.take(request()), /Preview again/);
  }
});
