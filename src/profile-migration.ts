import type {SourceRootKind} from './playlist-order';
/** Setup migration changes configuration, never audio tags. */
type LegacyWorkspace = {paths:string[];selected:string[];roots:string[];location:string;location_is_data:boolean;filter:string;filter_draft:string|null;anchor:string|null;playlist:string|null};
export type ProfileWorkspaceV2 = LegacyWorkspace & {version:2;root_kinds?:SourceRootKind[];working_directory:string|null;list_order:'columns'|'manual'|'playlist';source_order:string[]|null};
export type ProfileWorkspace = LegacyWorkspace | ProfileWorkspaceV2;
export type ProfileWindow = {version:1;preferences:Record<string,string>;workspace:LegacyWorkspace}|{version:2;preferences:Record<string,string>;workspace:ProfileWorkspaceV2|null};
type Item = {file:string;kind:string;name:string;warnings:string[];collision:boolean;supported:boolean;settings?:{label:string;enabled?:boolean;value?:string;verbatim?:boolean}[]};
type Report = {operation?:'profile'|'configuration';configuration?:{version:number;files:number;export_assets:number;preferences:number;workspace_rows:number;omitted:string[]};legacy_encoding?:string|null;items:Item[];imported:string[];kept:string[];source_hash:string};
type Pending = {id:string;attempted:boolean;failed?:boolean;window:ProfileWindow};
export type ProfileStatus = {desktop:boolean;phase:string;report:Report|null;restorable:boolean;pending:Pending|null;preference_keys:string[];portable_preference_keys?:string[]};
type Status=ProfileStatus;
type Preview = {token:string;source_hash:string;items:Item[]};
type Host = {
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  pick:(folder:boolean)=>Promise<string|null>;
  /** Save/discard or cancel pending audio/artwork/action drafts before a reload. */
  prepare:()=>Promise<boolean>;
  flush:()=>Promise<void>;
  pause:()=>void;
  workspace:()=>ProfileWorkspace|null;
  restoreWorkspace:(workspace:ProfileWorkspace)=>Promise<boolean>;
  storage:()=>Storage;
  reload:()=>void;
  recovered?:(state:ProfileStatus)=>void;
  fail:(error:unknown)=>void;
};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export const profileKinds:Record<string,string>={columns:'Columns',panel:'Tag panel',field_values:'Field values',settings:'Portable settings',actions:'Action groups',export:'Export templates',unsupported:'Unsupported'};

export function captureProfilePreferences(storage:Storage, keys:readonly string[]):Record<string,string> {
  return Object.fromEntries(keys.flatMap(key=>{const value=storage.getItem(key);return value===null?[]:[[key,value]];}));
}
export function captureProfileWindow(storage:Storage,keys:readonly string[],workspace:ProfileWorkspace|null):ProfileWindow {
  const preferences=captureProfilePreferences(storage,keys);
  return workspace===null||'version'in workspace?{version:2,preferences,workspace}:{version:1,preferences,workspace};
}
export function restoreProfilePreferences(storage:Storage, keys:readonly string[], values:Record<string,string>) {
  if(Object.keys(values).some(key=>!keys.includes(key)))throw new Error('The setup snapshot contains an unsupported preference.');
  // Roll back browser preferences on a rejected/quota-limited storage write.
  const previous=captureProfilePreferences(storage,keys);
  try{
    for(const key of keys){const value=values[key];if(value===undefined)storage.removeItem(key);else storage.setItem(key,value);}
    for(const key of keys)if(storage.getItem(key)!==(values[key]??null))throw new Error('Setup preference readback did not match the requested values.');
  }
  catch(error){for(const key of keys){try{const value=previous[key];if(value===undefined)storage.removeItem(key);else storage.setItem(key,value);}catch{/* The pending native receipt remains available for retry. */}}throw error;}
}

