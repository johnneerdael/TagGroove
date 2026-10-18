import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';

const root=new URL('../',import.meta.url),url=source=>`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`;
const transpile=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const moduleURL=async name=>url(transpile(await readFile(new URL(`src/${name}.ts`,root),'utf8')));
const toolbar=await import(await moduleURL('toolbar'));
const {chapterRowReason}=await import(await moduleURL('chapters'));
const {documentSelection}=await import(await moduleURL('document-state'));
const {ConfigurationTransferUI}=await import(await moduleURL('configuration-transfer'));
const coverSource=await readFile(new URL('src/covers.ts',root),'utf8');
const {CoverUI}=await import(url(transpile(coverSource).replace("import './covers.css';",'').replace("'./keyboard'",JSON.stringify(await moduleURL('keyboard'))).replace("'./cover-model'",JSON.stringify(await moduleURL('cover-model')))));
const main=await readFile(new URL('src/main.ts',root),'utf8'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
const functions=names=>names.map(name=>{const node=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name);assert.ok(node,name);return node.getText(parsed);}).join('\n');
const declaration=name=>{let found;const visit=node=>{if(ts.isVariableDeclaration(node)&&node.name.getText(parsed)===name)found=node;ts.forEachChild(node,visit);};visit(parsed);return found?`const ${found.getText(parsed)};`:'';};
const followups=['copy-text','remove-tag-types','strip-mp4-chapters','util-unsupported','util-optimize','help-docs','help-shortcuts','help-issues','help-updates','help-releases','about','columns','autosize-columns','list-unsorted','shuffle-list','toggle-toolbar','toolbar-customize','toggle-custom-toolbar','custom-toolbar-customize','config-save','config-load','config-folder','cover-replace','cover-extract','cover-adjust','cover-description','cover-cut','cover-copy','cover-paste'];
const art=(digest='front')=>({index:0,digest,mime:'image/jpeg',picture_type:'CoverFront',description:'',data_base64:'AA==',size:1,width:1,height:1});

test('daily toolbar choices append hidden while existing commands, visible defaults and imported names survive',()=>{
  const ids=Object.keys(toolbar.TOOLBAR_COMMANDS);
  assert.deepEqual(ids.slice(69),followups);
  assert.deepEqual(toolbar.defaultToolbar().items.filter(item=>followups.includes(item.id)),followups.map(id=>({id,visible:false})));
  assert.deepEqual(toolbar.visibleItems(toolbar.defaultToolbar().items).filter(item=>!toolbar.isDivider(item.id)).map(item=>item.id),['folder','files','save','reload','filename-tags','tags-filename','groups','number']);
  const saved={shown:false,items:[{id:'save',label:'  My saved name  ',visible:true},{id:'separator-8',visible:true},{id:'folder',visible:false}]};
  const normalized=toolbar.normalizeToolbar(saved);
  assert.deepEqual(normalized.items.slice(0,3),[{id:'save',label:'My saved name',visible:true},saved.items[1],saved.items[2]]);
  assert.deepEqual(normalized.items.filter(item=>followups.includes(item.id)),followups.map(id=>({id,visible:false})));
  const custom={shown:true,items:[{id:'tool:My tool',label:'Tools & names',visible:true},{id:'action:Album#Cleanup',visible:false},{id:'cover-copy',label:'Keep this name',visible:true}]};
  assert.deepEqual(toolbar.normalizeCustomToolbar(custom),custom);
});

test('followup hints translate while saved toolbar names remain authored text',async()=>{
  const {Translator}=await import(await moduleURL('i18n/core'));
  for(const locale of ['de','nl']){const catalog=JSON.parse(await readFile(new URL(`src/i18n/catalogs/${locale}.json`,root),'utf8')),translator=new Translator(locale,catalog);
    for(const id of followups)assert.notEqual(translator.translate(toolbar.itemHint(id)),toolbar.itemHint(id),`${locale}: ${id}`);
  }
  const named={id:'cover-copy',visible:true,label:'My Cover $& {title}'};assert.equal(toolbar.userLabel(named),true);assert.equal(toolbar.itemLabel(named),named.label);
});

