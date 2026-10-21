import test from 'node:test';
import assert from 'node:assert/strict';
import {loadProductionModule} from './helpers/load-production-module.mjs';
const {comparisonSources,comparisonQuery,ComparisonSearchSession}=await loadProductionModule('metadata-comparison-sources.ts');
const {buildComparisonPreview}=await loadProductionModule('metadata-comparison.ts');
const credentials={discogs_token_set:false,discogs_app_key:true,acoustid_key_set:true,lastfm_key_set:true,fanart_key_set:true};
const listing={folder:'/sources',sources:[],settings:[],problems:[]};
const files=[{path:'/a.flac',name:'a.flac',title:'One (Extended Mix)',artist:'Artist A',album:'Compilation',album_artist:'VA',track:'1',disc:'1',length:300,fields:{TITLE:['One (Extended Mix)'],ARTIST:['Artist A'],ALBUM:['Compilation'],ALBUMARTIST:['VA']}},{path:'/b.flac',name:'b.flac',title:'Two',artist:'Artist B',album:'Compilation',album_artist:'VA',track:'2',disc:'1',length:301,fields:{TITLE:['Two'],ARTIST:['Artist B'],ALBUM:['Compilation'],ALBUMARTIST:['VA']}}];
const terms={artist:'Artist A',title:'One (Extended Mix)',albumArtist:'VA',album:'Compilation'};
const row=(id='release:1',extra={})=>({id,title:'Compilation',artist:'VA',year:'2025',country:'',label:'Label',catalog_number:'X1',format:'Album',track_count:2,score:null,barcode:'',thumb:null,url:'https://example.invalid/release/1',...extra});
const release=(provider='musicbrainz',extra={})=>({provider,id:'release:1',title:'Compilation',artist:'Various Artists',url:'',fields:{ALBUM:['Compilation'],ALBUMARTIST:['Various Artists']},tracks:files.map((f,i)=>({title:f.title,artist:f.artist,number:i+1,disc:1,position:String(i+1),length:f.length,fields:{TITLE:[f.title],ARTIST:[f.artist]}})),covers:[],more_covers:false,...extra});
const source=name=>comparisonSources(credentials,listing).find(s=>s.provider===name);
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
function session(invoke,evaluate=async()=> 'Resolved words'){
  let ticket=100;const updates=[];
  const value=new ComparisonSearchSession({invoke,evaluate,nextTicket:()=>++ticket,onSourceState:s=>updates.push(s)});
  return{value,updates};
}

test('every native metadata source gets its real query mode and performing/album artist scope',()=>{
  for(const provider of ['musicbrainz','discogs','beatport','traxsource','itunes','deezer','lastfm']){
    const s=source(provider),single=comparisonQuery(s,'track',[files[0]],terms),album=comparisonQuery(s,'album',files,terms);
    assert.equal(single.mode,'artist_title',provider);assert.equal(single.artist,'Artist A');assert.equal(single.title,'One (Extended Mix)');
    assert.equal(album.artist,'VA',provider);assert.equal(album.album,'Compilation');
    assert.equal(album.mode,['beatport','traxsource'].includes(provider)?'release':'artist_album');
  }
  assert.equal(comparisonQuery(source('lastfm'),'album',files,{...terms,albumArtist:''}).mode,'text');
  for(const [provider,mode]of [['lrclib','files'],['acoustid','fingerprint']]){
    const query=comparisonQuery(source(provider),'album',files,terms);assert.equal(query.mode,mode);assert.equal(query.files.length,2);assert.equal(query.files[1].artist,'Artist B');
  }
  assert.equal(comparisonQuery(source('fanart'),'track',[files[0]],terms).album,'Compilation');
  assert.equal(comparisonQuery(source('freedb'),'album',files,terms).mode,'artist_album');
  assert.equal(comparisonQuery(source('freedb'),'track',[files[0]],terms).kind,'needs-input');
  assert.equal(comparisonQuery(source('local_freedb'),'album',files,terms).mode,'text');
});

