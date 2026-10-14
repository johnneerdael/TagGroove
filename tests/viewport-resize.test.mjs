import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Run the registered production resize handler and table window calculation.
// Keep platform bootstrap and full-workspace rendering at the harness boundary.
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const declarations=new Set(['compactScreen','rowHeight','headerHeight','overscan','resizeFrame']);
const variables=parsed.statements.filter(ts.isVariableStatement).flatMap(statement=>statement.declarationList.declarations)
  .filter(declaration=>declarations.has(declaration.name.getText(parsed)))
  .map(declaration=>`let ${declaration.getText(parsed)};`).join('\n');
const virtualRows=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='updateVirtualRows').getText(parsed);
const resize=parsed.statements.find(node=>ts.isExpressionStatement(node)&&ts.isCallExpression(node.expression)
  &&node.expression.expression.getText(parsed)==='window.addEventListener'&&node.expression.arguments[0]?.text==='resize').getText(parsed);
const factory=`export function createViewport(width=390,height=469) {
  const frames=[],listeners=new Map(),body={innerHTML:''};
  let renders=0,virtualStart=-1,virtualEnd=-1,virtualRevision=-1,virtualSelection='',dataRevision=0;
  const input={value:'Pending Album',selectionStart:4,selectionEnd:8},menu={open:true},dialog={open:true};
  let controls={input,menu,dialog};
  const wrap={clientHeight:height,scrollTop:0,querySelector:selector=>selector==='tbody'?body:null};
  const document={activeElement:input,querySelector:selector=>selector==='.table-wrap'?wrap:null};
  const window={matchMedia:()=>({matches:width<=1000}),addEventListener:(event,handler)=>listeners.set(event,handler)};
  const requestAnimationFrame=handler=>(frames.push(handler),frames.length);
  class HTMLTableRowElement {}
  const list=Array.from({length:100},(_,index)=>({path:'file-'+index}));
  const visible=()=>list,paths=()=>['file-0'],columns=[{}],timing=()=>{};
  const rowMarkup=(entry,index)=>'<tr data-path="'+entry.path+'" aria-rowindex="'+(index+2)+'"></tr>';
  function render(){
    renders++;
    controls={input:{...controls.input},menu:{open:false},dialog:controls.dialog};
    document.activeElement=controls.input;
    updateVirtualRows(true);
  }
  ${variables}
  ${virtualRows}
  ${resize}
  updateVirtualRows(true);
  return {
    resize(nextWidth,nextHeight){width=nextWidth;wrap.clientHeight=nextHeight;listeners.get('resize')();},
    flush(){const pending=frames.splice(0);pending.forEach(callback=>callback());},
    scroll(top){wrap.scrollTop=top;},
    snapshot:()=>({renders,controls,focused:document.activeElement,html:body.innerHTML,start:virtualStart,end:virtualEnd,pendingFrames:frames.length}),
  };
}`;
const javascript=ts.transpileModule(factory,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {createViewport}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

test('IME height changes retain editor controls and open surfaces while updating the table window',()=>{
  const viewport=createViewport(),before=viewport.snapshot();
  viewport.resize(390,249);viewport.flush();
  const reduced=viewport.snapshot();
  assert.equal(reduced.renders,0);
  assert.equal(reduced.focused,before.focused);
  assert.equal(reduced.controls.input,before.controls.input);
  assert.deepEqual(reduced.controls.input,{value:'Pending Album',selectionStart:4,selectionEnd:8});
  assert.equal(reduced.controls.menu,before.controls.menu);assert.equal(reduced.controls.menu.open,true);
  assert.equal(reduced.controls.dialog,before.controls.dialog);assert.equal(reduced.controls.dialog.open,true);
  assert.equal(reduced.end,13);assert.match(reduced.html,/height:3828px/);
  viewport.resize(390,689);viewport.flush();
  assert.equal(viewport.snapshot().renders,0);assert.equal(viewport.snapshot().end,23);
});

test('same-mode width changes and repeated resize events use the final table geometry once',()=>{
  const viewport=createViewport(1280,329),before=viewport.snapshot();
  viewport.scroll(300);viewport.resize(1100,389);viewport.resize(1001,629);
  assert.equal(viewport.snapshot().pendingFrames,1);
  viewport.flush();const after=viewport.snapshot();
  assert.equal(after.renders,0);assert.equal(after.controls,before.controls);
  assert.equal(after.start,2);assert.equal(after.end,38);
  assert.match(after.html,/height:60px/);assert.match(after.html,/data-path="file-37"/);
});

test('crossing the compact breakpoint refreshes the workspace and row geometry in both directions',()=>{
  const viewport=createViewport(1001,469);
  viewport.resize(1000,469);viewport.flush();
  assert.equal(viewport.snapshot().renders,1);assert.equal(viewport.snapshot().end,18);
  assert.match(viewport.snapshot().html,/height:3608px/);
  viewport.resize(1001,469);viewport.flush();
  assert.equal(viewport.snapshot().renders,2);assert.equal(viewport.snapshot().end,23);
  assert.match(viewport.snapshot().html,/height:2310px/);
});

test('a breakpoint excursion completed before the queued frame preserves the current workspace',()=>{
  const viewport=createViewport(1000,469);
  viewport.resize(1001,469);viewport.resize(1000,689);viewport.flush();
  assert.equal(viewport.snapshot().renders,0);assert.equal(viewport.snapshot().end,23);
});
