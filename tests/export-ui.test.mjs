import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/export-ui.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {copyExportConfiguration,removeExportConfiguration}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const library=()=>({version:1,configurations:[{id:'a',name:'Text',template:'%title%'},{id:'b',name:'Text copy',template:'%album%'}],preferences:{configuration:'a',filename:'out.txt',append:true,partition:null,write_bom:true,create_directories:false}});
test('duplicate gets independent identity, unique name, and preserves source and preferences',()=>{
 const source=library(),result=copyExportConfiguration(source,'a','c');
 assert.equal(result.configurations.length,3);assert.equal(result.configurations[2].id,'c');assert.equal(result.configurations[2].name,'Text copy 2');assert.equal(result.configurations[2].template,'%title%');assert.equal(result.preferences.configuration,'c');assert.equal(result.preferences.append,true);assert.equal(source.configurations.length,2);
 result.configurations[2].template='changed';assert.equal(result.configurations[0].template,'%title%');
 assert.throws(()=>copyExportConfiguration(source,'missing','c'),/exist/);assert.throws(()=>copyExportConfiguration(source,'a','a'),/identity/);
});
test('remove updates selected configuration and allows an empty saved library',()=>{
 const source=library(),result=removeExportConfiguration(source,'a');assert.deepEqual(result.configurations.map(c=>c.id),['b']);assert.equal(result.preferences.configuration,'b');assert.equal(source.preferences.configuration,'a');
 const empty=removeExportConfiguration(result,'b');assert.equal(empty.preferences.configuration,null);assert.deepEqual(empty.configurations,[]);assert.equal(empty.preferences.filename,'out.txt');
});
