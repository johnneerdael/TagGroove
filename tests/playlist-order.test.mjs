import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/playlist-order.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {mergeSourceOrder,rankSourceRows,sourceFolders}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const chapters=ts.transpileModule(await readFile(new URL('../src/chapters.ts',import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {replaceFileRows}=await import(`data:text/javascript;base64,${Buffer.from(chapters).toString('base64')}`);

test('grouped loads preserve first membership rank without accepting later manual order as source order',()=>{
  const initial=['/a.cue\u00012','/a.wav','/a.cue\u00011'];
  assert.deepEqual(mergeSourceOrder(initial,['/b.wav','/a.wav'],false),[...initial,'/b.wav']);
  assert.deepEqual(mergeSourceOrder(initial,['/b.wav'],true),['/b.wav']);
  const rows=['/b.wav','/a.cue\u00011','/a.wav','/a.cue\u00012','/new.wav'].map(path=>({path}));
  assert.deepEqual(rankSourceRows(rows,initial).map(row=>row.path),[...initial,'/b.wav','/new.wav']);
  assert.deepEqual(rows.map(row=>row.path),['/b.wav','/a.cue\u00011','/a.wav','/a.cue\u00012','/new.wav']);
  assert.deepEqual(rankSourceRows(rows.filter(row=>row.path!=='/a.wav'),initial).map(row=>row.path),['/a.cue\u00012','/a.cue\u00011','/b.wav','/new.wav'],'removed members stay removed until refresh');
});

test('source folder consumers use filesystem kinds rather than suffixes or listed rows',()=>{
  assert.deepEqual(sourceFolders(['/lists/album.m3u8','/music/one.wav','/collection','/music/archive.m3u'],{'/lists/album.m3u8':'file','/music/one.wav':'file','/collection':'directory','/music/archive.m3u':'directory'}),['/lists','/music','/collection','/music/archive.m3u']);
  assert.deepEqual(sourceFolders(['/missing.m3u','/unknown'],{'/missing.m3u':'unknown'}),[],'unknown roots never imply a folder');
});

test('resolved source folders keep native separator semantics',()=>{
  assert.deepEqual(sourceFolders(['/alias/song.wav'],{'/alias/song.wav':'file'},{'/alias/song.wav':'/physical/literal\\name.wav'}),['/physical']);
  assert.deepEqual(sourceFolders(['C:\\alias\\song.wav'],{'C:\\alias\\song.wav':'file'},{'C:\\alias\\song.wav':'D:\\physical\\song.wav'}),['D:\\physical']);
});

test('root-level POSIX files and playlists retain the filesystem root as folder context',()=>{
  assert.deepEqual(sourceFolders(['/song.mp3','/album.m3u8','/'],{'/song.mp3':'file','/album.m3u8':'file','/':'directory'}),['/','/','/']);
});

test('drive-root files and playlists retain an absolute Windows root with its separator',()=>{
  const roots=["C:\\song.mp3", "D:/song.mp3", "C:\\album.pls", "D:/album.xspf", "C:\\"];
  assert.deepEqual(sourceFolders(roots,Object.fromEntries(roots.map((path,index)=>[path,index===4?'directory':'file']))),["C:\\", "D:/", "C:\\", "D:/", "C:\\"]);
  const files=["\\\\server\\share\\song.mp3", "C:\\music\\song.mp3"];
  assert.deepEqual(sourceFolders(files,Object.fromEntries(files.map(path=>[path,'file']))),["\\\\server\\share", "C:\\music"]);
});

test('the existing row replacement policy retains source rank on parent/chapter transitions and partial row saves',()=>{
  const parent='/book.m4b',one=parent+'\u00011',two=parent+'\u00012',last='/last.wav';
  const expanded=replaceFileRows([parent,last],[last,one,two]);assert.deepEqual(expanded,[one,two,last]);
  assert.strictEqual(replaceFileRows(expanded,[two]),expanded,'saving one existing row must not collapse or reorder its siblings');
  const collapsed=replaceFileRows(expanded,[last,parent]);assert.deepEqual(collapsed,[parent,last]);
  assert.deepEqual(rankSourceRows([{path:last},{path:parent}],collapsed).map(row=>row.path),[parent,last]);
});
