import test from 'node:test';
import assert from 'node:assert/strict';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';
const {MetadataComparisonUI}=await loadProductionModule('metadata-comparison-ui.ts');
const pixel='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6yWQAAAAASUVORK5CYII=';
const originals=()=>[
 {path:'/a.flac',name:'a.flac',title:'One (Extended Mix)',artist:'Artist A',album:'Compilation',album_artist:'VA',track:'1',disc:'1',length:300,fields:{TITLE:['One (Extended Mix)'],ARTIST:['Artist A'],ALBUM:['Compilation'],ALBUMARTIST:['VA'],ISRC:['ID-A'],COMMENT:['Personal note'],DJ_COLLECTION:['Late night','Warm-up']},covers:[{digest:'old-cover',thumbnail:pixel,description:'Old front'}]},
 {path:'/b.flac',name:'b.flac',title:'Two',artist:'Artist B',album:'Compilation',album_artist:'VA',track:'2',disc:'1',length:301,fields:{TITLE:['Two'],ARTIST:['Artist B'],ALBUM:['Compilation'],ALBUMARTIST:['VA'],ISRC:['ID-B'],COMMENT:['Another note']},covers:[{digest:'old-cover',thumbnail:pixel,description:'Old front'}]}
];
const result=()=>({id:'release:44',title:'Compilation',artist:'Various Artists',year:'2025',country:'',label:'Label',catalog_number:'CAT44',format:'Album',track_count:2,score:null,barcode:'',thumb:null,url:''});
function sourceRelease(provider){
 const tracks=originals().map((f,i)=>({position:String(i+1),disc:1,number:i+1,title:f.title,artist:i===0?'Artist A feat. Guest':f.artist,length:f.length,fields:{TITLE:[f.title],ARTIST:[i===0?'Artist A feat. Guest':f.artist],ISRC:f.fields.ISRC,TRACK:[String(i+1)],CUSTOM_CREDIT:['First','Second']}}));
 if(provider==='traxsource'){tracks[0].fields.GENRE=[];tracks[1].fields.GENRE=['Deep House'];}
 if(provider==='lrclib'){tracks[0].fields.UNSYNCEDLYRICS=['Line one\nLine two\n'+('Long lyric text '.repeat(20))];tracks[1].fields.UNSYNCEDLYRICS=[];}
 return{provider,id:provider==='lrclib'?'':'release:44',title:'Compilation',artist:'Various Artists',url:'',fields:{ALBUM:['Compilation'],ALBUMARTIST:['Various Artists'],GENRE:['Deep House']},tracks,covers:[{url:'https://coverartarchive.org/release/example/front',thumb:null,kind:'Front',width:600,height:600}],more_covers:false};
}
function setup({local=originals(),invoke,apply}={}){
 const w=new Window({url:'https://test.invalid'});for(const key of ['window','document','Node','Element','HTMLElement','HTMLInputElement','HTMLButtonElement','HTMLDialogElement','Event','KeyboardEvent'])globalThis[key]=key==='window'?w:w[key];
 let files=structuredClone(local),counter=1000,reason='';const calls=[],written=[];
 const defaults=async(command,args)=>{
  if(command==='load_tag_source_settings')return{discogs_token_set:false,discogs_app_key:true,acoustid_key_set:true,lastfm_key_set:true,fanart_key_set:true};
  if(command==='web_sources_list')return{folder:'/sources',sources:[],settings:[],problems:[]};
  if(command==='local_freedb_index_status')return{directory:null,discs:0};
  if(command==='tag_source_comparison_snapshot')return 'snapshot-1';
  if(command==='tag_source_comparison_cancel'||command==='tag_source_cancel')return;
  if(command==='tag_source_image')return pixel;
  if(command==='tag_source_search')return{results:[{...result(),...(args.provider==='lrclib'?{id:'files:11,12'}:{})}],total:1};
  if(command==='tag_source_release')return sourceRelease(args.provider);
  assert.fail('Unexpected IPC command '+command);
 };
 const ui=new MetadataComparisonUI({files:()=>files,reason:()=>reason,nextTicket:()=>++counter,evaluate:async()=> 'Resolved words',loadExistingCovers:async path=>files.find(f=>f.path===path)?.covers||[],
  invoke:async(command,args)=>{calls.push({command,args});return invoke?invoke(command,args,defaults):defaults(command,args);},
  applyComparison:async(items,cover)=>{written.push({items,cover});return apply?apply(items,cover):{succeeded:items.map(item=>item.path),failed:[]};}
 });
 return{ui,w,calls,written,setFiles:value=>files=value,setReason:value=>reason=value};
}
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));
const main=()=>document.querySelector('dialog.metadata-comparison-dialog');
const picker=()=>document.querySelector('dialog.metadata-comparison-picker');
async function search(){main().querySelector('[data-search]').click();for(let i=0;i<12;i++)await tick();}
function choose(field,source){main().querySelector(`[data-pick-field="${field}"]`).click();picker().querySelector(`[data-source-choice="${source}"]`).click();}
function close(){main()?.querySelector('[data-close]').click();}

