import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {Window} from 'happy-dom';

const source=await readFile(new URL('../src/profile-migration.ts',import.meta.url),'utf8');
const javascript=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {ProfileMigrationUI,bootstrapProfileMigration,captureProfilePreferences,captureProfileWindow,restoreProfilePreferences}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const workspace=()=>({paths:['/Music/practice.flac'],selected:['/Music/practice.flac'],roots:['/Music'],location:'/Music',location_is_data:true,filter:'Artist HAS Practice',filter_draft:null,anchor:'/Music/practice.flac',playlist:null});
const windowState=()=>({version:1,preferences:{'opentag.language':'nl','opentag.filter-shown':'false','opentag.tagSources.lastBuiltIn':'source-discogs'},workspace:workspace()});
const state=()=>({desktop:true,phase:'none',report:null,restorable:false,pending:null,preference_keys:['opentag.language','opentag.filter-shown','opentag.tagSources.lastBuiltIn']});
const items=[
  {file:'columns.ini',kind:'columns',name:'columns',warnings:[],collision:true,supported:true},
  {file:'data/columns/DJ.ini',kind:'columns',name:'DJ',warnings:[],collision:true,supported:true},
  {file:'data/actions/Practice.mta',kind:'actions',name:'Practice',warnings:['Unresolved fixture action remains disabled'],collision:true,supported:true},
  {file:'Settings.bin',kind:'unsupported',name:'Settings',warnings:['No documented mapping'],collision:false,supported:false},
];
function page(){
  const window=new Window();
  globalThis.window=window;globalThis.document=window.document;
  window.localStorage.setItem('opentag.language','nl');window.localStorage.setItem('opentag.filter-shown','false');window.localStorage.setItem('opentag.tagSources.lastBuiltIn','source-discogs');window.localStorage.setItem('unrelated.credentials','never captured');
  const calls=[],errors=[],host={invoke:async(command,args)=>{calls.push([command,args]);if(command==='profile_migration_status')return state();if(command==='profile_migration_preview')return{token:'preview',source_hash:'fixture-hash',items};if(command==='profile_migration_apply')return state();throw new Error(command);},pick:async folder=>{calls.push(['pick',folder]);return folder?'/Downloaded/profile':'/Downloaded/profile.zip';},prepare:async()=>true,flush:async()=>{calls.push(['flush']);},pause:()=>{calls.push(['pause']);},workspace,restoreWorkspace:async value=>{calls.push(['workspace',value]);return true;},storage:()=>window.localStorage,reload:()=>{calls.push(['reload']);},fail:error=>errors.push(String(error))};
  return{window,host,calls,errors,close(){window.happyDOM.abort();}};
}
const settle=async()=>{for(let i=0;i<4;i++)await new Promise(resolve=>setTimeout(resolve,0));};
test('default bootstrap only reads native recovery status and never captures unrelated browser keys',async()=>{
  const p=page();try{
    assert.deepEqual(await bootstrapProfileMigration(p.host.invoke,p.host.storage),state());
    assert.deepEqual(captureProfilePreferences(p.host.storage(),state().preference_keys),windowState().preferences);
    assert.equal(p.host.storage().getItem('unrelated.credentials'),'never captured');
    await assert.rejects(bootstrapProfileMigration(async()=>{throw new Error('recovery denied');},p.host.storage),/recovery denied/);
    assert.throws(()=>restoreProfilePreferences(p.host.storage(),state().preference_keys,{'unrelated.credentials':'overwrite'}),/unsupported preference/);
  }finally{p.close();}
});
test('real migration dialog cancels without applying and respects the pending-edit gate',async()=>{
  const p=page();try{
    const ui=new ProfileMigrationUI(p.host),running=ui.open();await settle();
    p.window.document.querySelector('[data-cancel]').click();await running;
    assert.ok(!p.calls.some(([name])=>name==='profile_migration_preview'||name==='profile_migration_apply'||name==='pause'));
    p.calls.length=0;p.host.prepare=async()=>false;const pending=new ProfileMigrationUI(p.host).open();await settle();p.window.document.querySelector('[data-folder]').click();await settle();p.window.document.querySelector('[data-item="0"]').click();p.window.document.querySelector('[data-apply]').click();await settle();assert.ok(!p.calls.some(([name])=>name==='profile_migration_apply'||name==='pause'));p.window.document.querySelector('[data-cancel]').click();await pending;
  }finally{p.close();}
});
test('folder and ZIP routes expose unsupported files and pass reviewed selections, collision policy and encoding to native preflight',async()=>{
  for(const folder of [true,false]){
    const p=page();try{
      const ui=new ProfileMigrationUI(p.host),running=ui.open();await settle();
      p.window.document.querySelector('[data-encoding]').value='windows-1252';p.window.document.querySelector(folder?'[data-folder]':'[data-zip]').click();await settle();
      const dialog=p.window.document.querySelector('dialog');
      assert.match(dialog.textContent,/Settings.bin/);assert.match(dialog.textContent,/No documented mapping/);assert.equal(dialog.querySelector('[data-item="3"]').disabled,true);
      dialog.querySelector('[data-item="0"]').click();dialog.querySelector('[data-item="1"]').click();assert.equal(dialog.querySelector('[data-item="0"]').checked,false);
      dialog.querySelector('[data-item="2"]').click();dialog.querySelector('[data-collisions]').value='replace';dialog.querySelector('[data-apply]').click();await running;
      const preview=p.calls.find(([name])=>name==='profile_migration_preview');assert.equal(preview[1].legacyEncoding,'windows-1252');
      const apply=p.calls.find(([name])=>name==='profile_migration_apply');assert.deepEqual(apply[1],{token:'preview',selected:['data/columns/DJ.ini','data/actions/Practice.mta'],replaceNames:true,window:windowState()});
      assert.ok(p.calls.findIndex(([name])=>name==='pause')<p.calls.findIndex(([name])=>name==='profile_migration_apply'));
      assert.deepEqual(p.calls.filter(([name])=>name==='reload'),[['reload']]);assert.equal(p.host.storage().getItem('unrelated.credentials'),'never captured');assert.deepEqual(p.errors,[]);
    }finally{p.close();}
  }
});
test('a rejected mutation keeps recovery visible, pauses stale history writes and requires a reload before continuing',async()=>{
  const p=page();try{
    const original=p.host.invoke;p.host.invoke=async(command,args)=>{if(command==='profile_migration_apply'){p.calls.push([command,args]);throw new Error('controlled publication failure; snapshot retained');}return original(command,args);};
    const running=new ProfileMigrationUI(p.host).open();await settle();p.window.document.querySelector('[data-folder]').click();await settle();p.window.document.querySelector('[data-item="0"]').click();p.window.document.querySelector('[data-apply]').click();await settle();
    const dialog=p.window.document.querySelector('dialog');assert.match(dialog.querySelector('[role="alert"]').textContent,/snapshot retained/);assert.equal(dialog.querySelector('[data-apply]').disabled,true);assert.equal(dialog.querySelector('[data-cancel]').textContent,'Reload setup');assert.ok(!p.calls.some(([name])=>name==='reload'));
    dialog.querySelector('[data-cancel]').click();await running;assert.equal(p.calls.filter(([name])=>name==='reload').length,1);
  }finally{p.close();}
});
test('workspace recovery claims the persisted nonce once, acknowledges only success, and leaves failed attempts for explicit retry',async()=>{
  for(const success of [true,false]){
    const p=page();try{
      const current=state();current.pending={id:'persisted-receipt',attempted:false,window:windowState()};
      p.host.invoke=async(command,args)=>{assert.equal(command,'profile_migration_status');p.calls.push([command,args]);if(args?.start){assert.equal(args.start,current.pending.id);current.pending.attempted=true;}if(args?.failed){assert.equal(args.failed,current.pending.id);current.pending.failed=true;}if(args?.ack){assert.equal(args.ack,current.pending.id);current.pending=null;}return structuredClone(current);};
      p.host.restoreWorkspace=async value=>{p.calls.push(['workspace',value]);return success;};
      const initial=await bootstrapProfileMigration(p.host.invoke,p.host.storage);const ui=new ProfileMigrationUI(p.host,initial);await ui.restoreAfterStartup();await ui.restoreAfterStartup();
      assert.equal(p.calls.filter(([name])=>name==='workspace').length,1);
      assert.equal(p.calls.filter(([name,args])=>name==='profile_migration_status'&&args?.ack).length,success?1:0);
      const restarted=new ProfileMigrationUI(p.host,await bootstrapProfileMigration(p.host.invoke,p.host.storage));await restarted.restoreAfterStartup();assert.equal(p.calls.filter(([name])=>name==='workspace').length,1,'later starts do not replay a failed or consumed workspace');
    }finally{p.close();}
  }
});
test('browser preference failure rolls its own writes back and cannot be reported as complete setup restoration',async()=>{
  const p=page();try{
    const data=new Map([['opentag.language','en'],['opentag.filter-shown','true']]);const storage={getItem:key=>data.get(key)??null,removeItem:key=>data.delete(key),setItem:(key,value)=>{if(key==='opentag.filter-shown'&&value==='false')throw new Error('controlled browser storage quota');data.set(key,value);}};
    assert.throws(()=>restoreProfilePreferences(storage,state().preference_keys,windowState().preferences),/storage quota/);assert.deepEqual([...data],[['opentag.language','en'],['opentag.filter-shown','true']]);
    const current=state();current.restorable=true;current.report={items,imported:['columns.ini'],kept:[],source_hash:'fixture-hash'};
    p.host.storage=()=>storage;p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(command==='profile_migration_restore')current.pending={id:'retained-receipt',attempted:false,window:windowState()};return structuredClone(current);};
    const running=new ProfileMigrationUI(p.host).restore();await settle();assert.match(p.window.document.querySelector('dialog').textContent,/Settings.bin/);p.window.document.querySelector('[data-restore]').click();await settle();
    assert.equal(current.pending.id,'retained-receipt');assert.match(p.window.document.querySelector('[role="alert"]').textContent,/storage quota/);assert.ok(p.calls.some(([name])=>name==='pause'));assert.ok(!p.calls.some(([name])=>name==='reload'));p.window.document.querySelector('[data-close]').click();await running;assert.equal(p.calls.filter(([name])=>name==='reload').length,1);
  }finally{p.close();}
});
test('preference restoration verifies written and removed values before startup can acknowledge recovery',async()=>{
  for(const ignored of ['set','remove']){
    const data=new Map([['opentag.language','en'],['opentag.filter-shown','true']]);
    const before=[...data];
    const storage={getItem:key=>data.get(key)??null,setItem:(key,value)=>{if(!(ignored==='set'&&key==='opentag.language'&&value==='nl'))data.set(key,value);},removeItem:key=>{if(!(ignored==='remove'&&key==='opentag.filter-shown'))data.delete(key);}};
    assert.throws(()=>restoreProfilePreferences(storage,['opentag.language','opentag.filter-shown'],{'opentag.language':'nl'}),/readback/i);
    assert.deepEqual([...data],before,'failed restoration rolls already written preferences back');
    const current=state();current.pending={id:'unverified-browser-values',attempted:false,window:{...windowState(),preferences:{'opentag.language':'nl'}}};
    const calls=[],invoke=async(command,args)=>{calls.push([command,args]);return current;};
    await assert.rejects(bootstrapProfileMigration(invoke,()=>storage),/readback/i);
    assert.deepEqual(calls,[['profile_migration_status',undefined]],'no claim or acknowledgement follows failed browser verification');
    assert.equal(current.pending.id,'unverified-browser-values');
  }
});
test('manual workspace retry gates reload and preference changes on the existing unsaved-edits decision',async()=>{
  const p=page();try{
    const current=state();current.phase='applied';current.pending={id:'retained',attempted:true,window:windowState()};current.pending.window.preferences['opentag.language']='en';
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);return structuredClone(current);};p.host.prepare=async()=>false;
    const running=new ProfileMigrationUI(p.host).restore();await settle();p.window.document.querySelector('[data-retry]').click();await settle();
    assert.ok(!p.calls.some(([name,args])=>name==='profile_migration_status'&&args?.retry));assert.ok(!p.calls.some(([name])=>name==='pause'||name==='reload'));assert.equal(p.host.storage().getItem('opentag.language'),'nl');p.window.document.querySelector('[data-close]').click();await running;
  }finally{p.close();}
});

