import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>{
  const source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const chapters=await load('chapters'),options=await load('options-ui');

test('row paths name their file and subsong',()=>{
  assert.deepEqual(chapters.subsongParts('/m/book.m4b\u00012'),{file:'/m/book.m4b',number:2});
  assert.equal(chapters.subsongParts('/m/book.m4b'),undefined);
  assert.equal(chapters.subsongParts('/m/book.m4b\u00010'),undefined);
  assert.equal(chapters.subsongParts('\u00011'),undefined);
  assert.equal(chapters.isChapterRow('C:\\a\\b.mka\u000112'),true);
});

test('chapter rows run whole-file commands on their file and refuse renaming, moving and playlists',()=>{
  const row='/m/book.m4b\u00011',file='/m/song.mp3',cue='/m/live.cue\u00013';
  for(const op of ['save','extended','reload','copy-tags','paste-tags','action','groups','filename-tags','tag-tag','number','export-config','csv','source-musicbrainz','file-play','file-reveal','remove-tags','add-cover','remove-cover','cover-manage','remove-tag-types','strip-mp4-chapters','util-optimize','util-unsupported'])
    assert.equal(chapters.chapterRowReason(op,[row],()=>[row]),'',op);
  for(const op of ['tags-filename','filename-filename','file-move','file-copy','file-delete','save-copy','playlist-selected'])
    assert.equal(chapters.chapterRowReason(op,[file,row]),chapters.CHAPTER_ROW_REASON,op);
  assert.equal(chapters.chapterRowReason('tags-filename',[file],()=>[file,row]),'');
  assert.equal(chapters.chapterRowReason('playlist-all',[file],()=>[file,row]),chapters.CHAPTER_ROW_REASON);
  // Cue sheet tracks edit the sheet: whole-file commands are refused, playing is not.
  for(const op of ['remove-tags','add-cover','util-optimize','strip-mp4-chapters'])
    assert.equal(chapters.chapterRowReason(op,[cue]),chapters.CUE_ROW_REASON,op);
  assert.equal(chapters.chapterRowReason('file-play',[cue]),'');
  assert.equal(chapters.rowFile(row),'/m/book.m4b');
  assert.equal(chapters.rowFile(file),file);
});

test('whole-file results replace a file\'s rows in place',()=>{
  const r=(n,f='/m/book.m4b')=>`${f}\u0001${n}`;
  const order=['/m/a.mp3',r(1),r(2),r(3),'/m/z.mp3'];
  // Same rows back: nothing moves.
  assert.equal(chapters.replaceFileRows(order,[r(1),r(2),r(3)],['/m/book.m4b']),order);
  // A save returning one row changes nothing.
  assert.equal(chapters.replaceFileRows(order,[r(2)]),order);
  // Chapters stripped: the file replaces its rows where they were.
  assert.deepEqual(chapters.replaceFileRows(order,['/m/book.m4b'],['/m/book.m4b']),['/m/a.mp3','/m/book.m4b','/m/z.mp3']);
  // Fewer chapters after a whole-file command: the stale row leaves.
  assert.deepEqual(chapters.replaceFileRows(order,[r(1),r(2)],['/m/book.m4b']),['/m/a.mp3',r(1),r(2),'/m/z.mp3']);
  // Undo brings the rows back in place of the file.
  assert.deepEqual(chapters.replaceFileRows(['/m/a.mp3','/m/book.m4b'],[r(1),r(2)]),['/m/a.mp3',r(1),r(2)]);
  // Unlisted files are not added.
  assert.equal(chapters.replaceFileRows(order,['/m/other.mp3']),order);
});

test('Options → Tags → Advanced offers List chapters as separate files, off by default',()=>{
  assert.equal(options.defaultSettings().tags.advanced.list_chapters,false);
  const page=options.pageTree().map(n=>n.page).find(p=>p.id==='tags-advanced');
  assert.ok(page.controls.some(c=>c.path==='tags.advanced.list_chapters'&&c.label==='List chapters as separate files'));
});
