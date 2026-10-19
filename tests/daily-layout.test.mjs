import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
const compile=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const toolbar=await import(`data:text/javascript;base64,${Buffer.from(compile(await readFile(new URL('../src/toolbar.ts',import.meta.url),'utf8'))).toString('base64')}`);
const workspaceSession=await import(`data:text/javascript;base64,${Buffer.from(compile(await readFile(new URL('../src/workspace-session.ts',import.meta.url),'utf8'))).toString('base64')}`);
const {savedWorkspace}=workspaceSession;
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8'),parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
function production(name,host){const node=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name);assert.ok(node,name);return new Function('host',`with(host){${compile(node.getText(parsed)).replace('export {};','')};return ${name};}`)(host);}
test('toolbar arrangement survives normalize/save/reopen and Cancel keeps saved placement',async()=>{
 const window=new Window({url:'http://localhost'});globalThis.document=window.document;
 try{let saved=toolbar.defaultToolbar();const host={config:()=>saved,save:async config=>{saved=toolbar.normalizeToolbar(config);},mac:()=>false};
 for(const arrangement of ['custom-first','stacked-main','stacked-custom','main-first']){
  toolbar.customizeToolbar(host);let dialog=document.querySelector('.toolbar-dialog'),select=dialog.querySelector('#toolbar-arrangement');assert.ok(select,'main customization offers keyboard-accessible band placement');assert.equal(select.tagName,'SELECT');select.value=arrangement;dialog.querySelector('#toolbar-ok').click();await Promise.resolve();await Promise.resolve();
  assert.equal(saved.arrangement??'main-first',arrangement);toolbar.customizeToolbar(host);dialog=document.querySelector('.toolbar-dialog');assert.equal(dialog.querySelector('#toolbar-arrangement').value,arrangement);dialog.querySelector('#toolbar-arrangement').value='stacked-custom';dialog.querySelector('[data-close]').click();assert.equal(saved.arrangement??'main-first',arrangement);
 }
 assert.equal(toolbar.normalizeToolbar({arrangement:'invalid'}).arrangement,undefined);
 }finally{window.happyDOM.abort();}
});
test('production band renderer honors both order and row placement alongside either filter dock',()=>{
 for(const arrangement of ['main-first','custom-first','stacked-main','stacked-custom'])for(const filter of ['list','toolbar-left','toolbar-right']){
  const render=production('toolbarsMarkup',{toolbarConfig:{arrangement},toolbarMarkup:()=>'<div id="main"></div>',customToolbarMarkup:()=>'<div id="custom"></div>',compactScreen:()=>false,mainLayout:{filter}}),html=render('<div id="filter"></div>');
  assert.equal(html.indexOf('id="custom"')<html.indexOf('id="main"'),arrangement==='custom-first'||arrangement==='stacked-custom',arrangement);
  assert.equal(html.includes('data-toolbar-stack="true"'),arrangement.startsWith('stacked'),arrangement);
  if(filter==='toolbar-left')assert.ok(html.indexOf('id="filter"')<html.indexOf('id="main"'));
  if(filter==='toolbar-right')assert.ok(html.indexOf('id="filter"')>html.indexOf('id="custom"'));
 }
});

test('ordinary restart uses saved-only workspace capture, survives cleared list and retains local Windows identities',async()=>{
 const data=new Map(),storage={getItem:key=>data.get(key)??null,setItem:(key,value)=>data.set(key,value)};let captured={version:2,paths:['C:\\Music\\b.mp3','C:\\Music\\a.mp3'],selected:[],roots:['C:\\Music'],working_directory:'C:\\Music',list_order:'manual',source_order:['C:\\Music\\a.mp3','C:\\Music\\b.mp3'],location:'C:\\Music',location_is_data:true,filter:'',filter_draft:null,anchor:null,playlist:null},restored;
 const host={desktop:true,ordinaryWorkspaceReady:true,configurationRecoveryActive:false,runtimePlatform:{family:'desktop'},localStorage:storage,captureConfigurationWorkspace:()=>captured,restoreConfigurationWorkspace:async workspace=>{restored=workspace;return true;},ordinaryWorkspaceSaved:'',ordinaryWorkspaceRestoreFailed:false,ordinaryWorkspaceState:[],errors:[],entries:new workspaceSession.WorkspaceRows(),listRoots:[],listRootKinds:{},sourceOrder:undefined,listOrder:'manual',loadedPlaylist:undefined,fileOps:{history:{}},savedWorkspace};
 production('rememberOrdinaryWorkspace',host)();assert.ok(data.size,'ordinary list order is saved outside migration snapshots');await production('restoreOrdinaryWorkspace',host)();assert.deepEqual(restored,captured);
 captured={...captured,paths:[],roots:[],source_order:null,list_order:'columns',working_directory:null};production('rememberOrdinaryWorkspace',host)(true);restored=null;await production('restoreOrdinaryWorkspace',host)();assert.deepEqual(restored,captured,'clearing the list never resurrects previous rows');
});

