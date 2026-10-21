import test from 'node:test';
import assert from 'node:assert/strict';
import {loadProductionModule} from './helpers/load-production-module.mjs';

const {comparisonFields,comparisonExample,comparisonTrackMatches,buildComparisonPreview,comparisonSelectionKey}=await loadProductionModule('metadata-comparison.ts');
const file=(path,fields,extra={})=>({path,name:path.split('/').pop(),title:fields.TITLE?.[0]||'',artist:fields.ARTIST?.[0]||'',album:fields.ALBUM?.[0]||'',album_artist:fields.ALBUMARTIST?.[0]||'',track:fields.TRACK?.[0]||'',disc:'1',length:300,fields,...extra});
const files=()=>[
  file('/a.flac',{TITLE:['One (Extended Mix)'],ARTIST:['Artist A'],ALBUM:['Compilation'],ALBUMARTIST:['VA'],COMMENT:['Personal A'],DJ_COLLECTION:['Late night','Warm-up']}),
  file('/b.flac',{TITLE:['Two'],ARTIST:['Artist B'],ALBUM:['Compilation'],ALBUMARTIST:['VA'],COMMENT:['Personal B']})
];
const track=(title,artist,fields={},extra={})=>({title,artist,number:1,disc:1,position:'1',length:300,fields:{TITLE:[title],ARTIST:[artist],...fields},...extra});
const release=(overrides={})=>({provider:'musicbrainz',id:'release-1',title:'Compilation',artist:'Various Artists',url:'',fields:{ALBUM:['Compilation'],ALBUMARTIST:['Various Artists']},tracks:[track('Two','Artist B'),track('One (Extended Mix)','Artist A feat. Guest')],covers:[],more_covers:false,...overrides});
const candidate=(overrides={})=>({sourceId:'native:musicbrainz',revision:1,release:release(),tracks:{'/a.flac':1,'/b.flac':0},...overrides});
const from=(sourceId='native:musicbrainz',revision=1)=>({kind:'source',sourceId,revision});

test('Inherit leaves every existing field and file unwritten',()=>{
  const local=files(),before=structuredClone(local);
  assert.deepEqual(buildComparisonPreview(local,[candidate()],{ARTIST:{kind:'inherit'},COMMENT:{kind:'inherit'}}),{items:[],cover:null,missing:[],blocked:[]});
  assert.deepEqual(local,before);
});

test('None clears populated fields across an album and skips fields already absent',()=>{
  const result=buildComparisonPreview(files(),[],{COMMENT:{kind:'none'},MISSING:{kind:'none'}});
  assert.deepEqual(result.items,[{path:'/a.flac',updates:{COMMENT:[]}},{path:'/b.flac',updates:{COMMENT:[]}}]);
});

test('one album provider choice resolves each performing artist separately from album artist',()=>{
  const result=buildComparisonPreview(files(),[candidate()],{ARTIST:from(),ALBUMARTIST:from()});
  assert.deepEqual(result.items,[{path:'/a.flac',updates:{ARTIST:['Artist A feat. Guest'],ALBUMARTIST:['Various Artists']}},{path:'/b.flac',updates:{ALBUMARTIST:['Various Artists']}}]);
  assert.equal(result.blocked.length,0);
});

test('extended tags and ordered multi-values are discovered and preserved without an allowlist',()=>{
  const c=candidate({release:release({fields:{CUSTOM_CREDITS:['First','Second'],ARTWORK:['This is a text tag']}})});
  const names=comparisonFields(files(),[c]);
  for(const name of ['DJ_COLLECTION','CUSTOM_CREDITS','ARTWORK','TITLE','ARTIST','ALBUMARTIST'])assert.ok(names.includes(name),name);
  const result=buildComparisonPreview(files(),[c],{CUSTOM_CREDITS:from(),ARTWORK:from()});
  assert.deepEqual(result.items,[{path:'/a.flac',updates:{CUSTOM_CREDITS:['First','Second'],ARTWORK:['This is a text tag']}},{path:'/b.flac',updates:{CUSTOM_CREDITS:['First','Second'],ARTWORK:['This is a text tag']}}]);
  result.items[0].updates.CUSTOM_CREDITS.push('Local edit');
  assert.deepEqual(c.release.fields.CUSTOM_CREDITS,['First','Second']);
});

test('the album example is the first local file mapped to its actual source track',()=>{
  assert.deepEqual(comparisonExample(files(),candidate(),'ARTIST'),['Artist A feat. Guest']);
  const c=candidate({release:release({tracks:[track('Two','Artist B',{COMPOSER:['B Writer']}),track('One','Artist A')]})});
  assert.equal(comparisonExample(files(),c,'COMPOSER'),null);
});

test('arbitrary extended tag names never read inherited object properties as provider values',()=>{
  const c=candidate({release:release({fields:JSON.parse('{"__proto__":["Custom tag value"]}')})});
  const choices=Object.fromEntries([['__proto__',from()],['toString',from()],['constructor',{kind:'none'}]]);
  const result=buildComparisonPreview(files(),[c],choices);
  assert.deepEqual(Object.keys(result.items[0].updates),['__proto__']);
  assert.deepEqual(result.items[0].updates.__proto__,['Custom tag value']);
  assert.deepEqual(result.missing.map(x=>x.field),['toString','toString']);
});