function runtime(state={}){
  const window=new Window({url:'http://localhost:1420'}),document=window.document,calls=[];
  globalThis.document=document;
  window.location.reload=()=>calls.push(['reload']);
  const selected=new Set(state.selection??['/Music/a.mp3']),entries=new Map((state.list??['/Music/a.mp3','/Music/b.mp3']).map(path=>[path,{path,filename:path.split('/').pop(),format:'mp4',covers:state.coversByPath?.[path]??state.covers??[art()]}]));
  const dialogs=[],modal=(title,content,buttons)=>{calls.push(['dialog',title]);const dialog=document.createElement('dialog');dialog.innerHTML=`<header>${title}</header>${content}<button data-close>Cancel</button>${buttons}`;document.body.append(dialog);dialog.querySelector('[data-close]').onclick=()=>dialog.close();dialogs.push(dialog);dialog.showModal();return dialog;};
  let model;
  const coverUI=new CoverUI({selection:()=>[...selected].map(path=>({path,covers:entries.get(path)?.covers??[]})),displayed:()=>({path:[...selected][0],covers:entries.get([...selected][0])?.covers??[],index:0}),reason:()=>state.busy?'Wait for the current operation.':!selected.size?'Select files first.':state.desktop===false?'Cover editing requires the desktop app.':model.reason('add-cover'),clipboard:()=>state.clipboard!==false&&state.desktop!==false&&!state.pending,pending:()=>!!state.pending,mac:false,
    pickImage:async()=>{calls.push(['cover-picker']);return state.imageAnswer??null;},pickSave:async()=>{calls.push(['cover-save-picker']);return null;},
    invoke:async(command,args)=>{calls.push(['cover-invoke',command,args]);return command==='paste_cover_from_clipboard'?{token:'fixture',mime:'image/jpeg',data_base64:'AA=='}:command==='extract_cover'?{path:'/fixture/folder.jpg',exists:!!state.extractExists&&!args.overwrite}:undefined;},
    mutate:async(command,args)=>{calls.push(['cover-mutate',command,args]);return{errors:[]};},
    adjust:()=>calls.push(['adjust']),evaluate:async()=>'',modal,modalError:(_dialog,error)=>{throw error;},confirm:async()=>{calls.push(['cover-confirm']);return state.confirm!==false;},notify:message=>calls.push(['notice',message]),fail:error=>calls.push(['cover-fail',error]),show:()=>{},
  });
  const dependencies={state,calls,selected,entries,window,document,modal,coverUI,ConfigurationTransferUI,...toolbar,chapterRowReason,documentSelection,customizeToolbar:host=>{calls.push(['customize',host.custom?'custom':'main']);}};
  const setup=`
    const desktop=state.desktop??true,busy=state.busy??false,profileBootstrap={desktop:state.native??desktop};
    const runtimePlatform={family:desktop?'desktop':'android',folder_picker_supported:desktop,document_access_supported:!desktop,...state.platform};
    const sourcePolicy=()=>documentSelection([...selected],{states:state.documents??{},refreshError:null},runtimePlatform.document_access_supported);
    const dirtySelected=()=>!!state.dirty,paths=()=>[...selected],visible=()=>[...entries.values()];
    const fileOps={reason:()=>''},selectedFiles=()=>[],mp4ChapterFiles=()=>({files:state.chapters===false?0:1});
    const history={undo:null},repeatState=()=>({kind:'none'}),tagClipboard=null,listRoots=[];
    const columnLayout={sort:state.sort??[{id:'filename',ascending:true}]},tableColumns=()=>[{id:'filename'}],editKey=()=> '_FILENAME_EXT';
    let listOrder='columns';const sourceOrder=undefined,refreshList=()=>calls.push(['list-order',listOrder]);let orderCacheKey='',filterCacheKey='';
    const toolbarConfig={shown:true,items:followups.map(id=>({id,visible:true}))},customToolbar=toolbarConfig;
    const esc=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c])),glyphs={},render=()=>{};
    const toolsUI={tools:[]},actionUI={groupNames:[],initialize:async()=>{calls.push(['actions-initialize']);}};
    const tagSourcesUI={lastOperation:'source-musicbrainz',lastBuiltInOperation:'source-musicbrainz',keyReason:()=>''},lastWebSource=()=>undefined;
    const customItemReason=()=>'',saveToolbar=async config=>{calls.push(['main-toolbar-save',config]);},saveCustomToolbar=async config=>{calls.push(['custom-toolbar-save',config]);},macKeyboard=()=>false;
    const applyColumns=async config=>calls.push(['columns-save',config]);
    const copyAsTextDialog=()=>calls.push(['copy-text']),modalError=(_dialog,error)=>{throw error;},optionsUI={page:()=>({}),settings:{tags:{}},updatePage:async()=>{}};
    const removeTagTypesDialog=()=>calls.push(['remove-tag-types']),runUtil=async util=>calls.push(['util',util]);
    const messages={confirm:async()=>{calls.push(['confirm']);return state.confirm!==false;}},stripChaptersPrompt=()=>'',mutate=async command=>calls.push(['mutate',command]);
    const shortcutsDialog=()=>calls.push(['shortcuts']),menuShortcuts=()=>[],EXTRA_SHORTCUTS=[],checkForUpdates=async startup=>calls.push(['updates',startup]);
    const audioLicenses=[],themedAsset=()=>'',appVersion='fixture';
    const invoke=async(command,args)=>{calls.push(['invoke',command,args]);return command==='audio_fingerprint_backend'?{chromaprint:'fixture',ffmpeg:'fixture'}:command==='configuration_transfer_preview'?{token:'reviewed',version:2,source_hash:'fixture',files:['options.json'],export_assets:[],preference_keys:[],preferences_available:false,workspace:null,workspace_available:true,unavailable:[],warnings:[],replaces_snapshot:false}:command==='configuration_transfer_apply'?{pending:{id:'pending-reload'}}:[];};
    const open=async options=>{calls.push(['open',options.title]);return state.openAnswer??null;},save=async options=>{calls.push(['save',options.title]);return null;},configurationFileName=()=> 'fixture.zip',CONFIGURATION_FILTER={},ask=async()=>{calls.push(['restore-confirm']);return state.confirm!==false;};
    const helperLists={flush:async()=>calls.push(['lists-flush']),pause:()=>calls.push(['lists-pause'])},drafts={size:0,clear:()=>calls.push(['drafts-clear'])},basename=value=>value.split('/').pop();
    const configurationTransferUI=new ConfigurationTransferUI({invoke,pickLoad:()=>open({title:'Load configuration'}),pickSave:()=>save({title:'Save configuration'}),capture:()=>({version:1,preferences:{},workspace:null}),flush:async()=>calls.push(['configuration-flush']),prepare:async()=>{calls.push(['prepare-configuration']);return state.confirm!==false;},inspectWorkspace:async()=>[],hold:held=>calls.push(['configuration-hold',held]),pause:()=>calls.push(['configuration-pause']),resume:()=>calls.push(['configuration-resume']),reload:()=>window.location.reload(),report:text=>calls.push(['configuration-report',text]),fail:error=>{throw error;}});
    const listOperation=async op=>calls.push(['list',op]);
    let errors=[],status='';
    ${declaration('utilOperations')} ${declaration('listOperations')} ${declaration('tagSourceOperations')} ${declaration('coverOperations')}
  `;
  const code=transpile(`${setup}\n${functions(['activeSort','setListOrder','lastSourceOp','operationReason','enabled','listEnabled','button','toolbarMarkup','customToolbarMarkup','customToolbarChoices','operateImpl','operate'])}\nreturn{enabled,reason:operationReason,render:toolbarMarkup,custom:customToolbarMarkup,choices:customToolbarChoices,operate,errors:()=>errors};`);
  model=new Function(...Object.keys(dependencies),'followups',code)(...Object.values(dependencies),followups);
  return{...model,window,document,calls,dialogs,coverUI};
}

