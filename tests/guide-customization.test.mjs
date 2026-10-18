import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>{
  const text=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(text,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const {findMatcher,findInList,findStrategies}=await load('find');
const {defaultText,checkState}=await load('tag-panel');
const {editKey,sortTemplate}=await load('columns');
const read=name=>readFile(new URL(`../docs/user-guide/${name}.md`,import.meta.url),'utf8');
const literal=cell=>cell.startsWith('`')?cell.slice(1,-1):cell;
const table=(page,marker)=>page.split(`<!-- ${marker}:start -->`)[1].split(`<!-- ${marker}:end -->`)[0]
  .split(/\r?\n/).filter(line=>line.startsWith('| `')||line.startsWith('| **'))
  .map(line=>line.split('|').slice(1,-1).map(cell=>cell.trim()));

test('Find chapter exercises all actual strategy labels and its written edge cases',async()=>{
  const rows=table(await read('guides/find'),'find-strategy-examples');
  assert.equal(rows.length,8);const seen=new Set();
  for(const [label,criteria,value,expected] of rows){
    const strategy=findStrategies.find(([,name])=>name===label.replaceAll('**',''))?.[0];
    assert.ok(strategy,label);seen.add(strategy);
    if(strategy==='regex')continue; // Native documentation cases run in src-tauri/src/find.rs.
    assert.equal(findMatcher(literal(criteria),strategy,false)(literal(value)),expected==='Match',label+' '+value);
  }
  assert.deepEqual([...seen].sort(),findStrategies.map(([id])=>id).sort());
});

test('Find chapter distinguishes result rows from unique files using the written sample',async()=>{
  const page=await read('guides/find'),criteria=page.match(/use \*\*Contains\*\* with `([^`]+)`/)[1];
  const rows=table(page,'find-result-example').map(cells=>cells.map(literal));
  assert.equal(rows.length,2);
  const files=rows.map(([filename,title,comment])=>({filename,path:`/Practice/${filename}`,fields:{TITLE:[title],...(comment==='*(absent)*'?{}:{COMMENT:[comment]})}}));
  const matches=findInList(files,criteria,'contains',false);
  const expected=page.match(/Expect \*\*(\d+) matches in (\d+) files\*\*/);
  assert.equal(matches.length,Number(expected[1]));
  assert.equal(new Set(matches.map(row=>row.path)).size,Number(expected[2]));
  assert.deepEqual(matches.map(row=>row.field),['Filename','TITLE','COMMENT']);
});

test('panel default table preserves keep/blank and literal expressions',async()=>{
  const rows=table(await read('customize/tag-panel'),'panel-default-examples');
  assert.equal(rows.length,4);
  for(const [value,expected] of rows){
    const actual=defaultText({field:'MOOD',default_value:literal(value)});
    assert.equal(actual,expected==='No update'?null:expected==='Remove field'?'':literal(expected));
  }
});

test('panel checkbox table interprets loaded values from both files',async()=>{
  const rows=table(await read('customize/tag-panel'),'panel-check-examples');
  assert.equal(rows.length,3);
  for(const [a,b,expected] of rows){
    const values=[a,b].map(value=>value==='*(absent)*'?'':literal(value));
    assert.equal(checkState(values),expected.toLowerCase());
  }
});

test('column examples keep display, edit target and sort template separate',async()=>{
  const rows=table(await read('customize/columns'),'column-expression-examples');
  assert.equal(rows.length,3);
  const expected=['ISRC','ARTIST',''];
  rows.forEach((cells,index)=>{
    const value=literal(cells[1]),field=cells[2]==='*(empty)*'?'':literal(cells[2]),sort=cells[3]==='*(empty)*'?'':literal(cells[3]);
    assert.equal(editKey({field}),expected[index]);
    assert.equal(sortTemplate({value,sort_by:sort}),sort||value);
  });
});
