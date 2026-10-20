import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {loadProductionModule} from './helpers/load-production-module.mjs';

const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const declaration=name=>parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name);
const javascript=ts.transpileModule(`${declaration('commitCell').getText(parsed)}\n${declaration('renameFromList').getText(parsed)}\nreturn commitCell;`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;

function host({key='_FILENAME_EXT',desktop=true,unsupportedReason=''}={}){
  const entry={path:'/Music/old.wav',filename:'old.wav'},column={id:'filename',key};
  const calls=[],presented=[],notices=[];
  const batch={entries:[],errors:[],renamed:[{from:entry.path,to:'/Music/result.wav'}]};
  const dependencies={
    entries:new Map([[entry.path,entry]]),columnLayout:{columns:[column]},editKey:value=>value.key,
    desktop,sourcePolicy:()=>({unsupportedReason}),render:()=>{},
    run:async(_,work)=>work(),invoke:async(command,args)=>{calls.push([command,args]);return batch;},
    applyRenamed:value=>presented.push(value),accept:(value,label,notify)=>notices.push([value,label,notify]),
  };
  const commit=new Function(...Object.keys(dependencies),`let errors=[],queuedNotices=[];${javascript}`)(...Object.values(dependencies));
  return {commit:value=>commit({path:entry.path,column:column.id},value),calls,presented,notices,batch};
}

test('inline filename commits forward literal text without the converter language',async()=>{
  const h=host();
  assert.equal(await h.commit("O'Brien %title% $upper(x).wav"),'/Music/result.wav');
  assert.deepEqual(h.calls,[['rename_file_from_list',{path:'/Music/old.wav',name:"O'Brien %title% $upper(x).wav"}]]);
  assert.deepEqual(h.presented,[h.batch]);
  assert.deepEqual(h.notices,[[h.batch,'Renamed',false]]);
});

test('inline filename folder destinations reach the literal backend for both columns',async()=>{
  for(const key of ['_FILENAME','_FILENAME_EXT']){
    const h=host({key}),name=key==='_FILENAME'?'Album/Disc 1/Track':'Album/Disc 1/Track.wav';
    assert.equal(await h.commit(name),'/Music/result.wav');
    assert.deepEqual(h.calls,[['rename_file_from_list',{path:'/Music/old.wav',name}]]);
  }
});

test('inline filename forwards native interpretation of backslashes and a different suffix',async()=>{
  for(const name of ['Björk\\Live.wav','Track.flac']){
    const h=host();
    assert.equal(await h.commit(name),'/Music/result.wav');
    assert.deepEqual(h.calls,[['rename_file_from_list',{path:'/Music/old.wav',name}]]);
  }
});

test('unchanged and empty inline names do not issue a rename',async()=>{
  for(const [key,name] of [['_FILENAME','old'],['_FILENAME_EXT','old.wav'],['_FILENAME_EXT','  ']]){
    const h=host({key});
    assert.equal(await h.commit(name),name.trim()?'/Music/old.wav':undefined);
    assert.deepEqual(h.calls,[]);
  }
});

test('inline filename retains desktop and provider refusal before any command',async()=>{
  for(const options of [{desktop:false},{unsupportedReason:'Provider documents cannot be renamed.'}]){
    const h=host(options);
    assert.equal(await h.commit('Album/Track.wav'),undefined);
    assert.deepEqual(h.calls,[]);
  }
});

test('literal inline rename collision replies use the actual diagnostic normalization path',async()=>{
  const {normalizeBatchDiagnostics,batchErrorText}=await loadProductionModule('batch-errors.ts');
  const choice=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)==='DIAGNOSTIC_COMMANDS');
  const code=ts.transpileModule(`const ${choice.getText(parsed)};${declaration('invokeUntracked').getText(parsed)};return invokeUntracked;`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const diagnostic={kind:'destination_collision',path:'/Music/existing.wav'};
  const raw={entries:[],errors:[{path:'/Music/old.wav',error:batchErrorText(diagnostic)}],renamed:[],error_diagnostics:[{index:0,diagnostic}]};
  const invoke=new Function('normalizeBatchDiagnostics','coreInvoke','perfSink',code)(normalizeBatchDiagnostics,async()=>raw,()=>undefined);
  const result=await invoke('rename_file_from_list',{path:'/Music/old.wav',name:'existing.wav'});
  assert.deepEqual(result.errors[0].diagnostic,diagnostic);
  assert.equal(result.errors[0].diagnosticRaw,undefined);
  raw.errors[0].error='unmatched external text';
  const unmatched=await invoke('rename_file_from_list',{path:'/Music/old.wav',name:'existing.wav'});
  assert.equal(unmatched.errors[0].diagnostic,undefined);
  assert.equal(unmatched.errors[0].diagnosticRaw,true);
  const h=host();
  h.batch.errors.push(...result.errors);
  h.batch.renamed=[];
  assert.equal(await h.commit('existing.wav'),undefined);
});
