/** Fields and format string helper menus: the ► buttons next to field-name and
 * format-string inputs (docs.mp3tag.de/menus/fields/ and /menus/format-strings/).
 * Inputs opt in declaratively so modules only add attributes:
 *   data-helper="field"          field-name input (Remove field / Remove all fields / Reset fields)
 *   data-helper="format"         generating format string: fields, information fields, functions
 *   data-helper="format-export"  as format, plus Export and Playlist information fields
 *   data-helper="placeholder"    tag field placeholders only
 *   data-helper="pattern"        matching pattern (Filename → Tag, Guess Values): fields and %dummy%
 *   data-helper="regex" / "regex-replacement"  regular expression snippets
 *   data-helper="history"        history commands only
 *   data-helper="filter"         filter: format plus Auto-apply filter
 *   data-history="key"           remembered format strings; data-history-manage enables Manage history…;
 *                                data-history-enter also remembers on Enter inside a dialog
 *   data-specials="_FILENAME,…"  special names offered by a field-name input's drop-down list */
export type HelperKind='field'|'format'|'format-export'|'placeholder'|'pattern'|'regex'|'regex-replacement'|'history'|'filter';
export type HelperStorage={getItem(key:string):string|null;setItem(key:string,value:string):void;removeItem?(key:string):void};
/** `favorite` entries (ticked in Manage List) are offered in the ► menu. */
export type HistoryEntry={value:string;name?:string;description?:string;favorite?:boolean};
export type HelperHost={
  /** Translate new popup content before reading layout dimensions. */
  translateNow?:(root:HTMLElement)=>void;
  fieldNames:()=>string[];
  panelFields:()=>string[];
  confirm:(message:string)=>Promise<boolean>;
  filterAutoApply?:{get:()=>boolean;set:(value:boolean)=>void};
  storage?:HelperStorage;
};
export type HelperEntry={label:string;insert?:string;command?:string;disabled?:boolean;checked?:boolean;submenu?:HelperEntry[];heading?:boolean;separator?:boolean};

/** Default contents of the user-editable field list shown in field-name drop-down lists. */
export const defaultFieldList=['ALBUM','ALBUMARTIST','ALBUMARTISTSORT','ALBUMSORT','ARTIST','ARTISTSORT','BPM','COMMENT','COMPILATION','COMPOSER','COMPOSERSORT','CONDUCTOR','CONTENTGROUP','COPYRIGHT','DISCNUMBER','ENCODEDBY','ENCODERSETTINGS','GENRE','INITIALKEY','ISRC','LANGUAGE','LYRICIST','MOOD','ORIGARTIST','ORIGYEAR','PUBLISHER','RATING','SUBTITLE','TITLE','TITLESORT','TRACK','UNSYNCEDLYRICS','WWW','YEAR'];
const defaultPanelFields=['TITLE','ARTIST','ALBUM','YEAR','TRACK','GENRE','COMMENT','ALBUMARTIST','COMPOSER','DISCNUMBER'];

