import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const root=new URL('../',import.meta.url),transpile=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const load=async name=>import(`data:text/javascript;base64,${Buffer.from(transpile(await readFile(new URL(`src/${name}.ts`,root),'utf8'))).toString('base64')}`);
const {SourceRootTracker}=await load('source-tracking');
const columns=await load('columns'),order=await load('playlist-order'),{PlaylistUI,isPlaylistPath}=await load('playlist-ui'),{mergeOrder,shuffleOrder,moveSelection,autoSaveTargets,tagsPagePrefs}=await load('list-behaviour');
const {resolvePathChanges,rebaseCaseOnly}=await load('action-ui'),{DirectoryHistory}=await load('file-ops'),{insideFolders}=await load('general-options'),{rowFile,replaceFileRows}=await load('chapters'),{reconcileDrafts}=await load('document-state');
const source=await readFile(new URL('src/main.ts',root),'utf8'),parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const functionNames=['loadPaths','scan','sourceReadContext','activeSort','setListOrder','ordered','persistColumns','applyColumns','clearWorkspace','removeEntries','listEnabled','listOperation','operateImpl','sortMenuItems','reorderFiles','shuffleFiles','saveOnMove','applyRenamed','rebaseSourceRoots','executeGroups','renameFromList','accept','spliceRows','optionsApplied','captureConfigurationWorkspace'];
const functions=parsed.statements.filter(node=>ts.isFunctionDeclaration(node)&&functionNames.includes(node.name?.text)).map(node=>node.getText(parsed)).join('\n');
const fileHost=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>node.declarationList.declarations).find(node=>node.name.getText(parsed)==='fileOps').initializer.arguments[0];
const movedCallback=fileHost.properties.find(node=>node.name?.getText(parsed)==='moved').initializer.getText(parsed);
const toolsHost=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>node.declarationList.declarations).find(node=>node.name.getText(parsed)==='toolsUI').initializer.arguments[0];
const toolCallback=toolsHost.properties.find(node=>node.name?.getText(parsed)==='run').initializer.getText(parsed);
const deferred=()=>{let resolve;return{promise:new Promise(done=>resolve=done),resolve:value=>resolve(value)};};