test('all followups render and dispatch through production main guards to their existing handlers',async()=>{
  const model=runtime();
  try{
    for(const id of followups){assert.ok(toolbar.TOOLBAR_COMMANDS[id],id);assert.equal(model.enabled(id),true,id);assert.match(model.render(),new RegExp(`data-op="${id}"`),id);assert.match(model.custom(),new RegExp(`data-op="${id}"`),id);assert.ok(model.choices().some(choice=>choice.id===id),id);
      model.calls.length=0;const operation=model.operate(id);if(id==='cover-description'){await Promise.resolve();model.dialogs.at(-1).close();}await operation;assert.deepEqual(model.errors(),[],id);assert.ok(model.calls.length,id);
      const expected={
        'copy-text':['copy-text'],'remove-tag-types':['remove-tag-types'],'strip-mp4-chapters':['confirm'],'util-unsupported':['util','remove_unsupported_fields'],'util-optimize':['util','optimize'],
        'help-docs':['invoke','open_help_page',{page:'documentation'}],'help-issues':['invoke','open_help_page',{page:'issues'}],'help-releases':['invoke','open_help_page',{page:'releases'}],'help-shortcuts':['shortcuts'],'help-updates':['updates',false],about:['dialog','About TagGroove'],
        columns:['list','columns'],'autosize-columns':['list','autosize-columns'],'shuffle-list':['list','shuffle-list'],'list-unsorted':['list-order','manual'],'toolbar-customize':['customize','main'],'custom-toolbar-customize':['actions-initialize'],'config-save':['save','Save configuration'],'config-load':['open','Load configuration'],'config-folder':['invoke','open_configuration_folder',undefined],
        'cover-replace':['cover-picker'],'cover-extract':['dialog','Extract cover'],'cover-adjust':['adjust'],'cover-description':['dialog','Set cover description'],'cover-cut':['cover-invoke','copy_cover_to_clipboard',{path:'/Music/a.mp3',index:0}],'cover-copy':['cover-invoke','copy_cover_to_clipboard',{path:'/Music/a.mp3',index:0}],'cover-paste':['cover-invoke','paste_cover_from_clipboard',undefined],
      }[id];
      if(expected)assert.deepEqual(model.calls[0],expected,id);
      else assert.match(model.calls[0][0],/^(main-toolbar-save|custom-toolbar-save|columns-save)$/,id);
      model.dialogs.forEach(dialog=>dialog.close());
    }
  }finally{model.window.happyDOM.abort();}
});