/** Information placeholders by documentation section; Export/Playlist only where those values exist. */
export const informationFields:[string,string[],boolean?][]=[
  ['Technical information',['_bitrate','_bitspersample','_codec','_cover_description','_cover_height','_cover_mimetype','_cover_size','_cover_type','_cover_width','_covers','_id3v2_character_encoding','_id3v2_unknown_frames','_length','_length_seconds','_mode','_mp4_creation_time','_mp4_creation_time_raw','_mp4_modification_time','_mp4_modification_time_raw','_samplerate','_subsong','_subsong_count','_tag','_tag_read','_tag_size','_tag_size_appended','_tag_size_prepended','_tool','_vbr','_video_bitrate','_video_codec','_video_height','_video_width']],
  ['File name and path',['_directory','_extension','_filename','_filename_ext','_folderpath','_parent_directory','_path','_volume','_workingdir','_workingpath']],
  ['File properties',['_file_create_date','_file_create_datetime','_file_create_datetime_raw','_file_mod_date','_file_mod_datetime','_file_mod_datetime_raw','_file_size','_file_size_bytes','_file_size_kb','_file_size_mb']],
  ['Dynamic information',['_app','_counter','_date','_datetime']],
  ['Export',['_crc','_filename_rel','_folderpath_rel','_max_counter','_md5','_md5audio','_total_files','_total_size','_total_size_raw','_total_time','_total_time2','_total_time_raw'],true],
  ['Playlist',['_playlist_filename','_playlist_filename_ext','_playlist_folderpath'],true],
];
/** Documented scripting function signatures, grouped as in the scripting reference. */
export const scriptFunctions:[string,string[]][]=[
  ['String functions',['$ansi(x)','$caps(x,y)','$caps2(x,y)','$caps3(x,y)','$char(x)','$cutLeft(x,n)','$cutRight(x,n)','$dedup(x,y,c)','$distance(x,y)','$fmtDate(fmt,x)','$fmtDuration(x,y,h,ms)','$fmtNum(x)','$folderdepth(x)','$left(x,n)','$len(x)','$lower(x)','$mid(x,i,n)','$num(x,y)','$ord(x)','$regexp(x,e,r,c)','$repeat(x,n)','$replace(x,y,z)','$reverse(x)','$right(x,n)','$sort(x,y)','$strchr(x,y)','$strcmp(x,y)','$stricmp(x,y)','$strrchr(x,y)','$strrstr(x,y)','$strstr(x,y)','$trim(x,c)','$trimLeft(x,c)','$trimRight(x,c)','$upper(x)','$validate(x,y)']],
  ['Boolean functions',['$and(x,y)','$eql(x,y)','$geql(x,y)','$grtr(x,y)','$if(x,y,z)','$if2(x,y)','$ifgreater(a,b,x,y)','$iflonger(a,b,x,y)','$isdigit(x)','$leql(x,y)','$less(x,y)','$neql(x,y)','$not(x)','$odd(x)','$or(x,y)']],
  ['Arithmetic functions',['$add(x,y)','$div(x,y)','$mod(x,y)','$mul(x,y)','$rand()','$sub(x,y)','$uuid()']],
  ['Variable functions',['$getenv(x)']],
  ['Metadata functions',['$columnNames(x,y)','$columnValues(x,y)','$list(x,y,z,v)','$meta(x)','$meta(x,n)','$meta_sep(x,sep)']],
];
/** Regular expression snippets (an OpenTag addition to the helper menu for regex inputs). */
export const regexSnippets:[string,string][]=[['Any text (.*)','(.*)'],['Shortest text (.*?)','(.*?)'],['Digits \\d+','\\d+'],['Whitespace \\s+','\\s+'],['Word \\w+','\\w+'],['Word boundary \\b','\\b'],['Start of text ^','^'],['End of text $','$'],['Character set [abc]','[abc]'],['Excluded characters [^abc]','[^abc]'],['Group (…)','()'],['Alternatives a|b','|'],['Optional ?','?'],['Repeat n to m times {n,m}','{n,m}'],['Escaped dot \\.','\\.']];
export const regexReplacementSnippets:[string,string][]=[['Sub-expression 1 $1','$1'],['Sub-expression 2 $2','$2'],['Sub-expression 3 $3','$3']];

const upper=(name:string)=>name.replace(/[a-z]/g,c=>c.toUpperCase());
const validField=(name:string)=>!!name&&!/[=\0\r\n;%$]/.test(name);
function read<T>(storage:HelperStorage|undefined,key:string,fallback:T):T{try{const raw=storage?.getItem(key);return raw?JSON.parse(raw) as T:fallback;}catch{return fallback;}}
function write(storage:HelperStorage|undefined,key:string,value:unknown){try{storage?.setItem(key,JSON.stringify(value));}catch{/* Lists are conveniences; failures keep the session list. */}}

/** The field list behind field-name drop-down lists: grows as names are used, and can be pruned or reset. */
export class FieldList{
  private static key='opentag.field-list';
  constructor(private storage?:HelperStorage){}
  list():string[]{const value=read<unknown>(this.storage,FieldList.key,null);return Array.isArray(value)?value.filter((v):v is string=>typeof v==='string'):[...defaultFieldList];}
  has(name:string){const id=upper(name.trim());return this.list().some(field=>upper(field)===id);}
  add(name:string){const field=name.trim();if(!validField(field)||field.startsWith('_')||this.has(field))return false;write(this.storage,FieldList.key,[...this.list(),upper(field)].sort());return true;}
  remove(name:string){const id=upper(name.trim()),list=this.list(),next=list.filter(field=>upper(field)!==id);if(next.length===list.length)return false;write(this.storage,FieldList.key,next);return true;}
  clear(){write(this.storage,FieldList.key,[]);}
  reset(){write(this.storage,FieldList.key,[...defaultFieldList]);}
}

