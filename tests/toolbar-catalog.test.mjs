import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
const root=new URL('../',import.meta.url);
const load=async name=>{
  const source=await readFile(new URL(`src/${name}.ts`,root),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const toolbar=await load('toolbar');
const {FileOpsUI}=await load('file-ops');
const {chapterRowReason}=await load('chapters');
const {documentSelection}=await load('document-state');
const {TagSourcesUI,sourceRequest,sourceKeyReason}=await load('tag-sources');
const main=await readFile(new URL('src/main.ts',root),'utf8');
const parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
const js=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const functions=names=>names.map(name=>{
  const node=parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name);
  assert.ok(node,`actual main function ${name}`);return node.getText(parsed);
}).join('\n');
const declaration=name=>{
  let found;const visit=node=>{if(ts.isVariableDeclaration(node)&&node.name.getText(parsed)===name)found=node;ts.forEachChild(node,visit);};visit(parsed);
  assert.ok(found,`actual main declaration ${name}`);return `const ${found.getText(parsed)};`;
};
const legacy=['folder','files','save','reload','remove-tags','undo','redo','filename-tags','tags-filename','filename-filename','text-tags','tag-tag','groups','action','number','extended','playlist-all','export-config','find','refresh','source-musicbrainz','source-discogs','cover-manage','file-reveal','file-play','toggle-panel','toggle-filter','options'];
const added=[
  'file-copy','file-move','file-delete','file-delete-permanent',
  'dir-add','dir-up','dir-back','dir-forward','dir-favorite','dir-manage',
  'select-all','unselect-all','invert-selection','select-next','select-similar','select-next-similar','remove-from-list','clear-list','edit-cell',
  'add-cover','remove-cover','playlist-selected','playlist-load','playlist-folders','playlist-generate',
  'source-discogs-release','source-freedb','source-local-freedb','source-cover-freedb','source-cover-musicbrainz','source-cover-discogs','source-itunes','source-deezer','source-lastfm','source-acoustid','source-lrclib','source-cover-itunes','source-cover-deezer','source-cover-fanart','source-last','source-last-quick',
];
const oldDefaults=['folder','files','separator-1','save','reload','separator-2','filename-tags','tags-filename','separator-3','groups','number','undo','redo','remove-tags','extended','filename-filename','text-tags','tag-tag','action','playlist-all','export-config','find','refresh','source-musicbrainz','source-discogs','cover-manage','file-reveal','file-play','toggle-panel','toggle-filter','options'];

test('expanded catalog preserves the legacy 28 commands and all original default positions',()=>{
  assert.deepEqual(Object.keys(toolbar.TOOLBAR_COMMANDS).slice(0,28),legacy);
  const defaults=toolbar.defaultToolbar();
  assert.deepEqual(defaults.items.slice(0,oldDefaults.length).map(item=>item.id),oldDefaults);
  assert.deepEqual(toolbar.visibleItems(defaults.items).filter(item=>!toolbar.isDivider(item.id)).map(item=>item.id),['folder','files','save','reload','filename-tags','tags-filename','groups','number']);
  assert.deepEqual(defaults.items.slice(oldDefaults.length,oldDefaults.length+added.length),added.map(id=>({id,visible:false})));
  assert.deepEqual(new Set(defaults.items.filter(item=>!toolbar.isDivider(item.id)).map(item=>item.id)),new Set(Object.keys(toolbar.TOOLBAR_COMMANDS)));
});

test('older saved layouts gain hidden choices while authored labels, order and custom references survive',()=>{
  const saved={shown:false,items:[{id:'file-play',visible:true,label:'  Listen  '},{id:'separator-9',visible:true},{id:'save',visible:false,label:'Save safely'}]};
  const original=structuredClone(saved),normalized=toolbar.normalizeToolbar(saved);
  assert.equal(normalized.shown,false);assert.deepEqual(saved,original);
  assert.deepEqual(normalized.items.slice(0,3),[{id:'file-play',visible:true,label:'Listen'},saved.items[1],saved.items[2]]);
  for(const id of added){assert.equal(normalized.items.filter(item=>item.id===id).length,1,id);assert.equal(normalized.items.find(item=>item.id===id).visible,false,id);}
  const custom={shown:false,items:[{id:'tool:Inspect track',visible:true,label:'Inspect'},{id:'action:House#Credit cleanup',visible:false,label:'Credits'},{id:'spacer-4',visible:true},{id:'file-move',visible:true,label:'My move'}]};
  assert.deepEqual(toolbar.normalizeCustomToolbar(JSON.parse(JSON.stringify(custom))),custom);
  const fresh=toolbar.normalizeToolbar({shown:true,items:added.map(id=>({id,visible:true}))});
  assert.deepEqual(fresh.items.slice(0,added.length),added.map(id=>({id,visible:true})));
});

// Execute the production main render/guard/dispatch functions. Only external
// effects are spies; eligibility uses the actual chapter, document, folder
// and credential policies. This is DOM/control-flow evidence, not native UI.
function runtime(state={}){
  const selected=new Set(state.selection??['/Music/a.wav']);
  const entries=new Map((state.list??['/Music/a.wav','/Music/b.wav']).map(path=>[path,{path,filename:path.split('/').pop()}]));
  const files=new FileOpsUI({mac:false,selection:()=>[...selected].map(path=>({path,name:path})),invoke:async()=>{}});
  files.state.favorites=state.favorites??['/Favorites'];if(state.directory!==null){files.history.visit('/Music');files.history.visit(state.directory??'/Music/Album');files.history.visit('/Music/Next');files.history.move('back');}
  const calls=[],window=state.window??new Window();
  const dependencies={state,selected,entries,calls,added,document:window.document,chapterRowReason,documentSelection,sourceRequest,sourceKeyReason,...toolbar,
    fileOps:{reason:op=>files.reason(op),operate:async op=>{calls.push(['file',op]);}},
  };
  const setup=`
    const desktop=state.desktop??true,busy=state.busy??false;
    const runtimePlatform={family:desktop?'desktop':'android',folder_picker_supported:desktop,document_access_supported:!desktop,...state.platform};
    const sourcePolicy=()=>documentSelection([...selected],{states:state.documents??{},refreshError:null},runtimePlatform.document_access_supported);
    const visible=()=>[...entries.values()],dirtySelected=()=>!!state.dirty,paths=()=>[...selected];
    let anchor,coverIndex=0;const refreshSelection=()=>{calls.push(['selection','all']);};
    const utilOperations={},selectedFiles=()=>[],mp4ChapterFiles=()=>({files:0}),listRoots=['/Music'];
    const columnLayout={sort:[]},tableColumns=()=>[{id:'filename'}],editKey=()=> '_FILENAME_EXT';
    const history={undo:null},repeatState=()=>({kind:'none'}),tagClipboard=null;
    const coverUI={canRemove:()=>!!state.covers,run:async op=>{calls.push(['cover',op]);}};
    const tagSourcesUI=state.sourceUI??{lastOperation:state.lastSource??'source-musicbrainz',lastBuiltInOperation:state.lastBuiltIn??state.lastSource??'source-musicbrainz',keyReason:op=>{const request=sourceRequest(op);return request?sourceKeyReason(request.provider,state.credentials??null):'';},run:async op=>{calls.push(['source',op]);}};
    const lastWebSource=()=>state.webSource?.id===tagSourcesUI.lastWeb?state.webSource:undefined;
    const webSourcesUI={run:async source=>{calls.push(['web',source.id]);await tagSourcesUI.open('web',{web:source});}};
    const playlistUI={create:async scope=>{calls.push(['playlist',scope]);},createPerFolder:async()=>{calls.push(['playlist','folders']);},generate:()=>{calls.push(['playlist','generate']);}};
    const open=async()=>{calls.push(['picker','playlist']);return null;},converter=rename=>{calls.push(['converter',rename]);};
    const render=()=>{},customItemReason=()=>'',glyphs={},esc=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    const customToolbar={shown:true,items:state.items??added.map(id=>({id,visible:true}))},toolbarConfig=customToolbar;
    const toolsUI={tools:[{name:'Inspect track'}]},actionUI={groupNames:['House#Credit cleanup']};
    const listOperation=async op=>{calls.push(['list',op]);};
    let errors=[],status='';
    ${declaration('listOperations')} ${declaration('tagSourceOperations')} ${declaration('coverOperations')}
  `;
  const code=js(`${setup}\n${functions(['lastSourceOp','operationReason','enabled','listEnabled','button','toolbarMarkup','customToolbarMarkup','customToolbarChoices','operateImpl','operate'])}\nreturn{enabled,reason:operationReason,render:toolbarMarkup,custom:customToolbarMarkup,choices:customToolbarChoices,operate,errors:()=>errors};`);
  const model=new Function(...Object.keys(dependencies),code)(...Object.values(dependencies));
  return{...model,calls,selected,entries,window};
}

test('every added command renders on both toolbars and has a real existing route',async()=>{
  const model=runtime({covers:true});
  try{
    for(const id of added)assert.ok(toolbar.TOOLBAR_COMMANDS[id],id);
    const main=model.render(),custom=model.custom();
    for(const id of added){assert.ok(toolbar.TOOLBAR_COMMANDS[id],id);assert.match(main,new RegExp(`data-op="${id}"`),id);assert.match(custom,new RegExp(`data-op="${id}"`),id);assert.equal(model.enabled(id),true,id);}
    const choices=model.choices();assert.equal(choices.filter(item=>item.group==='Commands').length,Object.keys(toolbar.TOOLBAR_COMMANDS).length);
    assert.deepEqual(choices.slice(-2).map(item=>item.id),['tool:Inspect track','action:House#Credit cleanup']);
    for(const id of added){model.calls.length=0;await model.operate(id);assert.deepEqual(model.errors(),[],id);
      const expected=id.startsWith('file-')||id.startsWith('dir-')?['file',id]:id.startsWith('source-')?['source',id]:id==='add-cover'?['cover','add']:id==='remove-cover'?['cover','remove']:id==='playlist-selected'?['playlist','selected']:id==='playlist-folders'?['playlist','folders']:id==='playlist-generate'?['playlist','generate']:id==='playlist-load'?['picker','playlist']:id==='select-all'?['selection','all']:['list',id];
      if(expected)assert.deepEqual(model.calls,[expected],id);else assert.equal(model.selected.size,model.entries.size);
    }
    model.calls.length=0;await model.operate('tags-filename');assert.deepEqual(model.calls,[['converter',true]],'existing rename route remains available');
  }finally{model.window.happyDOM.abort();}
});

test('added toolbar buttons retain the actual unsupported-context and dynamic eligibility guards',async()=>{
  const cases=[
    [{selection:[]},'file-move','Select files first.'],
    [{dirty:true},'file-copy','Save pending changes first.'],
    [{selection:['/Music/book.m4b\u00011'],list:['/Music/book.m4b\u00011']},'file-delete','Chapter rows are not files of their own.'],
    [{selection:['content://provider/a'],documents:{'content://provider/a':{writable:false,state:'ready'}}},'file-move','not available for provider documents'],
    [{selection:['content://provider/a'],documents:{'content://provider/a':{writable:false,state:'ready'}}},'add-cover','read-only source'],
    [{desktop:false},'dir-up','Folder access is not available here.'],
    [{favorites:[]},'dir-favorite','Add a favorite'],
    [{directory:'/'},'dir-up','This is the top folder.'],
    [{selection:['/Music/book.m4b\u00011'],list:['/Music/book.m4b\u00011']},'playlist-selected','Chapter rows are not files of their own.'],
    [{list:['/Music/book.m4b\u00011']},'playlist-generate','Chapter rows are not files of their own.'],
    [{credentials:{acoustid_key_set:false,acoustid_built_in:false}},'source-acoustid','AcoustID needs an API key.'],
    [{lastSource:'source-acoustid',credentials:{acoustid_key_set:false,acoustid_built_in:false}},'source-last','AcoustID needs an API key.'],
    [{lastBuiltIn:'source-acoustid',credentials:{acoustid_key_set:false,acoustid_built_in:false}},'source-last-quick','AcoustID needs an API key.'],
    [{dirty:true,lastBuiltIn:'source-deezer'},'source-last-quick','Save pending changes first.'],
    [{selection:['content://provider/a'],documents:{'content://provider/a':{writable:false,state:'ready'}},lastBuiltIn:'source-deezer'},'source-last-quick','read-only source'],
    [{selection:[],lastBuiltIn:'source-deezer'},'source-last-quick',''],
    [{busy:true,lastBuiltIn:'source-deezer'},'source-last-quick',''],
    [{covers:false},'remove-cover',''],
    [{busy:true},'playlist-selected',''],
  ];
  for(const [state,id,reason] of cases){const model=runtime({...state,items:[{id,visible:true}]});try{
    assert.ok(toolbar.TOOLBAR_COMMANDS[id],id);assert.equal(model.enabled(id),false,id);if(reason)assert.ok(model.reason(id).includes(reason),`${id}: ${model.reason(id)}`);
    model.window.document.body.innerHTML=model.render();const button=model.window.document.querySelector(`[data-op="${id}"]`);assert.equal(button.disabled,true,id);if(reason)assert.ok(button.title.includes(reason),id);
    await model.operate(id);assert.deepEqual(model.calls,[],`stale ${id} invocation must still be refused`);assert.deepEqual(model.errors(),[],id);
  }finally{model.window.happyDOM.abort();}}
});

// Exercise real source dialogs and persisted state through the production main
// guards/dispatcher. Provider responses are fixtures, with no network or writes.
async function sourceRuntime(saved,check){
  const window=new Window({url:'http://localhost:1420'}),calls=[];
  const descriptors=new Map(['window','document'].map(key=>[key,Object.getOwnPropertyDescriptor(globalThis,key)]));
  globalThis.window=window;globalThis.document=window.document;
  for(const [key,value] of Object.entries(saved))window.localStorage.setItem(`opentag.tagSources.${key}`,value);
  const webSource={id:'installed-fixture',name:'Installed fixture',menu:['Installed fixture'],search_by:[]};
  const host={files:()=>[{path:'/Music/a.wav',name:'a.wav',artist:'Artist',album:'Album',title:'Track',album_artist:'',track:'1',disc:'1',length:200}],reason:()=>'',options:()=>({}),evaluate:async()=>'',apply:async()=>{throw new Error('No metadata apply expected');},pickSave:async()=>null,
    invoke:async(command,args)=>{calls.push([command,args]);if(command==='tag_source_search')return{results:[],total:0};if(command==='load_tag_source_settings')return{acoustid_key_set:false,acoustid_built_in:false};},
  };
  const create=()=>{const ui=new TagSourcesUI(host);return{ui,model:runtime({window,sourceUI:ui,webSource})};};
  const settle=async()=>{await window.happyDOM.waitUntilComplete();};
  const close=async()=>{document.querySelector('dialog')?.close();await settle();};
  try{await check({window,calls,webSource,create,close,settle});}
  finally{for(const [key,descriptor]of descriptors){if(descriptor)Object.defineProperty(globalThis,key,descriptor);else delete globalThis[key];}window.happyDOM.abort();}
}

test('quick source retains Deezer across an installed web source and a UI reload; ordinary last source still opens web',async()=>{
  await sourceRuntime({},async({window,calls,webSource,create,close,settle})=>{
    let{ui,model}=create();
    await ui.run('source-deezer');await close();
    await ui.open('web',{web:webSource});await close();
    ({ui,model}=create()); // Reconstruct the UI from persisted preferences.
    assert.equal(ui.lastWeb,webSource.id);
    await model.operate('source-last');await settle();
    assert.deepEqual(model.calls,[['web',webSource.id]]);
    assert.match(document.querySelector('dialog header').textContent,/Installed fixture/);await close();
    calls.length=0;
    await model.operate('source-last-quick');await settle();
    assert.deepEqual(model.errors(),[]);
    assert.equal(calls.find(([command])=>command==='tag_source_search')?.[1].provider,'deezer');
    assert.match(document.querySelector('dialog header').textContent,/Deezer/);
    assert.equal(window.localStorage.getItem('opentag.tagSources.lastBuiltIn'),'source-deezer');
    assert.equal(ui.lastWeb,null,'quick becomes the most recently opened source');
    await close();
  });
});

test('remembered built-in keeps cover and release modes, and falls back compatibly for older or invalid preferences',async()=>{
  await sourceRuntime({},async({window,webSource,create,close,settle})=>{
    let{ui,model}=create();
    await ui.run('source-cover-deezer');await close();await ui.open('web',{web:webSource});await close();
    ({ui,model}=create());await model.operate('source-last-quick');await settle();
    assert.match(document.querySelector('dialog header').textContent,/Deezer cover art/);await close();
    await ui.run('source-discogs-release');await close();await ui.open('web',{web:webSource});await close();
    ({ui,model}=create());await model.operate('source-last-quick');await settle();
    assert.match(document.querySelector('dialog header').textContent,/Discogs Release ID/);
    assert.equal(document.querySelector('#ts-mode').value,'release_id');await close();
    for(const [saved,lastSource,legacy,expected]of [
      [null,'source-cover-itunes','discogs','source-cover-itunes'],
      [null,'source-deezer',null,'source-deezer'],
      [null,`web:${webSource.id}`,'discogs','source-discogs'],
      [null,`web:${webSource.id}`,null,'source-musicbrainz'],
      [null,null,null,'source-musicbrainz'],
      ['web:invalid','source-deezer',null,'source-deezer'],
      ['bogus','bogus','discogs','source-discogs'],
    ]){
      window.localStorage.clear();
      for(const [key,value]of [['lastBuiltIn',saved],['lastSource',lastSource],['last',legacy]])if(value!==null)window.localStorage.setItem(`opentag.tagSources.${key}`,value);
      ({ui}=create());assert.equal(ui.lastBuiltInOperation,expected);
      await ui.open('web',{web:webSource});await close();
      ({ui}=create());assert.equal(ui.lastBuiltInOperation,expected,'opening web first must seed the older profile before replacing lastSource');
    }
  });
});

test('quick source guards use the remembered built-in provider after a web source',async()=>{
  await sourceRuntime({},async({window,webSource,create,close})=>{
    const{ui,model}=create();await ui.run('source-acoustid');await close();await ui.open('web',{web:webSource});await close();
    await ui.refreshCredentials();
    assert.equal(model.enabled('source-last-quick'),false);
    assert.match(model.reason('source-last-quick'),/AcoustID needs an API key/);
    await model.operate('source-last-quick');assert.equal(document.querySelector('dialog'),null);
    assert.equal(window.localStorage.getItem('opentag.tagSources.lastSource'),`web:${webSource.id}`,'refused quick invocation preserves the universal last source');
  });
});


test('new command hints translate in bundled languages without changing saved user labels',async()=>{
  const {Translator}=await load('i18n/core');
  for(const locale of ['de','nl']){
    const catalog=JSON.parse(await readFile(new URL(`src/i18n/catalogs/${locale}.json`,root),'utf8'));
    const translator=new Translator(locale,catalog);
    for(const id of added){const hint=toolbar.itemHint(id);assert.notEqual(translator.translate(hint),hint,`${locale}: ${id}`);}
    const item={id:'file-move',label:'Help $& {name}',visible:true};
    assert.equal(toolbar.userLabel(item),true);assert.equal(toolbar.itemLabel(item),'Help $& {name}');
  }
});

test('actual custom toolbar dialog cancels and saves an added file command without replacing tool/action state',async()=>{
  const model=runtime(),descriptor=Object.getOwnPropertyDescriptor(globalThis,'document');globalThis.document=model.window.document;
  const config={shown:false,items:[{id:'tool:Inspect track',visible:true,label:'Inspect'},{id:'action:House#Credit cleanup',visible:false,label:'Credits'}]};
  const original=structuredClone(config);let saved;
  const host={config:()=>config,save:async value=>{saved=structuredClone(value);},mac:()=>false,custom:{choices:model.choices}};
  const add=()=>{const choice=document.querySelector('#toolbar-choice');choice.value='file-move';document.querySelector('[data-tb="add"]').click();};
  try{
    toolbar.customizeToolbar(host);add();document.querySelector('[data-close]').click();await model.window.happyDOM.waitUntilComplete();
    assert.equal(saved,undefined);assert.deepEqual(config,original);
    toolbar.customizeToolbar(host);add();document.querySelector('#toolbar-ok').click();await model.window.happyDOM.waitUntilComplete();
    assert.deepEqual(config,original);assert.equal(saved.shown,false);
    assert.deepEqual(saved.items.filter(item=>item.id!=='file-move'),original.items);
    assert.deepEqual(saved.items.find(item=>item.id==='file-move'),{id:'file-move',visible:true});
    assert.deepEqual(toolbar.normalizeCustomToolbar(JSON.parse(JSON.stringify(saved))),saved);
  }finally{if(descriptor)Object.defineProperty(globalThis,'document',descriptor);else delete globalThis.document;model.window.happyDOM.abort();}
});

test('actual generic button binding invokes the existing dispatcher for an added toolbar command',async()=>{
  let binding;const visit=node=>{if(ts.isExpressionStatement(node)){const text=node.getText(parsed);if(text.startsWith("document.querySelectorAll<HTMLButtonElement>('[data-op]')")&&text.includes("addEventListener('click'")&&text.includes('void operate('))binding=text;}ts.forEachChild(node,visit);};visit(parsed);
  assert.ok(binding,'actual main data-op click binding');
  const model=runtime({items:[{id:'file-move',visible:true}]});
  try{
    model.window.document.body.innerHTML=model.render();
    new Function('document','operate',js(binding))(model.window.document,model.operate);
    model.window.document.querySelector('[data-op="file-move"]').click();
    await model.window.happyDOM.waitUntilComplete();
    assert.deepEqual(model.calls,[['file','file-move']]);assert.deepEqual(model.errors(),[]);
  }finally{model.window.happyDOM.abort();}
});


test('the user guide names every new toolbar choice under LF and CRLF without inventing another operation',async()=>{
  const text=await readFile(new URL('docs/user-guide/customize/toolbars.md',root),'utf8');
  for(const guide of [text.replace(/\r\n/g,'\n'),text.replace(/\r?\n/g,'\r\n')]){
    for(const id of added)assert.ok(guide.includes(toolbar.TOOLBAR_COMMANDS[id].label),id);
    assert.match(guide,/Existing\s+button positions, custom labels and visibility stay intact/);
  }
});
