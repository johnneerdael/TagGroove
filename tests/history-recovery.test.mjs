import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
import { Window } from 'happy-dom';
const compile = source => ts.transpileModule(source, { compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022} }).outputText;
const source = await readFile(new URL('../src/main.ts', import.meta.url), 'utf8');
const parsed = ts.createSourceFile('main.ts', source, ts.ScriptTarget.Latest, true);
function production(name, host) {
  const node = parsed.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === name);
  assert.ok(node, `Missing production ${name}`);
  return new Function('host', `with(host){${compile(node.getText(parsed)).replace('export {};', '')};return ${name};}`)(host);
}
const retained = {undo:null, redo:null, notice:null, recovery:{operations:1, copies:2, bytes:200}};
const platform = {family:'desktop', folder_picker_supported:true};
test('recovery notice remains visible with no Undo step and no transient notice', () => {
  const window = new Window();
  const host = {desktop:true, runtimePlatform:platform, history:retained};
  host.button = production('button', {enabled:()=>true, operationReason:()=>'', esc:String, glyphs:{}});
  const markup = production('historyRecoveryMarkup', host);
  try {
    window.document.body.innerHTML = markup();
    const notice = window.document.querySelector('[aria-label="History recovery"]');
    assert.ok(notice);
    assert.match(notice.textContent, /interrupted/i);
    assert.ok(notice.querySelector('button[data-op="history-recovery"]'));
    host.history = {undo:null, redo:null, notice:null};
    assert.equal(markup(), '');
    host.history = retained;
    host.runtimePlatform = {...platform, family:'ios'};
    assert.equal(markup(), '');
  } finally { window.happyDOM.abort(); }
});
test('recovery export can run without selected files and while tag drafts are pending', () => {
  const host = {desktop:true, runtimePlatform:platform, history:retained, busy:false, coverOperations:{}, listOperations:[], operationReason:()=>'', selected:new Set()};
  const enabled = production('enabled', host);
  assert.equal(enabled('history-recovery'), true);
  host.busy = true;
  assert.equal(enabled('history-recovery'), false);
  host.busy = false;
  host.history = {undo:null, redo:null, notice:null};
  assert.equal(enabled('history-recovery'), false);
});
test('export asks for a folder and invokes only the dedicated recovery command', async () => {
  const calls = [], history = structuredClone(retained);
  const host = {desktop:true, runtimePlatform:platform, history, status:'Ready', open:async options => {calls.push(['picker',options]);return '/chosen';}, run:async (_label,work) => work(), invoke:async (command,args) => {calls.push([command,args]);return '/chosen/TagGroove recovery example';}, t:(text,args={}) => text.replace('{path}',args.path??'')};
  await production('exportHistoryRecovery', host)();
  assert.equal(calls[0][0], 'picker');
  assert.equal(calls[0][1].directory, true);
  assert.deepEqual(calls.slice(1), [['export_history_recovery',{destination:'/chosen'}]]);
  assert.match(host.status, /\/chosen\/TagGroove recovery example/);
  assert.deepEqual(host.history, history);
});
test('cancelling recovery export leaves state and files untouched', async () => {
  const host = {desktop:true, runtimePlatform:platform, history:structuredClone(retained), status:'Ready', open:async()=>null, run:()=>{throw new Error('Cancelled export started work');}, invoke:()=>{throw new Error('Cancelled export called IPC');}};
  const before = structuredClone(host.history);
  await production('exportHistoryRecovery', host)();
  assert.equal(host.status, 'Ready');
  assert.deepEqual(host.history, before);
});

test('recovery export is independent of pending tag drafts and selected source permissions', () => {
  const host = {desktop:true, runtimePlatform:platform, coverOperations:{}, sourcePolicy:()=>({mutationReason:'Read-only source'}), chapterRowReason:()=>'', selected:new Set(), entries:new Map(), dirtySelected:()=>true, listOperations:[], utilOperations:{}, tagSourceOperations:[]};
  const reason = production('operationReason', host);
  assert.equal(reason('history-recovery'), '');
});

function startupPlatform(host) {
  let declaration;
  function visit(node) {
    if(ts.isVariableDeclaration(node) && node.name.getText(parsed)==='platformReady') declaration=node;
    ts.forEachChild(node,visit);
  }
  visit(parsed);assert.ok(declaration,'Actual desktop platform startup declaration');
  return new Function('host', `with(host){${compile('const value = '+declaration.initializer.getText(parsed)+';').replace('export {};','')};return value;}`)(host);
}
for(const restored of [true,false])test(`desktop startup loads retained history with ${restored?'an automatically restored':'an empty'} workspace`, async () => {
  const retained = {undo:{label:'Save tags',files:1}, redo:{label:'Rename',files:1}, notice:'Interrupted operation recovery is available', recovery:{operations:1,copies:2,bytes:200}};
  const host = {
    desktop:true, history:{undo:null,redo:null,notice:null}, status:'Ready',
    runtimePlatform:{}, nativeAppearanceReady:false, errors:[], directoriesReady:undefined,
    ordinaryWorkspaceReady:false, optionsReady:Promise.resolve(), profileWorkspaceReady:Promise.resolve(false),
    invoke:async command => {
      if(command==='runtime_platform')return {family:'desktop',folder_picker_supported:restored,document_access_supported:false};
      if(command==='history_state')return structuredClone(retained);
      throw new Error('Unexpected startup IPC '+command);
    },
    languagePage:{setFolderSupport(){}}, render(){}, syncNativeAppearance(){},
    coordinateConfigurationLaunches:job=>job(),
    fileOps:{initialize:async options=>{await options.restoreWorkspace();}},
    restoreOrdinaryWorkspace:async()=>restored, rememberOrdinaryWorkspace(){},
    ColumnImportStatus:class {},
    busy:false, coverOperations:{},listOperations:[],selected:new Set(),
    operationReason:()=>'',
  };
  host.refreshHistory=production('refreshHistory',host);
  await startupPlatform(host);await host.directoriesReady;
  assert.deepEqual(host.history,retained,'Retained history was never loaded on ordinary startup');
  const enabled=production('enabled',host);
  assert.equal(enabled('undo'),true);
  host.button=production('button',{enabled:()=>true,operationReason:()=>'',esc:String,glyphs:{}});
  const window=new Window();
  try {
    window.document.body.innerHTML=production('historyRecoveryMarkup',host)();
    assert.ok(window.document.querySelector('[aria-label="History recovery"] button'));
  } finally { window.happyDOM.abort(); }
});