test('a durable in-progress claim restores preferences and resumes the captured workspace after renderer restart',async()=>{
  const p=page();try{
    const current=state();current.pending={id:'claimed-before-crash',attempted:true,window:windowState()};
    p.host.storage().setItem('opentag.language','en');
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(args?.ack)current.pending=null;return structuredClone(current);};
    const initial=await bootstrapProfileMigration(p.host.invoke,p.host.storage);
    assert.equal(p.host.storage().getItem('opentag.language'),'nl');
    assert.equal(await new ProfileMigrationUI(p.host,initial).restoreAfterStartup(),true);
    assert.deepEqual(p.calls.find(([name])=>name==='workspace')[1],workspace());assert.equal(current.pending,null);
  }finally{p.close();}
});
test('workspace retry durably rearms before browser changes and preserves current preferences on native refusal',async()=>{
  const p=page();try{
    const current=state();current.pending={id:'failed-receipt',attempted:true,failed:true,window:windowState()};current.pending.window.preferences['opentag.language']='en';
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(args?.retry){assert.equal(p.host.storage().getItem('opentag.language'),'nl');throw new Error('controlled journal rearm refusal');}return structuredClone(current);};
    const running=new ProfileMigrationUI(p.host).restore();await settle();p.window.document.querySelector('[data-retry]').click();await settle();
    assert.match(p.window.document.querySelector('[role="alert"]').textContent,/journal rearm refusal/);assert.equal(p.host.storage().getItem('opentag.language'),'nl');
    assert.equal(current.pending.attempted,true);p.window.document.querySelector('[data-close]').click();await running;
  }finally{p.close();}
});

