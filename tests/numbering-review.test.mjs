import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/numbering-review.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {numberingCounter, numberingTotal, createNumberingReview, numberingDefaults, recallNumbering, rememberNumbering} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
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

const memoryStore = () => { const items = new Map(); return {getItem: key => items.get(key) ?? null, setItem: (key, value) => items.set(key, value), items}; };

test('the wizard remembers staged options but not the selected-file total', () => {
  const storage = memoryStore();
  assert.deepEqual(recallNumbering(storage), numberingDefaults());
  const options = {...numberingDefaults(), track_start: 5, track_digits: 3, track_total: true, disc_enabled: true, disc_start: 2, disc_total: true, disc_total_value: 3, next_disc_on_field: 'ALBUM', reset_track_on_next_disc: true};
  rememberNumbering(storage, options);
  assert.deepEqual(recallNumbering(storage), options);
  assert.ok(!('track_total_value' in JSON.parse(storage.items.get('opentag.numbering'))));
  // Damaged or out-of-range values fall back one by one.
  storage.setItem('opentag.numbering', JSON.stringify({...options, track_digits: 11, disc_start: 0, track_total: 'yes', next_disc_on_field: 4}));
  assert.deepEqual(recallNumbering(storage), {...options, track_digits: 2, disc_start: 1, track_total: false, next_disc_on_field: ''});
  storage.setItem('opentag.numbering', '{not json');
  assert.deepEqual(recallNumbering(storage), numberingDefaults());
  assert.deepEqual(recallNumbering(undefined), numberingDefaults());
});

test('Ctrl+Shift+K presets per-folder options without making them the Ctrl+K defaults', () => {
  const storage = memoryStore();
  rememberNumbering(storage, {...numberingDefaults(), track_start: 3, disc_enabled: true});
  const perFolder = recallNumbering(storage, true);
  assert.equal(perFolder.track_start, 3);
  assert.deepEqual([perFolder.next_disc_on_directory, perFolder.reset_track_on_next_disc, perFolder.disc_enabled], [true, true, false]);
  rememberNumbering(storage, {...perFolder, track_digits: 4}, true);
  const plain = recallNumbering(storage);
  assert.equal(plain.track_digits, 4);
  assert.deepEqual([plain.next_disc_on_directory, plain.reset_track_on_next_disc, plain.disc_enabled], [false, false, true]);
});
test('only per-folder numbering counts totals; provider documents keep the entered total', () => {
  const entered = () => 12;
  // Ctrl+Shift+K on local files: each folder's own total.
  assert.deepEqual(numberingTotal(true, true, entered), {track_total: null, count_track_total: true});
  // Ctrl+Shift+K on provider documents (no folders), or Ctrl+K: the entered total.
  const directoryAvailable = false;
  assert.deepEqual(numberingTotal(true && directoryAvailable, true, entered), {track_total: 12, count_track_total: false});
  assert.deepEqual(numberingTotal(false, true, entered), {track_total: 12, count_track_total: false});
  assert.deepEqual(numberingTotal(false, false, () => { throw new Error('not read'); }), {track_total: null, count_track_total: false});
  assert.deepEqual(numberingTotal(true, false, entered), {track_total: null, count_track_total: false});
});
