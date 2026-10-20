import test from 'node:test';
import assert from 'node:assert/strict';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const {ExtendedTags}=await loadProductionModule('extended-tags.ts');
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function fixture({files=['/one.flac'],mode='pending',titles=['Before']}={}){
  const window=new Window(),names=['window','document','HTMLElement','HTMLInputElement','HTMLTextAreaElement','MutationObserver'];
  const previous=names.map(name=>Object.getOwnPropertyDescriptor(globalThis,name));
  names.forEach(name=>globalThis[name]=name==='window'?window:window[name]);
  const state={selection:[...files],fields:new Map(files.map((path,index)=>[path,{TITLE:[titles[index]??titles[0]]}])) ,staged:[],saves:[],errors:[],fail:false,partial:false,gate:undefined,done:[],reason:undefined,coverBindings:0};
  state.disk=structuredClone(state.fields);
  const host={selection:()=>state.selection,listed:()=>['/one.flac','/two.flac'],fields:path=>state.fields.get(path)??{TITLE:['Other']},describe:path=>({name:path}),
    stage:changes=>{state.staged.push(changes);for(const {path,updates} of changes)state.fields.set(path,{...state.fields.get(path),...updates});},
    select:path=>{state.selection=[path];},completions:()=>[],autocomplete:()=>undefined,coverMarkup:()=>'<button data-cover-fixture>Add cover</button>',bindCover:()=>state.coverBindings++,
    modal:(_title,content,buttons)=>{const dialog=window.document.createElement('dialog');dialog.innerHTML=`<header></header><div class="dialog-content">${content}</div><footer>${buttons}</footer>`;window.document.body.append(dialog);dialog.showModal();return dialog;},
    modalError:(_dialog,error)=>state.errors.push(String(error)),done:paths=>state.done.push(paths),saveMode:()=>mode,saveReason:()=>state.reason,
    save:async paths=>{state.saves.push(structuredClone(paths));await state.gate;if(state.fail){if(state.partial)state.disk.set(paths[0],structuredClone(state.fields.get(paths[0])));throw new Error('Controlled save refusal');}for(const path of paths)state.disk.set(path,structuredClone(state.fields.get(path)));}};
  const ui=new ExtendedTags(host);ui.show();
  const edit=(value='After')=>{const input=window.document.querySelector('textarea');input.value=value;input.dispatchEvent(new window.Event('input',{bubbles:true}));};
  return{window,document:window.document,state,ui,edit,close(){window.happyDOM.abort();names.forEach((name,i)=>{if(previous[i])Object.defineProperty(globalThis,name,previous[i]);else delete globalThis[name];});}};
}
test('Extended Tags saves the displayed selection together and closes only after save completes',async()=>{
  const p=fixture({files:['/one.flac','/two.flac']});try{
    let finish;p.state.gate=new Promise(resolve=>finish=resolve);p.edit();
    const save=p.document.querySelector('#save-extended');assert.ok(save,'the dialog needs an explicit Save tags action');save.click();await tick();
    assert.deepEqual(p.state.saves,[['/one.flac','/two.flac']]);assert.equal(p.state.staged[0].length,2);assert.equal(p.ui.open,true);
    save.click();await tick();assert.equal(p.state.saves.length,1,'a pending save cannot be submitted twice');
    finish();await tick();assert.equal(p.ui.open,false);
  }finally{p.close();}
});
test('a failed Extended Tags save keeps the dialog, values and retry available',async()=>{
  const p=fixture();try{p.edit();p.state.fail=true;p.document.querySelector('#save-extended').click();await tick();
    assert.equal(p.ui.open,true);assert.equal(p.document.querySelector('textarea').value,'After');assert.match(p.state.errors[0],/Controlled save refusal/);
    assert.equal(p.document.querySelector('#save-extended').disabled,false);p.state.fail=false;p.document.querySelector('#save-extended').click();await tick();assert.equal(p.ui.open,false);
  }finally{p.close();}
});
test('retry or Keep changes after a failed name-only rename retains each file original values',async()=>{
  for(const partial of [false,true])for(const retry of ['save','keep']){const p=fixture({files:['/one.flac','/two.flac'],titles:['First title','Second title']});try{
    const name=p.document.querySelector('.extended-name input');name.value='SESSION_NOTE';name.dispatchEvent(new p.window.Event('input',{bubbles:true}));
    p.state.fail=true;p.state.partial=partial;p.document.querySelector('#save-extended').click();await tick();assert.equal(p.ui.open,true);
    p.state.fail=false;p.document.querySelector(retry==='save'?'#save-extended':'#apply-extended').click();await tick();
    assert.deepEqual(p.state.fields.get('/one.flac').SESSION_NOTE,['First title']);assert.deepEqual(p.state.fields.get('/two.flac').SESSION_NOTE,['Second title']);
    if(retry==='save'){assert.deepEqual(p.state.disk.get('/one.flac').SESSION_NOTE,['First title']);assert.deepEqual(p.state.disk.get('/two.flac').SESSION_NOTE,['Second title']);}
    assert.equal(p.ui.open,false);
  }finally{p.close();}}
});
test('an in-flight save blocks fresh field menus and cover redraws until a failure returns',async()=>{
  const p=fixture();try{p.edit();p.state.fail=true;let finish;p.state.gate=new Promise(resolve=>finish=resolve);p.document.querySelector('#save-extended').click();await tick();
    const row=p.document.querySelector('.extended-row');row.dispatchEvent(new p.window.MouseEvent('contextmenu',{bubbles:true,cancelable:true}));
    row.dispatchEvent(new p.window.KeyboardEvent('keydown',{bubbles:true,cancelable:true,key:'F10',shiftKey:true}));assert.equal(p.document.querySelector('.extended-menu'),null);
    const bindings=p.state.coverBindings;p.ui.refreshCover();assert.equal(p.state.coverBindings,bindings);assert.equal(p.document.querySelector('[data-cover-fixture]').disabled,true);
    finish();await tick();assert.equal(p.ui.open,true);assert.equal(p.document.querySelector('[data-cover-fixture]').disabled,false);assert.ok(p.state.coverBindings>bindings);
  }finally{p.close();}
});
test('correcting a failed field rename removes the superseded staged field, including partial saves',async()=>{
  for(const partial of [false,true])for(const replacement of ['TITLE','CORRECTED_NOTE']){
    const p=fixture({files:['/one.flac','/two.flac'],titles:['First title','Second title']});try{
      const rename=value=>{const input=p.document.querySelector('.extended-name input');input.value=value;input.dispatchEvent(new p.window.Event('input',{bubbles:true}));};
      rename('SESSION_NOTE');p.state.fail=true;p.state.partial=partial;p.document.querySelector('#save-extended').click();await tick();
      rename(replacement);p.state.fail=false;p.document.querySelector('#save-extended').click();await tick();
      for(const [path,title] of [['/one.flac','First title'],['/two.flac','Second title']]){
        assert.deepEqual(p.state.disk.get(path)[replacement],[title]);assert.deepEqual(p.state.disk.get(path).SESSION_NOTE,[],'superseded field must be removed');
      }
    }finally{p.close();}
  }
});
test('Keep changes stays explicit and never writes tags',()=>{
  const p=fixture();try{p.edit();p.document.querySelector('#apply-extended').click();assert.equal(p.ui.open,false);assert.equal(p.state.staged.length,1);assert.deepEqual(p.state.saves,[]);}finally{p.close();}
});
test('Extended Tags save navigation writes the file being left before changing selection',async()=>{
  const p=fixture({mode:'save'});try{let finish;p.state.gate=new Promise(resolve=>finish=resolve);p.edit();p.document.querySelector('[data-nav="1"]').click();await tick();
    assert.deepEqual(p.state.saves,[['/one.flac']]);assert.deepEqual(p.state.selection,['/one.flac']);assert.equal(p.document.querySelector('textarea').disabled,true);finish();await tick();assert.deepEqual(p.state.selection,['/two.flac']);assert.equal(p.document.querySelector('textarea').disabled,false);
  }finally{p.close();}
});
test('failed navigation save keeps the current file and editable pending values',async()=>{
  const p=fixture({mode:'save'});try{p.edit();p.state.fail=true;p.document.querySelector('[data-nav="1"]').click();await tick();
    assert.deepEqual(p.state.selection,['/one.flac']);assert.equal(p.document.querySelector('textarea').value,'After');assert.equal(p.document.querySelector('textarea').disabled,false);assert.equal(p.state.errors.length,1);
  }finally{p.close();}
});
test('Ask navigation honors discard and cancel without writing the edited file',async()=>{
  for(const answer of ['discard','cancel']){const p=fixture({mode:'ask'});try{p.edit();p.document.querySelector('[data-nav="1"]').click();await tick();
    const ask=p.document.querySelector('.extended-ask-dialog');assert.ok(ask);if(answer==='discard')ask.querySelector('#extended-discard').click();else ask.close();await tick();
    assert.deepEqual(p.state.selection,answer==='discard'?['/two.flac']:['/one.flac']);assert.deepEqual(p.state.saves,[]);assert.deepEqual(p.state.staged,[]);
  }finally{p.close();}}
});
test('a current save-back refusal keeps Keep changes available',async()=>{
  const p=fixture();try{p.edit();p.state.reason='Read-only source';p.document.querySelector('#save-extended').click();await tick();assert.deepEqual(p.state.saves,[]);assert.equal(p.ui.open,true);
    p.document.querySelector('#apply-extended').click();assert.equal(p.state.staged.length,1);assert.equal(p.ui.open,false);
  }finally{p.close();}
});
test('pending navigation does not turn the final save into a write to earlier files',async()=>{
  const p=fixture();try{p.edit('First draft');p.document.querySelector('[data-nav="1"]').click();await tick();assert.deepEqual(p.state.saves,[]);
    p.edit('Second draft');p.document.querySelector('#save-extended').click();await tick();assert.deepEqual(p.state.saves,[['/two.flac']]);
    assert.equal(p.state.fields.get('/one.flac').TITLE[0],'First draft');
  }finally{p.close();}
});
test('the production Extended Tags selection adapter does not invoke general save-on-move',async()=>{
  const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8'),parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
  const declaration=parsed.statements.flatMap(node=>ts.isVariableStatement(node)?[...node.declarationList.declarations]:[]).find(node=>node.name.getText(parsed)==='extendedTags');
  const select=declaration.initializer.arguments[0].properties.find(node=>node.name?.getText(parsed)==='select').initializer.getText(parsed);
  const selected=new Set(['/one.flac']),calls=[];
  const env={paths:()=>[...selected],selected,anchor:'/one.flac',coverIndex:0,coverError:undefined,activeCover:undefined,
    refreshSelection:()=>{},revealRow:()=>{},saveOnMove:()=>calls.push('unexpected save')};
  const js=ts.transpileModule(`return (${select});`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  new Function(...Object.keys(env),js)(...Object.values(env))('/two.flac');
  assert.deepEqual([...selected],['/two.flac']);assert.deepEqual(calls,[],'Extended Tags owns its ask/ignore/save/pending decision');
});

test('production partial selection refresh shows pending edits from the file left in Extended Tags',async()=>{
  const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8'),parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
  const functions=parsed.statements.filter(node=>ts.isFunctionDeclaration(node)&&['refreshSelection','refreshDraftCount'].includes(node.name?.text)).map(node=>node.getText(parsed)).join('\n');
  const window=new Window(),document=window.document;
  document.body.innerHTML='<div class="fields"></div><span class="panel-count"></span><div class="filterbar"><span class="count"></span></div><div class="selection-info"></div><footer><span class="status-message">Ready</span></footer>';
  const drafts=new Map([['/one.flac',{TITLE:['Pending Via Extended']}]]),selected=new Set(['/two.flac']);
  const entries=new Map(['/one.flac','/two.flac'].map(path=>[path,{path}])),noop=()=>{};
  const env={document,drafts,selected,entries,HTMLInputElement:window.HTMLInputElement,HTMLTextAreaElement:window.HTMLTextAreaElement,
    applyAutoDefaults:()=>false,runtimePlatform:{document_access_supported:false},compactScreen:()=>false,selectedProvider:()=>false,
    reviewedConverterContextChanged:undefined,visible:()=>[...entries.values()],paths:()=>[...selected],
    updateVirtualRows:noop,cellEditor:{reattach:noop},tagPanel:{fieldsHTML:()=>'',bind:noop},bindFields:noop,
    panelCount:()=> '1 file selected',countText:()=> '2 files',selectionInfoMarkup:()=>'',refreshCommandStates:noop,
    refreshCoverPanel:noop,hydrateActiveCover:noop,timing:noop,render:()=>assert.fail('Exercise the partial refresh'),
    invoke:()=>assert.fail('Selection refresh must not write audio')};
  try{
    const js=ts.transpileModule(`${functions}\nreturn refreshSelection;`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const refresh=new Function(...Object.keys(env),js)(...Object.values(env));refresh();
    assert.equal(document.querySelector('.dirty-count')?.textContent,'1 unsaved file');
    assert.deepEqual(drafts.get('/one.flac'),{TITLE:['Pending Via Extended']});assert.deepEqual([...selected],['/two.flac']);
    drafts.clear();refresh();assert.equal(document.querySelector('.dirty-count'),null);
  }finally{window.happyDOM.abort();}
});