function runtime(state={}){
  state.playlists??={'/lists/one.m3u8':{files:['/music/b.wav','/music/a.wav'],errors:[]}};
  const calls=[],entries=new Map(),selected=new Set(),drafts=new Map(),pendingCovers=new Map(),copyOverrides=new Map(),workingReadErrors=new Map();
  const invoke=async(command,args)=>{calls.push([command,structuredClone(args)]);
    if(command==='begin_source_tracking')return 'owned-source-session';
    if(command==='release_source_roots')return;
    if(command==='prepare_source_roots'){
      state.sourceSerial=(state.sourceSerial??0)+1;
      const sources=args.paths.map((path,index)=>({path,token:Object.hasOwn(state.directories??{},path)||isPlaylistPath(path)?null:`owned-file-${state.sourceSerial}-${index}`,error:null}));
      return{key:`owned-prepared-${state.sourceSerial}`,sources,acquired:sources.flatMap(source=>source.token?[source.token]:[])};
    }
    if(command==='resolve_source_roots'){
      const paths=args.sources.map(source=>source.path),renamed=(state.externalRenames??[]).filter(change=>paths.includes(change.from));
      state.sourceSerial=(state.sourceSerial??0)+1;
      const sources=args.sources.map(source=>({...source,path:renamed.find(change=>change.from===source.path)?.to??source.path}));
      return{key:`owned-resolved-${state.sourceSerial}`,paths:sources.map(source=>source.path),renamed,sources,acquired:[]};
    }
    if(command==='classify_source_paths'){if(state.kindGate)await state.kindGate.promise;return args.paths.map(path=>{const kind=Object.hasOwn(state.directories??{},path)?'directory':state.missing?.includes(path)?'unknown':'file';return{kind,resolved:kind==='unknown'?null:state.resolvedRoots?.[path]??path};});}
    if(command==='read_playlist'){const gate=state.readGates?.[args.path]??state.readGate;if(gate)await gate.promise;const result=state.playlists[args.path];if(result instanceof Error)throw result;if(!result)throw new Error('Unreadable playlist');return structuredClone(result);}
    if(command==='scan_files'){if(state.scanGate)await state.scanGate.promise;if(state.scanFailure)throw new Error(state.scanFailure);const expanded=state.scanEntries??args.paths.flatMap(path=>state.directories?.[path]??[path]),paths=[...new Set(expanded)].filter(path=>!state.missing?.includes(path));return{entries:paths.map(path=>({path,filename:path.split('/').pop(),fields:{TITLE:[path]},covers:[]})),errors:state.scanErrors??[]};}
    if(command==='run_tool')return {started:1};
    if(command==='save_columns')return;
    if(command==='execute_action_groups'||command==='rename_files'||command==='rename_file_from_list')return structuredClone(state.renameBatch);
    throw new Error(`Unexpected ${command}`);
  };
  const playlistUI=new PlaylistUI({invoke});
  const dependencies={state,calls,entries,selected,drafts,pendingCovers,copyOverrides,workingReadErrors,invoke,invokeUntracked:invoke,configurationWrites:{run:write=>write()},playlistUI,isPlaylistPath,...columns,...order,mergeOrder,shuffleOrder,moveSelection,autoSaveTargets,tagsPagePrefs,resolvePathChanges,rebaseCaseOnly,DirectoryHistory,insideFolders,rowFile,replaceFileRows,reconcileDrafts,SourceRootTracker};
  const code=transpile(`
    const desktop=state.desktop??true,runtimePlatform={document_access_supported:!!state.provider},documentAccess={states:{}};const sourceTracker=new SourceRootTracker({invoke});
    let columnLayout=defaultLayout(),listRoots=[],listRootKinds={},listRootResolved={},loadedPlaylist=state.legacyPlaylist,listOrder='columns',sourceOrder,sourceGeneration=0,columnSaveQueue=Promise.resolve();
    let busy=false,selectionChangePending=false,errors=[],status='',location='',locationIsData=false,coverIndex=0,anchor,progressJob,dataRevision=0,orderCacheKey='',filterCacheKey='',orderCache=[],filter='',filterDraft;
    let collator=new Intl.Collator('en',{numeric:true});const columnValues={revision:0,retain:()=>{}},cellValue=(entry,_template)=>entry.filename,sortValueOf=path=>path;
    const tableColumns=()=>columnLayout.columns.filter(column=>column.visible);
    const cellEditor={cancel:()=>calls.push(['cell-cancel'])},folderImages={clear:()=>calls.push(['art-clear'])},optionsUI={settings:{directories:{subfolders:false}},page:()=>({save_on_move:false})},generalPrefs=()=>({select_added:false,keep_moved:state.keepMoved??true});
    const render=()=>{},refreshList=()=>{},refreshSelection=()=>{},revealRow=()=>{},timing=()=>{},invalidateCovers=()=>{},changedData=()=>{dataRevision++;orderCacheKey='';};
    class HTMLElement{}class HTMLTableRowElement extends HTMLElement{};const esc=String,document={querySelector:()=>({scrollTop:0,textContent:''})};
    const visible=()=>ordered(),paths=()=>ordered().filter(entry=>selected.has(entry.path)).map(entry=>entry.path);
    const basename=path=>path.split('/').pop(),publicMessage=String;
    const dirname=path=>path.replace(/[\\/][^\\/]*$/,'')||path,sourcePolicy=()=>({unsupportedReason:''}),columnSpecs=()=>[],storeEntry=entry=>{entries.set(entry.path,entry);changedData();},movedOut=new Set();let lastActionGroups,queuedNotices=[];
    const fileOps={history:new DirectoryHistory(path=>calls.push(['working-directory',path]))};if(state.workingDirectory)fileOps.history.visit(state.workingDirectory);
    const messages={enabled:()=>false,confirm:async()=>{calls.push(['confirm']);if(state.confirmGate)await state.confirmGate.promise;return state.confirm!==false;}};
    const run=async(_message,job)=>{if(busy)return;busy=true;progressJob={cancelling:false};try{return await job('fixture-job');}catch(error){errors=[{path:'Operation',error:String(error)}];return undefined;}finally{busy=false;progressJob=undefined;}};
    const noticeLabels={};let autoPlaylistPending=false,languageGeneration=0,pendingOptionsBaseline;
    const naturalCollator=()=>new Intl.Collator('en',{numeric:true}),applyLanguage=async()=>++languageGeneration,applyAppearancePrefs=()=>{},applyNotificationArea=async()=>{},providerEntry=()=>false;
    const enabled=op=>op==='list-playlist-order'?!!sourceOrder&&listRoots.some(root=>listRootKinds[root]==='file'&&isPlaylistPath(root)):true;
    ${functions}
    const moved=${movedCallback},runTool=${toolCallback};
    return{loadPaths,capture:captureConfigurationWorkspace,tool:()=>runTool({},[...selected]),refresh:()=>listOperation('refresh'),playlistOrder:()=>operateImpl('list-playlist-order'),unsorted:()=>operateImpl('list-unsorted'),canRefresh:()=>listEnabled('refresh'),
      sort:async()=>applyColumns({...columnLayout,sort:[{id:'filename',ascending:true}]},false,true),resize:async()=>applyColumns({...columnLayout,columns:columnLayout.columns.map((column,index)=>index?column:{...column,width:300})}),
      manual:()=>reorderFiles(shown=>[...shown].reverse()),moveTop:()=>reorderFiles(shown=>moveSelection(shown,selected,'top')),shuffle:shuffleFiles,menu:sortMenuItems,forgetSourceOrder:()=>{sourceOrder=undefined;},remove:path=>removeEntries([path]),select:path=>{selected.clear();selected.add(path);anchor=path;},
      clear:clearWorkspace,dirty:(path='/music/a.wav')=>drafts.set(path,{TITLE:['Pending']}),artDirty:()=>pendingCovers.set('/music/a.wav',[{digest:'pending'}]),cancel:()=>{if(progressJob)progressJob.cancelling=true;},
      execute:()=>executeGroups([{name:'Rename fixture',enabled:true,actions:[]}]),move:()=>moved(state.renameBatch,'Moved fixture'),rename:(path,name)=>renameFromList(entries.get(path),'_FILENAME_EXT',name),
      optionsChanged:()=>optionsApplied({tags:{advanced:{list_chapters:!!state.listChapters}},pages:{}},{tags:{advanced:{list_chapters:!state.listChapters}},pages:{}}),
      snapshot:()=>({paths:ordered().map(entry=>entry.path),manual:[...entries.keys()],roots:[...listRoots],rootKinds:{...listRootKinds},rootResolved:{...listRootResolved},playlist:loadedPlaylist,order:sourceOrder,mode:listOrder,sort:structuredClone(columnLayout.sort),selected:[...selected],errors:structuredClone(errors),drafts:[...drafts],artwork:[...pendingCovers],workingDirectory:fileOps.history.current,location,locationIsData,status})};
  `);
  return{...new Function(...Object.keys(dependencies),code)(...Object.values(dependencies)),calls,state};
}

test('playlist load and explicit Playlist order preserve saved column sort across sorting and manual moves',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});
  assert.deepEqual(model.snapshot().roots,['/lists/one.m3u8']);assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav']);
  assert.deepEqual(model.snapshot().sort,[{id:'filename',ascending:true}]);assert.equal(model.calls.some(([command])=>command==='save_columns'),false);
  await model.sort();assert.deepEqual(model.snapshot().paths,['/music/a.wav','/music/b.wav']);model.manual();await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav']);
  await model.resize();assert.equal(model.snapshot().mode,'playlist');assert.deepEqual(model.snapshot().sort,[{id:'filename',ascending:true}]);
  await model.shuffle();assert.equal(model.snapshot().mode,'manual');assert.deepEqual(model.snapshot().sort,[{id:'filename',ascending:true}]);await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav']);
  assert.match(model.menu(),/data-op="list-playlist-order" role="menuitemradio" aria-checked="true"/);model.forgetSourceOrder();assert.doesNotMatch(model.menu(),/data-op="list-playlist-order"/,'legacy snapshots do not infer original grouping');
});

test('Refresh rereads updated source membership and order, reports missing entries, and never writes a playlist',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});
  model.state.playlists['/lists/one.m3u8']={files:['/music/c.wav','/music/b.wav'],errors:[{path:'/lists/one.m3u8',error:'Line 3: missing.wav: file not found'}]};
  await model.refresh();assert.deepEqual(model.snapshot().paths,['/music/c.wav','/music/b.wav']);assert.equal(model.snapshot().errors.length,1);assert.equal(model.calls.filter(([command])=>command==='read_playlist').length,2);
  assert.ok(model.calls.every(([command])=>['begin_source_tracking','prepare_source_roots','release_source_roots','resolve_source_roots','classify_source_paths','read_playlist','scan_files','save_columns','cell-cancel','art-clear'].includes(command)),JSON.stringify(model.calls));
  model.state.playlists['/lists/one.m3u8']={files:[],errors:[]};await model.refresh();assert.deepEqual(model.snapshot().paths,[],'valid empty playlist clears membership');assert.equal(model.canRefresh(),true,'source remains refreshable after an empty read');
});

