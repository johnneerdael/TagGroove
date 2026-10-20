import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=path=>readFile(new URL(`../src/${path}`,import.meta.url),'utf8');
const load=async file=>{const js=ts.transpileModule(await source(file),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);};
const {PlaylistUI,isPlaylistPath}=await load('playlist-ui.ts');
const {FileOpsUI,defaultDirectoryState}=await load('file-ops.ts');
const main=await source('main.ts'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
const functionJS=name=>ts.transpileModule(parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name).getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
function pipeline(failure,sorted=false){
  const calls=[],listed=['/Practice/old.wav'];
  const ui=new PlaylistUI({invoke:async(command,args)=>{calls.push([command,args]);if(command==='classify_source_paths')return args.paths.map(path=>({kind:'file',resolved:path}));assert.equal(command,'read_playlist');if(failure)throw new Error(failure);return{files:['/Practice/two.wav','/Practice/one.wav'],errors:[{path:args.path,error:'Line 4: missing.wav: file not found'}]};}});
  const dependencies={calls,isPlaylistPath,playlistUI:ui,publicMessage:String,render:()=>{},sourceReadContext:()=> 'fixture',
    scan:async(paths,options)=>{calls.push(['scan',paths,options]);if(options.replace)listed.splice(0,listed.length,...paths);return true;}};
  const code=`let errors=[],status='',busy=false,sourceGeneration=0,columnLayout={sort:${sorted?"[{id:'filename',ascending:true}]":'[]'}};${functionJS('loadPaths')}return{loadPaths,errors:()=>errors,status:()=>status,playlist:()=>calls.find(([command])=>command==='scan')?.[2].playlist,sort:()=>columnLayout.sort};`;
  return{...new Function(...Object.keys(dependencies),code)(...Object.values(dependencies)),calls,listed};
}

test('XML playlist extensions use the same expansion pipeline with case-insensitive recognition',()=>{
  for(const path of ['/Practice/list.XSPF','C:\\Practice\\list.WPL','list.asx'])assert.ok(isPlaylistPath(path),path);
  for(const path of ['list.xml','list.xspf.bak','/folder.asx/song.wav'])assert.equal(isPlaylistPath(path),false,path);
});

test('queued native file-open and second-instance requests reach actual loadPaths, reader and ordered scan',async()=>{
  for(const extension of ['xspf','wpl','asx']){
    const model=pipeline(),playlist=`/Practice/album.${extension}`;
    const invoke=async command=>command==='load_directory_state'?defaultDirectoryState():command==='take_launch_requests'?[{directories:[],files:[playlist],add:false}]:undefined;
    const files=new FileOpsUI({invoke,changed:()=>{},openReason:()=>'',busy:()=>false,pendingCount:()=>0,preferences:()=>({subfolders:false,startup:'empty'}),load:(paths,options)=>model.loadPaths(paths,options)});
    await files.initialize();
    assert.deepEqual(model.calls[1],['read_playlist',{path:playlist}]);
    assert.deepEqual(model.listed,['/Practice/two.wav','/Practice/one.wav']);
    assert.equal(model.calls[2][0],'scan');assert.equal(model.calls[2][2].replace,true);assert.equal(model.calls[2][2].errors.length,1);assert.equal(model.playlist(),playlist);
  }
});

test('fatal XML reader failure leaves the actual preexisting list unscanned and unchanged',async()=>{
  const model=pipeline('Invalid XML at line 7');
  assert.equal(await model.loadPaths(['/Practice/broken.xspf'],{replace:true}),false);
  assert.deepEqual(model.listed,['/Practice/old.wav']);assert.equal(model.calls.length,2);assert.equal(model.calls[1][0],'read_playlist');
  assert.match(model.errors()[0].error,/Invalid XML at line 7$/);assert.equal(model.status(),'Playlist could not be read');assert.equal(model.playlist(),undefined);
});

test('actual Load playlist picker offers all readable extensions while cancellation leaves the list alone',async()=>{
  const model=pipeline(),picks=[];
  const dependencies={enabled:()=>true,open:async options=>{picks.push(options);return null;},drafts:new Map(),loadPaths:model.loadPaths};
  const operate=new Function(...Object.keys(dependencies),`${functionJS('operateImpl')}return operateImpl;`)(...Object.values(dependencies));
  await operate('playlist-load');
  assert.deepEqual(picks[0].filters[0].extensions,['m3u','m3u8','pls','cue','xspf','wpl','asx']);
  assert.deepEqual(model.listed,['/Practice/old.wav']);assert.deepEqual(model.calls,[]);
});


test('XML loading passes original source roots and ordered members without clearing saved column sorting',async()=>{
  const model=pipeline(undefined,true);
  assert.equal(await model.loadPaths(['/Practice/order.xspf'],{replace:true}),true);
  assert.deepEqual(model.listed,['/Practice/two.wav','/Practice/one.wav']);
  assert.deepEqual(model.sort(),[{id:'filename',ascending:true}]);assert.deepEqual(model.calls[2][2].roots,['/Practice/order.xspf']);
  assert.deepEqual(model.calls.map(([command])=>command),['classify_source_paths','read_playlist','scan']);
});