test('storage failure after a durable retry keeps the rearmed receipt and restores quick-source preference on next bootstrap',async()=>{
  const p=page();try{
    const current=state();current.pending={id:'quota-after-rearm',attempted:true,failed:true,window:windowState()};current.pending.window.preferences['opentag.language']='en';current.pending.window.preferences['opentag.filter-shown']='true';current.pending.window.preferences['opentag.tagSources.lastBuiltIn']='source-itunes';
    const data=new Map([['opentag.language','nl'],['opentag.filter-shown','false'],['opentag.tagSources.lastBuiltIn','source-discogs']]);let quota=true;
    const storage={getItem:key=>data.get(key)??null,removeItem:key=>data.delete(key),setItem:(key,value)=>{if(quota&&key==='opentag.filter-shown'&&value==='true')throw new Error('controlled retry storage quota');data.set(key,value);}};p.host.storage=()=>storage;
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(args?.retry){assert.equal(data.get('opentag.language'),'nl');current.pending.attempted=false;current.pending.failed=false;}if(args?.start)current.pending.attempted=true;if(args?.ack)current.pending=null;return structuredClone(current);};
    const running=new ProfileMigrationUI(p.host).restore();await settle();p.window.document.querySelector('[data-retry]').click();await settle();
    assert.match(p.window.document.querySelector('[role="alert"]').textContent,/storage quota/);assert.equal(current.pending.attempted,false);assert.equal(current.pending.failed,false);assert.deepEqual([...data],[['opentag.language','nl'],['opentag.filter-shown','false'],['opentag.tagSources.lastBuiltIn','source-discogs']]);assert.ok(!p.calls.some(([name])=>name==='workspace'));
    p.window.document.querySelector('[data-close]').click();await running;quota=false;
    const initial=await bootstrapProfileMigration(p.host.invoke,p.host.storage);assert.equal(data.get('opentag.tagSources.lastBuiltIn'),'source-itunes');assert.equal(data.get('opentag.language'),'en');
    assert.equal(await new ProfileMigrationUI(p.host,initial).restoreAfterStartup(),true);assert.equal(current.pending,null);
  }finally{p.close();}
});

