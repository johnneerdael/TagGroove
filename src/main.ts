/// <reference types="vite/client" />
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { open, save, ask } from '@tauri-apps/plugin-dialog';
import { resolveTagEdits, type TagRowEdit } from './tag-edits';
import { ActionUI, panelKey, resolvePathChanges, type ActionGroup, type ActionLibrary, type ImportedGroup } from './action-ui';
import logoUrl from '../assets/brand/OpenTag-Brand-Kit/01-logos/horizontal/opentag-logo-light.svg';
import markUrl from '../assets/brand/OpenTag-Brand-Kit/01-logos/marks/opentag-mark-color.svg';
import artworkUrl from '../assets/brand/OpenTag-Brand-Kit/04-in-app/opentag-default-artwork-light-1024.svg';
import aboutUrl from '../assets/brand/OpenTag-Brand-Kit/04-in-app/opentag-splash-light-1600x1000.svg';
import glyphAlbum from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-album.svg?raw';
import glyphBatch from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-batch-edit.svg?raw';
import glyphCheck from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-check.svg?raw';
import glyphEdit from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-edit-tags.svg?raw';
import glyphExport from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-export.svg?raw';
import glyphImport from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-import.svg?raw';
import glyphInfo from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-info.svg?raw';
import glyphMusic from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-music-file.svg?raw';
import glyphOrganize from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-organize.svg?raw';
import glyphSave from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-save.svg?raw';
import glyphSearch from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-search.svg?raw';
import glyphSettings from '../assets/brand/OpenTag-Brand-Kit/04-in-app/ui-icons/opentag-settings.svg?raw';
import './style.css';

