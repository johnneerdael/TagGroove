// Main toolbar (docs.mp3tag.de/getting-started/main-window/, menus/view): View →
// Toolbars shows or hides it and opens Customize toolbar, where items are shown,
// hidden, renamed and reordered with the keys of Mp3tag's customization dialog
// (docs.mp3tag.de/keyboard/ → Toolbar Customization). Hovering or focusing a
// button shows its description in the status bar.
// Standalone (no imports), so the rules are unit-tested directly.

export type ToolbarItem={id:string;visible:boolean;label?:string};
export type ToolbarConfig={shown:boolean;items:ToolbarItem[]};
export type ToolbarCommand={label:string;icon?:string;hint:string};
export const SEPARATOR='separator';

/** Commands that can sit on the toolbar, with the status-bar hint shown on hover. */
export const TOOLBAR_COMMANDS:Record<string,ToolbarCommand>={
  folder:{label:'Open folder',icon:'▱',hint:'Change directory: load the audio files of a folder (⌘/Ctrl D)'},
  files:{label:'Add files',icon:'＋',hint:'Add audio files to the file list'},
  save:{label:'Save tags',icon:'▣',hint:'Save the pending tag changes of the selected files (⌘/Ctrl S)'},
  reload:{label:'Reload',icon:'↻',hint:'Read the tags of the selected files again (⌘/Ctrl T)'},
  'remove-tags':{label:'Remove tag',hint:'Remove the tags of the selected files (⌘/Ctrl R)'},
  undo:{label:'Undo',hint:'Undo the last change written to files (⌘/Ctrl Z)'},
  redo:{label:'Redo',hint:'Redo the last undone change, or repeat the last action (⌘/Ctrl Y)'},
  'filename-tags':{label:'Filename → Tag',hint:'Converter: extract tag values from the file names (Alt 2)'},
  'tags-filename':{label:'Tag → Filename',hint:'Converter: rename the files from their tags (Alt 1)'},
  'filename-filename':{label:'Filename → Filename',hint:'Converter: rearrange parts of the file names (Alt 3)'},
  'text-tags':{label:'Text File → Tag',hint:'Converter: import tags from a text file (Alt 4)'},
  'tag-tag':{label:'Tag → Tag',hint:'Converter: set a field from a format string (Alt 5)'},
  groups:{label:'Actions',icon:'⚙',hint:'Apply, edit and organize action groups (Alt 6)'},
  action:{label:'Quick action',hint:'Apply one action to the selected files (Alt Shift 6)'},
  number:{label:'Numbering',icon:'#',hint:'Auto-numbering wizard: number the selected files (⌘/Ctrl K)'},
  extended:{label:'Extended tags',hint:'View and edit every field and the cover art (Alt T)'},
  'playlist-all':{label:'Playlist',hint:'Create a playlist of all listed files (⌘/Ctrl P)'},
  'export-config':{label:'Export',hint:'Export the selected files with an export configuration (⌘/Ctrl E)'},
  find:{label:'Find',hint:'Find files in the file list (⌘/Ctrl F)'},
  refresh:{label:'Refresh',hint:'Read the listed folders again (F5)'},
  'source-musicbrainz':{label:'MusicBrainz',hint:'Tag sources: look up the selected files on MusicBrainz'},
  'source-discogs':{label:'Discogs',hint:'Tag sources: look up the selected files on Discogs'},
  'cover-manage':{label:'Covers',hint:'Manage the cover art of the selected files'},
  'file-reveal':{label:'Show in folder',hint:'Show the selected files in the file manager (Alt Enter)'},
  'file-play':{label:'Play',hint:'Play the selected files (F4)'},
  'toggle-panel':{label:'Tag panel',hint:'Show or hide the tag panel (Ctrl Q)'},
  'toggle-filter':{label:'Filter',hint:'Show or hide the filter (F3)'},
  options:{label:'Options',hint:'Open the options (⌘/Ctrl O)'},
};
const shown=(id:string)=>({id,visible:true}),hidden=(id:string)=>({id,visible:false});
let separators=0;const separator=(visible=true)=>({id:`${SEPARATOR}-${++separators}`,visible});
export function defaultToolbar():ToolbarConfig{
  separators=0;
  return{shown:true,items:[shown('folder'),shown('files'),separator(),shown('save'),shown('reload'),separator(),shown('filename-tags'),shown('tags-filename'),separator(),shown('groups'),shown('number'),
    hidden('undo'),hidden('redo'),hidden('remove-tags'),hidden('extended'),hidden('filename-filename'),hidden('text-tags'),hidden('tag-tag'),hidden('action'),hidden('playlist-all'),hidden('export-config'),hidden('find'),hidden('refresh'),
    hidden('source-musicbrainz'),hidden('source-discogs'),hidden('cover-manage'),hidden('file-reveal'),hidden('file-play'),hidden('toggle-panel'),hidden('toggle-filter'),hidden('options')]};
}
export const isSeparator=(id:string)=>id.startsWith(SEPARATOR);
/** Saved toolbar settings: unknown commands are dropped, labels trimmed, and commands added in later versions appended hidden. */
export function normalizeToolbar(value:unknown):ToolbarConfig{
  const fallback=defaultToolbar();
  const saved=value&&typeof value==='object'?value as Partial<ToolbarConfig>:{};
  if(!Array.isArray(saved.items))return{...fallback,shown:saved.shown!==false};
  const seen=new Set<string>(),items:ToolbarItem[]=[];
  for(const item of saved.items){
    if(!item||typeof item.id!=='string'||seen.has(item.id)||!(isSeparator(item.id)||item.id in TOOLBAR_COMMANDS))continue;
    seen.add(item.id);
    const label=typeof item.label==='string'&&item.label.trim()&&!isSeparator(item.id)?item.label.trim().slice(0,60):undefined;
    items.push({id:item.id,visible:item.visible!==false,...(label?{label}:{})});
  }
  for(const item of fallback.items)if(!isSeparator(item.id)&&!seen.has(item.id))items.push({...item,visible:false});
  return{shown:saved.shown!==false,items};
}
export const itemLabel=(item:ToolbarItem)=>isSeparator(item.id)?'Separator':item.label||TOOLBAR_COMMANDS[item.id]?.label||item.id;