for(const destination of ['/music/renamed.wav','/elsewhere/renamed.wav']){
  test(`external individual-file rename Refresh follows verified identity to ${destination} without admitting neighbors`,async()=>{
    const from='/music/original.wav',neighbor='/music/unrelated.wav';
    const model=runtime({directories:{'/music':[neighbor]}});
    await model.loadPaths([from],{replace:true});model.select(from);
    model.state.missing=[from];model.state.externalRenames=[{from,to:destination}];
    await model.refresh();
    assert.deepEqual(model.snapshot().paths,[destination]);
    assert.deepEqual(model.snapshot().roots,[destination]);
    assert.deepEqual(model.snapshot().selected,[destination]);
    assert.equal(model.snapshot().rootResolved[destination],destination);
    assert.deepEqual(model.calls.filter(([command])=>command==='scan_files').at(-1)[1].paths,[destination]);
    assert.ok(!model.snapshot().paths.includes(neighbor));
    assert.ok(!model.calls.some(([command,args])=>command==='scan_files'&&args.paths.includes('/music')));
  });
}

test('external alias rename Refresh retains the selected canonical row and manual position among other sources',async()=>{
  const alias='/logical/a.wav',from='/physical/a.wav',to='/physical/new.wav',other='/music/other.wav';
  const model=runtime({resolvedRoots:{[alias]:from},scanEntries:[from,other]});
  await model.loadPaths([alias,other],{replace:true});model.select(from);model.moveTop();
  model.state.scanEntries=[to,other];model.state.externalRenames=[{from:alias,to}];model.state.missing=[alias,from];
  await model.refresh();
  assert.deepEqual(model.snapshot().manual,[to,other]);
  assert.deepEqual(model.snapshot().selected,[to]);
  assert.deepEqual(model.snapshot().roots,[to,other]);
});

for(const kind of ['folder','playlist']){
  const abc=['/music/a.wav','/music/b.wav','/music/c.wav'],cab=[abc[2],abc[0],abc[1]],root=kind==='folder'?'/music':'/lists/manual.m3u8';
  const fixture=()=>kind==='folder'?{directories:{[root]:[...abc]}}:{playlists:{[root]:{files:[...abc],errors:[]}}};
  test(`${kind} Refresh retains manually moved rows independently of fresh source rank and saved column sort`,async()=>{
    const model=runtime(fixture());await model.loadPaths([root],{replace:true});model.select(abc[2]);model.moveTop();
    assert.deepEqual(model.snapshot().paths,cab);assert.equal(model.snapshot().mode,'manual');const sort=model.snapshot().sort;
    await model.refresh();assert.deepEqual(model.snapshot().paths,cab);assert.deepEqual(model.snapshot().manual,cab);assert.deepEqual(model.snapshot().order,abc);assert.equal(model.snapshot().mode,'manual');assert.deepEqual(model.snapshot().sort,sort);
    if(kind==='playlist'){
      await model.playlistOrder();assert.deepEqual(model.snapshot().paths,abc);await model.refresh();assert.equal(model.snapshot().mode,'playlist');assert.deepEqual(model.snapshot().paths,abc);assert.deepEqual(model.snapshot().manual,cab);
      await model.unsorted();assert.deepEqual(model.snapshot().paths,cab);
    }
    await model.sort();await model.refresh();assert.equal(model.snapshot().mode,'columns');assert.deepEqual(model.snapshot().paths,abc);assert.deepEqual(model.snapshot().manual,cab);assert.deepEqual(model.snapshot().sort,sort);
    await model.unsorted();assert.deepEqual(model.snapshot().paths,cab);
  });
  test(`${kind} Refresh removes absent exact rows and appends new rows without reordering survivors`,async()=>{
    const model=runtime(fixture());await model.loadPaths([root],{replace:true});model.select(abc[2]);model.moveTop();
    const d='/music/d.wav',fresh=[d,abc[1],abc[0]];
    if(kind==='folder')model.state.directories[root]=fresh;else model.state.playlists[root]={files:fresh,errors:[]};
    await model.refresh();assert.deepEqual(model.snapshot().paths,[abc[0],abc[1],d]);assert.deepEqual(model.snapshot().manual,[abc[0],abc[1],d]);assert.deepEqual(model.snapshot().order,fresh);assert.equal(model.snapshot().mode,'manual');
    if(kind==='playlist'){await model.playlistOrder();assert.deepEqual(model.snapshot().paths,fresh);}
  });
}

test('Refresh treats changed chapter identities as new rows while retaining surviving exact row positions',async()=>{
  const book='/music/book.m4b',one=book+'\u00011',two=book+'\u00012',three=book+'\u00013',other='/music/other.wav',root='/lists/chapters.m3u8';
  const model=runtime({playlists:{[root]:{files:[one,two,other],errors:[]}}});await model.loadPaths([root],{replace:true});model.select(other);model.moveTop();
  model.state.playlists[root]={files:[three,one,other],errors:[]};await model.refresh();
  assert.deepEqual(model.snapshot().manual,[other,one,three]);assert.deepEqual(model.snapshot().paths,[other,one,three]);assert.deepEqual(model.snapshot().order,[three,one,other]);
  await model.playlistOrder();assert.deepEqual(model.snapshot().paths,[three,one,other]);
});

test('Refresh scanning blocks manual moves while preserving the current mode and source context',async()=>{
  const abc=['/music/a.wav','/music/b.wav','/music/c.wav'],model=runtime({directories:{'/music':abc}});await model.loadPaths(['/music'],{replace:true});
  model.state.scanGate=deferred();const refreshing=model.refresh();await Promise.resolve();await Promise.resolve();model.select(abc[2]);model.moveTop();model.state.scanGate.resolve();await refreshing;
  assert.deepEqual(model.snapshot().paths,abc);assert.deepEqual(model.snapshot().order,abc);assert.equal(model.snapshot().mode,'columns');assert.equal(model.snapshot().status,'Refreshed: 3 files');
});

