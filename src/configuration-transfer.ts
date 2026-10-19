import type {ProfileStatus,ProfileWindow,ProfileWorkspaceV2} from './profile-migration';

type Selection={files:string[]|null;preferences:boolean;workspace:boolean;export_assets:boolean};
type Mapping={from:string;to:string};
export type ConfigurationPreview={token:string;version:1|2;source_hash:string;files:string[];export_assets:string[];preference_keys:string[];preferences_available:boolean;source_workspace:{rows:number;roots:string[];path_style:string}|null;workspace:ProfileWorkspaceV2|null;workspace_available:boolean;unavailable:string[];warnings:string[];replaces_snapshot:boolean};
type ExportReceipt={version:2;files:number;export_assets:number;preferences:number;workspace_rows:number;omitted:string[]};
type Host={
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  pickSave:()=>Promise<string|null>;pickLoad:()=>Promise<string|null>;
  capture:(keys:readonly string[])=>ProfileWindow;
  flush:()=>Promise<void>;pause:()=>void;resume:()=>void;hold:(held:boolean)=>void;
  /** Authorize later discard, without deleting live audio/artwork drafts. */
  prepare:()=>Promise<boolean>;
  inspectWorkspace:(workspace:ProfileWorkspaceV2)=>Promise<string[]>;
  reload:()=>void;report:(message:string)=>void;fail:(error:unknown)=>void;
  preferenceKeys?:()=>readonly string[];
  recoveryPending?:()=>boolean;
};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const idle=(state:ProfileStatus)=>!state.pending&&['none','applied','restored','failed'].includes(state.phase);

/** Await saved configuration publications, including failures already reported by a dialog. */
export class SavedConfigurationWrites {
  private pending=new Set<Promise<unknown>>();private failure:unknown;private paused=false;
  run<T>(write:()=>Promise<T>):Promise<T>{
    if(this.paused)return Promise.reject(new Error('Configuration writes are paused until setup recovery finishes.'));
    const result=write().then(value=>{this.pending.delete(result);return value;},error=>{this.pending.delete(result);this.failure=error;throw error;});
    this.pending.add(result);return result;
  }
  async flush(){while(this.pending.size)await Promise.allSettled([...this.pending]);if(this.failure!==undefined){const failure=this.failure;this.failure=undefined;throw failure;}}
  pause(){this.paused=true;}
  resume(){this.paused=false;}
}