test('a lost acknowledgement response reconciles the consumed durable receipt without failing or replaying the restored workspace',async()=>{
  const p=page();try{
    const current=state();current.pending={id:'acknowledged-before-response-loss',attempted:false,window:windowState()};
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(args?.start)current.pending.attempted=true;if(args?.ack){current.pending=null;throw new Error('controlled acknowledgement response loss');}if(args?.failed)throw new Error('receipt already consumed');return structuredClone(current);};
    const ui=new ProfileMigrationUI(p.host,await bootstrapProfileMigration(p.host.invoke,p.host.storage));assert.equal(await ui.restoreAfterStartup(),true);
    assert.equal(current.pending,null);assert.deepEqual(p.errors,[]);assert.ok(!p.calls.some(([name,args])=>args?.failed));
    assert.equal(await new ProfileMigrationUI(p.host,await bootstrapProfileMigration(p.host.invoke,p.host.storage)).restoreAfterStartup(),false);assert.equal(p.calls.filter(([name])=>name==='workspace').length,1);
  }finally{p.close();}
});

test('versioned snapshots retain legacy v1 and explicitly carry complete v2 order and working directory',()=>{
  const p=page();try{
    assert.equal(captureProfileWindow(p.host.storage(),state().preference_keys,workspace()).version,1);
    const current={...workspace(),version:2,working_directory:'/Music',list_order:'playlist',source_order:['/Music/practice.flac']};
    assert.deepEqual(captureProfileWindow(p.host.storage(),state().preference_keys,current),{...windowState(),version:2,workspace:current});
    assert.deepEqual(captureProfileWindow(p.host.storage(),state().preference_keys,null),{...windowState(),version:2,workspace:null});
  }finally{p.close();}
});

