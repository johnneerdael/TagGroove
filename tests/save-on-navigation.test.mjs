import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';

const behaviour=await loadProductionModule('list-behaviour.ts');
const columns=await loadProductionModule('columns.ts');
const commands=await loadProductionModule('list-commands.ts');
const {SavedConfigurationWrites}=await loadProductionModule('configuration-transfer.ts');
const {reconcileDrafts}=await loadProductionModule('document-state.ts');
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const names=new Set(['select','saveOnMove','saveDrafts','run','persistColumns','applyColumns','shuffleFiles','listOperation','operateImpl']);
const functions=parsed.statements.filter(node=>ts.isFunctionDeclaration(node)&&names.has(node.name?.text)).map(node=>node.getText(parsed)).join('\n');
let rowClick;
function locateRowClick(node){
  if(ts.isCallExpression(node)&&node.expression.getText(parsed)==='wrap.addEventListener'&&node.arguments[0]?.text==='click')rowClick=node.arguments[1].getText(parsed);
  ts.forEachChild(node,locateRowClick);
}
locateRowClick(parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='bind'));
const transpile=text=>ts.transpileModule(text,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const deferred=()=>{let resolve;return{promise:new Promise(done=>resolve=done),resolve};};
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));

function fixture(state={}){
  const window=new Window(),document=window.document,calls=[];
  const entries=new Map(['a','b','c'].map(path=>[path,{path,filename:path,fields:{TITLE:[path]}}]));
  const selected=new Set(state.selected??['a']),drafts=new Map(),pendingCovers=new Map();
  for(const path of state.dirty??['a'])drafts.set(path,{TITLE:['Edited '+path]});
  if(state.coverOnly){drafts.clear();pendingCovers.set('a',[{data_base64:'AA==',mime:'image/png'}]);}
  document.body.innerHTML='<main id="app"></main>';
  const render=()=>{document.querySelector('#app').innerHTML='<div class="fields"><input data-field="TITLE"></div><div class="table-wrap"><table><tbody>'+[...entries.keys()].map(path=>`<tr tabindex="0" data-path="${path}"><td><input type="checkbox" class="row-select" ${selected.has(path)?'checked':''}>${path}</td></tr>`).join('')+'</tbody></table></div>';};
  render();
  const configurationWrites=new SavedConfigurationWrites();
  const dependencies={state,window,document,entries,selected,drafts,pendingCovers,calls,render,reconcileDrafts,configurationWrites,...behaviour,...columns,...commands,
    HTMLElement:window.HTMLElement,HTMLTableRowElement:window.HTMLTableRowElement,MouseEvent:window.MouseEvent,CSS:window.CSS,
    invoke:async(command,args)=>{
      calls.push([command,structuredClone(args),[...selected]]);
      if(command==='save_columns'){
        if(state.columnGate)await state.columnGate.promise;
        if(state.columnFailures){state.columnFailures--;throw new Error('Column save refused');}
        return;
      }
      assert.equal(command,'save_fields_batch');
      if(state.gate)await state.gate.promise;
      if(state.fail)throw new Error('Save refused');
      const saved=state.cancel?[]:args.items.filter((_,index)=>!state.partial||index===0).map(item=>({...entries.get(item.path),fields:item.updates}));
      return{entries:saved,errors:state.partial?[{path:'b',error:'Save refused'}]:[]};
    }};
  const js=transpile(`
    const desktop=state.desktop??true,crypto=window.crypto;
    let busy=state.busy??false,selectionChangePending=false,anchor=[...selected][0],coverIndex=0,coverError,activeCover;
    let status='',errors=[],progressJob,orderCacheKey='',filterCacheKey='',listOrder='manual',columnLayout=defaultLayout(),columnSaveQueue=Promise.resolve();
    const copyOverrides=new Map(),workingReadErrors=new Map(),queuedNotices=[];let autoPlaylistPending=false;
    const optionsUI={page:()=>({save_on_move:state.enabled??true})},enabled=()=>true;
    const providerEntry=entry=>state.providers?.includes(entry.path)??false;
    const ordered=()=>[...entries.values()],visible=ordered,paths=()=>ordered().filter(entry=>selected.has(entry.path)).map(entry=>entry.path);
    const active=()=>entries.get(paths()[0]),rowHeight=24,headerHeight=20,sortValueOf=path=>path;
    const refreshSelection=()=>{},refreshList=()=>{},refreshSourceAccess=async()=>{},refreshHistory=async()=>{},flushNotices=async()=>{};
    const storeEntry=entry=>entries.set(entry.path,entry);
    const revealRow=(path,focus=true)=>{if(focus)document.querySelector('tr[data-path="'+path+'"]').focus();};
    const setListOrder=mode=>{listOrder=mode;};
    const rowFor=event=>event.target.closest('tr[data-path]'),compactScreen=()=>false;
    ${functions}
    return{
      select:(path,event={})=>select(path,event),command:listOperation,
      orderMode:operateImpl,
      sort:(ascending=false)=>applyColumns({...columnLayout,sort:[{id:'filename',ascending}]},false,true),shuffle:shuffleFiles,
      applyVerified:layout=>applyColumns(layout,true),
      bindRowClicks:()=>document.querySelector('.table-wrap').addEventListener('click',${rowClick}),
      selectOutside:targets=>{selected.clear();targets.forEach(path=>selected.add(path));},
      snapshot:()=>({selected:[...selected],dirty:[...drafts.keys()],covers:[...pendingCovers.keys()],order:[...entries.keys()],sort:structuredClone(columnLayout.sort),mode:listOrder,busy,anchor,errors}),
    };
  `);
  dependencies.invokeUntracked=dependencies.invoke;
  const runtime=new Function(...Object.keys(dependencies),js)(...Object.values(dependencies));
  return{...runtime,state,calls,window,document,writes:configurationWrites,close:()=>window.happyDOM.abort()};
}