/** Remembered format strings per input; named entries come from Manage history…. */
export class FormatHistory{
  static limit=25;
  constructor(private storage?:HelperStorage){}
  private key(id:string){return`opentag.history.${id}`;}
  list(id:string):HistoryEntry[]{const value=read<unknown>(this.storage,this.key(id),[]);return Array.isArray(value)?value.filter((v):v is HistoryEntry=>!!v&&typeof v.value==='string'):[];}
  /** Keeps at most `limit` entries; the oldest non-favorites go first. */
  set(id:string,entries:HistoryEntry[]){
    const kept=[...entries];
    for(let index=kept.length-1;index>=0&&kept.length>FormatHistory.limit;index--)if(!kept[index].favorite)kept.splice(index,1);
    write(this.storage,this.key(id),kept);
  }
  favorites(id:string){return this.list(id).filter(entry=>entry.favorite);}
  /** Most recent first; a named entry keeps its name and description when used again. */
  add(id:string,value:string){if(!value.trim())return;const list=this.list(id),existing=list.find(entry=>entry.value===value);this.set(id,[existing||{value},...list.filter(entry=>entry.value!==value)]);}
  remove(id:string,value:string){const list=this.list(id),next=list.filter(entry=>entry.value!==value);this.set(id,next);return next.length!==list.length;}
  clear(id:string){this.set(id,[]);}
}

/** Helper lists that belong to the configuration rather than the window. */
export const configListKey=(key:string)=>key==='opentag.field-list'||key.startsWith('opentag.history.');
type LegacyStorage=HelperStorage&{readonly length:number;key(index:number):string|null};
export type ListBackend={load:()=>Promise<{exists:boolean;lists:Record<string,unknown>}>;save:(lists:Record<string,unknown>)=>Promise<void>};
/**
 * The helper lists (field list, histories with names, descriptions and
 * favorites) kept in the configuration's lists.json, so configuration backups
 * include them. Reads are synchronous from memory; changes are saved shortly
 * after. The first load moves lists that earlier versions kept in the
 * window's local storage into the configuration.
 */
export class ConfigListStorage implements HelperStorage{
  private data=new Map<string,string>();
  private early=new Set<string>();
  private loaded=false;private paused=false;
  private timer:ReturnType<typeof setTimeout>|undefined;
  readonly ready:Promise<void>;
  constructor(private backend:ListBackend,private legacy?:LegacyStorage,private onError:(error:unknown)=>void=()=>{},private delay=150){this.ready=this.load();}
  getItem(key:string){return this.data.get(key)??null;}
  setItem(key:string,value:string){this.data.set(key,value);if(!this.loaded)this.early.add(key);this.schedule();}
  removeItem(key:string){this.data.delete(key);if(!this.loaded)this.early.add(key);this.schedule();}
  private async load(){
    let migrated:string[]=[];
    try{
      const saved=await this.backend.load();
      if(saved.exists){for(const [key,value] of Object.entries(saved.lists))if(!this.early.has(key))this.data.set(key,JSON.stringify(value));}
      else migrated=this.migrate();
    }catch(error){this.onError(error);}
    this.loaded=true;
    if(migrated.length){
      try{await this.flush();for(const key of migrated)this.legacy?.removeItem?.(key);}catch(error){this.onError(error);}
    }else if(this.early.size)this.schedule();
  }
  private migrate(){
    const keys:string[]=[];
    try{
      const legacy=this.legacy;if(!legacy)return keys;
      for(let index=0;index<legacy.length;index++){const key=legacy.key(index);if(key&&configListKey(key))keys.push(key);}
      for(const key of keys){
        const raw=legacy.getItem(key);if(raw===null||this.early.has(key))continue;
        try{if(Array.isArray(JSON.parse(raw)))this.data.set(key,raw);}catch{/* A damaged list is left behind. */}
      }
    }catch{/* Without local storage there is nothing to move. */}
    return keys.filter(key=>this.data.has(key));
  }
  private schedule(){
    if(!this.loaded||this.paused)return;
    if(this.timer!==undefined)clearTimeout(this.timer);
    this.timer=setTimeout(()=>{this.timer=undefined;this.flush().catch(this.onError);},this.delay);
  }
  /** Save now (for example before the configuration is replaced). */
  async flush(){
    if(this.timer!==undefined){clearTimeout(this.timer);this.timer=undefined;}
    const lists:Record<string,unknown>={};
    for(const [key,raw] of this.data){try{lists[key]=JSON.parse(raw);}catch{/* Unreadable values are not saved. */}}
    await this.backend.save(lists);
  }
  /** Stop saving, so a configuration that is being restored is not overwritten. */
  pause(){this.paused=true;if(this.timer!==undefined){clearTimeout(this.timer);this.timer=undefined;}}
}