test('grouped playlist, folder and individual loads merge first positions and Refresh expands every original group',async()=>{
  const model=runtime({directories:{'/albums':['/music/d.wav','/music/a.wav']},playlists:{'/lists/one.m3u8':{files:['/music/b.wav','/music/a.wav'],errors:[]},'/lists/two.xspf':{files:['/music/a.wav','/music/c.wav'],errors:[]}}});
  await model.loadPaths(['/lists/one.m3u8','/albums'],{replace:true});await model.loadPaths(['/lists/two.xspf','/music/e.wav']);
  assert.deepEqual(model.snapshot().roots,['/lists/one.m3u8','/albums','/lists/two.xspf','/music/e.wav']);
  model.manual();await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav','/music/d.wav','/music/c.wav','/music/e.wav']);
  await model.sort();await model.refresh();assert.deepEqual(model.snapshot().paths,['/music/a.wav','/music/b.wav','/music/c.wav','/music/d.wav','/music/e.wav']);await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav','/music/d.wav','/music/c.wav','/music/e.wav']);
});

test('fatal reader errors and refusal of pending changes retain the whole previous workspace',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});model.dirty();const before=model.snapshot();model.state.confirm=false;await model.refresh();assert.deepEqual(model.snapshot().paths,before.paths);assert.deepEqual(model.snapshot().drafts,before.drafts);
  model.state.confirm=true;model.state.playlists['/lists/one.m3u8']=new Error('Malformed XML');await model.refresh();assert.deepEqual(model.snapshot().paths,before.paths);assert.deepEqual(model.snapshot().drafts,before.drafts);assert.deepEqual(model.snapshot().roots,before.roots);
  model.state.playlists['/lists/one.m3u8']={files:[],errors:[{path:'/lists/one.m3u8',error:'Line 1: missing.wav: file not found'}]};await model.refresh();assert.deepEqual(model.snapshot().paths,[]);assert.equal(model.snapshot().errors.length,1);assert.deepEqual(model.snapshot().drafts,[]);
});

test('cancelled or stale reads never clear members or drafts after the async boundary',async()=>{
  for(const action of ['cancel','dirty','clear']){const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});const before=model.snapshot();model.state.scanGate=deferred();const refreshing=model.refresh();await Promise.resolve();await Promise.resolve();model[action]();const retained=model.snapshot();model.state.scanGate.resolve();await refreshing;assert.deepEqual(model.snapshot().paths,action==='clear'?[]:before.paths,action);assert.deepEqual(model.snapshot().drafts,retained.drafts,action);}
});

test('cancellation during playlist expansion never starts a scanner job and retains pending edits and the cancellation notice',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});model.dirty();model.artDirty();const before=model.snapshot(),scans=model.calls.filter(([command])=>command==='scan_files').length;
  model.state.readGate=deferred();const refreshing=model.refresh();await Promise.resolve();model.cancel();model.state.readGate.resolve();await refreshing;
  assert.equal(model.calls.filter(([command])=>command==='scan_files').length,scans,'no scan_files after cancellation while expanding');
  assert.deepEqual(model.snapshot().paths,before.paths);assert.deepEqual(model.snapshot().roots,before.roots);assert.deepEqual(model.snapshot().order,before.order);assert.deepEqual(model.snapshot().drafts,before.drafts);assert.deepEqual(model.snapshot().artwork,before.artwork);assert.equal(model.snapshot().status,'Refresh cancelled. The list is unchanged.');
});

test('production option reload and accept preserve a parent file source rank through chapter expansion and collapse after manual ordering',async()=>{
  const book='/music/book.m4b',row1=book+'\u00011',row2=book+'\u00012',last='/music/last.wav';
  const model=runtime({playlists:{'/lists/book.m3u8':{files:[book,last],errors:[]}}});await model.loadPaths(['/lists/book.m3u8'],{replace:true});await model.manual();
  model.state.listChapters=true;model.state.scanEntries=[last,row1,row2];await model.optionsChanged();await model.playlistOrder();
  assert.deepEqual(model.snapshot().order,[row1,row2,last]);assert.deepEqual(model.snapshot().paths,[row1,row2,last]);assert.equal(model.calls.at(-1)[0],'scan_files','actual Options reload reached the reader boundary');
  model.state.listChapters=false;model.state.scanEntries=[last,book];await model.optionsChanged();await model.playlistOrder();
  assert.deepEqual(model.snapshot().order,[book,last]);assert.deepEqual(model.snapshot().paths,[book,last]);
});

test('re-adding an existing parent transforms its source rank before scanning appends genuinely new members',async()=>{
  const book='/music/book.m4b',row1=book+'\u00011',row2=book+'\u00012',last='/music/last.wav',added='/music/new.wav';
  const model=runtime({playlists:{'/lists/book.m3u8':{files:[book,last],errors:[]}}});await model.loadPaths(['/lists/book.m3u8'],{replace:true});await model.manual();
  model.state.scanEntries=[row1,row2,added];await model.loadPaths([book,added]);await model.playlistOrder();
  assert.deepEqual(model.snapshot().order,[row1,row2,last,added]);assert.deepEqual(model.snapshot().paths,[row1,row2,last,added]);
  model.state.scanEntries=[book];await model.loadPaths([book]);await model.playlistOrder();
  assert.deepEqual(model.snapshot().order,[book,last,added]);assert.deepEqual(model.snapshot().paths,[book,last,added]);
  model.forgetSourceOrder();model.state.scanEntries=[row1,row2];await model.loadPaths([book]);assert.equal(model.snapshot().order,undefined,'an unknown legacy rank cannot be reconstructed from a partial add/scan');assert.doesNotMatch(model.menu(),/data-op="list-playlist-order"/);
});

test('a later load wins when an earlier playlist read completes late, and confirmation changes cannot discard new edits',async()=>{
  const gate=deferred(),model=runtime({readGates:{'/lists/old.m3u8':gate},playlists:{'/lists/old.m3u8':{files:['/old.wav'],errors:[]},'/lists/new.m3u8':{files:['/new.wav'],errors:[]}}});
  const older=model.loadPaths(['/lists/old.m3u8'],{replace:true});await model.loadPaths(['/lists/new.m3u8'],{replace:true});gate.resolve();assert.equal(await older,false);assert.deepEqual(model.snapshot().paths,['/new.wav']);assert.deepEqual(model.snapshot().roots,['/lists/new.m3u8']);
  const dirty=runtime();await dirty.loadPaths(['/lists/one.m3u8'],{replace:true});dirty.dirty();dirty.state.confirmGate=deferred();const refresh=dirty.refresh();dirty.dirty('/music/b.wav');dirty.state.confirmGate.resolve();await refresh;assert.equal(dirty.snapshot().drafts.length,2);assert.equal(dirty.calls.filter(([command])=>command==='read_playlist').length,1);
});