test('opening either scope shows existing values and makes no provider requests',async()=>{
 for(const local of [[originals()[0]],originals()]){
  const s=setup({local});await s.ui.open();const d=main();
  assert.equal(d.querySelector('[data-scope="album"]').getAttribute('aria-pressed'),String(local.length>1));
  assert.equal(s.calls.filter(c=>['tag_source_search','web_source_search'].includes(c.command)).length,0);
  assert.ok(d.querySelector('[data-field="DJ_COLLECTION"]'));assert.equal(d.querySelector('[data-review]').disabled,true);
  assert.match(d.querySelector('[data-field="ALBUMARTIST"]').textContent,/VA/);close();
 }
});

test('album search uses album artist while single-track search uses the performer',async()=>{
 const s=setup();await s.ui.open();await search();
 const query=s.calls.find(c=>c.command==='tag_source_search'&&c.args.provider==='musicbrainz').args.query;
 assert.equal(query.album,'Compilation');assert.equal(query.artist,'Various Artists');assert.notEqual(query.artist,'Artist A');close();
 const single=setup({local:[originals()[0]]});await single.ui.open();await search();
 const track=single.calls.find(c=>c.command==='tag_source_search'&&c.args.provider==='musicbrainz').args.query;
 assert.equal(track.mode,'artist_title');assert.equal(track.artist,'Artist A');assert.equal(track.title,'One (Extended Mix)');close();
});

test('the open album selector gives every provider a dedicated readonly metadata field using track 1',async()=>{
 const s=setup();await s.ui.open();await search();main().querySelector('[data-pick-field="ARTIST"]').click();const p=picker();
 assert.match(p.textContent,/track 1/i);assert.match(p.textContent,/all 2 tracks/i);
 const current=p.querySelector('[data-existing-value]');assert.ok(current.readOnly);assert.equal(current.value,'Artist A');
 const values=p.querySelectorAll('[data-provider-value]');assert.ok(values.length>=7);
 for(const value of values){assert.ok(value.readOnly);assert.equal(value.value,'Artist A feat. Guest');}
 assert.ok(p.querySelector('[data-source-choice="inherit"]'));assert.ok(p.querySelector('[data-source-choice="none"]'));
 p.querySelector('[data-close]').click();close();
});

test('the example never substitutes another album track when the chosen provider lacks track 1 data',async()=>{
 const s=setup();await s.ui.open();await search();main().querySelector('[data-pick-field="GENRE"]').click();
 const value=picker().querySelector('[data-provider-value="native:traxsource"]');
 assert.equal(value.value,'');assert.match(value.getAttribute('placeholder'),/no value/i);
 picker().querySelector('[data-close]').click();close();
});

test('lyrics and multi-value extended metadata stay completely visible in provider value fields',async()=>{
 const s=setup();await s.ui.open();await search();main().querySelector('[data-pick-field="UNSYNCEDLYRICS"]').click();
 const lyrics=picker().querySelector('[data-provider-value="native:lrclib"]');assert.equal(lyrics.tagName,'TEXTAREA');assert.ok(lyrics.value.endsWith('Long lyric text '));assert.match(lyrics.value,/Line one\nLine two/);
 picker().querySelector('[data-close]').click();main().querySelector('[data-pick-field="CUSTOM_CREDIT"]').click();
 const credit=picker().querySelector('[data-provider-value="native:musicbrainz"]');assert.match(credit.value,/First/);assert.match(credit.value,/Second/);picker().querySelector('[data-close]').click();close();
});

test('one provider choice per field saves each album artist and performer independently',async()=>{
 const s=setup();await s.ui.open();await search();choose('ARTIST','native:musicbrainz');choose('ALBUMARTIST','native:discogs');choose('COMMENT','none');
 assert.match(main().querySelector('[data-field="ALBUMARTIST"]').textContent,/VA/);
 main().querySelector('[data-review]').click();const review=document.querySelector('dialog.metadata-comparison-review');assert.match(review.textContent,/Remove/);
 review.querySelector('[data-save]').click();await tick();assert.equal(s.written.length,1);
 assert.deepEqual(s.written[0],{items:[{path:'/a.flac',updates:{ARTIST:['Artist A feat. Guest'],ALBUMARTIST:['Various Artists'],COMMENT:[]},comparison_snapshot:'snapshot-1'},{path:'/b.flac',updates:{ALBUMARTIST:['Various Artists'],COMMENT:[]},comparison_snapshot:'snapshot-1'}],cover:null});
 assert.equal(main(),null);
});

