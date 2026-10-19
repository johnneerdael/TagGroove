import {loadProductionModule} from './helpers/load-production-module.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const panel=await loadProductionModule('tag-panel.ts');
const {KEEP,BLANK,comboOptions,choiceText,selectionState,checkState,splitValues,mnemonic,defaultLayout,defaultSettings,layoutError,settingsError,parseValueList,withCustomValue,withLists,moveField,defaultText,directoryOptions,STANDARD_GENRES}=panel;
const values=options=>options.map(option=>option.value);

test('mixed selections keep values; shared values are shown as-is',()=>{
 assert.deepEqual(selectionState(['A','A']),{value:'A',mixed:false});
 assert.deepEqual(selectionState(['A','B']),{value:'',mixed:true});
 assert.deepEqual(selectionState(['','A']),{value:'',mixed:true});
 assert.deepEqual(selectionState([]),{value:'',mixed:false});
 assert.deepEqual(splitValues('One \\\\ Two'),['One','Two']);assert.deepEqual(splitValues(''),[]);
});

test('dropdown offers keep for multi-selection, blank, then distinct selection values',()=>{
 const options=comboOptions({field:'ARTIST',values:['Beta','alpha','Beta','',`A \\\\ B`]});
 assert.deepEqual(options.slice(0,2).map(o=>[o.kind,o.value]),[['keep',KEEP],['blank',BLANK]]);
 assert.deepEqual(values(options.slice(2)),['A \\\\ B','alpha','Beta']);
 assert.ok(options.slice(2).every(o=>o.group==='selection'));
 const single=comboOptions({field:'ARTIST',values:['Solo']});
 assert.deepEqual(single.map(o=>o.kind),['blank','value']);
 assert.deepEqual(comboOptions({field:'ARTIST',values:[]}),[]);
 assert.equal(choiceText(options[0]),null);assert.equal(choiceText(options[1]),'');assert.equal(choiceText(options[2]),'A \\\\ B');
});

test('custom values follow selection values without duplicates; prefer-tag-values hides them unless empty',()=>{
 const options=comboOptions({field:'COMPOSER',values:['Bach','Bach'],custom:['Mozart','Bach','Händel']});
 assert.deepEqual(values(options.filter(o=>o.kind==='value')),['Bach','Mozart','Händel']);
 assert.deepEqual(options.filter(o=>o.group==='custom').map(o=>o.value),['Mozart','Händel']);
 const preferred=comboOptions({field:'COMPOSER',values:['Bach'],custom:['Mozart'],preferTagValues:true});
 assert.deepEqual(values(preferred.filter(o=>o.kind==='value')),['Bach']);
 const empty=comboOptions({field:'COMPOSER',values:['',''],custom:['Mozart'],preferTagValues:true});
 assert.deepEqual(values(empty),[KEEP,BLANK,'Mozart']);
});

test('genre list merges selection, user-defined and standard genres',()=>{
 const options=comboOptions({field:'genre',values:['Rock'],custom:['Shoegaze','Synthwave']});
 const genres=options.filter(o=>o.group==='genre').map(o=>o.value);
 assert.equal(options.find(o=>o.value==='Rock').group,'selection');
 assert.equal(options.filter(o=>o.value==='Shoegaze').length,1);assert.equal(options.find(o=>o.value==='Shoegaze').group,'custom');
 assert.ok(!genres.includes('Rock')&&genres.includes('Blues')&&genres.includes('Psybient'));
 assert.deepEqual(genres,[...genres].sort(new Intl.Collator(undefined,{numeric:true,sensitivity:'base'}).compare));
 assert.equal(new Set(STANDARD_GENRES).size,STANDARD_GENRES.length);
 const onlyUser=comboOptions({field:'GENRE',values:['Rock'],custom:['Shoegaze'],onlyUserGenres:true});
 assert.deepEqual(values(onlyUser),[BLANK,'Rock','Shoegaze']);
 assert.equal(comboOptions({field:'MOOD',values:['x']}).some(o=>o.group==='genre'),false);
});

test('check fields map truthy values and report mixed selections',()=>{
 assert.equal(checkState(['1','1']),'on');assert.equal(checkState(['','0']),'off');assert.equal(checkState(['1','']),'mixed');assert.equal(checkState(['false']),'off');
});

test('mnemonics mark the Alt letter and keep literal ampersands',()=>{
 assert.deepEqual(mnemonic('Tit&le'),{label:'Title',key:'L',html:'Tit<u>l</u>e'});
 assert.deepEqual(mnemonic('R&&B'),{label:'R&B',key:undefined,html:'R&amp;B'});
 assert.equal(mnemonic('Album Artist').key,undefined);
 assert.equal(mnemonic('<b>&x').html,'&lt;b&gt;<u>x</u>');
});

