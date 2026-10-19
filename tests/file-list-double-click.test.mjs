import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';

const {ToolsUI}=await loadProductionModule('tools.ts');
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const file=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true);
const registrations=[];
function visit(node){
  if(ts.isCallExpression(node)&&ts.isPropertyAccessExpression(node.expression)&&
    node.expression.expression.getText(file)==='wrap'&&node.expression.name.text==='addEventListener'&&
    ts.isStringLiteral(node.arguments[0])&&node.arguments[0].text==='dblclick')registrations.push(node.getText(file));
  ts.forEachChild(node,visit);
}
visit(file);
assert.equal(registrations.length,1,'bind the actual file-list double-click registration');
const javascript=ts.transpileModule(`export function bind(wrap,rowFor,toolsUI,operate,window,slowEditTimer){${registrations[0]};}`,{
  compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022},
}).outputText;
const {bind}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

function fixture(values){
  const window=new Window(),calls=[],operations=[],reports=[];
  window.document.body.innerHTML='<div class="table-wrap"><table><thead><tr><th>Title</th></tr></thead><tbody><tr data-path="/one.flac"><td><span>One</span></td></tr></tbody></table></div>';
  const wrap=window.document.querySelector('.table-wrap');
  const tools=new ToolsUI({values:()=>values,mac:false,selection:()=>['/one.flac','/two.flac'],reason:()=>'',
    run:async(tool,paths)=>{calls.push({name:tool.name,paths});return{started:1};},
    confirm:async()=>true,report:message=>reports.push(message),fail:error=>assert.fail(String(error))});
  let pendingEdit=false;
  const timer=window.setTimeout(()=>{pendingEdit=true;},5);
  bind(wrap,event=>event.target.closest('tr[data-path]'),tools,operation=>operations.push(operation),window,timer);
  return {window,calls,operations,reports,get pendingEdit(){return pendingEdit;},
    doubleClick(selector='tbody span'){window.document.querySelector(selector).dispatchEvent(new window.MouseEvent('dblclick',{bubbles:true}));},
    close:()=>window.happyDOM.abort()};
}

const definition={name:'Inspect audio',path:'/never-launched',parameters:'',for_each_file:false};
test('No action and unavailable default tools perform no file-list double-click action',async()=>{
  for(const values of [{},{tools:[definition],default_tool:-1},{tools:[definition],default_tool:3}]){
    const page=fixture(values);
    try{page.doubleClick();await Promise.resolve();assert.deepEqual(page.calls,[]);assert.deepEqual(page.operations,[],'No action must not open Extended Tags or save tags');}
    finally{await page.close();}
  }
});
test('a configured default tool receives the current selection once without a fallback command',async()=>{
  const page=fixture({tools:[definition],default_tool:0});
  try{
    page.doubleClick();await Promise.resolve();
    assert.deepEqual(page.calls,[{name:'Inspect audio',paths:['/one.flac','/two.flac']}]);
    assert.deepEqual(page.operations,[]);assert.deepEqual(page.reports,['Started Inspect audio']);
  }finally{await page.close();}
});
test('header double-clicks do not launch a row tool and No action cancels delayed cell editing',async()=>{
  const header=fixture({tools:[definition],default_tool:0});
  try{header.doubleClick('th');await Promise.resolve();assert.deepEqual(header.calls,[]);assert.deepEqual(header.operations,[]);}
  finally{await header.close();}
  const row=fixture({tools:[definition],default_tool:-1});
  try{row.doubleClick();await row.window.happyDOM.waitUntilComplete();assert.equal(row.pendingEdit,false,'double-click must cancel the pending slow cell edit');}
  finally{await row.close();}
});
