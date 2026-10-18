import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const root=new URL('../',import.meta.url),transpile=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const load=async name=>import(`data:text/javascript;base64,${Buffer.from(transpile(await readFile(new URL(`src/${name}.ts`,root),'utf8'))).toString('base64')}`);
const columns=await load('columns'),order=await load('playlist-order'),{PlaylistUI,isPlaylistPath}=await load('playlist-ui'),{mergeOrder,shuffleOrder}=await load('list-behaviour');
const {resolvePathChanges,rebaseCaseOnly}=await load('action-ui'),{DirectoryHistory}=await load('file-ops'),{insideFolders}=await load('general-options'),{rowFile,replaceFileRows}=await load('chapters'),{reconcileDrafts}=await load('document-state');
const source=await readFile(new URL('src/main.ts',root),'utf8'),parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const functionNames=['loadPaths','scan','sourceReadContext','activeSort','setListOrder','ordered','applyColumns','clearWorkspace','removeEntries','listEnabled','listOperation','operateImpl','sortMenuItems','reorderFiles','shuffleFiles','applyRenamed','executeGroups','renameFromList','accept','spliceRows','optionsApplied'];
const functions=parsed.statements.filter(node=>ts.isFunctionDeclaration(node)&&functionNames.includes(node.name?.text)).map(node=>node.getText(parsed)).join('\n');
const fileHost=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>node.declarationList.declarations).find(node=>node.name.getText(parsed)==='fileOps').initializer.arguments[0];
const movedCallback=fileHost.properties.find(node=>node.name?.getText(parsed)==='moved').initializer.getText(parsed);
const deferred=()=>{let resolve;return{promise:new Promise(done=>resolve=done),resolve:value=>resolve(value)};};

