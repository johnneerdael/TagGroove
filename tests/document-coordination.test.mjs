import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Exercise production coordination against a controlled provider and dialog boundary.
// Avoid importing main.ts's platform bootstrap or replacing its draft/save/refresh logic.
const main=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const helper=await readFile(new URL('../src/document-state.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const names=new Set(['updateDraft','run','saveDrafts','saveCopy','value','accept','operateImpl','refreshSourceAccess','providerEntry','copyBaseUnknown','editedCopyFields','discardDrafts']);
const mergedDeclaration=parsed.statements.filter(ts.isVariableStatement).flatMap(statement=>statement.declarationList.declarations).find(declaration=>declaration.name.getText(parsed)==='merged');
const functions=parsed.statements.filter(node=>ts.isFunctionDeclaration(node)&&names.has(node.name?.text)).map(node=>node.getText(parsed)).join('\n');
const factory=`${helper.replace(/\bexport /g,'')}
export function createWorkspace(initial, options={}) {
  const path='retained.wav', original=structuredClone(initial);
  let working=structuredClone(initial), copied, dialog, status='', busy=false, errors=[], location='', anchor;
  const makeEntry=(path,fields)=>({path,filename:path,format:'WAV',duration:1,bitrate:1411,sample_rate:44100,channels:2,size:44,fields:structuredClone(fields),technical:{},covers:[]});
  const entries=new Map([[path,makeEntry(path,initial)]]),drafts=new Map(),copyOverrides=new Map(),selected=new Set([path]);
  const ready={uri:'content://provider/audio',name:path,writable:true,persisted:true,state:'ready',error:null};
  let documentAccess={states:{[path]:ready},refreshError:null}, nativeState=ready, refreshFails=!!options.refreshFails;
  const runtimePlatform={document_access_supported:options.provider!==false};
  const paths=()=>[...selected],dirtySelected=()=>paths().some(path=>drafts.has(path));
  const render=()=>{},changedData=()=>{},storeEntry=entry=>entries.set(entry.path,entry),esc=value=>String(value),publicMessage=value=>String(value),displayName=value=>value;
  const merged=${mergedDeclaration.initializer.getText(parsed)};
  const enabled=()=>true,ask=async()=>true,modalError=()=>{};
  const applyFields=(base,updates)=>{const result=structuredClone(base);for(const [key,values] of Object.entries(updates)){if(values.length)result[key]=structuredClone(values);else delete result[key];}return result;};
  async function invoke(command,args) {
    if(command==='save_fields') {
      working=applyFields(working,args.updates);
      nativeState={...ready,state:'recovery_required',error:'Source publication failed after local edit'};
      return {entries:[],errors:[{path,error:'Source publication failed after local edit'}]};
    }
    if(command==='document_states') {if(refreshFails)throw new Error('State refresh unavailable');return {[path]:nativeState};}
    if(command==='document_save_copy') {
      copied=applyFields(args.version==='original'?original:working,args.updates);
      return {entries:[makeEntry('copied.wav',copied)],errors:[]};
    }
    throw new Error('Unexpected native boundary: '+command);
  }
  function modal() {
    const controls=new Map();
    for(const selector of ['[data-close]','#copy-version','#save-document-copy','.copy-status'])controls.set(selector,{value:'edited',addEventListener(event,handler){this[event]=handler;}});
    dialog={open:true,controls,querySelector:selector=>controls.get(selector),setAttribute(){},addEventListener(){},close(){this.open=false;}};
    return dialog;
  }
  ${functions}
  return {
    edit:(key,values)=>updateDraft(path,key,values),save:()=>saveDrafts(),
    discard:()=>operateImpl('discard'),refresh:()=>operateImpl('refresh-source'),
    setRefreshFailure:failure=>refreshFails=failure,
    acceptLoaded:fields=>{working=structuredClone(fields);nativeState=ready;accept({entries:[makeEntry(path,fields)],errors:[]},'Reloaded');},
    copy:async(version='edited')=>{selected.clear();selected.add(path);saveCopy();dialog.controls.get('#copy-version').value=version;await dialog.controls.get('#save-document-copy').click();return structuredClone(copied);},
    snapshot:()=>({fields:structuredClone(merged(entries.get(path))),drafts:structuredClone([...drafts]),copyOverrides:structuredClone([...copyOverrides]),errors:structuredClone(errors),status,busy}),
  };
}`;
const javascript=ts.transpileModule(factory,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {createWorkspace}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

for(const refreshFails of [false,true]) {
  test(`failed publication then reverting to loaded tags exports the displayed intent (${refreshFails?'unknown':'recovery'})`,async()=>{
    const workspace=createWorkspace({TITLE:['A']},{refreshFails});
    workspace.edit('TITLE',['B']);await workspace.save();workspace.edit('TITLE',['A']);
    assert.deepEqual(workspace.snapshot().fields,{TITLE:['A']});
    assert.deepEqual(await workspace.copy(),{TITLE:['A']});
  });
}
test('restoring a removed tag after failed publication exports the restored value',async()=>{
  const workspace=createWorkspace({TITLE:['A']});
  workspace.edit('TITLE',[]);await workspace.save();workspace.edit('TITLE',['A']);
  assert.deepEqual(await workspace.copy(),{TITLE:['A']});
});
test('removing a newly added tag after failed publication exports an explicit deletion',async()=>{
  const workspace=createWorkspace({TITLE:['A']});
  workspace.edit('NEW',['B']);await workspace.save();workspace.edit('NEW',[]);
  const copied=await workspace.copy();assert.equal(Object.hasOwn(copied,'NEW'),false);assert.deepEqual(copied,{TITLE:['A']});
});
for(const field of ['TITLE','NEW']) {
  test(`Discard after failed publication clears pending drafts and exports displayed loaded tags (${field})`,async()=>{
    const workspace=createWorkspace({TITLE:['A']});
    workspace.edit(field,['B']);await workspace.save();await workspace.discard();
    const state=workspace.snapshot();assert.deepEqual(state.drafts,[]);assert.deepEqual(state.fields.TITLE,['A']);assert.equal((state.fields.NEW||[]).length,0);assert.match(state.status,/discarded/i);
    assert.deepEqual(await workspace.copy(),{TITLE:['A']});
  });
}
test('desktop and ready provider undo-to-loaded leave no pending draft',()=>{
  for(const provider of [false,true]){const workspace=createWorkspace({TITLE:['A']},{provider});workspace.edit('TITLE',['B']);workspace.edit('TITLE',['A']);assert.deepEqual(workspace.snapshot().drafts,[]);assert.deepEqual(workspace.snapshot().copyOverrides,[]);assert.deepEqual(workspace.snapshot().fields,{TITLE:['A']});}
});
test('original-backup copy ignores current recovery edits',async()=>{
  const workspace=createWorkspace({TITLE:['A']});workspace.edit('TITLE',['B']);await workspace.save();workspace.edit('TITLE',['A']);workspace.edit('COMMENT',['C']);
  assert.deepEqual(await workspace.copy('original'),{TITLE:['A']});
});
for(const refreshFails of [true,false]) {
  test(`manual ${refreshFails?'failed':'successful'} source refresh retains the original publication error`,async()=>{
    const workspace=createWorkspace({TITLE:['A']},{refreshFails:true});workspace.edit('TITLE',['B']);await workspace.save();
    const before=workspace.snapshot();assert.match(before.errors[0].error,/publication failed/);
    workspace.setRefreshFailure(refreshFails);await workspace.refresh();
    assert.deepEqual(workspace.snapshot().errors,before.errors);assert.deepEqual(workspace.snapshot().drafts,before.drafts);assert.equal(workspace.snapshot().busy,false);
  });
}

test('accepting a verified source baseline clears old recovery copy overrides',async()=>{
  const workspace=createWorkspace({TITLE:['A']});workspace.edit('TITLE',['B']);await workspace.save();workspace.edit('TITLE',['A']);
  assert.notDeepEqual(workspace.snapshot().copyOverrides,[]);
  workspace.acceptLoaded({TITLE:['Fresh source']});
  assert.deepEqual(workspace.snapshot().copyOverrides,[]);
  assert.deepEqual(workspace.snapshot().drafts,[]);
  assert.deepEqual(workspace.snapshot().fields,{TITLE:['Fresh source']});
  assert.deepEqual(await workspace.copy(),{TITLE:['Fresh source']});
});
