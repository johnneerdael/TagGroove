/** `file` names the `.mte` in the export folder for configurations found there (e.g. copied from Mp3tag). */
export type ExportConfiguration={id:string;name:string;template:string;file?:string};
export type ExportPreferences={configuration:string|null;filename:string;append:boolean;partition:string|null;write_bom:boolean;create_directories:boolean};
export type ExportLibrary={version:number;configurations:ExportConfiguration[];preferences:ExportPreferences};
function uniqueName(library:ExportLibrary,base:string):string {
  let name=base,index=2;while(library.configurations.some(c=>c.name.toLowerCase()===name.toLowerCase()))name=`${base} ${index++}`;return name;
}
export function copyExportConfiguration(library:ExportLibrary,id:string,newId:string):ExportLibrary {
  const result=structuredClone(library),source=result.configurations.find(c=>c.id===id);
  if(!source)throw new Error('Export configuration does not exist.');
  if(!newId||result.configurations.some(c=>c.id===newId))throw new Error('Duplicate configuration identity.');
  const {file:_file,...copy}=source;
  result.configurations.push({...copy,id:newId,name:uniqueName(result,`${source.name} copy`)});result.preferences.configuration=newId;return result;
}
/** Rename in place (F2 or a delayed click in the list): names stay nonempty and unique. */
export function renameExportConfiguration(library:ExportLibrary,id:string,name:string):ExportLibrary {
  const result=structuredClone(library),target=result.configurations.find(c=>c.id===id),trimmed=name.trim();
  if(!target)throw new Error('Export configuration does not exist.');
  if(!trimmed)throw new Error('Enter a name for the export configuration.');
  if(result.configurations.some(c=>c.id!==id&&c.name.toLowerCase()===trimmed.toLowerCase()))throw new Error(`An export configuration named ${trimmed} already exists.`);
  target.name=trimmed;return result;
}
export function removeExportConfiguration(library:ExportLibrary,id:string):ExportLibrary {
  const result=structuredClone(library);result.configurations=result.configurations.filter(c=>c.id!==id);
  if(result.preferences.configuration===id)result.preferences.configuration=result.configurations[0]?.id??null;return result;
}

