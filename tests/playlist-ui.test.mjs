import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/playlist-ui.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {isPlaylistPath,playlistValues,PLAYLIST_DEFAULTS,playlistOptionsPage,exportOptionsPage,EXPORT_DEFAULTS,PlaylistUI}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);

test('playlists and cue sheets are recognised by extension',()=>{
  for(const path of ['/m/a.m3u','C:\\m\\b.M3U8','x.pls','disc.CUE'])assert.ok(isPlaylistPath(path),path);
  for(const path of ['/m/a.mp3','/m/m3u','/m.m3u/a.flac','list.m3u.bak'])assert.ok(!isPlaylistPath(path),path);
});

test('options pages persist only behaviour-changing values with defaults',()=>{
  const paths=page=>page.controls.flatMap(control=>control.path?[control.path]:[]);
  assert.deepEqual(paths(playlistOptionsPage).map(path=>path.replace('pages.playlist.','')).sort(),['auto_create','confirm_filename','dont_create_directories','extended','extinf_format','filename','relative','write_bom']);
  for(const path of paths(playlistOptionsPage))assert.ok(path.replace('pages.playlist.','') in playlistOptionsPage.defaults,path);
  assert.deepEqual(paths(exportOptionsPage).map(path=>path.replace('pages.export.','')).sort(),Object.keys(EXPORT_DEFAULTS).sort());
  assert.equal(playlistOptionsPage.validate({pages:{playlist:{filename:'  '}}}),'Enter a filename of playlist.');
  assert.equal(playlistOptionsPage.validate({pages:{playlist:{filename:'%album%.m3u8'}}}),undefined);
  assert.deepEqual(playlistValues({filename:7,confirm_filename:'no',auto_create:true}),{filename:PLAYLIST_DEFAULTS.filename,confirm:true,auto:true,generatorFilename:PLAYLIST_DEFAULTS.generator_filename,generatorPartition:PLAYLIST_DEFAULTS.generator_partition});
});

function host(overrides={}){
  const calls=[],done=[];
  return {calls,done,host:{
    invoke:async(command,args)=>{calls.push([command,structuredClone(args)]);
      if(command==='classify_source_paths')return args.paths.map(path=>({kind:'file',resolved:path}));
      if(command==='read_playlist')return args.path.endsWith('b.cue')?{files:['/m/disc.flac'],errors:[]}:{files:['/m/2.mp3','/m/1.mp3'],errors:[{path:args.path,error:'Line 3: gone.mp3: file not found'}]};
      if(command==='create_playlists')return {playlists:[{path:'/w/list.m3u8',files:args.request.paths.length}],errors:[]};
      throw new Error(`unexpected ${command}`);},
    values:()=>({confirm_filename:false,filename:'%album%.m3u8'}),remember:async()=>{},
    files:scope=>scope==='all'?['/m/1.mp3','/m/2.mp3','/m/3.mp3']:['/m/2.mp3'],
    workingDirectory:()=>'/w',loadedPlaylist:()=>'/m/a.m3u8',columns:paths=>[{paths}],
    run:async(_message,job)=>job(),done:(plan,interactive)=>{done.push([plan,interactive]);},...overrides}};
}

test('expanding keeps playlist order, ordinary paths and missing-entry errors',async()=>{
  const {host:h,calls}=host(),ui=new PlaylistUI(h);
  const result=await ui.expand(['/m/a.m3u8','/m/x.flac','/m/b.cue']);
  assert.deepEqual(result.paths,['/m/2.mp3','/m/1.mp3','/m/x.flac','/m/disc.flac']);
  assert.equal(result.errors.length,1);assert.equal(result.playlist,'/m/b.cue');
  assert.deepEqual(calls.filter(([command])=>command==='read_playlist').map(([command,args])=>[command,args.path]),[['read_playlist','/m/a.m3u8'],['read_playlist','/m/b.cue']]);
  assert.deepEqual(await ui.expand(['/m/x.flac']),{paths:['/m/x.flac'],errors:[],rootKinds:['file'],rootResolved:['/m/x.flac']});
});

test('playlist creation sends list order, working directory and loaded playlist',async()=>{
  const {host:h,calls,done}=host(),ui=new PlaylistUI(h);
  await ui.create('all');await ui.create('selected',true);
  assert.deepEqual(calls.map(([command])=>command),['create_playlists','create_playlists']);
  assert.deepEqual(calls[0][1].request,{paths:['/m/1.mp3','/m/2.mp3','/m/3.mp3'],filename_template:'%album%.m3u8',partition_template:null,base_directory:'/w',playlist:'/m/a.m3u8',columns:[{paths:['/m/1.mp3','/m/2.mp3','/m/3.mp3']}]});
  assert.deepEqual(calls[1][1].request.paths,['/m/2.mp3']);
  assert.deepEqual(done.map(([plan,interactive])=>[plan.playlists[0].files,interactive]),[[3,true],[1,false]]);
  const empty=host({files:()=>[]});await new PlaylistUI(empty.host).create('all');assert.equal(empty.calls.length,0);
});
