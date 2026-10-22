import test from 'node:test';
import assert from 'node:assert/strict';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';
const {TagSourcesUI,searchModes,sourceRequest,catalogReleaseInput,trackSearchPrefill,tagSourcesOptionsPages}=await loadProductionModule('tag-sources.ts');
const file={path:'/music/test.flac',name:'test.flac',title:'After Hours (Extended Mix)',artist:'Guest',album_artist:'Various Artists',album:'Night',track:'7',disc:'1',length:300,fields:{TITLE:['After Hours (Extended Mix)'],ARTIST:['Guest'],ALBUM:['Night'],COMMENT:['Keep me']}};
const track=(number,title)=>({number,disc:1,position:String(number),title,artist:'Guest',length:300,fields:{TITLE:[title],ARTIST:['Guest'],TRACK:[String(number)],BEATPORT_TRACK_ID:[number===7?'12':'10']}});
const release=(provider='beatport')=>({provider,id:'track:12',title:'Night',artist:'Guest',url:'',fields:{ALBUM:['Night revised'],BEATPORT_RELEASE_ID:['44']},tracks:[track(7,'After Hours (Extended Mix)')],covers:[],more_covers:false});
function setup(invoke=async()=>{}){const w=new Window({url:'https://test.invalid'});for(const key of ['window','document','Node','Element','HTMLElement','HTMLInputElement','HTMLButtonElement','Event'])globalThis[key]=key==='window'?w:w[key];let files=[structuredClone(file)];const calls=[],applied=[];const ui=new TagSourcesUI({files:()=>files,reason:()=>'',options:()=>({}),pickSave:async()=>null,evaluate:async()=>'',invoke:async(command,args)=>{calls.push({command,args});return invoke(command,args);},apply:async(items,cover)=>applied.push({items,cover})});return{w,ui,calls,applied,setFiles:value=>files=value};}
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
test('comparison entry point accepts an album and releases its single-dialog guard on close',async()=>{
 const s=setup(async command=>command==='tag_source_comparison_snapshot'?'snapshot':command==='load_tag_source_settings'?{discogs_app_key:true}:command==='web_sources_list'?{folder:'',sources:[],settings:[],problems:[]}:undefined);
 s.setFiles([file,{...file,path:'/music/second.flac',name:'second.flac'}]);await s.ui.compare();
 assert.equal(document.querySelector('[data-scope="album"]').getAttribute('aria-pressed'),'true');
 await s.ui.open('beatport');assert.equal(document.querySelectorAll('dialog').length,1);
 document.querySelector('[data-close]').click();await s.ui.open('beatport');assert.ok(document.querySelector('#ts-mode'));document.querySelector('[data-close]').click();
});
test('native sources added before catalogs offer genuine track searches beside album searches',()=>{
  for(const provider of ['musicbrainz','itunes','deezer','lastfm'])assert.ok(searchModes(provider).some(([mode])=>mode==='artist_title'),provider);
});
for(const [provider,key]of [['musicbrainz','MUSICBRAINZ_TRACKID'],['itunes','ITUNESTRACKID'],['deezer','DEEZER_TRACK_ID'],['lastfm','LASTFM_TRACK_ID']])test(`${provider} track search preserves the selected track identity when its release loads`,async()=>{
  const loaded={...release(provider),id:'release-44',tracks:[{...track(1,'Unrelated intro'),fields:{TITLE:['Unrelated intro'],TRACK:['1'],[key]:['other']}},{...track(7,file.title),fields:{TITLE:[file.title],TRACK:['7'],[key]:['chosen-track']}}]};
  const s=setup(async command=>command==='tag_source_search'?{results:[{id:'release-44',track_id:'chosen-track',title:file.title,artist:'Guest'}],total:1}:command==='tag_source_release'?loaded:undefined);
  s.setFiles([{...file,track:'1',fields:{...file.fields,TRACK:['1']}}]);
  await s.ui.open(provider,{artistTitle:true});const d=document.querySelector('dialog');
  assert.equal(d.querySelector('#ts-mode').value,'artist_title');d.querySelector('#ts-search').click();await tick();d.querySelector('#ts-next').click();await tick();
  assert.equal(d.querySelector('#ts-use-track').value,'1');d.querySelector('#ts-apply').click();await tick();
  assert.deepEqual(s.applied[0].items[0].updates[key],['chosen-track']);
  assert.deepEqual(s.applied[0].items[0].updates.TRACK,['7']);
});
test('catalog modes, provider commands and track prefill preserve remix identity',()=>{for(const p of ['beatport','traxsource']){assert.equal(sourceRequest('source-'+p).provider,p);assert.deepEqual(searchModes(p).slice(0,2).map(([m])=>m),['artist_title','release']);}assert.equal(sourceRequest('source-discogs-artist-title').artistTitle,true);assert.deepEqual(trackSearchPrefill([file]),{artist:'Guest',title:'After Hours (Extended Mix)'});assert.equal(catalogReleaseInput(release()),'release:44');});
test('catalog search sends title and exact release navigation retains selected track',async()=>{const s=setup(async(command,args)=>command==='tag_source_search'?{results:[{id:'track:12',title:'After Hours',artist:'Guest',year:'',country:'',label:'Label',catalog_number:'X',format:'Track',track_count:1,score:null,barcode:'',thumb:null,url:''}],total:1}:command==='tag_source_release'?args.input==='release:44'?{...release(),id:'release:44',tracks:[track(1,'Intro'),track(7,'After Hours (Extended Mix)')]}:release():undefined);await s.ui.open('beatport');const d=document.querySelector('dialog');assert.equal(d.querySelector('#ts-title').value,file.title);d.querySelector('#ts-search').click();await tick();assert.equal(s.calls.find(c=>c.command==='tag_source_search').args.query.title,file.title);d.querySelector('#ts-next').click();await tick();d.querySelector('#ts-open-release').click();await tick();assert.equal(s.calls.filter(c=>c.command==='tag_source_release').at(-1).args.input,'release:44');assert.equal(d.querySelector('#ts-use-track').value,'1');assert.match(d.querySelector('.ts-matches tr[data-index="1"]').textContent,/test.flac/);assert.equal(s.applied.length,0);d.querySelector('[data-close]').click();});