test('errors from a refresh or load of a workspace already cleared cannot replace its current errors',async()=>{
  for(const refreshing of [false,true]){const model=runtime();if(refreshing)await model.loadPaths(['/lists/one.m3u8'],{replace:true});model.state.readGate=deferred();const pending=refreshing?model.refresh():model.loadPaths(['/lists/one.m3u8'],{replace:true});model.clear();model.state.playlists['/lists/one.m3u8']=new Error('Late reader failure');model.state.readGate.resolve();await pending;assert.deepEqual(model.snapshot().paths,[]);assert.deepEqual(model.snapshot().errors,[],refreshing?'refresh':'load');}
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});model.state.scanGate=deferred();const pending=model.refresh();await Promise.resolve();await Promise.resolve();model.clear();model.state.scanFailure='Late scanner failure';model.state.scanGate.resolve();await pending;assert.deepEqual(model.snapshot().errors,[]);assert.deepEqual(model.snapshot().paths,[]);
});

test('artwork-only drafts require Refresh confirmation and cancellation preserves confirmed field and artwork intent',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});model.artDirty();model.state.confirm=false;await model.refresh();assert.equal(model.snapshot().artwork.length,1);assert.equal(model.calls.filter(([command])=>command==='read_playlist').length,1);
  model.state.confirm=true;model.dirty();model.state.scanGate=deferred();const pending=model.refresh();await Promise.resolve();await Promise.resolve();model.cancel();model.state.scanGate.resolve();await pending;assert.equal(model.snapshot().artwork.length,1);assert.equal(model.snapshot().drafts.length,1);
});

test('source criterion retains removed rows as removed until Refresh and preserves selection intersection',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});model.remove('/music/a.wav');await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/b.wav']);await model.refresh();assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav']);
  model.select('/music/a.wav');model.state.playlists['/lists/one.m3u8']={files:['/music/a.wav','/music/c.wav'],errors:[]};await model.refresh();assert.deepEqual(model.snapshot().selected,['/music/a.wav']);
  model.state.playlists['/lists/one.m3u8']={files:['/music/c.wav'],errors:[]};await model.refresh();assert.deepEqual(model.snapshot().selected,['/music/c.wav']);
});

test('Refresh refuses provider/URI roots and directory-only loading keeps regular column sorting',async()=>{
  const directory=runtime({directories:{'/music':['/music/z.wav','/music/a.wav']}});await directory.loadPaths(['/music'],{replace:true});assert.deepEqual(directory.snapshot().paths,['/music/a.wav','/music/z.wav']);assert.equal(directory.snapshot().mode,'columns');
  const provider=runtime({provider:true});await provider.loadPaths(['/music/a.wav']);assert.equal(provider.canRefresh(),false);
  const uri=runtime({playlists:{'content://fixture/list.m3u8':{files:['/music/a.wav'],errors:[]}}});await uri.loadPaths(['content://fixture/list.m3u8']);assert.equal(uri.snapshot().paths.length,0);assert.equal(uri.canRefresh(),false);
});

test('production action, inline filename and file-move callbacks carry playlist row rank through known file changes until Refresh',async()=>{
  for(const route of ['execute','rename','move']){const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});const original=structuredClone(model.state.playlists);await model.sort();
    model.state.renameBatch={entries:[{path:'/music/renamed.wav',filename:'renamed.wav',fields:{},covers:[]}],errors:[],renamed:[{from:'/music/b.wav',to:'/music/renamed.wav'}]};
    if(route==='rename')assert.equal(await model.rename('/music/b.wav','renamed.wav'),'/music/renamed.wav');else await model[route]();
    await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/renamed.wav','/music/a.wav'],route);assert.deepEqual(model.snapshot().order,['/music/renamed.wav','/music/a.wav'],route);assert.deepEqual(model.snapshot().roots,['/lists/one.m3u8']);assert.deepEqual(model.state.playlists,original,'no source playlist rewrite');
    model.state.playlists['/lists/one.m3u8']={files:['/music/a.wav'],errors:[{path:'/lists/one.m3u8',error:'Line 1: b.wav: file not found'}]};await model.refresh();assert.deepEqual(model.snapshot().paths,['/music/a.wav']);assert.match(model.snapshot().errors[0].error,/b.wav/);
  }
});

test('inline folder rename uses one canonical row identity for selection drafts roots and later Refresh',async()=>{
  for(const name of ['Album/../Track.wav','alias/Track.wav']){
    const from='/music/a.wav',to='/music/real/Track.wav',model=runtime();
    await model.loadPaths([from],{replace:true});model.select(from);model.dirty(from);
    model.state.renameBatch={entries:[{path:to,filename:'Track.wav',fields:{TITLE:['Saved']},covers:[]}],errors:[],renamed:[{from,to}]};
    assert.equal(await model.rename(from,name),to);
    const after=model.snapshot();
    assert.deepEqual(after.paths,[to]);assert.deepEqual(after.roots,[to]);assert.deepEqual(after.selected,[to]);
    assert.equal(after.drafts[0][0],to);
    assert.deepEqual(model.calls.find(([command])=>command==='rename_file_from_list'),['rename_file_from_list',{path:from,name}]);
    model.state.missing=[from];await model.refresh();
    assert.deepEqual(model.snapshot().paths,[to]);assert.deepEqual(model.snapshot().roots,[to]);
  }
});

test('app folder rename rebases original source roots and active context while retaining playlist rank and untouched source contents',async()=>{
  const source='/music/album.m3u8',contents={files:['/music/b.wav','/music/a.wav'],errors:[]};
  const model=runtime({workingDirectory:'/music',playlists:{[source]:contents},directories:{'/music/disc':['/music/disc/c.wav']}});await model.loadPaths([source,'/music/disc'],{replace:true});
  model.state.renameBatch={entries:['/library/b.wav','/library/a.wav','/library/disc/c.wav'].map(path=>({path,filename:path.split('/').pop(),fields:{},covers:[]})),errors:[],moved_directories:[{from:'/music',to:'/library'}]};
  await model.execute();await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/library/b.wav','/library/a.wav','/library/disc/c.wav']);
  assert.deepEqual(model.snapshot().roots,['/library/album.m3u8','/library/disc']);assert.equal(model.snapshot().playlist,'/library/album.m3u8');assert.equal(model.snapshot().workingDirectory,'/library');assert.equal(model.snapshot().location,'/library/album.m3u8');assert.deepEqual(model.state.playlists[source],contents,'contents were not rewritten');
  delete model.state.playlists[source];model.state.playlists['/library/album.m3u8']={files:['/library/b.wav','/library/a.wav'],errors:[]};delete model.state.directories['/music/disc'];model.state.directories['/library/disc']=['/library/disc/c.wav'];
  await model.refresh();assert.deepEqual(model.snapshot().paths,['/library/b.wav','/library/a.wav','/library/disc/c.wav']);assert.deepEqual(model.snapshot().errors,[]);assert.equal(model.calls.at(-1)[0],'art-clear');
});