test('unknown album artist stays empty instead of inheriting a compilation performer',()=>{
  const query=comparisonQuery(source('musicbrainz'),'album',files,{...terms,albumArtist:''});
  assert.equal(query.artist,'');assert.equal(query.album,'Compilation');
});

test('single-track lyric lookup honors corrected search terms without rewriting album performers',()=>{
  const single=comparisonQuery(source('lrclib'),'track',[files[0]],{...terms,artist:'Correct performer',title:'Correct title'});
  assert.equal(single.files[0].artist,'Correct performer');assert.equal(single.files[0].title,'Correct title');
  const album=comparisonQuery(source('lrclib'),'album',files,{...terms,artist:'Correct performer',title:'Correct title'});
  assert.equal(album.files[1].artist,'Artist B');
});

test('missing keys and broken installed scripts remain visible without provider requests',async()=>{
  const available=comparisonSources({...credentials,lastfm_key_set:false},{...listing,problems:[{file:'Broken.src',error:'Invalid search declaration'}]});
  assert.match(available.find(s=>s.provider==='lastfm').setupReason,/key/i);
  assert.match(available.find(s=>s.label==='Broken.src').setupReason,/Invalid search/);
  const s=session(async()=>{assert.fail('a source requiring setup must not be invoked');});
  await s.value.search(available.filter(x=>x.setupReason),'album',files,terms);
  assert.ok([...s.value.states.values()].every(state=>state.status==='needs-setup'));
});

test('parallel sources share one cancellation ticket and at most three provider calls',async()=>{
  let active=0,peak=0;const tickets=new Set(),pending=[];
  const s=session(async(command,args)=>{
    if(command==='tag_source_cancel')return;
    tickets.add(args.ticket);active++;peak=Math.max(peak,active);
    await new Promise(resolve=>pending.push(resolve));active--;
    if(command==='tag_source_search')return{results:[row()],total:1};
    if(command==='tag_source_release')return release(args.provider);
    assert.fail(command);
  });
  const run=s.value.search(['musicbrainz','discogs','beatport','traxsource','itunes','deezer'].map(source),'album',files,terms);
  // Several completed searches can enqueue their detail lookups in the same turn.
  for(let round=0;round<8;round++){pending.splice(0).forEach(resolve=>resolve());await tick();}
  await run;
  assert.equal(tickets.size,1);assert.equal(peak,3);
  assert.ok([...s.value.states.values()].every(state=>state.status==='ready'));
  assert.deepEqual(s.value.states.get('native:musicbrainz').candidate.tracks,{'/a.flac':0,'/b.flac':1});
});

test('a failed source does not hide other results and retries only that source',async()=>{
  const searches=[];let failed=false;
  const s=session(async(command,args)=>{
    if(command==='tag_source_cancel')return;
    if(command==='tag_source_search'){searches.push(args.provider);if(args.provider==='beatport'&&!failed){failed=true;throw Error('Access restricted');}return{results:[row()],total:1};}
    if(command==='tag_source_release')return release(args.provider);
    assert.fail(command);
  });
  await s.value.search([source('musicbrainz'),source('beatport')],'album',files,terms);
  assert.equal(s.value.states.get('native:beatport').status,'failed');
  const kept=s.value.states.get('native:musicbrainz').candidate;
  await s.value.retry('native:beatport');
  assert.deepEqual(searches,['musicbrainz','beatport','beatport']);
  assert.equal(s.value.states.get('native:musicbrainz').candidate,kept);
  assert.equal(s.value.states.get('native:beatport').status,'ready');
});