test('Discogs artist/title review waits for an explicit track choice',async()=>{
  const s=setup(async command=>command==='tag_source_search'?{results:[{id:'discogs-44',title:'Night',artist:'Guest'}],total:1}:command==='tag_source_release'?{...release('discogs'),id:'44',tracks:[track(1,'Unrelated intro'),track(7,file.title)]}:undefined);
  await s.ui.run('source-discogs-artist-title');const d=document.querySelector('dialog');assert.equal(d.querySelector('#ts-mode').value,'artist_title');assert.equal(d.querySelector('#ts-artist').value,'Guest');d.querySelector('#ts-search').click();await tick();d.querySelector('#ts-next').click();await tick();assert.equal(d.querySelector('#ts-apply').disabled,true);const select=d.querySelector('#ts-use-track');select.value='1';select.dispatchEvent(new Event('change'));assert.equal(d.querySelector('#ts-apply').disabled,false);d.querySelector('#ts-apply').click();await tick();assert.equal(s.applied[0].items[0].updates.TITLE[0],file.title);assert.equal(s.applied[0].items[0].updates.TRACK[0],'7');
});
test('Cloudflare solver settings are optional, validated and have a reset action',async()=>{
  let reset=0;const page=tagSourcesOptionsPages(async()=>null,undefined,undefined,async()=>reset++).find(page=>page.id==='tag-sources-solver');assert.equal(page.label,'Cloudflare solver');assert.equal(page.controls.find(control=>control.id==='tag-source-solver-reset').label,'Retry direct access');assert.equal(page.defaults.endpoint,'');const draft={pages:{'tag-sources-solver':{endpoint:'http://localhost:8191/v1',timeout_seconds:'60'}}};assert.equal(page.validate(draft),undefined);draft.pages['tag-sources-solver'].endpoint='file:///tmp/solver';assert.match(page.validate(draft),/HTTP/);draft.pages['tag-sources-solver'].endpoint='';draft.pages['tag-sources-solver'].timeout_seconds='0';assert.match(page.validate(draft),/1 to 300/);await page.controls.find(control=>control.id==='tag-source-solver-reset').run();assert.equal(reset,1);
});

