/// <reference types="vite/client" />
import { invoke, isTauri } from '@tauri-apps/api/core';
import { buildFilterFields } from './filter-fields';
import { documentSelection, refreshDocumentAccess, copyRequest, copyOutcome, reconcileDrafts, sourcePresentation, publicDocumentMessage, type DocumentAccess, type DocumentState } from './document-state';
import { createConverterInputTracker, createConverterReview, validateFilenameItems, validateTextItems, type FilenameConversionRequest, type TextImportRequest, type FilenameConversionItem, type TextImportItem, type ConverterPreview } from './converter-review';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { createAppearance, type AppearanceState } from './theme';
import { installMenubar } from './menubar';
import { open, save, ask } from '@tauri-apps/plugin-dialog';
import { fieldNameIdentity, resolveTagEdits, selectionTagRows, type TagRowEdit } from './tag-edits';
import { ActionUI, panelKey, resolvePathChanges, type ActionGroup, type ActionLibrary, type ImportedGroup } from './action-ui';
import logoLightUrl from '../assets/brand/OpenTag-Brand-Kit/01-logos/horizontal/opentag-logo-light.svg';
import markLightUrl from '../assets/brand/OpenTag-Brand-Kit/01-logos/marks/opentag-mark-color.svg';
import artworkLightUrl from '../assets/brand/OpenTag-Brand-Kit/04-in-app/opentag-default-artwork-light-1024.svg';
import aboutLightUrl from '../assets/brand/OpenTag-Brand-Kit/04-in-app/opentag-splash-light-1600x1000.svg';
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
import logoDarkUrl from '../assets/brand/OpenTag-Brand-Kit/01-logos/horizontal/opentag-logo-dark.svg';
import markDarkUrl from '../assets/brand/OpenTag-Brand-Kit/01-logos/marks/opentag-mark-dark.svg';
import artworkDarkUrl from '../assets/brand/OpenTag-Brand-Kit/04-in-app/opentag-default-artwork-dark-1024.svg';
import aboutDarkUrl from '../assets/brand/OpenTag-Brand-Kit/04-in-app/opentag-splash-dark-1600x1000.svg';
import './style.css';

type Fields = Record<string, string[]>;
type Cover = { index: number; mime: string; description: string; picture_type: string; data_base64: string };
type Entry = { path: string; filename: string; format: string; duration: number; bitrate: number; sample_rate: number; channels: number; size: number; fields: Fields; technical?: Fields; covers: Cover[] };
type FileError = { path: string; error: string };
type Batch = { entries: Entry[]; errors: FileError[]; renamed?: { from: string; to: string }[]; moved_directories?: {from:string;to:string}[] };
type Preview = { path: string; value: string };
type RenamePreview = Preview & {plan_token:string};
type FilenamePreview = { path: string; fields: Fields };
type Operation = 'folder'|'files'|'save'|'save-copy'|'refresh-source'|'reload'|'extended'|'add-cover'|'remove-cover'|'filename-tags'|'tags-filename'|'filename-filename'|'text-tags'|'action'|'groups'|'number'|'remove-tags'|'playlist'|'csv'|'select-all'|'discard'|'about';
const glyphs:Partial<Record<Operation,string>>={folder:glyphImport,files:glyphMusic,save:glyphSave,'save-copy':glyphSave,'refresh-source':glyphOrganize,reload:glyphOrganize,extended:glyphEdit,'add-cover':glyphAlbum,'remove-cover':glyphEdit,'filename-tags':glyphImport,'tags-filename':glyphExport,'filename-filename':glyphOrganize,'text-tags':glyphImport,action:glyphBatch,groups:glyphSettings,number:glyphOrganize,'remove-tags':glyphEdit,playlist:glyphOrganize,csv:glyphExport,'select-all':glyphCheck,discard:glyphEdit,about:glyphInfo};
const glyph=(source:string)=>`<span class="icon" aria-hidden="true">${source}</span>`;
const desktop = isTauri();
const appearanceAssets = {
  light: {logo:logoLightUrl,mark:markLightUrl,artwork:artworkLightUrl,about:aboutLightUrl},
  dark: {logo:logoDarkUrl,mark:markDarkUrl,artwork:artworkDarkUrl,about:aboutDarkUrl},
};
let nativeAppearanceReady=false, nativeAppearanceChoice:string|undefined;
let nativeAppearanceQueue=Promise.resolve();
const appearance = createAppearance({
  media:window.matchMedia('(prefers-color-scheme: dark)'),
  storage:{getItem:key=>window.localStorage.getItem(key),setItem:(key,value)=>window.localStorage.setItem(key,value)},
  changed:applyAppearance,
});
const themedAsset=(kind:keyof typeof appearanceAssets.light)=>appearanceAssets[appearance.state.resolved][kind];
function syncNativeAppearance(){
  if(!desktop||!nativeAppearanceReady)return;
  const choice=appearance.state.preference;
  if(nativeAppearanceChoice===choice)return;
  nativeAppearanceChoice=choice;
  nativeAppearanceQueue=nativeAppearanceQueue.then(()=>getCurrentWindow().setTheme(choice==='system'?null:choice))
    .catch(error=>{nativeAppearanceChoice=undefined;console.warn('Native appearance could not update',error);});
}
function applyAppearance(state:AppearanceState){
  document.documentElement.dataset.theme=state.resolved;
  document.documentElement.style.colorScheme=state.resolved;
  document.querySelectorAll<HTMLImageElement>('img[data-theme-image]').forEach(image=>{
    const kind=image.dataset.themeImage as keyof typeof appearanceAssets.light;
    image.src=appearanceAssets[state.resolved][kind];
  });
  const select=document.querySelector<HTMLSelectElement>('#appearance');
  if(select){select.value=state.preference;select.title=`Appearance: ${state.preference==='system'?`System (${state.resolved})`:state.resolved}`;const system=select.querySelector<HTMLOptionElement>('option[value="system"]');if(system)system.textContent=state.preference==='system'?`System (${state.resolved==='dark'?'Dark':'Light'})`:'System';}
  syncNativeAppearance();
}
applyAppearance(appearance.state);