test('saved workspace rejects malformed order/provider identities and warns visibly on quota refusal',()=>{
 assert.equal(savedWorkspace({version:2,paths:['content://private'],roots:[],selected:[]}),null);
 const host={desktop:true,ordinaryWorkspaceReady:true,configurationRecoveryActive:false,runtimePlatform:{family:'desktop'},localStorage:{setItem:()=>{throw new Error('quota');}},captureConfigurationWorkspace:()=>({version:2,paths:[]}),ordinaryWorkspaceSaved:'',ordinaryWorkspaceRestoreFailed:false,ordinaryWorkspaceState:[],errors:[],entries:new workspaceSession.WorkspaceRows(),listRoots:[],listRootKinds:{},sourceOrder:undefined,listOrder:'manual',loadedPlaylist:undefined,fileOps:{history:{}},errors:[],render:()=>{host.rendered=true;}};
 production('rememberOrdinaryWorkspace',host)();assert.ok(host.errors.some(error=>/file list/i.test(error.path)));assert.equal(host.rendered,true);
});

test('loaded-row locale refresh updates only current technical timestamps and invalidates evaluated rows',async()=>{
 let release;const original={path:'/m/a.mp3',fields:{TITLE:['Saved']},technical:{_FILE_MOD_DATETIME_RAW:['0'],_FILE_MOD_DATE:['old']},covers:[]},other={...original,path:'/m/b.mp3'},calls=[],host={entries:new Map([[original.path,original],[other.path,other]]),invoke:async(command,args)=>{calls.push([command,args]);return new Promise(resolve=>{release=resolve;});},columnValues:{clear:()=>calls.push(['cache-clear'])},changedData:()=>calls.push(['changed']),render:()=>calls.push(['render']),orderCacheKey:'old',filterCacheKey:'old',searchRevision:1};
 const operation=production('refreshLoadedLocale',host)();assert.equal(calls[0][0],'reformat_loaded_timestamps');const replaced={...other,technical:{...other.technical,_FILE_MOD_DATE:['edited']}};host.entries.set(other.path,replaced);
 release([{path:original.path,technical:{...original.technical,_FILE_MOD_DATE:['new']}},{path:other.path,technical:{_FILE_MOD_DATE:['new']}}]);await operation;
 assert.equal(host.entries.get(original.path).technical._FILE_MOD_DATE[0],'new');assert.strictEqual(host.entries.get(original.path).fields,original.fields);assert.strictEqual(host.entries.get(other.path),replaced,'stale IPC never replaces freshly loaded rows');assert.ok(calls.some(call=>call[0]==='cache-clear'));assert.equal(host.searchRevision,-1);
});
test('locale notification coordinator coalesces polling and checks language/focus/visibility changes without rescanning',async()=>{
 const module=await readFile(new URL('../src/locale-refresh.ts',import.meta.url),'utf8').catch(()=>''),{observeLocale}=await import(`data:text/javascript;base64,${Buffer.from(compile(module)).toString('base64')}`);assert.equal(typeof observeLocale,'function','locale change observer exists');
 const window=new Window(),doc=window.document;let fingerprint='en',refreshes=0,tick,checks=0;window.setInterval=fn=>{tick=fn;return 1;};window.clearInterval=()=>{};
 const stop=observeLocale({window,document:doc,state:async()=>{checks++;return fingerprint;},refresh:async()=>{refreshes++;},ready:()=>true,fail:error=>{throw error;}});const settle=async()=>{for(let i=0;i<10;i++)await Promise.resolve();};
 try{await settle();const initial=refreshes;fingerprint='de';window.dispatchEvent(new window.Event('languagechange'));window.dispatchEvent(new window.Event('focus'));tick();await settle();assert.equal(refreshes,initial+1);tick();await settle();assert.equal(refreshes,initial+1);fingerprint='nl';doc.dispatchEvent(new window.Event('visibilitychange'));await settle();assert.equal(refreshes,initial+2);assert.ok(checks>=3);}finally{stop();window.happyDOM.abort();}
});

