import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

const source=await readFile(new URL('../src/theme.ts',import.meta.url),'utf8');
const javascript=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {createAppearance,parseAppearance,resolveAppearance,appearanceStorageKey}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
function fixture(value=null,dark=false){
  const values=new Map(value===null?[]:[[appearanceStorageKey,value]]),listeners=new Set(),changes=[];
  const media={matches:dark,addEventListener:(_,listener)=>listeners.add(listener),removeEventListener:(_,listener)=>listeners.delete(listener)};
  const storage={getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)};
  const controller=createAppearance({media,storage,changed:state=>changes.push(state)});
  return {controller,values,changes,media,storage,system(dark){media.matches=dark;for(const listener of listeners)listener();}};
}
test('appearance defaults to System and rejects unknown persisted choices',()=>{
  for(const value of [null,undefined,'','invalid',{},'DARK'])assert.equal(parseAppearance(value),'system');
  assert.equal(resolveAppearance('system',true),'dark');assert.equal(resolveAppearance('system',false),'light');
});
test('System follows OS changes without persisting an explicit mode',()=>{
  const f=fixture();assert.deepEqual(f.controller.state,{preference:'system',resolved:'light'});
  f.system(true);assert.deepEqual(f.changes,[{preference:'system',resolved:'dark'}]);assert.equal(f.values.size,0);
});
test('explicit choice persists and ignores OS changes until System is selected',()=>{
  const f=fixture(null,true);f.controller.set('light');f.system(false);f.system(true);
  assert.deepEqual(f.changes,[{preference:'light',resolved:'light'}]);assert.equal(f.values.get(appearanceStorageKey),'light');
  f.controller.set('system');assert.deepEqual(f.controller.state,{preference:'system',resolved:'dark'});f.system(false);
  assert.deepEqual(f.changes.at(-1),{preference:'system',resolved:'light'});
});
test('a fresh controller restores the manual selection from the same storage',()=>{
  const f=fixture();f.controller.set('dark');f.controller.dispose();
  const reopened=createAppearance({media:f.media,storage:f.storage,changed:()=>{}});
  assert.deepEqual(reopened.state,{preference:'dark',resolved:'dark'});
});
test('unavailable storage does not prevent manual appearance or OS following',()=>{
  const media={matches:false,addEventListener(){},removeEventListener(){}};
  const controller=createAppearance({media,storage:{getItem(){throw Error('denied');},setItem(){throw Error('denied');}},changed(){}});
  assert.equal(controller.state.preference,'system');assert.doesNotThrow(()=>controller.set('dark'));assert.equal(controller.state.resolved,'dark');
});
test('disposed controllers stop receiving system-change updates',()=>{
  const f=fixture();f.controller.dispose();f.system(true);assert.deepEqual(f.changes,[]);
});