test('default layout matches the backend defaults and validates',async()=>{
 const rust=await readFile(new URL('../src-tauri/src/tag_panel.rs',import.meta.url),'utf8');
 const block=rust.slice(rust.indexOf('impl Default for Layout'),rust.indexOf('show_panel: true'));
 const backend=[...block.matchAll(/field\("([^"]+)", (?:"([^"]+)"|DIRECTORY), Size::(\w+)\)/g)].map(m=>[m[1],m[2]||'_DIRECTORY',m[3].toLowerCase()]);
 const layout=defaultLayout();
 assert.deepEqual(layout.fields.map(f=>[f.name,f.field,f.size]),backend);
 assert.equal(layout.fields.find(f=>f.field==='COMMENT').type,'multiline');
 assert.equal(layoutError(layout),null);assert.equal(settingsError(defaultSettings()),null);
 assert.ok(layout.fields.every(f=>f.default_value===KEEP&&f.visible));
});

test('layout validation rejects duplicates, technical fields and bad rows but allows separators',()=>{
 const layout=defaultLayout();
 assert.match(layoutError({...layout,fields:[...layout.fields,{...layout.fields[0],name:'Again'}]}),/cannot be added twice/);
 assert.match(layoutError({...layout,fields:[{...layout.fields[0],field:'_PATH'}]}),/technical/);
 assert.match(layoutError({...layout,fields:[{...layout.fields[0],field:'title'}]}),/uppercase/);
 assert.match(layoutError({...layout,fields:[{...layout.fields[0],rows:0}]}),/rows/);
 assert.match(layoutError({...layout,fields:[{...layout.fields[0],name:' '}]}),/name/);
 const separator={...layout.fields[0],name:'',field:'_SEPARATOR'};
 assert.equal(layoutError({...layout,fields:[separator,separator]}),null);
 const settings=defaultSettings();settings.field_values.lists={GENRE:['Rock','Rock']};assert.match(settingsError(settings),/duplicate/);
 settings.field_values.lists={_DIRECTORY:['x']};assert.match(settingsError(settings),/technical/);
});

test('reordering returns a new layout and rejects bad indexes',()=>{
 const layout=defaultLayout(),moved=moveField(layout,0,2);
 assert.deepEqual(moved.fields.slice(0,3).map(f=>f.field),['ARTIST','ALBUM','TITLE']);assert.equal(layout.fields[0].field,'TITLE');
 assert.throws(()=>moveField(layout,0,99),/range/);
});

test('custom value lists normalize, round-trip and stay immutable',()=>{
 assert.deepEqual(parseValueList('\uFEFFRock\r\n\r\n  Jazz \nRock\n'),['Rock','Jazz']);
 assert.deepEqual(parseValueList('Rock\r\n\r\n  Jazz \nRock\n'),['Rock','Jazz']);
 const settings=defaultSettings(),added=withCustomValue(settings,'genre',' Shoegaze ');
 assert.deepEqual(added.field_values.lists,{GENRE:['Shoegaze']});assert.deepEqual(settings.field_values.lists,{});
 assert.deepEqual(withCustomValue(added,'GENRE','Shoegaze').field_values.lists.GENRE,['Shoegaze']);
 assert.throws(()=>withCustomValue(settings,'GENRE',''),/value/);assert.throws(()=>withCustomValue(settings,'_PATH','x'),/technical/);
 const lists=withLists(added,{composer:['Bach','',' Bach'],GENRE:[]});
 assert.deepEqual(lists.field_values.lists,{COMPOSER:['Bach']});assert.equal(settingsError(lists),null);
 assert.deepEqual(JSON.parse(JSON.stringify(lists)),lists);
});

test('default values stage only configured fields',()=>{
 const field=defaultLayout().fields[0];
 assert.equal(defaultText(field),null);assert.equal(defaultText({...field,default_value:BLANK}),'');assert.equal(defaultText({...field,default_value:'Unknown'}),'Unknown');
 assert.equal(defaultText({...field,field:'_DIRECTORY',default_value:'x'}),null);
});

test('directory switcher lists distinct loaded folders and a browse entry',()=>{
 const options=directoryOptions(['/m/b','/m/a','/m/b']);
 assert.deepEqual(options.map(o=>[o.kind,o.value]),[['value','/m/a'],['value','/m/b'],['browse','Open folder…']]);
});