test('Cloudflare solver endpoint validation matches backend URL restrictions',()=>{
  const page=tagSourcesOptionsPages(async()=>null).find(page=>page.id==='tag-sources-solver');
  const validate=endpoint=>page.validate({pages:{'tag-sources-solver':{endpoint,timeout_seconds:120}}});
  for(const endpoint of ['', '   ', 'http://localhost:8191', 'http://localhost:8191/v1', 'https://solver.example/proxy/v1/', 'http://[::1]:8191/v1', ' https://solver.example/v1 ']) {
    assert.equal(validate(endpoint),undefined,`accept ${endpoint}`);
  }
  for(const endpoint of [
    'http://localhost:8191/v1?token=x',
    'http://localhost:8191/v1?',
    'http://localhost:8191/v1#fragment',
    'http://localhost:8191/v1#',
    'http://user@localhost:8191/v1',
    'http://:password@localhost:8191/v1',
    'ftp://localhost:8191/v1',
    'file:///tmp/solver',
    'http://',
    'not a URL',
  ]) {
    assert.equal(typeof validate(endpoint),'string',`reject ${endpoint}`);
  }
});

test('switching catalog track/release mode uses the corresponding artist and title query',async()=>{const s=setup(async command=>command==='tag_source_search'?{results:[],total:0}:undefined);await s.ui.open('beatport');const d=document.querySelector('dialog'),mode=d.querySelector('#ts-mode');assert.equal(d.querySelector('#ts-artist').value,'Guest');mode.value='release';mode.dispatchEvent(new Event('change'));assert.equal(d.querySelector('#ts-artist').value,'Various Artists');d.querySelector('#ts-search').click();await tick();const query=s.calls.find(call=>call.command==='tag_source_search').args.query;assert.equal(query.mode,'release');assert.equal(query.album,'Night');d.querySelector('[data-close]').click();});

for (const provider of ['beatport','traxsource']) test(`pasted ${provider} track URL retains its canonical track when opening the release`,async()=>{
  const trackKey=provider==='beatport'?'BEATPORT_TRACK_ID':'TRAXSOURCE_TRACK_ID';
  const releaseKey=provider==='beatport'?'BEATPORT_RELEASE_ID':'TRAXSOURCE_RELEASE_ID';
  const url=provider==='beatport'?'https://www.beatport.com/track/after-hours/12':'https://www.traxsource.com/track/12/after-hours';
  const sourceTrack=(number,title)=>({...track(number,title),fields:{TITLE:[title],TRACK:[String(number)],[trackKey]:[number===7?'12':'10']}});
  const sourceRelease=input=>({...release(provider),id:input===url?'track:12':'release:44',fields:{ALBUM:['Night'],[releaseKey]:['44']},tracks:input===url?[sourceTrack(7,file.title)]:[sourceTrack(1,'Unrelated intro'),sourceTrack(7,file.title)]});
  const s=setup(async(command,args)=>command==='tag_source_release'?sourceRelease(args.input):undefined);
  s.setFiles([{...file,track:'1',fields:{...file.fields,TRACK:['1']}}]);
  await s.ui.open(provider);
  const d=document.querySelector('dialog'),mode=d.querySelector('#ts-mode');
  mode.value='release_id';mode.dispatchEvent(new Event('change'));
  d.querySelector('#ts-text').value=url;d.querySelector('#ts-search').click();await tick();
  assert.equal(s.calls.find(call=>call.command==='tag_source_release').args.input,url);
  d.querySelector('#ts-open-release').click();await tick();
  assert.equal(s.calls.filter(call=>call.command==='tag_source_release').at(-1).args.input,'release:44');
  assert.equal(d.querySelector('#ts-use-track').value,'1','pair the canonical track7/id12, despite stale local track1');
  assert.match(d.querySelector('.ts-matches tr[data-index="1"]').textContent,/test.flac/);
  assert.doesNotMatch(d.querySelector('.ts-matches tr[data-index="0"]').textContent,/test.flac/);
  d.querySelector('#ts-apply').click();await tick();
  assert.equal(s.applied.length,1);
  assert.deepEqual(s.applied[0].items,[{path:file.path,updates:{ALBUM:['Night'],[releaseKey]:['44'],TITLE:[file.title],TRACK:['7'],[trackKey]:['12']}}]);
});