export type ToolbarKey='edit'|'toggle'|'show-all'|'hide-all'|'up'|'down'|'top'|'bottom';
/** Toolbar Customization keys: F4 edits, Space shows or hides, Ctrl+E / Ctrl+D (⌘ on macOS) show or hide all, Alt+↑/↓/Page Up/Page Down move. */
export function toolbarKey(event:{key:string;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean;altKey:boolean},mac:boolean):ToolbarKey|undefined{
  const primary=mac?event.metaKey:event.ctrlKey,other=mac?event.ctrlKey:event.metaKey,key=event.key.length===1?event.key.toLowerCase():event.key;
  if(event.shiftKey||other)return undefined;
  if(event.altKey)return primary?undefined:({ArrowUp:'up',ArrowDown:'down',PageUp:'top',PageDown:'bottom'} as Record<string,ToolbarKey>)[key];
  if(primary)return key==='e'?'show-all':key==='d'?'hide-all':undefined;
  return key==='F4'?'edit':key===' '?'toggle':undefined;
}
/** Move one item; returns the new list and the item's new index. */
export function moveToolbarItem(items:ToolbarItem[],index:number,move:'up'|'down'|'top'|'bottom'):{items:ToolbarItem[];index:number}{
  const target=move==='top'?0:move==='bottom'?items.length-1:index+(move==='up'?-1:1);
  if(index<0||index>=items.length||target<0||target>=items.length||target===index)return{items,index};
  const next=[...items],[item]=next.splice(index,1);next.splice(target,0,item);return{items:next,index:target};
}
/** Visible items as rendered: separators at the start, the end or next to another separator are left out. */
export function visibleItems(items:ToolbarItem[]):ToolbarItem[]{
  const result:ToolbarItem[]=[];
  for(const item of items.filter(item=>item.visible)){if(isSeparator(item.id)&&(!result.length||isSeparator(result[result.length-1].id)))continue;result.push(item);}
  while(result.length&&isSeparator(result[result.length-1].id))result.pop();
  return result;
}

