import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
const source=await readFile(new URL('../src/action-ui.ts',import.meta.url),'utf8');
const javascript=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {ActionUI,rememberQuick}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
for(const [name,resize] of [
  ['disabled square',{pixels:0,size_mode:'maximum',scale_mode:'square',size_enabled:false,native_geometry:true}],
  ['enabled zero no-op',{pixels:0,size_mode:'maximum',scale_mode:'proportional',size_enabled:true,native_geometry:true}],
  ['legacy crop',{pixels:600,size_mode:'maximum',scale_mode:'crop_square'}],
])test(`Quick editor preserves imported source and ${name} semantics`,async()=>{
  const window=new Window();
  const globals=['window','document','HTMLElement','HTMLInputElement','HTMLSelectElement','HTMLTextAreaElement'];
  const saved=Object.fromEntries(globals.map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
  for(const key of globals)Object.defineProperty(globalThis,key,{configurable:true,value:key==='window'?window:window[key]});
  try{
    const data=new Map(),storage={getItem:key=>data.get(key)??null,setItem:(key,value)=>data.set(key,value)};
    const action={type:'adjust_cover',format:'original',quality:80,resize,mta_source:{properties:{T:'17','1':'0','2':'0|scaleToSquare|max|disabled','3':'10'}}};
    rememberQuick(storage,action);
    let actual;
    const ui=new ActionUI({storage,canExecute:()=>true,selectedCount:()=>1,execute:async groups=>{actual=groups[0].actions[0];}});
    ui.quick('adjust_cover');
    const dialog=window.document.querySelector('dialog');
    assert.equal(dialog.querySelector('[data-key="pixels"]').value,String(resize.pixels));
    assert.equal(dialog.querySelector('[data-key="resize_enabled"]').checked,resize.size_enabled??true);
    assert.equal(dialog.querySelector('.resize-options').hidden,false);
    // Happy DOM 21 selects the previous option for a trailing selected attribute.
    // Verify authored selection, then apply the browser's specified select value.
    const shape=dialog.querySelector('[data-key="scale_mode"]');
    assert.equal(shape.querySelector('option[selected]').value,resize.scale_mode);
    shape.value=resize.scale_mode;
    dialog.querySelector('[data-keep]').click();
    for(let i=0;i<5;i++)await Promise.resolve();
    assert.deepEqual(actual,action);
  }finally{
    await window.happyDOM.abort();
    for(const key of globals)if(saved[key])Object.defineProperty(globalThis,key,saved[key]);else delete globalThis[key];
  }
});
