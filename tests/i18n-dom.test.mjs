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
const {Translator}=await load('i18n/core.ts'),{DomTranslator}=await load('i18n/dom.ts');
const {ColumnUI,defaultLayout,setColumnNameTranslation}=await load('columns.ts'),{ActionUI}=await load('action-ui.ts');
const {TagPanel,defaultLayout:defaultPanelLayout}=await load('tag-panel.ts');
const {CoverUI}=await import(await moduleUrl('covers.ts',{"import './covers.css';":'',"'./keyboard'":JSON.stringify(await moduleUrl('keyboard.ts')),"'./cover-model'":JSON.stringify(await moduleUrl('cover-model.ts'))}));
const {contextMenuMarkup}=await load('row-context-menu.ts');
const {ToolsUI,toolsOptionsPage}=await load('tools.ts'),{installHelperMenus}=await load('helper-menu.ts');
const main=await source('main.ts'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
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
function page(){
  const window=new Window();
  for(const key of ['window','document','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','MutationObserver','MouseEvent','Event'])globalThis[key]=key==='window'?window:window[key];
  const body=window.document.body;
  let translator=new Translator('en');const dom=new DomTranslator(body,()=>translator);
  return {body,window,translate(locale){translator=new Translator(locale,catalogs[locale]??{});dom.apply();return translator;},close(){window.happyDOM.abort();}};
}
const memory=()=>{const data=new Map();return{getItem:key=>data.get(key)??null,setItem:(key,value)=>data.set(key,value)};};

for(const locale of ['de','nl']){
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
  });
}