test('selection waits for its save; a second click cannot skip the pending save',async()=>{
  const gate=deferred(),p=fixture({gate});try{
    const first=p.select('b');assert.deepEqual(p.snapshot().selected,['a']);
    assert.deepEqual(p.calls[0].slice(0,1),['save_fields_batch']);assert.deepEqual(p.calls[0][2],['a']);
    await p.select('c');assert.deepEqual(p.snapshot().selected,['a']);assert.equal(p.calls.length,1);
    gate.resolve();await first;assert.deepEqual(p.snapshot().selected,['b']);assert.deepEqual(p.snapshot().dirty,[]);
  }finally{gate.resolve();await tick();p.close();}
});

test('failed, cancelled and partial saves retain the previous selection and remaining drafts',async()=>{
  for(const outcome of ['fail','cancel','partial']){
    const p=fixture({[outcome]:true,selected:['a','b'],dirty:['a','b']});try{
      await p.select('c');assert.deepEqual(p.snapshot().selected,['a','b'],outcome);
      assert.deepEqual(p.snapshot().dirty,outcome==='partial'?['b']:['a','b']);
    }finally{p.close();}
  }
});

test('Ctrl+N from a tag field waits for save and restores the tag-field focus',async()=>{
  const gate=deferred(),p=fixture({gate});try{
    p.document.querySelector('[data-field="TITLE"]').focus();
    const next=p.command('select-next');assert.deepEqual(p.snapshot().selected,['a']);
    gate.resolve();await next;assert.deepEqual(p.snapshot().selected,['b']);
    assert.equal(p.document.activeElement.dataset.field,'TITLE');
  }finally{gate.resolve();await tick();p.close();}
});

test('selection extension, toggle, invert and empty-area deselection share the awaited save',async()=>{
  for(const [action,expected] of [[p=>p.select('c',{shiftKey:true}),['a','b','c']],[p=>p.select('b',{ctrlKey:true}),['a','b']],[p=>p.command('invert-selection'),['b','c']],[p=>p.command('unselect-all'),[]]]){
    const gate=deferred(),p=fixture({gate});try{
      const change=action(p);assert.deepEqual(p.snapshot().selected,['a']);
      gate.resolve();await change;assert.deepEqual(p.snapshot().selected,expected);
      assert.deepEqual(p.snapshot().dirty,[]);
    }finally{gate.resolve();await tick();p.close();}
  }
});

test('same-selection sort and shuffle save first and refuse incomplete saves without changing order',async()=>{
  for(const operation of ['sort','shuffle'])for(const fail of [false,true]){
    const gate=deferred(),p=fixture({gate,fail});try{
      const before=p.snapshot(),change=p[operation]();
      assert.equal(p.calls[0]?.[0],'save_fields_batch',operation);
      assert.deepEqual(p.snapshot().sort,before.sort);assert.deepEqual(p.snapshot().order,before.order);
      gate.resolve();if(fail&&operation==='sort')await assert.rejects(change,/Column layout was not applied/);else await change;
      if(fail){assert.deepEqual(p.snapshot().sort,before.sort);assert.equal(p.snapshot().mode,before.mode);assert.deepEqual(p.snapshot().order,before.order);}
      else{assert.deepEqual(p.snapshot().dirty,[]);assert.equal(p.snapshot().mode,operation==='sort'?'columns':'manual');}
    }finally{gate.resolve();await tick();p.close();}
  }
});

test('a displayed sort can reverse while preferences save, with persistence kept in order',async()=>{
  const columnGate=deferred(),p=fixture({dirty:[],columnGate});let first,second;
  try{
    first=p.sort(true);await tick();assert.equal(p.snapshot().sort[0].ascending,true);
    second=p.sort(false).then(()=>null,error=>error);await tick();
    assert.equal(p.snapshot().sort[0].ascending,false);
    assert.equal(p.calls.filter(([command])=>command==='save_columns').length,1);
    columnGate.resolve();await first;assert.equal(await second,null);
    assert.deepEqual(p.calls.filter(([command])=>command==='save_columns').map(([,args])=>args.layout.sort[0].ascending),[true,false]);
    assert.equal(p.calls.some(([command])=>command==='save_fields_batch'),false);
  }finally{columnGate.resolve();await Promise.allSettled([first,second]);p.close();}
});