test('settings-only mobile recovery verifies preferences and acknowledges without loading a startup workspace',async()=>{
  const p=page();try{
    const current={...state(),desktop:false,pending:{id:'portable-settings',attempted:false,window:{...windowState(),version:2,workspace:null}}};
    p.host.storage().setItem('opentag.language','en');
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(args?.start)current.pending.attempted=true;if(args?.ack)current.pending=null;return structuredClone(current);};
    let completed=false;p.host.recovered=state=>{assert.equal(state.pending,null);completed=true;};
    const initial=await bootstrapProfileMigration(p.host.invoke,p.host.storage);
    assert.equal(p.host.storage().getItem('opentag.language'),'nl');
    assert.equal(await new ProfileMigrationUI(p.host,initial).finishStartupRecovery(),true,'completion suppresses incidental startup-folder loading');
    assert.ok(!p.calls.some(([name])=>name==='workspace'));
    assert.equal(current.pending,null);
    assert.equal(completed,true,'complete is reported only after the shared durable acknowledgement');
  }finally{p.close();}
});

test('failed startup recovery requires explicit settings-only choice and keeps ordinary startup gated until reload',async()=>{
  const p=page();try{
    const current={...state(),phase:'applied',restorable:true,pending:{id:'missing-workspace',attempted:true,failed:true,window:{...windowState(),version:2,workspace:{...workspace(),version:2,working_directory:'/Music',list_order:'manual',source_order:null}}}};
    p.host.invoke=async(command,args)=>{p.calls.push([command,args]);if(args?.settingsOnly){assert.equal(args.settingsOnly,'missing-workspace');current.pending.window.workspace=null;current.pending.failed=false;current.pending.attempted=false;}return structuredClone(current);};
    let admitted=false;void new ProfileMigrationUI(p.host,current).finishStartupRecovery().then(()=>{admitted=true;});await settle();
    const dialog=p.window.document.querySelector('dialog');assert.equal(dialog.querySelector('[data-close]').hidden,true);assert.match(dialog.textContent,/file list starts empty/);
    dialog.querySelector('[data-settings-only]').click();await settle();
    assert.equal(current.pending.window.workspace,null);assert.ok(p.calls.some(([name])=>name==='reload'));assert.equal(admitted,false);assert.ok(!p.calls.some(([name,args])=>name==='profile_migration_status'&&args?.ack),'the next bootstrap verifies/acknowledges the rearmed settings-only receipt');
  }finally{p.close();}
});
test('required recovery status failure reaches the blocking startup retry surface',async()=>{
  const p=page();try{
    const current={...state(),pending:{id:'failed',attempted:true,failed:true,window:windowState()}};
    p.host.invoke=async()=>{throw new Error('recovery state unavailable');};
    const outcome=await Promise.race([new ProfileMigrationUI(p.host,current).finishStartupRecovery().then(()=> 'admitted',error=>String(error)),new Promise(resolve=>setTimeout(()=>resolve('unreachable recovery UI'),30))]);
    assert.match(outcome,/recovery state unavailable/);assert.equal(p.errors.length,0,'the main blocking retry surface owns required startup errors');
  }finally{p.close();}
});
test('shared rollback report names portable configuration restores and shows selected counts and omissions',async()=>{
  const p=page();try{
    const current={...state(),phase:'applied',restorable:true,report:{operation:'configuration',items:[],imported:['settings.json'],kept:[],source_hash:'portable-hash',configuration:{version:2,files:1,export_assets:2,preferences:0,workspace_rows:0,omitted:['Incoming workspace was not selected; the current captured workspace is preserved.']}}};
    p.host.invoke=async()=>structuredClone(current);const running=new ProfileMigrationUI(p.host,current).restore();await settle();const dialog=p.window.document.querySelector('dialog');
    assert.match(dialog.textContent,/Last configuration restore/);assert.match(dialog.textContent,/1 saved components/);assert.match(dialog.textContent,/2 export templates/);assert.match(dialog.textContent,/current captured workspace is preserved/);assert.ok(!dialog.textContent.includes('Last setup import'));assert.ok(!dialog.textContent.includes('Legacy codepage:'));
    dialog.querySelector('[data-close]').click();await running;
  }finally{p.close();}
});
