import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/numbering-review.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {numberingCounter, numberingTotal, recallNumbering, createNumberingReview} = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const doc = await readFile(new URL('../docs/user-guide/cookbook/number-tracks-per-folder.md', import.meta.url), 'utf8');
function rows(text, marker) {
  const part = text.split(`<!-- ${marker}:start -->`)[1].split(`<!-- ${marker}:end -->`)[0];
  return part.split(/\r?\n/).filter(line => line.startsWith('|')).slice(2)
    .map(line => line.split('|').slice(1, -1).map(cell => cell.trim()));
}
const code = value => {assert.ok(value.startsWith('`') && value.endsWith('`')); return value.slice(1, -1);};
test('written per-folder controls use the actual shortcut total mode and reviewed plan', () => {
  for (const text of [doc.replace(/\r\n/g, '\n'), doc.replace(/\r?\n/g, '\r\n')]) {
    const controls = Object.fromEntries(rows(text, 'folder-numbering-options'));
    const preset = recallNumbering(undefined, true);
    assert.equal(preset.next_disc_on_directory, controls['Next disc on change of directory'] === 'On');
    assert.equal(preset.reset_track_on_next_disc, controls['Reset track counter on next disc'] === 'On');
    assert.equal(preset.disc_enabled, controls['Disc numbers'] === 'On');
    const total = numberingTotal(true, controls['Save total track count'] === 'On', () => {throw Error('The disabled fixed-total input must not be read');});
    assert.deepEqual(total, {track_total: null, count_track_total: true});
    const cases = rows(text, 'folder-numbering-cases');assert.equal(cases.length, 5);
    const request = {items: cases.map(([path, track]) => ({path: code(path), fields: {TRACK: [code(track)]}})), options: {
      track_start: numberingCounter(code(controls['Begin at track']), 'Track start'),
      track_digits: numberingCounter(code(controls['Minimum digits under Track numbers']), 'Track digits', 10),
      ...total, disc_enabled: false, disc_start: 1, disc_digits: 1, disc_total: null,
      next_disc_on_directory: true, next_disc_on_field: null, reset_track_on_next_disc: true, reset_disc_on_field: null,
    }};
    const plan = cases.map(([path, , result]) => ({path: code(path), fields: {TRACK: [code(result)]}}));
    const review = createNumberingReview(), attempt = review.begin(request);
    assert.equal(review.complete(attempt, plan, request), true);
    assert.equal(review.canApply(request), true);
    const changed = structuredClone(request);changed.items.reverse();
    assert.equal(review.canApply(changed), false);
    assert.deepEqual(review.take(request), plan);
    assert.equal(review.canApply(request), false);
  }
});