test('cover aliases use real menu eligibility, preserve readable art and refuse stale invocations',async()=>{
  const cases=[
    [{selection:[]},['cover-replace','cover-extract','cover-adjust','cover-description','cover-cut','cover-copy','cover-paste']],
    [{covers:[]},['cover-replace','cover-extract','cover-adjust','cover-description','cover-cut','cover-copy']],
    [{busy:true},['cover-replace','cover-extract','cover-adjust','cover-description','cover-cut','cover-copy','cover-paste']],
    [{dirty:true},['cover-replace','cover-adjust','cover-description','cover-cut','cover-paste']],
    [{pending:true},['cover-extract','cover-cut','cover-copy','cover-paste']],
    [{clipboard:false},['cover-cut','cover-copy','cover-paste']],
    [{selection:['/Music/book.cue\u00011'],list:['/Music/book.cue\u00011']},['cover-replace','cover-adjust','cover-description','cover-cut','cover-paste']],
    [{selection:['content://provider/a'],list:['content://provider/a'],documents:{'content://provider/a':{writable:false,state:'ready'}}},['cover-replace','cover-adjust','cover-description','cover-cut','cover-paste']],
  ];
  for(const [state,ids]of cases){const model=runtime(state);try{for(const id of ids){assert.equal(model.enabled(id),false,`${JSON.stringify(state)} ${id}`);model.document.body.innerHTML=model.render();assert.equal(model.document.querySelector(`[data-op="${id}"]`).disabled,true,id);await model.operate(id);assert.deepEqual(model.calls,[],`stale ${id}`);}}finally{model.window.happyDOM.abort();}}
  for(const state of [{dirty:true},{selection:['content://provider/a'],list:['content://provider/a'],documents:{'content://provider/a':{writable:false,state:'ready'}}}]){const model=runtime(state);try{assert.equal(model.enabled('cover-copy'),true);assert.equal(model.enabled('cover-extract'),true);}finally{model.window.happyDOM.abort();}}
  const varying=runtime({selection:['/Music/a.mp3','/Music/b.mp3'],coversByPath:{'/Music/a.mp3':[art('a')],'/Music/b.mp3':[art('b')]}});
  try{for(const id of ['cover-replace','cover-extract','cover-cut','cover-copy'])assert.equal(varying.enabled(id),false,id);for(const id of ['cover-adjust','cover-description','cover-paste'])assert.equal(varying.enabled(id),true,id);}finally{varying.window.happyDOM.abort();}
});

test('cover aliases retain production clipboard payloads, cover plans and description dialog cancellation',async()=>{
  const model=runtime({imageAnswer:'/fixture/replacement.png'});
  try{
    await model.operate('cover-replace');
    assert.deepEqual(model.calls.find(call=>call[0]==='cover-mutate'),['cover-mutate','apply_cover_plan',{items:[{original:{digest:'front',picture_type:'CoverFront',description:''},image:{path:'/fixture/replacement.png'},picture_type:'CoverFront',description:'',all_files:false,occurrence:0}],label:'Replace cover'}]);
    model.calls.length=0;await model.operate('cover-copy');assert.deepEqual(model.calls[0],['cover-invoke','copy_cover_to_clipboard',{path:'/Music/a.mp3',index:0}]);assert.equal(model.calls.some(call=>call[0]==='cover-mutate'),false);
    model.calls.length=0;await model.operate('cover-cut');assert.deepEqual(model.calls.find(call=>call[0]==='cover-mutate'),['cover-mutate','apply_cover_plan',{items:[],label:'Cut cover'}]);
    model.calls.length=0;await model.operate('cover-paste');assert.deepEqual(model.calls.find(call=>call[0]==='cover-mutate'),['cover-mutate','add_cover_image',{image:{staged:'fixture'},pictureType:null}]);
    model.calls.length=0;let operation=model.operate('cover-description');await Promise.resolve();model.dialogs.at(-1).close();await operation;assert.equal(model.calls.some(call=>call[0]==='cover-mutate'),false);
    model.calls.length=0;operation=model.operate('cover-description');await Promise.resolve();const dialog=model.dialogs.at(-1);dialog.querySelector('#cover-description-value').value='Front sleeve';dialog.querySelector('#cover-description-ok').click();await operation;
    assert.equal(model.calls.find(call=>call[0]==='cover-mutate')[2].items[0].description,'Front sleeve');
  }finally{model.window.happyDOM.abort();}
});

