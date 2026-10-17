// Options → Library (https://docs.mp3tag.de/customization/options/library/):
// keep metadata read from files in a database in the configuration folder so
// unchanged files are not read again, optionally only for some directories,
// and remove entries of files that no longer exist.
// Kept free of runtime imports so tests can load it on its own.
import type { OptionsDraft, OptionsPage } from './options-ui';

export type LibraryOptions={enabled:boolean;restrict:boolean;directories:string[]};
export type LibraryStatus={enabled:boolean;created:boolean;files:number;bytes:number;location:string|null;last_hits:number;last_stored:number};
export type CleanupReport={checked:number;removed:number;cancelled:boolean};
export type LibraryHost={
  status:()=>Promise<LibraryStatus>;
  cleanup:(restrict:boolean,directories:string[],job:string)=>Promise<CleanupReport>;
  clear:()=>Promise<LibraryStatus>;
  cancel:(job:string)=>Promise<unknown>;
  progress:(job:string,update:(done:number,total:number)=>void)=>Promise<()=>void>;
  pickDirectories:()=>Promise<string[]>;
};

export const LIBRARY_DEFAULTS:LibraryOptions={enabled:false,restrict:false,directories:[]};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const plural=(count:number,one:string,many=`${one}s`)=>`${count.toLocaleString()} ${count===1?one:many}`;

export function libraryOptions(values:Record<string,unknown>|undefined):LibraryOptions{
  const v=values??{};
  return{enabled:v.enabled===true,restrict:v.restrict===true,directories:Array.isArray(v.directories)?v.directories.filter((d):d is string=>typeof d==='string'&&d.trim()!==''):[]};
}
/** Add picked folders to the list, skipping folders it already holds. */
export function addDirectories(list:string[],picked:string[]):string[]{
  const key=(path:string)=>path.replace(/[\\/]+$/,'').toLowerCase(),seen=new Set(list.map(key)),next=[...list];
  for(const path of picked){const trimmed=path.trim();if(trimmed&&!seen.has(key(trimmed))){seen.add(key(trimmed));next.push(trimmed);}}
  return next;
}
export function formatBytes(bytes:number){
  if(bytes<1024)return`${bytes} B`;
  const units=['KB','MB','GB'];let value=bytes/1024,unit=0;
  while(value>=1024&&unit<units.length-1){value/=1024;unit++;}
  return`${value.toFixed(value<10?1:0)} ${units[unit]}`;
}
export function libraryStatusText(status:LibraryStatus){
  if(!status.created)return'The library is empty. It is created when files are loaded with the library enabled.';
  const size=`${plural(status.files,'file')} in the library (${formatBytes(status.bytes)}).`;
  const last=status.last_hits+status.last_stored?` Last load: ${status.last_hits.toLocaleString()} from the library, ${status.last_stored.toLocaleString()} read from disk and stored.`:'';
  return size+last;
}
export function cleanupText(report:CleanupReport){
  const removalSummary=`removed ${plural(report.removed,'orphaned entry','orphaned entries')}`;
  return report.cancelled?`Cleanup cancelled after ${plural(report.checked,'file')}; ${removalSummary}.`:`Cleanup checked ${plural(report.checked,'file')} and ${removalSummary}.`;
}

const page=(draft:OptionsDraft)=>(draft.pages.library??={...LIBRARY_DEFAULTS,directories:[]}) as Record<string,unknown>;
let ticket=0;