type Fields = Record<string, string[]>;
type Cover = { index: number; mime: string; description: string; picture_type: string; data_base64: string };
type Entry = { path: string; filename: string; format: string; duration: number; bitrate: number; sample_rate: number; channels: number; size: number; fields: Fields; covers: Cover[] };
type FileError = { path: string; error: string };
type Batch = { entries: Entry[]; errors: FileError[]; renamed?: { from: string; to: string }[]; moved_directories?: {from:string;to:string}[] };
type Preview = { path: string; value: string };
type RenamePreview = Preview & {plan_token:string};
type FilenamePreview = { path: string; fields: Fields };
type Operation = 'folder'|'files'|'save'|'reload'|'extended'|'add-cover'|'remove-cover'|'filename-tags'|'tags-filename'|'action'|'groups'|'number'|'remove-tags'|'playlist'|'csv'|'select-all'|'discard'|'about';
const glyphs:Partial<Record<Operation,string>>={folder:glyphImport,files:glyphMusic,save:glyphSave,reload:glyphOrganize,extended:glyphEdit,'add-cover':glyphAlbum,'remove-cover':glyphEdit,'filename-tags':glyphImport,'tags-filename':glyphExport,action:glyphBatch,groups:glyphSettings,number:glyphOrganize,'remove-tags':glyphEdit,playlist:glyphOrganize,csv:glyphExport,'select-all':glyphCheck,discard:glyphEdit,about:glyphInfo};
const glyph=(source:string)=>`<span class="icon" aria-hidden="true">${source}</span>`;
const desktop = isTauri();
let appVersion='development';
const app = document.querySelector<HTMLDivElement>('#app')!;
const entries = new Map<string, Entry>();
const drafts = new Map<string, Fields>();
const selected = new Set<string>();
const columns = [{ key:'filename',label:'Filename',width:220 },{ key:'TITLE',label:'Title',width:175 },{ key:'ARTIST',label:'Artist',width:145 },{ key:'ALBUM',label:'Album',width:165 },{ key:'TRACK',label:'Track',width:55 },{ key:'YEAR',label:'Year',width:55 },{ key:'GENRE',label:'Genre',width:90 },{ key:'duration',label:'Length',width:60 },{ key:'format',label:'Format',width:65 },{ key:'bitrate',label:'Bitrate',width:65 }];
let filter = '', sortKey = 'filename', sortAscending = true, anchor: string | undefined, busy = false, status = 'Ready', errors: FileError[] = [], location = 'No folder loaded', coverIndex = 0;
const esc = (value: unknown) => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const basename = (path: string) => path.split(/[\\/]/).pop() || path;
const paths = () => ordered().filter(entry=>selected.has(entry.path)).map(entry=>entry.path);
const merged = (entry: Entry): Fields => ({ ...entry.fields, ...drafts.get(entry.path) });
const field = (entry: Entry, key: string) => (merged(entry)[panelKey({format:entry.format,fields:merged(entry)},key)] || []).join(' \\\\ ');
const length = (seconds: number) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2,'0')}`;
const dirtySelected = () => [...selected].some(path => drafts.has(path));
const active = () => entries.get(selected.values().next().value||'');
const collator = new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
const dev = (import.meta as ImportMeta & {env?: {DEV?: boolean}}).env?.DEV === true;
const compactScreen=()=>window.matchMedia('(max-width:700px)').matches;
let rowHeight = compactScreen()?44:30;
const headerHeight = 29, overscan = 8;
let mobilePane:'files'|'tags'='files';
let tableScrollTop=0,tableScrollLeft=0;
let runtimePlatform={family:'desktop' as 'desktop'|'android'|'ios',folder_picker_supported:true};
let dataRevision = 0, orderCacheKey = '', filterCacheKey = '', orderCache: Entry[] = [], filterCache: Entry[] = [];
let virtualStart = -1, virtualEnd = -1, virtualRevision = -1, virtualSelection = '';
const searchCache = new Map<string,string>();
let searchRevision = -1;
let filterMatch:{context:string;query:string;paths:Set<string>}|undefined;
let filterRequestKey='',filterPending=false,filterFailure='',filterTimer=0;
const actionUI=new ActionUI({
  load:()=>invoke<ActionLibrary>('load_action_library'),
  save:library=>invoke<void>('save_action_library',{library}),
  import:(path,legacyEncoding)=>invoke<ImportedGroup>('import_action_group',{path,legacyEncoding:legacyEncoding||null}),
  execute:executeGroups,
  pickFile:async kind=>{const extensions=kind==='mta'?['mta']:kind==='image'?['jpg','jpeg','png','webp','gif']:kind==='export'?['mte']:['txt','lrc'];const path=await open({multiple:false,title:kind==='mta'?'Import action group':kind==='image'?'Choose cover image':kind==='export'?'Choose export configuration':'Choose text file',filters:[{name:kind==='mta'?'Action groups':kind==='image'?'Images':kind==='export'?'Export configurations':'Text files',extensions}]});return typeof path==='string'?path:null;},
  confirm:message=>ask(message,{title:'Action groups',kind:'warning'}),
  canExecute:()=>enabled('action'),selectedCount:()=>selected.size,fieldNames:()=>[...new Set(paths().flatMap(path=>Object.keys(merged(entries.get(path)!))))],changed:()=>render(),
});
async function executeGroups(groups:ActionGroup[]){
  if(!enabled('action'))throw new Error('Select files and save pending tag edits before applying actions.');
  const targets=paths();
  const result=await run('Applying action groups…',async()=>{const batch=await invoke<Batch>('execute_action_groups',{paths:targets,groups,columns:columnSpecs(targets)});applyRenamed(batch);accept(batch,'Applied action groups');return batch;});
  if(!result)throw new Error(errors.map(error=>`${error.path}: ${error.error}`).join('\n')||'Action groups could not be applied.');
  if(result.errors.length)throw new Error(result.errors.map(error=>`${error.path}: ${error.error}`).join('\n'));
}
function columnSpecs(targets:string[]){
  const separator="' \\\\ '";
  const rawColumn=(key:string)=>key==='YEAR'?`$if2($meta_sep(YEAR,${separator}),$meta_sep(DATE,${separator}))`:key==='TRACK'?`$if($or($eql(%_filetype%,FLAC),$eql(%_filetype%,Vorbis),$eql(%_filetype%,Opus),$eql(%_filetype%,Speex),$eql(%_filetype%,Ogg)),$meta_sep(TRACKNUMBER,${separator}),$if2($meta_sep(TRACK,${separator}),$meta_sep(TRACKNUMBER,${separator})))`:`$meta_sep(${key},${separator})`;
  return(compactScreen()?columns.slice(0,3):columns).map(column=>({name:column.label,template:column.key==='filename'?'%_filename_ext%':column.key==='duration'?'$div(%_length_seconds%,60):$num($mod(%_length_seconds%,60),2)':column.key==='format'?'%_filetype%':column.key==='bitrate'?'%_bitrate% kbps':rawColumn(column.key),values:Object.fromEntries(targets.map(path=>{const entry=entries.get(path)!;const value=column.key==='filename'?entry.filename:column.key==='duration'?length(entry.duration):column.key==='format'?entry.format:column.key==='bitrate'?`${entry.bitrate} kbps`:field(entry,column.key);return[path,value];}))}));
}
function changedData(){dataRevision++;}
function timing(name:string,start:number,extra:Record<string,unknown>={}){
  if(dev)console.debug(`[OpenTag performance] ${name}`,{milliseconds:Math.round((performance.now()-start)*100)/100,files:entries.size,...extra});
}
function ordered(): Entry[] {
  const key=`${dataRevision}:${sortKey}:${sortAscending}`;
  if(orderCacheKey===key)return orderCache;
  const start=performance.now();
  const values=[...entries.values()].map(entry=>({entry,value:sortKey in entry?String(entry[sortKey as keyof Entry]):field(entry,sortKey)}));
  values.sort((a,b)=>collator.compare(a.value,b.value)*(sortAscending?1:-1));
  orderCache=values.map(item=>item.entry);orderCacheKey=key;
  timing('sort',start,{key:sortKey});return orderCache;
}
function filterFields(entry:Entry):Fields{
  const separator=entry.path.includes('\\')?'\\':'/',folder=entry.path.slice(0,Math.max(entry.path.lastIndexOf('/'),entry.path.lastIndexOf('\\')));
  const dot=entry.filename.lastIndexOf('.'),extension=dot>0?entry.filename.slice(dot+1):'',stem=dot>0?entry.filename.slice(0,dot):entry.filename;
  return {...merged(entry),_FILENAME:[stem],_FILENAME_EXT:[entry.filename],_PATH:[entry.path],_FOLDERPATH:[folder+separator],_DIRECTORY:[basename(folder)],_EXTENSION:[extension],_LENGTH_SECONDS:[String(entry.duration)],_BITRATE:[String(entry.bitrate)],_SAMPLERATE:[String(entry.sample_rate)],_FILE_SIZE:[String(entry.size)]};
}
function requestFilter(query:string){
  const context=filterContextKey(),key=`${context}:${query}`;
  if(filterRequestKey===key)return;
  filterRequestKey=key;filterPending=true;filterFailure='';window.clearTimeout(filterTimer);
  filterTimer=window.setTimeout(()=>{
    if(filterRequestKey!==key||filter!==query||filterContextKey()!==context)return;
    const started=performance.now(),items=ordered().map((entry,index)=>({path:entry.path,fields:{...filterFields(entry),_COUNTER:[String(index+1)]},raw_fields:merged(entry)}));
    void invoke<string[]>('filter_matches',{query,items,columns:columnSpecs(items.map(item=>item.path))}).then(matches=>{
      if(filterRequestKey!==key||filter!==query||filterContextKey()!==context)return;
      filterMatch={context,query,paths:new Set(matches)};filterPending=false;filterCacheKey='';filterFailure='';
      timing('filter expression',started,{query,matched:matches.length});render();
    }).catch(error=>{if(filterRequestKey===key&&filter===query&&filterContextKey()===context){filterPending=false;filterFailure=String(error);filterCacheKey='';render();}});
  },120);
}
function filterContextKey(){return`${dataRevision}:${sortKey}:${sortAscending}:${compactScreen()}`;}
function visible(): Entry[] {
  const query = filter.toLocaleLowerCase();
  const all=ordered(),key=`${orderCacheKey}:${query}`;
  if(desktop&&filter.trim()){
    if(filterMatch?.context===filterContextKey()&&filterMatch.query===filter){
      if(filterCacheKey===key)return filterCache;
      filterCache=all.filter(entry=>filterMatch!.paths.has(entry.path));filterCacheKey=key;return filterCache;
    }
    requestFilter(filter);return[];
  }
  if(!filter.trim()){filterPending=false;filterFailure='';filterRequestKey='';window.clearTimeout(filterTimer);}
  if(filterCacheKey===key)return filterCache;
  const start=performance.now();
  if(query&&searchRevision!==dataRevision){searchCache.clear();all.forEach(entry=>searchCache.set(entry.path,`${entry.filename} ${entry.path} ${Object.values(merged(entry)).flat().join(' ')}`.toLocaleLowerCase()));searchRevision=dataRevision;}
  filterCache=query?all.filter(entry=>searchCache.get(entry.path)!.includes(query)):all;filterCacheKey=key;
  timing('filter',start,{visible:filterCache.length});return filterCache;
}
type CachedCover = {epoch:number;covers:Cover[];bytes:number};
const coverEpochs=new Map<string,number>(),coverCache=new Map<string,CachedCover>(),coverRequests=new Map<string,Promise<void>>();
const coverCacheBytesLimit=12*1024*1024,coverCacheEntryLimit=8;
let coverCacheBytes=0,activeCover:{path:string;epoch:number;covers:Cover[]}|undefined;
let coverError:{path:string;message:string}|undefined;
function invalidateCovers(path:string){coverEpochs.set(path,(coverEpochs.get(path)||0)+1);const cached=coverCache.get(path);if(cached){coverCacheBytes-=cached.bytes;coverCache.delete(path);}if(activeCover?.path===path)activeCover=undefined;}
function cacheCovers(path:string,epoch:number,covers:Cover[]){
  const old=coverCache.get(path);if(old){coverCacheBytes-=old.bytes;coverCache.delete(path);}
  const bytes=covers.reduce((sum,cover)=>sum+cover.data_base64.length*2,0);
  if(bytes<=coverCacheBytesLimit){coverCache.set(path,{epoch,covers,bytes});coverCacheBytes+=bytes;}
  while(coverCache.size>coverCacheEntryLimit||coverCacheBytes>coverCacheBytesLimit){const oldest=coverCache.keys().next().value as string;coverCacheBytes-=coverCache.get(oldest)!.bytes;coverCache.delete(oldest);}
}
function storeEntry(entry:Entry){
  invalidateCovers(entry.path);const epoch=coverEpochs.get(entry.path)!;
  if(active()?.path===entry.path)coverIndex=Math.min(coverIndex,Math.max(0,entry.covers.length-1));
  if(entry.covers.length&&entry.covers.every(cover=>!!cover.data_base64)){cacheCovers(entry.path,epoch,entry.covers);if(active()?.path===entry.path)activeCover={path:entry.path,epoch,covers:entry.covers};}
  entries.set(entry.path,{...entry,covers:entry.covers.map(cover=>({...cover,data_base64:''}))});changedData();
}
function coversFor(entry:Entry|undefined):Cover[]{
  if(!entry)return[];const epoch=coverEpochs.get(entry.path)||0;
  if(activeCover?.path===entry.path&&activeCover.epoch===epoch)return activeCover.covers;
  const cached=coverCache.get(entry.path);if(cached?.epoch===epoch)return cached.covers;
  return entry.covers;
}
function coverMarkup(){
  const entry=active(),covers=coversFor(entry),cover=covers[Math.min(coverIndex,covers.length-1)];
  const loaded=!!cover?.data_base64,error=coverError&&coverError.path===entry?.path?coverError.message:'';
  return `<div class="cover-row"><div class="cover-art">${loaded?`<img alt="Embedded ${esc(cover.picture_type)} artwork">`:`<img class="fallback-artwork" src="${artworkUrl}" alt="" aria-hidden="true">`}</div><div class="cover-info"><strong>${cover?esc(cover.picture_type||'Embedded cover'):'No artwork'}</strong>${error?esc(error):cover?`${esc(cover.mime)}<br>${loaded?esc(cover.description||`${covers.length} embedded image${covers.length===1?'':'s'}`):'Loading artwork…'}`:selected.size>1?'First selected file':'Embedded cover art'}</div></div><div class="cover-controls">${button('add-cover','Add…')}${button('remove-cover','Remove')}${covers.length>1?`<select id="cover-choice" aria-label="Select cover">${covers.map((c,i)=>`<option value="${i}" ${i===coverIndex?'selected':''}>${i+1}: ${esc(c.picture_type)}</option>`).join('')}</select>`:''}</div>`;
}
function setCoverImage(){const covers=coversFor(active()),cover=covers[Math.min(coverIndex,covers.length-1)],image=document.querySelector<HTMLImageElement>('.cover-art img');if(image&&cover?.data_base64)image.src=`data:${cover.mime};base64,${cover.data_base64}`;}
function refreshCoverPanel(){
  const panel=document.querySelector('.cover-section');if(!panel)return;panel.innerHTML=coverMarkup();
  setCoverImage();
  panel.querySelectorAll<HTMLButtonElement>('[data-op]').forEach(el=>el.addEventListener('click',()=>void operate(el.dataset.op as Operation)));
  panel.querySelector<HTMLSelectElement>('#cover-choice')?.addEventListener('change',e=>{coverIndex=Number((e.target as HTMLSelectElement).value);refreshCoverPanel();});
}
function hydrateActiveCover(){
  const entry=active();if(!desktop||!entry?.covers.length)return;
  const path=entry.path,epoch=coverEpochs.get(path)||0,cached=coverCache.get(path);
  if(cached?.epoch===epoch){coverCache.delete(path);coverCache.set(path,cached);activeCover={path,epoch,covers:cached.covers};return;}
  if(activeCover?.path===path&&activeCover.epoch===epoch)return;
  activeCover=undefined;
  const requestKey=`${epoch}:${path}`;if(coverRequests.has(requestKey))return;
  const start=performance.now();
  const request=invoke<Cover[]>('get_covers',{path}).then(covers=>{
    if((coverEpochs.get(path)||0)!==epoch)return;
    cacheCovers(path,epoch,covers);
    if(active()?.path===path){activeCover={path,epoch,covers};coverError=undefined;refreshCoverPanel();}
    timing('cover hydrate',start,{path,bytes:covers.reduce((sum,c)=>sum+c.data_base64.length,0)});
  }).catch(error=>{if(active()?.path===path&&(coverEpochs.get(path)||0)===epoch){coverError={path,message:`Artwork could not be loaded: ${String(error)}`};refreshCoverPanel();}}).finally(()=>coverRequests.delete(requestKey));
  coverRequests.set(requestKey,request);
}
function rowMarkup(entry:Entry,index:number){
  return `<tr tabindex="0" data-path="${esc(entry.path)}" aria-rowindex="${index+2}" aria-selected="${selected.has(entry.path)}" class="${selected.has(entry.path)?'selected':''}">${columns.map(c=>`<td class="${c.key==='filename'?'filename ':''}${drafts.get(entry.path)?.[panelKey({format:entry.format,fields:merged(entry)},c.key)]?'pending':''}" title="${esc(c.key in entry?entry[c.key as keyof Entry]:field(entry,c.key))}">${c.key==='filename'?`<input class="row-select" type="checkbox" aria-label="Select ${esc(entry.filename)}" ${selected.has(entry.path)?'checked':''}>${drafts.has(entry.path)?'<span class="rowmarker" title="Pending changes">●</span>':''}${esc(entry.filename)}`:c.key==='duration'?length(entry.duration):c.key==='bitrate'?`${entry.bitrate} kbps`:c.key==='format'?esc(entry.format):esc(field(entry,c.key))}</td>`).join('')}</tr>`;
}
function updateVirtualRows(force=false){
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap'),body=wrap?.querySelector('tbody');if(!wrap||!body)return;
  const startTime=performance.now(),list=visible(),viewport=Math.max(rowHeight,wrap.clientHeight-headerHeight);
  const first=Math.floor(wrap.scrollTop/rowHeight),start=Math.max(0,Math.min(list.length,first-overscan)),end=Math.min(list.length,first+Math.ceil(viewport/rowHeight)+overscan);
  const selectionKey=paths().join('\0');
  if(!force&&start===virtualStart&&end===virtualEnd&&virtualRevision===dataRevision&&virtualSelection===selectionKey)return;
  const focused=document.activeElement instanceof HTMLTableRowElement?document.activeElement.dataset.path:undefined;
  const spacer=(height:number)=>height?`<tr class="virtual-spacer" aria-hidden="true"><td colspan="${columns.length}" style="height:${height}px"></td></tr>`:'';
  body.innerHTML=spacer(start*rowHeight)+list.slice(start,end).map((entry,i)=>rowMarkup(entry,start+i)).join('')+spacer((list.length-end)*rowHeight);
  virtualStart=start;virtualEnd=end;virtualRevision=dataRevision;virtualSelection=selectionKey;
  if(focused)wrap.querySelector<HTMLTableRowElement>(`tr[data-path="${CSS.escape(focused)}"]`)?.focus({preventScroll:true});
  timing('table window',startTime,{rendered:end-start,first:start,last:end});
}
function enabled(op: Operation): boolean {
  if (op === 'about') return true;
  if (busy) return false;
  if (op === 'select-all') return visible().length > 0;
  if (op === 'discard') return dirtySelected();
  if (!desktop) return false;
  if(op==='groups')return true;
  if(op==='folder'&&!runtimePlatform.folder_picker_supported)return false;
  if (op === 'folder' || op === 'files') return true;
  if (!selected.size) return false;
  if (op === 'save') return dirtySelected();
  if (op === 'reload' || op === 'extended') return true;
  if (op === 'remove-cover') return !dirtySelected() && !!coversFor(active()).length && selected.size === 1;
  if (op === 'playlist' || op === 'csv') return !dirtySelected();
  return !dirtySelected();
}
function button(op: Operation, label: string, icon = '', shortcut = '') {
  return `<button data-op="${op}" ${enabled(op) ? '' : 'disabled'} title="${esc(dirtySelected() && !['save','reload','extended','discard','folder','files','select-all','about'].includes(op) ? 'Save pending changes first' : label)}">${glyphs[op]?glyph(glyphs[op]!):icon?`<span class="icon" aria-hidden="true">${esc(icon)}</span>`:''}<span>${label}</span>${shortcut ? `<small>${shortcut}</small>` : ''}</button>`;
}
function menu(name: string, items: [Operation,string,string?][]) { return `<details class="menu" ${name==='Actions'?'id="actions-menu"':''}><summary>${name}</summary><div class="menu-list">${items.map(([op,label,key]) => button(op,label,'',key)).join('')}${name==='Actions'?actionUI.menuHTML():''}</div></details>`; }
function input(key: string) {
  const values = paths().map(path => field(entries.get(path)!,key));
  const mixed = values.some(value => value !== values[0]);
  const value = mixed ? '' : (values[0] || '');
  const control = key === 'COMMENT' ? `<textarea data-field="${key}" ${!selected.size || busy ? 'disabled' : ''} placeholder="${mixed ? '〈 keep existing values 〉' : ''}">${esc(value)}</textarea>` : `<input data-field="${key}" value="${esc(value)}" ${!selected.size || busy ? 'disabled' : ''} placeholder="${mixed ? '〈 keep existing values 〉' : ''}">`;
  return `<label class="field"><span>${key === 'ALBUMARTIST' ? 'ALBUM ARTIST' : key === 'DISCNUMBER' ? 'DISC NUMBER' : key}</span>${control}</label>`;
}
function render() {
  const started=performance.now(),oldWrap=document.querySelector<HTMLDivElement>('.table-wrap');
  if(oldWrap?.clientHeight){tableScrollTop=oldWrap.scrollTop;tableScrollLeft=oldWrap.scrollLeft;}
  const scrollTop=tableScrollTop,scrollLeft=tableScrollLeft,panelTop=document.querySelector('.fields')?.scrollTop||0;
  const focused=document.activeElement instanceof HTMLInputElement||document.activeElement instanceof HTMLTextAreaElement?document.activeElement:undefined;
  const focusSelector=focused?.id==="filter"?'#filter':focused?.dataset.field?`[data-field="${focused.dataset.field}"]`:undefined;
  const caretStart=focused?.selectionStart,caretEnd=focused?.selectionEnd;
  const list = visible(), entry = active();
  const selectedEntries = paths().map(path => entries.get(path)!);
  const selectedSize = selectedEntries.reduce((sum,e) => sum+e.size,0), selectedTime = selectedEntries.reduce((sum,e) => sum+e.duration,0);
  app.innerHTML = `<div class="app" data-pane="${mobilePane}"><header class="titlebar"><img class="brand-logo" src="${logoUrl}" alt="OpenTag"><span class="subtitle">Audio metadata editor</span><span class="window-mode">${desktop ? 'Local files' : 'Browser preview'}</span></header>
  <nav class="menubar" aria-label="Main menu">${menu('File',[['folder','Open folder…','⌘/Ctrl O'],['files','Add files…'],['save','Save tags','⌘/Ctrl S'],['reload','Reload tags'],['playlist','Create playlist…'],['csv','Export CSV…']])}${menu('Edit',[['select-all','Select all','⌘/Ctrl A'],['extended','Extended tags…'],['discard','Discard pending changes'],['remove-tags','Remove tags…']])}${menu('Convert',[['filename-tags','Filename → Tag…'],['tags-filename','Tag → Filename…']])}${menu('Actions',[['groups','Actions…','Alt 6'],['action','Actions (Quick)…','Alt Shift 6'],['number','Auto-numbering…']])}${menu('Help',[['about','About OpenTag']])}</nav>
  <div class="toolbar" role="toolbar" aria-label="File and tag operations">${button('folder','Open folder','▱')}${button('files','Add files','＋')}<span class="sep"></span>${button('save','Save tags','▣')}${button('reload','Reload','↻')}<span class="sep"></span>${button('filename-tags','Filename → Tag')}${button('tags-filename','Tag → Filename')}<span class="spacer"></span>${button('groups','Actions','⚙')}${button('number','Numbering','#')}</div>
  <nav class="mobile-panes" aria-label="Workspace view"><button data-pane="files" aria-pressed="${mobilePane==='files'}">Files (${entries.size})</button><button data-pane="tags" aria-pressed="${mobilePane==='tags'}" ${selected.size?'':'disabled'}>Tags (${selected.size})</button></nav>
  <main class="workspace"><aside class="editor" aria-label="Tag editor"><div class="panel-title">Tag panel <span>${selected.size ? `${selected.size} selected` : 'No selection'}</span></div><div class="fields">${input('TITLE')}${input('ARTIST')}${input('ALBUM')}${input('ALBUMARTIST')}<div class="field-pair">${input('YEAR')}${input('TRACK')}</div>${input('GENRE')}${input('COMMENT')}${input('COMPOSER')}${input('DISCNUMBER')}</div><div class="editor-actions">${button('save','Save tags')}${button('extended','Extended…')}</div>
  <section class="cover-section" aria-label="Embedded artwork">${coverMarkup()}</section></aside>
  <section class="files" aria-label="Audio files"><div class="location">${glyph(glyphOrganize)}<span class="path" title="${esc(location)}">${esc(location)}</span><strong>${entries.size} file${entries.size===1?'':'s'}</strong></div><div class="filterbar"><label for="filter">${glyph(glyphSearch)}<span class="visually-hidden">Filter</span></label><input id="filter" type="search" value="${esc(filter)}" placeholder="Filter filenames and tags, or enter a filter expression" autocomplete="off"><span class="count">${filterPending?'Filtering… · ':list.length === entries.size ? '' : `${list.length} visible · `}${selected.size} selected</span></div><div class="table-wrap"><table aria-label="Audio metadata" aria-rowcount="${list.length+1}"><thead><tr>${columns.map(c => `<th style="width:${c.width}px" aria-sort="${sortKey===c.key ? sortAscending?'ascending':'descending':'none'}"><button data-sort="${c.key}">${c.label}${sortKey===c.key?sortAscending?' ↑':' ↓':''}</button></th>`).join('')}</tr></thead><tbody></tbody></table>${!list.length ? `<div class="empty"><img class="empty-symbol" src="${markUrl}" alt="" aria-hidden="true"><h1>${filterPending ? 'Filtering files…' : filterFailure ? 'Invalid filter' : entries.size ? 'No matching files' : desktop ? 'Open your music folder' : 'Your music, ready to edit'}</h1><p>${filterPending ? 'Matching your filter against filenames and tag values.' : filterFailure ? esc(filterFailure) : entries.size ? 'Try another filename or tag value in the filter.' : desktop ? 'Open a folder, add audio files, or drop them into this window. Select files to edit their tags together.' : 'Open the desktop app to work with local audio files. Your files and tags stay on your computer.'}</p>${!entries.size ? `<div class="empty-actions">${button('folder','Open folder…')}${button('files','Add files…')}</div>` : ''}</div>` : ''}</div><div class="selection-info"><span>${selected.size ? `${selected.size} selected file${selected.size===1?'':'s'}` : 'Select a file to view its tags'}</span>${selected.size ? `<span>${length(selectedTime)} total</span><span>${(selectedSize/1048576).toFixed(1)} MB</span>` : ''}<span>${selected.size===1 && entry ? `${entry.sample_rate.toLocaleString()} Hz · ${entry.channels} channel${entry.channels===1?'':'s'}` : ''}</span></div>
  ${errors.length ? `<section class="errors" aria-label="Operation errors"><header>${errors.length} file error${errors.length===1?'':'s'}<button id="dismiss-errors">Dismiss</button></header>${errors.map(e => `<p><strong>${esc(basename(e.path))}:</strong> ${esc(e.error)}</p>`).join('')}</section>` : ''}</section></main><footer class="status ${busy?'busy':''}"><span class="status-dot" aria-hidden="true"></span><span class="status-message" role="status">${esc(status)}</span>${drafts.size ? `<span class="dirty-count">${drafts.size} unsaved file${drafts.size===1?'':'s'}</span>` : ''}<span>${entries.size} files</span></footer></div>`;
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap')!;
  wrap.querySelector('tbody')!.innerHTML=list.length?`<tr class="virtual-spacer" aria-hidden="true"><td colspan="${columns.length}" style="height:${list.length*rowHeight}px"></td></tr>`:'';
  wrap.scrollTop=scrollTop;wrap.scrollLeft=scrollLeft;updateVirtualRows(true);
  document.querySelector('.fields')!.scrollTop=panelTop;
  bind();setCoverImage();hydrateActiveCover();
  if(focusSelector){const replacement=document.querySelector<HTMLInputElement|HTMLTextAreaElement>(focusSelector);replacement?.focus({preventScroll:true});if(replacement&&caretStart!=null&&caretEnd!=null){try{replacement.setSelectionRange(caretStart,caretEnd);}catch{/* Search selection is not supported by every webview. */}}}
  timing('workspace render',started,{rendered:virtualEnd-virtualStart,selected:selected.size});
}
function updateDraft(path: string, key: string, values: string[]) {
  const original = entries.get(path)?.fields[key] || [];
  const draft = { ...drafts.get(path) };
  if (JSON.stringify(original) === JSON.stringify(values)) delete draft[key]; else draft[key] = values;
  if (Object.keys(draft).length) drafts.set(path,draft); else drafts.delete(path);
  changedData();
}
function draftField(key: string, value: string) {
  const values = value ? value.split('\\\\').map(v => v.trim()) : [];
  paths().forEach(path => {const entry=entries.get(path)!;updateDraft(path,panelKey({format:entry.format,fields:merged(entry)},key),values);});
  status = `Pending changes for ${selected.size} file${selected.size===1?'':'s'}. Save tags to write them.`;
  document.querySelector('.status-message')!.textContent = status;
  document.querySelectorAll<HTMLButtonElement>('button[data-op]').forEach(el => el.disabled = !enabled(el.dataset.op as Operation));
  const oldCount = document.querySelector('.dirty-count');
  if(oldCount) oldCount.textContent = `${drafts.size} unsaved file${drafts.size===1?'':'s'}`;
  else if(drafts.size) document.querySelector('.status-message')!.insertAdjacentHTML('afterend',`<span class="dirty-count">${drafts.size} unsaved file${drafts.size===1?'':'s'}</span>`);
  if(!drafts.size) oldCount?.remove();
  updateVirtualRows(true);
  document.querySelector('.filterbar .count')!.textContent=`${visible().length===entries.size?'':`${visible().length} visible · `}${selected.size} selected`;
}
function select(path: string, event: MouseEvent | KeyboardEvent) {
  const list = visible().map(e => e.path);
  if(event.shiftKey && anchor && list.includes(anchor)) {
    if(!event.metaKey && !event.ctrlKey) selected.clear();
    const a = list.indexOf(anchor), b = list.indexOf(path);
    list.slice(Math.min(a,b),Math.max(a,b)+1).forEach(p => selected.add(p));
  } else if(event.metaKey || event.ctrlKey) { selected.has(path) ? selected.delete(path) : selected.add(path); anchor = path; }
  else { selected.clear();selected.add(path);anchor = path; }
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap')!;
  const index=list.indexOf(path),top=index*rowHeight,bottom=top+rowHeight,viewport=wrap.clientHeight-headerHeight;
  if(top<wrap.scrollTop)wrap.scrollTop=top;else if(bottom>wrap.scrollTop+viewport)wrap.scrollTop=bottom-viewport;
  coverIndex = 0; coverError=undefined;if(activeCover?.path!==active()?.path)activeCover=undefined;render();
  document.querySelector<HTMLTableRowElement>(`tr[data-path="${CSS.escape(path)}"]`)?.focus({preventScroll:true});
}
function bind() {
  actionUI.bindMenu(app);
  document.querySelectorAll<HTMLButtonElement>('button[data-pane]').forEach(el=>el.addEventListener('click',()=>{mobilePane=el.dataset.pane as 'files'|'tags';render();}));
  document.querySelectorAll<HTMLButtonElement>('[data-op]').forEach(el => el.addEventListener('click', () => { document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d => d.open = false); void operate(el.dataset.op as Operation); }));
  document.querySelectorAll<HTMLInputElement|HTMLTextAreaElement>('[data-field]').forEach(el => el.addEventListener('input', () => draftField(el.dataset.field!,el.value)));
  document.querySelector<HTMLInputElement>('#filter')!.addEventListener('input',event => {
    const el = event.target as HTMLInputElement, position = el.selectionStart; filter = el.value; document.querySelector('.table-wrap')!.scrollTop=0;render();
    const replacement = document.querySelector<HTMLInputElement>('#filter')!;replacement.focus();try{replacement.setSelectionRange(position,position);}catch{/* Search controls do not expose selection on every webview. */}
  });
  document.querySelectorAll<HTMLButtonElement>('[data-sort]').forEach(el => el.addEventListener('click', () => { if(sortKey===el.dataset.sort) sortAscending=!sortAscending;else {sortKey=el.dataset.sort!;sortAscending=true;}document.querySelector('.table-wrap')!.scrollTop=0;render(); }));
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap')!;
  let scrollFrame=0;
  wrap.addEventListener('scroll',()=>{if(wrap.clientHeight){tableScrollTop=wrap.scrollTop;tableScrollLeft=wrap.scrollLeft;}if(!scrollFrame)scrollFrame=requestAnimationFrame(()=>{scrollFrame=0;if(wrap.isConnected&&wrap.clientHeight)updateVirtualRows();});},{passive:true});
  const rowFor=(event:Event)=>(event.target as Element).closest<HTMLTableRowElement>('tr[data-path]');
  wrap.addEventListener('click',e=>{
    const row=rowFor(e);if(!row)return;
    if((e.target as Element).classList.contains('row-select'))select(row.dataset.path!,new MouseEvent('click',{ctrlKey:true,shiftKey:e.shiftKey}));
    else{if(compactScreen())mobilePane='tags';select(row.dataset.path!,e);}
  });
  wrap.addEventListener('dblclick',e=>{if(rowFor(e))void operate('extended');});
  wrap.addEventListener('keydown',e=>{
    const row=rowFor(e);if(!row||e.target instanceof HTMLInputElement)return;
    if(e.key===' '){e.preventDefault();select(row.dataset.path!,e);}
    if(['ArrowDown','ArrowUp','Home','End','PageDown','PageUp'].includes(e.key)){
      e.preventDefault();const list=visible(),i=list.findIndex(x=>x.path===row.dataset.path),page=Math.max(1,Math.floor((wrap.clientHeight-headerHeight)/rowHeight));
      const next=e.key==='Home'?0:e.key==='End'?list.length-1:i+(e.key==='ArrowDown'?1:e.key==='ArrowUp'?-1:e.key==='PageDown'?page:-page);
      const entry=list[Math.max(0,Math.min(list.length-1,next))];if(entry)select(entry.path,e);
    }
    if(e.key==='Enter')void operate('extended');
  });
  document.querySelector('#dismiss-errors')?.addEventListener('click',()=>{errors=[];render();});
  document.querySelector<HTMLSelectElement>('#cover-choice')?.addEventListener('change',e=>{coverIndex=Number((e.target as HTMLSelectElement).value);refreshCoverPanel();});
}
async function run<T>(message: string, job: () => Promise<T>): Promise<T|undefined> {
  if(busy)return;busy=true;status=message;errors=[];render();
  try { return await job(); } catch(error) { errors=[{path:'Operation',error:String(error)}];status='Operation failed';return undefined; } finally {busy=false;render();}
}
function accept(batch: Batch, message: string, clearDrafts = true) {
  batch.entries.forEach(entry => {storeEntry(entry);if(clearDrafts)drafts.delete(entry.path);});
  errors=batch.errors;status=`${message}: ${batch.entries.length} file${batch.entries.length===1?'':'s'}${errors.length?`, ${errors.length} failed`:''}`;
}
function applyRenamed(batch:Batch){
  const changes=resolvePathChanges([...entries.keys()],batch.renamed||[],batch.moved_directories||[]);
  const snapshots=[...changes].map(([from,to])=>({from,to,entry:entries.get(from)!,draft:drafts.get(from),selected:selected.has(from)}));
  snapshots.forEach(({from})=>{invalidateCovers(from);entries.delete(from);drafts.delete(from);selected.delete(from);});
  snapshots.forEach(({from,to,entry,draft,selected:wasSelected})=>{storeEntry({...entry,path:to,filename:basename(to)});if(draft)drafts.set(to,draft);if(wasSelected)selected.add(to);if(anchor===from)anchor=to;});
}
async function scan(newPaths: string[]) {
  await run('Reading audio files…',async()=>{ const result=await invoke<Batch>('scan_files',{paths:newPaths,recursive:true});accept(result,'Loaded',false);location=newPaths.length===1?newPaths[0]:`${newPaths.length} added locations`;if(!selected.size&&result.entries.length){selected.add(result.entries[0].path);anchor=result.entries[0].path;} });
}
async function mutate(command: string, args: Record<string,unknown>, message: string) { await run(`${message}…`,async()=>accept(await invoke<Batch>(command,{paths:paths(),...args}),message)); }
async function saveDrafts() {
  await run('Saving tags…',async()=>{
    let successes=0;const failures:FileError[]=[];
    for(const path of paths()) { const updates=drafts.get(path);if(!updates)continue;const batch=await invoke<Batch>('save_fields',{paths:[path],updates});batch.entries.forEach(e=>{storeEntry(e);drafts.delete(e.path);successes++;});failures.push(...batch.errors); }
    errors=failures;status=`Saved tags: ${successes} file${successes===1?'':'s'}${failures.length?`, ${failures.length} failed`:''}`;
  });
}
function modal(title: string, content: string, buttons: string): HTMLDialogElement {
  const dialog=document.createElement('dialog');dialog.innerHTML=`<header>${esc(title)}</header><div class="dialog-content">${content}</div><div class="dialog-footer"><button data-close>Cancel</button>${buttons}</div>`;
  document.body.append(dialog);dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());dialog.addEventListener('close',()=>dialog.remove());dialog.showModal();return dialog;
}
function modalError(dialog: HTMLDialogElement, error: unknown) {let el=dialog.querySelector('.dialog-error');if(!el){el=document.createElement('p');el.className='dialog-error';el.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(el);}el.textContent=String(error);}
function value(dialog: HTMLDialogElement, id: string) {return dialog.querySelector<HTMLInputElement|HTMLSelectElement>(`#${id}`)!.value;}
function extended() {
  const all=paths().map(p=>merged(entries.get(p)!));const keys=[...new Set(all.flatMap(f=>Object.keys(f)))].sort();
  const dialog=modal('Extended tags',`<p>Each value can contain multiple lines. Use Add value for another tag value, or remove all values to remove a field. Renaming a field keeps each file's values.</p><div id="extended-rows"></div><button id="add-field">＋ Add field</button>`,`<button class="primary" id="apply-extended">Keep changes</button>`);
  const rows=dialog.querySelector('#extended-rows')!;
  const originalValues=new WeakMap<HTMLTextAreaElement,string>();
  const add=(key='',values:string[]=[],mixed=false)=>{
    const row=document.createElement('div');row.className='extended-row';row.dataset.original=key;row.dataset.changed=key?'false':'true';row.dataset.valueChanged='false';
    row.innerHTML=`<div class="extended-name"><label><span>FIELD</span><input aria-label="Field name" value="${esc(key)}" placeholder="FIELD"></label><button class="remove-field" aria-label="Remove field">Remove field</button></div><div class="extended-values"><div class="value-list"></div><span class="value-state">${mixed?'Mixed values — keep existing values':''}</span><button class="add-value">＋ Add value</button></div>`;
    const valueList=row.querySelector('.value-list')!,state=row.querySelector<HTMLElement>('.value-state')!;
    const changed=()=>{row.dataset.changed='true';row.dataset.valueChanged='true';state.textContent=valueList.childElementCount?'':'No values — this field will be removed';};
    const refreshLabels=()=>valueList.querySelectorAll<HTMLDivElement>('.extended-value').forEach((el,index)=>{el.querySelector('span')!.textContent=`VALUE ${index+1}`;el.querySelector('textarea')!.setAttribute('aria-label',`Field value ${index+1}`);el.querySelector('button')!.setAttribute('aria-label',`Remove value ${index+1}`);});
    const addValue=(text:string)=>{
      const item=document.createElement('div');item.className='extended-value';item.innerHTML='<label><span>VALUE</span><textarea aria-label="Field value"></textarea></label><button aria-label="Remove value">×</button>';
      const textarea=item.querySelector('textarea')!;textarea.value=text;originalValues.set(textarea,text);
      textarea.addEventListener('input',()=>{textarea.dataset.edited='true';changed();});item.querySelector('button')!.addEventListener('click',()=>{item.remove();refreshLabels();changed();});valueList.append(item);refreshLabels();
    };
    values.forEach(addValue);
    row.querySelector<HTMLInputElement>('.extended-name input')!.addEventListener('input',()=>row.dataset.changed='true');
    row.querySelector('.remove-field')!.addEventListener('click',()=>{row.dataset.changed='true';row.dataset.removed='true';row.hidden=true;});
    row.querySelector('.add-value')!.addEventListener('click',()=>{addValue('');changed();valueList.querySelector<HTMLTextAreaElement>('.extended-value:last-child textarea')!.focus();});
    rows.append(row);
  };
  keys.forEach(key=>{const vals=all.map(f=>f[key]||[]);const mixed=vals.some(v=>JSON.stringify(v)!==JSON.stringify(vals[0]));add(key,mixed?[]:vals[0],mixed);});
  dialog.querySelector('#add-field')!.addEventListener('click',()=>add());
  dialog.querySelector('#apply-extended')!.addEventListener('click',()=>{
    const edits:TagRowEdit[]=[...rows.querySelectorAll<HTMLDivElement>('.extended-row')].map(row=>({original:row.dataset.original||'',name:row.querySelector('input')!.value.trim().toUpperCase(),values:[...row.querySelectorAll<HTMLTextAreaElement>('.extended-value textarea')].map(textarea=>textarea.dataset.edited==='true'?textarea.value:originalValues.get(textarea)??textarea.value),changed:row.dataset.changed==='true',valueChanged:row.dataset.valueChanged==='true',removed:row.dataset.removed==='true'}));
    try {
      const perFile=paths().map(path=>({path,updates:resolveTagEdits(merged(entries.get(path)!),edits)}));
      perFile.forEach(({path,updates})=>Object.entries(updates).forEach(([key,values])=>updateDraft(path,key,values)));
    } catch(error) {modalError(dialog,error);return;}
    status='Extended tag changes are pending. Save tags to write them.';dialog.close();render();
  });
}
function converter(rename: boolean) {
  const dialog=modal(rename?'Tag → Filename':'Filename → Tag',`<p>${rename?'Preview filenames before applying. The audio file extension is retained.':'Extract tags from each filename using field placeholders. Preview the results before saving them to your files.'}</p><label class="field"><span>${rename?'FORMAT STRING':'FILENAME PATTERN'}</span><input id="template" value="%artist% - %title%"></label><div id="preview"></div>`,`<button id="preview-button">Preview</button><button class="primary" id="apply-converter" disabled>Apply to ${selected.size} files</button>`);
  const apply=dialog.querySelector<HTMLButtonElement>('#apply-converter')!;let previewed='',previewRevision=0,previewPaths:string[]=[],planToken:string|undefined;
  dialog.querySelector('#template')!.addEventListener('input',()=>{previewRevision++;previewed='';previewPaths=[];planToken=undefined;apply.disabled=true;dialog.querySelector('#preview')!.innerHTML='';});
  dialog.querySelector('#preview-button')!.addEventListener('click',async()=>{
    const previewButton=dialog.querySelector<HTMLButtonElement>('#preview-button')!;previewButton.disabled=true;apply.disabled=true;planToken=undefined;
    try{
      const template=value(dialog,'template'),targets=paths(),revision=previewRevision;
      if(rename){const result=await invoke<RenamePreview[]>('preview_rename',{paths:targets,template,columns:columnSpecs(targets)});if(!dialog.open||revision!==previewRevision)return;const token=result[0]?.plan_token;if(!token||result.length!==targets.length||result.some((row,index)=>row.path!==targets[index]||row.plan_token!==token))throw new Error('The rename preview could not be verified. Run Preview again.');dialog.querySelector('#preview')!.innerHTML=`<div class="preview">${result.map(r=>`<div><small>${esc(basename(r.path))}</small>${esc(r.value)}</div>`).join('')}</div>`;planToken=token;}
      else{const result=await invoke<FilenamePreview[]>('preview_filename',{paths:targets,pattern:template});if(!dialog.open||revision!==previewRevision)return;dialog.querySelector('#preview')!.innerHTML=`<div class="preview">${result.map(r=>`<div><small>${esc(basename(r.path))}</small>${Object.entries(r.fields).map(([key,values])=>`${esc(key)}: ${esc(values.join(' · '))}`).join('<br>')}</div>`).join('')}</div>`;}
      dialog.querySelector('.dialog-error')?.remove();previewed=template;previewPaths=targets;apply.disabled=value(dialog,'template')!==template||JSON.stringify(paths())!==JSON.stringify(targets)||(rename&&!planToken);
    }catch(e){modalError(dialog,e);}finally{previewButton.disabled=false;}
  });
  apply.addEventListener('click',async()=>{const template=value(dialog,'template');if(!template.trim()){modalError(dialog,'Enter a pattern.');return;}if(template!==previewed||JSON.stringify(paths())!==JSON.stringify(previewPaths)||(rename&&!planToken)){apply.disabled=true;modalError(dialog,'Run Preview again before applying this conversion.');return;}const reviewedPaths=[...previewPaths],reviewedToken=planToken;dialog.close();if(rename){await run('Renaming files…',async()=>{const batch=await invoke<Batch>('rename_files',{paths:reviewedPaths,template,planToken:reviewedToken});applyRenamed(batch);accept(batch,'Renamed');});}else await mutate('filename_to_tags',{pattern:template},'Converted filenames to tags');});
}
function numbering() {
  const dialog=modal('Auto-numbering',`<p>Number the selected files in their current table order.</p><div class="field-pair"><label class="field"><span>START AT</span><input id="number-start" type="number" min="1" value="1"></label><label class="field"><span>MINIMUM DIGITS</span><input id="number-digits" type="number" min="1" max="6" value="2"></label></div><label><input type="checkbox" id="number-total"> Include total tracks (01/12)</label>`,`<button class="primary" id="apply-number">Number ${selected.size} files</button>`);
  dialog.querySelector('#apply-number')!.addEventListener('click',()=>{const start=Number(value(dialog,'number-start')),digits=Number(value(dialog,'number-digits'));if(!Number.isInteger(start)||start<1||!Number.isInteger(digits)||digits<1||digits>6){modalError(dialog,'Enter a positive start number and 1–6 digits.');return;}const list=ordered().filter(e=>selected.has(e.path)),total=dialog.querySelector<HTMLInputElement>('#number-total')!.checked;list.forEach((entry,i)=>updateDraft(entry.path,panelKey({format:entry.format,fields:merged(entry)},'TRACK'),[String(start+i).padStart(digits,'0')+(total?`/${list.length}`:'')]));status=`Track numbers are pending for ${list.length} files. Save tags to write them.`;dialog.close();render();});
}
async function operateImpl(op: Operation) {
  if(!enabled(op))return;
  if(op==='about'){const dialog=modal('About OpenTag',`<img class="about-artwork" src="${aboutUrl}" alt="OpenTag — Tag, Organize, Enjoy"><p>Tag, Organize, Enjoy.</p><p>Open source music tag editor.</p><p>Version ${esc(appVersion)} · MIT License</p>`,'');dialog.querySelector('[data-close]')!.textContent='Close';return;}
  if(op==='folder'||op==='files'){const result=await open(op==='folder'?{directory:true,multiple:true,title:'Open audio folder'}:{multiple:true,title:'Add audio files'});if(result)await scan(Array.isArray(result)?result:[result]);return;}
  if(op==='save'){await saveDrafts();return;}
  if(op==='select-all'){selected.clear();visible().forEach(e=>selected.add(e.path));anchor=paths()[0];coverIndex=0;render();return;}
  if(op==='discard'){if(await ask('Discard the pending tag changes for the selected files?',{title:'Discard changes',kind:'warning'})){paths().forEach(p=>drafts.delete(p));changedData();status='Pending changes discarded';render();}return;}
  if(op==='reload'){if(dirtySelected()&&!await ask('Reload tags from disk and discard pending changes for the selected files?',{title:'Reload tags',kind:'warning'}))return;await run('Reloading tags…',async()=>accept(await invoke<Batch>('scan_files',{paths:paths(),recursive:false}),'Reloaded'));return;}
  if(op==='extended'){extended();return;}
  if(op==='filename-tags'||op==='tags-filename'){converter(op==='tags-filename');return;}
  if(op==='action'){actionUI.quick();return;}
  if(op==='groups'){await actionUI.groupsDialog();return;}
  if(op==='number'){numbering();return;}
  if(op==='add-cover'){const imagePath=await open({multiple:false,title:'Add embedded cover',filters:[{name:'Images',extensions:['jpg','jpeg','png','gif','webp']}]});if(typeof imagePath==='string')await mutate('add_cover',{imagePath},'Added cover');return;}
  if(op==='remove-cover'){const cover=coversFor(active())[coverIndex];if(cover)await mutate('remove_cover',{index:cover.index},'Removed cover');return;}
  if(op==='remove-tags'){if(await ask(`Remove all tags from ${selected.size} selected file${selected.size===1?'':'s'}?`,{title:'Remove tags',kind:'warning'}))await mutate('remove_tags',{},'Removed tags');return;}
  if(op==='playlist'||op==='csv'){const playlist=op==='playlist',outputPath=await save({title:playlist?'Create playlist':'Export tags as CSV',defaultPath:playlist?'playlist.m3u8':'tags.csv',filters:[{name:playlist?'M3U8 playlist':'CSV file',extensions:[playlist?'m3u8':'csv']}]});if(outputPath)await run('Exporting…',async()=>{await invoke(playlist?'export_playlist':'export_csv',{paths:paths(),outputPath});status=`Exported ${selected.size} files to ${basename(outputPath)}`;});}
}
async function operate(op: Operation) {
  try { await operateImpl(op); }
  catch(error) {errors=[{path:'Operation',error:String(error)}];status='Operation failed';render();}
}
document.addEventListener('keydown',event=>{
  if(document.querySelector('dialog[open]'))return;
  const typing=event.target instanceof HTMLInputElement||event.target instanceof HTMLTextAreaElement;
  if(event.altKey&&event.key==='6'){event.preventDefault();void operate(event.shiftKey?'action':'groups');return;}
  if(event.altKey&&event.key.toLowerCase()==='a'&&!typing){event.preventDefault();const menu=document.querySelector<HTMLDetailsElement>('#actions-menu')!;menu.open=true;actionUI.beginMnemonic();return;}
  if(!typing&&document.querySelector<HTMLDetailsElement>('#actions-menu')?.open&&event.key.length===1&&!event.metaKey&&!event.ctrlKey){event.preventDefault();void actionUI.mnemonic(event.key);return;}
  if(event.key==='Escape'){document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);}
  if(event.metaKey||event.ctrlKey){if(event.key.toLowerCase()==='s'){event.preventDefault();void operate('save');}if(event.key.toLowerCase()==='o'){event.preventDefault();void operate('folder');}if(event.key.toLowerCase()==='a'&&!typing){event.preventDefault();void operate('select-all');}}
});
document.addEventListener('click',e=>{if(!(e.target as Element).closest('.menu'))document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);});
window.addEventListener('beforeunload',event=>{if(drafts.size||actionUI.hasUnsavedChanges){event.preventDefault();event.returnValue='';}});
render();
let resizeFrame=0;
window.addEventListener('resize',()=>{if(!resizeFrame)resizeFrame=requestAnimationFrame(()=>{resizeFrame=0;rowHeight=compactScreen()?44:30;render();});});
if(desktop){
  void getVersion().then(version=>{appVersion=version;}).catch(()=>{appVersion='unknown';});
  void actionUI.initialize().catch(error=>{errors=[{path:'Action groups',error:String(error)}];render();});
  void invoke<typeof runtimePlatform>('runtime_platform').then(platform=>{runtimePlatform=platform;render();}).catch(error=>{errors=[{path:'Platform',error:String(error)}];render();});
  void getCurrentWindow().onCloseRequested(async event => {
    if(busy){event.preventDefault();return;}
    if(!drafts.size&&!actionUI.hasUnsavedChanges)return;
    event.preventDefault();
    const message=drafts.size?`Close OpenTag and discard pending changes in ${drafts.size} file${drafts.size===1?'':'s'}${actionUI.hasUnsavedChanges?' and unsaved action group changes':''}?`:'Close OpenTag and discard unsaved action group changes?';
    if(await ask(message,{title:'Unsaved changes',kind:'warning'})){
      await getCurrentWindow().destroy();
    }
  }).catch(error=>{errors=[{path:'Window',error:String(error)}];render();});
  void getCurrentWebview().onDragDropEvent(event=>{
    document.querySelector('.drop-overlay')?.remove();
    if(event.payload.type==='enter'||event.payload.type==='over'){const overlay=document.createElement('div');overlay.className='drop-overlay';overlay.textContent='Drop audio files or folders to add them';document.body.append(overlay);}
    if(event.payload.type==='drop'&&!busy)void scan(event.payload.paths);
  }).catch(error=>{errors=[{path:'Drag and drop',error:String(error)}];render();});
}
