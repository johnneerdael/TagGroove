import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';

const load=async name=>{
  const source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const {completeInput}=await load('general-options'),{CellEditor}=await load('cell-editor'),{TagPanel}=await load('tag-panel');
const {removeTagShortcut}=await load('remove-tag');
const main=await readFile(new URL('../src/main.ts',import.meta.url),'utf8'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
function rendererKeyHandler(marker,environment){
  const matches=[];
  const visit=node=>{
    if(ts.isCallExpression(node)&&ts.isPropertyAccessExpression(node.expression)&&node.expression.name.text==='addEventListener'&&node.arguments[0]?.text==='keydown'){
      const handler=node.arguments[1];
      if(handler&&ts.isArrowFunction(handler)&&handler.getText(parsed).includes(marker))matches.push(handler.getText(parsed));
    }
    ts.forEachChild(node,visit);
  };
  visit(parsed);assert.equal(matches.length,1,marker);
  const js=ts.transpileModule(`const handler=${matches[0]};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return new Function(...Object.keys(environment),`${js};return handler;`)(...Object.values(environment));
}
function page(){
  const window=new Window();
  const names=['window','document','HTMLElement','HTMLInputElement','HTMLTextAreaElement'];
  const previous=Object.fromEntries(names.map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]));
  for(const name of names)globalThis[name]=name==='window'?window:window[name];
  return {window,document:window.document,close(){window.happyDOM.abort();for(const name of names){if(previous[name])Object.defineProperty(globalThis,name,previous[name]);else delete globalThis[name];}}};
}
const key=(p,value,isComposing=false,extra={})=>new p.window.KeyboardEvent('keydown',{key:value,isComposing,bubbles:true,cancelable:true,...extra});

test('completion leaves composing input and textarea values and selection untouched',()=>{
  const p=page();
  try{
    for(const tag of ['input','textarea'])for(const inputType of ['insertText','insertCompositionText']){
      const input=p.document.createElement(tag);input.value='東';input.setSelectionRange(1,1);
      const event=new p.window.InputEvent('input',{data:'東',inputType,isComposing:true});
      assert.equal(event.isComposing,true);
      let reads=0;
      assert.equal(completeInput(input,event,()=>{reads++;return ['東京'];}),false,`${tag}: ${inputType}`);
      assert.deepEqual([input.value,input.selectionStart,input.selectionEnd,reads],['東',1,1,0]);
      assert.equal(completeInput(input,new p.window.InputEvent('input',{inputType:'insertText'}),()=>['東京']),true);
      assert.deepEqual([input.value,input.selectionStart,input.selectionEnd],['東京',1,2]);
    }
  }finally{p.close();}
});

test('an explicit composition update is never completed when the composing flag is absent',()=>{
  const p=page();
  try{
    const input=p.document.createElement('input');input.value='東';input.setSelectionRange(1,1);
    const event=new p.window.InputEvent('input',{inputType:'insertCompositionText'});
    assert.equal(completeInput(input,event,()=>['東京']),false);
    assert.equal(input.value,'東');
  }finally{p.close();}
});

test('cell editor keeps IME Enter, Escape and Tab out of commit, cancel and navigation',async()=>{
  for(const value of ['Enter','Escape','Tab']){
    const p=page(),commits=[],moves=[],ended=[];
    try{
      p.document.body.innerHTML='<div class="table-wrap"><table><tbody><tr><td></td></tr></tbody></table></div>';
      const target={path:'/Music/track.flac',column:'title'},cell=p.document.querySelector('td');
      const editor=new CellEditor({cell:()=>cell,initial:()=> '東京',commit:(target,text)=>{commits.push([target,text]);return target.path;},next:(...args)=>{moves.push(args);},ended:()=>ended.push(true),advance:()=>true});
      assert.equal(editor.start(target),true);
      const input=p.document.querySelector('.cell-editor');input.setSelectionRange(2,2);
      const event=key(p,value,true);input.dispatchEvent(event);await Promise.resolve();
      assert.equal(editor.active,true,value);assert.equal(input.isConnected,true,value);
      assert.deepEqual([commits,moves,ended],[[],[],[]],value);assert.equal(event.defaultPrevented,false,value);
      input.dispatchEvent(key(p,'Enter'));await Promise.resolve();
      assert.deepEqual(commits,[[target,'東京']]);assert.equal(editor.active,false);
    }finally{p.close();}
  }
});

function panelPage(){
  const p=page(),calls=[];
  const panel=new TagPanel({persistent:()=>false,values:()=>['東京','京都'],disabled:()=>false,directories:()=>({selected:['/Music'],all:['/Music']}),changeDirectory:path=>calls.push(['directory',path]),draft:(...args)=>calls.push(['draft',...args]),keep:field=>calls.push(['keep',field]),changed:()=>calls.push(['changed']),notify:text=>calls.push(['notice',text]),fail:error=>{throw error;},altShortcut:()=>true,autocomplete:()=>completeInput});
  p.document.body.innerHTML=panel.fieldsHTML();panel.bind(p.document.body);
  return {...p,panel,calls};
}

test('tag-panel directory IME Enter cannot open the partially composed path',()=>{
  const p=panelPage();
  try{
    const input=p.document.querySelector('[data-directory]');input.value='/Music/東京';
    const event=key(p,'Enter',true);input.dispatchEvent(event);
    assert.deepEqual(p.calls,[]);assert.equal(event.defaultPrevented,false);
    input.dispatchEvent(key(p,'Enter'));assert.deepEqual(p.calls,[['directory','/Music/東京']]);
  }finally{p.close();}
});

test('tag-panel composition cannot navigate or accept its value popup',()=>{
  const p=panelPage();
  try{
    const input=p.document.querySelector('[data-field="TITLE"]');input.value='東';
    input.dispatchEvent(key(p,'ArrowDown',true));assert.equal(input.getAttribute('aria-expanded'),'false');
    input.dispatchEvent(key(p,'ArrowDown'));assert.equal(input.getAttribute('aria-expanded'),'true');
    const popup=p.document.querySelector('.combo-list'),active=input.getAttribute('aria-activedescendant');
    for(const value of ['ArrowDown','Enter','Escape','Tab']){
      const event=key(p,value,true);input.dispatchEvent(event);
      assert.equal(event.defaultPrevented,false,value);assert.equal(popup.isConnected,true,value);
      assert.equal(input.getAttribute('aria-activedescendant'),active,value);assert.deepEqual(p.calls,[],value);
    }
    input.dispatchEvent(key(p,'Enter'));assert.equal(popup.isConnected,false);assert.deepEqual(p.calls,[['keep','TITLE'],['changed']]);
  }finally{p.close();}
});

test('global shortcuts cannot save or remove tags while a composition is active',()=>{
  const p=page(),calls=[];
  try{
    const noop=()=>undefined;
    const handler=rendererKeyHandler('windowShortcut(event',{
      isTextEntry:()=>true,macKeyboard:()=>false,macPlatform:false,windowShortcut:noop,altChord:()=>false,
      toolsUI:{shortcut:()=>false},autoSizeShortcut:()=>false,editShortcut:noop,fileShortcut:noop,listShortcut:noop,
      menubar:{close:noop},operate:operation=>calls.push(operation),progressJob:undefined,windowCommand:noop,
    });
    const remove=rendererKeyHandler('removeTagShortcut(event',{
      removeTagShortcut,macPlatform:false,enabled:()=>true,menubar:{close:noop},operate:operation=>calls.push(operation),
    });
    const input=p.document.createElement('input'),errors=[];p.document.body.append(input);
    for(const callback of [handler,remove])p.document.addEventListener('keydown',event=>{try{callback(event);}catch(error){errors.push(error);}});
    for(const value of ['s','r']){
      const event=key(p,value,true,{ctrlKey:true});input.dispatchEvent(event);
      assert.deepEqual(errors,[],value);assert.deepEqual(calls,[],value);assert.equal(event.defaultPrevented,false,value);
    }
    input.dispatchEvent(key(p,'s',false,{ctrlKey:true}));assert.deepEqual(calls,['save']);
    const typedRemove=key(p,'r',false,{ctrlKey:true});input.dispatchEvent(typedRemove);assert.equal(typedRemove.defaultPrevented,true);assert.deepEqual(calls,['save']);
    p.document.body.dispatchEvent(key(p,'r',false,{ctrlKey:true}));assert.deepEqual(calls,['save','remove-tags']);assert.deepEqual(errors,[]);
  }finally{p.close();}
});

function bindFilter(p,calls,autoApply){
  p.document.body.innerHTML='<input id="filter"><div class="table-wrap"></div>';
  const registrations=[];
  const visit=node=>{
    if(ts.isCallExpression(node)&&ts.isPropertyAccessExpression(node.expression)&&node.expression.name.text==='addEventListener'&&node.expression.expression.getText(parsed).includes("'#filter'"))registrations.push(node.getText(parsed));
    ts.forEachChild(node,visit);
  };
  visit(parsed);
  const producer=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='filterInputChanged')?.getText(parsed)??'';
  const js=ts.transpileModule(`let filter='old',filterDraft=undefined,filterAutoApply=initial;${producer};${registrations.join(';')};const state=()=>({filter,filterDraft});`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return new Function('document','refreshList','initial',`${js};return state;`)(p.document,()=>calls.push('refresh'),autoApply);
}

test('manual filter keeps composed text pending until a separate non-composing Enter',()=>{
  const p=page(),calls=[];
  try{
    const state=bindFilter(p,calls,false),input=p.document.querySelector('#filter');input.value='東京';
    input.dispatchEvent(new p.window.InputEvent('input',{inputType:'insertCompositionText',isComposing:true,bubbles:true}));
    input.dispatchEvent(key(p,'Enter',true));assert.deepEqual(state(),{filter:'old',filterDraft:'東京'});assert.deepEqual(calls,[]);
    input.dispatchEvent(new p.window.CompositionEvent('compositionend',{data:'東京',bubbles:true}));
    assert.deepEqual(state(),{filter:'old',filterDraft:'東京'});assert.deepEqual(calls,[]);
    input.dispatchEvent(key(p,'Enter'));assert.deepEqual(state(),{filter:'東京',filterDraft:undefined});assert.deepEqual(calls,['refresh']);
  }finally{p.close();}
});

test('automatic filtering waits for composition to finish before rebuilding the workspace',()=>{
  const p=page(),calls=[];
  try{
    const state=bindFilter(p,calls,true),input=p.document.querySelector('#filter');input.value='東';input.setSelectionRange(1,1);
    input.dispatchEvent(new p.window.InputEvent('input',{inputType:'insertCompositionText',isComposing:true,bubbles:true}));
    assert.deepEqual(state(),{filter:'old',filterDraft:'東'});assert.deepEqual(calls,[]);
    input.value='東京';input.dispatchEvent(new p.window.CompositionEvent('compositionend',{data:'東京',bubbles:true}));
    assert.deepEqual(state(),{filter:'東京',filterDraft:undefined});assert.deepEqual(calls,['refresh']);
    input.dispatchEvent(new p.window.InputEvent('input',{inputType:'insertCompositionText',isComposing:false,bubbles:true}));
    assert.deepEqual(state(),{filter:'東京',filterDraft:undefined});assert.deepEqual(calls,['refresh'],'a late non-composing composition notification keeps the committed state');
    input.dispatchEvent(new p.window.InputEvent('input',{inputType:'insertText',bubbles:true}));assert.deepEqual(calls,['refresh'],'a repeated commit notification does not rebuild again');
    input.value='京都';input.dispatchEvent(new p.window.InputEvent('input',{inputType:'insertText',bubbles:true}));
    assert.deepEqual(state(),{filter:'京都',filterDraft:undefined});assert.deepEqual(calls,['refresh','refresh']);
  }finally{p.close();}
});

test('tag-panel keyboard shortcut leaves composition active and works after it finishes',async()=>{
  const p=panelPage();
  try{
    p.panel.install(p.document);
    const input=p.document.querySelector('[data-field="TITLE"]');
    const event=key(p,'q',true,{ctrlKey:true,code:'KeyQ'});input.dispatchEvent(event);
    assert.equal(p.panel.visible,true);assert.deepEqual(p.calls,[]);assert.equal(event.defaultPrevented,false);
    input.dispatchEvent(key(p,'q',false,{ctrlKey:true,code:'KeyQ'}));await Promise.resolve();
    assert.equal(p.panel.visible,false);assert.equal(p.calls[0][0],'changed');
  }finally{p.close();}
});
