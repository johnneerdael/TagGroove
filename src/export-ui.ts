export type ExportConfiguration={id:string;name:string;template:string};
export type ExportPreferences={configuration:string|null;filename:string;append:boolean;partition:string|null;write_bom:boolean;create_directories:boolean};
export type ExportLibrary={version:number;configurations:ExportConfiguration[];preferences:ExportPreferences};
function uniqueName(library:ExportLibrary,base:string):string {
  let name=base,index=2;while(library.configurations.some(c=>c.name.toLowerCase()===name.toLowerCase()))name=`${base} ${index++}`;return name;
}
export function copyExportConfiguration(library:ExportLibrary,id:string,newId:string):ExportLibrary {
  const result=structuredClone(library),source=result.configurations.find(c=>c.id===id);
  if(!source)throw new Error('Export configuration does not exist.');
  if(!newId||result.configurations.some(c=>c.id===newId))throw new Error('Duplicate configuration identity.');
  result.configurations.push({...source,id:newId,name:uniqueName(result,`${source.name} copy`)});result.preferences.configuration=newId;return result;
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
      const current=active();if(current){current.name=value('export-name');current.template=value('export-template');}
      Object.assign(library.preferences,{filename:value('export-filename'),append:checked('export-append'),partition:checked('export-partition-enabled')?value('export-partition'):null});
    };
    const sync=()=>{
      dialog.querySelectorAll<HTMLInputElement|HTMLButtonElement|HTMLSelectElement|HTMLTextAreaElement>('input,button,select,textarea').forEach(control=>control.disabled=working);
      for(const id of ['export-name','export-template','export-copy','export-remove'])dialog.querySelector<HTMLInputElement|HTMLButtonElement>(`#${id}`)!.disabled=working||!active();
      dialog.querySelector<HTMLInputElement>('#export-partition')!.disabled=working||perDirectory||!checked('export-partition-enabled');
      if(perDirectory)dialog.querySelector<HTMLInputElement>('#export-partition-enabled')!.disabled=true;
      const reason=this.host.reason();dialog.querySelector<HTMLButtonElement>('#export-run')!.disabled=working||!active()||!!reason;
      dialog.querySelector<HTMLElement>('#export-reason')!.textContent=reason||`${this.host.selectedCount()} selected files.${perDirectory?' One export file is created in each folder (Shift).':''} Existing output files are replaced unless Append data is enabled.`;
    };
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();sync();
      try{await work();}catch(e){if(dialog.open)error(e);}finally{working=false;if(dialog.open)sync();}
    };
    const render=()=>{
      const current=active(),preferences=library.preferences;
      dialog.innerHTML=`<header>Export</header><div class="dialog-content"><label class="field"><span>Export configurations</span><select id="export-list" size="4" aria-label="Export configurations">${library.configurations.map(c=>`<option value="${html(c.id)}" ${c.id===preferences.configuration?'selected':''}>${html(c.name)}</option>`).join('')}</select></label>
      <div class="group-buttons"><button id="export-new">New</button><button id="export-copy">Duplicate</button><button id="export-remove">Remove</button><button id="export-import">Import .mte…</button></div>
      <label class="field"><span>Configuration name</span><input id="export-name" value="${html(current?.name||'')}"></label>
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
      dialog.querySelector('#export-list')!.addEventListener('change',()=>{const next=value('export-list');capture();library.preferences.configuration=next||null;render();});
      dialog.querySelector('#export-list')!.addEventListener('dblclick',()=>dialog.querySelector<HTMLTextAreaElement>('#export-template')!.focus());
      on('export-new',()=>{capture();const id=crypto.randomUUID();library.configurations.push({id,name:uniqueName(library,'New export'),template:'$filename(txt,utf-8)$loop(1)%title%$char(13)$char(10)$loopend()'});library.preferences.configuration=id;render();dialog.querySelector<HTMLInputElement>('#export-name')!.focus();});
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
    dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
    dialog.addEventListener('close',()=>{this.showing=false;dialog.remove();});
    dialog.addEventListener('keydown',event=>{if(!working&&(event.key==='F2'||event.key==='F4')){event.preventDefault();dialog.querySelector<HTMLInputElement|HTMLTextAreaElement>(event.key==='F2'?'#export-name':'#export-template')!.focus();}});
    render();dialog.showModal();
  }
}
