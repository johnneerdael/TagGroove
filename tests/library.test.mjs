import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>{
  const source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const library=await load('library');

test('library values fall back to a disabled library without directories',()=>{
  assert.deepEqual(library.libraryOptions(undefined),{enabled:false,restrict:false,directories:[]});
  assert.deepEqual(library.libraryOptions({enabled:true,restrict:'yes',directories:['/music',' ',3,'D:\\Audio']}),{enabled:true,restrict:false,directories:['/music','D:\\Audio']});
});

test('adding directories skips folders already listed',()=>{
  assert.deepEqual(library.addDirectories(['/music'],['/music/','/other',' ','/Other']),['/music','/other']);
  assert.deepEqual(library.addDirectories([],['C:\\Music\\']),['C:\\Music\\']);
});

test('status and cleanup messages',()=>{
  assert.equal(library.formatBytes(512),'512 B');
  assert.equal(library.formatBytes(1536),'1.5 KB');
  assert.equal(library.formatBytes(25*1024*1024),'25 MB');
  const status={enabled:true,created:true,files:1234,bytes:2*1024*1024,location:'/cfg/library/library.db',last_hits:0,last_stored:0};
  assert.equal(library.libraryStatusText(status),`${(1234).toLocaleString()} files in the library (2.0 MB).`);
  assert.match(library.libraryStatusText({...status,files:1,last_hits:3,last_stored:1}),/^1 file in the library .* Last load: 3 from the library, 1 read from disk and stored\.$/);
  assert.match(library.libraryStatusText({...status,created:false}),/empty/);
  assert.equal(library.cleanupText({checked:10,removed:1,cancelled:false}),'Cleanup checked 10 files and removed 1 orphaned entry.');
  assert.equal(library.cleanupText({checked:4,removed:2,cancelled:true}),'Cleanup cancelled after 4 files; removed 2 orphaned entries.');
});

test('the page sits between Appearance and Tags with Mp3tag’s controls',()=>{
  const page=library.libraryOptionsPage({});
  assert.equal(page.id,'library');assert.equal(page.label,'Library');
  assert.ok(page.order>40&&page.order<50);
  assert.deepEqual(page.defaults,{enabled:false,restrict:false,directories:[]});
  const flat=controls=>controls.flatMap(c=>c.kind==='group'?flat(c.controls):[c]);
  const controls=flat(page.controls);
  assert.deepEqual(controls.filter(c=>c.kind==='checkbox').map(c=>[c.path,c.label]),[['pages.library.enabled','Enable Library'],['pages.library.restrict','Only include these directories']]);
  const draft={pages:{library:{enabled:true,restrict:false,directories:['/a<b']}}};
  const list=controls.find(c=>c.id==='library-directories');
  assert.deepEqual(list.paths,['pages.library.directories']);
  const markup=list.render(draft);
  assert.match(markup,/\/a&lt;b/);
  assert.match(markup,/id="library-add" disabled/,'the list is disabled without the restriction');
  draft.pages.library.restrict=true;
  assert.doesNotMatch(list.render(draft),/id="library-add" disabled/);
  draft.pages.library.directories=[];
  assert.match(list.render(draft),/nothing is stored/);
  assert.match(controls.find(c=>c.id==='library-maintenance').render(draft),/Cleanup Library/);
});
