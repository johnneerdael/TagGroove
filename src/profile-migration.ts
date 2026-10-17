/** Setup migration changes configuration, never audio tags. */
export type ProfileWorkspace = {paths:string[];selected:string[];roots:string[];location:string;location_is_data:boolean;filter:string;filter_draft:string|null;anchor:string|null;playlist:string|null};
export type ProfileWindow = {version:1;preferences:Record<string,string>;workspace:ProfileWorkspace};
type Item = {file:string;kind:string;name:string;warnings:string[];collision:boolean;supported:boolean};
type Report = {items:Item[];imported:string[];kept:string[];source_hash:string};
type Pending = {id:string;attempted:boolean;window:ProfileWindow};
type Status = {report:Report|null;restorable:boolean;pending:Pending|null;preference_keys:string[]};
type Preview = {token:string;source_hash:string;items:Item[]};
type Host = {
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  pick:(folder:boolean)=>Promise<string|null>;
  /** Save/discard or cancel pending audio/artwork/action drafts before a reload. */
  prepare:()=>Promise<boolean>;
  flush:()=>Promise<void>;
  pause:()=>void;
  workspace:()=>ProfileWorkspace;
  restoreWorkspace:(workspace:ProfileWorkspace)=>Promise<boolean>;
  storage:Storage;
  reload:()=>void;
  fail:(error:unknown)=>void;
};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export const profileKinds:Record<string,string>={columns:'Columns',panel:'Tag panel',actions:'Action groups',export:'Export templates',unsupported:'Unsupported'};

export function captureProfilePreferences(storage:Storage, keys:readonly string[]):Record<string,string> {
  return Object.fromEntries(keys.flatMap(key=>{const value=storage.getItem(key);return value===null?[]:[[key,value]];}));
}
export function restoreProfilePreferences(storage:Storage, keys:readonly string[], values:Record<string,string>) {
  if(Object.keys(values).some(key=>!keys.includes(key)))throw new Error('The setup snapshot contains an unsupported preference.');
  // Roll back browser preferences on a rejected/quota-limited storage write.
  const previous=captureProfilePreferences(storage,keys);
  try{for(const key of keys){const value=values[key];if(value===undefined)storage.removeItem(key);else storage.setItem(key,value);}}
  catch(error){for(const key of keys){try{const value=previous[key];if(value===undefined)storage.removeItem(key);else storage.setItem(key,value);}catch{/* The pending native receipt remains available for retry. */}}throw error;}
}

export class ProfileMigrationUI {
  private showing=false;
  private state:Status={report:null,restorable:false,pending:null,preference_keys:[]};
  private resume:Pending|null=null;
  constructor(private host:Host){}
  private snapshot():ProfileWindow{return{version:1,preferences:captureProfilePreferences(this.host.storage,this.state.preference_keys),workspace:this.host.workspace()};}
  private async status(args:Record<string,unknown>={}){this.state=await this.host.invoke<Status>('profile_migration_status',args);return this.state;}

  /** Run before desktop configuration reads; recovery may restore native files. */
  async recoverBeforeStartup() {
    try{
      await this.status();const pending=this.state.pending;
      if(!pending||pending.attempted)return true;
      const current=captureProfilePreferences(this.host.storage,this.state.preference_keys);
      if(JSON.stringify(current)!==JSON.stringify(pending.window.preferences)){
        restoreProfilePreferences(this.host.storage,this.state.preference_keys,pending.window.preferences);
        this.host.reload();return false;
      }
      this.resume=pending;return true;
    }catch(error){this.host.fail(error);return false;}
  }
  /** Claim once; failed attempts stay in the report and require explicit retry. */
  async restoreAfterStartup() {
    if(!this.resume)return;
    const pending=this.resume;this.resume=null;
    try{await this.status({start:pending.id});if(await this.host.restoreWorkspace(pending.window.workspace))await this.status({ack:pending.id});}
    catch(error){this.host.fail(error);}
  }

