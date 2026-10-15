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
const {copyAsTextDialog}=await load('utils-ui.ts'),{PlaylistUI}=await load('playlist-ui.ts'),{openTagToTag}=await load('tag-tag-ui.ts');
const {FileOpsUI}=await load('file-ops.ts');
const {removeTagPrompt}=await load('remove-tag.ts');
const {WebSourcesUI}=await import(await moduleUrl('web-sources.ts',{"'./action-ui'":JSON.stringify(await moduleUrl('action-ui.ts'))}));
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
function modal(p){return(title,content,buttons)=>{const dialog=p.window.document.createElement('dialog');dialog.innerHTML=`<header>${esc(title)}</header>${content}<footer>${buttons}</footer>`;p.body.append(dialog);dialog.setAttribute('open','');return dialog;};}
function mainTemplate(needle,values,index=0){
  const templates=[];
  const visit=node=>{if(ts.isTemplateExpression(node)&&(typeof needle==='function'?needle(node):node.head.text.startsWith(needle)))templates.push(node);ts.forEachChild(node,visit);};visit(parsed);
  const template=templates[index];
  assert.ok(template,`Production template ${needle}`);
  const js=ts.transpileModule(`return ${template.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js)(...Object.values(values));
}
function rendererVariable(tree,name,values){
  let declaration;
  const visit=node=>{if(ts.isVariableDeclaration(node)&&node.name.getText(tree)===name)declaration=node;ts.forEachChild(node,visit);};visit(tree);
  assert.ok(declaration,`Production renderer ${name}`);
  const js=ts.transpileModule(`const ${declaration.getText(tree)};return ${name};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  return new Function(...Object.keys(values),js)(...Object.values(values));
}
function page(){
  const window=new Window();
  for(const key of ['window','document','Node','NodeFilter','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','HTMLButtonElement','MutationObserver','MouseEvent','Event'])globalThis[key]=key==='window'?window:window[key];
  const body=window.document.body;
  let translator=new Translator('en');const dom=new DomTranslator(body,()=>translator);
  return {body,window,translate(locale){translator=new Translator(locale,catalogs[locale]??{});dom.apply();return translator;},close(){window.happyDOM.abort();}};
}
const memory=()=>{const data=new Map();return{getItem:key=>data.get(key)??null,setItem:(key,value)=>data.set(key,value)};};

for(const locale of ['de','nl']){
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
    const confirm=new Function('dialog','t','translate','translationActive',`${js}\nreturn confirm;`)({confirm:async(...args)=>{calls.push(args);return false;}},key=>translator.t(key),text=>translator.translate(text),()=>true);
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
    await confirm(removeTagPrompt(2,tags,text=>translator.translate(text)).replace(/\n/g,newline),'Remove tag');
    const removal=calls.at(-1)[0];
    assert.ok(removal.includes(`${newline}${newline}MP3, TTA: ID3v1${newline}APE, MPC, WavPack, TAK, OptimFROG: APE${newline}`),removal);
    assert.notEqual(removal.split(newline)[0],'Remove tags from 2 selected files?');
    assert.notEqual(removal.split(newline).at(-1),'Other formats: all tags');
    for(const size of [0,1,2]){
      const filename=`Help  ${newline}Title.json`;
      const prompt=mainTemplate(node=>node.head.text===''&&node.getText(parsed).includes('OpenTag reloads its window'),{translate:text=>translator.translate(text),basename:path=>path,path:filename,drafts:{size}});
      await confirm(prompt.replace(/\r?\n/g,newline),'Load configuration');
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
      const render=rendererVariable(tagSourceParsed,'matchMarkup',{release:{tracks:[{position:'Title',length:1}]},files:[{path:'Help',length:1}],slots:['Help',null],trackField:'Title',html:esc,duration:()=>'',fileLength:()=>'',fileName:path=>path||'',trackValue:()=> 'Help'});
      p.body.innerHTML=render();const t=p.translate(locale);
      assert.equal(p.body.querySelector('thead th:nth-child(2)').textContent,'Title');
      assert.equal(p.body.querySelector('tbody td').textContent,'Title');
      assert.equal(p.body.querySelector('tbody tr td:nth-child(4)').textContent,'Help');
      assert.equal(p.body.querySelector('tbody tr td:nth-child(4)').title,'Help');
      assert.equal(p.body.querySelector('.ts-track-value').getAttribute('aria-label'),t.translate('Title of track Title',0,true));
      assert.equal(p.body.querySelector('thead th:nth-child(4)').textContent,t.t('File'));
      assert.equal(p.body.querySelector('tbody tr:nth-child(2) em').textContent,t.t('Not written'));
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
