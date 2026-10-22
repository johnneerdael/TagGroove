// Filesystem transport for the production review. No browser or HTTP server.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {closeSync} from 'node:fs';
import {loadProductionModule} from './load-production-module.mjs';
import {Window} from 'happy-dom';
import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import {createInterface} from 'node:readline';
import {setTimeout as pause} from 'node:timers/promises';

let bridge;
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
  const rpc=input.transport==='public-acquisition-v1';
  if(!rpc){
  assert.equal(input.release.provider,'web','expected actual web release');
  assert.ok(Array.isArray(input.search.results)&&input.search.results.length,'missing parsed search results');
  assert.equal(input.search.results[0].url,input.release.url,'release must come from chosen result');
  }
  let sequence=0;
  const pending=new Map();
  const invokeBackend=rpc?((command,args)=>new Promise((resolve,reject)=>{
    assert.ok(++sequence<=24,'bridge command bound exceeded');
    const line=JSON.stringify({version:1,id:sequence,command,args});
    assert.ok(Buffer.byteLength(line)<=1024*1024,'bridge request too large');
    pending.set(sequence,{resolve,reject});process.stdout.write(line+'\n');
  })):null;
  if(rpc){
    bridge=createInterface({input:process.stdin,crlfDelay:Infinity});
    bridge.on('line',line=>{
      try{
        assert.ok(Buffer.byteLength(line)<=1024*1024,'bridge response too large');
        const response=JSON.parse(line);
        assert.equal(response.version,1);assert.ok(pending.has(response.id),'unexpected bridge response');
        const waiter=pending.get(response.id);pending.delete(response.id);
        if(response.error)waiter.reject(Error(response.error));else waiter.resolve(response.result);
      }catch(error){for(const waiter of pending.values())waiter.reject(error);pending.clear();process.exitCode=1;bridge.close();}
    });
  }
  if(rpc&&input.bridge_control!=null){
    assert.ok(['large-stderr','stall','closed-protocol'].includes(input.bridge_control),'unknown bridge control');
    if(input.bridge_control==='large-stderr')await new Promise((resolve,reject)=>process.stderr.write('x'.repeat(256*1024),error=>error?reject(error):resolve()));
    if(input.bridge_control==='closed-protocol'){closeSync(process.stdout.fd);await pause(60_000);}
    if(input.bridge_control==='stall')await pause(60_000);
  }
  const waitFor=async(predicate)=>{for(let i=0;i<1500;i++){if(predicate())return;await pause(10);}throw Error('workflow bridge timed out');};
  const requests=[];
  const deny=(...args)=>{requests.push(String(args[0]));throw Error('Workflow replay attempted network I/O');};
  for(const client of [http,https]){client.request=deny;client.get=deny;}
  net.connect=deny;net.createConnection=deny;net.Socket.prototype.connect=deny;globalThis.fetch=deny;
  const window=new Window({url:'https://workflow.invalid/'});
  window.fetch=deny;window.XMLHttpRequest.prototype.open=deny;
  for(const key of ['window','document','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','MutationObserver','MouseEvent','Event'])globalThis[key]=key==='window'?window:window[key];
  const {TagSourcesUI}=await loadProductionModule('tag-sources.ts');
  const calls=[],applied=[];
  let selectedResultIndex=null;
  const ui=new TagSourcesUI({files:()=>input.files,reason:()=>'',options:()=>({cover_filename:'folder'}),pickSave:async()=>{throw Error('Unexpected picker');},evaluate:async()=> 'Fixture Artist',
    invoke:async(command,args)=>{
      calls.push({command,args});
      if(rpc)return invokeBackend(command,args);
      if(command==='web_source_search'){assert.equal(args.source,sourceId);return structuredClone(input.search);}
      if(command==='web_source_release'){assert.equal(args.source,sourceId);assert.equal(args.url,input.release.url);return structuredClone(input.release);}
      if(command==='tag_source_image'){assert.ok(input.release.covers.some(c=>c.url===args.url||c.thumb===args.url),'image outside parsed release');return `data:image/png;base64,${input.png}`;}
      if(command==='tag_source_cancel')return;
      throw Error(`Unexpected workflow command: ${command}`);
    },apply:async(items,cover)=>{if(rpc)await invokeBackend('tag_source_apply',{items,cover,columns:null});applied.push(structuredClone({items,cover}));}});
  try {
    await ui.open('web',{web:{id:sourceId,name:sourceName,search_by:rpc?input.source.search_by:[]}});
    const dialog=window.document.querySelector('dialog');
    assert.ok(dialog,'production source dialog did not open');
    if(rpc){assert.equal(input.values.length,input.source.search_by.length);input.values.forEach((value,i)=>{dialog.querySelector(`#ts-web-${i}`).value=value;});}
    dialog.querySelector('#ts-search').click();
    await window.happyDOM.waitUntilComplete();
    if(rpc)await waitFor(()=>dialog.querySelector('#ts-apply')||(dialog.querySelector('#ts-next')&&!dialog.querySelector('#ts-next').disabled));
    if(!(rpc?input.direct:input.search.direct)){
      assert.ok(dialog.querySelector('#ts-next'),'missing result selection');
      if(rpc&&input.expected_search!=null){
        const expected=input.expected_search;
        assert.ok(expected&&Array.isArray(expected.columns)&&Array.isArray(expected.results),'missing independent search oracle');
        assert.deepEqual([...dialog.querySelectorAll('.ts-results thead th')].map(cell=>cell.textContent),expected.columns,'rendered search columns');
        const rows=[...dialog.querySelectorAll('.ts-results tbody tr')];
        assert.deepEqual(rows.map(row=>[...row.querySelectorAll('td')].map(cell=>cell.textContent)),expected.results.map(row=>row.values),'rendered search rows');
        const selected=expected.results.flatMap((row,index)=>row.url===input.selected_result_url?[index]:[]);
        assert.equal(selected.length,1,'selected release must uniquely identify a parsed result');
        selectedResultIndex=selected[0];rows[selectedResultIndex].click();
        assert.deepEqual(rows.map(row=>row.getAttribute('aria-selected')),rows.map((_,index)=>String(index===selectedResultIndex)),'production selected search row');
      }else assert.ok(input.selected_result_url==null,'selected release requires an independent search oracle');
      dialog.querySelector('#ts-next').click();await window.happyDOM.waitUntilComplete();
    }
    if(rpc)await waitFor(()=>dialog.querySelector('#ts-apply'));
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
    if(rpc)await waitFor(()=>pending.size===0);
    dialog.querySelector('#ts-apply').click();await window.happyDOM.waitUntilComplete();
    if(rpc)await waitFor(()=>applied.length===1);
    assert.equal(applied.length,1,'review must emit exactly one import');
    assert.equal(requests.length,0,'replay must make zero network requests');
    await writeFile(outputPath,JSON.stringify({version:1,scenario:input.scenario,...applied[0],calls,network_requests:requests.length,selected_result_index:selectedResultIndex}));
  }finally{bridge?.close();await window.happyDOM.abort();}
}catch(error){bridge?.close();console.error(`Source workflow review protocol failed: ${error.stack||error}`);process.exitCode=1;}