const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export type ToolbarHost={
  config:()=>ToolbarConfig;
  save:(config:ToolbarConfig)=>Promise<void>;
  mac:()=>boolean;
};
/** View → Toolbars → Customize toolbar. */
export function customizeToolbar(host:ToolbarHost){
  if(document.querySelector('.toolbar-dialog'))return;
  let items=structuredClone(host.config().items),current=0,editing=false;
  const dialog=document.createElement('dialog');dialog.className='toolbar-dialog';dialog.setAttribute('aria-label','Customize toolbar');document.body.append(dialog);
  dialog.innerHTML=`<header>Customize toolbar</header><div class="dialog-content"><p>Checked items are shown on the main toolbar, in this order. Space shows or hides the selected item, F4 renames it, ⌘/Ctrl E shows all and ⌘/Ctrl D hides all, Alt ↑ ↓ and Alt Page Up / Page Down move it.</p>
    <div class="toolbar-config"><div class="group-list toolbar-items" role="listbox" aria-label="Toolbar items" tabindex="0"></div>
    <div class="group-buttons"><button type="button" data-tb="edit">Rename…</button><button type="button" data-tb="separator">Add separator</button><button type="button" data-tb="up">↑ Move up</button><button type="button" data-tb="down">↓ Move down</button><button type="button" data-tb="show-all">Show all</button><button type="button" data-tb="hide-all">Hide all</button><button type="button" data-tb="reset">Reset</button></div></div></div>
    <div class="dialog-footer"><button type="button" data-close>Cancel</button><button type="button" class="primary" id="toolbar-ok">OK</button></div>`;
  const list=dialog.querySelector<HTMLElement>('.toolbar-items')!;
  const render=()=>{
    current=Math.max(0,Math.min(current,items.length-1));
    list.innerHTML=items.map((item,index)=>`<div class="group-row ${index===current?'current':''}" role="option" id="toolbar-item-${index}" data-item="${index}" aria-selected="${index===current}"><input type="checkbox" tabindex="-1" data-show="${index}" aria-label="Show ${html(itemLabel(item))}" ${item.visible?'checked':''}><span>${html(itemLabel(item))}</span><small>${isSeparator(item.id)?'':html(TOOLBAR_COMMANDS[item.id]?.hint||'')}</small></div>`).join('');
    list.setAttribute('aria-activedescendant',`toolbar-item-${current}`);
    list.querySelector('.current')?.scrollIntoView({block:'nearest'});
    list.querySelectorAll<HTMLInputElement>('[data-show]').forEach(box=>box.addEventListener('change',()=>{items[Number(box.dataset.show)].visible=box.checked;}));
  };
  const edit=()=>{
    const item=items[current];if(!item||isSeparator(item.id)||editing)return;
    const row=list.querySelector<HTMLElement>(`[data-item="${current}"] span`);if(!row)return;
    editing=true;const input=document.createElement('input');input.className='toolbar-rename';input.value=itemLabel(item);input.setAttribute('aria-label','Toolbar item name');row.replaceWith(input);input.focus();input.select();
    const finish=(keep:boolean)=>{if(!editing)return;editing=false;if(keep){const label=input.value.trim();item.label=label&&label!==TOOLBAR_COMMANDS[item.id]?.label?label.slice(0,60):undefined;}render();list.focus();};
    input.addEventListener('keydown',event=>{event.stopPropagation();if(event.key==='Enter'){event.preventDefault();finish(true);}if(event.key==='Escape'){event.preventDefault();finish(false);}});
    input.addEventListener('blur',()=>finish(true));
  };
  const run=(command:string)=>{
    if(command==='edit'){edit();return;}
    if(command==='toggle'&&items[current])items[current].visible=!items[current].visible;
    if(command==='show-all'||command==='hide-all')items.forEach(item=>item.visible=command==='show-all');
    if(command==='up'||command==='down'||command==='top'||command==='bottom'){const moved=moveToolbarItem(items,current,command);items=moved.items;current=moved.index;}
    if(command==='separator'){let n=1;while(items.some(item=>item.id===`${SEPARATOR}-${n}`))n++;items.splice(current+1,0,{id:`${SEPARATOR}-${n}`,visible:true});current++;}
    if(command==='reset'){items=defaultToolbar().items;current=0;}
    render();
  };
  list.addEventListener('click',event=>{const row=(event.target as Element).closest<HTMLElement>('[data-item]');if(!row||(event.target as Element).matches('input'))return;current=Number(row.dataset.item);render();list.focus();});
  list.addEventListener('dblclick',event=>{if((event.target as Element).closest('[data-item]'))edit();});
  list.addEventListener('keydown',event=>{
    if(editing)return;
    if(event.key==='ArrowUp'||event.key==='ArrowDown'||event.key==='Home'||event.key==='End'){
      if(event.altKey&&(event.key==='ArrowUp'||event.key==='ArrowDown')){/* handled below */}
      else{event.preventDefault();current=event.key==='Home'?0:event.key==='End'?items.length-1:current+(event.key==='ArrowDown'?1:-1);render();return;}
    }
    const command=toolbarKey(event,host.mac());if(!command)return;
    event.preventDefault();event.stopPropagation();run(command);
  });
  dialog.querySelectorAll<HTMLButtonElement>('[data-tb]').forEach(button=>button.addEventListener('click',()=>{run(button.dataset.tb!);if(button.dataset.tb!=='edit')list.focus();}));
  dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());
  dialog.querySelector('#toolbar-ok')!.addEventListener('click',async()=>{
    const ok=dialog.querySelector<HTMLButtonElement>('#toolbar-ok')!;ok.disabled=true;
    try{await host.save({...host.config(),items});dialog.close();}
    catch(error){let p=dialog.querySelector('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(p);}p.textContent=String(error);ok.disabled=false;}
  });
  dialog.addEventListener('close',()=>dialog.remove());
  render();dialog.showModal();list.focus();
}
