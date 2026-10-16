import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';

const source=async file=>readFile(new URL(`../src/${file}`,import.meta.url),'utf8');
const moduleUrl=async(file,replacements={})=>{
  let js=ts.transpileModule(await source(file),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  for(const [from,to] of Object.entries(replacements))js=js.replace(from,to);
  return `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`;
};
const load=async file=>import(await moduleUrl(file));
const {Translator,catalogMeta}=await load('i18n/core.ts'),{DomTranslator}=await load('i18n/dom.ts');
const {controlMarkup}=await load('options-ui.ts');
const {ColumnUI,defaultLayout,setColumnNameTranslation}=await load('columns.ts'),{ActionUI}=await load('action-ui.ts');
const {TagPanel,defaultLayout:defaultPanelLayout}=await load('tag-panel.ts');
const {CoverUI}=await import(await moduleUrl('covers.ts',{"import './covers.css';":'',"'./keyboard'":JSON.stringify(await moduleUrl('keyboard.ts')),"'./cover-model'":JSON.stringify(await moduleUrl('cover-model.ts'))}));
const {PICTURE_TYPES}=await load('cover-model.ts');
const {contextMenuMarkup,RowContextMenu}=await load('row-context-menu.ts');
const {ExtendedTags}=await import(await moduleUrl('extended-tags.ts',{"import './extended-tags.css';":'',"'./tag-edits'":JSON.stringify(await moduleUrl('tag-edits.ts')),"'./row-context-menu'":JSON.stringify(await moduleUrl('row-context-menu.ts'))}));
const {ToolsUI,toolsOptionsPage}=await load('tools.ts'),{installHelperMenus}=await load('helper-menu.ts');
const {copyAsTextDialog}=await load('utils-ui.ts'),{PlaylistUI}=await load('playlist-ui.ts'),{openTagToTag}=await load('tag-tag-ui.ts');
const {offerOpenOutputs}=await load('export-ui.ts');
const {FileOpsUI,nameList}=await load('file-ops.ts');
const {copiedMessage}=await load('tag-clipboard.ts');
const {cleanupText}=await load('library.ts');
const {sourcePresentation}=await load('document-state.ts');
const {removeTagParts}=await load('remove-tag.ts');
const {Messages}=await load('messages.ts');
const {WebSourcesUI,settingsMarkup,readSettings}=await import(await moduleUrl('web-sources.ts',{"'./action-ui'":JSON.stringify(await moduleUrl('action-ui.ts'))}));
const main=await source('main.ts'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
const tagSourceParsed=ts.createSourceFile('tag-sources.ts',await source('tag-sources.ts'),ts.ScriptTarget.Latest,true);
const row=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='rowMarkup').getText(parsed);
// Execute the production row renderer with only its data dependencies stubbed.
const rowJS=ts.transpileModule(`export function render(entry){
  const esc=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const drafts=new Map([[entry.path,{TITLE:['Help']}]]),selected=new Set();
  const merged=()=>entry.fields,cellValue=()=>entry.fields.TITLE[0],editKey=()=> 'TITLE',panelKey=(_,key)=>key;
  const tableColumns=()=>[{id:'title',value:'%title%',align:'left'}];
  ${row}
  return ${JSON.stringify(main.match(/<tbody\b[^>]*><\/tbody>/)[0])}.replace('</tbody>',rowMarkup(entry,0)+'</tbody>');
}`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {render:renderRows}=await import(`data:text/javascript;base64,${Buffer.from(rowJS).toString('base64')}`);
const catalogs=Object.fromEntries(await Promise.all(['de','nl'].map(async locale=>[locale,JSON.parse(await source(`i18n/catalogs/${locale}.json`))])));
const esc=value=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function modal(p){return(title,content,buttons)=>{const dialog=p.window.document.createElement('dialog');dialog.innerHTML=`<header>${esc(title)}</header><div class="dialog-content">${content}</div><footer>${buttons}</footer>`;p.body.append(dialog);dialog.setAttribute('open','');return dialog;};}
function mainTemplate(needle,values,index=0){
  const templates=[];
  const visit=node=>{if(ts.isTemplateExpression(node)&&(typeof needle==='function'?needle(node):node.head.text.startsWith(needle)))templates.push(node);ts.forEachChild(node,visit);};visit(parsed);
  const template=templates[index];
  assert.ok(template,`Production template ${needle}`);
  const js=ts.transpileModule(`return ${template.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js)(...Object.values(values));
}
function rendererVariable(tree,name,values,owner){
  let declaration;
  const visit=node=>{if(ts.isVariableDeclaration(node)&&node.name.getText(tree)===name)declaration=node;ts.forEachChild(node,visit);};visit(tree);
  assert.ok(declaration,`Production renderer ${name}`);
  const js=ts.transpileModule(`const ${declaration.getText(tree)};return ${name};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js).call(owner,...Object.values(values));
}
function mainMessage(call,needle,index,values){
  let expression;
  const visit=node=>{if(ts.isCallExpression(node)&&node.expression.getText(parsed)===call&&node.arguments[index]?.getText(parsed).includes(needle))expression=node.arguments[index];ts.forEachChild(node,visit);};visit(parsed);
  assert.ok(expression,`Production ${needle} message`);
  const js=ts.transpileModule(`return ${expression.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js)(...Object.values(values));
}
const configurationMessage=values=>mainMessage('ask','OpenTag reloads its window',0,values);
const removalMessage=(count,tags)=>mainMessage('messages.confirm','removeTagParts',1,{removeTagParts,selected:{size:count},optionsUI:{settings:{tags}}});
function page(){
  const window=new Window();
  for(const key of ['window','document','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','MutationObserver','MouseEvent','Event'])globalThis[key]=key==='window'?window:window[key];
  const body=window.document.body;
  let translator=new Translator('en');const dom=new DomTranslator(body,()=>translator);
  return {body,window,translateTree:node=>dom.translateTree(node),translate(locale,catalog=catalogs[locale]??{}){translator=new Translator(locale,catalog);dom.apply();return translator;},close(){window.happyDOM.abort();}};
}
const memory=()=>{const data=new Map();return{getItem:key=>data.get(key)??null,setItem:(key,value)=>data.set(key,value)};};

// Happy DOM has no layout: deterministic text-based metrics verify the production
// ordering/clamping, rather than claiming native pixel/layout qualification.
function popupMetrics(p){
  const proto=p.window.HTMLElement.prototype,names=['offsetWidth','offsetHeight','getBoundingClientRect'],saved=Object.fromEntries(names.map(name=>[name,Object.getOwnPropertyDescriptor(proto,name)]));
  const isPopup=node=>node.matches('.column-menu,.cover-menu,.helper-menu,.helper-submenu,.cover-submenu,.menu-list,.group-submenu > div');
  const size=node=>{const labels=[...node.querySelectorAll('button,label,.context-heading')].filter(el=>!el.closest('[hidden]')).map(el=>el.textContent.trim());return{width:Math.min(650,Math.max(80,...labels.map(label=>label.length*9+24))),height:Math.min(400,Math.max(40,labels.reduce((sum,label)=>sum+20+Math.floor(label.length/24)*8,8)))};};
  for(const [name,axis] of [['offsetWidth','width'],['offsetHeight','height']])Object.defineProperty(proto,name,{configurable:true,get(){return isPopup(this)?size(this)[axis]:saved[name]?.get?.call(this)??0;}});
  Object.defineProperty(proto,'getBoundingClientRect',{configurable:true,writable:true,value:function(){if(!isPopup(this))return saved.getBoundingClientRect.value.call(this);const{width,height}=size(this),left=Number.parseFloat(this.style.left)||(this.parentElement?.classList.contains('group-submenu')?(this.parentElement.classList.contains('open-left')?680-width:680):0),top=Number.parseFloat(this.style.top)||0;return{left,top,width,height,right:left+width,bottom:top+height};}});
  Object.defineProperty(p.window,'innerWidth',{value:800,configurable:true});Object.defineProperty(p.window,'innerHeight',{value:600,configurable:true});
  return{size,restore(){for(const name of names)if(saved[name])Object.defineProperty(proto,name,saved[name]);else delete proto[name];}};
}

test('production popups localize synchronously before right/bottom placement for DE/NL and long custom text',()=>{
  const long=Object.fromEntries(['Copy','Copy field','Cut field','Paste field','Customize columns…','Add cover…','Manage history…','Customize tag panel…'].map(key=>[key,`${key} ${'long translated label '.repeat(4)}`]));
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',long]]){
    const p=page(),metrics=popupMetrics(p);
    try{
      const t=p.translate(locale,catalog),translateNow=node=>p.translateTree(node);
      const placed=(menu,label,margin=4,right=800-margin)=>{assert.ok(menu.textContent.includes(t.t(label)),menu.textContent);const size=metrics.size(menu);assert.equal(Number.parseFloat(menu.style.left),right-size.width);assert.equal(Number.parseFloat(menu.style.top),600-size.height-margin);assert.ok(Number.parseFloat(menu.style.left)+size.width<=800-margin);assert.ok(Number.parseFloat(menu.style.top)+size.height<=600-margin);};
      const table=p.window.document.createElement('table');table.innerHTML='<tbody><tr data-path="Help"><td>Help</td></tr></tbody>';p.body.append(table);
      const rows=new RowContextMenu({translateNow,prepare:()=>{},items:()=>[{label:'Copy',id:'copy'}],activate:()=>{}});rows.open(table.querySelector('tr'),795,595);placed(p.body.querySelector('.row-context-menu'),'Copy');rows.close();
      const columns=new ColumnUI({translateNow,layout:defaultLayout});columns.headerMenu(795,595,()=>{});placed(p.body.querySelector('.column-menu'),'Customize columns…');columns.closeMenu();
      const covers=new CoverUI({translateNow,selection:()=>[{path:'Help',covers:[]}],displayed:()=>({path:'Help',covers:[],index:0}),reason:()=>'',clipboard:()=>false,mac:false});covers.openMenu(795,595,false);placed(p.body.querySelector('.cover-menu'),'Add cover…');covers.closeMenu();
      const extended=new ExtendedTags({translateNow,selection:()=>['Help'],listed:()=>['Help'],fields:()=>({TITLE:['Help']}),describe:()=>({name:'Help'}),modal:modal(p),coverMarkup:()=>'',bindCover:()=>{}});extended.show();p.body.querySelector('.extended-row').dispatchEvent(new p.window.MouseEvent('contextmenu',{bubbles:true,clientX:795,clientY:595}));placed(p.body.querySelector('.extended-menu'),'Cut field');p.body.querySelector('.extended-dialog').close();
      p.body.insertAdjacentHTML('beforeend','<div><input id="popup-history" data-helper="history" data-history="probe" data-history-manage></div>');installHelperMenus({translateNow,storage:memory(),fieldNames:()=>[],panelFields:()=>[]});const helper=p.body.querySelector('#popup-history').parentElement.querySelector('.helper-button');helper.getBoundingClientRect=()=>({left:770,right:790,top:560,bottom:580,width:20,height:20});helper.click();placed(p.body.querySelector('.helper-menu'),'Manage history…',4,790);p.body.querySelector('.helper-menu').remove();
      const panel=new TagPanel({translateNow,values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]})});p.body.innerHTML=panel.titleHTML()+panel.fieldsHTML();panel.bind(p.body);const details=p.body.querySelector('.panel-menu');details.getBoundingClientRect=()=>({left:780,right:800,top:570,bottom:590,width:20,height:20});details.open=true;details.dispatchEvent(new p.window.Event('toggle'));placed(details.querySelector('.menu-list'),'Customize tag panel…',8);
      p.body.innerHTML='<details class="group-submenu"><summary translate="no">Help</summary><div><button>Copy field</button></div></details>';new ActionUI({translateNow}).bindMenu(p.body);const submenu=p.body.querySelector('details');submenu.open=true;submenu.dispatchEvent(new p.window.Event('toggle'));assert.equal(submenu.querySelector('button').textContent,t.t('Copy field'));assert.ok(submenu.classList.contains('open-left'));assert.ok(submenu.querySelector('div').getBoundingClientRect().right<=800);
    }finally{metrics.restore();p.close();}
  }
});

test('the actual Language options preserve authored catalog names in System default with existing catalog keys',async()=>{
  const custom={'System default ({nameOf})':'Default catalog: {nameOf}',Help:'Assistance',Title:'Heading'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const name of ['Help','Title']){
    const p=page();
    try{
      const user={'@meta':{name}},languages=[{locale:'en',name:'English',catalog:{},source:'bundled'},{...catalogMeta(user,'fr.json'),catalog:user,source:'user',file:'fr.json'}];
      const provider=`data:text/javascript;base64,${Buffer.from(`export const availableLanguages=()=>${JSON.stringify(languages)};export const systemLanguage=()=> 'fr';`).toString('base64')}`;
      const {languageOptionsPage}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(provider)}));
      const options=languageOptionsPage({}),control=options.controls[0];
      p.body.innerHTML=controlMarkup(control,{pages:{language:{language:'system'}}});const t=p.translate(locale,catalog);
      assert.equal(p.body.querySelector('option[value="system"]').textContent,t.t('System default ({nameOf})',{nameOf:name}));
      assert.equal(p.body.querySelector('option[value="fr"]').textContent,`${name} (fr.json)`);
      assert.equal(p.body.querySelector('select').value,'system');
    }finally{p.close();}
  }
});

test('application mutations equal to previous translations become fresh sources at observer and synchronous boundaries',async()=>{
  const custom={'Previewing…':'Preview',Preview:'Inspect',Inspect:'Wrong'};
  for(const boundary of ['observer','manual','language-off']){
    const p=page();
    try{
      p.body.innerHTML='<button title="Previewing…">Previewing…</button><input type="button" value="Previewing…"><textarea title="Previewing…">Help</textarea>';
      p.translate('en',custom);const button=p.body.querySelector('button'),input=p.body.querySelector('input'),textarea=p.body.querySelector('textarea');
      for(let pass=0;pass<2;pass++)p.translateTree(p.body);
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(button.textContent,'Preview');assert.equal(button.title,'Preview');assert.equal(input.getAttribute('value'),'Preview');
      // Multiple real app writes must be invalidated as a batch, before any own output is produced.
      button.firstChild.data='Other';button.firstChild.data='Preview';button.title='Other';button.title='Preview';input.setAttribute('value','Preview');textarea.title='Preview';
      if(boundary==='manual')p.translateTree(p.body);
      if(boundary==='language-off'){
        p.translate('en');assert.equal(button.textContent,'Preview');assert.equal(button.title,'Preview');assert.equal(input.getAttribute('value'),'Preview');
        p.translate('en',custom);
      }
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(button.textContent,'Inspect');assert.equal(button.title,'Inspect');assert.equal(input.getAttribute('value'),'Inspect');assert.equal(textarea.title,'Inspect');assert.equal(textarea.value,'Help');
      p.translateTree(p.body);await p.window.happyDOM.waitUntilComplete();assert.equal(button.textContent,'Inspect');
      p.translate('en');assert.equal(button.textContent,'Preview');assert.equal(button.title,'Preview');assert.equal(input.getAttribute('value'),'Preview');
    }finally{p.close();}
  }
});
test('production converter controls translate their pending and ready sources once',async()=>{
  const p=page();
  try{
    p.body.innerHTML='<dialog open><button id="apply"></button><button id="preview"></button><button id="close"></button></dialog>';
    const dialog=p.body.querySelector('dialog'),previewButton=dialog.querySelector('#preview'),review={pending:true,canApply:()=>false};
    const sync=rendererVariable(parsed,'syncControls',{paths:()=>['Help'],applying:false,picking:false,busy:false,dirtySelected:()=>false,rename:true,apply:dialog.querySelector('#apply'),readRequest:()=>({}),review,context:()=>'',previewButton,closeButton:dialog.querySelector('#close'),dialog});
    sync();p.translate('en',{'Previewing…':'Preview',Preview:'Inspect',Inspect:'Wrong'});p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();assert.equal(previewButton.textContent,'Preview');
    review.pending=false;sync();p.translateTree(dialog);await p.window.happyDOM.waitUntilComplete();assert.equal(previewButton.textContent,'Inspect');
    review.pending=true;sync();await p.window.happyDOM.waitUntilComplete();assert.equal(previewButton.textContent,'Preview');
    p.translate('en');assert.equal(previewButton.textContent,'Previewing…');
  }finally{p.close();}
});
for(const locale of ['de','nl'])test(`${locale}: provider source locations and names stay raw while unavailable fallbacks translate`,()=>{
  const p=page(),t=new Translator(locale,catalogs[locale]);
  try{
    let state={uri:'Help',name:'Title',writable:true,persisted:true,state:'ready',error:null};
    const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='sourceMarkup');
    const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const access={states:{fixture:state},refreshError:null};
    const values={active:()=>({path:'fixture'}),providerEntry:()=>true,documentAccess:access,sourcePresentation,sourcePolicy:()=>({mutationReason:'',unsupportedReason:''}),esc,workingMetadataNotice:()=>'',publicMessage:value=>value,button:(_op,label)=>`<button>${label}</button>`};
    const render=new Function(...Object.keys(values),`${js};return sourceMarkup;`)(...Object.values(values));
    p.body.innerHTML=render('files');p.translate(locale);
    assert.equal(p.body.querySelector('.source-location').textContent,'Help');assert.equal(p.body.querySelector('.source-location').title,'Help');assert.equal(p.body.querySelector('.source-access > div > span').textContent,'Title');
    assert.equal(p.body.querySelector('.source-access strong').textContent,t.t('Source writable'));
    assert.equal(p.body.querySelector('button').textContent,t.t('Save a copy…'));
    const locationHeader=()=>{
      const locationView=rendererVariable(parsed,'locationView',{entry:{path:'fixture'},providerEntry:()=>true,providerState:access.states.fixture,sourcePresentation,location:'Help',locationIsData:true});
      return rendererVariable(parsed,'locationMarkup',{esc,locationView});
    };
    p.body.innerHTML=locationHeader();p.translate(locale);assert.equal(p.body.querySelector('.path').textContent,'Help');assert.equal(p.body.querySelector('.path').title,'Help');
    access.states.fixture={...state,uri:'Source location unavailable',name:'Provider document'};p.body.innerHTML=render('files');p.translate(locale);
    assert.equal(p.body.querySelector('.source-location').textContent,'Source location unavailable');assert.equal(p.body.querySelector('.source-access > div > span').textContent,'Provider document');
    p.body.innerHTML=locationHeader();p.translate(locale);assert.equal(p.body.querySelector('.path').textContent,'Source location unavailable');
    for(const next of [{...state,uri:'',name:''},undefined]){
      access.states.fixture=next;p.body.innerHTML=render('files');p.translate(locale);
      assert.equal(p.body.querySelector('.source-location').textContent,t.t('Source location unavailable'));assert.equal(p.body.querySelector('.source-location').title,t.t('Source location unavailable'));assert.equal(p.body.querySelector('.source-access > div > span').textContent,t.t('Provider document'));
      p.body.innerHTML=locationHeader();p.translate(locale);assert.equal(p.body.querySelector('.path').textContent,t.t('Source location unavailable'));assert.equal(p.body.querySelector('.path').title,t.t('Source location unavailable'));
    }
    for(const [location,locationIsData,expected] of [['No folder loaded',false,t.t('No folder loaded')],['2 added locations',false,t.t('{length} added locations',{length:2})],['No folder loaded',true,'No folder loaded'],['2 added locations',true,'2 added locations'],['Help',true,'Help']]){
      const locationView=rendererVariable(parsed,'locationView',{entry:undefined,providerEntry:()=>false,providerState:undefined,sourcePresentation,location,locationIsData});
      p.body.innerHTML=rendererVariable(parsed,'locationMarkup',{esc,locationView});p.translate(locale);
      assert.equal(p.body.querySelector('.path').textContent,expected);assert.equal(p.body.querySelector('.path').title,expected);
    }
  }finally{p.close();}
});
test('the real language reload retains an active catalog on command failure and replaces it on an empty successful snapshot',async()=>{
  const p=page(),storage=Object.getOwnPropertyDescriptor(globalThis,'localStorage');
  Object.defineProperty(globalThis,'localStorage',{configurable:true,value:p.window.localStorage});
  try{
    const english=JSON.parse(await source('i18n/catalogs/en.json'));
    const jsonModule=value=>JSON.stringify(`data:text/javascript;base64,${Buffer.from(`export default ${JSON.stringify(value)};`).toString('base64')}`);
    const indexUrl=await moduleUrl('i18n/index.ts',{"'./core'":JSON.stringify(await moduleUrl('i18n/core.ts')),"'./dom'":JSON.stringify(await moduleUrl('i18n/dom.ts')),"'./catalogs/en.json'":jsonModule(english),"'./catalogs/nl.json'":jsonModule(catalogs.nl),"'./catalogs/de.json'":jsonModule(catalogs.de)});
    const i18n=await import(indexUrl),{languageOptionsPage,languageSetting}=await import(await moduleUrl('language-options.ts',{"'./i18n'":JSON.stringify(indexUrl)}));
    i18n.setUserCatalogs([{file:'fr.json',catalog:{...english,'@meta':{name:'Français'},'Save tags':'Enregistrer'}}]);i18n.setLanguage('fr');
    const languagePage=languageOptionsPage({}),controls=languagePage.controls;
    let failed=true,renders=0;
    const declaration=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='applyLanguage');
    const js=ts.transpileModule(declaration.getText(parsed),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const values={desktop:true,invoke:async command=>{assert.equal(command,'language_catalogs');if(failed)throw new Error('languages: access denied');return {catalogs:[],errors:[]};},languagePage,setUserCatalogs:i18n.setUserCatalogs,setLanguage:i18n.setLanguage,languageSetting,optionsUI:{page:()=>({language:'fr'})},publicMessage:error=>error.message,applyNotificationArea:()=>{},render:()=>{renders++;}};
    const apply=new Function(...Object.keys(values),`let shownTitle='';${js};return applyLanguage;`)(...Object.values(values));
    await apply();assert.equal(i18n.currentLanguage(),'fr');assert.equal(i18n.translate('Save tags'),'Enregistrer');assert.equal(renders,0);
    assert.ok(controls[0].choices.some(([value])=>value==='fr'));
    p.body.innerHTML=controls.map(controlMarkup).join('');assert.ok(p.body.textContent.includes('languages: access denied'));
    failed=false;await apply();assert.equal(i18n.currentLanguage(),'en');assert.equal(i18n.translate('Save tags'),'Save tags');assert.equal(renders,1);
    assert.ok(!controls[0].choices.some(([value])=>value==='fr'));
    p.body.innerHTML=controls.map(controlMarkup).join('');assert.ok(!p.body.textContent.includes('access denied'));
  }finally{if(storage)Object.defineProperty(globalThis,'localStorage',storage);else delete globalThis.localStorage;p.close();}
});
test('web-source result headers and search labels remain authored while application table/search controls translate',()=>{
  const custom={ARTIST:'Performer',Artist:'Performer',Help:'Assistance',Title:'Name',Search:'Find','Artist + Album':'Performer + Record','Search results':'Found records',Preview:'Inspect','Release artwork':'Release image','More results':'Show more','{length} of {total} shown':'{length} / {total} displayed'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();
    try{
      const webColumns=['ARTIST','Help','Title','MiXeD < & >'],values=['Help','Title','My  Files\nTitle','< & >'];
      const results=[{artist:'Help',title:'Title',year:'2026',country:'Help',label:'Help',catalog_number:'Title',format:'Help',track_count:1,score:100,id:'category/disc',url:'Help'}];
      const options={results,chosen:0,html:esc,rowAttributes:()=> 'data-index="0" aria-selected="true" tabindex="0"',webColumns,webRows:[{values}],total:2};
      const render=rendererVariable(tagSourceParsed,'resultMarkup',{...options,web:true});
      p.body.innerHTML=render();const t=p.translate(locale,catalog);
      assert.deepEqual([...p.body.querySelectorAll('th')].map(cell=>cell.textContent),webColumns);
      assert.deepEqual([...p.body.querySelectorAll('td')].map(cell=>[cell.textContent,cell.title]),values.map(value=>[value,value]));
      assert.equal(p.body.querySelector('table').getAttribute('aria-label'),t.t('Search results'));
      assert.equal(p.body.querySelector('tr[data-index]').getAttribute('aria-selected'),'true');
      p.translate('en');assert.deepEqual([...p.body.querySelectorAll('th')].map(cell=>cell.textContent),webColumns);
      const web={search_by:[{label:'ARTIST'},{label:'Search'},{label:'MiXeD label'},{label:'Search',default_label:true},{label:'Artist + Album',default_label:true}]};
      p.body.innerHTML=rendererVariable(tagSourceParsed,'searchFields',{web,webValues:['Help','Title'],html:esc});p.translate(locale,catalog);
      assert.deepEqual([...p.body.querySelectorAll('label > span')].map(span=>span.textContent),['ARTIST','Search','MiXeD label',t.t('Search'),t.t('Artist + Album')]);
      assert.equal(p.body.querySelector('#ts-web-0').value,'Help');assert.equal(p.body.querySelector('#ts-web-1').value,'Title');
      assert.equal(p.body.querySelector('#ts-search').textContent,t.t('Search'));
      for(const provider of ['musicbrainz','discogs','freedb']){
        const render=rendererVariable(tagSourceParsed,'resultMarkup',{...options,web:false,provider,freedb:provider==='freedb',freedbId:()=>({category:'Help',disc:'Title'})});
        p.body.innerHTML=render();p.translate(locale,catalog);
        assert.ok([...p.body.querySelectorAll('th')].some(cell=>cell.textContent===t.t('Artist')));
        assert.equal(p.body.querySelector('tbody td[title="Help"]').textContent,'Help');
        assert.equal(p.body.querySelector('#ts-more').textContent,t.t('More results'));
      }
      for(const web of [false,true]){
        const chosen=rendererVariable(tagSourceParsed,'chosenMarkup',{results,chosen:0,web,freedb:false,html:esc,preview:()=>'<button class="ts-preview">Preview</button>'});
        p.body.innerHTML=chosen();p.translate(locale,catalog);
        assert.equal(p.body.querySelector('.ts-url > span[translate="no"]').textContent,'Help');
        assert.equal(p.body.querySelector('.ts-preview').textContent,t.t('Preview'));
      }
      p.body.innerHTML='<section class="ts-cover"></section>';
      const renderCover=rendererVariable(tagSourceParsed,'renderCover',{release:{covers:[{kind:'Help',url:'https://example.invalid/cover.jpg'}]},dialog:p.body,coverIndex:0,$:selector=>p.body.querySelector(selector),storage:{get:()=>null},saveCoverKey:'fixture',syncReview:()=>{},coverTemplate:()=> 'Help',html:esc,loadImage:()=>{},extractCover:()=>{}});
      renderCover();p.translate(locale,catalog);
      assert.equal(p.body.querySelector('code').textContent,'Help');
      assert.equal(p.body.querySelector('#ts-cover-choice').value,'0');
      assert.equal(p.body.querySelector('#ts-cover-choice option').textContent,'1: Help');
    }finally{p.close();}
  }
});
test('source-authored settings prose remains authoritative while OpenTag controls and errors translate',async()=>{
  const custom={Save:'Store',Cancel:'Back',Help:'Assistance',Title:'Name','Select files first.':'Choose files','Save tags':'Write tags','{title} needs a number.':'A number is required for {title}.','This tag source has no settings.':'There are no settings.'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();
    try{
      const schema={key:'fixture',title:'Original source title',settings:[
        {type:'heading',key:'heading',title:'Original heading'},
        {type:'separator',key:'separator',title:'Original note'},
        {type:'bool',key:'flag',title:'Original flag',default:true},
        {type:'number',key:'count',title:'Original number',default:5},
        {type:'string',key:'text',title:'Original text',default:'Title'},
        {type:'string',key:'choice',title:'Original choice',choices:['Help','Title'],default:'Help'},
        {type:'string',key:'Help',default:'Title'},
      ],localizations:{[locale]:[
        {key:'fixture',title:'Save'},{key:'heading',title:'Help'},{key:'separator',title:'Title'},
        {key:'flag',title:'Cancel',description:'Select files first.'},
        {key:'count',title:'Help',description:'Save tags'},
        {key:'text',title:'Title',description:'Help'},{key:'choice',title:'Save',description:'Title'},
      ]}};
      let form={schema,values:{}},saved;
      const ui=new WebSourcesUI({invoke:async(command,args)=>{if(command==='web_source_settings')return form;assert.equal(command,'save_web_source_settings');saved=args.values;form={schema,values:saved};},fail:error=>{throw error;}});
      const entry={id:'fixture',menu:'Fallback name'};
      p.window.document.documentElement.lang=locale;await ui.settings(entry);const t=p.translate(locale,catalog);
      const dialog=p.body.querySelector('dialog');
      assert.equal(dialog.querySelector('header').textContent,'Save');
      assert.equal(p.body.querySelector('#'+dialog.getAttribute('aria-labelledby')).textContent,'Save');
      assert.equal(dialog.querySelector('.ws-heading').textContent,'Help');assert.equal(dialog.querySelector('.ws-note').textContent,'Title');
      assert.equal(dialog.querySelector('.action-check').textContent.trim(),'Cancel');
      assert.deepEqual([...dialog.querySelectorAll('.field > span')].map(node=>node.textContent),['Help','Title','Save','Help']);
      assert.deepEqual([...dialog.querySelectorAll('small')].map(node=>node.textContent),['Select files first.','Save tags','Help','Title']);
      assert.equal(dialog.querySelector('[data-close]').textContent,t.t('Cancel'));assert.equal(dialog.querySelector('#ws-save').textContent,t.t('Save'));
      assert.deepEqual([...dialog.querySelectorAll('option')].map(option=>option.textContent),['Help','Title']);
      assert.equal(dialog.querySelector('[data-setting="2"]').checked,true);assert.equal(dialog.querySelector('[data-setting="3"]').value,'5');
      assert.equal(dialog.querySelector('[data-setting="4"]').value,'Title');assert.equal(dialog.querySelector('[data-setting="5"]').value,'Help');
      dialog.querySelector('[data-setting="3"]').value='';dialog.querySelector('#ws-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.equal(saved,undefined);assert.equal(dialog.querySelector('.dialog-error').textContent,t.t('{title} needs a number.',{title:'Help'}));
      assert.equal(dialog.querySelector('.ws-heading').textContent,'Help');assert.equal(dialog.querySelector('#ws-save').textContent,t.t('Save'));
      dialog.querySelector('[data-setting="3"]').value='7';dialog.querySelector('[data-setting="4"]').value='Help';dialog.querySelector('[data-setting="5"]').value='Title';
      dialog.querySelector('#ws-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved,{flag:true,count:7,text:'Help',choice:'Title',Help:'Title'});
      await ui.settings(entry);p.translate(locale,catalog);assert.equal(p.body.querySelector('[data-setting="5"]').value,'Title');
      p.body.querySelector('[data-close]').click();form={schema:{...schema,settings:[]},values:{}};
      await ui.settings(entry);p.translate(locale,catalog);
      assert.equal(p.body.querySelector('header').textContent,'Save');assert.equal(p.body.querySelector('.dialog-content p').textContent,t.t('This tag source has no settings.'));
    }finally{p.close();}
  }
});

test('web-source settings save and redraw preserve choice tokens under bundled and custom catalogs',async()=>{
  const custom={Help:'Assistance',Title:'Nom','FORMAT STRING':'Format local','Select files first.':'Choisir des fichiers','Save':'Enregistrer'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const p=page();
    try{
      const schema={key:'fixture',settings:[{type:'string',key:'choice',title:'Format string',description:'Select files first.',choices:['Help','Title'],default:'Help'}]};
      let values={choice:'Title'},saved;
      const ui=new WebSourcesUI({invoke:async(command,args)=>{
        if(command==='web_source_settings')return{schema,values};
        assert.equal(command,'save_web_source_settings');saved=structuredClone(args.values);values=saved;
      },fail:error=>{throw error;}}),entry={id:'fixture',menu:'Source'};
      p.window.document.documentElement.lang=locale;
      await ui.settings(entry);const t=p.translate(locale,catalog),select=p.body.querySelector('select');
      assert.equal(select.value,'Title');assert.deepEqual([...select.options].map(option=>option.textContent),['Help','Title']);
      assert.equal(p.body.querySelector('label > span').textContent,'Format string');
      assert.equal(p.body.querySelector('small').textContent,'Select files first.');
      assert.equal(p.body.querySelector('#ws-save').textContent,t.t('Save'));
      select.value='Help';p.body.querySelector('#ws-save').click();await p.window.happyDOM.waitUntilComplete();
      assert.deepEqual(saved,{choice:'Help'});assert.equal(p.body.querySelector('dialog'),null);
      await ui.settings(entry);p.translate(locale,catalog);
      assert.equal(p.body.querySelector('select').value,'Help');
      assert.deepEqual([...p.body.querySelectorAll('option')].map(option=>option.textContent),['Help','Title']);
    }finally{p.close();}
  }
});

test('structured confirmations translate source parts once through Messages and the native ask wrapper',async()=>{
  const first='Replace the current options, action groups, export configurations, columns, tag panel, favorite directories and remembered lists with {path}?';
  const reload='OpenTag reloads its window afterwards. The Discogs access token is not part of a configuration file.';
  const translator=new Translator('en',{[first]:'Save tags',[reload]:'Save tags','Save tags':'Write tags','Save tags Save tags':'Incorrect second translation',
    'Remove tags from {count} selected files?':'Save tags','MP3, TTA: {kinds}':'Save tags','APE, MPC, WavPack, TAK, OptimFROG: {kinds}':'Save tags','Other formats: all tags':'Save tags',
    'Save tags Save tags Save tags Save tags':'Incorrect second translation',Yes:'Save tags',No:'Keep tags','Load configuration':'Save tags','Remove tag':'Save tags'});
  const calls=[],nativeSource=(await source('i18n/dialog.ts')).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,'');
  const js=ts.transpileModule(nativeSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  const ask=new Function('dialog','t','translate','translationActive',`${js}\nreturn ask;`)({ask:async(...args)=>{calls.push(args);return false;}},key=>translator.t(key),text=>translator.translate(text),()=>true);
  await ask(configurationMessage({translate:text=>translator.translate(text),basename:path=>path,path:'Help  Title.json',drafts:{size:0}}),'Load configuration');
  assert.equal(calls.at(-1)[0],'Save tags\n\nSave tags');
  assert.deepEqual(calls.at(-1)[1],{title:'Save tags',okLabel:'Save tags',cancelLabel:'Keep tags'});
  const messages=new Messages({values:()=>({}),ask:(text,title)=>ask(text,title)}),tags={mpeg:{remove_id3v1:true},ape:{remove_ape:true}};
  await messages.confirm('confirm_remove_tags',removalMessage(2,tags),'Remove tag');
  assert.equal(calls.at(-1)[0],'Save tags\n\nSave tags\nSave tags\nSave tags');
});

for(const locale of ['de','nl']){
  test(`${locale}: post-export filenames and paths remain verbatim while prompt controls translate`,async()=>{
    const p=page();
    try{
      const opened=[],host={modal:modal(p),modalError:(_dialog,error)=>{throw error;},open:async paths=>{opened.push(paths);},reveal:async()=>{},fileManager:'Finder'};
      for(const output of ['Help','Title']){
        const done=offerOpenOutputs(host,[output],'Save tags');const t=p.translate(locale);
        assert.equal(p.body.querySelector('strong').textContent,output);
        assert.equal(p.body.querySelector('p').textContent,t.translate('Save tags.'));
        assert.equal(p.body.querySelector('#open-output-run').textContent,t.t('Open'));
        p.body.querySelector('#open-output-run').click();await p.window.happyDOM.waitUntilComplete();await done;
        assert.deepEqual(opened.at(-1),[output]);p.body.replaceChildren();
      }
      const outputs=['Help','My  Files/Title'],done=offerOpenOutputs(host,outputs,'Save tags');const t=p.translate(locale),select=p.body.querySelector('select');
      assert.deepEqual([...select.options].map(option=>option.textContent),['Help — Help','Title — My  Files']);
      assert.deepEqual([...select.options].map(option=>option.title),outputs);
      assert.equal(select.getAttribute('aria-label'),t.t('Written files'));
      select.value='1';p.body.querySelector('#open-output-run').click();await p.window.happyDOM.waitUntilComplete();await done;
      assert.deepEqual(opened.at(-1),[outputs[1]]);
    }finally{p.close();}
  });
  test(`${locale}: web-source choice labels and saved tokens stay verbatim while settings UI translates`,()=>{
    const p=page();
    try{
      const choices=['Help','Title','My  Files','A&B"<'],schema={key:'fixture',settings:[{type:'string',key:'choice',title:'Format string',description:'Select files first.',choices,default:'Help'}]};
      p.body.innerHTML=settingsMarkup({schema,values:{choice:'Title'}},locale);
      const t=p.translate(locale),select=p.body.querySelector('select');
      assert.deepEqual([...select.options].map(option=>option.textContent),choices);
      assert.deepEqual([...select.options].map(option=>option.value),choices);
      assert.equal(select.value,'Title');assert.deepEqual(readSettings(schema,p.body),{choice:'Title'});
      assert.equal(p.body.querySelector('label > span').textContent,'Format string');
      assert.equal(p.body.querySelector('small').textContent,'Select files first.');
      for(const choice of choices){select.value=choice;assert.deepEqual(readSettings(schema,p.body),{choice});}
      p.translate('en');assert.equal(select.value,choices.at(-1));assert.deepEqual([...select.options].map(option=>option.textContent),choices);
    }finally{p.close();}
  });
  test(`${locale}: saved tag-source cover notes translate their scaffold and retain the returned path`,async()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      for(const path of ['Help','Title','/My  Files/Help\nTitle.jpg']){
        p.body.innerHTML='<dialog open><p class="ts-cover-note" role="status"></p></dialog>';
        const dialog=p.body.querySelector('dialog'),note=dialog.querySelector('p'),calls=[];
        const extract=rendererVariable(tagSourceParsed,'extractCover',{release:{artist:'Artist',title:'Album',covers:[{url:'https://example.invalid/cover.png'}]},coverIndex:0,working:false,dialog,$:selector=>dialog.querySelector(selector),error:error=>{throw error;}},{host:{pickSave:async()=>path,invoke:async(command,args)=>{calls.push([command,args]);return path;}}});
        await extract();assert.equal(note.textContent,`Saved to ${path}`);p.translate(locale);
        assert.notEqual(note.textContent,`Saved to ${path}`);
        assert.equal(note.textContent,t.t('Saved to {outputPath}',{outputPath:path}));
        assert.deepEqual(calls,[['tag_source_save_cover',{url:'https://example.invalid/cover.png',path}]]);
        p.translate('en');assert.equal(note.textContent,`Saved to ${path}`);
      }
    }finally{p.close();}
  });
  test(`${locale}: copied and staged tag statuses translate optional UI sentences and retain numeric count guards`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      for(const count of [1,2])for(const cut of [false,true])for(const covers of [0,1,2])for(const failed of [0,1,2]){
        const clipboard={items:Array.from({length:count},()=>({artwork:Array.from({length:covers},()=>({}))}))};
        const source=copiedMessage(clipboard,cut,failed),art=covers?` ${t.t(`with {covers} cover${count*covers===1?'':'s'}`,{covers:count*covers})}`:'';
        const missingNote=failed?` ${t.t(`Artwork of {failed} file${failed===1?'':'s'} could not be copied.`,{failed})}`:'';
        const expected=t.t(`${cut?'Cut':'Copied'} tags of {files} file${count===1?'':'s'}{art}.{missingNote}`,{files:count,art,missingNote});
        p.body.textContent=source;p.translate(locale);assert.notEqual(p.body.textContent,source);assert.equal(p.body.textContent,expected);
        for(const restricted of [false,true]){
          const statusMessage=()=>source;
          const staged=mainTemplate(node=>node.getText(parsed).includes('new Set([...updates,...coverUpdates]'),{done:statusMessage,statusMessage,updates:[{path:'Help'}],coverUpdates:[],sourcePolicy:()=>({mutationReason:restricted?'read-only':''})});
          p.body.textContent=staged;p.translate(locale);
          const key=`{statusMessage} ${restricted?'Use Save a copy to keep these edits.':'Save tags to write them.'}`;
          assert.equal(p.body.textContent,t.t(key,{statusMessage:expected}));
        }
      }
      assert.equal(t.translate('Saved tags: Help files'),'Saved tags: Help files');
      assert.equal(t.translate('1 of Help tracks have a file. Saving writes the checked fields immediately.'),'1 of Help tracks have a file. Saving writes the checked fields immediately.');
    }finally{p.close();}
  });
  test(`${locale}: cleanup and index summaries translate UI phrases while bounded filename lists preserve raw values`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      for(const cancelled of [false,true])for(const removed of [0,1,2]){
        const source=cleanupText({checked:2,removed,cancelled});p.body.textContent=source;p.translate(locale);assert.notEqual(p.body.textContent,source);
        const removalSummary=t.t('removed {plural}',{plural:t.t(`{count} orphaned entr${removed===1?'y':'ies'}`,{count:removed})});
        assert.equal(p.body.textContent,t.t(cancelled?'Cleanup cancelled after {plural}; {removalSummary}.':'Cleanup checked {plural} and {removalSummary}.',{plural:t.t('{count} files',{count:2}),removalSummary}));
      }
      const indexText=rendererVariable(tagSourceParsed,'indexText',{directory:draft=>draft.folder});
      for(const discs of [1,2])for(const directory of ['Help','/My  Files/Title']){
        const source=indexText({discs,directory},{folder:'Help'});p.body.textContent=source;p.translate(locale);assert.notEqual(p.body.textContent,source);
        const key=directory==='Help'?'Text index: {discSummary} from this folder.':'The text index ({discSummary}) was built for {directory}. Build it again for this folder.';
        assert.equal(p.body.textContent,t.t(key,{discSummary:t.t(`{discs} disc${discs===1?'':'s'}`,{discs}),directory}));
      }
      const names=['Help','My  Files\nTitle'];const source=nameList([...names,'Other'],2);p.body.textContent=source;p.translate(locale);
      assert.equal(p.body.textContent,t.t('{names} and {length} more',{names:names.join(', '),length:1}));
      for(const field of ['TRACK','TOTALTRACKS','DISCNUMBER','TOTALDISCS']){
        p.body.textContent=`${field} requires a 16-bit unsigned number`;p.translate(locale);
        assert.equal(p.body.textContent,t.t('{field} requires a 16-bit unsigned number',{field}));
      }
    }finally{p.close();}
  });
  test(`${locale}: web-source names remain verbatim in enabled tooltips while disabled reasons translate`,()=>{
    const p=page();let reason='';
    try{
      const ui=new WebSourcesUI({reason:()=>reason,last:()=>null,shortcut:'Ctrl+I'});
      ui.listing.sources=['Help','Title'].map(name=>({id:name,menu:name,name,search_by:[],direct:false}));
      p.body.innerHTML=ui.menuHTML();const t=p.translate(locale);
      assert.deepEqual([...p.body.querySelectorAll('[data-web-source]')].map(button=>button.title),['Help','Title']);
      reason='Select files first.';p.body.innerHTML=ui.menuHTML();p.translate(locale);
      for(const button of p.body.querySelectorAll('[data-web-source]')){assert.equal(button.title,t.t(reason));assert.ok(button.disabled);}
      assert.deepEqual([...p.body.querySelectorAll('[data-web-source] > span')].map(span=>span.textContent),['Help…','Title…']);
    }finally{p.close();}
  });
  for(const newline of ['\n','\r\n'])test(`${locale}: native file confirmations preserve data and paragraphs (${JSON.stringify(newline)})`,async()=>{
    const translator=new Translator(locale,catalogs[locale]),calls=[];
    const nativeSource=(await source('i18n/dialog.ts')).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,'');
    const js=ts.transpileModule(nativeSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
    const dialogs=new Function('dialog','t','translate','translationActive',`${js}\nreturn {ask,confirm};`)({ask:async(...args)=>{calls.push(args);return false;},confirm:async(...args)=>{calls.push(args);return false;}},key=>translator.t(key),text=>translator.translate(text),()=>true);
    const {confirm,ask}=dialogs;
    const names=['Help',`My  Files${newline}Title`],destination='/Music/My  Files';
    for(const mac of [false,true])for(const operation of ['file-delete','file-delete-permanent','file-copy','file-move']){
      const host={mac,selection:()=>names.map(name=>({name,path:name})),openReason:()=>'',pickDirectories:async()=>[destination],confirm:(text,title)=>confirm(text.replace(/\r?\n/g,newline),title)};
      await new FileOpsUI(host).operate(operation);
      const [message,options]=calls.at(-1);
      assert.ok(message.includes(`${newline}${newline}${names.join(', ')}${newline}${newline}`),message);
      if(operation==='file-copy'||operation==='file-move')assert.ok(message.includes(destination));
      assert.notEqual(options.title,operation==='file-copy'?'Copy files':operation==='file-move'?'Move files':operation==='file-delete'?'Delete files':'Delete permanently');
      assert.equal(options.okLabel,translator.t('OK'));assert.equal(options.cancelLabel,translator.t('Cancel'));
    }
    const tags={mpeg:{remove_id3v1:true,remove_id3v2:false,remove_ape:false},ape:{remove_id3v1:false,remove_id3v2:false,remove_ape:true}};
    await ask({...removalMessage(2,tags),separator:newline},'Remove tag');
    const removal=calls.at(-1)[0];
    assert.ok(removal.includes(`${newline}${newline}MP3, TTA: ID3v1${newline}APE, MPC, WavPack, TAK, OptimFROG: APE${newline}`),removal);
    assert.notEqual(removal.split(newline)[0],'Remove tags from 2 selected files?');
    assert.notEqual(removal.split(newline).at(-1),'Other formats: all tags');
    for(const size of [0,1,2]){
      const filename=`Help  ${newline}Title.json`;
      const prompt=configurationMessage({basename:path=>path,path:filename,drafts:{size}});
      await ask({...prompt,separator:newline+newline},'Load configuration');
      const configuration=calls.at(-1)[0];
      assert.ok(configuration.includes(filename),configuration);
      assert.ok(configuration.includes(`${newline}${newline}${translator.t('OpenTag reloads its window afterwards. The Discogs access token is not part of a configuration file.')}`),configuration);
      assert.notEqual(configuration.split(newline)[0].startsWith('Replace the current options'),true);
    }
  });
  test(`${locale}: generated copy, playlist and tag-to-tag previews retain data and translate their controls`,async()=>{
    const p=page();
    try{
      const fail=(_dialog,error)=>{throw error;},uiModal=modal(p);
      copyAsTextDialog({modal:uiModal,modalError:fail,values:()=>({}),count:()=>7,evaluate:async()=>['Help','Title','My  Files\nTitle'],write:async()=>{},remember:async()=>{},done:()=>{}});
      new PlaylistUI({modal:uiModal,modalError:fail,values:()=>({}),files:()=>['Help'],workingDirectory:()=>'',loadedPlaylist:()=>'',columns:()=>[],invoke:async()=>({playlists:[{path:'Help',files:1}],errors:[]})}).generate();
      openTagToTag({modal:uiModal,modalError:fail,esc,paths:()=>['Help'],fieldNames:()=>[],displayName:path=>path,canApply:()=>true,preview:async()=>[{path:'Help',value:'Help'}],storage:{getItem:()=>JSON.stringify({field:'Help'}),setItem:()=>{}}});
      await p.window.happyDOM.waitUntilComplete();
      const t=p.translate(locale);
      assert.equal(p.body.querySelector('#copy-text-preview > div').textContent,'Help');
      assert.equal(p.body.querySelector('#copy-text-preview > div:nth-child(2)').textContent,'Title');
      assert.equal(p.body.querySelector('#copy-text-preview > div:nth-child(3)').textContent,'My  Files\nTitle');
      assert.equal(p.body.querySelector('#copy-text-preview').getAttribute('aria-label'),t.t('Preview'));
      assert.equal(p.body.querySelector('#copy-text-preview small').textContent,t.translate('… 2 more'));
      assert.equal(p.body.querySelector('#generate-preview > div:nth-child(2) > span').textContent,'Help');
      assert.equal(p.body.querySelector('#generate-preview small').textContent,`(${t.translate('1 file')})`);
      const tagRow=p.body.querySelector('#tag-tag-preview .preview > div');
      assert.equal(tagRow.querySelector('small').textContent,'Help');
      assert.equal(tagRow.textContent,'HelpHelp: Help');
      assert.equal(p.body.querySelector('#tag-tag-preview .preview').getAttribute('aria-label'),t.t('Tag → Tag results'));
      p.body.querySelector('#tag-tag-field').value='_DIRECTORY';p.body.querySelector('#tag-tag-field').dispatchEvent(new p.window.Event('input'));
      await p.window.happyDOM.waitUntilComplete();p.translate(locale);
      assert.equal(p.body.querySelector('#tag-tag-preview .preview > div').textContent,`Help${t.t('New folder name')}: Help`);
    }finally{p.close();}
  });
  test(`${locale}: main converter and numbering previews preserve their data while UI states translate`,()=>{
    const p=page(),data={esc,displayName:value=>value,publicMessage:value=>value};
    try{
      p.body.innerHTML=mainTemplate('<div class="preview">',{...data,result:[{path:'Help',value:'Help'},{path:'Title',value:'Title'}]});
      p.translate(locale);assert.equal(p.body.textContent,'HelpHelpTitleTitle');
      p.body.innerHTML=mainTemplate('<div class="preview">',{...data,result:[{path:'Help',fields:{Title:['Title']}},{path:'Help',fields:null}]},1);
      const t=p.translate(locale);assert.equal(p.body.querySelector('.preview > div').textContent,'HelpTitle: Title');
      assert.equal(p.body.querySelector('.no-match').textContent,`Help${t.t('No match, not changed')}`);
      p.body.innerHTML=mainTemplate('<div class="preview" tabindex="0" aria-label="Numbering preview">',{...data,rows:[{path:'Help',fields:{TRACK:['Help'],DISCNUMBER:['Help']}}],attempt:{request:{options:{disc_enabled:true}}}});
      p.translate(locale);assert.deepEqual([...p.body.querySelectorAll('tbody td')].map(el=>el.textContent),['Help','Help','Help']);
      assert.equal(p.body.querySelector('th').textContent,t.t('File'));
      const fieldRows=rendererVariable(parsed,'fieldRows',{esc});
      const reviewed=rendererVariable(parsed,'previewMarkup',{esc,fieldRows});
      p.body.innerHTML=reviewed([{path:'Help',target:'Title',captures:{Help:['Title','']}},{path:'Title',fields:{Help:[]},source_line:1,selection_index:0}]);
      p.translate(locale);
      assert.equal(p.body.querySelector('.preview').getAttribute('aria-label'),t.t('Reviewed conversion results'));
      assert.deepEqual([...p.body.querySelectorAll('strong')].map(el=>el.textContent),['Title','Title']);
      assert.equal(p.body.querySelector('small').textContent,'Help');
      assert.equal(p.body.querySelector('dt').textContent,'Help');
      assert.equal(p.body.querySelector('dd > span > span').textContent,'Title');
      assert.deepEqual([...p.body.querySelectorAll('em')].map(el=>el.textContent),[t.t('(empty)'),t.t('No values')]);
    }finally{p.close();}
  });
  test(`${locale}: tag-source matching previews preserve file and field data, including accessible labels`,()=>{
    const p=page();
    try{
      const track={position:'Title',length:1};
      const render=rendererVariable(tagSourceParsed,'matchMarkup',{release:{tracks:[track]},files:[{path:'Help',length:1}],slots:['Help',null],trackField:'Title',html:esc,duration:()=>'',fileLength:()=>'',fileName:path=>path||'',trackValue:()=> 'Help'});
      p.body.innerHTML=render();const t=p.translate(locale);
      assert.equal(p.body.querySelector('thead th:nth-child(2)').textContent,'Title');
      assert.equal(p.body.querySelector('tbody td').textContent,'Title');
      assert.equal(p.body.querySelector('tbody tr td:nth-child(4)').textContent,'Help');
      assert.equal(p.body.querySelector('tbody tr td:nth-child(4) span').title,'Help');
      assert.equal(p.body.querySelector('.ts-track-value').getAttribute('aria-label'),t.t('{trackField} of track {trackPosition}',{trackField:'Title',trackPosition:'Title'}));
      assert.equal(p.body.querySelector('thead th:nth-child(4)').textContent,t.t('File'));
      assert.equal(p.body.querySelector('tbody tr:nth-child(2) em').textContent,t.t('Not written'));
      track.position='A1';p.body.innerHTML=render();p.translate(locale);
      assert.equal(p.body.querySelector('.ts-track-value').getAttribute('aria-label'),t.t('{trackField} of track {trackPosition}',{trackField:'Title',trackPosition:'A1'}));
      for(const extra of [{country:'Title'},{barcode:'Help'}]){
        const chosen=rendererVariable(tagSourceParsed,'chosenMarkup',{results:[{artist:'Title',title:'Help',...extra}],chosen:0,web:false,freedb:false,html:esc,preview:()=>''});
        p.body.innerHTML=chosen();p.translate(locale);
        assert.equal(p.body.querySelector('strong').textContent,'Title – Help');
        assert.equal(p.body.querySelector('strong + span').textContent,extra.country||t.translate('Barcode Help',0,true));
      }
    }finally{p.close();}
  });
  test(`${locale}: Manage Covers keeps description edits verbatim and translates the cleared state`,async()=>{
    const p=page();
    try{
      const cover={digest:'fixture',mime:'image/png',size:42,width:1,height:1,picture_type:'CoverFront',description:'',files:[0],index:0,data_base64:'AA=='};
      const ui=new CoverUI({reason:()=>'',selection:()=>[{path:'Help',covers:[cover]}],invoke:async()=>({files:1,covers:[cover],errors:[]}),modal:modal(p),clipboard:()=>false,displayed:()=>({path:'Help',covers:[cover],index:0})});
      await ui.manage();const t=p.translate(locale),input=p.body.querySelector('#manage-cover-description');
      const shown=()=>p.body.querySelector('.manage-cover-text > span');
      assert.equal(shown().textContent,t.t('No description'));
      for(const value of ['Help','','No description','']){
        input.value=value;input.dispatchEvent(new p.window.Event('input'));
        await p.window.happyDOM.waitUntilComplete();
        assert.equal(shown().textContent,value||t.t('No description'));
        input.dispatchEvent(new p.window.Event('change'));p.translate(locale);
        assert.equal(shown().textContent,value||t.t('No description'));
      }
    }finally{p.close();}
  });
  test(`${locale}: textarea attributes translate while editable text and values stay verbatim`,async()=>{
    const p=page();
    try{
      p.body.innerHTML='<textarea id="template" aria-label="Export template" placeholder="Help">Help</textarea><textarea id="field" aria-label="Field value 1">Help</textarea><textarea translate="no" aria-label="Export template">Help</textarea>';
      const template=p.body.querySelector('#template'),field=p.body.querySelector('#field');
      template.value='Title';
      const t=p.translate(locale);
      assert.equal(template.getAttribute('aria-label'),t.t('Export template'));
      assert.equal(template.placeholder,t.t('Help'));
      assert.equal(field.getAttribute('aria-label'),t.t('Field value {index}',{index:1}));
      assert.equal(template.textContent,'Help');assert.equal(template.defaultValue,'Help');assert.equal(template.value,'Title');
      assert.equal(field.value,'Help');assert.equal(p.body.querySelector('[translate="no"]').getAttribute('aria-label'),'Export template');
      field.setAttribute('aria-label','Field value 2');
      field.firstChild.data='Title';
      template.replaceChildren(p.window.document.createTextNode('Help'));
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(field.getAttribute('aria-label'),t.t('Field value {index}',{index:2}));
      assert.equal(field.textContent,'Title');assert.equal(field.defaultValue,'Title');assert.equal(field.value,'Title');
      assert.equal(template.textContent,'Help');assert.equal(template.value,'Title');
      p.body.insertAdjacentHTML('beforeend','<textarea id="added" aria-label="Export template">Help</textarea>');
      await p.window.happyDOM.waitUntilComplete();
      assert.equal(p.body.querySelector('#added').getAttribute('aria-label'),t.t('Export template'));
      assert.equal(p.body.querySelector('#added').value,'Help');
      p.translate('en');
      assert.equal(template.getAttribute('aria-label'),'Export template');assert.equal(template.placeholder,'Help');
      assert.equal(field.getAttribute('aria-label'),'Field value 2');assert.equal(field.value,'Title');assert.equal(template.value,'Title');
    }finally{p.close();}
  });
  test(`${locale}: cover accessible labels translate UI states and types alongside verbatim descriptions`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    try{
      const cover={index:0,digest:'fixture',mime:'image/png',size:42,width:1,height:1,picture_type:'CoverFront',description:'Help',data_base64:'AA=='};
      let covers=[cover],selection=[{path:'Help',covers}],index=0;
      const ui=new CoverUI({selection:()=>selection,displayed:()=>({path:'Help',covers,index})});
      const options={fallback:'',error:'',unavailable:false,buttons:''};
      const check=(state,extra={})=>{
        p.body.innerHTML=ui.markup({...options,...extra});p.translate(locale);
        const art=p.body.querySelector('.cover-art'),label=art.getAttribute('aria-label');
        assert.ok(label.includes(t.t(state)),`${state}: ${label}`);
        const key=`Cover art: {coverState}${covers.length>1?', {index} of {length}':''}. Enter views it larger; Shift+F10 opens cover commands.`;
        assert.equal(label,t.t(key,{coverState:t.t(state),index:index+1,length:covers.length}));
        assert.equal(art.title,t.t('Right-click for cover commands; double-click to enlarge'));
        assert.equal(p.body.querySelector('[data-cover="menu"]').getAttribute('aria-label'),t.t('Cover commands'));
      };
      for(const [id,label] of PICTURE_TYPES){
        cover.picture_type=id;check(label);
        assert.equal(p.body.querySelector('[data-cover-image]').getAttribute('alt'),t.t('Embedded {pictureTypeLabel} artwork',{pictureTypeLabel:t.t(label)}));
        assert.equal(p.body.querySelector('.cover-info > span[translate="no"]').textContent,'Help');
      }
      cover.picture_type='CoverFront';covers=[cover,{...cover,index:1,picture_type:'CoverBack'}];selection=[{path:'Help',covers}];index=1;
      check('Back Cover');
      for(const command of ['previous','next'])assert.equal(p.body.querySelector(`[data-cover="${command}"]`).getAttribute('aria-label'),t.t(`${command==='previous'?'Previous':'Next'} cover`));
      ui.openViewer();p.translate(locale);
      assert.equal(p.body.querySelector('.cover-viewer').getAttribute('aria-label'),t.t('Cover viewer'));
      assert.equal(p.body.querySelector('.cover-viewer img').alt,t.t('{pictureTypeLabel} artwork',{pictureTypeLabel:t.t('Back Cover')}));
      assert.equal(p.body.querySelector('[data-viewer="close"]').getAttribute('aria-label'),t.t('Close cover viewer'));
      p.body.querySelector('.cover-viewer').close();
      covers=[];selection=[{path:'Help',covers}];index=0;
      check('No artwork');check('Artwork unavailable',{unavailable:true});
      check('Folder image',{folder:{name:'Help',mime:'image/png',size:42,width:1,height:1,data_base64:'AA=='}});
      assert.equal(p.body.querySelector('[data-folder-image]').alt,t.t('Folder image {name}, not embedded',{name:'Help'}));
      selection=[{path:'Help',covers:[cover]},{path:'Title',covers:[]}];check('Covers vary');
      p.translate('en');assert.ok(p.body.querySelector('.cover-art').getAttribute('aria-label').startsWith('Cover art: Covers vary.'));
    }finally{p.close();}
  });
  test(`${locale}: cover descriptions and folder-image names stay verbatim while cover UI translates`,()=>{
    const p=page();
    try{
      const cover={index:0,digest:'fixture',mime:'image/png',size:42,width:1,height:1,picture_type:'CoverFront',description:'Help',data_base64:'AA=='};
      let covers=[cover];
      const ui=new CoverUI({selection:()=>[{path:'one',covers},{path:'two',covers}],displayed:()=>({path:'one',covers,index:0})});
      const options={fallback:'',error:'',unavailable:false,buttons:'',note:'Loading artwork…'};
      p.body.innerHTML=ui.markup(options);let t=p.translate(locale);
      assert.equal(p.body.querySelector('.cover-info strong').textContent,t.t('Front Cover'));
      assert.ok([...p.body.querySelectorAll('.cover-info > span')].some(span=>span.textContent==='Help'));
      assert.ok([...p.body.querySelectorAll('.cover-info > span')].some(span=>span.textContent===t.t('Shared by {selected} files',{selected:2})));
      assert.equal(p.body.querySelector('.cover-info > span:last-child').textContent,t.t('Loading artwork…'));
      cover.description='Loading artwork…';p.body.innerHTML=ui.markup(options);p.translate(locale);
      assert.equal(p.body.querySelectorAll('.cover-info > span')[1].textContent,'Loading artwork…');
      assert.equal(p.body.querySelector('.cover-info > span:last-child').textContent,t.t('Loading artwork…'));
      covers=[];p.body.innerHTML=ui.markup({...options,folder:{name:'Help',mime:'image/png',size:42,width:1,height:1,data_base64:'AA=='}});p.translate(locale);
      assert.equal(p.body.querySelector('.cover-info strong').textContent,t.t('Folder image'));
      assert.equal(p.body.querySelector('.cover-info > span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-folder-image]').getAttribute('alt'),t.t('Folder image {name}, not embedded',{name:'Help'}));
    }finally{p.close();}
  });
  test(`${locale}: editors translate shipped names and retain custom names through editing`,()=>{
    const p=page(),t=new Translator(locale,catalogs[locale]);
    setColumnNameTranslation(label=>t.translate(label));
    try{
      const layout=defaultLayout();layout.columns=[...layout.columns.slice(0,2),{...layout.columns[1],id:'custom',name:'Help'}];
      new ColumnUI({layout:()=>layout}).customize();
      const panel=new TagPanel({translateLabel:label=>t.translate(label),values:()=>[],disabled:()=>false,directories:()=>({selected:[],all:[]})});
      const title=defaultPanelLayout().fields.find(field=>field.field==='TITLE');
      panel.settings.panel.fields=[title,{...title,field:'CUSTOM',name:'Help'}];
      p.body.insertAdjacentHTML('beforeend',panel.fieldsHTML());panel.bind(p.body.querySelector('.panel-grid'));
      p.body.querySelector('.combo-toggle').click();p.translate(locale);
      assert.equal(p.body.querySelector('.combo-list').getAttribute('aria-label'),t.t('{label} values',{label:t.t('Title')}));
      assert.deepEqual([...p.body.querySelectorAll('.panel-grid > .field > label')].map(label=>label.textContent),[t.t('Title'),'Help']);
      panel.closeList();panel.configure();
      p.translate(locale);
      const columnRows=[...p.body.querySelectorAll('.column-list li')],panelRows=[...p.body.querySelectorAll('.panel-config-row')];
      assert.deepEqual(columnRows.map(row=>row.querySelector('span').textContent),[t.t('Filename'),t.t('Title'),'Help']);
      for(const [i,name] of [t.t('Filename'),t.t('Title'),'Help'].entries())assert.equal(columnRows[i].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name}));
      assert.deepEqual(panelRows.map(row=>row.querySelector('span').textContent),[t.t('Title'),'Help']);
      assert.deepEqual(panelRows.map(row=>row.querySelector('small').textContent),['TITLE','CUSTOM']);
      for(const [i,name] of [t.t('Title'),'Help'].entries())assert.equal(panelRows[i].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name}));
      assert.equal(p.body.querySelector('#column-name').value,'Filename');assert.equal(p.body.querySelector('#tp-name').value,'Tit&le');
      const columnName=p.body.querySelector('#column-name'),panelName=p.body.querySelector('#tp-name');
      for(const name of ['Help','Filename']){
        columnName.value=name;columnName.dispatchEvent(new p.window.Event('input',{bubbles:true}));p.translate(locale);
        const shown=name==='Filename'?t.t(name):name;
        assert.equal(columnRows[0].querySelector('span').textContent,shown);
        assert.equal(columnRows[0].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name:shown}));
      }
      for(const name of ['Help','Tit&le']){
        panelName.value=name;panelName.dispatchEvent(new p.window.Event('input',{bubbles:true}));p.translate(locale);
        const shown=name==='Tit&le'?t.t('Title'):name;
        assert.equal(panelRows[0].querySelector('span').textContent,shown);
        assert.equal(panelRows[0].querySelector('input').getAttribute('aria-label'),t.t('Show {name}',{name:shown}));
      }
    }finally{setColumnNameTranslation(label=>label);p.close();}
  });
  test(`${locale}: file-row data stays verbatim while checkbox and pending tooltip translate`,()=>{
    const p=page();
    try{
      p.body.innerHTML='<table>'+renderRows({path:'/Help',filename:'Help',format:'MP3',fields:{TITLE:['Help']}})+'</table>';
      const t=p.translate(locale),checkbox=p.body.querySelector('.row-select'),cell=p.body.querySelector('td');
      assert.equal(checkbox.getAttribute('aria-label'),t.t('Select {filename}',{filename:'Help'}));
      assert.equal(p.body.querySelector('.rowmarker').title,t.t('Pending changes'));
      assert.equal(cell.querySelector('[translate="no"]').title,'Help');assert.equal(cell.textContent,'●Help');
      p.translate('en');assert.equal(checkbox.getAttribute('aria-label'),'Select Help');assert.equal(cell.querySelector('[translate="no"]').title,'Help');
    }finally{p.close();}
  });
  test(`${locale}: custom columns, action groups and tools keep names in text and accessible labels`,async()=>{
    const p=page();
    try{
      const layout=defaultLayout();layout.columns=[{...layout.columns[0],name:'Help'}];
      new ColumnUI({layout:()=>layout}).customize();
      const actions=new ActionUI({load:async()=>({groups:[{name:'Help',enabled:true,actions:[{type:'format_value',field:'Help',format:'Help'}]}],presets:[]}),canExecute:()=>true,selectedCount:()=>1,changed:()=>{}});
      await actions.groupsDialog();
      const tool={name:'Help',path:'Help',parameters:'',for_each_file:false};
      const tools=new ToolsUI({values:()=>({tools:[tool]}),reason:()=>'',mac:false});
      p.body.insertAdjacentHTML('beforeend',`<nav>${tools.menuHTML()}</nav>`);
      p.body.insertAdjacentHTML('beforeend',contextMenuMarkup([{label:'Help',id:'tool:0',title:'Help',userLabel:true,userTitle:true},{label:'Help',id:'tool:1',title:'Select files first.',disabled:true,userLabel:true}]));
      const options=toolsOptionsPage({pickProgram:async()=>null});
      p.body.insertAdjacentHTML('beforeend',options.controls[0].render({pages:{tools:{tools:[tool]}}}));
      const t=p.translate(locale);
      assert.equal(p.body.querySelector('[data-visible]').getAttribute('aria-label'),t.t('Show {name}',{name:'Help'}));
      assert.equal(p.body.querySelector('.column-list span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-enable]').getAttribute('aria-label'),t.t('Enable {name}',{name:'Help'}));
      assert.equal(p.body.querySelector('.group-actions .action-section-title').textContent,'Help');
      assert.equal(p.body.querySelector('.action-summary-list button span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-tool] span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-tool]').title,'Help');
      assert.equal(p.body.querySelector('[data-context="tool:0"] span').textContent,'Help');
      assert.equal(p.body.querySelector('[data-context="tool:0"]').title,'Help');
      assert.equal(p.body.querySelector('[data-context="tool:1"]').title,t.t('Select files first.'));
      assert.equal(p.body.querySelector('#tools-list option').textContent,'Help');
      p.body.querySelector('.action-groups-dialog [data-command="edit"]').click();p.translate(locale);
      assert.equal(p.body.querySelector('.group-editor-dialog header').textContent,t.t('Actions — {name}',{name:'Help'}));
      assert.equal(p.body.querySelector('.action-edit-list button span').textContent,'Help');
    }finally{p.close();}
  });
  test(`${locale}: saved-history values stay verbatim while controls and empty state translate`,()=>{
    const p=page();
    try{
      p.body.innerHTML='<div><input value="Help" data-helper="history" data-history="probe" data-history-manage></div>';
      const helpers=installHelperMenus({storage:memory(),fieldNames:()=>[],panelFields:()=>[]});
      helpers.history.set('probe',[{name:'Help',value:'Help',description:'Help',favorite:true}]);
      p.body.querySelector('.helper-button').click();
      p.translate(locale);assert.equal([...p.body.querySelectorAll('.helper-menu button')].find(el=>el.textContent==='Help')?.textContent,'Help');
      [...p.body.querySelectorAll('.helper-menu button')].find(el=>el.textContent===catalogs[locale]['Manage history…']).click();
      const t=p.translate(locale),checkbox=p.body.querySelector('[data-favorite]');
      assert.equal(checkbox.title,t.t('Favorite'));
      assert.equal(checkbox.getAttribute('aria-label'),t.t('Favorite: {name}',{name:'Help'}));
      assert.deepEqual([...p.body.querySelectorAll('.history-text > *')].map(el=>el.textContent),['Help','Help','Help']);
      p.body.querySelector('[data-remove]').click();p.translate(locale);
      assert.equal(p.body.querySelector('.history-list').getAttribute('aria-label'),t.t('History'));
      assert.equal(p.body.querySelector('.history-list .group-empty').textContent,t.t('No saved format strings.'));
      p.translate('en');assert.equal(p.body.querySelector('.history-list .group-empty').textContent,'No saved format strings.');
    }finally{p.close();}
  });
}

for(const [ending,newline] of [['LF','\n'],['CRLF','\r\n']]){
  test(`native dialogs honor an active English user catalog and preserve picker data (${ending} source)`,async()=>{
    const nativeSource=(await source('i18n/dialog.ts')).replace(/\r?\n/g,newline).replace(/^import .*;\r?\n/gm,'').replace(/^export /gm,'');
    const js=ts.transpileModule(`export function create(dialog,translator){
      const t=(key)=>translator.t(key),translate=(text)=>translator.translate(text),translationActive=()=>translator.active;
      ${nativeSource}
      return {open,save,ask,confirm,message};
    }`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
    const {create}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
    const calls=[],native=Object.fromEntries(['open','save','ask','confirm','message'].map(name=>[name,(...args)=>(calls.push([name,...args]),args)]));
    const custom=new Translator('en',{'Open files':'Choose tracks','Audio files':'Music','Save tags':'Write metadata',Yes:'Proceed',No:'Decline',OK:'Accept',Cancel:'Back'});
    const dialogs=create(native,custom),picker={title:'Open files',defaultPath:'Help',filters:[{name:'Audio files',extensions:['mp3']}]};
    assert.deepEqual(dialogs.open(picker)[0],{title:'Choose tracks',defaultPath:'Help',filters:[{name:'Music',extensions:['mp3']}]});
    assert.equal(dialogs.save(picker)[0].title,'Choose tracks');
    assert.deepEqual(dialogs.ask('Save tags','Open files'),['Write metadata',{title:'Choose tracks',okLabel:'Proceed',cancelLabel:'Decline'}]);
    assert.deepEqual(dialogs.confirm('Save tags'),['Write metadata',{okLabel:'Accept',cancelLabel:'Back'}]);
    assert.deepEqual(dialogs.message('Save tags'),['Write metadata',{buttons:{ok:'Accept'}}]);
    assert.deepEqual(picker,{title:'Open files',defaultPath:'Help',filters:[{name:'Audio files',extensions:['mp3']}]});
    assert.equal(create(native,new Translator('en')).open(picker)[0],picker);
    assert.deepEqual(create(native,new Translator('en')).ask({parts:['Help','Title'],separator:'\n\n'},'Open files'),['Help\n\nTitle','Open files']);
  });
}
