import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>{
  const source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const mapping=await load('field-mapping'),remove=await load('remove-tag'),options=await load('options-ui');
const rule=(tag_type,source,target)=>({tag_type,source,target});
const key=(k,o={})=>({key:k,code:/^[a-z]$/i.test(k)?`Key${k.toUpperCase()}`:k,ctrlKey:false,metaKey:false,altKey:false,shiftKey:false,...o});

test('Ctrl+R removes tags outside text fields and every Ctrl/⌘+R is consumed',()=>{
  assert.equal(remove.removeTagShortcut(key('r',{ctrlKey:true}),false,false),'remove-tags');
  assert.equal(remove.removeTagShortcut(key('R',{metaKey:true}),false,true),'remove-tags');
  // A layout that reports another character still matches by physical key.
  assert.equal(remove.removeTagShortcut({...key('к',{ctrlKey:true}),code:'KeyR'},false,false),'remove-tags');
  for(const [event,typing,mac] of [[key('r',{ctrlKey:true}),true,false],[key('r',{ctrlKey:true,shiftKey:true}),false,false],[key('r',{ctrlKey:true}),false,true],[key('r',{metaKey:true,altKey:true}),false,true],[key('r',{metaKey:true}),false,false]])
    assert.equal(remove.removeTagShortcut(event,typing,mac),'suppress',JSON.stringify(event));
  assert.equal(remove.removeTagShortcut(key('r'),false,false),undefined);
  assert.equal(remove.removeTagShortcut({...key('¶',{ctrlKey:true,altKey:true}),code:'KeyR'},true,false),undefined,'AltGr+R types its character');
  assert.equal(remove.removeTagShortcut(key('s',{ctrlKey:true}),false,false),undefined);
});

test('the Remove tag prompt names the kinds removed per format family',()=>{
  const tags=options.defaultSettings().tags;
  const text=remove.removeTagPrompt(2,tags);
  assert.match(text,/^Remove tags from 2 selected files\?/);
  assert.match(text,/MP3, TTA: ID3v1, ID3v2 and APE/);
  assert.match(text,/APE, MPC, WavPack, TAK, OptimFROG: ID3v1 and APE/);
  assert.match(text,/Other formats: all tags/);
  const none={...tags,mpeg:{...tags.mpeg,remove_id3v1:false,remove_id3v2:false,remove_ape:false}};
  assert.match(remove.removeTagPrompt(1,none),/^Remove tags from 1 selected file\?[\s\S]*MP3, TTA: nothing \(no tag type is checked under Options → Tags → Mpeg\)/);
});

test('defaults: Remove selects every kind except ID3v2 for the APE family, no mappings',()=>{
  const {mpeg,ape,mappings}=options.defaultSettings().tags;
  assert.deepEqual([mpeg.remove_id3v1,mpeg.remove_id3v2,mpeg.remove_ape],[true,true,true]);
  assert.deepEqual([ape.remove_id3v1,ape.remove_id3v2,ape.remove_ape],[true,false,true]);
  assert.deepEqual(mappings,[]);
});

test('mapping rules are normalized and validated like the backend',()=>{
  assert.deepEqual(mapping.normalizeMappings(null),[]);
  assert.deepEqual(mapping.normalizeMappings([{tag_type:'ID3v2',source:'COMPOSER',target:7},null,'x']),[rule('ID3v2','COMPOSER','7')]);
  assert.equal(mapping.mappingsError([]),undefined);
  assert.equal(mapping.mappingsError([rule('VorbisComment','DATE','YEAR'),rule('vorbiscomment','RELEASE','year'),rule('vorbiscomment','date','Year')]),undefined,'many sources may share one target');
  assert.match(mapping.mappingsError([rule('RIFF','TITLE','NAME')]),/Mapping 1: choose a tag type/);
  assert.match(mapping.mappingsError([rule('MP4',' ','YEAR')]),/source/);
  assert.match(mapping.mappingsError([rule('MP4','DATE','')]),/target/);
  assert.match(mapping.mappingsError([rule('MP4','DA\nTE','X')]),/line breaks/);
  assert.match(mapping.mappingsError([rule('APEv2','DATE','YEAR'),rule('APEv2','date','OTHER')]),/Mapping 2: .*several targets/);
  assert.equal(mapping.mappingsError([rule('APEv2','DATE','YEAR'),rule('WMA','DATE','OTHER')]),undefined,'rules are per tag type');
  assert.equal(mapping.mappingLabel(rule('VorbisComment','DATE','YEAR')),'Vorbis Comments: DATE → YEAR');
  assert.equal(mapping.mappingLabel(rule('ID3v2','','')),'ID3v2: ? → ?');
});

test('the Mapping page sits under Tags, stores tags.mappings and validates through Options',()=>{
  const page=mapping.mappingOptionsPage();
  assert.deepEqual([page.id,page.parent,page.order],['tags-mapping','tags',30]);
  const control=page.controls[0];
  assert.deepEqual(control.paths,['tags.mappings']);
  const draft={...options.defaultSettings(),appearance:'system'};
  assert.match(control.render(draft),/No mappings are defined/);
  draft.tags.mappings=[rule('VorbisComment','DATE','YEAR'),rule('ID3v2','COMPOSER','WRITER')];
  const html=control.render(draft);
  assert.match(html,/<option value="0" selected>Vorbis Comments: DATE → YEAR<\/option>/);
  assert.match(html,/id="mapping-source" value="DATE"/);
  assert.match(html,/<option value="VorbisComment" selected>/);
  assert.match(html,/id="mapping-up" disabled/);
  assert.equal(options.validateOptions(draft,[page]),undefined);
  draft.tags.mappings.push(rule('VorbisComment','DATE','OTHER'));
  assert.deepEqual(options.validateOptions(draft,[page]).page,'tags-mapping');
});

test('mapping editor keeps typed values when adding, moving and removing rules',()=>{
  const control=mapping.mappingOptionsPage().controls[0];
  const draft={...options.defaultSettings(),appearance:'system'};
  // A minimal stand-in for the rendered page: elements record their listeners.
  let elements;
  const element=()=>({listeners:{},value:'',textContent:'',addEventListener(type,fn){this.listeners[type]=fn;},focus(){}});
  const root={querySelector:selector=>elements[selector]??(elements[selector]=element())};
  const refresh=()=>{elements={};control.bind(root,draft,refresh);};
  const fire=(selector,type,value)=>{const el=root.querySelector(selector);el.value=value;el.listeners[type]?.({target:el});};
  refresh();
  fire('#mapping-new','click');
  fire('#mapping-source','input','DATE');fire('#mapping-target','input','YEAR');
  assert.equal(root.querySelector('#mapping-list option[value="0"]').textContent,'Vorbis Comments: DATE → YEAR');
  fire('#mapping-new','click');
  fire('#mapping-tag-type','change','ID3v2');fire('#mapping-source','input','COMPOSER');fire('#mapping-target','input','WRITER');
  assert.deepEqual(draft.tags.mappings,[rule('VorbisComment','DATE','YEAR'),rule('ID3v2','COMPOSER','WRITER')]);
  fire('#mapping-up','click');
  assert.deepEqual(draft.tags.mappings.map(r=>r.source),['COMPOSER','DATE']);
  fire('#mapping-target','input','COMPOSER BY');
  fire('#mapping-remove','click');
  assert.deepEqual(draft.tags.mappings,[rule('VorbisComment','DATE','YEAR')]);
});
