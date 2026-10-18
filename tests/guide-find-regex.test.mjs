import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
import { Window } from 'happy-dom';

const source = await readFile(new URL('../src/find.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {
  compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022},
}).outputText;
const { findInListAsync, FindUI } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const items = [{path:'/Practice/a.flac', filename:'a.flac', fields:{TITLE:['Paper Boats','Paper Boats Live'], ARTIST:['Guest'], _PATH:['/hidden']}}];

test('regex Find sends raw Boost expressions and one ordered value batch to the native matcher', async () => {
  const calls=[];
  const result=await findInListAsync(items, String.raw`(?i)\APaper Boats\z`, 'regex', true, async(...args)=>{
    calls.push(args);return [false,true,false,false];
  });
  assert.deepEqual(calls, [[String.raw`(?i)\APaper Boats\z`,true,['a.flac','Paper Boats','Paper Boats Live','Guest']]]);
  assert.deepEqual(result,[{path:'/Practice/a.flac',filename:'a.flac',field:'TITLE',value:'Paper Boats'}]);
  assert.throws(()=>new RegExp(calls[0][0], 'u'),SyntaxError,'pattern cannot be implemented by JS Unicode regex');
  await assert.rejects(findInListAsync(items,'(', 'regex',false,async()=>{throw 'Invalid regular expression: unmatched parenthesis';}),/Invalid regular expression/);
  await assert.rejects(findInListAsync(items,'x', 'regex',false,async()=>[true]),/Incomplete/);
  await assert.rejects(findInListAsync([], '(', 'regex',false,async()=>{throw new Error('Invalid regular expression');}),/Invalid regular expression/,'invalid patterns must be validated for an empty list');
  await assert.rejects(findInListAsync(items,'','regex',false,async()=>assert.fail('empty pattern')),/Enter text/);
  const literal=await findInListAsync(items,'Boats','contains',false,async()=>assert.fail('literal strategy must stay local'));
  assert.equal(literal.length,2);
});

test('Find ignores stale success/errors after edits, newer searches and closing', async () => {
  const window=new Window();
  globalThis.document=window.document;
  const dialogs=window.HTMLDialogElement.prototype;
  dialogs.show=function(){this.open=true;};
  dialogs.close=function(){this.open=false;this.dispatchEvent(new window.Event('close'));};
  const pending=[];
  const ui=new FindUI({items:()=>items,select:()=>{},listed:()=>true,regex:()=>new Promise((resolve,reject)=>pending.push({resolve,reject}))});
  const flush=async()=>{await Promise.resolve();await Promise.resolve();await Promise.resolve();};
  const setup=()=>{
    ui.open();
    const dialog=document.querySelector('dialog'),input=dialog.querySelector('#find-criteria'),strategy=dialog.querySelector('#find-strategy');
    strategy.value='regex';input.value='first';
    return {dialog,input,run:()=>dialog.querySelector('#find-run').click(),status:()=>dialog.querySelector('#find-status').textContent};
  };
  const a=setup();a.run();
  a.input.value='second';a.input.dispatchEvent(new window.Event('input'));a.run();
  pending[1].resolve([false,true,false,false]);await flush();
  assert.match(a.status(),/1 match in 1 file/);
  pending[0].reject('Old error');await flush();
  assert.match(a.status(),/1 match in 1 file/);
  a.run();a.input.value='third';a.input.dispatchEvent(new window.Event('input'));
  pending[2].resolve([true,true,true,true]);await flush();
  assert.match(a.status(),/Criteria changed/);
  assert.equal(a.dialog.querySelectorAll('tbody tr').length,0);
  a.run();a.dialog.close();const b=setup();
  pending[3].resolve([true,true,true,true]);await flush();
  assert.match(b.status(),/Search all tag fields/);
  b.dialog.close();window.happyDOM.abort();delete globalThis.document;
});