let appVersion='development';
const app = document.querySelector<HTMLDivElement>('#app')!;
const entries = new Map<string, Entry>();
const drafts = new Map<string, Fields>();
// Recovery working bytes can differ from loaded tags. Preserve explicit resets
// for edited copies separately from pending drafts, including absent-field tombstones.
const copyOverrides = new Map<string, Fields>();
const workingReadErrors = new Map<string, string>();
const selected = new Set<string>();
const columns = [{ key:'filename',label:'Filename',width:220 },{ key:'TITLE',label:'Title',width:175 },{ key:'ARTIST',label:'Artist',width:145 },{ key:'ALBUM',label:'Album',width:165 },{ key:'TRACK',label:'Track',width:55 },{ key:'YEAR',label:'Year',width:55 },{ key:'GENRE',label:'Genre',width:90 },{ key:'duration',label:'Length',width:60 },{ key:'format',label:'Format',width:65 },{ key:'bitrate',label:'Bitrate',width:65 }];
let filter = '', sortKey = 'filename', sortAscending = true, anchor: string | undefined, busy = false, status = 'Ready', errors: FileError[] = [], location = 'No folder loaded', coverIndex = 0;
const esc = (value: unknown) => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const basename = (path: string) => path.split(/[\\/]/).pop() || path;
const paths = () => ordered().filter(entry=>selected.has(entry.path)).map(entry=>entry.path);
const merged = (entry: Entry): Fields => ({ ...entry.fields, ...copyOverrides.get(entry.path), ...drafts.get(entry.path) });
const field = (entry: Entry, key: string) => (merged(entry)[panelKey({format:entry.format,fields:merged(entry)},key)] || []).join(' \\\\ ');
const length = (seconds: number) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2,'0')}`;
const dirtySelected = () => [...selected].some(path => drafts.has(path));
const active = () => entries.get(selected.values().next().value||'');
const collator = new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
const dev = (import.meta as ImportMeta & {env?: {DEV?: boolean}}).env?.DEV === true;
const compactScreen=()=>window.matchMedia('(max-width:1000px)').matches;
let rowHeight = compactScreen()?44:30;
const headerHeight = 29, overscan = 8;
let mobilePane:'files'|'tags'='files';
let tableScrollTop=0,tableScrollLeft=0;
let editorScrollTop=0;
let runtimePlatform={family:'desktop' as 'desktop'|'android'|'ios',folder_picker_supported:true,document_access_supported:false};
let documentAccess:DocumentAccess={states:{},refreshError:null};
let dataRevision = 0, orderCacheKey = '', filterCacheKey = '', orderCache: Entry[] = [], filterCache: Entry[] = [];
let reviewedConverterContextChanged: (() => void) | undefined;
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
  canExecute:()=>enabled('action'),executeReason:()=>operationReason('action'),selectedCount:()=>selected.size,fieldNames:()=>[...new Set(paths().flatMap(path=>Object.keys(merged(entries.get(path)!))))],changed:()=>render(),
});
async function executeGroups(groups:ActionGroup[]){
  if(!enabled('action'))throw new Error(operationReason('action')||'Select files and save pending tag edits before applying actions.');
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
function changedData(){dataRevision++;reviewedConverterContextChanged?.();}
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
function providerEntry(entry:Entry){return runtimePlatform.document_access_supported||!!documentAccess.states[entry.path]||!!entry.technical?._PATH?.[0]?.startsWith('content://');}
function publicMessage(error:unknown){return publicDocumentMessage(error,documentAccess.states);}
function displayName(path:string){return documentAccess.states[path]?.name||entries.get(path)?.filename||basename(publicMessage(path));}
function filterFields(entry:Entry):Fields{
  const source=documentAccess.states[entry.path];
  const technical=source?{...entry.technical,_PATH:[source.uri]}:entry.technical;
  return buildFilterFields({...entry,technical},merged(entry),providerEntry(entry)?{uri:source?.uri||entry.technical?._PATH?.[0]}:undefined);
}
function sourcePolicy(){return documentSelection(paths(),documentAccess,runtimePlatform.document_access_supported);}
async function refreshSourceAccess(){
  if(!runtimePlatform.document_access_supported)return;
  documentAccess=await refreshDocumentAccess(documentAccess,()=>invoke<Record<string,DocumentState>>('document_states'));
}
function workingMetadataNotice(path:string){
  const error=workingReadErrors.get(path);
  return error?`<p class="source-error" role="alert">Retained tags and artwork are unavailable: ${esc(publicMessage(error))}. Displayed tags are the last read values plus pending edits; they may differ from an edited copy. Choose Original backup to export the last verified source.</p>`:'';
}
function sourceMarkup(context:'files'|'tags'){
  const entry=active();if(!entry||!providerEntry(entry))return '';
  const state=documentAccess.states[entry.path],source=sourcePresentation(state,!!documentAccess.refreshError),policy=sourcePolicy();
  const reason=policy.mutationReason||'Tag and cover saves are verified against the source.';
  return `<section class="source-access ${state?.state==='recovery_required'?'recovery':''}" aria-label="Document source access"><div><strong>${esc(source.label)}</strong><span>${esc(source.name)}</span></div><p class="source-location" title="${esc(source.location)}">${esc(source.location)}</p><p>${esc(source.detail)}</p>${workingMetadataNotice(entry.path)}${state?.error?`<p class="source-error" role="alert">${esc(publicMessage(state.error))}</p>`:''}${documentAccess.refreshError?`<p class="source-error" role="alert">Source access could not be refreshed: ${esc(publicMessage(documentAccess.refreshError))}. Previous source details and pending edits are retained.</p>`:''}<p id="source-policy-${context}">${esc(reason)}</p><p class="source-limit">${esc(policy.unsupportedReason)}</p><div class="source-actions">${button('save-copy','Save a copy…')}${button('refresh-source','Refresh source access')}</div></section>`;
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
  if(query&&searchRevision!==dataRevision){searchCache.clear();all.forEach(entry=>searchCache.set(entry.path,`${entry.filename} ${providerEntry(entry)?documentAccess.states[entry.path]?.uri||'':entry.path} ${Object.values(merged(entry)).flat().join(' ')}`.toLocaleLowerCase()));searchRevision=dataRevision;}
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
  if(coverError?.path===entry.path)coverError=undefined;
  invalidateCovers(entry.path);const epoch=coverEpochs.get(entry.path)!;
  if(active()?.path===entry.path)coverIndex=Math.min(coverIndex,Math.max(0,entry.covers.length-1));
  if(entry.covers.length&&entry.covers.every(cover=>!!cover.data_base64)){cacheCovers(entry.path,epoch,entry.covers);if(active()?.path===entry.path)activeCover={path:entry.path,epoch,covers:entry.covers};}
  entries.set(entry.path,{...entry,covers:entry.covers.map(cover=>({...cover,data_base64:''}))});changedData();
}
function coversFor(entry:Entry|undefined):Cover[]{
  if(!entry||workingReadErrors.has(entry.path))return[];const epoch=coverEpochs.get(entry.path)||0;
  if(activeCover?.path===entry.path&&activeCover.epoch===epoch)return activeCover.covers;
  const cached=coverCache.get(entry.path);if(cached?.epoch===epoch)return cached.covers;
  return entry.covers;
}
function coverMarkup(){
  const entry=active(),covers=coversFor(entry),cover=covers[Math.min(coverIndex,covers.length-1)];
  const loaded=!!cover?.data_base64,error=coverError&&coverError.path===entry?.path?coverError.message:'';
  return `<div class="cover-row"><div class="cover-art">${loaded?`<img alt="Embedded ${esc(cover.picture_type)} artwork">`:`<img class="fallback-artwork" data-theme-image="artwork" src="${themedAsset('artwork')}" alt="" aria-hidden="true">`}</div><div class="cover-info"><strong>${entry&&workingReadErrors.has(entry.path)?'Artwork unavailable':cover?esc(cover.picture_type||'Embedded cover'):'No artwork'}</strong>${error?esc(error):cover?`${esc(cover.mime)}<br>${loaded?esc(cover.description||`${covers.length} embedded image${covers.length===1?'':'s'}`):'Loading artwork…'}`:selected.size>1?'First selected file':'Embedded cover art'}</div></div><div class="cover-controls">${button('add-cover','Add…')}${button('remove-cover','Remove')}${covers.length>1?`<select id="cover-choice" aria-label="Select cover">${covers.map((c,i)=>`<option value="${i}" ${i===coverIndex?'selected':''}>${i+1}: ${esc(c.picture_type)}</option>`).join('')}</select>`:''}</div>`;
}
function setCoverImage(){const covers=coversFor(active()),cover=covers[Math.min(coverIndex,covers.length-1)],image=document.querySelector<HTMLImageElement>('.cover-art img');if(image&&cover?.data_base64)image.src=`data:${cover.mime};base64,${cover.data_base64}`;}
function refreshCoverPanel(){
  const panel=document.querySelector('.cover-section');if(!panel)return;panel.innerHTML=coverMarkup();
  setCoverImage();
  panel.querySelectorAll<HTMLButtonElement>('[data-op]').forEach(el=>el.addEventListener('click',()=>void operate(el.dataset.op as Operation)));
  panel.querySelector<HTMLSelectElement>('#cover-choice')?.addEventListener('change',e=>{coverIndex=Number((e.target as HTMLSelectElement).value);refreshCoverPanel();});
}
function hydrateActiveCover(){
  const entry=active();if(!desktop||!entry?.covers.length||workingReadErrors.has(entry.path))return;
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
  }).catch(error=>{if(active()?.path===path&&(coverEpochs.get(path)||0)===epoch){coverError={path,message:`Artwork could not be loaded: ${publicMessage(error)}`};refreshCoverPanel();}}).finally(()=>coverRequests.delete(requestKey));
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
function operationReason(op:Operation):string {
  const policy=sourcePolicy();
  if(op==='folder'&&!runtimePlatform.folder_picker_supported)return 'Folder access is not available here. Use Add files.';
  if(op==='save-copy')return policy.copyReason;
  if(['action','tags-filename','filename-filename','text-tags','playlist','csv'].includes(op)&&policy.unsupportedReason)return policy.unsupportedReason;
  if(['save','add-cover','remove-cover','remove-tags','filename-tags'].includes(op)&&policy.mutationReason)return policy.mutationReason;
  if(op==='reload'&&policy.reloadReason)return policy.reloadReason;
  if(op==='number'&&dirtySelected()&&!sourcePolicy().unsupportedReason)return 'Save pending changes first.';
  if(dirtySelected()&&!['save','save-copy','refresh-source','reload','extended','number','discard','folder','files','select-all','about','groups'].includes(op))return 'Save pending changes first.';
  return '';
}
function enabled(op: Operation): boolean {
  if (op === 'about') return true;
  if (busy) return false;
  if (op === 'select-all') return visible().length > 0;
  if (op === 'discard') return dirtySelected();
  if (!desktop) return false;
  if(op==='groups')return true;
  if(op==='refresh-source')return runtimePlatform.document_access_supported;
  if(operationReason(op))return false;
  if (op === 'folder' || op === 'files') return true;
  if (!selected.size) return false;
  if (op === 'save') return dirtySelected();
  if (op === 'reload' || op === 'extended' || op === 'number' || op === 'save-copy') return true;
  if (op === 'remove-cover') return !!coversFor(active()).length && selected.size === 1;
  return true;
}
function button(op: Operation, label: string, icon = '', shortcut = '') {
  return `<button data-op="${op}" ${enabled(op) ? '' : 'disabled'} title="${esc(operationReason(op)||label)}">${glyphs[op]?glyph(glyphs[op]!):icon?`<span class="icon" aria-hidden="true">${esc(icon)}</span>`:''}<span>${label}</span>${shortcut ? `<small>${shortcut}</small>` : ''}</button>`;
}
function menu(name: string, items: [Operation,string,string?][]) { return `<details class="menu" ${name==='Actions'?'id="actions-menu"':''}><summary>${name}</summary><div class="menu-list">${items.map(([op,label,key]) => button(op,label,'',key)).join('')}${name==='Actions'?actionUI.menuHTML():''}${name==='File'&&!runtimePlatform.folder_picker_supported?'<p class="menu-note">Folder access is not available here. Use Add files.</p>':''}${['Convert','Actions'].includes(name)&&sourcePolicy().unsupportedReason?`<p class="menu-note">${esc(sourcePolicy().unsupportedReason)}</p>`:''}</div></details>`; }
function input(key: string) {
  const values = paths().map(path => field(entries.get(path)!,key));
  const mixed = values.some(value => value !== values[0]);
  const value = mixed ? '' : (values[0] || '');
  const control = key === 'COMMENT' ? `<textarea data-field="${key}" ${!selected.size || busy ? 'disabled' : ''} placeholder="${mixed ? '〈 keep existing values 〉' : ''}">${esc(value)}</textarea>` : `<input data-field="${key}" value="${esc(value)}" ${!selected.size || busy ? 'disabled' : ''} placeholder="${mixed ? '〈 keep existing values 〉' : ''}">`;
  return `<label class="field"><span>${key === 'ALBUMARTIST' ? 'ALBUM ARTIST' : key === 'DISCNUMBER' ? 'DISC NUMBER' : key}</span>${control}</label>`;
}
function render() {
  reviewedConverterContextChanged?.();
  const started=performance.now(),oldWrap=document.querySelector<HTMLDivElement>('.table-wrap');
  if(oldWrap?.clientHeight){tableScrollTop=oldWrap.scrollTop;tableScrollLeft=oldWrap.scrollLeft;}
  const scrollTop=tableScrollTop,scrollLeft=tableScrollLeft,panelTop=document.querySelector('.fields')?.scrollTop||0;
  const oldEditor=document.querySelector<HTMLElement>('.editor');
  if(oldEditor?.clientHeight)editorScrollTop=oldEditor.scrollTop;
  const focused=document.activeElement instanceof HTMLInputElement||document.activeElement instanceof HTMLTextAreaElement?document.activeElement:undefined;
  const focusSelector=focused?.id==="filter"?'#filter':focused?.dataset.field?`[data-field="${focused.dataset.field}"]`:undefined;
  const caretStart=focused?.selectionStart,caretEnd=focused?.selectionEnd;
  const list = visible(), entry = active();
  const selectedEntries = paths().map(path => entries.get(path)!);
  const selectedSize = selectedEntries.reduce((sum,e) => sum+e.size,0), selectedTime = selectedEntries.reduce((sum,e) => sum+e.duration,0);
  app.innerHTML = `<div class="app" data-pane="${mobilePane}"><header class="titlebar"><img class="brand-logo" data-theme-image="logo" src="${themedAsset('logo')}" alt="OpenTag"><span class="subtitle">Audio metadata editor</span><span class="window-mode">${runtimePlatform.document_access_supported?'Provider documents':desktop?'Local files':'Browser preview'}</span><label class="appearance-control"><span>Appearance</span><select id="appearance" aria-label="Appearance"><option value="system">${appearance.state.preference==='system'?`System (${appearance.state.resolved==='dark'?'Dark':'Light'})`:'System'}</option><option value="light">Light</option><option value="dark">Dark</option></select></label></header>
  <nav class="menubar" aria-label="Main menu">${menu('File',[['folder','Open folder…',runtimePlatform.document_access_supported?undefined:'⌘/Ctrl O'],['files','Add files…',runtimePlatform.document_access_supported?'⌘/Ctrl O':undefined],['save','Save tags','⌘/Ctrl S'],['save-copy','Save a copy…'],['reload','Reload tags'],['playlist','Create playlist…'],['csv','Export CSV…']])}${menu('Edit',[['select-all','Select all','⌘/Ctrl A'],['extended','Extended tags…'],['discard','Discard pending changes'],['remove-tags','Remove tags…']])}${menu('Convert',[['filename-tags','Filename → Tag…'],['tags-filename','Tag → Filename…'],['filename-filename','Filename → Filename…'],['text-tags','Text File → Tag…']])}${menu('Actions',[['groups','Actions…','Alt 6'],['action','Actions (Quick)…','Alt Shift 6'],['number','Auto-numbering…']])}${menu('Help',[['about','About OpenTag']])}</nav>
  <div class="toolbar" role="toolbar" aria-label="File and tag operations">${button('folder','Open folder','▱')}${button('files','Add files','＋')}<span class="sep"></span>${button('save','Save tags','▣')}${button('reload','Reload','↻')}<span class="sep"></span>${button('filename-tags','Filename → Tag')}${button('tags-filename','Tag → Filename')}<span class="spacer"></span>${button('groups','Actions','⚙')}${button('number','Numbering','#')}</div>
  <nav class="mobile-panes" aria-label="Workspace view"><button data-pane="files" aria-pressed="${mobilePane==='files'}">Files (${entries.size})</button><button data-pane="tags" aria-pressed="${mobilePane==='tags'}" ${selected.size?'':'disabled'}>Tags (${selected.size})</button></nav>
  <main class="workspace"><aside class="editor" aria-label="Tag editor"><div class="panel-title">Tag panel <span>${selected.size ? `${selected.size} selected` : 'No selection'}</span></div>${sourceMarkup('tags')}<div class="fields">${input('TITLE')}${input('ARTIST')}${input('ALBUM')}${input('ALBUMARTIST')}<div class="field-pair">${input('YEAR')}${input('TRACK')}</div>${input('GENRE')}${input('COMMENT')}${input('COMPOSER')}${input('DISCNUMBER')}</div><div class="editor-actions">${button('save','Save tags')}${button('extended','Extended…')}</div>
  <section class="cover-section" aria-label="Embedded artwork">${coverMarkup()}</section></aside>
  <section class="files" aria-label="Audio files"><div class="location">${glyph(glyphOrganize)}<span class="path" title="${esc(entry&&providerEntry(entry)?sourcePresentation(documentAccess.states[entry.path]).location:location)}">${esc(entry&&providerEntry(entry)?sourcePresentation(documentAccess.states[entry.path]).location:location)}</span><strong>${entries.size} file${entries.size===1?'':'s'}</strong></div>${sourceMarkup('files')}<div class="filterbar"><label for="filter">${glyph(glyphSearch)}<span class="visually-hidden">Filter</span></label><input id="filter" type="search" value="${esc(filter)}" placeholder="Filter filenames and tags, or enter a filter expression" autocomplete="off"><span class="count">${filterPending?'Filtering… · ':list.length === entries.size ? '' : `${list.length} visible · `}${selected.size} selected</span></div><div class="table-wrap"><table aria-label="Audio metadata" aria-rowcount="${list.length+1}"><thead><tr>${columns.map(c => `<th style="width:${c.width}px" aria-sort="${sortKey===c.key ? sortAscending?'ascending':'descending':'none'}"><button data-sort="${c.key}">${c.label}${sortKey===c.key?sortAscending?' ↑':' ↓':''}</button></th>`).join('')}</tr></thead><tbody></tbody></table>${!list.length ? `<div class="empty"><img class="empty-symbol" data-theme-image="mark" src="${themedAsset('mark')}" alt="" aria-hidden="true"><h1>${filterPending ? 'Filtering files…' : filterFailure ? 'Invalid filter' : entries.size ? 'No matching files' : runtimePlatform.document_access_supported?'Add audio documents':desktop ? 'Open your music folder' : 'Your music, ready to edit'}</h1><p>${filterPending ? 'Matching your filter against filenames and tag values.' : filterFailure ? esc(filterFailure) : entries.size ? 'Try another filename or tag value in the filter.' : runtimePlatform.document_access_supported?'Choose audio files from your document provider. Source access and recovery options appear with your selection.':desktop ? 'Open a folder, add audio files, or drop them into this window. Select files to edit their tags together.' : 'Open the desktop app to work with local audio files. Your files and tags stay on your computer.'}</p>${!entries.size ? `<div class="empty-actions">${button('folder','Open folder…')}${button('files','Add files…')}</div>` : ''}</div>` : ''}</div><div class="selection-info"><span>${selected.size ? `${selected.size} selected file${selected.size===1?'':'s'}` : 'Select a file to view its tags'}</span>${selected.size ? `<span>${length(selectedTime)} total</span><span>${(selectedSize/1048576).toFixed(1)} MB</span>` : ''}<span>${selected.size===1 && entry ? `${entry.sample_rate.toLocaleString()} Hz · ${entry.channels} channel${entry.channels===1?'':'s'}` : ''}</span></div>
  ${errors.length ? `<section class="errors" aria-label="Operation errors"><header>${errors.length} file error${errors.length===1?'':'s'}<button id="dismiss-errors">Dismiss</button></header>${errors.map(e => `<p><strong>${esc(displayName(e.path))}:</strong> ${esc(publicMessage(e.error))}</p>`).join('')}</section>` : ''}</section></main><footer class="status ${busy?'busy':''}"><span class="status-dot" aria-hidden="true"></span><span class="status-message" role="status">${esc(status)}</span>${drafts.size ? `<span class="dirty-count">${drafts.size} unsaved file${drafts.size===1?'':'s'}</span>` : ''}<span>${entries.size} files</span></footer></div>`;
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap')!;
  wrap.querySelector('tbody')!.innerHTML=list.length?`<tr class="virtual-spacer" aria-hidden="true"><td colspan="${columns.length}" style="height:${list.length*rowHeight}px"></td></tr>`:'';
  wrap.scrollTop=scrollTop;wrap.scrollLeft=scrollLeft;updateVirtualRows(true);
  document.querySelector('.fields')!.scrollTop=panelTop;
  document.querySelector<HTMLElement>('.editor')!.scrollTop=editorScrollTop;
  bind();setCoverImage();hydrateActiveCover();applyAppearance(appearance.state);
  if(focusSelector){const replacement=document.querySelector<HTMLInputElement|HTMLTextAreaElement>(focusSelector);replacement?.focus({preventScroll:true});if(replacement&&caretStart!=null&&caretEnd!=null){try{replacement.setSelectionRange(caretStart,caretEnd);}catch{/* Search selection is not supported by every webview. */}}}
  timing('workspace render',started,{rendered:virtualEnd-virtualStart,selected:selected.size});
}
function copyBaseUnknown(path:string){
  const entry=entries.get(path);
  return !!entry&&providerEntry(entry)&&(!!documentAccess.refreshError||documentAccess.states[path]?.state!=='ready');
}
function editedCopyFields(path:string):Fields{return {...copyOverrides.get(path),...drafts.get(path)};}
function discardDrafts(path:string){
  if(copyBaseUnknown(path)){
    const overrides={...copyOverrides.get(path)};
    for(const key of Object.keys(drafts.get(path)||{}))overrides[key]=[...(entries.get(path)?.fields[key]||[])];
    if(Object.keys(overrides).length)copyOverrides.set(path,overrides);
  }else copyOverrides.delete(path);
  drafts.delete(path);
}
function updateDraft(path: string, key: string, values: string[]) {
  const original = entries.get(path)?.fields[key] || [];
  const draft = { ...drafts.get(path) };
  const resetToLoaded=JSON.stringify(original)===JSON.stringify(values);
  const overrides={...copyOverrides.get(path)};
  if(copyBaseUnknown(path)&&resetToLoaded)overrides[key]=[...values];else delete overrides[key];
  if(Object.keys(overrides).length)copyOverrides.set(path,overrides);else copyOverrides.delete(path);
  if (resetToLoaded) delete draft[key]; else draft[key] = values;
  if (Object.keys(draft).length) drafts.set(path,draft); else drafts.delete(path);
  changedData();
}
function draftField(key: string, value: string) {
  const values = value ? value.split('\\\\').map(v => v.trim()) : [];
  paths().forEach(path => {const entry=entries.get(path)!;updateDraft(path,panelKey({format:entry.format,fields:merged(entry)},key),values);});
  status = `Pending changes for ${selected.size} file${selected.size===1?'':'s'}. ${sourcePolicy().mutationReason?'Use Save a copy to keep these edits.':'Save tags to write them.'}`;
  document.querySelector('.status-message')!.textContent = status;
  document.querySelectorAll<HTMLButtonElement>('button[data-op]').forEach(el => {el.disabled = !enabled(el.dataset.op as Operation);el.title=operationReason(el.dataset.op as Operation)||el.textContent||'';});
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
  document.querySelector<HTMLSelectElement>('#appearance')!.addEventListener('change',event=>appearance.set((event.target as HTMLSelectElement).value));
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
async function run<T>(message: string, job: () => Promise<T>, options: {preserveErrors?:boolean} = {}): Promise<T|undefined> {
  if(busy)return;busy=true;status=message;if(!options.preserveErrors)errors=[];render();
  try { return await job(); } catch(error) { const failure={path:'Operation',error:String(error)};errors=options.preserveErrors?[...errors,failure]:[failure];status='Operation failed';return undefined; } finally {await refreshSourceAccess();busy=false;render();}
}
function accept(batch: Batch, message: string, clearDrafts = true) {
  batch.entries.forEach(storeEntry);reconcileDrafts(drafts,batch.entries.map(entry=>entry.path),clearDrafts);if(clearDrafts)batch.entries.forEach(entry=>{copyOverrides.delete(entry.path);workingReadErrors.delete(entry.path);});
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
// Immediate mutations may change retained bytes even when publication fails.
// Keep those local contents separate from source success and pending field intent.
async function reconcileWorkingEntries(targets:string[]){
  const managed=[...new Set(targets)].filter(path=>{const entry=entries.get(path);return !!entry&&providerEntry(entry);});
  if(!managed.length)return;
  const unavailable=(path:string,error:string)=>{workingReadErrors.set(path,error);invalidateCovers(path);};
  try{
    const batch=await invoke<Batch>('document_working_entries',{paths:managed});
    for(const path of managed){
      const entry=batch.entries.find(entry=>entry.path===path);
      if(entry){workingReadErrors.delete(path);storeEntry(entry);}
      else unavailable(path,batch.errors.find(error=>error.path===path)?.error||'Retained metadata was not returned');
    }
  }catch(error){managed.forEach(path=>unavailable(path,String(error)));}
}
async function mutate(command: string, args: Record<string,unknown>, message: string) {
  const targets=paths();
  await run(`${message}…`,async()=>{
    let batch:Batch;
    try{batch=await invoke<Batch>(command,{paths:targets,...args});}
    catch(error){await reconcileWorkingEntries(targets);throw error;}
    accept(batch,message);
    await reconcileWorkingEntries(batch.errors.map(error=>error.path));
  });
}
async function saveDrafts() {
  await run('Saving tags…',async()=>{
    let successes=0;const failures:FileError[]=[];
    for(const path of paths()) { const updates=drafts.get(path);if(!updates)continue;const batch=await invoke<Batch>('save_fields',{paths:[path],updates});batch.entries.forEach(e=>{storeEntry(e);successes++;});reconcileDrafts(drafts,batch.entries.map(entry=>entry.path),true);batch.entries.forEach(entry=>{copyOverrides.delete(entry.path);workingReadErrors.delete(entry.path);});failures.push(...batch.errors); }
    errors=failures;status=`Saved tags: ${successes} file${successes===1?'':'s'}${failures.length?`, ${failures.length} failed`:''}`;
  });
}
function modal(title: string, content: string, buttons: string): HTMLDialogElement {
  const dialog=document.createElement('dialog');dialog.innerHTML=`<header>${esc(title)}</header><div class="dialog-content">${content}</div><div class="dialog-footer"><button data-close>Cancel</button>${buttons}</div>`;
  document.body.append(dialog);dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());dialog.addEventListener('close',()=>dialog.remove());dialog.showModal();return dialog;
}
function modalError(dialog: HTMLDialogElement, error: unknown) {let el=dialog.querySelector('.dialog-error');if(!el){el=document.createElement('p');el.className='dialog-error';el.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(el);}el.textContent=publicMessage(error);}
function value(dialog: HTMLDialogElement, id: string) {return dialog.querySelector<HTMLInputElement|HTMLSelectElement>(`#${id}`)!.value;}
function saveCopy() {
  const path=paths()[0],source=documentAccess.states[path];if(!source)return;
  const dialog=modal('Save a copy',`<p><strong>${esc(source.name)}</strong></p><p>Create a new document in a location you choose. The source and its recovery copies are retained.</p>${workingMetadataNotice(path)}<label class="field"><span>COPY VERSION</span><select id="copy-version"><option value="edited">Edited copy — includes pending tag changes</option><option value="original">Original backup — last verified source</option></select></label><p class="copy-status" role="status">${source.state==='recovery_required'?'Recovery required. Choose the edited copy or original backup.':'The edited copy includes the current tags and any pending changes.'}</p>`,`<button class="primary" id="save-document-copy">Choose destination…</button>`);
  dialog.setAttribute('aria-label','Save a copy');
  const saveButton=dialog.querySelector<HTMLButtonElement>('#save-document-copy')!,closeButton=dialog.querySelector<HTMLButtonElement>('[data-close]')!;
  let copying=false;
  dialog.addEventListener('cancel',event=>{if(copying)event.preventDefault();});
  saveButton.addEventListener('click',async()=>{
    if(copying||busy)return;
    const request=copyRequest(path,value(dialog,'copy-version') as 'edited'|'original',editedCopyFields(path));
    copying=true;saveButton.disabled=true;closeButton.disabled=true;dialog.querySelector<HTMLSelectElement>('#copy-version')!.disabled=true;
    dialog.querySelector('.dialog-error')?.remove();
    const progress=dialog.querySelector<HTMLElement>('.copy-status')!;progress.textContent='Choose a destination. The copy will be checked after writing.';
    const result=await run('Saving a copy…',async()=>{
      const batch=await invoke<Batch>('document_save_copy',request),outcome=copyOutcome(batch);
      if(outcome==='cancelled'){status='Copy cancelled. No copy was saved; pending edits are retained.';progress.textContent=status;return batch;}
      if(outcome==='failed'){errors=batch.errors;status='Copy could not be saved. Pending edits and recovery files are retained.';modalError(dialog,batch.errors.map(error=>`${displayName(error.path)}: ${publicMessage(error.error)}`).join('\n'));return batch;}
      accept(batch,'Saved copy',false);selected.clear();batch.entries.forEach(entry=>selected.add(entry.path));anchor=batch.entries[0].path;
      location='Provider documents';dialog.close();return batch;
    });
    copying=false;
    if(dialog.open){saveButton.disabled=false;closeButton.disabled=false;dialog.querySelector<HTMLSelectElement>('#copy-version')!.disabled=false;if(!result){progress.textContent='Copy failed. Pending edits and recovery files are retained.';modalError(dialog,errors.map(error=>publicMessage(error.error)).join('\n'));}}
  });
}
function extended() {
  const all=paths().map(p=>merged(entries.get(p)!));
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
  selectionTagRows(all).forEach(({name,values,mixed})=>add(name,values,mixed));
  rows.querySelector<HTMLInputElement>('input')?.focus({preventScroll:true});
  dialog.querySelector('.dialog-content')!.scrollTop=0;
  dialog.querySelector('#add-field')!.addEventListener('click',()=>add());
  dialog.querySelector('#apply-extended')!.addEventListener('click',()=>{
    const edits:TagRowEdit[]=[...rows.querySelectorAll<HTMLDivElement>('.extended-row')].map(row=>({original:row.dataset.original||'',name:fieldNameIdentity(row.querySelector('input')!.value.trim()),values:[...row.querySelectorAll<HTMLTextAreaElement>('.extended-value textarea')].map(textarea=>textarea.dataset.edited==='true'?textarea.value:originalValues.get(textarea)??textarea.value),changed:row.dataset.changed==='true',valueChanged:row.dataset.valueChanged==='true',removed:row.dataset.removed==='true'}));
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
      if(rename){const result=await invoke<RenamePreview[]>('preview_rename',{paths:targets,template,columns:columnSpecs(targets)});if(!dialog.open||revision!==previewRevision)return;const token=result[0]?.plan_token;if(!token||result.length!==targets.length||result.some((row,index)=>row.path!==targets[index]||row.plan_token!==token))throw new Error('The rename preview could not be verified. Run Preview again.');dialog.querySelector('#preview')!.innerHTML=`<div class="preview">${result.map(r=>`<div><small>${esc(displayName(r.path))}</small>${esc(r.value)}</div>`).join('')}</div>`;planToken=token;}
      else{const result=await invoke<FilenamePreview[]>('preview_filename',{paths:targets,pattern:template});if(!dialog.open||revision!==previewRevision)return;dialog.querySelector('#preview')!.innerHTML=`<div class="preview">${result.map(r=>`<div><small>${esc(displayName(r.path))}</small>${Object.entries(r.fields).map(([key,values])=>`${esc(key)}: ${esc(values.join(' · '))}`).join('<br>')}</div>`).join('')}</div>`;}
      dialog.querySelector('.dialog-error')?.remove();previewed=template;previewPaths=targets;apply.disabled=value(dialog,'template')!==template||JSON.stringify(paths())!==JSON.stringify(targets)||(rename&&!planToken);
    }catch(e){modalError(dialog,e);}finally{previewButton.disabled=false;}
  });
  apply.addEventListener('click',async()=>{const template=value(dialog,'template');if(!template.trim()){modalError(dialog,'Enter a pattern.');return;}if(template!==previewed||JSON.stringify(paths())!==JSON.stringify(previewPaths)||(rename&&!planToken)){apply.disabled=true;modalError(dialog,'Run Preview again before applying this conversion.');return;}const reviewedPaths=[...previewPaths],reviewedToken=planToken;dialog.close();if(rename){await run('Renaming files…',async()=>{const batch=await invoke<Batch>('rename_files',{paths:reviewedPaths,template,planToken:reviewedToken});applyRenamed(batch);accept(batch,'Renamed');});}else await mutate('filename_to_tags',{pattern:template},'Converted filenames to tags');});
}
function reviewedConverter(kind:'filename'|'text') {
  type Request = FilenameConversionRequest | TextImportRequest;
  type Item = FilenameConversionItem | TextImportItem;
  const rename=kind==='filename',title=rename?'Filename → Filename':'Text File → Tag';
  const content=rename?`<p>Rearrange numbered parts using %1 through %9. The original audio extension is retained. Preview the destinations before renaming your files.</p>
    <label class="field"><span>OLD FILENAME PATTERN</span><input id="old-pattern" value="%1 - %2" spellcheck="false" autocomplete="off"></label>
    <label class="field"><span>NEW FILENAME PATTERN</span><input id="new-pattern" value="%2 - %1" spellcheck="false" autocomplete="off"></label>`:
    `<p>Import one record per selected file in table order. Use %_filename_ext% or %_path% in the mask to match by file. Apply saves the reviewed tag values immediately.</p>
    <label class="field"><span>TEXT SOURCE</span><select id="text-source-kind"><option value="file">Text file</option><option value="text">Pasted text</option></select></label>
    <div id="text-file-source"><div class="field"><span>SOURCE FILE</span><div class="action-input"><input id="text-source-path" aria-label="Text source file" placeholder="Choose a text file" autocomplete="off" spellcheck="false"><button id="pick-text-source" type="button">Choose file…</button></div></div>
    <label class="field"><span>LEGACY ENCODING FALLBACK (OPTIONAL)</span><input id="text-codepage" placeholder="e.g. windows-1252" list="text-codepages" spellcheck="false"><datalist id="text-codepages"><option value="windows-1252"><option value="windows-1251"><option value="cp932"><option value="cp936"><option value="cp949"><option value="cp950"></datalist></label>
    <p>UTF-8 and BOM-marked UTF-16 are detected first. The fallback is used only when the file is not valid UTF-8.</p></div>
    <label class="field" id="text-paste-source" hidden><span>RECORDS TO IMPORT</span><textarea id="text-source-records" rows="5" spellcheck="false"></textarea></label>
    <label class="field"><span>IMPORT MASK</span><input id="text-mask" value="%title%;%artist%" autocomplete="off" spellcheck="false"></label>
    <label class="action-check"><input id="text-case-sensitive" type="checkbox" checked> Match filename and path keys case-sensitively</label>`;
  const dialog=modal(title,`${content}<p class="converter-status" role="status" aria-live="polite">Run Preview to review every selected file.</p><div id="reviewed-preview" aria-label="Conversion preview"></div>`,
    `<button id="reviewed-preview-button">Preview</button><button class="primary" id="reviewed-apply" disabled></button>`);
  dialog.setAttribute('aria-label',title);
  const apply=dialog.querySelector<HTMLButtonElement>('#reviewed-apply')!,previewButton=dialog.querySelector<HTMLButtonElement>('#reviewed-preview-button')!;
  const previewArea=dialog.querySelector<HTMLDivElement>('#reviewed-preview')!,progress=dialog.querySelector<HTMLElement>('.converter-status')!;
  const closeButton=dialog.querySelector<HTMLButtonElement>('[data-close]')!;
  const review=createConverterReview<Request,Item>((request,items)=>rename?
    validateFilenameItems(request as FilenameConversionRequest,items as FilenameConversionItem[]):
    validateTextItems(request as TextImportRequest,items as TextImportItem[]));
  const inputControls=[...dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLTextAreaElement>('input,select,textarea')];
  const inputValues=()=>inputControls.map(control=>control instanceof HTMLInputElement&&control.type==='checkbox'?control.checked:control.value);
  const inputsChanged=createConverterInputTracker(inputValues());
  let applying=false,picking=false,expiryTimer=0;
  const context=()=>JSON.stringify({revision:dataRevision,paths:paths(),dirty:dirtySelected(),compact:compactScreen(),busy});
  let workspaceContext=context();
  const readRequest=():Request=>{
    if(!paths().length)throw new Error('Select at least one audio file.');
    if(dirtySelected())throw new Error('Save pending tag changes before converting files.');
    const targets=paths();
    if(rename){
      const old_pattern=value(dialog,'old-pattern'),new_pattern=value(dialog,'new-pattern');
      if(!old_pattern.trim()||!new_pattern.trim())throw new Error('Enter both filename patterns.');
      return{paths:targets,old_pattern,new_pattern,columns:columnSpecs(targets)};
    }
    const mask=value(dialog,'text-mask');
    if(!mask.trim())throw new Error('Enter an import mask.');
    const file=value(dialog,'text-source-kind')==='file',path=value(dialog,'text-source-path');
    if(file&&!path.trim())throw new Error('Choose a source text file.');
    return{paths:targets,source:file?{kind:'file',path}:{kind:'text',text:value(dialog,'text-source-records')},mask,
      options:{ansi_codepage:file?(value(dialog,'text-codepage').trim()||null):null,case_sensitive_paths:dialog.querySelector<HTMLInputElement>('#text-case-sensitive')!.checked}};
  };
  const syncControls=()=>{
    const count=paths().length,blocked=applying||picking||busy||dirtySelected()||!count;
    apply.textContent=`${rename?'Rename':'Save tags to'} ${count} file${count===1?'':'s'}`;
    let ready=false,inputsValid=false;
    try{const request=readRequest();inputsValid=true;ready=review.canApply(request,context());}catch{/* Missing inputs remain visible and can be completed. */}
    apply.disabled=blocked||!ready;previewButton.disabled=blocked||review.pending||!inputsValid;
    previewButton.textContent=review.pending?'Previewing…':'Preview';
    closeButton.disabled=applying;
    dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLTextAreaElement>('input,select,textarea').forEach(control=>control.disabled=applying);
    const picker=dialog.querySelector<HTMLButtonElement>('#pick-text-source');if(picker)picker.disabled=applying||picking||busy;
  };
  const invalidate=(message='Inputs changed. Run Preview again.')=>{
    review.invalidate();window.clearTimeout(expiryTimer);previewArea.replaceChildren();
    progress.textContent=message;dialog.querySelector('.dialog-error')?.remove();syncControls();
  };
  const observeWorkspace=()=>{
    const next=context();
    if(next!==workspaceContext){workspaceContext=next;if(applying){review.invalidate();syncControls();}else invalidate(busy?'Wait for the current file operation, then run Preview again.':'Files or selection changed. Run Preview again.');}
    else syncControls();
  };
  reviewedConverterContextChanged=observeWorkspace;
  dialog.addEventListener('close',()=>{
    review.invalidate();window.clearTimeout(expiryTimer);
    if(reviewedConverterContextChanged===observeWorkspace)reviewedConverterContextChanged=undefined;
    if(!document.querySelector('dialog[open]'))[...app.querySelectorAll<HTMLElement>('.menubar > .menu > summary')].find(summary=>summary.textContent==='Convert')?.focus({preventScroll:true});
  });
  dialog.addEventListener('cancel',event=>{if(applying)event.preventDefault();});
  inputControls.forEach(control=>{
    const edited=()=>{
      if(!inputsChanged(inputValues()))return;
      if(!rename){const file=value(dialog,'text-source-kind')==='file';dialog.querySelector<HTMLElement>('#text-file-source')!.hidden=!file;dialog.querySelector<HTMLElement>('#text-paste-source')!.hidden=file;}
      invalidate();
    };
    control.addEventListener('input',edited);control.addEventListener('change',edited);
  });
  dialog.querySelector('#pick-text-source')?.addEventListener('click',async()=>{
    invalidate('Choose a text source, then run Preview.');picking=true;syncControls();
    try{const source=await open({multiple:false,title:'Choose text import source'});if(dialog.open&&typeof source==='string'){dialog.querySelector<HTMLInputElement>('#text-source-path')!.value=source;inputsChanged(inputValues());}}
    catch(error){if(dialog.open)modalError(dialog,error);}
    finally{picking=false;if(dialog.open){syncControls();dialog.querySelector<HTMLInputElement>('#text-source-path')!.focus({preventScroll:true});}}
  });
  const fieldRows=(fields:Fields)=>`<dl class="converter-values">${Object.entries(fields).map(([name,values])=>`<dt>${esc(name)}</dt><dd>${values.length?values.map((value,index)=>`<span>${values.length>1?`${index+1}. `:''}${value===''?'<em>(empty)</em>':esc(value)}</span>`).join(''):'<em>No values</em>'}</dd>`).join('')}</dl>`;
  const previewMarkup=(items:Item[])=>`<div class="preview" tabindex="0" role="region" aria-label="Reviewed conversion results">${items.map(item=>'target' in item?
    `<div><small>${esc(item.path)}</small><strong>${esc(item.target)}</strong>${fieldRows(item.captures)}</div>`:
    `<div><small>Source line ${item.source_line} → selected file ${item.selection_index+1}</small><strong>${esc(item.path)}</strong>${fieldRows(item.fields)}</div>`).join('')}</div>`;
  previewButton.addEventListener('click',async()=>{
    let attempt:ReturnType<typeof review.begin>|undefined;
    previewButton.focus({preventScroll:true});
    const restoreFocus=document.activeElement===previewButton;
    try{
      inputsChanged(inputValues()); // Capture also acknowledges AX edits with deferred change events.
      const request=readRequest();attempt=review.begin(request,context());
      window.clearTimeout(expiryTimer);previewArea.replaceChildren();dialog.querySelector('.dialog-error')?.remove();progress.textContent='Reading files and preparing preview…';syncControls();
      const response=await invoke<ConverterPreview<Item>>(rename?'preview_filename_conversion':'preview_text_import',{request:attempt.request});
      if(!dialog.open)return;
      const markup=previewMarkup(response.items);
      if(!review.complete(attempt,response,readRequest(),context())){review.fail(attempt);return;}
      previewArea.innerHTML=markup;progress.textContent=`Reviewed ${response.items.length} file${response.items.length===1?'':'s'}. Apply uses these exact results.`;
      expiryTimer=window.setTimeout(()=>{if(dialog.open)invalidate('The preview expired. Run Preview again.');},300_000);
    }catch(error){if(dialog.open&&(!attempt||review.fail(attempt))){modalError(dialog,error);progress.textContent='Preview failed. Review the error and try again.';}}
    finally{if(dialog.open){syncControls();if(restoreFocus&&(document.activeElement===dialog||document.activeElement===document.body))previewButton.focus({preventScroll:true});}}
  });
  apply.addEventListener('click',async()=>{
    let reviewed:ReturnType<typeof review.take>;
    apply.focus({preventScroll:true});
    try{if(busy)throw new Error('Wait for the current file operation to finish.');reviewed=review.take(readRequest(),context());}
    catch(error){invalidate('Run Preview again before applying.');modalError(dialog,error);return;}
    applying=true;window.clearTimeout(expiryTimer);progress.textContent=rename?'Renaming reviewed files…':'Saving reviewed tag values…';syncControls();
    let failure:string|undefined;
    const result=await run(rename?'Renaming files…':'Importing text tags…',async()=>{
      try{
        const batch=await invoke<Batch>(rename?'apply_filename_conversion':'apply_text_import',{request:reviewed.request,planToken:reviewed.planToken});
        applyRenamed(batch);accept(batch,rename?'Renamed':'Imported text tags',false);return batch;
      }catch(error){
        failure=String(error);
        try{accept(await invoke<Batch>('scan_files',{paths:reviewed.request.paths,recursive:false}),'Reloaded reviewed source files',false);}
        catch(reloadError){errors=[{path:'Reload',error:String(reloadError)}];}
        errors=[{path:'Conversion',error:failure},...errors];status='Conversion failed. Review file errors and run Preview again.';return undefined;
      }
    });
    applying=false;
    if(!dialog.open)return;
    if(result&&!result.errors.length){dialog.close();return;}
    invalidate('Apply did not complete. Review the errors and run Preview again.');
    modalError(dialog,failure||result?.errors.map(error=>`${error.path}: ${error.error}`).join('\n')||'Conversion did not complete. Run Preview again.');
    previewButton.focus({preventScroll:true});
  });
  syncControls();dialog.querySelector<HTMLInputElement|HTMLSelectElement>(rename?'#old-pattern':'#text-source-kind')!.focus({preventScroll:true});
}
function numbering() {
  const dialog=modal('Auto-numbering',`<p>Number the selected files in their current table order.</p><div class="field-pair"><label class="field"><span>START AT</span><input id="number-start" type="number" min="1" value="1"></label><label class="field"><span>MINIMUM DIGITS</span><input id="number-digits" type="number" min="1" max="6" value="2"></label></div><label><input type="checkbox" id="number-total"> Include total tracks (01/12)</label>`,`<button class="primary" id="apply-number">Number ${selected.size} files</button>`);
  dialog.querySelector('#apply-number')!.addEventListener('click',()=>{const start=Number(value(dialog,'number-start')),digits=Number(value(dialog,'number-digits'));if(!Number.isInteger(start)||start<1||!Number.isInteger(digits)||digits<1||digits>6){modalError(dialog,'Enter a positive start number and 1–6 digits.');return;}const list=ordered().filter(e=>selected.has(e.path)),total=dialog.querySelector<HTMLInputElement>('#number-total')!.checked;list.forEach((entry,i)=>updateDraft(entry.path,panelKey({format:entry.format,fields:merged(entry)},'TRACK'),[String(start+i).padStart(digits,'0')+(total?`/${list.length}`:'')]));status=`Track numbers are pending for ${list.length} files. Save tags to write them.`;dialog.close();render();});
}
async function operateImpl(op: Operation) {
  if(!enabled(op))return;
  if(op==='about'){const dialog=modal('About OpenTag',`<img class="about-artwork" data-theme-image="about" src="${themedAsset('about')}" alt="OpenTag — Tag, Organize, Enjoy"><p>Tag, Organize, Enjoy.</p><p>Open source music tag editor.</p><p>Version ${esc(appVersion)} · MIT License</p>`,'');dialog.querySelector('[data-close]')!.textContent='Close';return;}
  if(op==='refresh-source'){await run('Refreshing source access…',async()=>{},{preserveErrors:true});status=documentAccess.refreshError?'Source access refresh failed. Pending edits are retained.':'Source access refreshed';render();return;}
  if(op==='save-copy'){saveCopy();return;}
  if(op==='files'&&runtimePlatform.document_access_supported){await run('Choosing audio documents…',async()=>{const batch=await invoke<Batch>('pick_document_files');if(!batch.entries.length&&!batch.errors.length){status='No files added';return;}accept(batch,'Loaded',false);location='Provider documents';if(batch.entries.length){selected.clear();batch.entries.forEach(entry=>selected.add(entry.path));anchor=batch.entries[0].path;}});return;}
  if(op==='folder'||op==='files'){const result=await open(op==='folder'?{directory:true,multiple:true,title:'Open audio folder'}:{multiple:true,title:'Add audio files'});if(result)await scan(Array.isArray(result)?result:[result]);return;}
  if(op==='save'){await saveDrafts();return;}
  if(op==='select-all'){selected.clear();visible().forEach(e=>selected.add(e.path));anchor=paths()[0];coverIndex=0;render();return;}
  if(op==='discard'){if(await ask('Discard the pending tag changes for the selected files?',{title:'Discard changes',kind:'warning'})){paths().forEach(discardDrafts);changedData();status='Pending changes discarded';render();}return;}
  if(op==='reload'){if(dirtySelected()&&!await ask('Reload tags from the source and discard pending changes for the selected files?',{title:'Reload tags',kind:'warning'}))return;await run('Reloading tags…',async()=>accept(await invoke<Batch>('scan_files',{paths:paths(),recursive:false}),'Reloaded'));return;}
  if(op==='extended'){extended();return;}
  if(op==='filename-tags'||op==='tags-filename'){converter(op==='tags-filename');return;}
  if(op==='filename-filename'||op==='text-tags'){reviewedConverter(op==='filename-filename'?'filename':'text');return;}
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
const menubar = installMenubar(document);
document.addEventListener('keydown',event=>{
  if(event.defaultPrevented||document.querySelector('dialog[open]'))return;
  const typing=event.target instanceof HTMLInputElement||event.target instanceof HTMLTextAreaElement;
  if(event.altKey&&(event.code==='Digit6'||event.key==='6')){event.preventDefault();menubar.close();void operate(event.shiftKey?'action':'groups');return;}
  if(event.altKey&&(event.code==='KeyA'||event.key.toLowerCase()==='a')&&!typing){event.preventDefault();const menu=document.querySelector<HTMLDetailsElement>('#actions-menu')!;menubar.open(menu);menu.querySelector('summary')?.focus();actionUI.beginMnemonic();return;}
  if(!typing&&document.querySelector<HTMLDetailsElement>('#actions-menu')?.open&&event.key.trim().length===1&&!event.metaKey&&!event.ctrlKey){event.preventDefault();void actionUI.mnemonic(event.key);return;}
  if(event.key==='Escape'){document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);}
  if(event.metaKey||event.ctrlKey){if(event.key.toLowerCase()==='s'){event.preventDefault();void operate('save');}if(event.key.toLowerCase()==='o'){event.preventDefault();void operate(runtimePlatform.document_access_supported?'files':'folder');}if(event.key.toLowerCase()==='a'&&!typing){event.preventDefault();void operate('select-all');}}
});
window.addEventListener('beforeunload',event=>{if(drafts.size||actionUI.hasUnsavedChanges){event.preventDefault();event.returnValue='';}});
render();
let resizeFrame=0;
window.addEventListener('resize',()=>{if(!resizeFrame)resizeFrame=requestAnimationFrame(()=>{
  resizeFrame=0;
  const nextRowHeight=compactScreen()?44:30;
  // IME and orientation resizes within a mode must keep editor/menu nodes intact.
  if(nextRowHeight===rowHeight){updateVirtualRows();return;}
  rowHeight=nextRowHeight;render();
});});
if(desktop){
  void getVersion().then(version=>{appVersion=version;}).catch(()=>{appVersion='unknown';});
  void actionUI.initialize().catch(error=>{errors=[{path:'Action groups',error:String(error)}];render();});
  void invoke<typeof runtimePlatform>('runtime_platform').then(async platform=>{runtimePlatform=platform;nativeAppearanceReady=platform.family==='desktop';render();syncNativeAppearance();if(platform.document_access_supported)await run('Restoring recovery copies…',async()=>{const batch=await invoke<Batch>('restore_document_recovery');accept(batch,'Restored recovery copies',false);location='Provider documents';if(!selected.size&&batch.entries.length){selected.add(batch.entries[0].path);anchor=batch.entries[0].path;}if(!batch.entries.length&&!batch.errors.length)status='Ready';});}).catch(error=>{errors=[{path:'Platform',error:String(error)}];render();});
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
    if(event.payload.type==='drop'&&!busy&&!runtimePlatform.document_access_supported)void scan(event.payload.paths);
  }).catch(error=>{errors=[{path:'Drag and drop',error:String(error)}];render();});
}