test('Cancel does not apply chosen provider values or explicit removals',async()=>{
 const s=setup();await s.ui.open();choose('COMMENT','none');close();await tick();assert.equal(s.written.length,0);assert.ok(s.calls.some(c=>c.command==='tag_source_comparison_cancel'));
});

test('a changed selection is rejected before applying the reviewed batch',async()=>{
 const s=setup();await s.ui.open();choose('COMMENT','none');main().querySelector('[data-review]').click();
 s.setFiles([{...originals()[0],path:'/other.flac'}]);document.querySelector('[data-save]').click();await tick();assert.equal(s.written.length,0);assert.match(document.body.textContent,/selection changed|metadata changed/i);
 document.querySelector('dialog.metadata-comparison-review')?.querySelector('[data-close]').click();close();
});

test('artwork None removes embedded covers without requesting an external image save',async()=>{
 const s=setup();await s.ui.open();main().querySelector('[data-pick-artwork]').click();picker().querySelector('[data-source-choice="none"]').click();main().querySelector('[data-review]').click();document.querySelector('[data-save]').click();await tick();
 assert.deepEqual(s.written[0],{items:[{path:'/a.flac',updates:{},remove_covers:true,comparison_snapshot:'snapshot-1'},{path:'/b.flac',updates:{},remove_covers:true,comparison_snapshot:'snapshot-1'}],cover:null});
});

test('album groups with conflicting local titles require an explicit grouping decision',async()=>{
 const local=originals();local[1].album='Different album';local[1].fields.ALBUM=['Different album'];const s=setup({local});await s.ui.open();
 assert.match(main().textContent,/different albums|different album titles/i);assert.equal(main().querySelector('[data-search]').disabled,true);assert.ok(main().querySelector('[data-confirm-album]'));close();
});

test('extended field names that resemble object properties still have a working value picker',async()=>{
 const local=[originals()[0]];local[0].fields=Object.fromEntries([...Object.entries(local[0].fields),['toString',['My custom value']]]);
 const s=setup({local});await s.ui.open();main().querySelector('[data-pick-field="toString"]').click();
 assert.equal(picker()?.querySelector('[data-existing-value]')?.value,'My custom value');picker().querySelector('[data-close]').click();close();
});

test('credential read failures have an honest setup state instead of an endless checking label',async()=>{
 const s=setup({invoke:async(command,args,defaults)=>{if(command==='load_tag_source_settings')throw Error('Settings unavailable');return defaults(command,args);}});
 await s.ui.open();main().querySelector('[data-sources]').click();
 assert.match(document.querySelector('dialog.metadata-comparison-sources').textContent,/API key settings.*unavailable/i);
 document.querySelector('dialog.metadata-comparison-sources [data-close]').click();close();
});

test('the captured native metadata refreshes the shown existing value and default track query',async()=>{
 const local=[originals()[0]];
 const s=setup({local,invoke:async(command,args,defaults)=>{
  if(command==='tag_source_comparison_snapshot')return{token:'snapshot-fresh',files:[{path:'/a.flac',fields:{...local[0].fields,TITLE:['Current native title'],COMMENT:['Current native note']},duration:300,covers:local[0].covers}]};
  return defaults(command,args);
 }});
 await s.ui.open();assert.equal(main().querySelector('[data-query-title]').value,'Current native title');assert.match(main().querySelector('.mc-heading strong').textContent,/Current native title/);
 main().querySelector('[data-pick-field="COMMENT"]').click();assert.equal(picker().querySelector('[data-existing-value]').value,'Current native note');picker().querySelector('[data-close]').click();close();
});

test('capturing current metadata preserves search terms already edited while the read was pending',async()=>{
 let resolve;const local=[originals()[0]];
 const s=setup({local,invoke:async(command,args,defaults)=>command==='tag_source_comparison_snapshot'?new Promise(done=>resolve=done):defaults(command,args)});
 const opening=s.ui.open();await tick();const input=main().querySelector('[data-query-title]');input.value='My chosen search';input.dispatchEvent(new Event('input',{bubbles:true}));
 resolve({token:'fresh',files:[{path:'/a.flac',fields:{...local[0].fields,TITLE:['Current native title']},duration:300,covers:local[0].covers}]});await opening;
 assert.equal(main().querySelector('[data-query-title]').value,'My chosen search');close();
});
