import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>{
  const source=await readFile(new URL(`../src/${name}.ts`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const general=await load('general-options'),options=await load('options-ui'),messages=await load('messages'),utils=await load('utils-ui');

test('auto-completion keeps the typed text and completes from the first match',()=>{
  const values=['Rock','Rockabilly','Pop'];
  assert.equal(general.completion('ro',values),'rock');
  assert.equal(general.completion('Rocka',values),'Rockabilly');
  assert.equal(general.completion('Rock',values),'Rockabilly','an exact match is not a completion');
  assert.equal(general.completion('Jazz',values),undefined);
  assert.equal(general.completion('',values),undefined);
  const input={value:'Po',selectionStart:2,range:undefined,setSelectionRange(a,b){this.range=[a,b];this.selectionStart=a;}};
  assert.equal(general.completeInput(input,{inputType:'insertText'},()=>values),true);
  assert.deepEqual([input.value,input.range],['Pop',[2,3]]);
  const deleting={value:'Po',selectionStart:2,setSelectionRange(){throw new Error('no');}};
  assert.equal(general.completeInput(deleting,{inputType:'deleteContentBackward'},()=>values),false);
  assert.equal(general.completeInput({...deleting,selectionStart:1},{inputType:'insertText'},()=>values),false,'only at the end of the text');
  assert.equal(general.completeInput(deleting,{inputType:'insertFromPaste'},()=>values),false);
});

test('General and Appearance values fall back to the page defaults',()=>{
  assert.deepEqual(general.generalOptions({}),options.GENERAL_DEFAULTS);
  assert.deepEqual(general.generalOptions({autocomplete:false,keep_moved:'no'}),{...options.GENERAL_DEFAULTS,autocomplete:false});
  assert.deepEqual(general.appearanceOptions({}),{toolbarScale:1,uiScale:1,grid:'off'});
  assert.deepEqual(general.appearanceOptions(options.APPEARANCE_DEFAULTS),{toolbarScale:1,uiScale:1,grid:'off'});
  assert.deepEqual(general.appearanceOptions({toolbar_scale:'150',ui_scale:'125',grid:'lines'}),{toolbarScale:1.5,uiScale:1.25,grid:'lines'});
  assert.deepEqual(general.appearanceOptions({toolbar_scale:'900',ui_scale:'x',grid:'tartan'}),{toolbarScale:1,uiScale:1,grid:'off'});
  const draft={...options.defaultSettings(),appearance:'system'};
  const page=options.pageTree().find(({page})=>page.id==='general').page;
  const filled=options.withPageDefaults(structuredClone(draft));
  const markup=page.controls.map(control=>options.controlMarkup(control,filled)).join('');
  for(const key of Object.keys(options.GENERAL_DEFAULTS))assert.match(markup,new RegExp(`data-option="pages.general.${key}"`));
  assert.match(markup,/data-option="pages.general.autocomplete" checked/);
});

test('number options render and Networking validates timeout and proxy',()=>{
  const draft={...options.defaultSettings(),appearance:'system'};
  draft.pages.networking={timeout:30};
  assert.match(options.controlMarkup({kind:'number',path:'pages.networking.timeout',label:'Timeout',min:5,max:300,suffix:'seconds'},draft),/<input type="number" id="option-pages-networking-timeout" data-option="pages.networking.timeout" data-number value="30" min="5" max="300" step="1"> seconds/);
  const problem=networking=>general.networkProblem({pages:{networking}});
  assert.equal(problem({}),undefined);
  assert.match(problem({timeout:2}),/between 5 and 300/);
  assert.match(problem({timeout:Number.NaN}),/timeout/);
  assert.equal(problem({proxy:'manual',proxy_url:'proxy.local:3128'}),undefined);
  assert.equal(problem({proxy:'manual',proxy_url:'https://10.0.0.1:8443'}),undefined);
  for(const bad of ['','socks5://x:1','http://x:1/path'])assert.match(problem({proxy:'manual',proxy_url:bad}),/host:port/,bad);
  assert.match(problem({proxy:'manual',proxy_url:'http://me:pw@x:1'}),/passwords/);
  assert.equal(problem({proxy:'none',proxy_url:'junk'}),undefined,'the address only matters for a manual proxy');
  const page=general.networkingOptionsPage,address=page.controls.find(c=>c.path==='pages.networking.proxy_url');
  assert.match(options.controlMarkup(address,{...draft,pages:{networking:{proxy:'system'}}}),/disabled/);
  assert.doesNotMatch(options.controlMarkup(address,{...draft,pages:{networking:{proxy:'manual'}}}),/disabled/);
});

test('moved files stay listed only inside the listed folders',()=>{
  assert.equal(general.insideFolders('/music/a/b.mp3',['/music/']),true);
  assert.equal(general.insideFolders('/music2/b.mp3',['/music']),false);
  assert.equal(general.insideFolders('C:\\Music\\b.mp3',['C:\\Music']),true);
  assert.equal(general.insideFolders('/other/b.mp3',['/music','/other']),true);
});

test('update messages and release notes after an update',()=>{
  const release={version:'0.2.0',name:'OpenTag 0.2.0',url:'https://github.com/johnneerdael/opentag/releases/tag/v0.2.0',prerelease:false,published:null};
  assert.match(general.updateMessage({current:'0.1.0',latest:release,newer:true}).text,/OpenTag 0\.2\.0 \(0\.2\.0\) is available\. You have OpenTag 0\.1\.0/);
  assert.equal(general.updateMessage({current:'0.2.0',latest:release,newer:false}).title,'OpenTag is up to date');
  assert.equal(general.updateMessage({current:'0.1.0',latest:null,newer:false}).title,'No releases found');
  const prefs=general.updateOptions({});
  assert.deepEqual(prefs,general.UPDATE_DEFAULTS);
  assert.equal(general.showReleaseNotes({...prefs,last_version:''},'0.2.0'),false,'not on a fresh install');
  assert.equal(general.showReleaseNotes({...prefs,last_version:'0.1.0'},'0.2.0'),true);
  assert.equal(general.showReleaseNotes({...prefs,last_version:'0.2.0'},'0.2.0'),false);
  assert.equal(general.showReleaseNotes({...prefs,release_notes:false,last_version:'0.1.0'},'0.2.0'),false);
  assert.equal(general.releaseNotesUrl('0.2.0'),'https://github.com/johnneerdael/opentag/releases/tag/v0.2.0');
  const page=general.updatesOptionsPage(()=>{});
  assert.deepEqual(page.controls.filter(c=>c.kind==='checkbox').map(c=>c.path),['pages.updates.check_on_startup','pages.updates.include_beta','pages.updates.release_notes']);
});

test('new messages default on and remember the cover choice',()=>{
  assert.equal(messages.messageEnabled({},'action_warnings'),true);
  assert.equal(messages.messageEnabled({},'confirm_import_covers'),true);
  assert.equal(messages.coverChoice({}),'replace');
  assert.equal(messages.coverChoice({import_covers_choice:'preserve'}),'preserve');
  const choice=messages.messagesOptionsPage.controls.flatMap(c=>c.kind==='group'?c.controls:[c]).find(c=>c.path==='pages.messages.import_covers_choice');
  assert.equal(choice.enabledWhen({pages:{messages:{}}}),false,'only when the message is off');
  assert.equal(choice.enabledWhen({pages:{messages:{confirm_import_covers:false}}}),true);
});

test('Utils apply to their formats and say what they skip',()=>{
  assert.equal(utils.utilTargets('optimize',['FLAC','MP3','MP4']),2);
  assert.equal(utils.utilTargets('remove_unsupported_fields',['MP3','FLAC']),1);
  assert.equal(utils.utilTargets('strip_chapters',['MP3']),0);
  const text=utils.utilMessage('optimize',['FLAC','MP3']);
  assert.match(text,/^Optimize for 1 of 2 selected files\?/);
  assert.match(text,/1 file is skipped: this applies to FLAC and MP4 files\./);
  assert.match(text,/Edit → Undo restores/);
  assert.doesNotMatch(utils.utilMessage('strip_chapters',['MP4']),/skipped/);
});