test('a known individual-file move follows only that source root and never implies a containing-folder move',async()=>{
  const model=runtime({workingDirectory:'/music',directories:{'/music2':['/music2/neighbor.wav']}});await model.loadPaths(['/lists/one.m3u8','/music/direct.wav','/music2'],{replace:true});
  model.state.renameBatch={entries:[{path:'/other/direct.wav',filename:'direct.wav',fields:{},covers:[]}],errors:[],renamed:[{from:'/music/direct.wav',to:'/other/direct.wav'}]};
  await model.move();await model.playlistOrder();assert.deepEqual(model.snapshot().roots,['/lists/one.m3u8','/other/direct.wav','/music2']);assert.equal(model.snapshot().workingDirectory,'/music');assert.equal(model.snapshot().playlist,'/lists/one.m3u8');assert.equal(model.snapshot().location,'/lists/one.m3u8');assert.deepEqual(model.snapshot().paths,['/music/b.wav','/music/a.wav','/other/direct.wav','/music2/neighbor.wav']);
});

test('actual folder mappings preserve virtual row identities and leave prefix-neighbor roots and working folders alone',async()=>{
  const source='/album/list.cue',rows=['/album/list.cue\u00012','/album/list.cue\u00011'];
  const model=runtime({workingDirectory:'/album2',playlists:{[source]:{files:rows,errors:[]}},directories:{'/album2':['/album2/neighbor.wav']}});await model.loadPaths([source,'/album/selected.wav','/album2'],{replace:true});model.select('/album/selected.wav');
  model.state.renameBatch={entries:[{path:'/renamed/selected.wav',filename:'selected.wav',fields:{},covers:[]}],errors:[],moved_directories:[{from:'/album',to:'/renamed'}]};
  await model.execute();await model.playlistOrder();assert.deepEqual(model.snapshot().roots,['/renamed/list.cue','/renamed/selected.wav','/album2']);assert.equal(model.snapshot().workingDirectory,'/album2');assert.equal(model.snapshot().playlist,'/renamed/list.cue');assert.deepEqual(model.snapshot().order,['/renamed/list.cue\u00012','/renamed/list.cue\u00011','/renamed/selected.wav','/album2/neighbor.wav']);assert.deepEqual(model.snapshot().paths,model.snapshot().order);assert.deepEqual(model.state.playlists[source].files,rows);
});


test('playlist-suffixed directories load and refresh as folders and keep moved-file boundaries',async()=>{
  for(const extension of ['m3u','m3u8','pls','cue','xspf','wpl','asx']){
    const root=`/music/archive.${extension}`,inside=`${root}/a.wav`,outside='/music/moved.wav';
    const model=runtime({directories:{[root]:[inside]},keepMoved:false,renameBatch:{entries:[{path:outside,filename:'moved.wav',fields:{},covers:[]}],errors:[],renamed:[{from:inside,to:outside}]}});
    assert.equal(await model.loadPaths([root],{replace:true}),true,extension);
    assert.deepEqual(model.snapshot().roots,[root]);assert.deepEqual(model.snapshot().paths,[inside]);assert.equal(model.snapshot().playlist,undefined);
    await model.tool();assert.equal(model.calls.find(([command])=>command==='run_tool')[1].root,root);assert.doesNotMatch(model.menu(),/data-op="list-playlist-order"/);
    await model.refresh();assert.deepEqual(model.snapshot().paths,[inside]);assert.equal(model.calls.some(([command])=>command==='read_playlist'),false);
    await model.execute();assert.deepEqual(model.snapshot().paths,[],`${extension}: moving to the parent leaves the loaded directory`);
  }
});

for(const alias of [false,true]){
  test(`same-folder rename keeps its row with Keep moved files off through ${alias?'an alias':'a canonical'} root`,async()=>{
    const physical='/physical/album',root=alias?'/logical/album':physical,from=`${physical}/a.wav`,to=`${physical}/renamed.wav`;
    const model=runtime({directories:{[root]:[from]},resolvedRoots:{[root]:physical},keepMoved:false,
      renameBatch:{entries:[{path:to,filename:'renamed.wav',fields:{},covers:[]}],errors:[],renamed:[{from,to}]}});
    await model.loadPaths([root],{replace:true});assert.deepEqual(model.snapshot().roots,[root]);
    await model.execute();assert.deepEqual(model.snapshot().paths,[to],'a rename inside the actual loaded folder is not a move out');
    assert.deepEqual(model.snapshot().selected,[to]);assert.deepEqual(model.snapshot().roots,[root],'the logical folder remains the Refresh source');
  });
  test(`known in-app rename updates an individually added ${alias?'alias':'canonical'} file root`,async()=>{
    const from='/physical/album/a.wav',to='/physical/album/renamed.wav',root=alias?'/logical/album/a.wav':from;
    const model=runtime({scanEntries:[from],resolvedRoots:{[root]:from},keepMoved:true,
      renameBatch:{entries:[{path:to,filename:'renamed.wav',fields:{},covers:[]}],errors:[],renamed:[{from,to}]}});
    await model.loadPaths([root],{replace:true});assert.deepEqual(model.snapshot().roots,[root]);
    await model.execute();assert.deepEqual(model.snapshot().roots,[to],'Refresh must not retain the pre-rename file name');
    assert.equal(model.snapshot().rootKinds[to],'file');assert.deepEqual(model.snapshot().paths,[to]);
    assert.equal(model.snapshot().location,to);assert.equal(model.snapshot().locationIsData,true);
    assert.equal(model.capture().location,to);assert.equal(model.capture().location_is_data,true);
  });
}

test('root-backed rename locations follow their selected transform once across mixed roots',async()=>{
  const a='/physical/a.wav',b='/physical/b.wav',c='/physical/c.wav',alias='/logical/a.wav';
  for(const aliases of [false,true])for(const [renamed,expected] of [
    [[{from:a,to:b},{from:b,to:c}],b],
    [[{from:a,to:b},{from:b,to:a}],b],
  ]){
    const root=aliases?alias:a,model=runtime({scanEntries:[a,b],resolvedRoots:{[root]:a},renameBatch:{entries:[],errors:[],renamed}});
    await model.loadPaths([root,b],{replace:true,location:root,locationIsData:true});await model.execute();
    assert.equal(model.snapshot().location,expected);assert.equal(model.capture().location,expected);
    assert.deepEqual(model.snapshot().roots,renamed.map(move=>move.to));
  }
});