/** Portable configuration uses the shared setup receipt; it has no second recovery loop. */
export class ConfigurationTransferUI {
  private showing=false;
  constructor(private host:Host){}
  async save(){
    if(this.showing)return;this.showing=true;
    try{
      if(this.host.recoveryPending?.())throw new Error('Finish or restore the pending setup change before saving configuration.');
      const outputPath=await this.host.pickSave();if(!outputPath)return;
      await this.host.flush();
      const receipt=await this.host.invoke<ExportReceipt>('configuration_transfer_export',{outputPath,window:this.host.capture(this.host.preferenceKeys?.()??[])});
      this.host.report(`Saved configuration: ${receipt.files} saved components, ${receipt.export_assets} export templates, ${receipt.preferences} preferences and ${receipt.workspace_rows} workspace rows. Excluded: ${receipt.omitted.join('; ')}`);
    }catch(error){this.host.fail(error);}finally{this.showing=false;}
  }
  async load(){
    if(this.showing)return;this.showing=true;
    try{
      const path=await this.host.pickLoad();if(!path)return;
      await this.host.flush();
      const keys=this.host.preferenceKeys?.()??[];
      const preview=await this.host.invoke<ConfigurationPreview>('configuration_transfer_preview',{path,window:this.host.capture(keys)});
      await this.review(path,preview,keys);
    }catch(error){this.host.fail(error);}finally{this.showing=false;}
  }
  private async review(path:string,initial:ConfigurationPreview,keys:readonly string[]){
    let preview=initial,working=false,reviewed=true,recoveryRequired=false,revision=0;
    const missing=initial.workspace?await this.host.inspectWorkspace(initial.workspace):[];
    const files=[...initial.files],selection:Selection={files:[...files],preferences:initial.preferences_available,workspace:!!initial.workspace&&initial.workspace_available&&missing.length===0,export_assets:initial.export_assets.length>0};
    reviewed=!(initial.source_workspace||initial.workspace)||selection.workspace;
    const dialog=document.createElement('dialog');dialog.className='tag-source-dialog configuration-transfer-dialog';
    dialog.setAttribute('aria-label','Review saved configuration');
    dialog.innerHTML=`<header>Review saved configuration</header><div class="dialog-content"><p>Restore saved configuration and local workspace references only. Unsaved audio, artwork and action-editor changes are not included. No audio tags are saved by this operation.</p><p translate="no">${html(path)}</p>${files.map(file=>`<label class="action-check"><input type="checkbox" data-file="${html(file)}" checked><span translate="no">${html(file)}</span></label>`).join('')}<label class="action-check"><input type="checkbox" data-assets ${selection.export_assets?'checked':''} ${initial.export_assets.length?'':'disabled'}>Export templates</label><ul translate="no">${initial.export_assets.map(asset=>`<li>${html(asset)}</li>`).join('')}</ul><label class="action-check"><input type="checkbox" data-preferences ${selection.preferences?'checked':''} ${initial.preferences_available?'':'disabled'}>Replace saved preferences; absent known preferences reset to defaults</label><label class="action-check"><input type="checkbox" data-workspace ${selection.workspace?'checked':''} ${initial.workspace?'':'disabled'}>Restore the complete workspace</label><p data-workspace-summary></p><details data-workspace-inventory><summary>Captured workspace details</summary><p>Path style: <span data-workspace-path-style translate="no"></span></p><p>Original roots</p><ul data-workspace-roots translate="no"></ul></details><p>Unselected components and the current file list are preserved. Restoring a selected empty workspace clears the list.</p><div data-review-details></div><p>${initial.replaces_snapshot?'This replaces the latest setup rollback checkpoint.':'A rollback checkpoint will retain the current setup.'}</p><details><summary>Move a local music root</summary><p>Map only explicitly chosen local roots. Other saved paths are not rewritten.</p><div data-mapping-rows><div data-mapping><label class="field"><span>Original root</span><input data-map-from></label><label class="field"><span>Replacement root</span><input data-map-to></label></div></div><button type="button" data-add-mapping>Add root mapping</button></details><p class="dialog-error" role="alert" hidden></p></div><div class="dialog-footer"><button data-cancel>Cancel</button><button data-review>Review selection</button><button class="primary" data-apply>Replace selected configuration</button></div>`;
    document.body.append(dialog);dialog.showModal();
    const apply=dialog.querySelector<HTMLButtonElement>('[data-apply]')!,review=dialog.querySelector<HTMLButtonElement>('[data-review]')!,cancel=dialog.querySelector<HTMLButtonElement>('[data-cancel]')!;
    apply.disabled=!reviewed;
    const error=(value:unknown)=>{const target=dialog.querySelector<HTMLElement>('[role="alert"]')!;target.hidden=false;target.textContent=String(value).replace(/^Error: /,'');};
    const selected=()=>{
      selection.files=[...dialog.querySelectorAll<HTMLInputElement>('[data-file]:checked')].map(box=>box.dataset.file!);
      selection.preferences=dialog.querySelector<HTMLInputElement>('[data-preferences]')!.checked;
      selection.workspace=dialog.querySelector<HTMLInputElement>('[data-workspace]')!.checked;
      selection.export_assets=dialog.querySelector<HTMLInputElement>('[data-assets]')!.checked;
      return selection;
    };
    const mappings=():Mapping[]=>[...dialog.querySelectorAll<HTMLElement>('[data-mapping]')].flatMap(row=>{
      const from=row.querySelector<HTMLInputElement>('[data-map-from]')!.value.trim(),to=row.querySelector<HTMLInputElement>('[data-map-to]')!.value.trim();
      if(!from&&!to)return[];if(!from||!to)throw new Error('Enter both roots for each path mapping.');return[{from,to}];
    });
    const visibleChoices=()=>JSON.stringify({selection:selected(),mappings:[...dialog.querySelectorAll<HTMLElement>('[data-mapping]')].map(row=>({from:row.querySelector<HTMLInputElement>('[data-map-from]')!.value,to:row.querySelector<HTMLInputElement>('[data-map-to]')!.value}))});
    let reviewedChoices=visibleChoices(),reviewedRevision=revision;
    const unchanged=(version:number,choices:string)=>dialog.isConnected&&dialog.open&&version===revision&&choices===visibleChoices();
    const disabledBeforeWork=new Map<HTMLInputElement|HTMLButtonElement,boolean>();
    const lockControls=()=>{
      for(const control of dialog.querySelectorAll<HTMLInputElement|HTMLButtonElement>('input,[data-add-mapping]')){disabledBeforeWork.set(control,control.disabled);control.disabled=true;}
      review.disabled=true;cancel.disabled=true;
    };
    const unlockControls=()=>{
      for(const [control,disabled]of disabledBeforeWork)control.disabled=recoveryRequired||disabled;
      disabledBeforeWork.clear();review.disabled=recoveryRequired;cancel.disabled=false;
      apply.disabled=recoveryRequired||!reviewed||!unchanged(reviewedRevision,reviewedChoices);
    };
    const details=(unavailable:string[])=>{
      const source=preview.source_workspace,local=preview.workspace;
      dialog.querySelector('[data-workspace-summary]')!.textContent=source||local?`Captured workspace: ${source?.rows??local?.paths.length??0} rows and ${source?.roots.length??local?.roots.length??0} roots. Unavailable: ${unavailable.length}.`:'No workspace is included in this backup.';
      dialog.querySelector<HTMLElement>('[data-workspace-inventory]')!.hidden=!source;
      dialog.querySelector('[data-workspace-path-style]')!.textContent=source?.path_style??'';
      const roots=dialog.querySelector('[data-workspace-roots]')!;roots.replaceChildren(...(source?.roots??[]).map(root=>{const item=document.createElement('li');item.textContent=root;return item;}));
      dialog.querySelector('[data-review-details]')!.innerHTML=`<ul translate="no">${[...preview.warnings,...unavailable].map(warning=>`<li>${html(warning)}</li>`).join('')}</ul>`;
    };
    details([...new Set([...initial.unavailable,...missing])]);
    const invalidate=()=>{revision++;reviewed=false;apply.disabled=true;};
    dialog.addEventListener('change',invalidate);dialog.addEventListener('input',invalidate);
    dialog.querySelector('[data-add-mapping]')!.addEventListener('click',()=>{if(working||recoveryRequired)return;const row=dialog.querySelector('[data-mapping]')!.cloneNode(true) as HTMLElement;row.querySelectorAll('input').forEach(input=>input.value='');dialog.querySelector('[data-mapping-rows]')!.append(row);invalidate();});
    review.addEventListener('click',async()=>{
      if(working||recoveryRequired)return;working=true;apply.disabled=true;const requestRevision=revision,choices=visibleChoices();lockControls();
      try{
        const options=structuredClone(selected()),roots=mappings();
        const next=await this.host.invoke<ConfigurationPreview>('configuration_transfer_preview',{path,window:this.host.capture(keys),options,mappings:roots});
        if(!unchanged(requestRevision,choices)){reviewed=false;return;}
        if(next.source_hash!==initial.source_hash)throw new Error('The backup changed. Close this review and open the backup again.');
        const unavailable=next.workspace?await this.host.inspectWorkspace(next.workspace):[];
        if(!unchanged(requestRevision,choices)){reviewed=false;return;}
        preview=next;details([...new Set([...preview.unavailable,...unavailable])]);reviewed=!(options.workspace&&(!preview.workspace_available||unavailable.length));
        reviewedChoices=choices;reviewedRevision=requestRevision;
      }catch(cause){reviewed=false;error(cause);}finally{working=false;unlockControls();}
    });
    cancel.addEventListener('click',()=>{if(working)return;if(recoveryRequired)this.host.reload();dialog.close();});
    dialog.addEventListener('cancel',event=>{if(working||recoveryRequired)event.preventDefault();});
    apply.addEventListener('click',async()=>{
      if(working||!reviewed||recoveryRequired)return;
      if(!unchanged(reviewedRevision,reviewedChoices)){invalidate();return;}
      const requestRevision=revision,choices=reviewedChoices,token=preview.token;
      working=true;apply.disabled=true;lockControls();let paused=false;
      try{
        if(!await this.host.prepare())return;
        if(!unchanged(requestRevision,choices)){reviewed=false;return;}
        this.host.hold(true);await this.host.flush();
        if(!unchanged(requestRevision,choices)){reviewed=false;this.host.hold(false);return;}
        this.host.pause();paused=true;
        const state=await this.host.invoke<ProfileStatus>('configuration_transfer_apply',{token,window:this.host.capture(keys)});
        if(!state.pending){recoveryRequired=true;throw new Error('Configuration restore could not confirm its recovery state. Reload setup recovery before continuing.');}
        recoveryRequired=true;this.host.report('Configuration applied. Reload must finish restoring saved preferences and the file list; recovery is pending.');this.host.reload();dialog.close();
      }catch(cause){
        reviewed=false;error(cause);
        if(paused&&!recoveryRequired){
          try{const state=await this.host.invoke<ProfileStatus>('profile_migration_status');if(idle(state)){this.host.resume();this.host.hold(false);}else recoveryRequired=true;}
          catch{recoveryRequired=true;}
        }else if(!paused)this.host.hold(false);
        if(recoveryRequired){cancel.textContent='Reload setup recovery';review.disabled=true;}
      }finally{working=false;unlockControls();}
    });
    await new Promise<void>(resolve=>dialog.addEventListener('close',()=>{dialog.remove();resolve();},{once:true}));
  }
}