test('two installed web sources retain their own result URLs, inputs, names and candidates',async()=>{
  const web={...listing,sources:[{id:'one.src',name:'First catalog',menu:'First catalog',search_by:[{label:'Artist/title',format:'%artist% %title%'}],direct:false},{id:'two.src',name:'Second catalog',menu:'Second catalog',search_by:[{label:'Album',format:'%album%'}],direct:true}]};
  const available=comparisonSources(credentials,web).filter(s=>s.provider==='web'),requests=[];
  const s=session(async(command,args)=>{
    if(command==='tag_source_cancel')return;
    requests.push({command,args});
    if(command==='web_source_search')return{columns:['Title'],results:[{url:'https://catalog.invalid/'+args.source+'?edition=exact',preview:'https://catalog.invalid/preview/'+args.source,cover:null,values:['Compilation']}],direct:args.source==='two.src'};
    if(command==='web_source_release')return release('web',{fields:{CUSTOM_SOURCE:[args.source]}});
    assert.fail(command);
  },async(path,format)=>format==='%album%'?'Compilation':'Artist A One (Extended Mix)');
  await s.value.search(available,'album',files,terms);
  assert.deepEqual([...s.value.states.keys()],['web:one.src','web:two.src']);
  assert.equal(s.value.states.get('web:one.src').candidate.release.fields.CUSTOM_SOURCE[0],'one.src');
  assert.equal(s.value.states.get('web:two.src').candidate.release.fields.CUSTOM_SOURCE[0],'two.src');
  assert.deepEqual(requests.find(x=>x.command==='web_source_search'&&x.args.source==='two.src').args.values,['Compilation']);
  const load=requests.find(x=>x.command==='web_source_release'&&x.args.source==='one.src');
  assert.equal(load.args.url,'https://catalog.invalid/one.src?edition=exact');assert.equal(load.args.previewUrl,'https://catalog.invalid/preview/one.src');
});

test('a script needing extra input is queryable after that input is supplied',async()=>{
  const web={...listing,sources:[{id:'custom.src',name:'Custom catalog',menu:'Custom catalog',search_by:[{label:'Catalog number',format:''}],direct:false}]};
  const selected=comparisonSources(credentials,web).filter(s=>s.provider==='web'),calls=[];
  const s=session(async(command,args)=>{if(command==='tag_source_cancel')return;calls.push({command,args});return{columns:[],results:[],direct:false};},async()=> '');
  await s.value.search(selected,'track',[files[0]],terms);
  assert.equal(s.value.states.get('web:custom.src').status,'needs-input');assert.equal(calls.length,0);
  await s.value.search(selected,'track',[files[0]],{...terms,webValues:{'web:custom.src':['CAT004']}});
  assert.deepEqual(calls[0].args.values,['CAT004']);
});

test('an explicitly supplied optional script input can remain empty',async()=>{
  const web={...listing,sources:[{id:'optional.src',name:'Catalog',menu:'Catalog',search_by:[{label:'Title',format:'%title%'},{label:'Edition (optional)',format:''}],direct:false}]};
  const selected=comparisonSources(credentials,web).filter(s=>s.provider==='web'),calls=[];
  const s=session(async(command,args)=>{if(command==='tag_source_cancel')return;calls.push({command,args});return{columns:[],results:[],direct:false};});
  await s.value.search(selected,'track',[files[0]],{...terms,webValues:{'web:optional.src':['One','']}});
  assert.deepEqual(calls[0]?.args.values,['One','']);
});

test('supplying a source-specific retry input preserves other provider candidates',async()=>{
  const web={...listing,sources:[{id:'custom.src',name:'Catalog',menu:'Catalog',search_by:[{label:'Catalog number',format:''}],direct:false}]};
  const selected=[source('musicbrainz'),...comparisonSources(credentials,web).filter(s=>s.provider==='web')],calls=[];
  const s=session(async(command,args)=>{
    if(command==='tag_source_cancel')return;
    calls.push({command,args});
    if(command==='tag_source_search')return{results:[row()],total:1};
    if(command==='tag_source_release')return release();
    if(command==='web_source_search')return{columns:[],results:[],direct:false};
    assert.fail(command);
  },async()=> '');
  await s.value.search(selected,'album',files,terms);
  const kept=s.value.states.get('native:musicbrainz').candidate;
  await s.value.retry('web:custom.src',{...terms,webValues:{'web:custom.src':['CAT004']}});
  assert.deepEqual(calls.find(c=>c.command==='web_source_search')?.args.values,['CAT004']);
  assert.equal(s.value.states.get('native:musicbrainz').candidate,kept);
});

