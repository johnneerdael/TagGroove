import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/converter-review.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {createConverterInputTracker, createConverterReview, validateFilenameItems, validateTextItems} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const request = () => ({paths: ['/a.mp3'], old_pattern: '%1', new_pattern: '%1-$uuid()', columns: []});
const preview = (token = 'reviewed') => ({plan_token: token, items: [{path: '/a.mp3', target: '/a-fixed.mp3', captures: {'1': ['a']}}]});

test('apply uses the captured request and reviewed result exactly once', () => {
  const state = createConverterReview(validateFilenameItems), original = request();
  const attempt = state.begin(original, 'workspace1');
  original.paths.push('/later.mp3');
  assert.deepEqual(attempt.request.paths, ['/a.mp3']);
  assert.equal(state.complete(attempt, preview(), request(), 'workspace1'), true);
  assert.equal(state.canApply(request(), 'workspace1'), true);
  const reviewed = state.take(request(), 'workspace1');
  assert.deepEqual(reviewed.request, request());
  assert.equal(reviewed.planToken, 'reviewed');
  assert.equal(reviewed.items[0].target, '/a-fixed.mp3');
  assert.equal(state.canApply(request(), 'workspace1'), false);
  assert.throws(() => state.take(request(), 'workspace1'), /Preview again/);
});
test('input edits and workspace changes reject a response still in flight', () => {
  const state = createConverterReview(validateFilenameItems), attempt = state.begin(request(), 'workspace1');
  assert.equal(state.complete(attempt, preview(), {...request(), new_pattern: '%1-new'}, 'workspace1'), false);
  state.invalidate();
  assert.equal(state.complete(attempt, preview(), request(), 'workspace1'), false);
  const next = state.begin(request(), 'workspace1');
  assert.equal(state.complete(next, preview(), request(), 'workspace2'), false);
  assert.equal(state.canApply(request(), 'workspace2'), false);
});
test('late failures and successes cannot replace or disable a newer preview', () => {
  const state = createConverterReview(validateFilenameItems);
  const old = state.begin(request(), 'workspace');
  const current = state.begin(request(), 'workspace');
  assert.equal(state.complete(current, preview('current-token'), request(), 'workspace'), true);
  assert.equal(state.fail(old), false);
  assert.equal(state.complete(old, preview('old-token'), request(), 'workspace'), false);
  assert.equal(state.take(request(), 'workspace').planToken, 'current-token');
});
test('any selection order, source, encoding, mask or case-policy edit requires review again', () => {
  const original = {paths: ['/a.mp3', '/b.mp3'], source: {kind: 'text', text: 'A\nB'}, mask: '%title%', options: {ansi_codepage: null, case_sensitive_paths: true}};
  const response = {plan_token: 'text-token', items: [{selection_index: 0, source_line: 1, path: '/a.mp3', fields: {TITLE: ['A']}}, {selection_index: 1, source_line: 2, path: '/b.mp3', fields: {TITLE: ['B']}}]};
  for (const changed of [
    {...original, paths: ['/b.mp3', '/a.mp3']}, {...original, source: {kind: 'file', path: '/records.txt'}},
    {...original, source: {kind: 'text', text: 'new\ntext'}}, {...original, mask: '%artist%'},
    {...original, options: {...original.options, ansi_codepage: 'windows-1252'}},
    {...original, options: {...original.options, case_sensitive_paths: false}},
  ]) {
    const state = createConverterReview(validateTextItems), attempt = state.begin(original, 'workspace');
    state.complete(attempt, response, original, 'workspace');
    assert.equal(state.canApply(changed, 'workspace'), false);
    assert.throws(() => state.take(changed, 'workspace'), /Preview again/);
  }
});
test('expired previews cannot be applied', () => {
  let time = 0;
  const state = createConverterReview(validateFilenameItems, () => time), attempt = state.begin(request(), 'workspace');
  state.complete(attempt, preview(), request(), 'workspace');
  time = 300000;
  assert.equal(state.canApply(request(), 'workspace'), false);
  assert.throws(() => state.take(request(), 'workspace'), /Preview again/);
});
test('changing and restoring an input or closing the dialog cannot resurrect a completed review', () => {
  const state = createConverterReview(validateFilenameItems), attempt = state.begin(request(), 'workspace');
  state.complete(attempt, preview(), request(), 'workspace');
  state.invalidate();
  assert.equal(state.canApply(request(), 'workspace'), false);
  assert.equal(state.complete(attempt, preview(), request(), 'workspace'), false);
});
test('filename row identity is checked and keyed text previews may follow source-line order', () => {
  assert.throws(() => validateFilenameItems(request(), [{path: '/other.mp3', target: '/out.mp3'}]), /selected files/);
  const text = {paths: ['/a.mp3', '/b.mp3']};
  const reversed = [{selection_index: 1, source_line: 1, path: '/b.mp3', fields: {TITLE: ['B']}}, {selection_index: 0, source_line: 2, path: '/a.mp3', fields: {TITLE: ['A']}}];
  assert.doesNotThrow(() => validateTextItems(text, reversed));
  assert.throws(() => validateTextItems(text, [reversed[0], reversed[0]]), /selected files/);
});
test('failed or malformed current previews never enable Apply', () => {
  const state = createConverterReview(validateFilenameItems), attempt = state.begin(request(), 'workspace');
  assert.throws(() => state.complete(attempt, preview(''), request(), 'workspace'), /could not be verified/);
  assert.equal(state.fail(attempt), true);
  assert.equal(state.pending, false);
  assert.equal(state.canApply(request(), 'workspace'), false);
});
test('first Preview survives the delayed change event repeating the final filename input', () => {
  const state = createConverterReview(validateFilenameItems);
  const current = request();
  const values = () => [current.old_pattern, current.new_pattern];
  const changed = createConverterInputTracker(values());
  const edited = () => { if (changed(values())) state.invalidate(); };
  current.old_pattern = '%1 - %2 - %3'; edited();
  current.new_pattern = '%2 - %3-$uuid()'; edited();
  changed(values()); // Preview captures the committed control values.
  const attempt = state.begin(current, 'workspace');
  edited(); // Native blur delivers change after begin, with no value edit.
  assert.equal(state.complete(attempt, preview('first-preview'), current, 'workspace'), true);
  assert.equal(state.take(current, 'workspace').planToken, 'first-preview');
});
test('AX edits first reported by a deferred commit are acknowledged at Preview capture', () => {
  const state = createConverterReview(validateFilenameItems), current = request();
  const values = () => [current.old_pattern, current.new_pattern];
  const changed = createConverterInputTracker(values());
  current.new_pattern = '%1-exact-$uuid()'; // No input event before AX activation.
  changed(values());
  const attempt = state.begin(current, 'workspace');
  if (changed(values())) state.invalidate(); // Later change repeats the captured value.
  assert.equal(state.complete(attempt, preview(), current, 'workspace'), true);
});
test('actual edits and restoration while a preview is pending still invalidate its generation', () => {
  const state = createConverterReview(validateFilenameItems), current = request();
  const changed = createConverterInputTracker([current.new_pattern]);
  const attempt = state.begin(current, 'workspace');
  const original = current.new_pattern;
  current.new_pattern = '%1-edited';
  if (changed([current.new_pattern])) state.invalidate();
  current.new_pattern = original;
  if (changed([current.new_pattern])) state.invalidate();
  assert.equal(state.complete(attempt, preview(), current, 'workspace'), false);
  assert.equal(state.canApply(current, 'workspace'), false);
});
test('picker value synchronization ignores duplicate commits but tracks checkbox and source changes', () => {
  const changed = createConverterInputTracker(['file', '', '', true]);
  assert.equal(changed(['file', '/records.txt', '', true]), true);
  assert.equal(changed(['file', '/records.txt', '', true]), false);
  assert.equal(changed(['file', '/records.txt', '', false]), true);
  assert.equal(changed(['file', '/records.txt', '', true]), true);
  assert.equal(changed(['text', '/records.txt', 'literal records', true]), true);
});
test('files left unchanged must account for the rest of the selection, and an empty plan never applies', () => {
  const three = {paths: ['/a.mp3', '/odd.mp3', '/b.mp3'], old_pattern: '%1 - %2', new_pattern: '%2 - %1', columns: []};
  const renamed = [{path: '/a.mp3', target: '/x.mp3', captures: {}}, {path: '/b.mp3', target: '/y.mp3', captures: {}}];
  assert.doesNotThrow(() => validateFilenameItems(three, renamed, ['/odd.mp3']));
  assert.throws(() => validateFilenameItems(three, renamed), /selected files/);
  assert.throws(() => validateFilenameItems(three, renamed, ['/other.mp3']), /selected files/);
  assert.throws(() => validateFilenameItems(three, [renamed[1], renamed[0]], ['/odd.mp3']), /selected files/);
  const state = createConverterReview(validateFilenameItems), attempt = state.begin(three, 'workspace');
  assert.equal(state.complete(attempt, {plan_token: 'partial', items: renamed, unmatched: ['/odd.mp3']}, three, 'workspace'), true);
  assert.equal(state.planned, 2);
  assert.equal(state.canApply(three, 'workspace'), true);
  const none = state.begin(three, 'workspace');
  assert.equal(state.complete(none, {plan_token: 'none', items: [], unmatched: three.paths}, three, 'workspace'), true);
  assert.equal(state.planned, 0);
  assert.equal(state.canApply(three, 'workspace'), false);
  assert.throws(() => state.take(three, 'workspace'), /Preview again/);
  const text = {paths: ['/a.mp3', '/b.mp3', '/c.mp3']};
  const keyed = [{selection_index: 2, source_line: 1, path: '/c.mp3', fields: {TITLE: ['C']}}, {selection_index: 0, source_line: 4, path: '/a.mp3', fields: {TITLE: ['A']}}];
  assert.doesNotThrow(() => validateTextItems(text, keyed, ['/b.mp3']));
  assert.throws(() => validateTextItems(text, keyed), /selected files/);
  assert.throws(() => validateTextItems(text, keyed, ['/a.mp3']), /selected files/);
});