type ExportHost={
  load:()=>Promise<ExportLibrary>;save:(library:ExportLibrary)=>Promise<void>;
  import:(path:string,encoding:string)=>Promise<ExportConfiguration>;
  pickTemplate:()=>Promise<string|null>;pickOutput:(current:string)=>Promise<string|null>;
  /** Run the active configuration; `perDirectory` writes one file per folder (File → Export with Shift). */
  execute:(library:ExportLibrary,perDirectory:boolean)=>Promise<void>;reason:()=>string;selectedCount:()=>number;
};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export class ExportUI {
  private showing=false;
  constructor(private host:ExportHost){}
  /** File → Export without the selection dialog (Options → Export). */
  async runSaved(perDirectory=false){
    const reason=this.host.reason();if(reason)throw new Error(reason);
    const library=await this.host.load();
    if(!library.configurations.some(c=>c.id===library.preferences.configuration))throw new Error('Select an export configuration in File → Export first.');
    await this.host.execute(library,perDirectory);
  }
  async open(perDirectory=false){
    if(this.showing)return;this.showing=true;
    let library:ExportLibrary;
    try{library=structuredClone(await this.host.load());}catch(error){this.showing=false;throw error;}
    let working=false;
    const dialog=document.createElement('dialog');dialog.className='export-dialog';document.body.append(dialog);
    const active=()=>library.configurations.find(c=>c.id===library.preferences.configuration);
    const value=(id:string)=>(dialog.querySelector<HTMLInputElement|HTMLTextAreaElement|HTMLSelectElement>(`#${id}`)!).value;
    const checked=(id:string)=>dialog.querySelector<HTMLInputElement>(`#${id}`)!.checked;
    const error=(error:unknown)=>{let p=dialog.querySelector<HTMLElement>('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(p);}p.textContent=String(error);};
    const capture=()=>{
      const current=active();if(current)current.template=value('export-template');
      Object.assign(library.preferences,{filename:value('export-filename'),append:checked('export-append'),partition:checked('export-partition-enabled')?value('export-partition'):null});
    };
    const sync=()=>{
      dialog.querySelectorAll<HTMLInputElement|HTMLButtonElement|HTMLSelectElement|HTMLTextAreaElement>('input,button,select,textarea').forEach(control=>control.disabled=working);
      for(const id of ['export-template','export-copy','export-remove','export-rename'])dialog.querySelector<HTMLInputElement|HTMLButtonElement>(`#${id}`)!.disabled=working||!active();
      dialog.querySelector<HTMLInputElement>('#export-partition')!.disabled=working||perDirectory||!checked('export-partition-enabled');
      if(perDirectory)dialog.querySelector<HTMLInputElement>('#export-partition-enabled')!.disabled=true;
      const reason=this.host.reason();dialog.querySelector<HTMLButtonElement>('#export-run')!.disabled=working||!active()||!!reason;
      dialog.querySelector<HTMLElement>('#export-reason')!.textContent=reason||`${this.host.selectedCount()} selected files.${perDirectory?' One export file is created in each folder (Shift).':''} Existing output files are replaced unless Append data is enabled.`;
    };
    /** Edit the selected name in place: Enter or leaving the field keeps it, Escape cancels. */
    const rename=()=>{
      const current=active(),item=current&&dialog.querySelector<HTMLElement>(`#export-list li[data-id="${CSS.escape(current.id)}"]`);
      if(!current||!item||working||item.querySelector('input'))return;
      capture();
      const input=document.createElement('input');input.className='export-rename';input.value=current.name;input.setAttribute('aria-label','Configuration name');input.spellcheck=false;
      item.replaceChildren(input);input.focus();input.select();
      let done=false;
      const finish=(keep:boolean)=>{
        if(done)return;done=true;
        if(keep&&input.value.trim()!==current.name){try{library=renameExportConfiguration(library,current.id,input.value);}catch(e){render();error(e instanceof Error?e.message:String(e));return;}}
        render();dialog.querySelector<HTMLElement>('#export-list')!.focus();
      };
      input.addEventListener('keydown',event=>{event.stopPropagation();if(event.key==='Enter'){event.preventDefault();finish(true);}if(event.key==='Escape'){event.preventDefault();finish(false);}});
      input.addEventListener('blur',()=>finish(true));
    };
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();sync();
      try{await work();}catch(e){if(dialog.open)error(e);}finally{working=false;if(dialog.open)sync();}
    };
    const render=()=>{
      const current=active(),preferences=library.preferences;
      dialog.innerHTML=`<header>Export</header><div class="dialog-content"><div class="field"><span id="export-list-label">Export configurations</span><ul id="export-list" class="export-list" role="listbox" tabindex="0" aria-labelledby="export-list-label"${current?` aria-activedescendant="export-item-${html(current.id)}"`:''}>${library.configurations.map(c=>`<li role="option" id="export-item-${html(c.id)}" data-id="${html(c.id)}" aria-selected="${c.id===preferences.configuration}" translate="no">${html(c.name)}</li>`).join('')}</ul></div>
      <div class="group-buttons"><button id="export-new">New</button><button id="export-copy">Duplicate</button><button id="export-rename">Rename</button><button id="export-remove">Remove</button><button id="export-import">Import .mte…</button></div>
      <p class="export-hint">Rename the selected configuration with F2 or a second click on its name; F4 edits its template.</p>
      <label class="field"><span>Export template</span><textarea id="export-template" aria-label="Export template" spellcheck="false" autocorrect="off" autocapitalize="off" data-helper="format-export">${html(current?.template||'')}</textarea></label>
      <label class="field"><span>Legacy import encoding (optional)</span><input id="export-encoding" placeholder="For example windows-1252"></label>
      <label class="field"><span>Export file name</span><div class="action-input"><input id="export-filename" value="${html(preferences.filename)}" placeholder="Use configuration default" data-helper="format-export"><button id="export-browse">Browse…</button></div></label>
      <p>Relative output names use the first selected file’s folder. Leave the name empty to use the template’s default filename.</p>
      <label class="action-check"><input type="checkbox" id="export-append" ${preferences.append?'checked':''}> Append data</label>
      <label class="action-check"><input type="checkbox" id="export-partition-enabled" ${preferences.partition!==null?'checked':''}> Create one output file per</label>
      <label class="field"><span>Partition format</span><input id="export-partition" value="${html(preferences.partition||'')}" placeholder="%album%" data-helper="format"></label>
      <p>Byte order marks and directory creation are set in Options → Export.</p>
      <p id="export-reason"></p><p id="export-status" role="status">Save configurations to remember templates and options. Export also saves them.</p></div>
      <div class="dialog-footer"><button id="export-cancel">Cancel</button><button id="export-save">Save configurations</button><button class="primary" id="export-run">Export</button></div>`;
      const on=(id:string,handler:()=>void)=>dialog.querySelector(`#${id}`)!.addEventListener('click',handler);
      const list=dialog.querySelector<HTMLElement>('#export-list')!;
      const choose=(id:string)=>{if(id===library.preferences.configuration)return;capture();library.preferences.configuration=id;render();dialog.querySelector<HTMLElement>('#export-list')!.focus();};
      let renameTimer:ReturnType<typeof setTimeout>|undefined;
      const cancelTimer=()=>{if(renameTimer!==undefined){clearTimeout(renameTimer);renameTimer=undefined;}};
      list.addEventListener('click',event=>{
        const item=(event.target as Element).closest<HTMLElement>('li[data-id]');if(!item||working||item.querySelector('input'))return;
        // A second, separate click on the selected name renames it (Windows list behaviour).
        if(item.dataset.id===library.preferences.configuration&&event.detail===1){cancelTimer();renameTimer=setTimeout(()=>{renameTimer=undefined;rename();},500);return;}
        choose(item.dataset.id!);
      });
      list.addEventListener('dblclick',()=>{cancelTimer();dialog.querySelector<HTMLTextAreaElement>('#export-template')!.focus();});
      list.addEventListener('keydown',event=>{
        if(working||(event.target as Element).closest('input'))return;
        const ids=library.configurations.map(c=>c.id),index=ids.indexOf(library.preferences.configuration??'');
        const move=(next:number)=>{event.preventDefault();const id=ids[Math.max(0,Math.min(ids.length-1,next))];if(id)choose(id);};
        if(event.key==='ArrowDown')move(index+1);else if(event.key==='ArrowUp')move(index-1);else if(event.key==='Home')move(0);else if(event.key==='End')move(ids.length-1);
        else if(event.key==='Enter'){event.preventDefault();dialog.querySelector<HTMLTextAreaElement>('#export-template')!.focus();}
      });
      on('export-new',()=>{capture();const id=crypto.randomUUID();library.configurations.push({id,name:uniqueName(library,'New export'),template:'$filename(txt,utf-8)$loop(1)%title%$char(13)$char(10)$loopend()'});library.preferences.configuration=id;render();rename();});
      on('export-rename',()=>rename());
      on('export-copy',()=>{capture();library=copyExportConfiguration(library,library.preferences.configuration!,crypto.randomUUID());render();});
      on('export-remove',()=>{capture();library=removeExportConfiguration(library,library.preferences.configuration!);render();});
      on('export-import',()=>{capture();const encoding=value('export-encoding');void perform(async()=>{const path=await this.host.pickTemplate();if(!path)return;const imported=await this.host.import(path,encoding);imported.name=uniqueName(library,imported.name);library.configurations.push(imported);library.preferences.configuration=imported.id;render();});});
      on('export-browse',()=>{capture();void perform(async()=>{const path=await this.host.pickOutput(library.preferences.filename);if(path){library.preferences.filename=path;dialog.querySelector<HTMLInputElement>('#export-filename')!.value=path;}});});
      on('export-save',()=>{capture();const snapshot=structuredClone(library);void perform(async()=>{await this.host.save(snapshot);if(dialog.open){render();dialog.querySelector<HTMLElement>('#export-status')!.textContent='Export configurations and options saved.';}});});
      on('export-run',()=>{capture();const reason=this.host.reason();if(reason){error(reason);return;}const snapshot=structuredClone(library);void perform(async()=>{await this.host.execute(snapshot,perDirectory);dialog.close();});});
      on('export-cancel',()=>dialog.close());
      dialog.querySelector('#export-partition-enabled')!.addEventListener('change',sync);
      sync();
    };
    dialog.addEventListener('cancel',event=>{if(working||dialog.querySelector('.export-rename'))event.preventDefault();});
    dialog.addEventListener('close',()=>{this.showing=false;dialog.remove();});
    dialog.addEventListener('keydown',event=>{if(working)return;if(event.key==='F2'){event.preventDefault();rename();}if(event.key==='F4'){event.preventDefault();dialog.querySelector<HTMLTextAreaElement>('#export-template')!.focus();}});
    render();dialog.showModal();
  }
}

export type OpenOutputsHost={
  modal:(title:string,content:string,buttons:string)=>HTMLDialogElement;
  modalError:(dialog:HTMLDialogElement,error:unknown)=>void;
  /** Open files with their default programs. */
  open:(paths:string[])=>Promise<void>;
  reveal:(paths:string[])=>Promise<void>;
  fileManager:string;
};
const baseName=(path:string)=>path.split(/[\\/]/).pop()||path;
const folderName=(path:string)=>path.replace(/[\\/][^\\/]*$/,'');
/**
 * Options → Messages "at exporting and creating playlists": offer to open what
 * was written. One file is opened directly; with several (partitions, one file
 * per folder) the file to open is chosen from the list. Resolves when closed.
 */
export function offerOpenOutputs(host:OpenOutputsHost,outputs:string[],message:string,title='Export'):Promise<void>{
  if(!outputs.length)return Promise.resolve();
  const single=outputs.length===1;
  const content=single?`<p>${html(message)}.</p><p>Open <strong translate="no">${html(baseName(outputs[0]))}</strong> now?</p>`
    :`<p>${html(message)}.</p><label class="field"><span>Written files (${outputs.length})</span><select id="open-output-file" size="${Math.min(outputs.length,6)}" aria-label="Written files">${outputs.map((path,index)=>`<option value="${index}" translate="no" ${index===0?'selected':''} title="${html(path)}">${html(baseName(path))} — ${html(folderName(path))}</option>`).join('')}</select></label>`;
  const dialog=host.modal(title,content,`<button id="open-output-reveal">Show in ${html(host.fileManager)}</button><button class="primary" id="open-output-run">Open</button>`);
  dialog.classList.add('open-output-dialog');
  const chosen=()=>single?outputs[0]:outputs[Number(dialog.querySelector<HTMLSelectElement>('#open-output-file')!.value)]??outputs[0];
  const act=(work:(path:string)=>Promise<void>,close:boolean)=>async()=>{try{await work(chosen());if(close)dialog.close();}catch(error){host.modalError(dialog,error);}};
  dialog.querySelector('#open-output-run')!.addEventListener('click',act(path=>host.open([path]),true));
  dialog.querySelector('#open-output-reveal')!.addEventListener('click',act(path=>host.reveal([path]),false));
  dialog.querySelector('#open-output-file')?.addEventListener('dblclick',act(path=>host.open([path]),true));
  dialog.querySelector<HTMLElement>(single?'#open-output-run':'#open-output-file')!.focus();
  return new Promise(resolve=>dialog.addEventListener('close',()=>resolve()));
}