test('a sparse source preserves missing values and never falls back to another provider',()=>{
  const local=files();local[0].fields.COMPOSER=['My existing credit'];
  const c=candidate({release:release({tracks:[track('Two','Artist B',{COMPOSER:['B Writer']}),track('One','Artist A',{COMPOSER:[]})]})});
  const other=candidate({sourceId:'native:discogs',release:release({fields:{COMPOSER:['Wrong fallback']}})});
  const result=buildComparisonPreview(local,[c,other],{COMPOSER:from()});
  assert.deepEqual(result.items,[{path:'/b.flac',updates:{COMPOSER:['B Writer']}}]);
  assert.deepEqual(result.missing,[{path:'/a.flac',field:'COMPOSER',sourceId:'native:musicbrainz'}]);
  assert.equal(result.blocked.length,0);
});

test('changed candidate revisions block writes from an obsolete field choice',()=>{
  const result=buildComparisonPreview(files(),[candidate({revision:2})],{ARTIST:from()});
  assert.equal(result.blocked.length,2);assert.equal(result.items.length,0);
  assert.match(result.blocked[0].reason,/changed|review/i);
});

test('an unmatched source track blocks track fields while shared release fields remain available',()=>{
  const c=candidate({tracks:{'/a.flac':null,'/b.flac':0}});
  const result=buildComparisonPreview(files(),[c],{TITLE:from(),ALBUMARTIST:from()});
  assert.deepEqual(result.blocked.map(x=>[x.path,x.field]),[['/a.flac','TITLE']]);
  assert.deepEqual(result.items,[{path:'/a.flac',updates:{ALBUMARTIST:['Various Artists']}},{path:'/b.flac',updates:{ALBUMARTIST:['Various Artists']}}]);
});

test('LRCLIB lyrics use raw-lyrics writing only when that field is imported',()=>{
  const c=candidate({sourceId:'native:lrclib',release:release({provider:'lrclib',tracks:[track('Two','Artist B',{LYRICS:['Line one\nLine two']}),track('One','Artist A',{LYRICS:['Another song']})]})});
  assert.deepEqual(buildComparisonPreview(files(),[c],{LYRICS:from('native:lrclib')}).items,[{path:'/a.flac',updates:{LYRICS:['Another song']},raw_lyrics:true},{path:'/b.flac',updates:{LYRICS:['Line one\nLine two']},raw_lyrics:true}]);
  assert.equal(buildComparisonPreview(files(),[c],{ALBUMARTIST:from('native:lrclib')}).items[0].raw_lyrics,undefined);
});

test('artwork choices cannot collide with an extended text tag named ARTWORK',()=>{
  const local=files().map(f=>({...f,covers:[{digest:'cover-1',thumbnail:'',description:'Front'}]}));
  const result=buildComparisonPreview(local,[],{ARTWORK:{kind:'none'}},{kind:'none'});
  assert.deepEqual(result.items,[{path:'/a.flac',updates:{},remove_covers:true},{path:'/b.flac',updates:{},remove_covers:true}]);
  const c=candidate({release:release({covers:[{url:'https://images.example/cover.jpg',thumb:null,kind:'Front',width:600,height:600}]})});
  const imported=buildComparisonPreview(local,[c],{},from());
  assert.deepEqual(imported.cover,{url:'https://images.example/cover.jpg',replace_existing:true,preserve_existing:false,save_as:null});
  assert.deepEqual(imported.items,[{path:'/a.flac',updates:{}},{path:'/b.flac',updates:{}}]);
});

test('mapping follows actual identities even when local names and track numbers are stale',()=>{
  const local=[file('/a.flac',{TITLE:['Old title'],ARTIST:['Old artist'],MUSICBRAINZ_TRACKID:['recording-7'],TRACK:['1']})];
  const source=release({tracks:[track('Unrelated','Other',{MUSICBRAINZ_TRACKID:['recording-1']}),track('Canonical','Artist',{MUSICBRAINZ_TRACKID:['recording-7']},{number:7})]});
  assert.deepEqual(comparisonTrackMatches(local,source),{'/a.flac':1});
  assert.deepEqual(comparisonTrackMatches([files()[0]],source,'recording-7'),{'/a.flac':1});
});

test('title matches preserve mix identity and do not trust stale positions',()=>{
  const local=files();
  const source=release({tracks:[track('Two','Artist B'),track('One (Extended Mix)','Artist A')]});
  assert.deepEqual(comparisonTrackMatches(local,source),{'/a.flac':1,'/b.flac':0});
  assert.deepEqual(comparisonTrackMatches([local[0]],release({tracks:[track('One (Radio Edit)','Artist A')]})),{'/a.flac':null});
});

test('ambiguous or incompatible-duration tracks require explicit matching',()=>{
  const a=files()[0];
  assert.deepEqual(comparisonTrackMatches([a],release({tracks:[track(a.title,a.artist),track(a.title,a.artist)]})),{'/a.flac':null});
  assert.deepEqual(comparisonTrackMatches([a],release({tracks:[track(a.title,a.artist,{}, {length:240})]})),{'/a.flac':null});
  assert.deepEqual(comparisonTrackMatches([a,{...a,path:'/copy.flac'}],release({tracks:[track(a.title,a.artist)]})),{'/a.flac':null,'/copy.flac':null});
});

test('a selection key ignores field-key ordering but detects edited values and reordered files',()=>{
  const local=files(),copy=structuredClone(local);
  copy[0].fields=Object.fromEntries(Object.entries(copy[0].fields).reverse());
  assert.equal(comparisonSelectionKey(local),comparisonSelectionKey(copy));
  copy[0].fields.COMMENT=['New note'];
  assert.notEqual(comparisonSelectionKey(local),comparisonSelectionKey(copy));
  assert.notEqual(comparisonSelectionKey(local),comparisonSelectionKey([...local].reverse()));
});