/** A copy of `entries[index]` placed after it, named so the copy can be told apart. */
export function duplicateEntry(entries:HistoryEntry[],index:number):HistoryEntry{
  const source=entries[index];if(!source)throw new Error('Select an entry to duplicate.');
  const copy:HistoryEntry={...source};
  if(source.name){let name=`${source.name} copy`,count=2;while(entries.some(entry=>entry.name===name))name=`${source.name} copy ${count++}`;copy.name=name;}
  return copy;
}

/** Insert text at the caret (replacing a selection) and select the first parameter of an inserted function. */
export function insertion(value:string,start:number,end:number,text:string):{value:string;start:number;end:number}{
  const next=value.slice(0,start)+text+value.slice(end),open=text.indexOf('(');
  if(text.startsWith('$')&&open>0&&text.endsWith(')')){const stop=text.slice(open+1).search(/[,)]/);if(stop>0)return{value:next,start:start+open+1,end:start+open+1+stop};}
  return{value:next,start:start+text.length,end:start+text.length};
}
export const placeholder=(field:string)=>`%${field.toLowerCase()}%`;
/** Names for the Extended fields submenu: everything known that the panel section does not already list. */
export function extendedFields(panel:string[],known:string[]){const shown=new Set(panel.map(upper));return [...new Set(known.filter(name=>validField(name)&&!name.startsWith('_')).map(upper))].filter(name=>!shown.has(name)).sort();}

export type HelperContext={kind:HelperKind;panel:string[];known:string[];history?:boolean;manage?:boolean;historySelected?:boolean;value?:string;inFieldList?:boolean;autoApply?:boolean;favorites?:HistoryEntry[]};
/** Menu model per documented section order. Pure so the contents can be tested without a DOM. */
export function helperEntries(context:HelperContext):HelperEntry[]{
  const {kind}=context,entries:HelperEntry[]=[],separator=()=>{if(entries.length&&!entries[entries.length-1].separator)entries.push({label:'',separator:true});};
  if(kind==='field'){
    const choices=[...new Set([...context.known.map(upper)])].filter(validField).sort();
    if(choices.length)entries.push({label:'Choose field',submenu:choices.map(name=>({label:name,command:`set:${name}`}))});
    separator();entries.push({label:'Remove field',command:'field-remove',disabled:!context.inFieldList},{label:'Remove all fields…',command:'field-clear'},{label:'Reset fields…',command:'field-reset'});
    return entries;
  }
  if(['format','format-export','placeholder','pattern','filter'].includes(kind)){
    const panel=context.panel.filter(name=>validField(name)&&!name.startsWith('_'));
    entries.push(...panel.map(name=>({label:name,insert:placeholder(name)})));
    if(kind==='pattern')entries.push({label:'DUMMY (skip text)',insert:'%dummy%'});
    const extended=extendedFields(panel,context.known);
    if(extended.length)entries.push({label:'Extended fields',submenu:extended.map(name=>({label:name,insert:placeholder(name)}))});
    if(kind==='format'||kind==='format-export'||kind==='filter'){
      entries.push({label:'Information fields',submenu:informationFields.filter(([, ,exportOnly])=>!exportOnly||kind==='format-export').flatMap(([group,names])=>[{label:group,heading:true},...names.map(name=>({label:`%${name}%`,insert:`%${name}%`}))])});
      entries.push({label:'Functions',submenu:scriptFunctions.flatMap(([group,names])=>[{label:group,heading:true},...names.map(name=>({label:name,insert:name}))])});
    }
  }
  if(kind==='regex'||kind==='regex-replacement')entries.push({label:'Regular expression',submenu:(kind==='regex'?regexSnippets:regexReplacementSnippets).map(([label,insert])=>({label,insert}))});
  if(context.history){
    separator();
    // Favorites (ticked in Manage List) replace the input's value.
    for(const entry of context.favorites||[])entries.push({label:entry.name||entry.value,command:`set:${entry.value}`});
    if(context.favorites?.length)separator();
    if(context.manage)entries.push({label:'Manage history…',command:'history-manage'});
    else entries.push({label:'Remove from history',command:'history-remove',disabled:!context.historySelected},{label:'Remove all from history…',command:'history-clear'});
  }
  if(kind==='filter'&&context.autoApply!==undefined){separator();entries.push({label:'Auto-apply filter',command:'filter-auto',checked:context.autoApply});}
  return entries;
}

/** Keyboard reference "Useful helpers": Ctrl+Space opens the helper menu of the focused
 * input (the physical Control key on macOS too, where ⌘Space is Spotlight; Control+Option+Space
 * also works there because macOS can claim ⌃Space for input sources). Shift+Del removes the
 * current value from that input's drop-down history. */