/** Recover before constructing any configuration-backed frontend service. */
export async function bootstrapProfileMigration(invoke:Host['invoke'], storage:()=>Storage):Promise<Status> {
  const state=await invoke<Status>('profile_migration_status');
  if(state.pending&&!state.pending.failed)restoreProfilePreferences(storage(),state.preference_keys,state.pending.window.preferences);
  return state;
}

export function showProfileRecoveryFailure(target:HTMLElement,error:unknown,reload:()=>void) {
  target.style.visibility='';target.inert=false;target.replaceChildren();
  const dialog=document.createElement('dialog');dialog.className='tag-source-dialog profile-recovery-dialog';dialog.setAttribute('aria-label','Startup recovery');
  const message=document.createElement('p');message.setAttribute('role','alert');message.textContent=`Startup recovery could not finish. Existing recovery files are retained. ${String(error)}`;
  const retry=document.createElement('button');retry.textContent='Retry startup recovery';retry.addEventListener('click',reload);
  dialog.append(message,retry);dialog.addEventListener('cancel',event=>event.preventDefault());target.append(dialog);dialog.showModal();retry.focus();
}

export class ProfileMigrationUI {
  private showing=false;
  private state:Status={desktop:true,phase:'none',report:null,restorable:false,pending:null,preference_keys:[]};
  private resume:Pending|null=null;
  private startupFailure:unknown;
  constructor(private host:Host, initial?:Status){if(initial){this.state=initial;if(initial.pending&&!initial.pending.failed)this.resume=initial.pending;}}
  private snapshot():ProfileWindow{return captureProfileWindow(this.host.storage(),this.state.preference_keys,this.host.workspace());}
  private async status(args:Record<string,unknown>={}){this.state=await this.host.invoke<Status>('profile_migration_status',args);return this.state;}

  /** Resume interrupted claims; durably failed attempts require explicit retry. */
  async restoreAfterStartup():Promise<boolean> {
    if(!this.resume)return false;
    const pending=this.resume;this.resume=null;
    // An ambiguous claim failure must block startup; there is no durable proof
    // that replay was marked attempted, so later work must not be admitted.
    await this.status({start:pending.id});
    let restored=false;
    try{
      restored=pending.window.workspace===null||await this.host.restoreWorkspace(pending.window.workspace);
      if(restored){
        try{await this.status({ack:pending.id});}
        catch(error){
          // A durable acknowledgement may outlive its IPC response. Reconcile
          // consumption before trying to mark a receipt that no longer exists.
          if((await this.status()).pending!==null)throw error;
        }
      }else throw new Error('The captured workspace could not be read completely.');
    }catch(error){
      // Publish a completed failure before admitting later work. A crash before
      // this write leaves the claim resumable; a refused write blocks startup.
      await this.status({failed:pending.id});
      this.startupFailure=new Error(`Workspace recovery remains pending. Open Restore setup to retry. ${String(error)}`);
      this.host.fail(this.startupFailure);
    }
    if(restored)this.host.recovered?.(this.state);
    return restored;
  }
  reportStartupFailure(){if(this.startupFailure)this.host.fail(this.startupFailure);}
  /** A failed receipt requires an explicit choice before normal startup can proceed. */
  async finishStartupRecovery():Promise<boolean>{
    const restored=await this.restoreAfterStartup();
    if(this.state.pending?.failed){await this.restore(true);return await new Promise<never>(()=>{});}
    return restored;
  }

