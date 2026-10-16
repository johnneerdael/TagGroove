import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const root = new URL('../', import.meta.url);
const source = await readFile(new URL('src/file-ops.ts', root), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {FileOpsUI} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const opening = await readFile(new URL('docs/user-guide/getting-started/opening-files.md', root),'utf8');
const windowPage = await readFile(new URL('docs/user-guide/getting-started/main-window.md', root),'utf8');
const forms = text => {const lf=text.replace(/\r\n/g,'\n');return [lf,lf.replace(/\n/g,'\r\n')];};
const rows = (text, marker) => text.split(`<!-- ${marker}:start -->`)[1].split(`<!-- ${marker}:end -->`)[0].split(/\r?\n/).filter(line=>line.startsWith('|')).slice(2).map(line=>line.split('|').slice(1,-1).map(v=>v.trim()));
function harness({pending=0,answer=true,startup='empty',requests=[]}={}) {
 const loads=[], confirmations=[],pickCalls=[];
 const state={version:1,recent:['/previous'],favorites:['/favorite']};
 const host={mac:false,selection:()=>[],pendingCount:()=>pending,confirm:async()=>{confirmations.push('discard');return answer;},
 pickDirectories:async(title,multiple)=>{pickCalls.push({title,multiple});return ['/practice'];},
 invoke:async command=>command==='take_launch_requests'?requests:structuredClone(state),
 load:async(paths,options)=>{loads.push({paths,options});return true;},preferences:()=>({subfolders:true,startup}),
 changed(){},openReason:()=>'',run:async(_message,job)=>job(),report(){},removeEntries(){},moved(){}};
 return {ui:new FileOpsUI(host),loads,confirmations,pickCalls};
}
test('written replacement and append routes use the actual loading controller', async()=>{
 for(const text of forms(opening)) {
  const cases=rows(text,'first-use-load-scope'); assert.equal(cases.length,5);
  for(const [index,operation] of ['folder','dir-add','drop'].entries()) {
   const fx=harness();fx.ui.history.visit('/old');
   if(operation==='drop')await fx.ui.replaceWith(['/practice']);else await fx.ui.operate(operation);
   const row=cases[operation==='drop'?3:index];
   assert.equal(fx.loads.length,1);assert.equal(fx.loads[0].options.replace,row[1]==='Replace');
   assert.equal(fx.ui.history.current,operation==='folder'?'/practice':'/old');
  }
  // An unsaved-work rejection blocks the actual replacement picker/load,
  // whereas Add directory does not discard the old workspace.
  const stopped=harness({pending:2,answer:false});await stopped.ui.operate('folder');
  assert.equal(stopped.loads.length,0);assert.equal(stopped.pickCalls.length,0);assert.equal(stopped.confirmations.length,1);
  await stopped.ui.operate('dir-add');assert.equal(stopped.loads[0].options.replace,false);
 }
});
test('startup folder restoration yields to an actual queued launch request',async()=>{
 for(const startup of ['empty','previous','favorite']) {
  const fx=harness({startup});await fx.ui.initialize();
  assert.equal(fx.loads.length,startup==='empty'?0:1);
  if(startup!=='empty')assert.deepEqual(fx.loads[0].paths,[startup==='previous'?'/previous':'/favorite']);
  const launched=harness({startup,requests:[{directories:['/requested'],files:[],add:false}]});await launched.ui.initialize();
  assert.equal(launched.loads.length,1);assert.deepEqual(launched.loads[0].paths,['/requested']);
 }
});
test('written loaded-visible-selected counts execute the actual main-window producer',async()=>{
 const main=await readFile(new URL('src/main.ts',root),'utf8');
 const ast=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
 const declarations=ast.statements.filter(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='countText');assert.equal(declarations.length,1);
 const producer=ts.transpileModule(declarations[0].getText(ast),{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
 for(const text of forms(windowPage)) {const cases=rows(text,'first-use-count-cases');assert.equal(cases.length,2);
  for(const row of cases){const values=row.slice(0,3).map(value=>Number(value.slice(1,-1)));
   const execute=new Function('entries','selected','filterPending',`${producer};return countText(Array(${values[1]}));`);
   const expected=row[3].slice(1,-1);assert.equal(execute({size:values[0]},{size:values[2]},false),expected);
   assert.equal(execute({size:values[0]},{size:values[2]},true),row[4].slice(1,-1));
  }
 }
});