test('extract alias retains existing-file confirmation and cancelled overwrite never retries',async()=>{
  for(const confirm of [false,true]){const model=runtime({confirm,extractExists:true});try{
    await model.operate('cover-extract');await Promise.resolve();const dialog=model.dialogs.at(-1);assert.equal(dialog.querySelector('#cover-extract-save').disabled,false);dialog.querySelector('#cover-extract-save').click();
    await model.window.happyDOM.waitUntilComplete();
    const writes=model.calls.filter(call=>call[0]==='cover-invoke'&&call[1]==='extract_cover');
    assert.equal(writes.length,confirm?2:1);assert.deepEqual(writes[0][2],{path:'/Music/a.mp3',index:0,output:'folder',overwrite:false});if(confirm)assert.equal(writes[1][2].overwrite,true);
    assert.equal(model.calls.filter(call=>call[0]==='cover-confirm').length,1);
  }finally{model.window.happyDOM.abort();}}
});

test('existing menu safety keeps chapter removal confirmation and cancelled setup restore intact',async()=>{
  for(const confirm of [false,true]){const model=runtime({confirm});try{await model.operate('strip-mp4-chapters');assert.equal(model.calls.some(call=>call[0]==='mutate'),confirm);}finally{model.window.happyDOM.abort();}}
  const settleReview=async model=>{for(let i=0;i<12&&!model.document.querySelector('.configuration-transfer-dialog');i++)await Promise.resolve();const dialog=model.document.querySelector('.configuration-transfer-dialog');assert.ok(dialog);return dialog;};
  const model=runtime({openAnswer:'/fixture/setup.zip'});try{const operation=model.operate('config-load'),dialog=await settleReview(model);dialog.querySelector('[data-cancel]').click();await operation;assert.deepEqual(model.calls.map(call=>call[0]),['open','configuration-flush','invoke']);assert.equal(model.calls[2][1],'configuration_transfer_preview');assert.equal(model.calls.some(call=>call[1]==='configuration_transfer_apply'),false);}finally{model.window.happyDOM.abort();}
  for(const confirm of [false,true]){const restored=runtime({confirm,openAnswer:'/fixture/setup.zip'});try{const operation=restored.operate('config-load'),dialog=await settleReview(restored);dialog.querySelector('[data-apply]').click();for(let i=0;i<12;i++)await Promise.resolve();
    assert.equal(restored.calls.some(call=>call[0]==='prepare-configuration'),true);
    assert.equal(restored.calls.some(call=>call[1]==='configuration_transfer_apply'),confirm);
    assert.equal(restored.calls.some(call=>call[0]==='reload'),confirm);
    if(confirm){assert.equal(restored.calls.find(call=>call[1]==='configuration_transfer_apply')[2].token,'reviewed');assert.equal(restored.calls.some(call=>call[0]==='configuration-pause'),true);}else dialog.querySelector('[data-cancel]').click();
    await operation;assert.equal(restored.calls.some(call=>call[0]==='drafts-clear'),false,'the old window never discards drafts before recovery reload');
  }finally{restored.window.happyDOM.abort();}}
  for(const [state,id]of [[{selection:[]},'copy-text'],[{chapters:false},'strip-mp4-chapters'],[{dirty:true},'util-optimize'],[{busy:true},'help-updates'],[{list:['/Music/a.mp3']},'shuffle-list'],[{sort:[]},'list-unsorted'],[{native:false},'config-load'],[{native:false},'config-save'],[{desktop:false},'config-load'],[{desktop:false},'config-save']]){const model=runtime(state);try{assert.equal(model.enabled(id),false,id);await model.operate(id);assert.deepEqual(model.calls,[],id);}finally{model.window.happyDOM.abort();}}
});
