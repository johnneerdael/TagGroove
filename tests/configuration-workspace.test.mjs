import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const names=['captureConfigurationWorkspace','readConfigurationWorkspace','restoreConfigurationWorkspace','clearWorkspace','prepareConfigurationLoad','flushSavedConfiguration','enabled','coordinateConfigurationLaunches'];
const functions=names.map(name=>{const node=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name);assert.ok(node,name);return node.getText(parsed);}).join('\n');
const compiled=ts.transpileModule(functions,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const chapters=ts.transpileModule(await readFile(new URL('../src/chapters.ts',import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {rowFile}=await import(`data:text/javascript;base64,${Buffer.from(chapters).toString('base64')}`);
const order=ts.transpileModule(await readFile(new URL('../src/playlist-order.ts',import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {rankSourceRows}=await import(`data:text/javascript;base64,${Buffer.from(order).toString('base64')}`);
function harness(){
  const calls=[],entry=path=>({path,fields:{TITLE:['Saved']},covers:[]});
  const h={rowFile,rankSourceRows,entries:new Map([['/Current/keep.wav',entry('/Current/keep.wav')]]),selected:new Set(['/Current/keep.wav']),listRoots:['/Current'],fileOps:{history:{current:'/Current'}},listOrder:'manual',sourceOrder:['/Current/keep.wav','/Removed/tombstone.wav'],location:'/Current',locationIsData:true,filter:'TITLE HAS Saved',filterDraft:'unapplied',anchor:'/Current/keep.wav',loadedPlaylist:'/Current/list.m3u8',documentAccess:{states:{}},sourceGeneration:0,cellEditor:{cancel:()=>calls.push(['cancel-cell'])},coverIndex:0,busy:false,actionUI:{hasUnsavedChanges:false},drafts:new Map([['/Current/keep.wav',{TITLE:['Unsaved']}]]),pendingCovers:new Map([['/Current/keep.wav',['pending-art']]]),configurationRecoveryActive:false,layoutSaveTimer:0,ownLayoutSave:'',mainLayout:{dock:'left'},configurationWrites:{flush:async()=>calls.push(['saved-flush'])},helperLists:{ready:Promise.resolve(),flush:async()=>calls.push(['history-flush'])},window:{clearTimeout:()=>calls.push(['clear-layout-timer'])},normalizeMainLayout:value=>value,optionsUI:{updatePage:async(...args)=>calls.push(['layout-save',...args])},messages:{confirm:async()=>true},render:()=>{},changedData:()=>calls.push(['changed']),status:'Ready'};
  h.removeEntries=paths=>{calls.push(['remove',paths]);paths.forEach(path=>h.entries.delete(path));};
  h.accept=batch=>{calls.push(['accept',batch]);batch.entries.forEach(item=>h.entries.set(item.path,item));};
  h.invoke=async(command,args)=>{calls.push([command,structuredClone(args)]);if(command==='scan_files')return{entries:args.paths.map(entry),errors:[]};if(command==='set_working_directory')return;throw new Error(command);};
  Object.assign(h,{desktop:true,profileBootstrap:{desktop:true},runtimePlatform:{family:'desktop'},coverOperations:{},listOperations:[],configurationLaunchesPending:0});
  const api=new Function('host',`with(host){${compiled};return{${names.join(',')}};}`)(h);
  return{h,calls,...api};
}
test('production workspace capture keeps manual insertion, prunes removed source rank and excludes provider working copies',()=>{
  const p=harness();assert.deepEqual(p.captureConfigurationWorkspace(),{version:2,paths:['/Current/keep.wav'],selected:['/Current/keep.wav'],roots:['/Current'],working_directory:'/Current',list_order:'manual',source_order:['/Current/keep.wav'],location:'/Current',location_is_data:true,filter:'TITLE HAS Saved',filter_draft:'unapplied',anchor:'/Current/keep.wav',playlist:'/Current/list.m3u8'});
  p.h.documentAccess.states['/Current/keep.wav']={uri:'content://provider/private'};assert.equal(p.captureConfigurationWorkspace(),null,'a provider working path is never serialized as local workspace');
  assert.equal(p.h.drafts.get('/Current/keep.wav').TITLE[0],'Unsaved');
  p.h.entries.clear();p.h.selected.clear();p.h.listRoots=['content://provider/tree'];assert.equal(p.captureConfigurationWorkspace(),null,'empty provider trees do not leak grants through roots');
});
test('capture ranks new unranked rows exactly as Playlist order displays them while retaining a null legacy baseline',()=>{
  const p=harness();p.h.entries.set('/Current/new.wav',{path:'/Current/new.wav'});
  assert.deepEqual(p.captureConfigurationWorkspace().source_order,['/Current/keep.wav','/Current/new.wav']);
  p.h.sourceOrder=undefined;assert.equal(p.captureConfigurationWorkspace().source_order,null);
});
test('production restoration validates every exact virtual row before clearing live state and restores both orders/workdir',async()=>{
  const p=harness(),one='/Music/chapters.m4a\u00011',two='/Music/chapters.m4a\u00012';
  const workspace={...p.captureConfigurationWorkspace(),paths:[two,one],selected:[one],roots:['/Music/list.m3u8'],working_directory:'/Music',source_order:[one,two],list_order:'playlist',location:'/Music/list.m3u8',filter:'',filter_draft:null,anchor:one,playlist:'/Music/list.m3u8'};
  let release;p.h.invoke=async(command,args)=>{p.calls.push([command,args]);if(command==='scan_files')return new Promise(resolve=>{release=()=>resolve({entries:args.paths.map(path=>({path})),errors:[]});});};
  const restoring=p.restoreConfigurationWorkspace(workspace);assert.deepEqual([...p.h.entries.keys()],['/Current/keep.wav']);assert.ok(!p.calls.some(([name])=>name==='remove'));release();assert.equal(await restoring,true);
  assert.deepEqual(p.calls.find(([name])=>name==='scan_files')[1],{paths:[two,one],recursive:false,exactRows:true});
  assert.deepEqual([...p.h.entries.keys()],[two,one]);assert.deepEqual([...p.h.selected],[one]);assert.deepEqual(p.h.sourceOrder,[one,two]);assert.equal(p.h.listOrder,'playlist');assert.equal(p.h.fileOps.history.current,'/Music');assert.deepEqual(p.h.listRoots,['/Music/list.m3u8']);
});
test('workspace scan protocol preserves saved CUE and chapter IDs without substituting physical paths',async()=>{
  const p=harness(),paths=['/Music/mix.cue\u00012','/Music/chapters.m4a\u00013'];
  p.h.invoke=async(command,args)=>{assert.equal(command,'scan_files');p.calls.push([command,args]);return{entries:args.paths.map(path=>({path})),errors:[]};};
  const result=await p.readConfigurationWorkspace({...p.captureConfigurationWorkspace(),paths,selected:[],anchor:null,source_order:paths});
  assert.deepEqual(p.calls[0][1],{paths,recursive:false,exactRows:true});assert.deepEqual(result.unavailable,[]);assert.deepEqual([...result.available.keys()],paths);
});
test('exact workspace reads retain saved physical parents when ordinary destination scans expand chapters',async()=>{
  const p=harness(),parent='/Music/book.m4b',child=`${parent}\u00011`;
  p.h.invoke=async(command,args)=>{assert.equal(command,'scan_files');p.calls.push([command,args]);return{entries:args.paths.flatMap(path=>args.exactRows?[{path}]:[{path:`${path}\u00011`}]),errors:[]};};
  const paths=[parent,child],result=await p.readConfigurationWorkspace({...p.captureConfigurationWorkspace(),paths,selected:[],anchor:null,source_order:paths});
  assert.deepEqual(result.unavailable,[],'workspace identity cannot depend on destination chapter-display preference');
  assert.deepEqual([...result.available.keys()],paths);assert.equal(p.calls[0][1].exactRows,true);
});
test('missing chapter identity never collapses to the physical file or a partial workspace',async()=>{
  const p=harness(),before=p.captureConfigurationWorkspace(),workspace={...before,paths:['/Music/album.m4a\u00012'],selected:[],anchor:null};
  p.h.invoke=async()=>({entries:[{path:'/Music/album.m4a'}],errors:[]});
  await assert.rejects(p.restoreConfigurationWorkspace(workspace),/incomplete/);assert.deepEqual(p.captureConfigurationWorkspace(),before);assert.ok(!p.calls.some(([name])=>name==='remove'));assert.equal(p.h.drafts.size,1);assert.equal(p.h.pendingCovers.size,1);
});
test('production Load gate authorizes later discard without clearing drafts; action-editor and busy cases refuse',async()=>{
  const p=harness();assert.equal(await p.prepareConfigurationLoad(),true);assert.equal(p.h.drafts.size,1);assert.equal(p.h.pendingCovers.size,1);
  p.h.messages.confirm=async()=>false;assert.equal(await p.prepareConfigurationLoad(),false);assert.equal(p.h.drafts.size,1);
  p.h.actionUI.hasUnsavedChanges=true;await assert.rejects(p.prepareConfigurationLoad(),/action group editor/);p.h.actionUI.hasUnsavedChanges=false;p.h.busy=true;assert.equal(await p.prepareConfigurationLoad(),false);
});
test('production saved-state flush awaits layout and prior publications, then histories; pending recovery never flushes stale writers',async()=>{
  const p=harness();p.h.layoutSaveTimer=123;await p.flushSavedConfiguration();assert.deepEqual(p.calls.map(([name])=>name),['clear-layout-timer','layout-save','saved-flush','history-flush','saved-flush']);
  p.calls.length=0;p.h.configurationRecoveryActive=true;await p.flushSavedConfiguration();assert.deepEqual(p.calls,[]);
  p.h.configurationRecoveryActive=false;p.h.helperLists.flush=async()=>{throw new Error('history save refused');};await assert.rejects(p.flushSavedConfiguration(),/history save refused/);assert.equal(p.h.drafts.size,1);
});
test('portable transfer availability follows native local-desktop capability without disabling configuration folder access',()=>{
  const p=harness();p.h.runtimePlatform.family='android';assert.equal(p.enabled('config-save'),false);assert.equal(p.enabled('config-load'),false);assert.equal(p.enabled('config-folder'),true);
  p.h.runtimePlatform.family='desktop';assert.equal(p.enabled('config-save'),true);p.h.profileBootstrap.desktop=false;assert.equal(p.enabled('config-load'),false);
});
test('Load preparation refuses while an explicit launch is being drained',async()=>{
  const p=harness();p.h.configurationLaunchesPending=1;assert.equal(await p.prepareConfigurationLoad(),false);assert.equal(p.h.drafts.size,1);
});
test('production launch coordinator does not consume queued launch requests while configuration reload is held',async()=>{
  const p=harness();p.h.busy=true;let consumed=false,release;
  const pending=p.coordinateConfigurationLaunches(async()=>{consumed=true;await new Promise(resolve=>{release=resolve;});});
  await new Promise(resolve=>setTimeout(resolve,5));assert.equal(consumed,false);assert.equal(p.h.configurationLaunchesPending,0);
  p.h.busy=false;await new Promise(resolve=>setTimeout(resolve,110));assert.equal(consumed,true);assert.equal(p.h.configurationLaunchesPending,1);assert.equal(await p.prepareConfigurationLoad(),false);
  release();await pending;assert.equal(p.h.configurationLaunchesPending,0);assert.equal(await p.prepareConfigurationLoad(),true);
});
