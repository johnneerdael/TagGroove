import {loadProductionModule} from './helpers/load-production-module.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

async function load(name) {
  if(name==='tag-panel')return loadProductionModule(`${name}.ts`);
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const js = ts.transpileModule(source, {compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022,
  }}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
}
const panel = await load('tag-panel');
const clipboard = await load('tag-clipboard');
const {reconcileDrafts} = await load('document-state');
const read = name => readFile(new URL(`../docs/user-guide/${name}.md`, import.meta.url), 'utf8');
function rows(page, marker) {
  const section = page.split(`<!-- ${marker}:start -->`)[1]?.split(`<!-- ${marker}:end -->`)[0];
  assert.ok(section, `Missing written example ${marker}`);
  return section.split(/\r?\n/).filter(line => line.startsWith('| `') || line.startsWith('| Choose') || line.startsWith('| Type'))
    .map(line => line.split('|').slice(1, -1).map(cell => cell.trim()));
}
const code = value => { assert.ok(value.startsWith('`') && value.endsWith('`'), value); return value.slice(1, -1); };
const json = value => JSON.parse(code(value));
const newlineForms = text => [text.replace(/\r\n/g, '\n'), text.replace(/\r?\n/g, '\r\n')];

test('written panel choices preserve keep, clear and literal multivalue semantics under LF and CRLF', async () => {
  for (const page of newlineForms(await read('guides/tag-panel'))) {
    const examples = rows(page, 'panel-choice-examples');
    assert.equal(examples.length, 5);
    for (const [input, expected] of examples) {
      if (input.startsWith('Choose')) {
        const value = input.match(/`([^`]+)`/)[1];
        const option = panel.comboOptions({field: 'ARTIST', values: ['Juniper Vale', 'Mira Solberg']})
          .find(option => option.value === value);
        assert.ok(option, input);
        const text = panel.choiceText(option);
        if (text === null) assert.equal(expected, "Keep each file's loaded values");
        else { assert.equal(expected, "Field removed"); assert.deepEqual(panel.splitValues(text), []); }
      } else {
        assert.deepEqual(panel.splitValues(input.match(/`([^`]+)`/)[1]), [...expected.matchAll(/`([^`]+)`/g)].map(match => match[1]));
      }
    }
  }
});

test('written tag paste replaces the field set, including the target-only comment', async () => {
  for (const page of newlineForms(await read('guides/copy-paste-tags'))) {
    const examples = rows(page, 'tag-paste-example');
    assert.equal(examples.length, 3);
    const fields = column => Object.fromEntries(examples.map(row => [code(row[0]), json(row[column])]).filter(([, value]) => value.length));
    const copied = clipboard.copyTags([{name: 'source.mp3', fields: fields(1), covers: 0}]);
    const target = fields(2);
    const plan = clipboard.pastePlan(copied, [{path: '/target.mp3', fields: target}]);
    assert.equal(plan.length, 1);
    const result = {...target, ...plan[0].updates};
    assert.deepEqual(Object.fromEntries(Object.entries(result).filter(([, value]) => value.length)), fields(3));
    assert.deepEqual(plan[0].updates.COMMENT, []);
  }
});

test('written source and target count cases use ordered tag mapping or reject mismatches', async () => {
  for (const page of newlineForms(await read('guides/copy-paste-tags'))) {
    const examples = rows(page, 'tag-paste-counts');
    assert.equal(examples.length, 4);
    for (const [sourceCell, targetCell, outcome] of examples) {
      const sourceCount = Number(code(sourceCell)), targetCount = Number(code(targetCell));
      const copied = clipboard.copyTags(Array.from({length: sourceCount}, (_, i) => ({name: `${i}.mp3`, fields: {TITLE: [`Source ${i + 1}`]}, covers: 0})));
      const targets = Array.from({length: targetCount}, (_, i) => ({path: `/target${i}.mp3`, fields: {}}));
      if (outcome.startsWith('Refused')) assert.throws(() => clipboard.pastePlan(copied, targets), /holds tags/);
      else {
        const updates = clipboard.pastePlan(copied, targets);
        assert.equal(updates.length, targetCount);
        updates.forEach((item, index) => assert.deepEqual(item.updates.TITLE, [`Source ${sourceCount === 1 ? 1 : index + 1}`]));
      }
    }
  }
});

test('partial-save table clears only successful selected files and retains other pending edits', async () => {
  for (const page of newlineForms(await read('getting-started/editing-and-saving'))) {
    const examples = rows(page, 'save-outcomes');
    assert.equal(examples.length, 3);
    const pending = new Map(examples.map(([file]) => [code(file), {ALBUM: ['Harbour Lights — practice']}]));
    const saved = examples.filter(([, selected, result]) => selected === 'Yes' && result === 'Saved').map(([file]) => code(file));
    reconcileDrafts(pending, saved, true);
    examples.forEach(([file, , , expected]) => assert.equal(pending.has(code(file)), expected === 'Yes', file));
  }
});