test('root-backed locations survive alias convergence and case-only folder moves',async()=>{
  const alias='/logical/album',physical='/physical/album',to='/physical/ALBUM';
  const model=runtime({directories:{[alias]:[`${physical}/one.wav`],[physical]:[`${physical}/one.wav`]},resolvedRoots:{[alias]:physical},renameBatch:{entries:[],errors:[],moved_directories:[{from:physical,to}]}});
  await model.loadPaths([alias,physical],{replace:true,location:alias,locationIsData:true});await model.execute();
  assert.deepEqual(model.snapshot().roots,[to]);assert.equal(model.snapshot().location,to);
  assert.equal(model.capture().location,to);assert.equal(model.capture().location_is_data,true);
});

test('alias-root renames preserve literal labels and rebase non-root path locations normally',async()=>{
  const root='/logical/a.wav',from='/physical/a.wav',to='/physical/renamed.wav';
  for(const [location,locationIsData,expected] of [[root,false,root],['Chosen file',false,'Chosen file'],[from,true,to]]){
    const model=runtime({scanEntries:[from],resolvedRoots:{[root]:from},renameBatch:{entries:[],errors:[],renamed:[{from,to}]}});
    await model.loadPaths([root],{replace:true,location,locationIsData});await model.execute();
    assert.deepEqual(model.snapshot().roots,[to]);assert.equal(model.snapshot().location,expected);
    assert.equal(model.snapshot().locationIsData,locationIsData);assert.equal(model.capture().location,expected);
    assert.equal(model.capture().location_is_data,locationIsData);
  }
});

test('root-backed active playlists follow known folder moves through canonical and alias inputs',async()=>{
  const physical='/physical/album',destination='/physical/renamed',playlist=`${physical}/list.m3u8`,nextPlaylist=`${destination}/list.m3u8`,song=`${physical}/one.wav`,nextSong=`${destination}/one.wav`;
  for(const root of [playlist,'/logical/list.m3u8']){
    const model=runtime({playlists:{[root]:{files:[song],errors:[]}},resolvedRoots:{[root]:playlist},renameBatch:{entries:[],errors:[],moved_directories:[{from:physical,to:destination}]}});
    await model.loadPaths([root],{replace:true});await model.execute();
    assert.deepEqual(model.snapshot().roots,[nextPlaylist]);assert.equal(model.snapshot().playlist,nextPlaylist);
    assert.equal(model.capture().playlist,nextPlaylist);assert.equal(model.capture().location,nextPlaylist);
    model.state.playlists[nextPlaylist]={files:[nextSong],errors:[]};await model.refresh();
    assert.equal(model.snapshot().playlist,nextPlaylist);assert.deepEqual(model.snapshot().paths,[nextSong]);
  }
});

test('root-backed working folders follow known and case-only moves while non-root paths keep generic rebasing',async()=>{
  const physical='/physical/album',alias='/logical/album';
  for(const destination of ['/physical/renamed','/physical/ALBUM'])for(const root of [physical,alias])for(const working of [root,`${physical}/child`]){
    const model=runtime({directories:{[root]:[`${physical}/one.wav`]},resolvedRoots:{[root]:physical},workingDirectory:working,renameBatch:{entries:[],errors:[],moved_directories:[{from:physical,to:destination}]}});
    await model.loadPaths([root],{replace:true});await model.execute();
    const expected=working===root?destination:`${destination}/child`;
    assert.equal(model.snapshot().workingDirectory,expected);assert.equal(model.capture().working_directory,expected);
  }
});

for(const pending of [false,true]){
  test(`an alias-root move outside the loaded folder ${pending?'retains a pending draft':'leaves the list'}`,async()=>{
    const root='/logical/album',from='/physical/album/a.wav',to='/physical/elsewhere/moved.wav';
    const model=runtime({directories:{[root]:[from]},resolvedRoots:{[root]:'/physical/album'},keepMoved:false,
      renameBatch:{entries:[{path:to,filename:'moved.wav',fields:{},covers:[]}],errors:[],renamed:[{from,to}]}});
    await model.loadPaths([root],{replace:true});if(pending)model.dirty(from);
    await model.move();assert.deepEqual(model.snapshot().paths,pending?[to]:[]);
    assert.deepEqual(model.snapshot().drafts,pending?[[to,{TITLE:['Pending']}]]:[]);
  });
}

test('Refresh retains the logical alias while resolving its current target for tool and rename context',async()=>{
  const root='/logical/album',old='/physical/old/a.wav',fresh='/physical/new/a.wav',to='/physical/new/renamed.wav';
  const model=runtime({directories:{[root]:[old]},resolvedRoots:{[root]:'/physical/old'},keepMoved:false});
  await model.loadPaths([root],{replace:true});model.state.directories[root]=[fresh];model.state.resolvedRoots[root]='/physical/new';
  await model.refresh();assert.deepEqual(model.snapshot().roots,[root]);assert.deepEqual(model.snapshot().paths,[fresh]);
  await model.tool();assert.equal(model.calls.find(([name])=>name==='run_tool')[1].root,'/physical/new');
  model.state.renameBatch={entries:[{path:to,filename:'renamed.wav',fields:{},covers:[]}],errors:[],renamed:[{from:fresh,to}]};
  await model.execute();assert.deepEqual(model.snapshot().paths,[to]);assert.deepEqual(model.snapshot().roots,[root]);
});

test('alias and canonical exact-file roots converge after a known rename without losing kind metadata',async()=>{
  const alias='/logical/a.wav',from='/physical/a.wav',to='/physical/renamed.wav';
  const model=runtime({scanEntries:[from],resolvedRoots:{[alias]:from},renameBatch:{entries:[{path:to,filename:'renamed.wav',fields:{},covers:[]}],errors:[],renamed:[{from,to}]}});
  await model.loadPaths([alias,from],{replace:true});assert.deepEqual(model.snapshot().roots,[alias,from]);
  await model.execute();assert.deepEqual(model.snapshot().roots,[to]);assert.deepEqual(model.snapshot().rootKinds,{[to]:'file'});assert.deepEqual(model.snapshot().paths,[to]);
});


test('kind metadata follows add/dedup, removed direct rows and root rebasing without suffix guesses',async()=>{
  const root='/music/album.cue',file='/direct.wav',model=runtime({directories:{[root]:[`${root}/one.wav`]}});
  await model.loadPaths([root,file],{replace:true});await model.loadPaths([root]);
  assert.deepEqual(model.snapshot().rootKinds,{[root]:'directory',[file]:'file'});assert.deepEqual(model.snapshot().roots,[root,file]);
  model.remove(file);await model.loadPaths(['/next.wav']);assert.equal(model.snapshot().rootKinds[file],'file');
  model.state.renameBatch={entries:[],errors:[],moved_directories:[{from:root,to:'/MUSIC/Album.cue'}]};await model.execute();
  assert.deepEqual(model.snapshot().rootKinds,{'/MUSIC/Album.cue':'directory',[file]:'file','/next.wav':'file'});
  model.clear();assert.deepEqual(model.snapshot().rootKinds,{});await model.loadPaths(['/replacement.wav'],{replace:true});assert.deepEqual(model.snapshot().rootKinds,{'/replacement.wav':'file'});
});

