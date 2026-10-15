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

test('chapter rows allow tag editing and refuse whole-file commands',()=>{
  const row='/m/book.m4b\u00011',file='/m/song.mp3';
  for(const op of ['save','extended','reload','copy-tags','paste-tags','action','groups','filename-tags','tag-tag','number','export-config','csv','source-musicbrainz','file-play'])
    assert.equal(chapters.chapterRowReason(op,[row],()=>[row]),'',op);
  for(const op of ['remove-tags','add-cover','cover-manage','strip-mp4-chapters','tags-filename','filename-filename','file-move','file-delete','file-reveal','util-optimize','save-copy','playlist-selected'])
    assert.equal(chapters.chapterRowReason(op,[file,row]),chapters.CHAPTER_ROW_REASON,op);
  assert.equal(chapters.chapterRowReason('remove-tags',[file],()=>[file,row]),'');
  assert.equal(chapters.chapterRowReason('playlist-all',[file],()=>[file,row]),chapters.CHAPTER_ROW_REASON);
  assert.equal(chapters.chapterRowReason('file-play',['/m/live.cue\u00013']),chapters.CHAPTER_ROW_REASON);
});

test('Options → Tags → Advanced offers List chapters as separate files, off by default',()=>{
  assert.equal(options.defaultSettings().tags.advanced.list_chapters,false);
  const page=options.pageTree().map(n=>n.page).find(p=>p.id==='tags-advanced');
  assert.ok(page.controls.some(c=>c.path==='tags.advanced.list_chapters'&&c.label==='List chapters as separate files'));
});