test('ordinary file-list persistence tracks structural order rather than serializing tags or unrelated renders',()=>{
 assert.equal(typeof workspaceSession.WorkspaceRows,'function');const rows=new workspaceSession.WorkspaceRows();rows.set('/a',{title:'Saved'});rows.set('/b',{});let captures=0;
 const host={desktop:true,ordinaryWorkspaceReady:true,configurationRecoveryActive:false,runtimePlatform:{family:'desktop'},localStorage:{setItem:()=>{}},captureConfigurationWorkspace:()=>{captures++;return{paths:[...rows.keys()]};},ordinaryWorkspaceSaved:'',ordinaryWorkspaceRestoreFailed:false,ordinaryWorkspaceState:[],errors:[],entries:rows,listRoots:[],listRootKinds:{},sourceOrder:undefined,listOrder:'manual',loadedPlaylist:undefined,fileOps:{history:{current:undefined}}};
 const remember=production('rememberOrdinaryWorkspace',host);remember();remember();rows.set('/a',{title:'Draft edit or locale refresh'});remember();assert.equal(captures,1);
 rows.delete('/a');rows.set('/a',{});remember();assert.equal(captures,2,'second manual order in same mode is persisted');remember(true);assert.equal(captures,3,'close can synchronously capture latest selection/filter');
});

test('failed ordinary restore survives unrelated renders until a real list change and quota recovery retries forced close',async()=>{
 const rows=new workspaceSession.WorkspaceRows(),data=new Map([['opentag.workspace','{corrupt']]);let blocked=false,writes=0;
 const host={desktop:true,ordinaryWorkspaceReady:false,configurationRecoveryActive:false,runtimePlatform:{family:'desktop'},entries:rows,listRoots:[],listRootKinds:{},sourceOrder:undefined,listOrder:'columns',loadedPlaylist:undefined,fileOps:{history:{}},localStorage:{getItem:key=>data.get(key)??null,setItem:(key,value)=>{if(blocked)throw new Error('quota');writes++;data.set(key,value);}},captureConfigurationWorkspace:()=>({version:2,paths:[...rows.keys()]}),ordinaryWorkspaceSaved:'',ordinaryWorkspaceRestoreFailed:false,ordinaryWorkspaceState:[],errors:[],savedWorkspace,errors:[],render:()=>{}};
 const remember=production('rememberOrdinaryWorkspace',host),restore=production('restoreOrdinaryWorkspace',host);await assert.rejects(restore(),/corrupt/);host.ordinaryWorkspaceReady=true;remember();remember(true);assert.equal(data.get('opentag.workspace'),'{corrupt','an unrelated render preserves the failed snapshot');
 rows.set('/new.wav',{});blocked=false;remember();assert.equal(writes,1,'an explicit new list replaces invalid saved state');
 rows.set('/other.wav',{});blocked=true;remember();assert.equal(writes,1);assert.ok(host.errors.length);const savedBefore=host.ordinaryWorkspaceSaved;blocked=false;remember(true);assert.equal(writes,2,'normal close retries a previously failed snapshot');assert.notEqual(host.ordinaryWorkspaceSaved,savedBefore);assert.equal(host.errors.length,0,'successful retry clears the failure');
});

test('a forced close retries an unchanged quota-failed snapshot and clears its warning',()=>{
 const host={desktop:true,ordinaryWorkspaceReady:true,configurationRecoveryActive:false,runtimePlatform:{family:'desktop'},entries:new workspaceSession.WorkspaceRows(),listRoots:[],listRootKinds:{},sourceOrder:undefined,listOrder:'manual',loadedPlaylist:undefined,fileOps:{history:{}},captureConfigurationWorkspace:()=>({paths:['/same.wav']}),ordinaryWorkspaceSaved:'',ordinaryWorkspaceRestoreFailed:false,ordinaryWorkspaceState:[],errors:[],errors:[],render:()=>{}};let blocked=true,writes=0;host.localStorage={setItem:()=>{if(blocked)throw new Error('quota');writes++;}};const remember=production('rememberOrdinaryWorkspace',host);remember();blocked=false;remember(true);assert.equal(writes,1);assert.equal(host.errors.length,0);
});