  private dialog(title:string, body:string, footer:string) {
    const dialog=document.createElement('dialog');dialog.className='tag-source-dialog';dialog.setAttribute('aria-label',title);
    dialog.innerHTML=`<header>${html(title)}</header><div class="dialog-content">${body}<p class="dialog-error" role="alert" hidden></p></div><div class="dialog-footer">${footer}</div>`;
    document.body.append(dialog);dialog.showModal();return dialog;
  }
  private error(dialog:HTMLDialogElement,error:unknown){const target=dialog.querySelector<HTMLParagraphElement>('.dialog-error')!;target.hidden=false;target.textContent=String(error).replace(/^Error: /,'');}
  async open() {
    if(this.showing||!await this.host.prepare())return;
    this.showing=true;
    try{
      await this.host.flush();await this.status();
      const mode=await new Promise<boolean|null>(resolve=>{
        const dialog=this.dialog('Import Mp3tag setup','<p>Inspect a configuration folder or ZIP. Columns, tag panels, action groups and export templates can be selected before anything changes. Unsupported settings remain listed. Credentials and audio files are left untouched.</p>','<button data-cancel>Cancel</button><button data-folder>Folder…</button><button data-zip>ZIP…</button>');
        let choice:boolean|null=null;
        dialog.querySelector('[data-folder]')!.addEventListener('click',()=>{choice=true;dialog.close();});dialog.querySelector('[data-zip]')!.addEventListener('click',()=>{choice=false;dialog.close();});dialog.querySelector('[data-cancel]')!.addEventListener('click',()=>dialog.close());dialog.addEventListener('close',()=>{dialog.remove();resolve(choice);},{once:true});
      });
      if(mode===null)return;
      const path=await this.host.pick(mode);if(!path)return;
      const preview=await this.host.invoke<Preview>('profile_migration_preview',{path,window:this.snapshot()});
      await this.preview(preview);
    }catch(error){this.host.fail(error);}finally{this.showing=false;}
  }
  private preview(preview:Preview) {
    const rows=preview.items.map((item,index)=>`<tr><td><input type="checkbox" data-item="${index}" ${item.supported?'':'disabled'} aria-label="${html(item.file)}"></td><td><strong translate="no">${html(item.file)}</strong><div>${html(profileKinds[item.kind]||'Unsupported')}${item.collision?' · Existing configuration':''}</div>${item.warnings.length?`<ul translate="no">${item.warnings.map(warning=>`<li>${html(warning)}</li>`).join('')}</ul>`:''}</td></tr>`).join('');
    const dialog=this.dialog('Review setup migration',`<p>Select supported items to import. Selected column and tag-panel layouts replace the current layouts. Imported action groups stay disabled, including unresolved actions that need editing.</p><table><tbody>${rows}</tbody></table><label class="field"><span>Existing action-group or export-template names</span><select data-collisions><option value="keep">Keep current items</option><option value="replace">Replace current items</option></select></label><p>The previous configuration, physical export templates, histories and workspace remain available through Restore setup. This is separate from audio Undo.</p>`, '<button data-cancel>Cancel</button><button class="primary" data-apply disabled>Import selected setup</button>');
    let working=false;
    const apply=dialog.querySelector<HTMLButtonElement>('[data-apply]')!;
    dialog.querySelector('[data-cancel]')!.addEventListener('click',()=>{if(!working)dialog.close();});
    dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
    dialog.querySelectorAll<HTMLInputElement>('[data-item]').forEach(box=>box.addEventListener('change',()=>{
      const kind=preview.items[Number(box.dataset.item)].kind;
      if(box.checked&&(kind==='columns'||kind==='panel'))dialog.querySelectorAll<HTMLInputElement>('[data-item]').forEach(other=>{if(other!==box&&preview.items[Number(other.dataset.item)].kind===kind)other.checked=false;});
      apply.disabled=!dialog.querySelector('[data-item]:checked');
    }));
    apply.addEventListener('click',async()=>{
      if(working)return;working=true;apply.disabled=true;
      try{
        if(!await this.host.prepare())return;
        await this.host.flush();
        const selected=[...dialog.querySelectorAll<HTMLInputElement>('[data-item]:checked')].map(box=>preview.items[Number(box.dataset.item)].file);
        this.state=await this.host.invoke<Status>('profile_migration_apply',{token:preview.token,selected,replaceNames:dialog.querySelector<HTMLSelectElement>('[data-collisions]')!.value==='replace',window:this.snapshot()});
        this.host.pause();this.host.reload();dialog.close();
      }catch(error){this.error(dialog,error);}finally{working=false;apply.disabled=!dialog.querySelector('[data-item]:checked');}
    });
    return new Promise<void>(resolve=>dialog.addEventListener('close',()=>{dialog.remove();resolve();},{once:true}));
  }
  async restore() {
    if(this.showing||!await this.host.prepare())return;
    this.showing=true;
    try{
      await this.host.flush();await this.status();
      const report=this.state.report;
      const dialog=this.dialog('Restore setup',`${report?`<p>Last setup import</p><ul translate="no">${report.imported.map(file=>`<li>${html(file)}</li>`).join('')}</ul>${report.kept.length?`<p>Kept current items</p><ul translate="no">${report.kept.map(file=>`<li>${html(file)}</li>`).join('')}</ul>`:''}`:'<p>No setup migration snapshot is available.</p>'}<p>Restore replaces the managed configuration and export files with the pre-import snapshot, including histories and workspace. Later configuration changes are replaced. Credentials and audio files remain untouched.</p>${this.state.pending?.attempted?'<p>Workspace restoration is pending. Retry it after its files become available.</p>':''}`, `<button data-close>Cancel</button>${this.state.pending?.attempted?'<button data-retry>Retry workspace</button>':''}<button class="primary" data-restore ${this.state.restorable?'':'disabled'}>Restore previous setup</button>`);
      let working=false;dialog.querySelector('[data-close]')!.addEventListener('click',()=>{if(!working)dialog.close();});dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
      dialog.querySelector('[data-retry]')?.addEventListener('click',async()=>{const pending=this.state.pending;if(!pending||working)return;working=true;try{restoreProfilePreferences(this.host.storage,this.state.preference_keys,pending.window.preferences);await this.status({retry:pending.id});this.host.pause();this.host.reload();dialog.close();}catch(error){this.error(dialog,error);}finally{working=false;}});
      dialog.querySelector('[data-restore]')!.addEventListener('click',async()=>{
        if(working)return;working=true;
        try{
          if(!await this.host.prepare())return;await this.host.flush();
          this.state=await this.host.invoke<Status>('profile_migration_restore',{window:this.snapshot()});
          this.host.pause();
          if(this.state.pending)restoreProfilePreferences(this.host.storage,this.state.preference_keys,this.state.pending.window.preferences);
          this.host.pause();this.host.reload();dialog.close();
        }catch(error){this.error(dialog,error);}finally{working=false;}
      });
      await new Promise<void>(resolve=>dialog.addEventListener('close',()=>{dialog.remove();resolve();},{once:true}));
    }catch(error){this.host.fail(error);}finally{this.showing=false;}
  }
}
