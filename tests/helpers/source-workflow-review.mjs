// Filesystem transport for the production review. No browser or HTTP server.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
import http from 'node:http';
import https from 'node:https';
import net from 'node:net';

try {
  const [inputPath,outputPath]=process.argv.slice(2);
  assert.ok(inputPath&&outputPath,'expected input and output JSON paths');
  const input=JSON.parse(await readFile(inputPath,'utf8'));
  assert.equal(input.version,1,'unsupported workflow protocol version');
  assert.ok(['multi','scalar'].includes(input.scenario),'unknown workflow scenario');
  assert.ok(Array.isArray(input.files)&&input.files.length>=1,'missing file summaries');
  const sourceId=input.source?.id??'Replay.src',sourceName=input.source?.name??'Independent replay';
  assert.ok(typeof sourceId==='string'&&sourceId.length,'missing source identity');
  const checkedFields=input.checked_fields??['TITLE','ALBUM','COMMENT'];
  assert.ok(Array.isArray(checkedFields)&&checkedFields.every(field=>typeof field==='string'),'invalid checked fields');
  assert.equal(input.release.provider,'web','expected actual web release');
  assert.ok(Array.isArray(input.search.results)&&input.search.results.length,'missing parsed search results');
  assert.equal(input.search.results[0].url,input.release.url,'release must come from chosen result');
  const requests=[];
  const deny=(...args)=>{requests.push(String(args[0]));throw Error('Workflow replay attempted network I/O');};
  for(const client of [http,https]){client.request=deny;client.get=deny;}
  net.connect=deny;net.createConnection=deny;net.Socket.prototype.connect=deny;globalThis.fetch=deny;
  const window=new Window({url:'https://workflow.invalid/'});
  window.fetch=deny;window.XMLHttpRequest.prototype.open=deny;
  for(const key of ['window','document','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','MutationObserver','MouseEvent','Event'])globalThis[key]=key==='window'?window:window[key];
  const source=await readFile(new URL('../../src/tag-sources.ts',import.meta.url),'utf8');
  const javascript=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  const {TagSourcesUI}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
  const calls=[],applied=[];
  const ui=new TagSourcesUI({files:()=>input.files,reason:()=>'',options:()=>({cover_filename:'folder'}),pickSave:async()=>{throw Error('Unexpected picker');},evaluate:async()=> 'Fixture Artist',
    invoke:async(command,args)=>{
      calls.push({command,args});
      if(command==='web_source_search'){assert.equal(args.source,sourceId);return structuredClone(input.search);}
      if(command==='web_source_release'){assert.equal(args.source,sourceId);assert.equal(args.url,input.release.url);return structuredClone(input.release);}
      if(command==='tag_source_image'){assert.ok(input.release.covers.some(c=>c.url===args.url||c.thumb===args.url),'image outside parsed release');return `data:image/png;base64,${input.png}`;}
      if(command==='tag_source_cancel')return;
      throw Error(`Unexpected workflow command: ${command}`);
    },apply:async(items,cover)=>{applied.push(structuredClone({items,cover}));}});
  try {
    await ui.open('web',{web:{id:sourceId,name:sourceName,search_by:[]}});
    const dialog=window.document.querySelector('dialog');
    assert.ok(dialog,'production source dialog did not open');
    dialog.querySelector('#ts-search').click();
    await window.happyDOM.waitUntilComplete();
    if(!input.search.direct){assert.ok(dialog.querySelector('#ts-next'),'missing result selection');dialog.querySelector('#ts-next').click();await window.happyDOM.waitUntilComplete();}
    assert.ok(dialog.querySelector('#ts-apply'),'production review did not open');
    for(const box of dialog.querySelectorAll('input[data-field]')){
      const checked=checkedFields.includes(box.dataset.field);
      if(box.checked!==checked){box.checked=checked;box.dispatchEvent(new window.Event('change',{bubbles:true}));}
    }
    const save=dialog.querySelector('#ts-cover-save');assert.ok(save,'missing artwork selection');
    save.checked=true;save.dispatchEvent(new window.Event('change',{bubbles:true}));
    assert.equal(dialog.querySelector('#ts-cover-write').checked,true);
    assert.equal(dialog.querySelector('#ts-cover-replace').checked,true);
    assert.equal(dialog.querySelector('#ts-apply').disabled,false);
    dialog.querySelector('#ts-apply').click();await window.happyDOM.waitUntilComplete();
    assert.equal(applied.length,1,'review must emit exactly one import');
    assert.equal(requests.length,0,'replay must make zero network requests');
    await writeFile(outputPath,JSON.stringify({version:1,scenario:input.scenario,...applied[0],calls,network_requests:requests.length}));
  }finally{await window.happyDOM.abort();}
}catch(error){console.error(`Source workflow review protocol failed: ${error.stack||error}`);process.exitCode=1;}
