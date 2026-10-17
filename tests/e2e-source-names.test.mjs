import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {Window} from 'happy-dom';
import ts from 'typescript';

for(const suite of ['freedb-local','gnudb','sources-3','sources-4','keyboard-ui','web-sources']){
  test(`${suite} checks the current accessible source name and preserves authored text`,async()=>{
    const source=await readFile(new URL(`e2e/${suite}.js`,import.meta.url),'utf8');
    const ast=ts.createSourceFile(`${suite}.js`,source,ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
    let declaration;
    const visit=node=>{if(ts.isVariableDeclaration(node)&&node.name.getText(ast)==='sourceName')declaration=node;ts.forEachChild(node,visit);};visit(ast);
    assert.ok(declaration,'the native suite must resolve its actual dialog label');
    const sourceName=new Function(`return (${declaration.initializer.getText(ast)});`)();
    const window=new Window();
    try{
      const dialog=window.document.createElement('dialog');window.document.body.append(dialog);
      dialog.setAttribute('aria-label','freedb (local)');assert.equal(sourceName(dialog),'freedb (local)');
      dialog.innerHTML='<header><span id="source-name">freedb (lokal)</span>: <span>Unrelated album</span></header>';
      dialog.setAttribute('aria-labelledby','source-name');
      assert.equal(sourceName(dialog),'freedb (lokal)','the live labelled-by name takes priority over a stale aria-label');
      dialog.querySelector('#source-name').textContent='Fixture › MiXeD source';
      assert.equal(sourceName(dialog),'Fixture › MiXeD source');
      dialog.querySelector('#source-name').remove();assert.equal(sourceName(dialog),undefined,'a broken accessible-name binding must fail');
    }finally{window.happyDOM.abort();window.close();}
  });
}
