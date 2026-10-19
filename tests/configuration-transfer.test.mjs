import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';
const source=await readFile(new URL('../src/configuration-transfer.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {ConfigurationTransferUI,SavedConfigurationWrites}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const profileSource=await readFile(new URL('../src/profile-migration.ts',import.meta.url),'utf8');
const profileJs=ts.transpileModule(profileSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {ProfileMigrationUI,bootstrapProfileMigration}=await import(`data:text/javascript;base64,${Buffer.from(profileJs).toString('base64')}`);
const workspace=()=>({version:2,paths:['/Music/song.wav'],selected:['/Music/song.wav'],roots:['/Music'],working_directory:'/Music',list_order:'manual',source_order:['/Music/song.wav'],location:'/Music',location_is_data:true,filter:'',filter_draft:null,anchor:'/Music/song.wav',playlist:null});
const captured=()=>({version:2,preferences:{'opentag.tag-tag':'{"field":"TITLE","template":"%title%"}'},workspace:workspace()});
const status=()=>({desktop:true,phase:'none',report:null,restorable:false,pending:null,preference_keys:['opentag.tag-tag'],portable_preference_keys:['opentag.tag-tag']});
const preview=()=>({token:'bound-plan',version:2,source_hash:'source-hash',files:['settings.json','actions.json'],export_assets:['Practice.mte'],preference_keys:['opentag.tag-tag'],preferences_available:true,source_workspace:{rows:1,roots:['/Music'],path_style:'posix'},workspace:workspace(),workspace_available:true,unavailable:[],warnings:[],replaces_snapshot:true});
const settle=async()=>{for(let i=0;i<5;i++)await new Promise(resolve=>setTimeout(resolve,0));};
function page(){
  const window=new Window(),globals={};for(const name of ['window','document']){globals[name]=Object.getOwnPropertyDescriptor(globalThis,name);Object.defineProperty(globalThis,name,{configurable:true,writable:true,value:name==='window'?window:window.document});}
  const calls=[],errors=[],reports=[],live={draft:'unsaved',rows:['/Current/keep.wav']};let current=status(),nextPreview=preview(),flushes=0;
  const host={invoke:async(command,args)=>{calls.push([command,structuredClone(args)]);if(command==='profile_migration_status')return structuredClone(current);if(command==='configuration_transfer_preview')return structuredClone(nextPreview);if(command==='configuration_transfer_export')return{version:2,files:7,export_assets:1,preferences:1,workspace_rows:1,omitted:['Unsaved audio edits']};if(command==='configuration_transfer_apply'){current={...status(),phase:'applied',pending:{id:'new-pending',attempted:false,window:captured()}};return structuredClone(current);}throw new Error(command);},pickSave:async()=>'/Backups/saved.json',pickLoad:async()=>'/Backups/saved.json',capture:()=>captured(),flush:async()=>{flushes++;calls.push(['flush']);},pause:()=>calls.push(['pause']),resume:()=>calls.push(['resume']),hold:value=>calls.push(['hold',value]),prepare:async()=>{calls.push(['prepare']);return true;},inspectWorkspace:async()=>[],reload:()=>calls.push(['reload']),report:text=>reports.push(text),fail:error=>errors.push(String(error))};
  host.preferenceKeys=()=>status().preference_keys;host.recoveryPending=()=>!!current.pending;
  return{window,host,calls,errors,reports,live,get current(){return current;},set current(value){current=value;},set preview(value){nextPreview=value;},get flushes(){return flushes;},ui:()=>new ConfigurationTransferUI(host),close(){window.happyDOM.abort();for(const[name,descriptor]of Object.entries(globals))if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];}};
}
test('Save flushes saved state, exports an explicit window and reports exclusions without applying or discarding edits',async()=>{
  const p=page();try{await p.ui().save();const call=p.calls.find(([name])=>name==='configuration_transfer_export');assert.deepEqual(call[1],{outputPath:'/Backups/saved.json',window:captured()});assert.ok(p.calls.findIndex(([name])=>name==='flush')<p.calls.indexOf(call));assert.deepEqual(p.live,{draft:'unsaved',rows:['/Current/keep.wav']});assert.ok(!p.calls.some(([name])=>['prepare','pause','configuration_transfer_apply','profile_migration_status'].includes(name)));assert.match(p.reports[0],/Unsaved audio edits/);}finally{p.close();}
});
test('Save refuses unresolved recovery and flush errors before publishing an output',async()=>{
  for(const pending of [true,false]){const p=page();try{if(pending)p.current={...status(),pending:{id:'retained',attempted:false,window:captured()}};else p.host.flush=async()=>{throw new Error('saved layout flush failed');};await p.ui().save();assert.ok(!p.calls.some(([name])=>name==='configuration_transfer_export'));assert.equal(p.errors.length,1);assert.deepEqual(p.live.draft,'unsaved');}finally{p.close();}}
});
test('Load preview cancellation is read-only for configuration, preferences, drafts and current list',async()=>{
  const p=page();try{const running=p.ui().load();await settle();assert.ok(p.calls.some(([name])=>name==='configuration_transfer_preview'));p.window.document.querySelector('[data-cancel]').click();await running;assert.ok(!p.calls.some(([name])=>['prepare','pause','configuration_transfer_apply','reload'].includes(name)));assert.deepEqual(p.live,{draft:'unsaved',rows:['/Current/keep.wav']});}finally{p.close();}
});
test('changed components require another bound review and explicit gate before Apply',async()=>{
  const p=page();try{const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog');const box=dialog.querySelector('[data-file="actions.json"]');box.checked=false;box.dispatchEvent(new p.window.Event('change',{bubbles:true}));assert.equal(dialog.querySelector('[data-apply]').disabled,true);p.preview={...preview(),token:'selected-plan'};dialog.querySelector('[data-review]').click();await settle();const reviewed=p.calls.filter(([name])=>name==='configuration_transfer_preview').at(-1);assert.deepEqual(reviewed[1].options,{files:['settings.json'],preferences:true,workspace:true,export_assets:true});dialog.querySelector('[data-apply]').click();await running;assert.deepEqual(p.calls.find(([name])=>name==='configuration_transfer_apply')[1],{token:'selected-plan',window:captured()});assert.ok(p.calls.findIndex(([name])=>name==='prepare')<p.calls.findIndex(([name])=>name==='pause'));assert.ok(p.calls.some(([name])=>name==='reload'));assert.match(p.reports.at(-1),/pending/i);assert.deepEqual(p.live.draft,'unsaved');}finally{p.close();}
});
test('a denied unsaved-edit gate leaves preview and live edits intact without pausing writers',async()=>{
  const p=page();try{p.host.prepare=async()=>false;const running=p.ui().load();await settle();p.window.document.querySelector('[data-apply]').click();await settle();assert.ok(!p.calls.some(([name])=>['pause','configuration_transfer_apply'].includes(name)));assert.equal(p.live.draft,'unsaved');p.window.document.querySelector('[data-cancel]').click();await running;}finally{p.close();}
});
test('successful ordinary flushes remain saved when a later Apply flush refuses',async()=>{
  const p=page();try{let calls=0;p.host.flush=async()=>{p.calls.push(['flush']);if(++calls===2)throw new Error('later history flush failed');};const running=p.ui().load();await settle();p.window.document.querySelector('[data-apply]').click();await settle();assert.match(p.window.document.querySelector('[role="alert"]').textContent,/later history/);assert.equal(calls,2);assert.ok(!p.calls.some(([name])=>['pause','configuration_transfer_apply','reload'].includes(name)));assert.deepEqual(p.live,{draft:'unsaved',rows:['/Current/keep.wav']});p.window.document.querySelector('[data-cancel]').click();await running;}finally{p.close();}
});
test('Apply failure resumes writers only after durable status proves no new pending publication',async()=>{
  const p=page();try{const invoke=p.host.invoke;p.host.invoke=async(command,args)=>{if(command==='configuration_transfer_apply'){p.calls.push([command,args]);throw new Error('bound plan expired');}return invoke(command,args);};const running=p.ui().load();await settle();p.window.document.querySelector('[data-apply]').click();await settle();assert.ok(p.calls.some(([name])=>name==='resume'));assert.ok(!p.calls.some(([name])=>name==='reload'));assert.deepEqual(p.live,{draft:'unsaved',rows:['/Current/keep.wav']});assert.equal(p.window.document.querySelector('[data-apply]').disabled,true,'an expired plan needs another review');p.window.document.querySelector('[data-cancel]').click();await running;}finally{p.close();}
});
test('a lost Apply response with a retained receipt keeps writers gated and requires recovery reload',async()=>{
  const p=page();try{const invoke=p.host.invoke;p.host.invoke=async(command,args)=>{const result=await invoke(command,args);if(command==='configuration_transfer_apply')throw new Error('response lost after publication');return result;};const running=p.ui().load();await settle();p.window.document.querySelector('[data-apply]').click();await settle();assert.ok(p.current.pending);assert.ok(!p.calls.some(([name])=>name==='resume'));assert.equal(p.window.document.querySelector('[data-apply]').disabled,true);assert.match(p.window.document.querySelector('[data-cancel]').textContent,/Reload/);assert.deepEqual(p.live.draft,'unsaved');p.window.document.querySelector('[data-cancel]').click();await running;assert.ok(p.calls.some(([name])=>name==='reload'));}finally{p.close();}
});
test('missing complete workspace is initially unselected and root mapping is explicit in a new review',async()=>{
  const p=page();try{p.preview={...preview(),workspace_available:false,unavailable:['/Music/song.wav']};p.host.inspectWorkspace=async()=>['/Music/song.wav'];const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog');assert.equal(dialog.querySelector('[data-workspace]').checked,false);assert.match(dialog.textContent,/\/Music\/song.wav/);dialog.querySelector('[data-map-from]').value='/Music';dialog.querySelector('[data-map-to]').value='/Moved';dialog.querySelector('[data-map-to]').dispatchEvent(new p.window.Event('input',{bubbles:true}));dialog.querySelector('[data-review]').click();await settle();const call=p.calls.filter(([name])=>name==='configuration_transfer_preview').at(-1);assert.deepEqual(call[1].mappings,[{from:'/Music',to:'/Moved'}]);assert.equal(call[1].options.workspace,false);dialog.querySelector('[data-cancel]').click();await running;}finally{p.close();}
});

test('saved publication tracking drains in-flight work, exposes prior failures and blocks stale writes during recovery',async()=>{
  const writes=new SavedConfigurationWrites();let release,written=0;
  const first=writes.run(()=>new Promise(resolve=>{release=()=>{written++;resolve();};}));
  let drained=false;const flushing=writes.flush().then(()=>{drained=true;});await Promise.resolve();assert.equal(drained,false);release();await first;await flushing;assert.equal(written,1);
  await assert.rejects(writes.run(async()=>{throw new Error('ordinary saved publication refused');}),/publication refused/);
  await assert.rejects(writes.flush(),/publication refused/);await writes.flush();
  writes.pause();await assert.rejects(writes.run(async()=>{written++;}),/paused/);assert.equal(written,1);
  writes.resume();await writes.run(async()=>{written++;});await writes.flush();assert.equal(written,2);
});
test('a present empty preference snapshot can reset saved choices while an absent snapshot preserves them',async()=>{
  for(const present of [true,false]){const p=page();try{
    p.preview={...preview(),preference_keys:[],preferences_available:present,source_workspace:null,workspace:null};
    const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog'),box=dialog.querySelector('[data-preferences]');
    assert.equal(box.disabled,!present);assert.equal(box.checked,present);
    dialog.querySelector('[data-review]').click();await settle();
    assert.equal(p.calls.filter(([name])=>name==='configuration_transfer_preview').at(-1)[1].options.preferences,present);
    dialog.querySelector('[data-cancel]').click();await running;
  }finally{p.close();}}
});

for(const rows of [3,0])test(`unsupported source inventory (${rows} rows) requires bound settings-only review and preserves destination recovery`,async()=>{
  const p=page();try{
    const root='opaque:<root>&"',style='unsupported<style>',source={rows,roots:rows?[root]:[],path_style:style};
    const incoming={...preview(),source_workspace:source,workspace:null,workspace_available:false,unavailable:['The captured workspace path style is unavailable here.']};
    const destination={...captured(),workspace:{...workspace(),paths:['/Destination/keep.wav'],selected:['/Destination/keep.wav'],roots:['/Destination'],working_directory:'/Destination',source_order:['/Destination/keep.wav'],anchor:'/Destination/keep.wav'}};
    p.preview=incoming;p.live.rows=[...destination.workspace.paths];p.host.capture=()=>structuredClone(destination);p.host.inspectWorkspace=async()=>assert.fail('unsupported source inventory must never be scanned');
    const invoke=p.host.invoke;p.host.invoke=async(command,args)=>{
      if(command==='configuration_transfer_preview'&&args.options){assert.equal(args.options.workspace,false);p.calls.push([command,structuredClone(args)]);return{...incoming,token:'settings-only-bound'};}
      if(command==='configuration_transfer_apply'){p.calls.push([command,structuredClone(args)]);assert.equal(args.token,'settings-only-bound');p.current={...status(),phase:'applied',pending:{id:'preserve-current',attempted:false,window:structuredClone(destination)}};return structuredClone(p.current);}
      return invoke(command,args);
    };
    const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog'),choice=dialog.querySelector('[data-workspace]'),apply=dialog.querySelector('[data-apply]');
    assert.equal(choice.checked,false);assert.equal(choice.disabled,true);assert.equal(apply.disabled,true,'the default native token must not authorize visible settings-only choices');
    assert.match(dialog.querySelector('[data-workspace-summary]').textContent,new RegExp(`Captured workspace: ${rows} rows and ${source.roots.length} roots`));
    assert.equal(dialog.querySelector('[data-workspace-path-style]').textContent,style);assert.equal(dialog.querySelector('[data-workspace-path-style]').getAttribute('translate'),'no');
    assert.deepEqual([...dialog.querySelectorAll('[data-workspace-roots] li')].map(item=>item.textContent),source.roots);assert.equal(dialog.querySelector('[data-workspace-roots]').getAttribute('translate'),'no');assert.equal(dialog.querySelector('root'),null);assert.equal(dialog.querySelector('style'),null);
    apply.click();await settle();assert.ok(!p.calls.some(([name])=>name==='configuration_transfer_apply'));
    dialog.querySelector('[data-review]').click();await settle();assert.equal(apply.disabled,false);apply.click();await running;
    assert.deepEqual(p.calls.find(([name])=>name==='configuration_transfer_apply')[1],{token:'settings-only-bound',window:destination});assert.deepEqual(p.live.rows,['/Destination/keep.wav'],'live list remains until successful reload');assert.equal(p.live.draft,'unsaved');
    const recoveryHost={...p.host,storage:()=>p.window.localStorage,workspace:()=>destination.workspace,restoreWorkspace:async value=>{assert.deepEqual(value,destination.workspace);p.live.rows=[...value.paths];return true;}};
    recoveryHost.invoke=async(command,args)=>{assert.equal(command,'profile_migration_status');if(args?.start)p.current.pending.attempted=true;if(args?.ack)p.current.pending=null;return structuredClone(p.current);};
    const initial=await bootstrapProfileMigration(recoveryHost.invoke,recoveryHost.storage);assert.equal(await new ProfileMigrationUI(recoveryHost,initial).finishStartupRecovery(),true);assert.deepEqual(p.live.rows,['/Destination/keep.wav']);assert.equal(p.current.pending,null);
  }finally{p.close();}
});

test('absent workspace differs from supported empty workspace and both keep valid default Apply immediately available',async()=>{
  for(const supportedEmpty of [true,false]){const p=page();try{
    p.preview={...preview(),source_workspace:supportedEmpty?{rows:0,roots:[],path_style:'posix'}:null,workspace:supportedEmpty?{...workspace(),paths:[],selected:[],roots:[],source_order:[],anchor:null}:null,workspace_available:supportedEmpty};
    const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog');assert.equal(dialog.querySelector('[data-apply]').disabled,false);assert.equal(dialog.querySelector('[data-workspace]').checked,supportedEmpty);assert.equal(dialog.querySelector('[data-workspace]').disabled,!supportedEmpty);
    if(!supportedEmpty)assert.match(dialog.textContent,/No workspace is included/);
    dialog.querySelector('[data-cancel]').click();await running;
  }finally{p.close();}}
});
test('a changed backup inventory requires reopening rather than applying unseen replacements',async()=>{
  const p=page();try{const running=p.ui().load();await settle();p.preview={...preview(),source_hash:'different-source',export_assets:['Unexpected.mte']};const dialog=p.window.document.querySelector('dialog');dialog.querySelector('[data-review]').click();await settle();assert.equal(dialog.querySelector('[data-apply]').disabled,true);assert.match(dialog.querySelector('[role="alert"]').textContent,/changed/);assert.ok(!p.calls.some(([name])=>name==='configuration_transfer_apply'));dialog.querySelector('[data-cancel]').click();await running;}finally{p.close();}
});
test('an Apply response without the required recovery state fails closed rather than admitting stale saved-state writers',async()=>{
  const p=page();try{const invoke=p.host.invoke;p.host.invoke=async(command,args)=>command==='configuration_transfer_apply'?status():invoke(command,args);const running=p.ui().load();await settle();p.window.document.querySelector('[data-apply]').click();await settle();assert.ok(!p.calls.some(([name])=>name==='resume'));assert.match(p.window.document.querySelector('[data-cancel]').textContent,/Reload/);p.window.document.querySelector('[data-cancel]').click();await running;}finally{p.close();}
});

for(const boundary of ['preview','workspace'])for(const edit of ['checkbox','mapping'])test(`Review rejects ${edit} changes while awaiting ${boundary} and locks mutable controls`,async()=>{
  const p=page();try{
    let release,requests=0,inspections=0;
    const invoke=p.host.invoke;p.host.invoke=async(command,args)=>{if(command==='configuration_transfer_preview'&&++requests===2)return boundary==='preview'?new Promise(resolve=>{release=()=>resolve({...preview(),token:'stale-plan'});}):{...preview(),token:'stale-plan'};return invoke(command,args);};
    p.host.inspectWorkspace=async()=>boundary==='workspace'&&++inspections===2?new Promise(resolve=>{release=()=>resolve([]);}):[];
    const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog'),box=dialog.querySelector('[data-file="actions.json"]'),from=dialog.querySelector('[data-map-from]'),to=dialog.querySelector('[data-map-to]');
    if(edit==='checkbox'){box.checked=false;box.dispatchEvent(new p.window.Event('change',{bubbles:true}));}else{from.value='/Music';to.value='/First';to.dispatchEvent(new p.window.Event('input',{bubbles:true}));}
    dialog.querySelector('[data-review]').click();await settle();const locked=box.disabled&&from.disabled&&to.disabled&&dialog.querySelector('[data-add-mapping]').disabled;
    if(edit==='checkbox'){box.checked=true;box.dispatchEvent(new p.window.Event('change',{bubbles:true}));}else{to.value='/Second';to.dispatchEvent(new p.window.Event('input',{bubbles:true}));}
    release();await settle();assert.equal(dialog.querySelector('[data-apply]').disabled,true,'stale completion cannot authorize different visible choices');assert.equal(locked,true);assert.equal(box.disabled,false);assert.equal(from.disabled,false);
    dialog.querySelector('[data-cancel]').click();await running;assert.ok(!p.calls.some(([name])=>name==='configuration_transfer_apply'));
  }finally{p.close();}
});

for(const boundary of ['prepare','flush'])for(const edit of ['checkbox','mapping'])test(`Apply refuses ${edit} changes while awaiting ${boundary} and requires a fresh review`,async()=>{
  const p=page();try{
    let release,flushes=0;
    if(boundary==='prepare')p.host.prepare=()=>new Promise(resolve=>{release=()=>resolve(true);});
    else p.host.flush=async()=>{if(++flushes===2)await new Promise(resolve=>{release=resolve;});};
    const running=p.ui().load();await settle();const dialog=p.window.document.querySelector('dialog'),box=dialog.querySelector('[data-file="actions.json"]'),from=dialog.querySelector('[data-map-from]'),to=dialog.querySelector('[data-map-to]');
    dialog.querySelector('[data-apply]').click();await settle();const locked=box.disabled&&from.disabled&&to.disabled;
    if(edit==='checkbox'){box.checked=false;box.dispatchEvent(new p.window.Event('change',{bubbles:true}));}else{from.value='/Music';to.value='/Changed';to.dispatchEvent(new p.window.Event('input',{bubbles:true}));}
    release();await settle();assert.ok(!p.calls.some(([name])=>name==='configuration_transfer_apply'),'no obsolete token reaches Apply');assert.equal(locked,true);assert.equal(dialog.querySelector('[data-apply]').disabled,true);assert.ok(!p.calls.some(([name])=>name==='pause'));assert.ok(!p.calls.some(([name])=>name==='reload'));
    dialog.querySelector('[data-cancel]').click();await running;
  }finally{p.close();}
});
