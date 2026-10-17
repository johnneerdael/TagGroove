import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {Window} from 'happy-dom';

const script=await readFile(new URL('e2e/native-fingerprints.js',import.meta.url),'utf8');

async function run({stuck=false,badBackend=false,badLicense=false,badSettings=false}={}){
  const window=new Window(),document=window.document,reports=[],clicks=[];
  let clock=0,closedAt,about,exited,removed=false,replaced=false;
  document.body.innerHTML='<div id="app"><button data-op="about">About</button><button data-op="source-options" disabled>Options</button><table><tbody></tbody></table><footer class="status busy"><span class="status-message">Reading audio files…</span></footer></div>';
  const app=document.querySelector('#app');app.inert=true;
  const sourceButton=enabled=>{
    const old=document.querySelector('[data-op="source-options"]'),button=document.createElement('button');
    button.dataset.op='source-options';button.disabled=!enabled;old.replaceWith(button);
    button.addEventListener('click',()=>{
      clicks.push({kind:'options',connected:button.isConnected,aboutConnected:about?.isConnected,busy:!!document.querySelector('footer.busy')});
      const options=document.createElement('dialog');options.className='options-dialog';options.setAttribute('open','');
      options.innerHTML='<button data-page="tag-sources-online">Online</button><div class="page"></div><button id="options-cancel">Cancel</button>';
      options.querySelector('[data-page]').addEventListener('click',()=>{
        options.querySelector('.page').innerHTML='<input data-option="pages.tag-sources-online.fpcalc" placeholder="Leave empty for built-in fingerprinting"><p>no extra program is needed</p>';
        if(badSettings)options.querySelector('input').value='unexpected external command';
      });
      options.querySelector('#options-cancel').addEventListener('click',()=>options.remove());document.body.append(options);
    });
  };
  sourceButton(false);
  document.querySelector('[data-op="about"]').addEventListener('click',()=>{
    clicks.push({kind:'about',inert:app.inert,busy:!!document.querySelector('footer.busy'),rows:document.querySelectorAll('tr[data-path]').length});
    about=document.createElement('dialog');about.setAttribute('open','');
    about.innerHTML='<header>Built-in audio fingerprinting</header><p>LGPL 2.1-or-later; modifying and relinking</p><button id="audio-source-release">Sources</button><details><summary>FFmpeg 9.0.2 — LGPL 2.1 license</summary><pre>GNU LESSER GENERAL PUBLIC LICENSE</pre></details><button data-close>Close</button>';
    if(badLicense)about.querySelector('pre').textContent='missing license';
    about.querySelector('[data-close]').addEventListener('click',()=>{
      about.removeAttribute('open');closedAt=clock;
      // Closing removes the dialog asynchronously. A redraw also replaces the
      // command while native startup work briefly keeps it unavailable.
      document.querySelector('[data-op="source-options"]').disabled=true;
    });document.body.append(about);
  });
  const tick=()=>{
    clock+=100;
    if(!stuck){
      if(clock>=100)app.inert=false;
      if(clock>=200&&!document.querySelector('tr[data-path]'))document.querySelector('tbody').innerHTML='<tr data-path="C:/fixture/sine-a.mp3"></tr><tr data-path="C:/fixture/sine-b.mp3"></tr>';
      if(clock>=300){document.querySelector('footer').classList.remove('busy');document.querySelector('.status-message').textContent='Loaded: 2 files';if(closedAt===undefined)sourceButton(true);}
    }
    if(closedAt!==undefined&&clock>=closedAt+100&&!removed){about.remove();removed=true;sourceButton(false);}
    if(closedAt!==undefined&&clock>=closedAt+200&&!replaced){sourceButton(true);replaced=true;}
  };
  window.OPENTAG_E2E_OFFLINE=true;
  window.__TAURI_INTERNALS__={invoke:async(command,args)=>{
    if(command==='audio_fingerprint_backend')return{chromaprint:badBackend?'wrong':'1.6.2',ffmpeg:'9.0.2',audio_decoders:218,non_audio_decoders:0,protocols:0};
    if(command==='e2e_report'){reports.push(args.message);return;}
    if(command==='e2e_exit'){exited=args.code;return;}
    throw new Error('Unexpected command: '+command);
  }};
  class Clock extends Date{static now(){return clock;}}
  try{
    // Execute the complete checked-in native smoke script; only DOM/native IPC
    // and time are controlled, including the disabled-button click behavior.
    await new Function('window','document','Date','setTimeout','return '+script.slice(script.indexOf('(async')))(window,document,Clock,resolve=>{tick();queueMicrotask(resolve);});
    return{reports,clicks,exited};
  }finally{await window.happyDOM.abort();window.close();}
}

test('actual native smoke waits for startup, dialog removal and a fresh enabled command',async()=>{
  const result=await run();
  assert.equal(result.exited,0,result.reports.join('\n'));
  assert.ok(result.reports.includes('SUMMARY native-fingerprints 9 passed, 0 failed'));
  assert.deepEqual(result.clicks,[{kind:'about',inert:false,busy:false,rows:2},{kind:'options',connected:true,aboutConnected:false,busy:false}]);
});

test('startup timeout reports the blocked UI state instead of clicking through it',async()=>{
  const result=await run({stuck:true});
  assert.equal(result.exited,1);
  assert.deepEqual(result.clicks,[]);
  const failure=result.reports.find(line=>line.startsWith('FAIL'));
  for(const state of ['inert=true','busy=true','source-options=disabled','rows=0','Reading audio files'])assert.ok(failure.includes(state),failure);
});

for(const [mismatch,message] of [['badBackend','actual linked backend versions'],['badLicense','complete LGPL text bundled in About'],['badSettings','default needs no executable path']]){
  test('actual native smoke still rejects '+mismatch,async()=>{
    const result=await run({[mismatch]:true});
    assert.equal(result.exited,1);
    assert.ok(result.reports.includes('FAIL '+message),result.reports.join('\n'));
  });
}
