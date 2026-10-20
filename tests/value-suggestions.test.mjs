import test from 'node:test';
import assert from 'node:assert/strict';
import {loadProductionModule} from './helpers/load-production-module.mjs';
import {Window} from 'happy-dom';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=name=>loadProductionModule(`${name}.ts`);
const {TagPanel}=await load('tag-panel'),{completeInput}=await load('general-options'),{CellEditor}=await load('cell-editor'),{ExtendedTags}=await load('extended-tags'),{panelKey}=await load('action-ui');
const main=await readFile(new URL('../src/main.ts',import.meta.url),'utf8'),parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
const statement=name=>parsed.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text===name)?.getText(parsed);
const variable=name=>parsed.statements.flatMap(node=>ts.isVariableStatement(node)?[...node.declarationList.declarations]:[]).find(node=>node.name.getText(parsed)===name)?.getText(parsed);
function evaluate(source,env){const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;return new Function(...Object.keys(env),js)(...Object.values(env));}
const key=(p,input,value,extra={})=>{const event=new p.window.KeyboardEvent('keydown',{key:value,bubbles:true,cancelable:true,...extra});input.dispatchEvent(event);return event;};
function page(){const window=new Window(),names=['window','document','HTMLElement','HTMLInputElement','HTMLTextAreaElement','MutationObserver'];const previous=names.map(name=>Object.getOwnPropertyDescriptor(globalThis,name));names.forEach(name=>globalThis[name]=name==='window'?window:window[name]);return{window,document:window.document,close(){window.happyDOM.abort();names.forEach((name,i)=>{if(previous[i])Object.defineProperty(globalThis,name,previous[i]);else delete globalThis[name];});}};}
const type=(p,input,value,inputType='insertText')=>{input.value=value;input.setSelectionRange(value.length,value.length);input.dispatchEvent(new p.window.InputEvent('input',{bubbles:true,inputType,data:value.slice(-1)}));};
const options=p=>[...p.document.querySelectorAll('.value-suggestions [role=option]')].map(el=>el.textContent);
function panelPage(values=['Rock','Rockabilly','Pop'],enabled=true){const p=page(),drafts=[],state={enabled,disabled:false,reads:0,values};const panel=new TagPanel({persistent:()=>false,values:()=>{state.reads++;return state.values;},disabled:()=>state.disabled,directories:()=>({selected:[],all:[]}),draft:(field,value)=>drafts.push([field,value]),changed:()=>{},keep:field=>drafts.push(['keep',field]),autocomplete:()=>state.enabled?completeInput:undefined});p.document.body.innerHTML=panel.fieldsHTML();state.beforeBindReads=state.reads;panel.bind(p.document.body);return{...p,panel,drafts,state};}
test('production tag panel retains inline completion and shows every matching value',()=>{const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Ro');assert.equal(input.value,'Rock');assert.deepEqual([input.selectionStart,input.selectionEnd],[2,4]);assert.deepEqual(options(p),['Rock','Rockabilly']);assert.deepEqual(p.drafts,[]);}finally{p.close();}});

test('filtering uses raw prefix, preserves casing and keeps exact/free text valid',()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();
    type(p,input,'ro');assert.equal(input.value,'rock');assert.deepEqual(options(p),['Rock','Rockabilly']);
    type(p,input,'Rock');assert.deepEqual(options(p),['Rock','Rockabilly']);
    type(p,input,'unlisted');assert.equal(input.value,'unlisted');assert.deepEqual(options(p),[]);
    type(p,input,'');assert.equal(input.value,'');assert.deepEqual(options(p),[]);
    p.state.values=['Roman','Rosy'];type(p,input,'Ro','insertFromPaste');assert.equal(input.value,'Ro');assert.deepEqual(options(p),['Roman','Rosy']);
    type(p,input,'R','deleteContentBackward');assert.equal(input.value,'R');assert.deepEqual(options(p),['Roman','Rosy']);
  }finally{p.close();}
});
test('automatic list follows selection/custom/genre order and excludes manual commands',()=>{
  const p=panelPage(['Rock','Rockabilly']);try{
    p.panel.settings.field_values.lists.GENRE=['Roots','Rock'];
    const input=p.document.querySelector('[data-field="GENRE"]');type(p,input,'Ro');
    assert.deepEqual(options(p).slice(0,3),['Rock','Rockabilly','Roots']);assert.equal(options(p).filter(text=>text==='Rock').length,1);
    assert.ok(options(p).includes('Rock & Roll'));assert.ok(options(p).every(text=>text.toLowerCase().startsWith('ro')));
    assert.ok(!options(p).includes('< keep >'));assert.ok(!options(p).includes('< blank >'));
  }finally{p.close();}
});
test('Enter chooses only an explicitly navigated option, then returns to the editor policy',()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Ro');
    assert.equal(input.hasAttribute('aria-activedescendant'),false);assert.equal(key(p,input,'Enter').defaultPrevented,false);assert.deepEqual(p.drafts,[]);
    key(p,input,'ArrowDown');key(p,input,'ArrowDown');assert.equal(key(p,input,'Enter').defaultPrevented,true);
    assert.equal(input.value,'Rockabilly');assert.deepEqual(p.drafts,[['TITLE','Rockabilly']]);assert.deepEqual(options(p),[]);
    assert.equal(key(p,input,'Enter').defaultPrevented,false);
  }finally{p.close();}
});
test('Tab closes without choosing a highlighted alternative; Escape closes before editor cancellation',()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Ro');key(p,input,'ArrowDown');key(p,input,'ArrowDown');
    assert.equal(key(p,input,'Tab').defaultPrevented,false);assert.equal(input.value,'Rock');assert.deepEqual(p.drafts,[]);assert.deepEqual(options(p),[]);
    type(p,input,'Ro');assert.equal(key(p,input,'Escape').defaultPrevented,true);assert.equal(input.value,'Rock');assert.equal(key(p,input,'Escape').defaultPrevented,false);
  }finally{p.close();}
});
test('pointer selection prevents blur and stages the chosen value once without dispatching a save',()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Ro');
    const option=p.document.querySelectorAll('.value-suggestions [role=option]')[1],down=new p.window.PointerEvent('pointerdown',{bubbles:true,cancelable:true});
    option.dispatchEvent(down);assert.equal(down.defaultPrevented,true);assert.equal(p.document.activeElement,input);
    option.click();assert.equal(input.value,'Rockabilly');assert.deepEqual(p.drafts,[['TITLE','Rockabilly']]);assert.equal(input.getAttribute('aria-expanded'),'false');
  }finally{p.close();}
});
test('multiline panel values remain whole and unchanged until an explicit choice',()=>{
  const values=['Rock\nfirst line','Rock\nsecond line'],p=panelPage(values);try{const input=p.document.querySelector('[data-field="COMMENT"]');input.focus();type(p,input,'Rock\ns');
    assert.equal(input.value,'Rock\ns');assert.deepEqual([input.selectionStart,input.selectionEnd],[6,6]);assert.deepEqual(options(p),[values[1]]);
    assert.equal(key(p,input,'Enter').defaultPrevented,false);assert.equal(input.value,'Rock\ns');assert.deepEqual(p.drafts,[]);
    key(p,input,'ArrowDown');assert.equal(key(p,input,'Enter').defaultPrevented,true);assert.equal(input.value,values[1]);assert.deepEqual(p.drafts,[['COMMENT',values[1]]]);
    assert.equal(key(p,input,'Enter').defaultPrevented,false);
  }finally{p.close();}
});
test('manual full panel list and automatic matches close one another while keeping Keep and Blank',()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Ro');
    key(p,input,'ArrowDown',{altKey:true});assert.deepEqual(options(p),[]);
    const manual=[...p.document.querySelectorAll('.combo-list [role=option]')].map(el=>el.textContent);assert.ok(manual.includes('< keep >'));assert.ok(manual.includes('< blank >'));assert.ok(manual.includes('Pop'));
    type(p,input,'Ro');assert.deepEqual(options(p),['Rock','Rockabilly']);assert.equal(p.document.querySelectorAll('.combo-list').length,1);
    input.parentElement.querySelector('button').click();assert.deepEqual(options(p),[]);assert.equal(p.document.querySelectorAll('.combo-list').length,1);
  }finally{p.close();}
});
test('composition never reads candidates, changes the caret, or navigates stale choices; late commits remain valid',()=>{
  const p=panelPage(['東京','東京都']);try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'東');assert.equal(options(p).length,2);
    input.dispatchEvent(new p.window.CompositionEvent('compositionstart',{bubbles:true}));assert.deepEqual(options(p),[]);
    input.value='東';input.setSelectionRange(0,1);const before=p.state.reads;
    for(const inputType of ['insertText','insertCompositionText'])input.dispatchEvent(new p.window.InputEvent('input',{bubbles:true,inputType,isComposing:true}));
    for(const value of ['Enter','Escape','Tab','ArrowDown'])for(const isComposing of [true,false])assert.equal(key(p,input,value,{isComposing}).defaultPrevented,false);
    assert.equal(p.state.reads,before);assert.deepEqual([input.value,input.selectionStart,input.selectionEnd],['東',0,1]);assert.deepEqual(p.drafts,[]);
    input.dispatchEvent(new p.window.CompositionEvent('compositionend',{bubbles:true}));type(p,input,'東');assert.equal(input.value,'東京');assert.deepEqual(options(p),['東京','東京都']);
  }finally{p.close();}
});
test('disabled option, disabled/busy/read-only controls and composition notifications suppress candidate reads',()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');
    for(const mode of ['off','busy','disabled','readonly','composition']){
      p.state.enabled=mode!=='off';p.state.disabled=mode==='busy';input.disabled=mode==='disabled';input.readOnly=mode==='readonly';const before=p.state.reads;
      type(p,input,'Ro',mode==='composition'?'insertCompositionText':'insertText');assert.deepEqual(options(p),[],mode);assert.equal(input.value,'Ro',mode);assert.equal(p.state.reads,before,mode);
    }
  }finally{p.close();}
});
test('literal Unicode and untrusted markup are displayed as text, never elements',()=>{
  const p=panelPage(['Élan','Élève','<img src=x onerror=alert(1)>']);try{const input=p.document.querySelector('[data-field="TITLE"]');
    type(p,input,'é');assert.deepEqual(options(p),['Élan','Élève']);assert.equal(input.value,'élan');
    type(p,input,'<','insertFromPaste');assert.deepEqual(options(p),['<img src=x onerror=alert(1)>']);assert.equal(p.document.querySelector('.value-suggestions img'),null);
  }finally{p.close();}
});
test('all matches remain reachable with bounded option DOM and existing active-descendant nodes',()=>{
  const values=Array.from({length:2000},(_,i)=>`Rock ${String(i).padStart(4,'0')}`),p=panelPage(values);try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Rock');
    assert.ok(options(p).length<=16);key(p,input,'ArrowUp');const id=input.getAttribute('aria-activedescendant');assert.equal(p.document.getElementById(id)?.textContent,values.at(-1));
    assert.equal(p.document.querySelectorAll('.value-suggestions [role=option]').length,16);key(p,input,'Enter');assert.equal(input.value,values.at(-1));
    type(p,input,'Rock','insertFromPaste');const popup=p.document.querySelector('.value-suggestions');popup.scrollTop=1000*30;popup.dispatchEvent(new p.window.Event('scroll'));assert.ok(options(p).includes(values[1000]));
  }finally{p.close();}
});
test('blur, rerender, disabled-state changes, removal, scroll and resize clean up popup state',async()=>{
  for(const cause of ['blur','rerender','disabled','removed','scroll','resize']){const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]');input.focus();type(p,input,'Ro');
    if(cause==='blur')input.blur();if(cause==='rerender')p.document.body.innerHTML=p.panel.fieldsHTML();if(cause==='disabled')input.disabled=true;if(cause==='removed')input.remove();
    if(cause==='scroll')p.document.dispatchEvent(new p.window.Event('scroll'));if(cause==='resize')p.window.dispatchEvent(new p.window.Event('resize'));
    await p.window.happyDOM.whenAsyncComplete();assert.deepEqual(options(p),[],cause);assert.equal(input.hasAttribute('aria-controls'),false,cause);
  }finally{p.close();}}
});
function cellPage({enabled=true,busy=false,column='TITLE',values=['Rock','Rockabilly','Pop'],initial='',custom=['Roots','Rock'],productionCommit=false,originalValues}={}){
  const p=page(),commits=[],pending=[],state={enabled,busy,reads:0},path='/track.flac';p.document.body.innerHTML='<div class="table-wrap"><table><tbody><tr><td data-col="title"></td></tr></tbody></table></div>';
  const wrap=p.document.querySelector('.table-wrap'),append=wrap.append.bind(wrap);wrap.append=(...nodes)=>{state.beforeAppendMode=nodes[0].getAttribute('aria-autocomplete');return append(...nodes);};
  if(originalValues)initial=originalValues.join(' '+String.fromCharCode(92,92)+' ');
  const entry={filename:'track.flac',format:'flac',fields:{TITLE:originalValues??(initial?[initial]:[])}},candidateEntry={filename:'candidate.flac',format:'flac',fields:{TITLE:values}},entries=new Map([[path,entry],['/candidate.flac',candidateEntry]]);
  const candidates=evaluate(`${statement('cellCompletions')};return cellCompletions;`,{entries,merged:entry=>entry.fields,panelKey:(_entry,key)=>key,tagPanel:{settings:{field_values:{lists:{TITLE:custom}}}},collator:new Intl.Collator()});
  const env={CellEditor,commitCell:(target,value)=>{commits.push([target,value]);return path;},revealRow:()=>p.document.querySelector('tr'),CSS:p.window.CSS,entries,editKey:column=>column.field,columnLayout:{columns:[{id:'title',name:'Title',field:column}]},columnName:()=> 'Title',field:()=>initial,anchor:undefined,paths:()=>[],selected:new Set(),generalPrefs:()=>({autocomplete:state.enabled,auto_select_next:false}),busy:state.busy,cellCompletions:key=>{state.reads++;return candidates(key);},completeInput,visible:()=>[entry],tableColumns:()=>[{id:"title",field:column}],adjacentCell:()=>undefined};
  if(productionCommit){const commit=evaluate(`let status='';${statement('commitCell')};return commitCell;`,{...env,merged:item=>item.fields,panelKey:(_item,key)=>key,updateDraft:(...args)=>pending.push(args),drafts:new Map(),sourcePolicy:()=>({mutationReason:''}),render:()=>{},renameFromList:()=>{throw new Error('unexpected rename');}});env.commitCell=(target,value,exactValues)=>{commits.push([target,value]);return commit(target,value,exactValues);};}
  const editor=evaluate(`const ${variable('cellEditor')};return cellEditor;`,env);editor.start({path,column:'title'});
  return{...p,editor,commits,pending,state,input:p.document.querySelector('.cell-editor')};
}
test('production File List host uses custom then loaded candidates and separates choosing from committing',async()=>{
  const p=cellPage();try{type(p,p.input,'Ro');assert.equal(p.input.value,'Roots');assert.deepEqual(options(p),['Roots','Rock','Rockabilly']);
    key(p,p.input,'ArrowDown');key(p,p.input,'ArrowDown');assert.equal(key(p,p.input,'Enter').defaultPrevented,true);assert.equal(p.input.value,'Rock');assert.deepEqual(p.commits,[]);assert.equal(p.editor.active,true);
    key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.commits,[[{path:'/track.flac',column:'title'},'Rock']]);assert.equal(p.editor.active,false);
  }finally{p.editor.cancel();p.close();}
});
test('production File List host excludes filename, technical, option-off and busy controls',()=>{
  for(const config of [{column:'_FILENAME'},{column:'_FILENAME_EXT'},{column:'_BITRATE'},{enabled:false},{busy:true}]){const p=cellPage(config);try{type(p,p.input,'Ro');assert.equal(p.input.value,'Ro');assert.equal(p.input.getAttribute('aria-autocomplete'),'none');assert.deepEqual(options(p),[]);assert.deepEqual(p.commits,[]);}finally{p.editor.cancel();p.close();}}
});
test('cell Tab commits current inline value without accepting highlighted alternative; first Escape only closes',async()=>{
  const p=cellPage();try{type(p,p.input,'Ro');key(p,p.input,'ArrowDown');key(p,p.input,'ArrowDown');key(p,p.input,'Tab');await Promise.resolve();assert.equal(p.commits[0][1],'Roots');assert.deepEqual(options(p),[]);
    p.editor.start({path:'/track.flac',column:'title'});p.input=p.document.querySelector('.cell-editor');type(p,p.input,'Ro');key(p,p.input,'Escape');assert.equal(p.editor.active,true);key(p,p.input,'Escape');assert.equal(p.editor.active,false);
  }finally{p.editor.cancel();p.close();}
});
test('pointer choice beats cell blur, and reattach/cancel dispose stale lists',async()=>{
  const p=cellPage();try{type(p,p.input,'Ro');const option=p.document.querySelectorAll('.value-suggestions [role=option]')[2];const event=new p.window.PointerEvent('pointerdown',{bubbles:true,cancelable:true});option.dispatchEvent(event);assert.equal(event.defaultPrevented,true);option.click();assert.deepEqual(p.commits,[]);p.input.blur();await Promise.resolve();assert.equal(p.commits[0][1],'Rockabilly');
    p.editor.start({path:'/track.flac',column:'title'});p.input=p.document.querySelector('.cell-editor');type(p,p.input,'Ro');p.input.setSelectionRange(1,3);p.editor.reattach();assert.deepEqual(options(p),[]);assert.deepEqual([p.input.selectionStart,p.input.selectionEnd],[1,3]);type(p,p.input,'Ro');p.editor.cancel();assert.deepEqual(options(p),[]);
  }finally{p.editor.cancel();p.close();}
});
function extendedPage({enabled=true}={}){
  const p=page(),stages=[],saves=[],state={enabled,reads:0},values=['Rock\nfirst line','Rock\nsecond line'];
  const extended=new ExtendedTags({selection:()=>['/track.flac'],listed:()=>['/track.flac'],fields:()=>({COMMENT:['Original']}),describe:()=>({name:'track.flac'}),stage:changes=>stages.push(changes),select:()=>{},completions:()=>{state.reads++;return values;},autocomplete:()=>state.enabled?completeInput:undefined,coverMarkup:()=>'',bindCover:()=>{},modal:(_title,content,buttons)=>{const dialog=p.document.createElement('dialog');dialog.innerHTML=`<header></header><div class="dialog-content">${content}</div><div>${buttons}</div>`;p.document.body.append(dialog);dialog.showModal();return dialog;},modalError:(_dialog,error)=>{throw error;},done:()=>{},saveMode:()=> 'pending',save:async path=>saves.push(path)});extended.show();return{...p,extended,state,stages,saves,values,input:p.document.querySelector('textarea')};
}
test('actual Extended Tags textarea suggests whole values inside its dialog without saving or staging',()=>{
  const p=extendedPage();try{p.input.focus();type(p,p.input,'Rock\ns');assert.equal(p.input.value,'Rock\ns');assert.equal(p.document.querySelector('.value-suggestions').parentElement,p.document.querySelector('dialog'));
    assert.equal(key(p,p.input,'Enter').defaultPrevented,false);assert.deepEqual(p.stages,[]);assert.deepEqual(p.saves,[]);
    key(p,p.input,'ArrowDown');key(p,p.input,'Enter');assert.equal(p.input.value,p.values[1]);assert.deepEqual(p.stages,[]);assert.deepEqual(p.saves,[]);assert.equal(key(p,p.input,'Enter').defaultPrevented,false);
    p.document.querySelector('#apply-extended').click();assert.deepEqual(p.stages,[[{path:'/track.flac',updates:{COMMENT:[p.values[1]]}}]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
test('Extended Tags option-off/composition, field rename, value/row removal and modal close clean up',async()=>{
  for(const cause of ['off','composition','rename','value','row','modal']){const p=extendedPage();try{p.input.focus();
    if(cause==='off')p.state.enabled=false;
    if(cause==='composition'){p.input.dispatchEvent(new p.window.CompositionEvent('compositionstart'));const before=p.state.reads;type(p,p.input,'Rock');assert.equal(p.state.reads,before);p.input.dispatchEvent(new p.window.CompositionEvent('compositionend'));}
    type(p,p.input,'Rock');
    if(cause==='off'){assert.deepEqual(options(p),[]);assert.equal(p.state.reads,0);continue;}
    if(cause==='rename')type(p,p.document.querySelector('.extended-name input'),'TITLE');
    if(cause==='value')p.document.querySelector('.extended-value button').click();if(cause==='row')p.document.querySelector('.remove-field').click();if(cause==='modal')p.document.querySelector('dialog').close();
    if(cause==='composition'){key(p,p.input,'Escape');}
    await p.window.happyDOM.whenAsyncComplete();assert.deepEqual(options(p),[],cause);assert.deepEqual(p.stages,[]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}}
});

test('cell composition lifecycle blocks even unflagged editor keys and preserves late non-composing completion',async()=>{
  const p=cellPage();try{type(p,p.input,'Ro');p.input.dispatchEvent(new p.window.CompositionEvent('compositionstart'));p.input.value='R';p.input.setSelectionRange(0,1);
    for(const value of ['Enter','Tab','Escape','ArrowDown'])key(p,p.input,value);await Promise.resolve();assert.equal(p.editor.active,true);assert.deepEqual(p.commits,[]);assert.deepEqual(options(p),[]);assert.deepEqual([p.input.value,p.input.selectionStart,p.input.selectionEnd],['R',0,1]);
    p.input.dispatchEvent(new p.window.CompositionEvent('compositionend'));type(p,p.input,'Ro');assert.equal(p.input.value,'Roots');assert.deepEqual(options(p),['Roots','Rock','Rockabilly']);
  }finally{p.editor.cancel();p.close();}
});
test('scrolling away from an active virtual option never leaves a dangling accessible descendant',()=>{
  const p=panelPage(Array.from({length:100},(_,i)=>`Rock ${String(i).padStart(3,'0')}`));try{const input=p.document.querySelector('[data-field="TITLE"]');type(p,input,'Rock');key(p,input,'ArrowDown');assert.ok(p.document.getElementById(input.getAttribute('aria-activedescendant')));
    const popup=p.document.querySelector('.value-suggestions');popup.scrollTop=1500;popup.dispatchEvent(new p.window.Event('scroll'));assert.equal(input.hasAttribute('aria-activedescendant'),false);assert.equal(key(p,input,'Enter').defaultPrevented,false);assert.deepEqual(p.drafts,[]);
  }finally{p.close();}
});


test('automatic panel matches retain visible and semantic selection/custom/genre groups',()=>{
  const p=panelPage(['Rock','Rockabilly']);try{p.panel.settings.field_values.lists.GENRE=['Roots','Rock'];const input=p.document.querySelector('[data-field="GENRE"]');type(p,input,'Ro');
    const groups=[...p.document.querySelectorAll('.value-suggestions [role=group]')];
    assert.deepEqual(groups.map(group=>group.getAttribute('aria-label')),['Selected files','Custom values','Genres']);
    assert.deepEqual(groups.map(group=>group.querySelector('.combo-group')?.textContent),['Selected files','Custom values','Genres']);
    assert.deepEqual([...groups[0].querySelectorAll('[role=option]')].map(option=>option.textContent),['Rock','Rockabilly']);
    assert.deepEqual([...groups[1].querySelectorAll('[role=option]')].map(option=>option.textContent),['Roots']);
    assert.ok([...groups[2].querySelectorAll('[role=option]')].every(option=>option.textContent.startsWith('Ro')));
    assert.equal(p.document.querySelectorAll('.value-suggestions [role=option]').length,options(p).length);
  }finally{p.close();}
});
test('actual File List choice and later commit preserve whole LF, CRLF and CR values losslessly',async()=>{
  for(const value of ['Rock\nlive','Rock\r\nlive','Rock\rlive']){const p=cellPage({values:[value]});try{
    type(p,p.input,'Ro','insertFromPaste');assert.ok(options(p).includes(value));
    key(p,p.input,'ArrowUp');key(p,p.input,'Enter');p.input=p.document.querySelector('.cell-editor');
    assert.equal(p.input.value,value.replace(/\r\n?/g,'\n'));assert.equal(p.input.tagName,'TEXTAREA');assert.deepEqual(p.commits,[]);assert.equal(p.editor.active,true);
    p.editor.reattach();assert.equal(p.input.value,value.replace(/\r\n?/g,'\n'));
    key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.commits,[[{path:'/track.flac',column:'title'},value]]);
  }finally{p.editor.cancel();p.close();}}
});
test('actual responsive suggestion row geometry drives virtual scrolling and active descendants',async()=>{
  const css=await readFile(new URL('../src/style.css',import.meta.url),'utf8');
  for(const [width,row] of [[1200,30],[800,44]]){const values=Array.from({length:200},(_,i)=>`Rock ${String(i).padStart(3,'0')}`),p=panelPage(values);try{
    p.window.happyDOM.setWindowSize({width,height:800});const style=p.document.createElement('style');style.textContent=css;p.document.head.append(style);
    const input=p.document.querySelector('[data-field="TITLE"]');type(p,input,'Ro','insertFromPaste');const popup=p.document.querySelector('.value-suggestions');
    assert.equal(p.window.getComputedStyle(popup.querySelector('[role=option]')).height,`${row}px`);
    popup.scrollTop=100*row;popup.dispatchEvent(new p.window.Event('scroll'));assert.ok(options(p).includes(values[100]));assert.ok(options(p).length<=16);
    key(p,input,'ArrowUp');assert.equal(p.document.getElementById(input.getAttribute('aria-activedescendant'))?.textContent,values.at(-1));
    assert.ok(popup.scrollTop>190*row-300);key(p,input,'Enter');assert.equal(input.value,values.at(-1));
  }finally{p.close();}}
});

test('actual File List untouched original metadata preserves raw CRLF/CR through Enter and Tab',async()=>{
  for(const initial of ['Rock\r\nlive','Rock\rlive'])for(const finishKey of ['Enter','Tab']){const p=cellPage({initial});try{
    assert.equal(p.input.value,initial.replace(/\r\n?/g,'\n'));p.editor.reattach();key(p,p.input,finishKey);await Promise.resolve();assert.equal(p.commits[0][1],initial);
  }finally{p.editor.cancel();p.close();}}
});
test('actual Extended Tags choice preserves raw CRLF/CR when later kept pending, then real edits replace it',()=>{
  for(const value of ['Rock\r\nlive','Rock\rlive'])for(const edit of [false,true]){const p=extendedPage();try{
    p.values.splice(0,p.values.length,value);type(p,p.input,'Ro','insertFromPaste');key(p,p.input,'ArrowDown');key(p,p.input,'Enter');
    assert.equal(p.input.value,value.replace(/\r\n?/g,'\n'));assert.deepEqual(p.stages,[]);assert.deepEqual(p.saves,[]);assert.equal(key(p,p.input,'Enter').defaultPrevented,false);
    if(edit)type(p,p.input,'New\ntext','insertFromPaste');p.document.querySelector('#apply-extended').click();
    assert.deepEqual(p.stages,[[{path:'/track.flac',updates:{COMMENT:[edit?'New\ntext':value]}}]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}}
});

test('group headers remain non-selectable across virtual navigation between selection, custom and genre origins',()=>{
  const selection=Array.from({length:40},(_,i)=>`Rock selection ${String(i).padStart(2,'0')}`),custom=Array.from({length:40},(_,i)=>`Rock custom ${String(i).padStart(2,'0')}`),p=panelPage(selection);try{
    p.panel.settings.field_values.lists.GENRE=custom;const input=p.document.querySelector('[data-field="GENRE"]');type(p,input,'Ro','insertFromPaste');
    for(let step=0;step<41;step++)key(p,input,'ArrowDown');let active=p.document.getElementById(input.getAttribute('aria-activedescendant'));
    assert.equal(active.textContent,custom[0]);assert.equal(active.closest('[role=group]').getAttribute('aria-label'),'Custom values');assert.ok(options(p).length<=16);
    key(p,input,'Escape');type(p,input,'Ro','insertFromPaste');key(p,input,'ArrowUp');active=p.document.getElementById(input.getAttribute('aria-activedescendant'));
    assert.equal(active.closest('[role=group]').getAttribute('aria-label'),'Genres');assert.ok(options(p).length<=16);key(p,input,'Enter');assert.deepEqual(p.drafts,[['GENRE',active.textContent]]);
  }finally{p.close();}
});
test('actual File List explicit whole-value choice preserves raw line endings through Tab and edits replace its snapshot',async()=>{
  for(const edit of [false,true]){const value='Rock\r\nlive',p=cellPage({values:[value]});try{
    type(p,p.input,'Ro','insertFromPaste');key(p,p.input,'ArrowUp');key(p,p.input,'Enter');assert.deepEqual(p.commits,[]);
    if(edit)type(p,p.input,'New\ntext','insertFromPaste');key(p,p.input,'Tab');await Promise.resolve();assert.equal(p.commits[0][1],edit?'New\ntext':value);
  }finally{p.editor.cancel();p.close();}}
});

test('typed prefixes never inline-insert LF/CR candidates in panel inputs or actual metadata cells',async()=>{
  for(const value of ['Rock\nlive','Rock\r\nlive','Rock\rlive']){
    const p=panelPage([value]);try{const input=p.document.querySelector('[data-field="TITLE"]');type(p,input,'Ro');assert.equal(input.value,'Ro');assert.deepEqual([input.selectionStart,input.selectionEnd],[2,2]);assert.deepEqual(options(p),[value]);assert.deepEqual(p.drafts,[]);
      key(p,input,'ArrowDown');key(p,input,'Enter');assert.deepEqual(p.drafts,[['TITLE',value]]);
    }finally{p.close();}
    const c=cellPage({values:[value],custom:[]});try{type(c,c.input,'Ro');assert.equal(c.input.value,'Ro');assert.deepEqual([c.input.selectionStart,c.input.selectionEnd],[2,2]);assert.deepEqual(options(c),[value]);assert.deepEqual(c.commits,[]);
      key(c,c.input,'ArrowDown');key(c,c.input,'Enter');assert.equal(c.input.value,value.replace(/\r\n?/g,'\n'));assert.deepEqual(c.commits,[]);key(c,c.input,'Enter');await Promise.resolve();assert.equal(c.commits[0][1],value);
    }finally{c.editor.cancel();c.close();}
  }
});
test('actual metadata cell multiline text is never silently completed while editing within that whole value',()=>{
  const p=cellPage({values:['Rock\nlive'],custom:[],initial:'Rock\nli'});try{const event=new p.window.InputEvent('input',{bubbles:true,inputType:'insertText',data:'i'});p.input.setSelectionRange(p.input.value.length,p.input.value.length);p.input.dispatchEvent(event);assert.equal(p.input.value,'Rock\nli');assert.ok(options(p).includes('Rock\nlive'));assert.deepEqual([p.input.selectionStart,p.input.selectionEnd],[7,7]);assert.deepEqual(p.commits,[]);
  }finally{p.editor.cancel();p.close();}
});

test('actual cell host and production commitCell stage exact whole line endings only after later Enter',async()=>{
  for(const value of ['Rock\nlive','Rock\r\nlive','Rock\rlive']){const p=cellPage({values:[value],custom:[],productionCommit:true});try{
    type(p,p.input,'Ro');assert.equal(p.input.value,'Ro');assert.deepEqual(options(p),[value]);key(p,p.input,'ArrowDown');key(p,p.input,'Enter');assert.deepEqual(p.commits,[]);assert.deepEqual(p.pending,[]);
    key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.commits,[[{path:'/track.flac',column:'title'},value]]);assert.deepEqual(p.pending,[['/track.flac','TITLE',[value]]]);
  }finally{p.editor.cancel();p.close();}}
});


test('actual panel advertises no automatic completion when General is off at construction',()=>{
  const p=panelPage(undefined,false);try{for(const control of p.document.querySelectorAll('[data-field]'))assert.equal(control.getAttribute('aria-autocomplete'),'none');assert.equal(p.state.reads,p.state.beforeBindReads);assert.deepEqual(options(p),[]);}finally{p.close();}
});
test('actual cell advertises no automatic completion when General is off at construction',()=>{
  const p=cellPage({enabled:false});try{assert.equal(p.state.beforeAppendMode,'none');assert.equal(p.input.getAttribute('aria-autocomplete'),'none');assert.equal(p.state.reads,0);assert.deepEqual(p.commits,[]);}finally{p.editor.cancel();p.close();}
});
test('actual Extended Tags advertises no automatic completion when General is off at construction',()=>{
  const p=extendedPage({enabled:false});try{assert.equal(p.input.getAttribute('aria-autocomplete'),'none');assert.equal(p.state.reads,0);assert.deepEqual(p.stages,[]);assert.deepEqual(p.saves,[]);}finally{p.close();}
});
test('new actual metadata cell advertises its enabled completion before append without reading candidates',()=>{
  const p=cellPage();try{assert.equal(p.state.beforeAppendMode,'both');assert.equal(p.input.getAttribute('aria-autocomplete'),'both');assert.equal(p.state.reads,0);}finally{p.editor.cancel();p.close();}
});
test('actual panel focus, key and host rerender refresh on/off ARIA status without candidate reads',()=>{
  const p=panelPage(undefined,false);try{const input=p.document.querySelector('[data-field="TITLE"]'),comment=p.document.querySelector('[data-field="COMMENT"]'),reads=p.state.reads;input.value='Free';input.setSelectionRange(1,3);
    p.state.enabled=true;input.focus();assert.equal(input.getAttribute('aria-autocomplete'),'both');comment.focus();assert.equal(comment.getAttribute('aria-autocomplete'),'list');
    p.state.enabled=false;key(p,comment,'Shift');assert.equal(comment.getAttribute('aria-autocomplete'),'none');input.focus();assert.equal(input.getAttribute('aria-autocomplete'),'none');
    assert.equal(input.value,'Free');assert.deepEqual([input.selectionStart,input.selectionEnd],[1,3]);assert.equal(p.state.reads,reads);assert.deepEqual(p.drafts,[]);
    p.document.body.innerHTML=p.panel.fieldsHTML();p.panel.bind(p.document.body);assert.ok([...p.document.querySelectorAll('[data-field]')].every(control=>control.getAttribute('aria-autocomplete')==='none'));
  }finally{p.close();}
});
test('actual cell reattach refreshes host on/off status while retaining text, selection and draft boundary',()=>{
  const p=cellPage();try{p.input.value='Free';p.input.setSelectionRange(1,3);p.state.enabled=false;p.editor.reattach();assert.equal(p.input.getAttribute('aria-autocomplete'),'none');
    p.state.enabled=true;p.editor.reattach();assert.equal(p.input.getAttribute('aria-autocomplete'),'both');assert.equal(p.input.value,'Free');assert.deepEqual([p.input.selectionStart,p.input.selectionEnd],[1,3]);assert.equal(p.state.reads,0);assert.deepEqual(p.commits,[]);
  }finally{p.editor.cancel();p.close();}
});
test('actual Extended Tags focus/key and field eligibility refresh ARIA on/off without reading candidates',()=>{
  const p=extendedPage({enabled:false});try{p.input.value='Free';p.input.setSelectionRange(1,3);p.state.enabled=true;p.input.focus();assert.equal(p.input.getAttribute('aria-autocomplete'),'list');
    p.state.enabled=false;key(p,p.input,'Shift');assert.equal(p.input.getAttribute('aria-autocomplete'),'none');p.state.enabled=true;p.input.blur();p.input.focus();assert.equal(p.input.getAttribute('aria-autocomplete'),'list');
    const name=p.document.querySelector('.extended-name input');type(p,name,'_TECHNICAL');assert.equal(p.input.getAttribute('aria-autocomplete'),'none');type(p,name,'COMMENT');assert.equal(p.input.getAttribute('aria-autocomplete'),'list');
    assert.equal(p.input.value,'Free');assert.deepEqual([p.input.selectionStart,p.input.selectionEnd],[1,3]);assert.equal(p.state.reads,0);assert.deepEqual(p.stages,[]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
test('disabled and readonly attributes refresh ARIA even with no popup and during composition',async()=>{
  const p=panelPage();try{const input=p.document.querySelector('[data-field="TITLE"]'),reads=p.state.reads;input.value='Free';input.setSelectionRange(1,3);input.dispatchEvent(new p.window.CompositionEvent('compositionstart'));
    for(const attribute of ['disabled','readonly']){input.setAttribute(attribute,'');await p.window.happyDOM.whenAsyncComplete();assert.equal(input.getAttribute('aria-autocomplete'),'none');input.removeAttribute(attribute);await p.window.happyDOM.whenAsyncComplete();assert.equal(input.getAttribute('aria-autocomplete'),'both');}
    p.state.enabled=false;input.dispatchEvent(new p.window.InputEvent('input',{bubbles:true,inputType:'insertCompositionText',isComposing:true}));assert.equal(input.getAttribute('aria-autocomplete'),'none');
    assert.equal(p.state.reads,reads);assert.equal(input.value,'Free');assert.deepEqual([input.selectionStart,input.selectionEnd],[1,3]);assert.deepEqual(p.drafts,[]);assert.deepEqual(options(p),[]);
  }finally{p.close();}
});


function productionPanelPage(records,field='COMMENT',pendingRecords){
  const p=page(),pending=[],saves=[],state={reads:0},drafts=new Map((pendingRecords??[]).map((values,index)=>[`/track-${index}.flac`,{[field]:values}])),entries=new Map(records.map((values,index)=>[`/track-${index}.flac`,{path:`/track-${index}.flac`,filename:`track-${index}.flac`,format:'flac',fields:{[field]:values}}])),selected=new Set(entries.keys());
  const env={TagPanel,desktop:false,translateNow:()=>{},translate:value=>value,paths:()=>[...selected],entries,selected,busy:false,copyOverrides:new Map(),drafts,panelKey,
    updateDraft:(path,key,values)=>{pending.push([path,key,values]);drafts.set(path,{[key]:values});},sourcePolicy:()=>({mutationReason:''}),enabled:()=>true,operationReason:()=>'',updateVirtualRows:()=>{},countText:()=>'',visible:()=>[],render:()=>{},generalPrefs:()=>({autocomplete:true}),completeInput,compactScreen:()=>true,providerEntry:()=>false,dirname:()=>'/Music',invoke:(...args)=>{saves.push(args);throw new Error('unexpected native call');}};
  const model=evaluate(`let status='';const ${variable('merged')};const ${variable('field')};${statement('refreshDraftCount')};${statement('draftField')};${statement('keepField')};const ${variable('tagPanel')};${statement('bindFields')};return{panel:tagPanel,bindFields};`,env);
  const sourceValues=model.panel.host.values;model.panel.host.values=key=>{state.reads++;return sourceValues(key);};
  p.document.body.innerHTML=`<div class="fields">${model.panel.fieldsHTML()}</div><div class="status-message"></div><div class="filterbar"><span class="count"></span></div>`;
  model.panel.bind(p.document.querySelector('.fields'));model.bindFields(p.document.querySelector('.fields'));
  const result={...p,...model,pending,saves,entries,selected,drafts,state,input:p.document.querySelector(`[data-field="${field}"]`)};
  result.rerender=()=>{const fields=p.document.querySelector('.fields');fields.innerHTML=model.panel.fieldsHTML();model.panel.bind(fields);model.bindFields(fields);result.input=fields.querySelector(`[data-field="${field}"]`);};
  return result;
}
test('actual panel explicit whole choices preserve endpoint whitespace and line endings through draftField to updateDraft',()=>{
  for(const value of ['Rock\nlive\n','Rock\r\nlive\r\n','Rock\rlive\r','\nRock live\n','\r\nRock live\r\n','\rRock live\r',' Rock live ']){const p=productionPanelPage([[value]]);try{
    type(p,p.input,value.replace(/\r\n?/g,'\n').slice(0,1),'insertFromPaste');assert.ok(options(p).includes(value));p.pending.length=0;key(p,p.input,'ArrowDown');key(p,p.input,'Enter');
    assert.deepEqual(p.pending,[['/track-0.flac','COMMENT',[value]]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}}
});
test('actual cell explicit choices preserve endpoint whitespace and line endings through commitCell to updateDraft',async()=>{
  for(const value of ['Rock\nlive\n','Rock\r\nlive\r\n','Rock\rlive\r','\nRock live\n','\r\nRock live\r\n','\rRock live\r',' Rock live ']){const p=cellPage({values:[value],custom:[],productionCommit:true});try{
    type(p,p.input,value.replace(/\r\n?/g,'\n').slice(0,1),'insertFromPaste');assert.ok(options(p).includes(value));key(p,p.input,'ArrowDown');key(p,p.input,'Enter');assert.deepEqual(p.pending,[]);key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.pending,[['/track.flac','TITLE',[value]]]);
  }finally{p.editor.cancel();p.close();}}
});
test('actual panel keeps genuine arrays distinct from a scalar containing the display delimiter',()=>{
  const scalar='Rock '+String.fromCharCode(92,92)+' live',records=[['Rock','live'],[scalar]];
  for(const [label,expected] of [['“Rock” · “live”',['Rock','live']],[scalar,[scalar]]]){const p=productionPanelPage(records,'ARTIST');try{
    type(p,p.input,'Rock','insertFromPaste');assert.deepEqual(options(p),['“Rock” · “live”',scalar]);const option=[...p.document.querySelectorAll('.value-suggestions [role=option]')].find(item=>item.textContent===label);assert.ok(option);p.pending.length=0;option.click();
    assert.deepEqual(p.pending,[['/track-0.flac','ARTIST',expected],['/track-1.flac','ARTIST',expected]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}}
});
test('actual cell scalar delimiter choice remains one raw value while typed delimiter parsing keeps its existing policy',async()=>{
  const scalar=' Rock '+String.fromCharCode(92,92)+' live ',p=cellPage({values:[scalar],custom:[],productionCommit:true});try{
    type(p,p.input,' ','insertFromPaste');key(p,p.input,'ArrowDown');key(p,p.input,'Enter');assert.deepEqual(p.pending,[]);key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.pending,[['/track.flac','TITLE',[scalar]]]);
  }finally{p.editor.cancel();p.close();}
  const c=cellPage({custom:[],productionCommit:true});try{type(c,c.input,scalar,'insertFromPaste');key(c,c.input,'Enter');await Promise.resolve();assert.deepEqual(c.pending,[['/track.flac','TITLE',['Rock','live']]]);}finally{c.editor.cancel();c.close();}
  const t=productionPanelPage([['Other']],'ARTIST');try{type(t,t.input,scalar,'insertFromPaste');assert.deepEqual(t.pending,[['/track-0.flac','ARTIST',['Rock','live']]]);}finally{t.close();}
});

test('actual cell explicit scalar delimiter can replace a genuine array with the same display text',async()=>{
  const scalar='Rock '+String.fromCharCode(92,92)+' live',p=cellPage({values:[scalar],custom:[],productionCommit:true,originalValues:['Rock','live']});try{
    type(p,p.input,'Rock','insertFromPaste');const option=[...p.document.querySelectorAll('.value-suggestions [role=option]')].find(item=>item.textContent===scalar);assert.ok(option);option.click();assert.deepEqual(p.pending,[]);key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.pending,[['/track.flac','TITLE',[scalar]]]);
  }finally{p.editor.cancel();p.close();}
});

test('actual panel treats array/scalar display collisions as mixed and keeps both selectable origins',()=>{
  const scalar='Rock '+String.fromCharCode(92,92)+' live',p=productionPanelPage([['Rock','live'],[scalar]],'ARTIST');try{
    assert.equal(p.input.value,'');assert.equal(p.input.placeholder,'< keep >');type(p,p.input,'Rock','insertFromPaste');assert.deepEqual(options(p),['“Rock” · “live”',scalar]);
  }finally{p.close();}
});

test('actual manual panel choices preserve genuine arrays, scalar delimiters and endpoint whitespace',()=>{
  const scalar=' Rock '+String.fromCharCode(92,92)+' live ',records=[['Rock','live'],[scalar]];
  for(const [label,expected] of [['“Rock” · “live”',['Rock','live']],[scalar,[scalar]]]){const p=productionPanelPage(records,'ARTIST');try{
    p.input.parentElement.querySelector('.combo-toggle').click();const option=[...p.document.querySelectorAll('.combo-list [role=option]')].find(item=>item.textContent===label);assert.ok(option);option.click();
    assert.deepEqual(p.pending,[['/track-0.flac','ARTIST',expected],['/track-1.flac','ARTIST',expected]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}}
});
test('real editing after an explicit scalar choice restores ordinary cell and panel parsing',async()=>{
  const raw='Rock\r\nlive\r\n',typed=' Rock '+String.fromCharCode(92,92)+' live ',p=cellPage({values:[raw],custom:[],productionCommit:true});try{
    type(p,p.input,'Rock','insertFromPaste');key(p,p.input,'ArrowDown');key(p,p.input,'Enter');assert.deepEqual(p.pending,[]);type(p,p.input,typed,'insertFromPaste');key(p,p.input,'Enter');await Promise.resolve();assert.deepEqual(p.pending,[['/track.flac','TITLE',['Rock','live']]]);
  }finally{p.editor.cancel();p.close();}
  const q=productionPanelPage([[raw]],'COMMENT');try{type(q,q.input,'Rock','insertFromPaste');q.pending.length=0;key(q,q.input,'ArrowDown');key(q,q.input,'Enter');assert.deepEqual(q.pending,[['/track-0.flac','COMMENT',[raw]]]);q.pending.length=0;type(q,q.input,typed,'insertFromPaste');assert.deepEqual(q.pending,[['/track-0.flac','COMMENT',['Rock','live']]]);}finally{q.close();}
});

test('actual panel keeps literal Keep/Blank strings as raw choices distinct from manual commands',()=>{
  const p=productionPanelPage([['< keep >'],['< blank >']]);try{type(p,p.input,'<','insertFromPaste');assert.deepEqual(options(p),['< blank >','< keep >']);p.pending.length=0;
    [...p.document.querySelectorAll('.value-suggestions [role=option]')].find(option=>option.textContent==='< keep >').click();assert.deepEqual(p.pending,[['/track-0.flac','COMMENT',['< keep >']],['/track-1.flac','COMMENT',['< keep >']]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});

test('actual sequential scalar typing retains all selection matches after its own inline draft publication',()=>{
  const p=productionPanelPage([['Rock'],['Rockabilly']],'TITLE');try{
    type(p,p.input,'R');assert.equal(p.input.value,'Rock');assert.deepEqual(options(p),['Rock','Rockabilly']);type(p,p.input,'Ro');assert.equal(p.input.value,'Rock');assert.deepEqual([p.input.selectionStart,p.input.selectionEnd],[2,4]);assert.deepEqual(options(p),['Rock','Rockabilly']);
    type(p,p.input,'Roc');assert.deepEqual(options(p),['Rock','Rockabilly']);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
test('actual sequential typing retains genuine-array and scalar-delimiter origins and stages the explicit raw choice',()=>{
  const scalar='Rock '+String.fromCharCode(92,92)+' live',records=[['Rock','live'],[scalar]];
  for(const [label,expected] of [['“Rock” · “live”',['Rock','live']],[scalar,[scalar]]]){const p=productionPanelPage(records,'ARTIST');try{
    for(const prefix of ['R','Ro','Roc']){type(p,p.input,prefix);assert.equal(p.input.value,prefix);assert.deepEqual(options(p),['“Rock” · “live”',scalar]);}
    p.pending.length=0;[...p.document.querySelectorAll('.value-suggestions [role=option]')].find(option=>option.textContent===label).click();assert.deepEqual(p.pending,[['/track-0.flac','ARTIST',expected],['/track-1.flac','ARTIST',expected]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}}
});
test('actual sequential typing retains pre-existing pending array origins without substituting loaded values',()=>{
  const scalar='Royal '+String.fromCharCode(92,92)+' live',p=productionPanelPage([['Loaded A'],['Loaded B']],'ARTIST',[['Royal','live'],[scalar]]);try{
    type(p,p.input,'R');type(p,p.input,'Ro');assert.deepEqual(options(p),['“Royal” · “live”',scalar]);p.pending.length=0;key(p,p.input,'ArrowDown');key(p,p.input,'Enter');assert.deepEqual(p.pending,[['/track-0.flac','ARTIST',['Royal','live']],['/track-1.flac','ARTIST',['Royal','live']]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
test('actual composition publication keeps origins without candidate reads and late committed input refreshes matches',()=>{
  const p=productionPanelPage([['Rock'],['Rockabilly']],'TITLE');try{const reads=p.state.reads;p.input.dispatchEvent(new p.window.CompositionEvent('compositionstart'));type(p,p.input,'R','insertCompositionText');assert.equal(p.state.reads,reads);assert.equal(p.input.value,'R');assert.deepEqual(options(p),[]);p.input.dispatchEvent(new p.window.CompositionEvent('compositionend'));assert.equal(p.state.reads,reads);
    type(p,p.input,'Ro');assert.deepEqual(options(p),['Rock','Rockabilly']);assert.equal(p.input.value,'Rock');assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
test('actual editing origins honor fresh custom settings and legitimate external pending-value changes',()=>{
  const p=productionPanelPage([['Rock'],['Rockabilly']],'TITLE');try{
    type(p,p.input,'R');p.panel.settings.field_values.lists.TITLE=['Roman'];type(p,p.input,'Ro');assert.deepEqual(options(p),['Rock','Rockabilly','Roman']);
    p.panel.settings.field_values.lists.TITLE=['Rosy'];type(p,p.input,'Ro');assert.deepEqual(options(p),['Rock','Rockabilly','Rosy']);
    p.drafts.set('/track-0.flac',{TITLE:['Rose']});p.drafts.set('/track-1.flac',{TITLE:['Roswell']});type(p,p.input,'Ro');assert.deepEqual(options(p),['Rose','Roswell','Rosy']);
    p.panel.settings.field_values.prefer_tag_values=true;type(p,p.input,'Ro');assert.deepEqual(options(p),['Rose','Roswell']);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
test('actual selection and reload rerenders reset origins and do not restore discarded draft candidates',()=>{
  const p=productionPanelPage([['Rock'],['Rockabilly']],'TITLE');try{type(p,p.input,'R');const old=p.input;p.entries.set('/other.flac',{path:'/other.flac',filename:'other.flac',format:'flac',fields:{TITLE:['Royal']}});p.selected.clear();p.selected.add('/other.flac');p.rerender();assert.equal(old.isConnected,false);assert.equal(old.getAttribute('aria-expanded'),'false');type(p,p.input,'Ro');assert.deepEqual(options(p),['Royal']);
    p.drafts.clear();p.entries.get('/other.flac').fields.TITLE=['Roman'];p.rerender();assert.equal(p.input.value,'Roman');type(p,p.input,'Ro');assert.deepEqual(options(p),['Roman']);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});

test('actual manual full dropdown retains the same origins after sequential automatic editing',()=>{
  const scalar='Rock '+String.fromCharCode(92,92)+' live',p=productionPanelPage([['Rock','live'],[scalar]],'ARTIST');try{
    type(p,p.input,'R');type(p,p.input,'Ro');key(p,p.input,'Escape');p.input.parentElement.querySelector('.combo-toggle').click();
    const options=[...p.document.querySelectorAll('.combo-list [role=option]')];assert.deepEqual(options.map(option=>option.textContent),['< keep >','< blank >','“Rock” · “live”',scalar]);p.pending.length=0;options[2].click();assert.deepEqual(p.pending,[['/track-0.flac','ARTIST',['Rock','live']],['/track-1.flac','ARTIST',['Rock','live']]]);assert.deepEqual(p.saves,[]);
  }finally{p.close();}
});
