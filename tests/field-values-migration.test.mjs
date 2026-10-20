import test from 'node:test';
import assert from 'node:assert/strict';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';
const {TagPanel,defaultLayout}=await loadProductionModule('tag-panel.ts');
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function page(){
  const window=new Window(),names=['window','document','HTMLElement','HTMLInputElement','HTMLTextAreaElement','MutationObserver'];
  const previous=names.map(name=>Object.getOwnPropertyDescriptor(globalThis,name));names.forEach(name=>globalThis[name]=name==='window'?window:window[name]);
  const state={values:['Loaded'],drafts:[],imports:0,exports:[]};
  const imported={panel:defaultLayout(),field_values:{lists:{COMPOSER:['Native composer']},prefer_tag_fields:{COMPOSER:true}},skipped:[]};
  const panel=new TagPanel({persistent:()=>false,values:()=>state.values,disabled:()=>false,directories:()=>({selected:[],all:[]}),draft:(...args)=>state.drafts.push(args),changed:()=>{},keep:()=>{},autocomplete:()=>undefined,notify:()=>{},
    pickOpen:async()=>'/owned/usrfields.ini',pickSave:async()=>'/owned/list.txt',exportList:async(...args)=>state.exports.push(args),importMp3tag:async()=>{state.imports++;return structuredClone(imported);}});
  const render=()=>{window.document.body.innerHTML=panel.fieldsHTML();panel.bind(window.document.body);};
  const options=field=>{window.document.querySelector(`[data-field="${field}"]`).parentElement.querySelector('button').click();return[...window.document.querySelectorAll('.combo-list [role=option]')].map(option=>option.textContent);};
  return{window,document:window.document,panel,state,render,options,close(){window.happyDOM.abort();names.forEach((name,index)=>{if(previous[index])Object.defineProperty(globalThis,name,previous[index]);else delete globalThis[name];});}};
}
test('imported per-field preferences override the global fallback independently',()=>{
  const p=page();try{
    p.panel.settings.field_values={lists:{ARTIST:['Custom artist'],COMPOSER:['Custom composer']},only_user_genres:false,prefer_tag_values:false,prefer_tag_fields:{ARTIST:true,COMPOSER:false}};p.render();
    assert.ok(!p.options('ARTIST').includes('Custom artist'));assert.ok(p.options('COMPOSER').includes('Custom composer'));
    p.panel.settings.field_values.prefer_tag_values=true;p.render();assert.ok(p.options('COMPOSER').includes('Custom composer'),'explicit false overrides global true');
    p.state.values=[''];p.render();assert.ok(p.options('ARTIST').includes('Custom artist'),'an empty field still offers custom values');assert.deepEqual(p.state.drafts,[]);
  }finally{p.close();}
});
test('Field values edits one preference without normalizing untouched imported values',async()=>{
  const p=page();try{
    p.panel.settings.field_values={lists:{ARTIST:[' Alice '],COMPOSER:['Bach']},only_user_genres:true,prefer_tag_values:false,prefer_tag_fields:{ARTIST:true}};
    p.panel.fieldValues('COMPOSER');const preference=p.document.querySelector('#fv-prefer-field');assert.ok(preference,'per-field preference must be editable');preference.value='tags';
    const field=p.document.querySelector('#fv-field');field.value='ARTIST';field.dispatchEvent(new p.window.Event('change'));
    p.document.querySelector('#fv-ok').click();await tick();assert.equal(p.panel.settings.field_values.prefer_tag_fields.COMPOSER,true);assert.equal(p.panel.settings.field_values.prefer_tag_fields.ARTIST,true);
    assert.deepEqual(p.panel.settings.field_values.lists.ARTIST,[' Alice ']);assert.equal(p.panel.settings.field_values.only_user_genres,true);
  }finally{p.close();}
});
test('using the global preference removes only the displayed field override',async()=>{
  const p=page();try{p.panel.settings.field_values.prefer_tag_fields={ARTIST:true,COMPOSER:false};p.panel.fieldValues('COMPOSER');const preference=p.document.querySelector('#fv-prefer-field');assert.ok(preference);preference.value='global';p.document.querySelector('#fv-ok').click();await tick();assert.deepEqual(p.panel.settings.field_values.prefer_tag_fields,{ARTIST:true});}finally{p.close();}
});
test('native panel import previews suggestions and applies them only on OK',async()=>{
  const p=page();try{
    p.panel.settings.field_values={lists:{GENRE:['Existing genre']},only_user_genres:true,prefer_tag_values:false};const before=structuredClone(p.panel.settings);
    p.panel.configure(true);await tick();p.document.querySelector('[data-close]').click();assert.deepEqual(p.panel.settings,before);
    p.panel.configure(true);await tick();p.document.querySelector('#tp-ok').click();await tick();
    assert.deepEqual(p.panel.settings.field_values.lists,{GENRE:['Existing genre'],COMPOSER:['Native composer']});assert.equal(p.panel.settings.field_values.prefer_tag_fields.COMPOSER,true);assert.equal(p.panel.settings.field_values.only_user_genres,true);assert.equal(p.state.imports,2);
  }finally{p.close();}
});
test('editing or appending a list line preserves untouched literal whitespace values',async()=>{
  const p=page();try{
    p.panel.settings.field_values.lists.ARTIST=[' Alice ',' ','A|B'];p.panel.fieldValues('ARTIST');
    p.document.querySelector('#fv-values').value+='\nNew artist';p.document.querySelector('#fv-ok').click();await tick();
    assert.deepEqual(p.panel.settings.field_values.lists.ARTIST,[' Alice ',' ','A|B','New artist']);
  }finally{p.close();}
});
test('empty new and preference-only fields export an explicit empty value list',async()=>{
  for(const field of ['LYRICIST','ARTIST']){const p=page();try{
    p.panel.settings.field_values.prefer_tag_fields={ARTIST:true};p.panel.fieldValues(field);p.document.querySelector('#fv-export').click();await tick();
    assert.deepEqual(p.state.exports,[['/owned/list.txt',[]]]);
  }finally{p.close();}}
});
