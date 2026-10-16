import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const js = ts.transpileModule(source, {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const {moveSelection} = await load('list-behaviour');
const {selectionTagRows,resolveTagEdits} = await load('tag-edits');
const {presence} = await load('cover-model');
const read = name => readFile(new URL(`../docs/user-guide/guides/${name}.md`, import.meta.url), 'utf8');
const table = (page, marker) => page.split(`<!-- ${marker}:start -->`)[1]
  .split(`<!-- ${marker}:end -->`)[0].split(/\r?\n/).filter(line=>line.startsWith('| `'))
  .map(line=>line.split('|').slice(1,-1).map(cell=>cell.trim().replace(/^`|`$/g,'')));

test('written manual-order examples retain relative selected order and boundary behavior', async()=>{
  const examples=table(await read('file-list'),'manual-file-order-examples');
  assert.equal(examples.length,4);
  for(const [move,expected] of examples){
    assert.equal(moveSelection(['01.mp3','02.mp3','03.mp3','04.mp3'],new Set(['02.mp3','04.mp3']),move).join(', '),expected);
  }
});

test('written mixed-field rename keeps each file’s own values instead of supplying a shared replacement',async()=>{
  const page=await read('extended-tags'),examples=table(page,'extended-rename-example');
  assert.equal(examples.length,2);
  const files=examples.map(([,before])=>({SESSION_NOTE:[before],TITLE:['Unchanged track']}));
  assert.equal(selectionTagRows(files).find(row=>row.name==='SESSION_NOTE').mixed,true);
  for(const [index,[,before,name,expected]] of examples.entries()){
    const updates=resolveTagEdits(files[index],[{original:'SESSION_NOTE',name,values:[],changed:true,valueChanged:false,removed:false}]);
    assert.deepEqual(updates.SESSION_NOTE,[]);
    assert.deepEqual(updates[name],[expected]);
    assert.equal(before,expected);
    assert.equal(updates.TITLE,undefined);
  }
});

test('written partial-cover membership changes only when applying the cover to all',async()=>{
  const examples=table(await read('covers'),'cover-presence-examples');
  assert.equal(examples.length,2);
  for(const [state,expected] of examples){
    const result=presence({original:{digest:'practice-cover',picture_type:'CoverFront',description:''},all_files:state==='apply-to-all',files:1,total:2});
    assert.equal(result.text,expected);
    assert.equal(result.full,state==='apply-to-all');
  }
});