test('a failed preference write does not block the next displayed sort or its persistence',async()=>{
  const columnGate=deferred(),p=fixture({dirty:[],columnGate,columnFailures:1});let first,second;
  try{
    first=p.sort(true).then(()=>null,error=>error);await tick();
    second=p.sort(false).then(()=>null,error=>error);await tick();
    columnGate.resolve();assert.match(String(await first),/Column save refused/);assert.equal(await second,null);
    assert.equal(p.snapshot().sort[0].ascending,false);
    assert.deepEqual(p.calls.filter(([command])=>command==='save_columns').map(([,args])=>args.layout.sort[0].ascending),[true,false]);
  }finally{columnGate.resolve();await Promise.allSettled([first,second]);p.close();}
});

test('configuration flushing waits for queued column preferences as well as the active write',async()=>{
  const columnGate=deferred(),p=fixture({dirty:[],columnGate});let first,second,flushing;
  try{
    first=p.sort(true);await tick();second=p.sort(false);await tick();
    let flushed=false;flushing=p.writes.flush().then(()=>flushed=true);await tick();assert.equal(flushed,false);
    columnGate.resolve();await Promise.all([first,second,flushing]);assert.equal(flushed,true);
    assert.deepEqual(p.calls.filter(([command])=>command==='save_columns').map(([,args])=>args.layout.sort[0].ascending),[true,false]);
  }finally{columnGate.resolve();await Promise.allSettled([first,second,flushing]);p.close();}
});

test('busy navigation stays put; disabled autosave and provider documents do not write',async()=>{
  const busy=fixture({busy:true});try{await busy.select('b');assert.deepEqual(busy.snapshot().selected,['a']);assert.deepEqual(busy.calls,[]);}finally{busy.close();}
  for(const state of [{enabled:false},{desktop:false},{providers:['a']},{dirty:[]}]){
    const p=fixture(state);try{await p.select('b');assert.deepEqual(p.snapshot().selected,['b']);assert.deepEqual(p.calls,[]);}finally{p.close();}
  }
});

test('explicit load/playlist order changes also wait for saving the current selection',async()=>{
  for(const operation of ['list-unsorted','list-playlist-order']){
    const gate=deferred(),p=fixture({gate,fail:true});try{
      const change=p.orderMode(operation);assert.equal(p.calls[0]?.[0],'save_fields_batch');
      gate.resolve();await change;assert.deepEqual(p.snapshot().dirty,['a']);assert.equal(p.snapshot().mode,'manual');
    }finally{gate.resolve();await tick();p.close();}
  }
});

test('selecting the same row does not save, while cover-only pending changes are saved before leaving',async()=>{
  const same=fixture();try{await same.select('a');assert.deepEqual(same.calls,[]);assert.deepEqual(same.snapshot().dirty,['a']);}finally{same.close();}
  const p=fixture({coverOnly:true});try{await p.select('b');assert.equal(p.calls[0]?.[1].items.length,1);assert.deepEqual(p.calls[0][1].items[0].updates,{});assert.ok(p.calls[0][1].items[0].covers);assert.deepEqual(p.snapshot().covers,[]);assert.deepEqual(p.snapshot().selected,['b']);}finally{p.close();}
});

test('a selection changed by another owner during save is not overwritten by the old request',async()=>{
  const gate=deferred(),p=fixture({gate});try{
    const change=p.select('b');p.selectOutside(['c']);gate.resolve();await change;
    assert.deepEqual(p.snapshot().selected,['c']);
  }finally{gate.resolve();await tick();p.close();}
});

test('Customize columns retains its edited layout when saving before an order change is refused',async()=>{
  for(const outcome of ['fail','cancel']){
    const p=fixture({[outcome]:true}),oldDocument=globalThis.document;
    try{
      globalThis.document=p.document;
      const ui=new columns.ColumnUI({layout:()=>columns.defaultLayout(),apply:p.applyVerified});ui.customize();
      const dialog=p.document.querySelector('.columns-dialog');dialog.querySelector('#column-name').value='My unsaved layout';
      dialog.querySelector('#column-ok').click();await tick();await tick();
      assert.equal(dialog.open,true,outcome);assert.equal(dialog.isConnected,true,outcome);
      assert.equal(dialog.querySelector('#column-name').value,'My unsaved layout');
      assert.equal(p.calls.some(([command])=>command==='save_columns'),false);
      assert.ok(dialog.querySelector('.dialog-error'));
      p.state[outcome]=false;dialog.querySelector('#column-ok').click();await tick();await tick();
      assert.equal(dialog.open,false);assert.equal(p.calls.find(([command])=>command==='save_columns')[1].layout.columns[0].name,'My unsaved layout');
    }finally{if(oldDocument===undefined)delete globalThis.document;else globalThis.document=oldDocument;p.close();}
  }
});

test('a checkbox click rejected while busy leaves its checkmark consistent with selection',async()=>{
  const p=fixture({busy:true});try{
    p.bindRowClicks();p.document.querySelector('tr[data-path="b"] .row-select').click();await tick();
    assert.deepEqual(p.snapshot().selected,['a']);assert.equal(p.document.querySelector('tr[data-path="b"] .row-select').checked,false);
  }finally{p.close();}
});
