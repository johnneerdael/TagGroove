import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/export-ui.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {configurationFilename,copyExportConfiguration,removeExportConfiguration,renameExportConfiguration}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const library=()=>({version:1,configurations:[{id:'a',name:'Text',template:'%title%'},{id:'b',name:'Text copy',template:'%album%'}],preferences:{configuration:'a',filename:'out.txt',append:true,partition:null,write_bom:true,create_directories:false}});
test('duplicate gets independent identity, unique name, and preserves source and preferences',()=>{
 const source=library(),result=copyExportConfiguration(source,'a','c');
 assert.equal(result.configurations.length,3);assert.equal(result.configurations[2].id,'c');assert.equal(result.configurations[2].name,'Text copy 2');assert.equal(result.configurations[2].template,'%title%');assert.equal(result.preferences.configuration,'c');assert.equal(result.preferences.append,true);assert.equal(source.configurations.length,2);
 result.configurations[2].template='changed';assert.equal(result.configurations[0].template,'%title%');
 assert.throws(()=>copyExportConfiguration(source,'missing','c'),/exist/);assert.throws(()=>copyExportConfiguration(source,'a','a'),/identity/);
});
test('a duplicate of a configuration found in the export folder gets its own file',()=>{
 const source=library();source.configurations[0].file='Text.mte';
 const result=copyExportConfiguration(source,'a','c');
 assert.equal(result.configurations[2].file,undefined);assert.equal(result.configurations[0].file,'Text.mte');
});
test('remove updates selected configuration and allows an empty saved library',()=>{
 const source=library(),result=removeExportConfiguration(source,'a');assert.deepEqual(result.configurations.map(c=>c.id),['b']);assert.equal(result.preferences.configuration,'b');assert.equal(source.preferences.configuration,'a');
 const empty=removeExportConfiguration(result,'b');assert.equal(empty.preferences.configuration,null);assert.deepEqual(empty.configurations,[]);assert.equal(empty.preferences.filename,'out.txt');
});
const {ExportUI}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
test('export without the selection dialog runs the saved configuration, per directory on request',async()=>{
 const runs=[];let reason='';
 const ui=new ExportUI({load:async()=>library(),execute:async(saved,perDirectory)=>{runs.push([saved.preferences.configuration,perDirectory]);},reason:()=>reason,selectedCount:()=>2});
 await ui.runSaved();await ui.runSaved(true);
 assert.deepEqual(runs,[['a',false],['a',true]]);
 reason='Select files to export.';await assert.rejects(ui.runSaved(),/Select files/);
 reason='';const missing=new ExportUI({load:async()=>({...library(),preferences:{...library().preferences,configuration:null}}),execute:async()=>runs.push('ran'),reason:()=>'',selectedCount:()=>1});
 await assert.rejects(missing.runSaved(),/export configuration/);assert.equal(runs.length,2);
});

test('rename in place keeps names nonempty and unique without touching the source library',()=>{
 const source=library(),result=renameExportConfiguration(source,'a','  Track list ');
 assert.equal(result.configurations[0].name,'Track list');assert.equal(source.configurations[0].name,'Text');
 assert.equal(renameExportConfiguration(source,'a','TEXT').configurations[0].name,'TEXT','changing only the case of its own name is allowed');
 assert.throws(()=>renameExportConfiguration(source,'a','text COPY'),/already exists/);
 assert.throws(()=>renameExportConfiguration(source,'a','  '),/Enter a name/);
 assert.throws(()=>renameExportConfiguration(source,'zz','x'),/exist/);
});
test('choosing a configuration prefills the export file name from its $filename',()=>{
 const html='$filename(html,utf-8)<html>$loop(1)%title%$loopend()';
 assert.equal(configurationFilename('export.txt',html),'export.html');
 assert.equal(configurationFilename('C:\\Lists\\%album%.txt',html),'C:\\Lists\\%album%.html');
 assert.equal(configurationFilename('/lists/name',' $FILENAME(csv)'),'/lists/name.csv');
 assert.equal(configurationFilename('/lists/v1.0/export.txt','\uFEFF$filename(tracks.cue,utf-8)'),'/lists/v1.0/tracks.cue');
 assert.equal(configurationFilename('out/export.txt','$filename(/abs/list.json)'),'/abs/list.json');
 assert.equal(configurationFilename('C:\\Out\\export.txt','$filename(D:\\Lists\\all.htm)'),'D:\\Lists\\all.htm');
 // A dynamic first argument is left to the export: the field is cleared, so no
 // typed name replaces it (out.%EXPORT_NAME% would have written out.report.csv).
 for(const template of ['$filename(%EXPORT_NAME%,utf-8)','$filename(%_workingpath%list.m3u)','$filename($lower(%album%).htm,ansi)',"$filename('list (all)'.txt)",'$filename($if(%album%,csv,txt))'])
  assert.equal(configurationFilename('out.txt',template),'',template);
 // No $filename, an empty argument or an empty field (the configuration's default) keeps the field.
 assert.equal(configurationFilename('out/export.txt','$loop(1)%title%$loopend()'),'out/export.txt');
 assert.equal(configurationFilename('out/export.txt','$filename(,utf-16)body'),'out/export.txt');
 assert.equal(configurationFilename('',html),'');
});

test('relative filename subpaths keep the current output folder while rooted paths replace it',()=>{
 for(const [current,argument,expected] of [
  ['/lists/export.txt','sub/report.csv','/lists/sub/report.csv'],
  ['/lists/export.txt','../report.csv','/lists/../report.csv'],
  ['lists/export.txt','sub/report.csv','lists/sub/report.csv'],
  ['export.txt','sub/report.csv','sub/report.csv'],
  ['/lists/export.txt','é:report.csv','/lists/é:report.csv'],
  ['C:\\Lists\\export.txt','sub\\report.csv','C:\\Lists\\sub\\report.csv'],
  ['\\\\server\\share\\Lists\\export.txt','sub\\report.csv','\\\\server\\share\\Lists\\sub\\report.csv'],
  ['\\\\?\\C:\\Lists\\export.txt','sub\\report.csv','\\\\?\\C:\\Lists\\sub\\report.csv'],
  ['/lists/export.txt','/elsewhere/report.csv','/elsewhere/report.csv'],
  ['C:\\Lists\\export.txt','D:\\Elsewhere\\report.csv','D:\\Elsewhere\\report.csv'],
  ['C:\\Lists\\export.txt','\\\\server\\share\\report.csv','\\\\server\\share\\report.csv'],
  ['C:\\Lists\\export.txt','\\rooted\\report.csv','\\rooted\\report.csv'],
 ]) assert.equal(configurationFilename(current,`$filename(${argument},utf-8)`),expected,`${current} / ${argument}`);
});