export function libraryOptionsPage(host:LibraryHost):OptionsPage{
  let current=0;
  const directoriesMarkup=(draft:OptionsDraft)=>{
    const {restrict,directories}=libraryOptions(page(draft)),index=Math.min(current,directories.length-1),off=restrict?'':'disabled';
    return`<label class="field"><span>Included directories</span><select id="library-directories" size="5" aria-label="Included directories" ${off}>${directories.map((d,i)=>`<option value="${i}" translate="no" ${i===index?'selected':''}>${html(d)}</option>`).join('')}</select></label>
      <div class="group-buttons"><button type="button" id="library-add" ${off}>Add…</button><button type="button" id="library-remove" ${restrict&&directories.length?'':'disabled'}>Remove</button></div>
      ${restrict&&!directories.length?'<p>No directory is included, so nothing is stored in the library.</p>':''}`;
  };
  const bindDirectories=(root:HTMLElement,draft:OptionsDraft,refresh:()=>void)=>{
    const values=page(draft),list=()=>libraryOptions(values).directories;
    root.querySelector('#library-directories')?.addEventListener('change',event=>{current=Number((event.target as HTMLSelectElement).value);refresh();});
    root.querySelector('#library-add')!.addEventListener('click',async()=>{
      const picked=await host.pickDirectories();if(!picked.length)return;
      const before=list(),next=addDirectories(before,picked);values.directories=next;current=Math.max(0,next.length-1);refresh();root.querySelector<HTMLElement>('#library-add')?.focus();
    });
    root.querySelector('#library-remove')!.addEventListener('click',()=>{const next=list().filter((_,i)=>i!==Math.min(current,list().length-1));values.directories=next;current=Math.max(0,current-1);refresh();root.querySelector<HTMLElement>(next.length?'#library-directories':'#library-add')?.focus();});
  };
  let job='';
  const maintenanceMarkup=()=>`<p id="library-status" role="status">Reading the library…</p>
    <div class="group-buttons"><button type="button" id="library-cleanup">Cleanup Library</button><button type="button" id="library-cancel" hidden>Cancel</button><button type="button" id="library-clear">Clear Library…</button></div>
    <div id="library-clear-confirm" class="group-buttons" hidden><span>Remove every entry? Files are read again the next time they are loaded.</span><button type="button" id="library-clear-yes">Clear</button><button type="button" id="library-clear-no">Keep</button></div>`;
  const bindMaintenance=(root:HTMLElement,draft:OptionsDraft)=>{
    const text=root.querySelector<HTMLElement>('#library-status')!,cleanup=root.querySelector<HTMLButtonElement>('#library-cleanup')!,cancel=root.querySelector<HTMLButtonElement>('#library-cancel')!,
      clear=root.querySelector<HTMLButtonElement>('#library-clear')!,confirm=root.querySelector<HTMLElement>('#library-clear-confirm')!;
    const say=(message:string)=>{if(root.isConnected)text.textContent=message;};
    const show=(status:LibraryStatus)=>{say(libraryStatusText(status));clear.disabled=!status.created||!status.files;};
    const fail=(error:unknown)=>say(String(error).replace(/^Error: /,''));
    void host.status().then(show).catch(fail);
    cancel.addEventListener('click',()=>{if(job)void host.cancel(job);cancel.disabled=true;});
    cleanup.addEventListener('click',async()=>{
      if(job)return;
      const {restrict,directories}=libraryOptions(page(draft));
      job=`library-cleanup-${++ticket}`;cleanup.disabled=true;clear.disabled=true;confirm.hidden=true;cancel.hidden=false;cancel.disabled=false;say('Processing files…');
      const stop=await host.progress(job,(done,total)=>say(`Processing files … ${done.toLocaleString()} of ${total.toLocaleString()}`)).catch(()=>()=>undefined);
      try{const report=await host.cleanup(restrict,directories,job);const status=await host.status();show(status);say(`${cleanupText(report)} ${libraryStatusText(status)}`);}
      catch(error){fail(error);}
      finally{stop();job='';cleanup.disabled=false;cancel.hidden=true;}
    });
    clear.addEventListener('click',()=>{confirm.hidden=false;root.querySelector<HTMLElement>('#library-clear-no')?.focus();});
    root.querySelector('#library-clear-no')!.addEventListener('click',()=>{confirm.hidden=true;clear.focus();});
    root.querySelector('#library-clear-yes')!.addEventListener('click',async()=>{
      confirm.hidden=true;clear.disabled=true;
      try{const status=await host.clear();show(status);say(`The library was cleared. ${libraryStatusText(status)}`);}catch(error){fail(error);}
    });
  };
  return{id:'library',label:'Library',order:45,defaults:{...LIBRARY_DEFAULTS,directories:[]},
    description:'With the library enabled, metadata read from files is kept in a database in the configuration folder. Later loads take files whose modification time and size are unchanged from the database instead of reading them again.',
    controls:[
      {kind:'checkbox',path:'pages.library.enabled',label:'Enable Library'},
      {kind:'group',label:'Directories',controls:[
        {kind:'checkbox',path:'pages.library.restrict',label:'Only include these directories'},
        {kind:'custom',id:'library-directories',paths:['pages.library.directories'],render:directoriesMarkup,bind:bindDirectories},
        {kind:'note',text:'Files outside these directories are read from disk every time and never stored.'}]},
      {kind:'group',label:'Maintenance',controls:[
        {kind:'custom',id:'library-maintenance',paths:[],render:maintenanceMarkup,bind:bindMaintenance},
        {kind:'note',text:'Saving, renaming, moving and deleting files in TagGroove updates the library. Cleanup Library removes entries of files that were deleted or moved outside TagGroove and, with a directory restriction, of files outside the included directories. File → Reload tags (⌘/Ctrl T) reads the selected files again and replaces their entries.'}]}]};
}