test('unknown roots retain playlist reader/scanner diagnostics and supply no invented tool folder',async()=>{
  const missing='/missing.m3u',model=runtime({missing:[missing],playlists:{[missing]:new Error('file not found')}});
  assert.equal(await model.loadPaths([missing]),false);assert.match(model.snapshot().errors[0].error,/file not found/);
  model.state.missing=['/missing.wav'];model.state.scanErrors=[{path:'/missing.wav',error:'file not found'}];await model.loadPaths(['/missing.wav']);
  assert.deepEqual(model.snapshot().rootKinds,{'/missing.wav':'unknown'});assert.match(model.snapshot().errors[0].error,/file not found/);await model.tool();assert.equal(model.calls.find(([name])=>name==='run_tool')[1].root,null);
});

test('stale filesystem classification cannot change roots, kinds, rows or pending edits',async()=>{
  const model=runtime();await model.loadPaths(['/lists/one.m3u8']);const before=model.snapshot();model.state.kindGate=deferred();
  const reading=model.loadPaths(['/later.cue'],{replace:true});await new Promise(resolve=>setImmediate(resolve));model.clear();model.dirty('/pending.wav');model.state.kindGate.resolve();assert.equal(await reading,false);
  assert.deepEqual(model.snapshot().roots,[]);assert.deepEqual(model.snapshot().rootKinds,{});assert.deepEqual(model.snapshot().paths,[]);assert.equal(model.snapshot().drafts.length,1);assert.notDeepEqual(model.snapshot().roots,before.roots);
});


test('ordinary single-root scans preserve explicit location labels and data flags',async()=>{
  for(const root of ['/music/archive','/music/archive.m3u']){
    const model=runtime({directories:{[root]:[`${root}/one.wav`]}});await model.loadPaths([root],{replace:true,location:'Chosen album',locationIsData:false});
    assert.equal(model.snapshot().location,'Chosen album');assert.equal(model.snapshot().locationIsData,false);
  }
});


test('Refresh retains the chosen active playlist and never adopts a later appended playlist',async()=>{
  const a='/lists/a.m3u',b='/lists/b.m3u',playlists={[a]:{files:['/a.wav'],errors:[]},[b]:{files:['/b.wav'],errors:[]}};
  for(const initial of [a,'/music']){
    const model=runtime({playlists,directories:{'/music':['/music/one.wav']}});await model.loadPaths([initial],{replace:true});await model.loadPaths([b]);
    const active=initial===a?a:undefined;assert.equal(model.snapshot().playlist,active);await model.refresh();assert.equal(model.snapshot().playlist,active);
  }
});


test('Refresh rechecks a captured active playlist outside legacy input roots without adopting new context',async()=>{
  const playlist='/outside/list.m3u',model=runtime({legacyPlaylist:playlist,directories:{'/music':['/music/one.wav']}});
  await model.loadPaths(['/music']);assert.equal(model.snapshot().playlist,playlist);await model.refresh();assert.equal(model.snapshot().playlist,playlist);
  assert.deepEqual(model.snapshot().rootKinds,{'/music':'directory'});assert.ok(model.calls.some(([name,args])=>name==='classify_source_paths'&&args.paths.length===1&&args.paths[0]===playlist));
  model.state.directories[playlist]=[];await model.refresh();assert.equal(model.snapshot().playlist,undefined);assert.deepEqual(model.snapshot().rootKinds,{'/music':'directory'});
});


test('appending preserves an active playlist with no valid rows, its source location and chosen order',async()=>{
  const a='/lists/empty.m3u',b='/lists/added.m3u';
  for(const errors of [[],[{path:a,error:'Line 1: missing.wav: file not found'}]])for(const mode of ['playlist','columns','manual']){
    const model=runtime({playlists:{[a]:{files:[],errors},[b]:{files:['/music/added.wav'],errors:[]}}});
    assert.equal(await model.loadPaths([a],{replace:true}),true);assert.equal(model.snapshot().playlist,a);assert.deepEqual(model.snapshot().paths,[]);
    if(mode==='columns')await model.sort();else if(mode==='manual')await model.unsorted();
    const before=model.snapshot();assert.equal(before.mode,mode);
    assert.equal(await model.loadPaths([b]),true);assert.equal(model.snapshot().playlist,a,'appending rows must retain the chosen active source');
    assert.equal(model.snapshot().location,before.location);assert.equal(model.snapshot().locationIsData,before.locationIsData);assert.equal(model.snapshot().mode,mode);
    assert.deepEqual(model.snapshot().roots,[a,b]);assert.deepEqual(model.snapshot().paths,['/music/added.wav']);
    await model.refresh();assert.equal(model.snapshot().playlist,a);assert.ok(model.calls.every(([name])=>name!=='create_playlists'));
  }
});

test('appending a first playlist adopts it only when the prior folder list is empty',async()=>{
  const folder='/music',playlist='/lists/added.m3u';
  for(const rows of [[],['/music/existing.wav']]){
    const model=runtime({directories:{[folder]:rows},playlists:{[playlist]:{files:['/music/added.wav'],errors:[]}}});
    await model.loadPaths([folder],{replace:true});assert.equal(model.snapshot().playlist,undefined);await model.loadPaths([playlist]);
    assert.equal(model.snapshot().playlist,rows.length?undefined:playlist);assert.equal(model.snapshot().mode,rows.length?'columns':'playlist');assert.deepEqual(model.snapshot().roots,[folder,playlist]);
  }
});

test('explicit replacement selects the new playlist even when the old and new playlists are empty',async()=>{
  const a='/lists/empty.m3u',b='/lists/replacement.m3u';
  for(const files of [[],['/music/replacement.wav']]){
    const model=runtime({playlists:{[a]:{files:[],errors:[]},[b]:{files,errors:[]}}});await model.loadPaths([a],{replace:true});await model.loadPaths([b],{replace:true});
    assert.equal(model.snapshot().playlist,b);assert.deepEqual(model.snapshot().roots,[b]);assert.deepEqual(model.snapshot().paths,files);assert.equal(model.snapshot().mode,'playlist');
  }
});
