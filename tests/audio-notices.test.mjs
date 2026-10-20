import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';

const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const ast=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const names=['operateImpl','modalError','publicMessage'];
const declarations=names.map(name=>{
  const node=ast.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name);
  assert.ok(node,`Production function ${name} is required`);
  return node.getText(ast);
}).join('\n');
const js=ts.transpileModule(declarations,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
const stateSource=await readFile(new URL('../src/document-state.ts',import.meta.url),'utf8');
const stateJS=ts.transpileModule(stateSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {publicDocumentMessage}=await import(`data:text/javascript;base64,${Buffer.from(stateJS).toString('base64')}`);
const moduleURL=async(file,replacements={})=>{
  let code=ts.transpileModule(await readFile(new URL(`../src/${file}`,import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  for(const [from,to] of Object.entries(replacements))code=code.replace(from,JSON.stringify(to));
  return `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`;
};
const coreURL=await moduleURL('i18n/core.ts');
const converterURL=await moduleURL('converter-errors.ts',{"'./i18n/core'":coreURL});
const {showFileError}=await import(await moduleURL('batch-errors.ts',{"'./i18n/core'":coreURL,"'./converter-errors'":converterURL}));

function aboutHost(failure){
  const button=new EventTarget(),content=[],calls=[],versions={textContent:''},close={textContent:''};
  const dialog={querySelector(selector){
    if(selector==='#audio-source-release')return button;
    if(selector==='#audio-backend-versions')return versions;
    if(selector==='[data-close]')return close;
    if(selector==='.dialog-content')return {append:element=>content.push(element)};
    if(selector==='.dialog-error')return content.find(element=>element.className==='dialog-error')||null;
    throw new Error(`Unexpected selector ${selector}`);
  }};
  const invoke=(command,args)=>{
    calls.push({command,args});
    if(command==='audio_fingerprint_backend')return Promise.resolve({chromaprint:'1.6.2',ffmpeg:'9.0.2'});
    assert.equal(command,'open_help_page');assert.deepEqual(args,{page:'releases'});
    if(!failure)return Promise.resolve();
    const rejection=Promise.reject(failure);
    // Observe the host rejection so the RED case reports the missing visible
    // fallback rather than a test-runner unhandled-rejection interruption.
    void rejection.catch(()=>{});
    return rejection;
  };
  const document={createElement:()=>({textContent:'',className:'',attributes:{},setAttribute(name,value){this.attributes[name]=value;},removeAttribute(name){delete this.attributes[name];}})};
  dialog.ownerDocument=document;
  const factory=new Function('enabled','audioLicenses','esc','modal','themedAsset','appVersion','invoke','document','publicDocumentMessage','documentAccess','showFileError','t','translate','onLanguageChange',`${js}\nreturn operateImpl;`);
  const operate=factory(()=>true,[],String,(_title,markup)=>{assert.ok(markup.includes('id="audio-source-release"'));return dialog;},()=>'', '0.2.0',invoke,document,publicDocumentMessage,{states:{}},showFileError,text=>text,text=>text,()=>()=>{});
  return {operate,button,dialog,calls};
}

for(const platform of ['Android','iOS'])test(`About source link displays the ${platform} browser fallback`,async()=>{
  const fallback='Open https://github.com/johnneerdael/TagGroove/releases in a browser';
  const host=aboutHost(fallback);await host.operate('about');
  host.button.dispatchEvent(new Event('click'));await new Promise(resolve=>setImmediate(resolve));
  const error=host.dialog.querySelector('.dialog-error');
  assert.ok(error,'The rejected native browser request must be visible in the notice');
  assert.equal(error.attributes.role,'alert');assert.equal(error.textContent,fallback);
});

test('About source link invokes the release page without an error on desktop',async()=>{
  const host=aboutHost();await host.operate('about');
  host.button.dispatchEvent(new Event('click'));await new Promise(resolve=>setImmediate(resolve));
  assert.equal(host.calls.filter(call=>call.command==='open_help_page').length,1);
  assert.equal(host.dialog.querySelector('.dialog-error'),null);
});

test('About translates its controls while retaining literal upstream notice text',async()=>{
  const {Translator}=await import(coreURL);
  const {DomTranslator}=await import(await moduleURL('i18n/dom.ts',{"'./core'":coreURL}));
  const window=new Window(),saved={};
  for(const name of ['window','document','Node','Element','Text','MutationObserver','NodeFilter','HTMLInputElement','HTMLTextAreaElement']){
    saved[name]=Object.getOwnPropertyDescriptor(globalThis,name);
    Object.defineProperty(globalThis,name,{configurable:true,writable:true,value:name==='window'?window:window[name]});
  }
  let watcher;
  try{
    let shown;
    const translator=new Translator('nl',{'Close':'Sluiten','Dependency licenses and notices':'Licenties en vermeldingen van afhankelijkheden'});
    const modal=(_title,markup)=>{
      shown=document.createElement('dialog');shown.innerHTML=markup+'<button data-close>Close</button>';document.body.append(shown);return shown;
    };
    const esc=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    const factory=new Function('enabled','audioLicenses','esc','modal','themedAsset','appVersion','invoke','document','publicDocumentMessage','documentAccess','showFileError','t','translate','onLanguageChange',`${js}\nreturn operateImpl;`);
    const operate=factory(()=>true,[{title:'Dependency licenses and notices',text:'Close'}],esc,modal,()=>'', '0.2.0',async()=>({chromaprint:'1.6.2',ffmpeg:'9.0.2'}),document,publicDocumentMessage,{states:{}},showFileError,text=>text,text=>text,()=>()=>{});
    await operate('about');
    watcher=new DomTranslator(document.body,()=>translator);watcher.translateTree(shown);
    assert.equal(shown.querySelector('[data-close]').textContent,'Sluiten');
    assert.equal(shown.querySelector('summary').textContent,'Licenties en vermeldingen van afhankelijkheden');
    assert.equal(shown.querySelector('pre').textContent,'Close','upstream notice bytes must not be treated as owned UI labels');
  }finally{
    await window.happyDOM.abort();
    for(const[name,value]of Object.entries(saved))if(value)Object.defineProperty(globalThis,name,value);else delete globalThis[name];
  }
});
