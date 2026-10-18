import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';

const source=await readFile(new URL('../src/action-ui.ts',import.meta.url),'utf8');
const javascript=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {ActionUI,rememberQuick,recallQuick}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const key='opentag.actions.quick';
const memory=()=>{const values=new Map();return{getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)};};
const settle=async()=>{for(let i=0;i<4;i++)await Promise.resolve();};

function page(storage=memory(),actions=[]){
  const window=new Window(),globals={};
  for(const name of ['window','document','navigator','HTMLInputElement','HTMLSelectElement','HTMLTextAreaElement','MouseEvent']){
    globals[name]=Object.getOwnPropertyDescriptor(globalThis,name);
    Object.defineProperty(globalThis,name,{configurable:true,writable:true,value:name==='window'?window:window[name]});
  }
  const executed=[],saved=[];
  const library={groups:[{name:'Practice',enabled:true,actions:structuredClone(actions)}],presets:[]};
  const host={storage,load:async()=>structuredClone(library),save:async value=>saved.push(structuredClone(value)),execute:async groups=>executed.push(structuredClone(groups)),confirm:async()=>true,canExecute:()=>true,selectedCount:()=>1,changed:()=>{}};
  const ui=()=>new ActionUI(host);
  const closeDialogs=()=>{for(const dialog of [...window.document.querySelectorAll('dialog')].reverse())dialog.close();};
  return{window,storage,executed,saved,ui,closeDialogs,async groupEditor(instance=ui()){
    await instance.groupsDialog();
    const groups=window.document.querySelector('.action-groups-dialog');
    groups.querySelector('[data-command="edit"]').click();
    return window.document.querySelector('.group-editor-dialog');
  },async close(){closeDialogs();await window.happyDOM.abort();for(const[name,descriptor]of Object.entries(globals))if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];}};
}
// Happy DOM can retain an earlier default option while parsing selected markup.
// Normalize that initial selection; production change/click handlers remain real.
function initialSelections(root){for(const select of root.querySelectorAll('select')){const selected=select.querySelector('option[selected]');if(selected)select.value=selected.value;}return root;}
const editor=p=>initialSelections(p.window.document.querySelector('dialog:has(#full-action-type)'));
function selectType(p,dialog,type){const select=dialog.querySelector('#full-action-type');select.value=type;select.dispatchEvent(new p.window.Event('change'));initialSelections(dialog.querySelector('#full-action-controls'));}
function newCodepage(p,group){
  group.querySelector('[data-command="new"]').click();
  const dialog=editor(p);
  selectType(p,dialog,'convert_codepage');
  return dialog;
}
const codepage=dialog=>dialog.querySelector('[data-key="codepage"]');
async function keep(dialog){dialog.querySelector('[data-keep]').click();await settle();assert.equal(dialog.isConnected,false);}

test('confirmed group codepage survives another editor and restart even when outer group is discarded',async()=>{
  const p=page();try{
    const group=await p.groupEditor();
    let dialog=newCodepage(p,group);
    assert.equal(codepage(dialog).value,'1251');
    dialog.querySelector('[data-key="field"]').value='ARTIST';
    codepage(dialog).value='1252';await keep(dialog);
    dialog=newCodepage(p,group);
    assert.equal(codepage(dialog).value,'1252');
    assert.equal(dialog.querySelector('[data-key="field"]').value,'TITLE','only codepage becomes a new-action default');
    p.closeDialogs();
    const restarted=await p.groupEditor(p.ui());
    dialog=newCodepage(p,restarted);
    assert.equal(codepage(dialog).value,'1252');
    assert.deepEqual(p.saved,[],'remembering the editor choice does not save discarded group contents');
    assert.deepEqual(p.executed,[]);
  }finally{await p.close();}
});

test('cancelled and switched-away codepage edits do not replace the last confirmed group choice',async()=>{
  const storage=memory();storage.setItem(key,JSON.stringify({actions:{},groupCodepage:'850'}));
  const p=page(storage);try{
    const group=await p.groupEditor();
    let dialog=newCodepage(p,group);assert.equal(codepage(dialog).value,'850');
    codepage(dialog).value='932';dialog.querySelector('[data-close]').click();
    dialog=newCodepage(p,group);assert.equal(codepage(dialog).value,'850');
    codepage(dialog).value='65001';selectType(p,dialog,'format_value');await keep(dialog);
    dialog=newCodepage(p,group);assert.equal(codepage(dialog).value,'850');
    assert.equal(JSON.parse(storage.getItem(key)).groupCodepage,'850');
    codepage(dialog).value='';dialog.querySelector('[data-keep]').click();await settle();
    assert.match(dialog.querySelector('.dialog-error').textContent,/codepage/);
    assert.equal(JSON.parse(storage.getItem(key)).groupCodepage,'850','invalid confirmation leaves preference intact');
  }finally{await p.close();}
});

