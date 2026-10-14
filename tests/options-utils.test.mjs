import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>{
  const source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const menu=await load('row-context-menu'),options=await load('options-ui'),messages=await load('messages'),tools=await load('tools'),utils=await load('utils-ui');

test('page defaults fill unsaved values and survive partial saves',()=>{
  options.registerOptionsPage({id:'defaults-test',label:'Defaults',defaults:{a:true,b:'x'},controls:[{kind:'checkbox',path:'pages.defaults-test.a',label:'A'}]});
  assert.deepEqual(options.pageValues({pages:{}},'defaults-test'),{a:true,b:'x'});
  assert.deepEqual(options.pageValues({pages:{'defaults-test':{a:false}}},'defaults-test'),{a:false,b:'x'});
  assert.deepEqual(options.pageValues({pages:{}},'unknown'),{});
  const filled=options.withPageDefaults({...options.defaultSettings(),pages:{'defaults-test':{b:'y'}}});
  assert.deepEqual(filled.pages['defaults-test'],{a:true,b:'y'});
});
test('button and custom controls render with stable hooks',()=>{
  const draft={...options.defaultSettings(),appearance:'system'};
  assert.match(options.controlMarkup({kind:'button',id:'open-x',label:'Open <x>…',run:()=>{}},draft),/<button type="button" id="option-button-open-x" data-option-button="open-x">Open &lt;x&gt;…<\/button>/);
  assert.equal(options.controlMarkup({kind:'custom',id:'c',paths:[],render:d=>`<b>${d.appearance}</b>`,bind:()=>{}},draft),'<div class="options-custom" data-option-custom="c"><b>system</b></div>');
});

test('messages fall back to documented defaults and gate confirmations',async()=>{
  assert.equal(messages.messageEnabled({},'confirm_remove_tags'),true);
  assert.equal(messages.messageEnabled({},'after_save'),false);
  assert.equal(messages.messageEnabled({confirm_remove_tags:false},'confirm_remove_tags'),false);
  assert.equal(messages.messageEnabled({confirm_remove_tags:'no'},'confirm_remove_tags'),true);
  assert.deepEqual(messages.messagesOptionsPage.defaults,messages.MESSAGE_DEFAULTS);
  const paths=JSON.stringify(messages.messagesOptionsPage.controls).match(/pages\.messages\.\w+/g).map(p=>p.slice(15)).sort();
  assert.deepEqual(paths,Object.keys(messages.MESSAGE_DEFAULTS).sort(),'every message has a checkbox and a default');
  let values={},asked=[],told=[];
  const ui=new messages.Messages({values:()=>values,ask:async(m)=>{asked.push(m);return false;},tell:async m=>{told.push(m);}});
  assert.equal(await ui.confirm('confirm_remove_tags','Remove?','Remove tags'),false);
  values={confirm_remove_tags:false};
  assert.equal(await ui.confirm('confirm_remove_tags','Remove?','Remove tags'),true);
  await ui.notify('after_save','Saved','OpenTag');values={after_save:true};await ui.notify('after_save','Saved 2','OpenTag');
  assert.deepEqual([asked,told],[['Remove?'],['Saved 2']]);
});

test('tools are normalized, validated and mapped to Ctrl/⌘ digits',()=>{
  assert.deepEqual(tools.normalizeTools([{name:'A',path:'/bin/a',for_each_file:'yes'},null,3]),[{name:'A',path:'/bin/a',parameters:'',for_each_file:false}]);
  assert.deepEqual(tools.normalizeTools('x'),[]);
  const tool=(o={})=>({name:'Echo',path:'/bin/echo',parameters:'"%_path%"',for_each_file:true,...o});
  assert.equal(tools.toolsError([tool(),tool({path:'C:\\Tools\\x.exe'}),tool({path:'\\\\server\\x.exe'})]),undefined);
  assert.match(tools.toolsError([tool({name:' '})]),/Tool 1 needs a name/);
  assert.match(tools.toolsError([tool({path:''})]),/program path/);
  assert.match(tools.toolsError([tool({path:'bin/echo'})]),/full path/);
  assert.match(tools.toolsError([tool({parameters:'"%_path%'})]),/unmatched quote/);
  assert.match(tools.toolsError(Array.from({length:100},()=>tool())),/at most 99/);
  assert.equal(tools.defaultToolIndex({default_tool:1},[tool(),tool()]),1);
  assert.equal(tools.defaultToolIndex({default_tool:2},[tool(),tool()]),-1);
  assert.equal(tools.defaultToolIndex({},[tool()]),-1);
  const key=(k,o={})=>({key:k,code:`Digit${k}`,ctrlKey:false,metaKey:false,altKey:false,shiftKey:false,...o});
  assert.equal(tools.toolShortcutIndex(key('1',{ctrlKey:true}),false),0);
  assert.equal(tools.toolShortcutIndex(key('0',{ctrlKey:true}),false),9);
  assert.equal(tools.toolShortcutIndex(key('1',{metaKey:true}),true),0);
  assert.equal(tools.toolShortcutIndex(key('1',{ctrlKey:true}),true),-1);
  assert.equal(tools.toolShortcutIndex(key('1',{ctrlKey:true,shiftKey:true}),false),-1);
  assert.equal(tools.toolShortcutIndex(key('1',{altKey:true}),false),-1,'Alt+digit stays with the converters');
  assert.equal(tools.toolShortcutIndex({...key('a',{ctrlKey:true}),code:'KeyA'},false),-1);
  assert.deepEqual([0,8,9,10].map(i=>tools.toolShortcut(i,false)),['Ctrl 1','Ctrl 9','Ctrl 0','']);
});
test('tool runs report, confirm large batches and skip unusable states',async()=>{
  const calls=[],reports=[];let values={tools:[{name:'Each',path:'/bin/echo',parameters:'',for_each_file:true},{name:'Once',path:'/bin/echo',parameters:'',for_each_file:false}]},reason='',confirmed=false;
  const files=Array.from({length:11},(_,i)=>`/m/${i}.mp3`);
  const ui=new tools.ToolsUI({values:()=>values,mac:false,run:async(tool,paths)=>{calls.push([tool.name,paths.length]);return{started:tool.for_each_file?paths.length:1};},selection:()=>files,reason:()=>reason,confirm:async()=>confirmed,report:m=>reports.push(m),fail:e=>reports.push('fail '+e)});
  await ui.run(0);assert.deepEqual(calls,[],'declined confirmation');
  confirmed=true;await ui.run(0);await ui.run(1);
  assert.deepEqual(calls,[['Each',11],['Once',11]]);
  assert.deepEqual(reports,['Started Each 11 times','Started Once']);
  reason='Select files first.';await ui.run(1);assert.equal(reports.at(-1),'Select files first.');
  assert.match(ui.menuHTML(),/data-tool="1" disabled/);
  values={tools:[]};assert.match(ui.menuHTML(),/No tools defined/);
});

test('remove tag types counts present kinds from _TAG',()=>{
  assert.deepEqual([...utils.presentTagKinds('ID3v2.3 APE ID3v1')].sort(),['ape','id3v1','id3v2']);
  assert.deepEqual([...utils.presentTagKinds('VorbisComment')],[]);
  assert.deepEqual([...utils.presentTagKinds(undefined)],[]);
  assert.deepEqual(utils.tagKindCounts(['ID3v2.4 ID3v1','APE','ID3v2.3',undefined]),{id3v1:1,id3v2:2,ape:1});
  const chaptered=[{format:'MP4',technical:{_SUBSONG_COUNT:['3']}},{format:'MP4',technical:{}},{format:'Matroska',technical:{_SUBSONG_COUNT:['2']}},{format:'MP4',technical:{_SUBSONG_COUNT:['1']}},{}];
  assert.deepEqual(utils.mp4ChapterFiles(chaptered),{files:2,chapters:4});
  assert.match(utils.stripChaptersPrompt({files:1,chapters:1}),/^Permanently remove 1 chapter from 1 MP4 file\?/);
});
test('copy as text keeps a short format history',()=>{
  assert.deepEqual(utils.rememberFormat(undefined,'%title%'),['%title%']);
  assert.deepEqual(utils.rememberFormat(['a','%title%','b',3],'%title%'),['%title%','a','b']);
  assert.equal(utils.rememberFormat(Array.from({length:20},(_,i)=>String(i)),'x').length,utils.COPY_HISTORY_LIMIT);
});
test('Ctrl+Y redoes when it can and otherwise repeats the last action groups',()=>{
  const redo={label:'Save tags',files:2};
  assert.deepEqual(utils.redoOrRepeat({redo},[{name:'Clean'}]),{kind:'redo',label:'Redo Save tags (2 files)'});
  assert.deepEqual(utils.redoOrRepeat({redo:null},[{name:'Clean'}]),{kind:'repeat',label:'Repeat Clean'});
  assert.deepEqual(utils.redoOrRepeat({redo:null},[{name:'A'},{name:'B'}]),{kind:'repeat',label:'Repeat 2 action groups'});
  assert.deepEqual(utils.redoOrRepeat({redo:null},undefined),{kind:'none',label:'Redo'});
});
test('keyboard shortcut rows are read from the rendered menus',()=>{
  const node=(text,children=[],small)=>({textContent:text,children,querySelector:s=>s==='small'?small?{textContent:small}:null:s.startsWith('span')?{textContent:text}:s==='summary'?{textContent:text}:null});
  const button=(label,keys)=>node(label,[],keys);
  const menu=(name,buttons)=>({querySelector:s=>s==='summary'?{textContent:name}:null,querySelectorAll:()=>buttons});
  const bar={querySelectorAll:()=>[menu('File',[button('Save tags','⌘/Ctrl S'),button('Reload tags')]),menu('Edit',[button('Undo','⌘/Ctrl Z')])]};
  assert.deepEqual(utils.menuShortcuts(bar),[{group:'File',command:'Save tags',keys:'⌘/Ctrl S'},{group:'Edit',command:'Undo',keys:'⌘/Ctrl Z'}]);
  assert.match(utils.configurationFileName(new Date('2026-10-14T12:00:00Z')),/^opentag-configuration-2026-10-14\.json$/);
});
test('row context menu markup escapes labels and keeps disabled reasons',()=>{
  const markup=menu.contextMenuMarkup([{heading:'Tools'},{label:'A <b>',id:'tool:0',shortcut:'Ctrl 1'},{separator:true},{label:'Remove tag types…',id:'op:remove-tag-types',disabled:true,title:'Select files first.'}]);
  assert.match(markup,/^<div class="context-heading" role="presentation">Tools<\/div>/);
  assert.match(markup,/data-context="tool:0"  title="A &lt;b&gt;"><span>A &lt;b&gt;<\/span><small>Ctrl 1<\/small>/);
  assert.match(markup,/<hr><button type="button" role="menuitem" data-context="op:remove-tag-types" disabled title="Select files first.">/);
});