test('Cancel discards late replies and prevents queued provider calls',async()=>{
  const pending=[];let calls=0;const cancelled=[];
  const s=session((command,args)=>{
    if(command==='tag_source_cancel'){cancelled.push(args.ticket);return Promise.resolve();}
    calls++;return new Promise(resolve=>pending.push(resolve));
  });
  const search=s.value.search(['musicbrainz','discogs','beatport','traxsource'].map(source),'album',files,terms);
  await tick();assert.equal(calls,3);s.value.cancel();pending.forEach(resolve=>resolve({results:[row()],total:1}));await search;
  assert.equal(calls,3);assert.equal(cancelled.length,1);
  assert.ok([...s.value.states.values()].every(state=>!state.candidate));
});

test('an older detail response cannot replace a newer explicit candidate',async()=>{
  let resolveOld;
  const s=session(async(command,args)=>{
    if(command==='tag_source_cancel')return;
    if(command==='tag_source_search')return{results:[],total:0};
    if(command==='tag_source_release'&&args.input==='old')return new Promise(resolve=>resolveOld=resolve);
    if(command==='tag_source_release')return release(args.provider,{id:'new',fields:{ALBUM:['New candidate']}});
    assert.fail(command);
  });
  await s.value.search([source('beatport')],'album',files,terms);
  const old=s.value.load('native:beatport',row('old'));await tick();await s.value.load('native:beatport',row('new'));
  resolveOld(release('beatport',{id:'old'}));await old;
  assert.equal(s.value.states.get('native:beatport').candidate.release.id,'new');
});

test('LRCLIB preserves the per-file lookup order even when lyric titles are not usable for matching',async()=>{
  const s=session(async(command,args)=>command==='tag_source_search'?{results:[row('files:11,12')],total:1}:command==='tag_source_release'?release('lrclib',{tracks:[{...release().tracks[0],title:'Unrelated lyric title'},{...release().tracks[1],title:''}]}):undefined);
  await s.value.search([source('lrclib')],'album',files,terms);
  assert.deepEqual(s.value.states.get('native:lrclib').candidate.tracks,{'/a.flac':0,'/b.flac':1});
});

test('AcoustID imports only each file’s own recording-associated identifier',async()=>{
  const s=session(async(command,args)=>{
    if(command==='tag_source_search')return{results:[row()],total:1};
    if(command==='tag_source_release')return release('acoustid',{tracks:[{...release().tracks[0],fields:{MUSICBRAINZ_TRACKID:['recording-b'],ACOUSTID_ID:['wrong-b']}},{...release().tracks[1],fields:{MUSICBRAINZ_TRACKID:['recording-a'],ACOUSTID_ID:['wrong-a']}}]});
    if(command==='acoustid_file_matches')return{'/a.flac':[{recording:'recording-a',acoustid:'own-a'}],'/b.flac':[{recording:'recording-b',acoustid:'own-b'}]};
    assert.fail(command);
  });
  await s.value.search([source('acoustid')],'album',files,terms);
  const c=s.value.states.get('native:acoustid').candidate;
  assert.deepEqual(c.tracks,{'/a.flac':1,'/b.flac':0});
  assert.deepEqual(buildComparisonPreview(files,[c],{ACOUSTID_ID:{kind:'source',sourceId:c.sourceId,revision:c.revision}}).items,[{path:'/a.flac',updates:{ACOUSTID_ID:['own-a']}},{path:'/b.flac',updates:{ACOUSTID_ID:['own-b']}}]);
});