function runtime(state={}){
  state.playlists??={'/lists/one.m3u8':{files:['/music/b.wav','/music/a.wav'],errors:[]}};
  const calls=[],entries=new Map(),selected=new Set(),drafts=new Map(),pendingCovers=new Map(),copyOverrides=new Map(),workingReadErrors=new Map();
  const invoke=async(command,args)=>{calls.push([command,structuredClone(args)]);
    if(command==='read_playlist'){const gate=state.readGates?.[args.path]??state.readGate;if(gate)await gate.promise;const result=state.playlists[args.path];if(result instanceof Error)throw result;if(!result)throw new Error('Unreadable playlist');return structuredClone(result);}
    if(command==='scan_files'){if(state.scanGate)await state.scanGate.promise;if(state.scanFailure)throw new Error(state.scanFailure);const expanded=state.scanEntries??args.paths.flatMap(path=>state.directories?.[path]??[path]),paths=[...new Set(expanded)].filter(path=>!state.missing?.includes(path));return{entries:paths.map(path=>({path,filename:path.split('/').pop(),fields:{TITLE:[path]},covers:[]})),errors:state.scanErrors??[]};}
    if(command==='save_columns')return;
    if(command==='execute_action_groups'||command==='rename_files')return structuredClone(state.renameBatch);
    throw new Error(`Unexpected ${command}`);
  };
  const playlistUI=new PlaylistUI({invoke});
  const dependencies={state,calls,entries,selected,drafts,pendingCovers,copyOverrides,workingReadErrors,invoke,playlistUI,isPlaylistPath,...columns,...order,mergeOrder,shuffleOrder,resolvePathChanges,rebaseCaseOnly,DirectoryHistory,insideFolders,rowFile,replaceFileRows,reconcileDrafts};
  const code=transpile(`
    const desktop=state.desktop??true,runtimePlatform={document_access_supported:!!state.provider};
    let columnLayout=defaultLayout(),listRoots=[],loadedPlaylist,listOrder='columns',sourceOrder,sourceGeneration=0;
    let busy=false,errors=[],status='',location='',locationIsData=false,coverIndex=0,anchor,progressJob,dataRevision=0,orderCacheKey='',filterCacheKey='',orderCache=[],filter='';
    let collator=new Intl.Collator('en',{numeric:true});const columnValues={revision:0,retain:()=>{}},cellValue=(entry,_template)=>entry.filename,sortValueOf=path=>path;
    const tableColumns=()=>columnLayout.columns.filter(column=>column.visible);
    const cellEditor={cancel:()=>calls.push(['cell-cancel'])},folderImages={clear:()=>calls.push(['art-clear'])},optionsUI={settings:{directories:{subfolders:false}}},generalPrefs=()=>({select_added:false,keep_moved:state.keepMoved??true});
    const render=()=>{},refreshList=()=>{},refreshSelection=()=>{},revealRow=()=>{},timing=()=>{},invalidateCovers=()=>{},changedData=()=>{dataRevision++;orderCacheKey='';};
    const esc=String,document={querySelector:()=>({scrollTop:0,textContent:''})};
    const visible=()=>ordered(),paths=()=>ordered().filter(entry=>selected.has(entry.path)).map(entry=>entry.path);
    const basename=path=>path.split('/').pop(),publicMessage=String;
    const dirname=path=>path.replace(/[\\/][^\\/]*$/,'')||path,sourcePolicy=()=>({unsupportedReason:''}),columnSpecs=()=>[],storeEntry=entry=>{entries.set(entry.path,entry);changedData();},movedOut=new Set();let lastActionGroups,queuedNotices=[];
    const fileOps={history:new DirectoryHistory(path=>calls.push(['working-directory',path]))};if(state.workingDirectory)fileOps.history.visit(state.workingDirectory);
    const messages={enabled:()=>false,confirm:async()=>{calls.push(['confirm']);if(state.confirmGate)await state.confirmGate.promise;return state.confirm!==false;}};
    const run=async(_message,job)=>{if(busy)return;busy=true;progressJob={cancelling:false};try{return await job('fixture-job');}catch(error){errors=[{path:'Operation',error:String(error)}];return undefined;}finally{busy=false;progressJob=undefined;}};
    const noticeLabels={};let autoPlaylistPending=false,languageGeneration=0,pendingOptionsBaseline;
    const naturalCollator=()=>new Intl.Collator('en',{numeric:true}),applyLanguage=async()=>++languageGeneration,applyAppearancePrefs=()=>{},applyNotificationArea=async()=>{},providerEntry=()=>false;
    const enabled=op=>op==='list-playlist-order'?!!sourceOrder&&listRoots.some(isPlaylistPath):true;
    ${functions}
    const moved=${movedCallback};
    return{loadPaths,refresh:()=>listOperation('refresh'),playlistOrder:()=>operateImpl('list-playlist-order'),unsorted:()=>operateImpl('list-unsorted'),canRefresh:()=>listEnabled('refresh'),
      sort:async()=>applyColumns({...columnLayout,sort:[{id:'filename',ascending:true}]},false,true),resize:async()=>applyColumns({...columnLayout,columns:columnLayout.columns.map((column,index)=>index?column:{...column,width:300})}),
      manual:()=>reorderFiles(shown=>[...shown].reverse()),shuffle:shuffleFiles,menu:sortMenuItems,forgetSourceOrder:()=>{sourceOrder=undefined;},remove:path=>removeEntries([path]),select:path=>{selected.clear();selected.add(path);anchor=path;},
      clear:clearWorkspace,dirty:(path='/music/a.wav')=>drafts.set(path,{TITLE:['Pending']}),artDirty:()=>pendingCovers.set('/music/a.wav',[{digest:'pending'}]),cancel:()=>{if(progressJob)progressJob.cancelling=true;},
      execute:()=>executeGroups([{name:'Rename fixture',enabled:true,actions:[]}]),move:()=>moved(state.renameBatch,'Moved fixture'),rename:(path,name)=>renameFromList(entries.get(path),'_FILENAME_EXT',name),
      optionsChanged:()=>optionsApplied({tags:{advanced:{list_chapters:!!state.listChapters}},pages:{}},{tags:{advanced:{list_chapters:!state.listChapters}},pages:{}}),
      snapshot:()=>({paths:ordered().map(entry=>entry.path),manual:[...entries.keys()],roots:[...listRoots],playlist:loadedPlaylist,order:sourceOrder,mode:listOrder,sort:structuredClone(columnLayout.sort),selected:[...selected],errors:structuredClone(errors),drafts:[...drafts],artwork:[...pendingCovers],workingDirectory:fileOps.history.current,location,status})};
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
  assert.ok(model.calls.every(([command])=>['read_playlist','scan_files','save_columns','cell-cancel','art-clear'].includes(command)),JSON.stringify(model.calls));
  model.state.playlists['/lists/one.m3u8']={files:[],errors:[]};await model.refresh();assert.deepEqual(model.snapshot().paths,[],'valid empty playlist clears membership');assert.equal(model.canRefresh(),true,'source remains refreshable after an empty read');
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

test('production action, inline converter and file-move callbacks carry playlist row rank through known file changes until Refresh',async()=>{
  for(const route of ['execute','rename','move']){const model=runtime();await model.loadPaths(['/lists/one.m3u8'],{replace:true});const original=structuredClone(model.state.playlists);await model.sort();
    model.state.renameBatch={entries:[{path:'/music/renamed.wav',filename:'renamed.wav',fields:{},covers:[]}],errors:[],renamed:[{from:'/music/b.wav',to:'/music/renamed.wav'}]};
    if(route==='rename')assert.equal(await model.rename('/music/b.wav','renamed.wav'),'/music/renamed.wav');else await model[route]();
    await model.playlistOrder();assert.deepEqual(model.snapshot().paths,['/music/renamed.wav','/music/a.wav'],route);assert.deepEqual(model.snapshot().order,['/music/renamed.wav','/music/a.wav'],route);assert.deepEqual(model.snapshot().roots,['/lists/one.m3u8']);assert.deepEqual(model.state.playlists,original,'no source playlist rewrite');
    model.state.playlists['/lists/one.m3u8']={files:['/music/a.wav'],errors:[{path:'/lists/one.m3u8',error:'Line 1: b.wav: file not found'}]};await model.refresh();assert.deepEqual(model.snapshot().paths,['/music/a.wav']);assert.match(model.snapshot().errors[0].error,/b.wav/);
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