test('group codepage and complete Quick settings survive writes in both directions',async()=>{
  const storage=memory();
  const converted={type:'convert_codepage',field:'ARTIST',codepage:'932'};
  const replaced={type:'replace',field:'ALBUM',from:'old',to:'new',whole_word:true,case_sensitive:false};
  rememberQuick(storage,converted);rememberQuick(storage,replaced);
  const before=JSON.parse(storage.getItem(key));
  const p=page(storage);try{
    const group=await p.groupEditor();const dialog=newCodepage(p,group);
    codepage(dialog).value='850';await keep(dialog);
    const grouped=JSON.parse(storage.getItem(key));
    assert.equal(grouped.groupCodepage,'850');
    assert.equal(grouped.last,before.last);assert.deepEqual(grouped.actions,before.actions);
    p.closeDialogs();
    const quick=p.ui();quick.quick('convert_codepage');
    const quickEditor=editor(p);
    assert.equal(codepage(quickEditor).value,'932');assert.equal(quickEditor.querySelector('[data-key="field"]').value,'ARTIST');
    codepage(quickEditor).value='65001';await keep(quickEditor);
    assert.deepEqual(recallQuick(storage),{...converted,codepage:'65001'});
    assert.equal(JSON.parse(storage.getItem(key)).groupCodepage,'850','Quick write preserves group choice');
    const next=await p.groupEditor();assert.equal(codepage(newCodepage(p,next)).value,'850');
    assert.deepEqual(p.executed,[[{name:'Quick action',enabled:true,actions:[{...converted,codepage:'65001'}]}]]);
  }finally{await p.close();}
});

for(const importedCodepage of ['932','windows-1252'])test(`existing imported codepage ${importedCodepage} and unresolved action remain intact when group memory differs`,async()=>{
  const storage=memory();storage.setItem(key,JSON.stringify({actions:{},groupCodepage:'850'}));
  const imported={type:'convert_codepage',field:'ARTIST',codepage:importedCodepage};
  const unresolved={type:'unresolved_mta',type_id:'9999',properties:{T:'9999','1':'retained'},reason:'Unknown action'};
  const p=page(storage,[imported,unresolved]);try{
    const group=await p.groupEditor();group.querySelector('[data-command="edit"]').click();
    let dialog=editor(p);
    assert.equal(codepage(dialog).value,importedCodepage);await keep(dialog);
    group.querySelector('[data-action-index="1"]').click();group.querySelector('[data-command="edit"]').click();
    dialog=editor(p);
    assert.equal(dialog.querySelector('#full-action-type').value,'unresolved_mta');assert.equal(dialog.querySelector('[data-keep]').disabled,true);
    dialog.querySelector('[data-close]').click();await keep(group);
    p.window.document.querySelector('#save-groups').click();await settle();
    assert.deepEqual(p.saved[0].groups[0].actions,[imported,unresolved]);
    assert.equal(JSON.parse(storage.getItem(key)).groupCodepage,importedCodepage==='932'?'932':'850','only offered codepages become new-action defaults');
  }finally{await p.close();}
});

for(const stored of ['{',JSON.stringify({actions:{},groupCodepage:'not-a-codepage'}),JSON.stringify({actions:{},groupCodepage:1252})])test(`invalid group codepage memory falls back safely: ${stored}`,async()=>{
  const storage=memory();storage.setItem(key,stored);const p=page(storage);try{
    const group=await p.groupEditor();assert.equal(codepage(newCodepage(p,group)).value,'1251');
  }finally{await p.close();}
});

test('unavailable preference storage does not prevent confirming a group codepage action',async()=>{
  const p=page({getItem:()=>{throw new Error('denied');},setItem:()=>{throw new Error('quota');}});try{
    const group=await p.groupEditor();const dialog=newCodepage(p,group);assert.equal(codepage(dialog).value,'1251');
    codepage(dialog).value='1252';await keep(dialog);await keep(group);
    p.window.document.querySelector('#save-groups').click();await settle();
    assert.deepEqual(p.saved[0].groups[0].actions,[{type:'convert_codepage',field:'TITLE',codepage:'1252'}]);
  }finally{await p.close();}
});