test('AcoustID cannot silently assign one remote album track to two local files',async()=>{
  const s=session(async(command,args)=>{
    if(command==='tag_source_search')return{results:[row()],total:1};
    if(command==='tag_source_release')return release('acoustid',{tracks:[{...release().tracks[0],fields:{MUSICBRAINZ_TRACKID:['same-recording']}},{...release().tracks[1],fields:{MUSICBRAINZ_TRACKID:['other-recording']}}]});
    if(command==='acoustid_file_matches')return{'/a.flac':[{recording:'same-recording',acoustid:'own-a'}],'/b.flac':[{recording:'same-recording',acoustid:'own-b'}]};
    assert.fail(command);
  });
  await s.value.search([source('acoustid')],'album',files,terms);
  const state=s.value.states.get('native:acoustid');
  assert.deepEqual(state.candidate.tracks,{'/a.flac':null,'/b.flac':null});
  assert.equal(state.status,'needs-match');
});

test('more native results retain the selected candidate and use the next page',async()=>{
  const pages=[];
  const s=session(async(command,args)=>{
    if(command==='tag_source_search'){pages.push(args.query.page);return args.query.page===0?{results:[row('1'),row('2')],total:3}:{results:[row('3')],total:3};}
    if(command==='tag_source_release')return release(args.provider,{id:args.input});
    assert.fail(command);
  });
  await s.value.search([source('deezer')],'album',files,terms);
  const kept=s.value.states.get('native:deezer').candidate;
  await s.value.more('native:deezer');
  assert.deepEqual(pages,[0,1]);assert.equal(s.value.states.get('native:deezer').results.length,3);assert.equal(s.value.states.get('native:deezer').candidate,kept);
});

test('explicit track remapping is one-to-one and invalidates previously selected source values',async()=>{
  const s=session(async(command,args)=>command==='tag_source_search'?{results:[row()],total:1}:command==='tag_source_release'?release(args.provider):undefined);
  await s.value.search([source('musicbrainz')],'album',files,terms);
  const before=s.value.states.get('native:musicbrainz').candidate;
  s.value.match('native:musicbrainz','/b.flac',0);
  const changed=s.value.states.get('native:musicbrainz');
  assert.deepEqual(changed.candidate.tracks,{'/a.flac':null,'/b.flac':0});assert.equal(changed.status,'needs-match');
  assert.ok(changed.candidate.revision>before.revision);
  assert.equal(buildComparisonPreview(files,[changed.candidate],{ARTIST:{kind:'source',sourceId:before.sourceId,revision:before.revision}}).blocked.length,2);
  s.value.match('native:musicbrainz','/not-selected',1);s.value.match('native:musicbrainz','/a.flac',99);
  assert.equal(s.value.states.get('native:musicbrainz').candidate,changed.candidate);
  s.value.match('native:musicbrainz','/a.flac',1);assert.equal(s.value.states.get('native:musicbrainz').status,'ready');
});

test('an in-flight detail lookup cannot overwrite a later explicit track mapping',async()=>{
  let resolveLate;
  const s=session(async(command,args)=>{
    if(command==='tag_source_search')return{results:[row()],total:1};
    if(command==='tag_source_release'&&args.input==='late')return new Promise(resolve=>resolveLate=resolve);
    if(command==='tag_source_release')return release(args.provider);
    assert.fail(command);
  });
  await s.value.search([source('musicbrainz')],'album',files,terms);
  const pending=s.value.load('native:musicbrainz',row('late'));await tick();
  s.value.match('native:musicbrainz','/a.flac',1);
  resolveLate(release());await pending;
  assert.deepEqual(s.value.states.get('native:musicbrainz').candidate.tracks,{'/a.flac':1,'/b.flac':null});
});