  private dialog(title:string, body:string, footer:string) {
    const dialog=document.createElement('dialog');dialog.className='tag-source-dialog';dialog.setAttribute('aria-label',title);
    dialog.innerHTML=`<header>${html(title)}</header><div class="dialog-content">${body}<p class="dialog-error" role="alert" hidden></p></div><div class="dialog-footer">${footer}</div>`;
    document.body.append(dialog);dialog.showModal();return dialog;
  }
  private error(dialog:HTMLDialogElement,error:unknown){const target=dialog.querySelector<HTMLParagraphElement>('.dialog-error')!;target.hidden=false;target.textContent=String(error).replace(/^Error: /,'');}
  async open() {
    if(this.showing)return;
    this.showing=true;
    try{
      await this.host.flush();await this.status();
      let legacyEncoding:string|null=null;
      const mode=await new Promise<boolean|null>(resolve=>{
        const dialog=this.dialog('Import Mp3tag setup','<p>Inspect a configuration folder or ZIP. Columns, tag panels, field values, portable settings, action groups and export templates can be selected before anything changes. Unsupported settings remain listed. Credentials and audio files are left untouched.</p><label class="field"><span>Legacy codepage for actions and exports (optional)</span><input data-encoding placeholder="For example windows-1252; empty uses current importer defaults" maxlength="64"></label>','<button data-cancel>Cancel</button><button data-folder>Folder…</button><button data-zip>ZIP…</button>');
        let choice:boolean|null=null;
        const choose=(folder:boolean)=>{legacyEncoding=dialog.querySelector<HTMLInputElement>('[data-encoding]')!.value.trim()||null;choice=folder;dialog.close();};
        dialog.querySelector('[data-folder]')!.addEventListener('click',()=>choose(true));dialog.querySelector('[data-zip]')!.addEventListener('click',()=>choose(false));dialog.querySelector('[data-cancel]')!.addEventListener('click',()=>dialog.close());dialog.addEventListener('close',()=>{dialog.remove();resolve(choice);},{once:true});
      });
      if(mode===null)return;
      const path=await this.host.pick(mode);if(!path)return;
      const preview=await this.host.invoke<Preview>('profile_migration_preview',{path,window:this.snapshot(),legacyEncoding});
      await this.preview(preview);
    }catch(error){this.host.fail(error);}finally{this.showing=false;}
  }
  private preview(preview:Preview) {
    const rows=preview.items.map((item,index)=>`<tr><td><input type="checkbox" data-item="${index}" ${item.supported?'':'disabled'} aria-label="${html(item.file)}"></td><td><strong translate="no">${html(item.file)}</strong><div>${html(profileKinds[item.kind]||'Unsupported')}${item.collision?' · Existing configuration':''}</div>${item.settings?.length?`<ul>${item.settings.map(setting=>`<li><span>${html(setting.label)}</span>: <span${setting.verbatim?' translate="no"':''}>${html(setting.value??(setting.enabled?'On':'Off'))}</span></li>`).join('')}</ul>`:''}${item.warnings.length?`<ul translate="no">${item.warnings.map(warning=>`<li>${html(warning)}</li>`).join('')}</ul>`:''}</td></tr>`).join('');
    const dialog=this.dialog('Review setup migration',`<p>Select supported items to import. Selected column and tag-panel layouts replace the current layouts. Selected portable settings replace the listed matching options. Unmapped binary records are not imported. Imported field-value entries replace matching saved lists and preferences. A selected usrfieldvalues.ini takes precedence over values in a selected legacy panel. Imported action groups stay disabled, including unresolved actions that need editing.</p><table><tbody>${rows}</tbody></table><label class="field"><span>Existing action-group or export-template names</span><select data-collisions><option value="keep">Keep current items</option><option value="replace">Replace current items</option></select></label><p>The previous configuration, physical export templates, histories and workspace remain available through Restore setup. This is separate from audio Undo.</p>`, '<button data-cancel>Cancel</button><button class="primary" data-apply disabled>Import selected setup</button>');
    let working=false,reloadRequired=false,completed=false;
    const apply=dialog.querySelector<HTMLButtonElement>('[data-apply]')!;
    dialog.querySelector('[data-cancel]')!.addEventListener('click',()=>{if(!working)dialog.close();});
    dialog.addEventListener('cancel',event=>{if(working||reloadRequired)event.preventDefault();});
    dialog.querySelectorAll<HTMLInputElement>('[data-item]').forEach(box=>box.addEventListener('change',()=>{
      const kind=preview.items[Number(box.dataset.item)].kind;
      if(box.checked&&['columns','panel','field_values','settings'].includes(kind))dialog.querySelectorAll<HTMLInputElement>('[data-item]').forEach(other=>{if(other!==box&&preview.items[Number(other.dataset.item)].kind===kind)other.checked=false;});
      apply.disabled=!dialog.querySelector('[data-item]:checked');
    }));
    apply.addEventListener('click',async()=>{
      if(working)return;working=true;apply.disabled=true;
      try{
        if(!await this.host.prepare())return;
        await this.host.flush();
        const selected=[...dialog.querySelectorAll<HTMLInputElement>('[data-item]:checked')].map(box=>preview.items[Number(box.dataset.item)].file);
        this.host.pause();reloadRequired=true;
        this.state=await this.host.invoke<Status>('profile_migration_apply',{token:preview.token,selected,replaceNames:dialog.querySelector<HTMLSelectElement>('[data-collisions]')!.value==='replace',window:this.snapshot()});
        completed=true;this.host.reload();dialog.close();
      }catch(error){this.error(dialog,error);if(reloadRequired)dialog.querySelector('[data-cancel]')!.textContent='Reload setup';}finally{working=false;apply.disabled=reloadRequired||!dialog.querySelector('[data-item]:checked');}
    });
    return new Promise<void>(resolve=>dialog.addEventListener('close',()=>{dialog.remove();if(reloadRequired&&!completed)this.host.reload();resolve();},{once:true}));
  }
  async restore(required=false) {
    if(this.showing)return;
    this.showing=true;
    try{
      await this.host.flush();await this.status();
      const report=this.state.report,configuration=report?.operation==='configuration',summary=report?.configuration;
      const summaryMarkup=summary?`<p>Selected restoration: ${summary.files} saved components, ${summary.export_assets} export templates, ${summary.preferences} saved preference values and ${summary.workspace_rows} imported workspace rows.</p><ul>${summary.omitted.map(item=>`<li>${html(item)}</li>`).join('')}</ul>`:'';
      const phaseLabels:Record<string,string>={none:'No migration',applied:'Imported',restored:'Restored',failed:'Rolled back'};
      const dialog=this.dialog('Restore setup',`<p>Setup state: <span>${html(phaseLabels[this.state.phase]||'Recovery pending')}</span></p>${report?`<p>${configuration?'Last configuration restore':'Last setup import'}</p>${summaryMarkup}<ul translate="no">${report.imported.map(file=>`<li>${html(file)}</li>`).join('')}</ul>${report.kept.length?`<p>Kept current items</p><ul translate="no">${report.kept.map(file=>`<li>${html(file)}</li>`).join('')}</ul>`:''}`:'<p>No setup migration snapshot is available.</p>'}${report?`<details><summary>${configuration?'Full configuration restore report':'Full migration report'}</summary>${configuration?'':`<p>Legacy codepage: <span translate="no">${html(report.legacy_encoding||'Importer defaults')}</span></p>`}<ul>${report.items.map(item=>`<li><strong translate="no">${html(item.file)}</strong> · ${report.imported.includes(item.file)?'Imported':report.kept.includes(item.file)?'Kept current':'Not imported'}${item.warnings.length?`<ul translate="no">${item.warnings.map(warning=>`<li>${html(warning)}</li>`).join('')}</ul>`:''}</li>`).join('')}</ul><p translate="no">SHA-256: ${html(report.source_hash)}</p></details>`:''}<p>${configuration?'Restore replaces saved configuration, export templates, histories and workspace with the snapshot from before the configuration restore.':'Restore replaces the managed configuration and export files with the pre-import snapshot, including histories and workspace.'} Later configuration changes are replaced. Credentials and audio files remain untouched.</p>${this.state.pending?.attempted?'<p>Workspace restoration is pending. Retry it after its files become available.</p><p>Retry workspace restores captured window preferences and the file list. The managed native configuration stays as it is.</p>':''}`, `<button data-close>Cancel</button>${this.state.pending?.attempted?'<button data-retry>Retry workspace</button>':''}<button class="primary" data-restore ${this.state.restorable?'':'disabled'}>Restore previous setup</button>`);
      const cancel=dialog.querySelector<HTMLButtonElement>('[data-close]')!;
      if(required)cancel.hidden=true;
      if(this.state.pending?.window.workspace!==null&&this.state.pending){
        const only=document.createElement('button');only.dataset.settingsOnly='';only.textContent='Continue with settings only';dialog.querySelector('.dialog-footer')!.insertBefore(only,dialog.querySelector('[data-restore]'));
        const explanation=document.createElement('p');explanation.textContent='Continue with settings only omits the captured workspace. After reload, the file list starts empty; the startup folder is not opened.';dialog.querySelector('.dialog-content')!.append(explanation);
        only.addEventListener('click',async()=>{
          const pending=this.state.pending;if(!pending||working)return;working=true;
          try{
            if(!await this.host.prepare())return;await this.host.flush();this.host.pause();reloadRequired=true;
            await this.status({settingsOnly:pending.id});
            if(!this.state.pending||this.state.pending.window.workspace!==null)throw new Error('Settings-only recovery was not confirmed.');
            restoreProfilePreferences(this.host.storage(),this.state.preference_keys,this.state.pending.window.preferences);
            completed=true;this.host.reload();dialog.close();
          }catch(error){this.error(dialog,error);if(reloadRequired){cancel.hidden=false;cancel.textContent='Reload setup';only.disabled=true;}}
          finally{working=false;}
        });
      }
      let working=false,reloadRequired=false,completed=false;cancel.addEventListener('click',()=>{if(!working)dialog.close();});dialog.addEventListener('cancel',event=>{if(required||working||reloadRequired)event.preventDefault();});
      dialog.querySelector('[data-retry]')?.addEventListener('click',async()=>{const pending=this.state.pending;if(!pending||working)return;working=true;try{
        if(!await this.host.prepare())return;await this.host.flush();this.host.pause();reloadRequired=true;
        await this.status({retry:pending.id});restoreProfilePreferences(this.host.storage(),this.state.preference_keys,pending.window.preferences);completed=true;this.host.reload();dialog.close();
      }catch(error){this.error(dialog,error);if(reloadRequired){cancel.hidden=false;cancel.textContent='Reload setup';dialog.querySelector<HTMLButtonElement>('[data-retry]')!.disabled=true;dialog.querySelector<HTMLButtonElement>('[data-restore]')!.disabled=true;}}finally{working=false;}});
      dialog.querySelector('[data-restore]')!.addEventListener('click',async()=>{
        if(working)return;working=true;
        try{
          if(!await this.host.prepare())return;await this.host.flush();
          this.host.pause();reloadRequired=true;
          this.state=await this.host.invoke<Status>('profile_migration_restore',{window:this.snapshot()});
          this.host.pause();
          if(this.state.pending)restoreProfilePreferences(this.host.storage(),this.state.preference_keys,this.state.pending.window.preferences);
          completed=true;this.host.reload();dialog.close();
        }catch(error){this.error(dialog,error);if(reloadRequired){cancel.hidden=false;cancel.textContent='Reload setup';dialog.querySelector<HTMLButtonElement>('[data-restore]')!.disabled=true;}}finally{working=false;}
      });
      await new Promise<void>(resolve=>dialog.addEventListener('close',()=>{dialog.remove();if(reloadRequired&&!completed)this.host.reload();resolve();},{once:true}));
    }catch(error){if(required)throw error;this.host.fail(error);}finally{this.showing=false;}
  }
}
