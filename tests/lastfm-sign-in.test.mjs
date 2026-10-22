import test from 'node:test';
import assert from 'node:assert/strict';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';
const {TagSourcesUI}=await loadProductionModule('tag-sources.ts');
const url='https://www.last.fm/api/auth/?api_key=app&token=tok';
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
async function run(openPage,check){
  const window=new Window({url:'http://localhost:1420'}),calls=[];
  const saved=Object.fromEntries(['window','document'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
  globalThis.window=window;globalThis.document=window.document;
  const host={files:()=>[],reason:()=>'',options:()=>({}),evaluate:async()=>'',apply:async()=>{throw Error('no apply');},pickSave:async()=>null,
    invoke:async(command,args)=>{
      calls.push([command,args]);
      if(command==='load_tag_source_settings')return{lastfm_available:true,lastfm_can_sign_in:true,lastfm_user:''};
      if(command==='lastfm_sign_in_start')return{token:'tok',url};
      if(command==='tag_source_open_page')return openPage();
      if(command==='lastfm_sign_in_finish')return{lastfm_available:true,lastfm_can_sign_in:true,lastfm_user:'someone'};
      throw Error('Unexpected command '+command);
    }};
  try{await check({ui:new TagSourcesUI(host),calls,q:selector=>document.querySelector(selector)});}
  finally{for(const [key,descriptor] of Object.entries(saved)){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}window.happyDOM.abort();}
}
async function signIn({ui,q}){
  await ui.keys();
  q('#ts-lastfm-start').click();
  for(let i=0;i<6;i++)await tick();
}

test('Last.fm sign-in opens the browser, then Finish keeps the session',async()=>{
  await run(async()=>undefined,async env=>{
    const {q,calls}=env;
    await signIn(env);
    assert.deepEqual(calls.find(([command])=>command==='tag_source_open_page')[1],{url});
    assert.match(q('#ts-lastfm-steps').textContent,/page that opened in your browser/);
    assert.equal(q('#ts-lastfm-url').hidden,true);
    assert.equal(q('#ts-lastfm-finish').hidden,false);assert.equal(q('#ts-lastfm-start').hidden,true);
    assert.equal(q('.dialog-error'),null);
    q('#ts-lastfm-finish').click();
    for(let i=0;i<6;i++)await tick();
    assert.deepEqual(calls.find(([command])=>command==='lastfm_sign_in_finish')[1],{token:'tok'});
    assert.match(q('#ts-lastfm-state').textContent,/Signed in as someone/);
    assert.equal(q('#ts-lastfm-out').hidden,false);
  });
});

test('where no browser can be opened the approval address is shown and sign-in still completes',async()=>{
  await run(async()=>{throw 'Open https://www.last.fm/api/auth/ in a browser';},async env=>{
    const {q,calls}=env;
    await signIn(env);
    assert.match(q('#ts-lastfm-steps').textContent,/could not open a browser/);
    assert.equal(q('#ts-lastfm-url').hidden,false);assert.equal(q('#ts-lastfm-url').value,url);
    assert.equal(q('#ts-lastfm-finish').hidden,false);
    assert.equal(q('.dialog-error'),null,'a missing browser is not an error');
    q('#ts-lastfm-finish').click();
    for(let i=0;i<6;i++)await tick();
    assert.match(q('#ts-lastfm-state').textContent,/Signed in as someone/);
    assert.equal(q('#ts-lastfm-url').hidden,true);
    assert.ok(calls.some(([command])=>command==='lastfm_sign_in_finish'));
  });
});