export function helperKey(event:{key:string;code?:string;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean;altKey:boolean}):'open'|'remove-history'|undefined{
  if(event.ctrlKey&&!event.metaKey&&!event.shiftKey&&(event.code==='Space'||event.key===' '))return 'open';
  if(event.shiftKey&&!event.ctrlKey&&!event.metaKey&&!event.altKey&&event.key==='Delete')return 'remove-history';
  return undefined;
}

const esc=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
let nextList=0;
type Control=HTMLInputElement|HTMLTextAreaElement;

/** Binds helper buttons to every marked input now and in later renders and dialogs. */
export function installHelperMenus(host:HelperHost,root:HTMLElement=document.body){
  const storage=host.storage??(()=>{try{return window.localStorage;}catch{return undefined;}})();
  const fields=new FieldList(storage),history=new FormatHistory(storage);
  let open:{menu:HTMLElement;control:Control;button:HTMLButtonElement}|undefined;
  const known=(control:Control)=>[...fields.list(),...host.fieldNames(),...(control.dataset.specials?.split(',').filter(Boolean)||[])];
  const close=(refocus=false)=>{if(!open)return;const {menu,button}=open;open=undefined;menu.remove();button.setAttribute('aria-expanded','false');document.removeEventListener('pointerdown',outside,true);if(refocus)button.focus();};
  const outside=(event:Event)=>{if(open&&!open.menu.contains(event.target as Node)&&event.target!==open.button)close();};
  const listFor=(control:Control)=>{
    if(!(control instanceof HTMLInputElement))return;
    const kind=control.dataset.helper as HelperKind,owned=control.getAttribute('list')?.startsWith('helper-list-');
    if((control.hasAttribute('list')&&!owned)||(kind!=='field'&&!control.dataset.history))return;
    const id=control.getAttribute('list')||`helper-list-${++nextList}`;
    let list=document.getElementById(id) as HTMLDataListElement|null;
    if(!list){list=document.createElement('datalist');list.id=id;control.after(list);control.setAttribute('list',id);}
    const options:HistoryEntry[]=kind==='field'?[...new Set(known(control).map(upper))].filter(validField).sort().map(value=>({value})):control.dataset.history?history.list(control.dataset.history):[];
    list.innerHTML=options.map(entry=>`<option value="${esc(entry.value)}"${entry.name?` label="${esc(entry.name)}"`:''} translate="no"></option>`).join('');
  };
  const insert=(control:Control,text:string)=>{
    const start=control.selectionStart??control.value.length,end=control.selectionEnd??start,next=insertion(control.value,start,end,text);
    control.value=next.value;control.focus();try{control.setSelectionRange(next.start,next.end);}catch{/* Some input types do not expose a selection. */}
    control.dispatchEvent(new Event('input',{bubbles:true}));
  };
  const setValue=(control:Control,value:string)=>{control.value=value;control.focus();control.dispatchEvent(new Event('input',{bubbles:true}));control.dispatchEvent(new Event('change',{bubbles:true}));};
  const remember=(control:Control)=>{const key=control.dataset.history;if(key&&control.value.trim()){history.add(key,control.value);listFor(control);}};
  const command=async(control:Control,name:string)=>{
    const key=control.dataset.history||'';
    if(name.startsWith('set:'))setValue(control,name.slice(4));
    if(name==='field-remove'){fields.remove(control.value);listFor(control);}
    if(name==='field-clear'&&await host.confirm('Are you sure that you want to remove all fields from the list?')){fields.clear();listFor(control);}
    if(name==='field-reset'&&await host.confirm('Are you sure that you want to reset the fields list to its default?')){fields.reset();listFor(control);}
    if(name==='history-remove'){history.remove(key,control.value);listFor(control);}
    if(name==='history-clear'&&await host.confirm('Are you sure that you want to remove all items?')){history.clear(key);listFor(control);}
    if(name==='history-manage')manageHistory(control,key);
    if(name==='filter-auto'&&host.filterAutoApply)host.filterAutoApply.set(!host.filterAutoApply.get());
  };
  const manageHistory=(control:Control,key:string)=>{
    const dialog=document.createElement('dialog');dialog.className='manage-history-dialog';dialog.setAttribute('aria-label','Manage History');
    dialog.innerHTML='<header>Manage History</header><div class="dialog-content"></div><div class="dialog-footer"><button data-close>Close</button><button class="primary" data-use disabled>Use selected</button></div>';
    document.body.append(dialog);dialog.addEventListener('close',()=>{dialog.remove();listFor(control);control.focus();});
    let entries=history.list(key),current=entries.findIndex(entry=>entry.value===control.value);
    const content=dialog.querySelector<HTMLElement>('.dialog-content')!,use=dialog.querySelector<HTMLButtonElement>('[data-use]')!;
    const save=()=>{history.set(key,entries);};
    /** `copyAt` places a duplicate there; it must differ from the entries it copies. */
    const edit=(index:number,draft?:HistoryEntry,copyAt?:number)=>{
      const entry=draft||entries[index]||{value:index<0?control.value:''},form=document.createElement('div');form.className='history-form';
      form.innerHTML=`${copyAt!==undefined?'<p>Change the copy before keeping it: each format string is listed once.</p>':''}<label class="field"><span>NAME (OPTIONAL)</span><input data-name value="${esc(entry.name)}"></label><label class="field"><span>FORMAT STRING</span><input data-value value="${esc(entry.value)}" spellcheck="false" autocomplete="off"></label><label class="field"><span>DESCRIPTION (OPTIONAL)</span><textarea data-description rows="2">${esc(entry.description)}</textarea></label><label class="action-check"><input type="checkbox" data-favorite-edit ${entry.favorite?'checked':''}> Favorite (offered in the ► menu)</label><div class="group-buttons"><button data-cancel>Cancel</button><button class="primary" data-keep>${index<0?'Add':'Keep'}</button></div>`;
      content.replaceChildren(form);form.querySelector<HTMLInputElement>('[data-value]')!.focus();
      form.querySelector('[data-cancel]')!.addEventListener('click',draw);
      form.querySelector('[data-keep]')!.addEventListener('click',()=>{
        const value=form.querySelector<HTMLInputElement>('[data-value]')!.value,name=form.querySelector<HTMLInputElement>('[data-name]')!.value.trim(),description=form.querySelector<HTMLTextAreaElement>('[data-description]')!.value.trim();
        if(!value.trim()){form.querySelector('.dialog-error')?.remove();form.insertAdjacentHTML('beforeend','<p class="dialog-error" role="alert">Enter a format string.</p>');return;}
        if(name&&entries.some((other,i)=>i!==index&&other.name===name)){form.querySelector('.dialog-error')?.remove();form.insertAdjacentHTML('beforeend','<p class="dialog-error" role="alert">An entry with this description already exists. Please use another description.</p>');return;}
        const favorite=form.querySelector<HTMLInputElement>('[data-favorite-edit]')!.checked;
        const next:HistoryEntry={value,...(name?{name}:{}),...(description?{description}:{}),...(favorite?{favorite}:{})};
        if(copyAt!==undefined){
          if(entries.some(other=>other.value===value)){form.querySelector('.dialog-error')?.remove();form.insertAdjacentHTML('beforeend','<p class="dialog-error" role="alert">This format string is already listed. Change the copy before keeping it.</p>');return;}
          entries.splice(copyAt,0,next);current=copyAt;
        }
        else if(index<0){entries=[next,...entries.filter(other=>other.value!==value)];current=0;}else{entries[index]=next;entries=entries.filter((other,i)=>i===index||other.value!==value);current=entries.indexOf(next);}
        save();draw();
      });
    };
    const draw=()=>{
      content.innerHTML=`<p>Saved format strings appear in the drop-down list of this field. Give an entry a name to find it again. Ticked entries are favorites, offered in the ► menu.</p><ul class="history-list" role="listbox" aria-label="History">${entries.map((entry,i)=>`<li role="option" tabindex="0" data-index="${i}" aria-selected="${i===current}" class="${i===current?'current':''}"><input type="checkbox" data-favorite="${i}" aria-label="Favorite: ${esc(entry.name||entry.value)}" title="Favorite" ${entry.favorite?'checked':''}><span class="history-text" translate="no"><strong>${esc(entry.name||entry.value)}</strong>${entry.name?`<code>${esc(entry.value)}</code>`:''}${entry.description?`<small>${esc(entry.description)}</small>`:''}</span></li>`).join('')||'<li class="group-empty">No saved format strings.</li>'}</ul><div class="group-buttons"><button data-add>Add…</button><button data-duplicate ${current>=0?'':'disabled'}>Duplicate…</button><button data-edit ${current>=0?'':'disabled'}>Edit…</button><button data-remove ${current>=0?'':'disabled'}>Remove</button><button data-up ${current>0?'':'disabled'}>↑ Move up</button><button data-down ${current>=0&&current<entries.length-1?'':'disabled'}>↓ Move down</button></div>`;
      use.disabled=current<0;
      content.querySelectorAll<HTMLElement>('[data-index]').forEach(item=>{const index=Number(item.dataset.index);item.addEventListener('click',()=>{current=index;draw();content.querySelector<HTMLElement>(`[data-index="${index}"]`)?.focus();});item.addEventListener('dblclick',()=>{current=index;use.click();});item.addEventListener('keydown',event=>{if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();current=Math.max(0,Math.min(entries.length-1,index+(event.key==='ArrowDown'?1:-1)));draw();content.querySelector<HTMLElement>(`[data-index="${current}"]`)?.focus();}if(event.key==='Enter'){event.preventDefault();use.click();}if(event.key==='F2'){event.preventDefault();edit(index);}});});
      content.querySelectorAll<HTMLInputElement>('[data-favorite]').forEach(box=>{
        box.addEventListener('click',event=>event.stopPropagation());
        box.addEventListener('change',()=>{const index=Number(box.dataset.favorite),entry=entries[index];if(!entry)return;if(box.checked)entry.favorite=true;else delete entry.favorite;save();});
      });
      content.querySelector('[data-add]')!.addEventListener('click',()=>edit(-1));
      content.querySelector('[data-duplicate]')!.addEventListener('click',()=>edit(-1,duplicateEntry(entries,current),current+1));
      content.querySelector('[data-edit]')!.addEventListener('click',()=>edit(current));
      content.querySelector('[data-remove]')!.addEventListener('click',()=>{entries.splice(current,1);current=Math.min(current,entries.length-1);save();draw();});
      const move=(by:number)=>{const target=current+by,[entry]=entries.splice(current,1);entries.splice(target,0,entry);current=target;save();draw();};
      content.querySelector('[data-up]')!.addEventListener('click',()=>move(-1));content.querySelector('[data-down]')!.addEventListener('click',()=>move(1));
    };
    dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());
    use.addEventListener('click',()=>{const entry=entries[current];if(!entry)return;dialog.close();setValue(control,entry.value);});
    draw();dialog.showModal();
  };
  const show=(control:Control,button:HTMLButtonElement)=>{
    if(open?.button===button){close(true);return;}
    close();const kind=control.dataset.helper as HelperKind,key=control.dataset.history;
    const entries=helperEntries({kind,panel:host.panelFields().length?host.panelFields():defaultPanelFields,known:kind==='field'?known(control):[...fields.list(),...host.fieldNames()],history:!!key,manage:control.hasAttribute('data-history-manage'),historySelected:!!key&&history.list(key).some(entry=>entry.value===control.value),favorites:key?history.favorites(key):[],value:control.value,inFieldList:fields.has(control.value),autoApply:kind==='filter'?host.filterAutoApply?.get():undefined});
    const menu=document.createElement('div');menu.className='helper-menu';menu.setAttribute('role','menu');menu.setAttribute('aria-label',kind==='field'?'Fields':'Format string helper');
    const build=(items:HelperEntry[],parent:HTMLElement)=>items.forEach(item=>{
      if(item.separator){parent.append(Object.assign(document.createElement('hr'),{role:'separator'}));return;}
      if(item.heading){const heading=document.createElement('div');heading.className='helper-heading';heading.setAttribute('role','presentation');heading.textContent=item.label;parent.append(heading);return;}
      const element=document.createElement('button');element.type='button';element.textContent=item.label;element.disabled=!!item.disabled;if(item.command?.startsWith('set:'))element.setAttribute('translate','no');
      element.setAttribute('role',item.checked===undefined?'menuitem':'menuitemcheckbox');if(item.checked!==undefined)element.setAttribute('aria-checked',String(item.checked));
      if(item.submenu){
        const group=document.createElement('div');group.className='helper-group';element.className='helper-parent';element.setAttribute('aria-haspopup','menu');element.setAttribute('aria-expanded','false');
        const sub=document.createElement('div');sub.className='helper-submenu';sub.setAttribute('role','menu');sub.setAttribute('aria-label',item.label);sub.hidden=true;build(item.submenu,sub);
        const toggle=(value:boolean)=>{menu.querySelectorAll<HTMLElement>(':scope .helper-group').forEach(other=>{if(other!==group&&!other.contains(group)&&value){const s=other.querySelector<HTMLElement>(':scope>.helper-submenu')!;s.hidden=true;other.querySelector(':scope>button')!.setAttribute('aria-expanded','false');}});sub.hidden=!value;element.setAttribute('aria-expanded',String(value));if(value&&!menu.classList.contains('helper-sheet')){host.translateNow?.(sub);sub.classList.remove('open-left');sub.style.top='';const box=sub.getBoundingClientRect(),top=group.getBoundingClientRect().top;if(box.right>window.innerWidth)sub.classList.add('open-left');if(box.bottom>window.innerHeight-4)sub.style.top=`${Math.max(4-top,window.innerHeight-4-box.bottom)}px`;}};
        element.addEventListener('click',()=>toggle(sub.hidden));
        element.addEventListener('keydown',event=>{if(event.key==='ArrowRight'){event.preventDefault();toggle(true);sub.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();}});
        sub.addEventListener('keydown',event=>{if(event.key==='ArrowLeft'){event.preventDefault();event.stopPropagation();toggle(false);element.focus();}});
        group.addEventListener('pointerenter',()=>{if(window.matchMedia('(hover:hover)').matches&&window.innerWidth>700)toggle(true);});
        group.append(element,sub);parent.append(group);return;
      }
      element.addEventListener('click',()=>{close();if(item.insert!==undefined)insert(control,item.insert);else if(item.command)void command(control,item.command);});
      parent.append(element);
    });
    build(entries,menu);
    (control.closest('dialog')||document.body).append(menu);host.translateNow?.(menu);
    const rect=button.getBoundingClientRect(),narrow=window.innerWidth<=700;
    if(narrow){menu.classList.add('helper-sheet');}
    else{menu.style.top=`${Math.min(rect.bottom+2,window.innerHeight-Math.min(menu.offsetHeight,window.innerHeight-8)-4)}px`;menu.style.left=`${Math.max(4,Math.min(rect.right-menu.offsetWidth,window.innerWidth-menu.offsetWidth-4))}px`;}
    button.setAttribute('aria-expanded','true');open={menu,control,button};document.addEventListener('pointerdown',outside,true);
    menu.addEventListener('keydown',event=>{
      const items=[...menu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')].filter(item=>!item.closest('.helper-submenu[hidden]'));
      const index=items.indexOf(document.activeElement as HTMLButtonElement);
      if(event.key==='Escape'){event.preventDefault();event.stopPropagation();close(true);}
      if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();items[(index+(event.key==='ArrowDown'?1:-1)+items.length)%items.length]?.focus();}
      if(event.key==='Tab')close();
    });
    menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
  };
  const bind=(control:Control)=>{
    control.dataset.helperBound='true';
    const kind=control.dataset.helper as HelperKind,button=document.createElement('button');
    button.type='button';button.className='helper-button';button.textContent='►';
    const label=kind==='field'?'Fields':'Format string helper';button.setAttribute('aria-label',label);button.title=label;button.setAttribute('aria-haspopup','menu');button.setAttribute('aria-expanded','false');
    button.disabled=control.disabled;
    button.addEventListener('click',()=>show(control,button));
    // Never move the control itself: moving a focused input would drop focus.
    const parent=control.parentElement;
    if(parent&&!parent.classList.contains('action-input')&&!parent.classList.contains('filterbar'))parent.classList.add('helper-host');
    control.after(button);
    listFor(control);
    control.addEventListener('focus',()=>listFor(control));
    if(kind==='field')control.addEventListener('change',()=>{if(fields.add(control.value))listFor(control);});
    if(control.dataset.history){
      const dialog=control.closest('dialog');
      if(dialog&&!dialog.dataset.historyBound){dialog.dataset.historyBound='true';dialog.addEventListener('click',event=>{const target=(event.target as Element).closest('button');if(target?.classList.contains('primary')&&!target.closest('.manage-history-dialog'))dialog.querySelectorAll<Control>('[data-history]').forEach(remember);},true);}
      if(!dialog||control.hasAttribute('data-history-enter'))control.addEventListener('keydown',event=>{if((event as KeyboardEvent).key==='Enter')remember(control);});
    }
  };
  const scan=(node:ParentNode)=>node.querySelectorAll<Control>('[data-helper]:not([data-helper-bound])').forEach(bind);
  root.addEventListener('keydown',event=>{
    const control=event.target,key=helperKey(event);
    if(!key||!(control instanceof HTMLInputElement||control instanceof HTMLTextAreaElement)||!control.dataset.helperBound)return;
    if(key==='open'){
      let button=control.nextElementSibling;while(button&&!button.classList.contains('helper-button'))button=button.nextElementSibling;
      if(!(button instanceof HTMLButtonElement)||button.disabled)return;
      event.preventDefault();event.stopPropagation();if(open?.control!==control)show(control,button);return;
    }
    const historyKey=control.dataset.history;
    if(historyKey&&history.list(historyKey).some(entry=>entry.value===control.value)){event.preventDefault();history.remove(historyKey,control.value);listFor(control);}
  });
  scan(root);
  new MutationObserver(records=>{if(records.some(record=>record.addedNodes.length))scan(root);}).observe(root,{childList:true,subtree:true});
  window.addEventListener('resize',()=>close());
  return{fields,history,close};
}
