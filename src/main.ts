/// <reference types="vite/client" />
import { invoke as coreInvoke, isTauri, type InvokeArgs } from '@tauri-apps/api/core';
import { buildFilterFields } from './filter-fields';
import { documentSelection, refreshDocumentAccess, copyRequest, copyOutcome, reconcileDrafts, sourcePresentation, publicDocumentMessage, type DocumentAccess, type DocumentState } from './document-state';
import { createConverterInputTracker, createConverterReview, validateFilenameItems, validateTextItems, type FilenameConversionRequest, type TextImportRequest, type FilenameConversionItem, type TextImportItem, type ConverterPreview } from './converter-review';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { createAppearance, type AppearanceState } from './theme';
import { installMenubar } from './menubar';
import { ExportUI, type ExportLibrary, type ExportConfiguration } from './export-ui';
import { OptionsUI, registerOptionsPage, naturalCollator, windowTitle, type Settings, type SettingsResponse } from './options-ui';
import { createNumberingReview, numberingCounter, type NumberingRequest, type NumberingUpdate } from './numbering-review';
import { open, save, ask, message } from '@tauri-apps/plugin-dialog';
import { Messages, messagesOptionsPage, coverChoice, type MessageKey } from './messages';
import { appearanceOptions, completeInput, generalOptions, insideFolders, networkingOptionsPage, releaseNotesUrl, showReleaseNotes, updateMessage, updateOptions, updatesOptionsPage, type UpdateCheck } from './general-options';
import { ToolsUI, toolsOptionsPage, toolShortcut, type Tool } from './tools';
import { RowContextMenu, type ContextItem } from './row-context-menu';
import { removeTagTypesDialog, mp4ChapterFiles, stripChaptersPrompt, copyAsTextDialog, redoOrRepeat, menuShortcuts, EXTRA_SHORTCUTS, shortcutsDialog, CONFIGURATION_FILTER, configurationFileName, UTILS, utilMessage, utilTargets, type RemovableTag, type Util } from './utils-ui';
import { fieldNameIdentity, resolveTagEdits, selectionTagRows, type TagRowEdit } from './tag-edits';
import { TagSourcesUI, tagSourcesOptionsPages, type Provider } from './tag-sources';
import { ActionUI, panelKey, rebaseCaseOnly, resolvePathChanges, type ActionGroup, type ActionLibrary, type ImportedGroup } from './action-ui';
import { ColumnUI, ColumnValueCache, columnFields, defaultLayout, editKey, headerMarkup, normalizeLayout, quickValue, sortClick, sortRows, sortTemplate, updateSortHeader, visibleColumns, type ColumnLayout } from './columns';
import { FilterSync, filterCacheMiss } from './filter-sync';
import { CellEditor, adjacentCell, type CellTarget } from './cell-editor';
import { invertSelection, listShortcut, listSummary, nextFile, nextSimilarFiles, similarFiles } from './list-commands';
import { FindUI } from './find';
import { TagPanel, type PanelLayout, type TagPanelSettings } from './tag-panel';
import { editShortcut, emptyHistory, historyLabel, type HistoryState } from './edit-history';
import { clipboardText, copyTags, cutPlan, pastePlan, type PasteUpdate, type TagClipboard } from './tag-clipboard';
import { openTagToTag } from './tag-tag-ui';
import { installHelperMenus } from './helper-menu';
import { FileOpsUI, fileShortcut, startupChoice, startupOptionsPage, type FileOperation } from './file-ops';
import { PlaylistUI, isPlaylistPath, playlistOptionsPage, exportOptionsPage, playlistValues, type PlaylistPlan } from './playlist-ui';
import { listen } from '@tauri-apps/api/event';
import { CoverUI } from './covers';
import { imageExtensions, type CoverInfo } from './cover-model';
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

type PerfRecord={name:string;at:number;ms:number;pending?:boolean;in?:number;out?:number;[key:string]:unknown};
// Profiling scripts (tests/e2e/performance.js) create this array to collect timings; normal runs never do.
const perfSink=()=>(window as Window&{__OPENTAG_TIMINGS__?:PerfRecord[]}).__OPENTAG_TIMINGS__;
function invoke<T>(command:string,args?:InvokeArgs):Promise<T>{
  const sink=perfSink();if(!sink)return coreInvoke<T>(command,args);
  const record:PerfRecord={name:`ipc ${command}`,at:performance.now(),ms:0,pending:true,in:JSON.stringify(args??{}).length};sink.push(record);
  return coreInvoke<T>(command,args).then(result=>{record.out=JSON.stringify(result??null)?.length??0;return result;}).finally(()=>{record.ms=performance.now()-record.at;record.pending=false;});
}
type Fields = Record<string, string[]>;
type Cover = CoverInfo;
type Entry = { path: string; filename: string; format: string; duration: number; bitrate: number; sample_rate: number; channels: number; size: number; fields: Fields; technical?: Fields; covers: Cover[] };
type FileError = { path: string; error: string };
type Batch = { entries: Entry[]; errors: FileError[]; renamed?: { from: string; to: string }[]; moved_directories?: {from:string;to:string}[]; outputs?: string[] };
type Preview = { path: string; value: string };
type RenamePreview = Preview & {plan_token:string};
type FilenamePreview = { path: string; fields: Fields };
type Operation = 'folder'|'files'|'save'|'save-copy'|'refresh-source'|'reload'|'extended'|'add-cover'|'remove-cover'|'filename-tags'|'tags-filename'|'filename-filename'|'text-tags'|'action'|'groups'|'number'|'remove-tags'|'playlist-load'|'playlist-all'|'playlist-selected'|'playlist-generate'|'export-folders'|'list-unsorted'|'csv'|'export-config'|'select-all'|'discard'|'options'|'about'|'toggle-panel'|'panel-options'|'field-values'|'panel-defaults'|'undo'|'redo'|'copy-tags'|'cut-tags'|'paste-tags'|'tag-tag'|'source-musicbrainz'|'source-freedb'|'source-local-freedb'|'source-cover-musicbrainz'|'source-cover-discogs'|'source-discogs'|'source-token'|'source-options'|'cover-manage'|'copy-text'|'remove-tag-types'|'strip-mp4-chapters'|'tools-options'|'config-save'|'config-load'|'config-folder'|'help-shortcuts'|'help-docs'|'help-issues'|'help-releases'|'help-updates'|UtilOperation|ListOperation|FileOperation;
type UtilOperation='util-unsupported'|'util-optimize';
const utilOperations:Record<UtilOperation,Util>={'util-unsupported':'remove_unsupported_fields','util-optimize':'optimize'};
const tagSourceOperations:Operation[]=['source-musicbrainz','source-discogs','source-freedb','source-local-freedb','source-cover-musicbrainz','source-cover-discogs'];
type ListOperation='unselect-all'|'invert-selection'|'select-next'|'select-similar'|'select-next-similar'|'remove-from-list'|'clear-list'|'refresh'|'find'|'edit-cell'|'columns';
const listOperations:Operation[]=['unselect-all','invert-selection','select-next','select-similar','select-next-similar','remove-from-list','clear-list','refresh','find','edit-cell','columns'];
const glyphs:Partial<Record<Operation,string>>={folder:glyphImport,files:glyphMusic,save:glyphSave,'save-copy':glyphSave,'refresh-source':glyphOrganize,reload:glyphOrganize,extended:glyphEdit,'add-cover':glyphAlbum,'remove-cover':glyphEdit,'filename-tags':glyphImport,'tags-filename':glyphExport,'filename-filename':glyphOrganize,'text-tags':glyphImport,action:glyphBatch,groups:glyphSettings,number:glyphOrganize,'remove-tags':glyphEdit,'playlist-all':glyphOrganize,csv:glyphExport,options:glyphSettings,'select-all':glyphCheck,discard:glyphEdit,about:glyphInfo,undo:glyphEdit,redo:glyphEdit,'cut-tags':glyphEdit,'copy-tags':glyphEdit,'paste-tags':glyphEdit,'tag-tag':glyphEdit};
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
// The tag panel width, set with the splitter and remembered on this device.
const PANEL_WIDTH:[number,number]=[200,640];
function setPanelWidth(width:number,save=false){
  const clamped=Math.round(Math.max(PANEL_WIDTH[0],Math.min(PANEL_WIDTH[1],width)));
  document.documentElement.style.setProperty('--panel-width',`${clamped}px`);
  if(save)try{localStorage.setItem('opentag.panel-width',String(clamped));}catch{/* The width applies to this session only. */}
  return clamped;
}
try{const saved=Number(localStorage.getItem('opentag.panel-width'));if(saved)setPanelWidth(saved);}catch{/* Default width. */}
const app = document.querySelector<HTMLDivElement>('#app')!;
const entries = new Map<string, Entry>();
const drafts = new Map<string, Fields>();
// Recovery working bytes can differ from loaded tags. Preserve explicit resets
// for edited copies separately from pending drafts, including absent-field tombstones.
const copyOverrides = new Map<string, Fields>();
const workingReadErrors = new Map<string, string>();
const selected = new Set<string>();
let columnLayout:ColumnLayout=defaultLayout(),listRoots:string[]=[],slowEditTimer=0;
let filter = '', anchor: string | undefined, busy = false, status = 'Ready', errors: FileError[] = [], location = 'No folder loaded', coverIndex = 0;
const esc = (value: unknown) => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const basename = (path: string) => path.split(/[\\/]/).pop() || path;
const paths = () => ordered().filter(entry=>selected.has(entry.path)).map(entry=>entry.path);
const merged = (entry: Entry): Fields => ({ ...entry.fields, ...copyOverrides.get(entry.path), ...drafts.get(entry.path) });
const field = (entry: Entry, key: string) => (merged(entry)[panelKey({format:entry.format,fields:merged(entry)},key)] || []).join(' \\\\ ');
const length = (seconds: number) => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2,'0')}`;
const dirtySelected = () => [...selected].some(path => drafts.has(path));
const active = () => entries.get(selected.values().next().value||'');
let collator = new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
const dev = (import.meta as ImportMeta & {env?: {DEV?: boolean}}).env?.DEV === true;
const compactScreen=()=>window.matchMedia('(max-width:1000px)').matches;
const tableColumns=()=>{const shown=visibleColumns(columnLayout);return compactScreen()?shown.slice(0,3):shown;};
let rowHeight = compactScreen()?44:30;
const headerHeight = 29, overscan = 8;
let mobilePane:'files'|'tags'='files';
let tableScrollTop=0,tableScrollLeft=0;
let editorScrollTop=0;
let runtimePlatform={family:'desktop' as 'desktop'|'android'|'ios',folder_picker_supported:true,document_access_supported:false};
let documentAccess:DocumentAccess={states:{},refreshError:null};
let dataRevision = 0, orderCacheKey = '', filterCacheKey = '', orderCache: Entry[] = [], filterCache: Entry[] = [];
let reviewedConverterContextChanged: (() => void) | undefined;
let history:HistoryState=emptyHistory,tagClipboard:TagClipboard|undefined;
let virtualStart = -1, virtualEnd = -1, virtualRevision = -1, virtualSelection = '', virtualViewport = -1;
const searchCache = new Map<string,string>();
let searchRevision = -1;
let filterMatch:{context:string;query:string;paths:Set<string>}|undefined;
let filterRequestKey='',filterPending=false,filterFailure='',filterTimer=0;
type ProgressJob={id:string;done:number;total:number;cancelling:boolean};
let progressJob:ProgressJob|undefined;
// With Auto-apply filter off, typing edits a draft that Return applies.
let filterAutoApply=true,filterDraft:string|undefined;
try{filterAutoApply=localStorage.getItem('opentag.filter-auto-apply')!=='false';}catch{/* Defaults to filtering as you type. */}
const actionUI=new ActionUI({
  load:()=>invoke<ActionLibrary>('load_action_library'),
  save:library=>invoke<void>('save_action_library',{library}),
  import:(path,legacyEncoding)=>invoke<ImportedGroup>('import_action_group',{path,legacyEncoding:legacyEncoding||null}),
  execute:executeGroups,
  pickFile:async kind=>{const extensions=kind==='mta'?['mta']:kind==='image'?['jpg','jpeg','png','webp','gif']:kind==='export'?['mte']:['txt','lrc'];const path=await open({multiple:false,title:kind==='mta'?'Import action group':kind==='image'?'Choose cover image':kind==='export'?'Choose export configuration':'Choose text file',filters:[{name:kind==='mta'?'Action groups':kind==='image'?'Images':kind==='export'?'Export configurations':'Text files',extensions}]});return typeof path==='string'?path:null;},
  confirm:message=>ask(message,{title:'Action groups',kind:'warning'}),
  canExecute:()=>enabled('action'),executeReason:()=>operationReason('action'),selectedCount:()=>selected.size,fieldNames:()=>[...new Set(paths().flatMap(path=>Object.keys(merged(entries.get(path)!))))],changed:()=>render(),
  exportGroup:async group=>{const path=await save({title:'Export action group',defaultPath:`${group.name.replace(/[\\/:*?"<>|]/g,'_')}.mta`,filters:[{name:'Action groups',extensions:['mta']}]});if(!path)return null;await invoke('export_action_group',{path,group});return path;},
});
const exportUI=new ExportUI({
  load:()=>invoke<ExportLibrary>('load_export_library'),save:library=>invoke<void>('save_export_library',{library}),
  import:(path,encoding)=>invoke<ExportConfiguration>('import_export_configuration',{path,legacyEncoding:encoding||null}),
  pickTemplate:async()=>{const path=await open({multiple:false,title:'Import export configuration',filters:[{name:'Export configurations',extensions:['mte']}]});return typeof path==='string'?path:null;},
  pickOutput:current=>save({title:'Export file name',defaultPath:current||'export.txt'}),
  selectedCount:()=>selected.size,
  reason:()=>busy?'Wait for the current operation.':!selected.size?'Select files to export.':operationReason('csv'),
  execute:async(library,perDirectory)=>{
    const targets=paths(),columns=columnSpecs(targets);
    const result=await run('Exporting metadata…',async()=>{
      await invoke('save_export_library',{library});
      const batch=await invoke<Batch>('run_export',{paths:targets,columns,perDirectory});errors=batch.errors;status=batch.errors.length?'Export incomplete. Review the file errors.':`Exported metadata for ${batch.entries.length} files${batch.outputs&&batch.outputs.length>1?` to ${batch.outputs.length} files`:''}`;return batch;
    });
    if(!result||result.errors.length)throw new Error(result?.errors.map(e=>`${e.path}: ${e.error}`).join('\n')||errors.map(e=>e.error).join('\n')||'Export did not complete.');
    await offerOpen(result.outputs??[]);
  },
});
const macPlatform=/Mac|iPhone|iPad/.test(navigator.platform);
// After exporting or creating a playlist, Options → Messages may offer to open the written file.
async function offerOpen(outputs:string[]){
  if(outputs.length===1&&messages.enabled('confirm_export')&&await ask(`${status}. Open ${basename(outputs[0])} now?`,{title:'Export',kind:'info'}))await invoke('play_files',{paths:outputs});
}
let loadedPlaylist:string|undefined,autoPlaylistPending=false;
const playlistUI=new PlaylistUI({
  invoke:(command,args)=>invoke(command,args),values:()=>optionsUI.page('playlist'),remember:values=>optionsUI.updatePage('playlist',values),
  files:scope=>scope==='all'?visible().map(entry=>entry.path):paths(),
  workingDirectory:()=>loadedPlaylist?dirname(loadedPlaylist):fileOps.history.current,loadedPlaylist:()=>loadedPlaylist,columns:targets=>columnSpecs(targets),
  modal:(title,content,buttons)=>modal(title,content,buttons),modalError:(dialog,error)=>modalError(dialog,error),run:(text,job)=>run(text,job),
  done:async(plan:PlaylistPlan,interactive)=>{
    const files=plan.playlists.reduce((sum,p)=>sum+p.files,0);
    status=plan.playlists.length===1?`Created playlist ${basename(plan.playlists[0].path)} with ${files} file${files===1?'':'s'}`:`Created ${plan.playlists.length} playlists with ${files} files`;
    errors=plan.errors;render();if(interactive)await offerOpen(plan.playlists.map(p=>p.path));
  },
});
registerOptionsPage(exportOptionsPage);
registerOptionsPage(playlistOptionsPage);
const coverUI=new CoverUI({
  invoke:(command,args)=>invoke(command,args),mac:macPlatform,
  selection:()=>paths().map(path=>({path,covers:workingReadErrors.has(path)?[]:entries.get(path)!.covers})),
  displayed:()=>({path:active()?.path,covers:coversFor(active()),index:coverIndex}),
  show:index=>{coverIndex=index;refreshCoverPanel();},
  reason:()=>busy?'Wait for the current operation.':!selected.size?'Select files first.':!desktop?'Cover editing requires the desktop app.':operationReason('add-cover'),
  clipboard:()=>desktop&&runtimePlatform.family==='desktop',
  mutate:(command,args,message)=>mutate(command,args,message),
  pickImage:async directory=>{const path=await open({multiple:false,title:'Choose cover image',defaultPath:directory,filters:[{name:'Images',extensions:imageExtensions()}]});return typeof path==='string'?path:null;},
  pickSave:(defaultPath,extension)=>save({title:'Extract cover',defaultPath,filters:[{name:'Images',extensions:[extension]}]}),
  confirm:(message,title)=>ask(message,{title,kind:'warning'}),
  modal:(title,content,buttons)=>modal(title,content,buttons),modalError:(dialog,error)=>modalError(dialog,error),
  evaluate:async(path,template)=>(await invoke<Preview[]>('preview_format',{paths:[path],template,columns:columnSpecs([path])}))[0]?.value??'',
  adjust:()=>actionUI.quick('adjust_cover'),
  notify:message=>{status=message;const el=document.querySelector('.status-message');if(el)el.textContent=message;},
  fail:error=>{errors=[{path:'Cover',error:error instanceof Error?error.message:String(error)}];status='Cover operation failed';render();},
});
const fileOps=new FileOpsUI({
  invoke:(command,args)=>invoke(command,args),
  pickDirectories:async(title,multiple)=>{const result=await open({directory:true,multiple,title});return result?Array.isArray(result)?result:[result]:[];},
  confirm:(text,title)=>title==='Delete files'?messages.confirm('confirm_delete_files',text,title):title==='Unsaved changes'?messages.confirm('unsaved_changes',text,title):ask(text,{title,kind:'warning'}),
  selection:()=>paths().map(path=>({path,name:displayName(path)})),pendingCount:()=>drafts.size,
  load:(targets,options)=>loadPaths(targets,options),removeEntries,
  moved:(result,message)=>{const batch=result as Batch;applyRenamed(batch);accept(batch,'Moved',false);status=message;},
  run:(message,job)=>run(message,job),report:(failures,message)=>{errors=failures;status=message;render();},
  openReason:()=>busy?'Wait for the current operation.':runtimePlatform.folder_picker_supported?'':'Folder access is not available here. Use Add files.',
  changed:()=>render(),mac:macPlatform,
  preferences:()=>({subfolders:optionsUI.settings.directories.subfolders,startup:startupChoice(optionsUI.settings.pages['directories-startup']?.startup)}),
});
registerOptionsPage(startupOptionsPage);
const dirname=(path:string)=>path.replace(/[\\/][^\\/]*$/,'')||path;
const tagPanel=new TagPanel({
  persistent:()=>desktop,
  load:()=>invoke<TagPanelSettings>('load_tag_panel'),save:settings=>invoke<void>('save_tag_panel',{settings}),
  importLayout:path=>invoke<PanelLayout>('import_tag_panel_layout',{path}),exportLayout:(path,panel)=>invoke<void>('export_tag_panel_layout',{path,panel}),
  importList:path=>invoke<string[]>('import_value_list',{path}),exportList:(path,values)=>invoke<void>('export_value_list',{path,values}),
  pickOpen:async kind=>{const path=await open({multiple:false,title:kind==='layout'?'Load tag panel configuration':'Import custom values',filters:[kind==='layout'?{name:'Tag panel configurations',extensions:['json']}:{name:'Text files',extensions:['txt']}]});return typeof path==='string'?path:null;},
  pickSave:(kind,name)=>save({title:kind==='layout'?'Save tag panel configuration':'Export custom values',defaultPath:name,filters:[kind==='layout'?{name:'Tag panel configurations',extensions:['json']}:{name:'Text files',extensions:['txt']}]}),
  values:key=>paths().map(path=>field(entries.get(path)!,key)),disabled:()=>!selected.size||busy,
  directories:()=>{const local=(list:Entry[])=>list.filter(entry=>!providerEntry(entry)).map(entry=>dirname(entry.path));return{selected:local(paths().map(path=>entries.get(path)!)),all:local([...entries.values()])};},
  selectDirectory:directory=>{selected.clear();visible().filter(entry=>!providerEntry(entry)&&dirname(entry.path)===directory).forEach(entry=>selected.add(entry.path));anchor=paths()[0];coverIndex=0;render();},
  openFolder:()=>void operate('folder'),draft:draftField,keep:keepField,changed:()=>render(),
  notify:message=>{status=message;const el=document.querySelector('.status-message');if(el)el.textContent=message;},
  fail:error=>{errors=[{path:'Tag panel',error:error instanceof Error?error.message:String(error)}];status='Operation failed';render();},
  autocomplete:()=>generalPrefs().autocomplete?completeInput:undefined,
});
const optionsUI=new OptionsUI({
  load:()=>invoke<SettingsResponse>('load_settings'),save:settings=>invoke<SettingsResponse>('save_settings',{settings}),
  appearance:{get:()=>appearance.state.preference,set:value=>appearance.set(value)},applied:optionsApplied,
});
registerOptionsPage(messagesOptionsPage);
registerOptionsPage(networkingOptionsPage);
registerOptionsPage(updatesOptionsPage(()=>checkForUpdates(false)));
const generalPrefs=()=>generalOptions(optionsUI.page('general'));
registerOptionsPage({id:'tag-panel',label:'Tag Panel',order:55,description:'Fields, order, sizes and defaults of the tag panel. Changes are saved by the editor itself.',controls:[{kind:'button',id:'customize-tag-panel',label:'Customize tag panel…',run:()=>tagPanel.configure()}]});
registerOptionsPage({id:'field-values',label:'Field Values',order:56,description:'Custom values per field for the tag panel drop-down lists, including user-defined genres. Changes are saved by the editor itself.',controls:[{kind:'button',id:'edit-field-values',label:'Edit field values…',run:()=>tagPanel.fieldValues()}]});
registerOptionsPage(toolsOptionsPage({pickProgram:async()=>{const path=await open({multiple:false,directory:false,title:'Choose program'});return typeof path==='string'?path:null;}}));
const messages=new Messages({values:()=>optionsUI.page('messages'),ask:(text,title)=>ask(text,{title,kind:'warning'}),tell:async(text,title)=>{await message(text,{title,kind:'info'});}});
let queuedNotices:[MessageKey,string][]=[];
const noticeLabels:Record<string,MessageKey>={'Removed tags':'after_delete','Removed tag types':'after_delete','Removed unsupported fields':'after_delete','Applied action groups':'after_change','Tag → Tag':'after_change','Renamed':'after_change','Imported text tags':'after_change','Converted filenames to tags':'after_change'};
async function flushNotices(){const pending=queuedNotices;queuedNotices=[];for(const [key,text] of pending)await messages.notify(key,text,'OpenTag');
  await autoPlaylist();}
// Options → Playlist → Create playlist automatically, after saving tags or renaming files.
async function autoPlaylist(){
  if(!autoPlaylistPending)return;autoPlaylistPending=false;
  if(desktop&&entries.size&&playlistValues(optionsUI.page('playlist')).auto)await playlistUI.create('all',true).catch(error=>{errors=[{path:'Playlist',error:publicMessage(error)}];status='Automatic playlist failed';render();});
}
let lastActionGroups:ActionGroup[]|undefined;
const repeatState=()=>redoOrRepeat(history,enabled('action')?lastActionGroups:undefined);
const toolsUI=new ToolsUI({
  values:()=>optionsUI.page('tools'),mac:macPlatform,
  run:(tool:Tool,targets)=>invoke<{started:number}>('run_tool',{tool,paths:targets,root:listRoots.length===1?listRoots[0]:null}),
  selection:()=>paths(),reason:()=>!desktop?'Tools require the desktop app.':!selected.size?'Select files first.':operationReason('file-play'),
  confirm:(text,title)=>ask(text,{title,kind:'warning'}),
  report:text=>{status=text;render();},fail:error=>{errors=[{path:'Tools',error:publicMessage(error)}];status='Tool failed';render();},
});
const rowMenu=new RowContextMenu({
  prepare:path=>{if(!selected.has(path)){selected.clear();selected.add(path);anchor=path;coverIndex=0;coverError=undefined;render();}},
  items:()=>{
    const item=(op:Operation,label:string):ContextItem=>({label,id:`op:${op}`,disabled:!enabled(op),title:operationReason(op)||label});
    const tools=toolsUI.tools,reason=toolsUI.reason;
    return[item('extended','Extended tags…'),item('copy-text','Copy as Text…'),item('remove-tags','Remove tags…'),{separator:true},{heading:'Tools'},
      ...(tools.length?tools.map((tool,index):ContextItem=>({label:tool.name,id:`tool:${index}`,disabled:!!reason,title:reason||tool.path,shortcut:toolShortcut(index,macPlatform)})):[item('tools-options','Configure tools…')]),
      {separator:true},{heading:'Utils'},item('remove-tag-types','Remove tag types…'),item('strip-mp4-chapters','Strip chapters MP4…'),item('util-unsupported','Remove unsupported fields…'),item('util-optimize','Optimize…'),{separator:true},
      item('remove-from-list','Remove from list'),item('file-play','Play'),item('file-reveal',`Open in ${fileOps.fileManager}`)];
  },
  activate:id=>{if(id.startsWith('tool:'))void toolsUI.run(Number(id.slice(5)));else void operate(id.slice(3) as Operation);},
});
let shownTitle='';
function syncWindowTitle(){const title=windowTitle(location,optionsUI.settings);if(desktop&&title!==shownTitle){shownTitle=title;void getCurrentWindow().setTitle(title).catch(()=>{shownTitle='';});}}
// Read preferences change which tag is displayed, so reload files without pending edits.
async function optionsApplied(next:Settings,previous:Settings){
  collator=naturalCollator(next);changedData();applyAppearancePrefs();
  const clean=JSON.stringify(next.tags)===JSON.stringify(previous.tags)?[]:[...entries.values()].filter(entry=>!drafts.has(entry.path)&&!providerEntry(entry)).map(entry=>entry.path);
  if(clean.length)await run('Applying tag options…',async()=>accept(await invoke<Batch>('scan_files',{paths:clean,recursive:false}),'Options saved. Reloaded',false));
  else{status=drafts.size&&JSON.stringify(next.tags)!==JSON.stringify(previous.tags)?'Options saved. Reload files with pending changes to apply tag options.':'Options saved';render();}
}
// Options → Appearance: toolbar and interface scaling and the file list grid.
let appliedZoom=1;
function applyAppearancePrefs(){
  const prefs=appearanceOptions(optionsUI.page('appearance')),root=document.documentElement;
  root.dataset.grid=prefs.grid;root.style.setProperty('--toolbar-scale',String(prefs.toolbarScale));
  if(desktop&&prefs.uiScale!==appliedZoom){appliedZoom=prefs.uiScale;void getCurrentWebview().setZoom(prefs.uiScale).catch(error=>{appliedZoom=1;errors=[{path:'Appearance',error:String(error)}];render();});}
}
// Options → Messages → at importing covers: whether files that already have artwork keep it.
async function preserveCovers(targets:string[]):Promise<boolean|undefined>{
  const count=targets.filter(path=>(entries.get(path)?.covers.length??0)>0).length;
  if(!count)return false;
  if(!messages.enabled('confirm_import_covers'))return coverChoice(optionsUI.page('messages'))==='preserve';
  return new Promise(resolve=>{
    const dialog=modal('Import cover art',`<p>${count} of the ${targets.length} file${targets.length===1?'':'s'} already ${count===1?'has':'have'} embedded cover art. Keep the existing cover art in ${count===1?'it':'them'}? Files without cover art receive the imported cover either way.</p><label class="action-check"><input type="checkbox" id="covers-remember"> Don't show this message again</label>`,'<button id="covers-replace">Use the imported cover</button><button id="covers-keep" class="primary">Keep existing cover art</button>');
    dialog.classList.add('import-covers-dialog');
    let answer:boolean|undefined;
    const choose=(preserve:boolean)=>{
      answer=preserve;
      if(dialog.querySelector<HTMLInputElement>('#covers-remember')!.checked)void optionsUI.updatePage('messages',{confirm_import_covers:false,import_covers_choice:preserve?'preserve':'replace'}).catch(error=>{errors=[{path:'Options',error:String(error)}];render();});
      dialog.close();
    };
    dialog.querySelector('#covers-keep')!.addEventListener('click',()=>choose(true));
    dialog.querySelector('#covers-replace')!.addEventListener('click',()=>choose(false));
    dialog.addEventListener('close',()=>resolve(answer));
    dialog.querySelector<HTMLButtonElement>('#covers-keep')!.focus();
  });
}
for(const page of tagSourcesOptionsPages(async()=>{const folder=await open({directory:true,multiple:false,title:'Choose the local freedb folder'});return typeof folder==='string'?folder:null;}))registerOptionsPage(page);
const tagSourcesUI=new TagSourcesUI({
  invoke:(command,args)=>invoke(command,args),
  files:()=>paths().map(path=>{const entry=entries.get(path)!;return{path,name:displayName(path),title:field(entry,'TITLE'),artist:field(entry,'ARTIST'),album:field(entry,'ALBUM'),album_artist:field(entry,'ALBUMARTIST'),track:field(entry,'TRACK'),disc:field(entry,'DISCNUMBER'),length:entry.duration};}),
  reason:()=>busy?'Wait for the current operation.':!selected.size?'Select the files of one album.':operationReason('source-musicbrainz'),
  options:()=>optionsUI.page('tag-sources'),
  pickSave:name=>save({title:'Save cover',defaultPath:name,filters:[{name:'Images',extensions:['jpg','jpeg','png','gif','webp','bmp']}]}),
  apply:async(items,chosenCover)=>{
    const targets=items.map(item=>item.path);let cover=chosenCover;
    if(cover){const preserve=await preserveCovers(targets);if(preserve===undefined)throw new Error('Import cancelled. Nothing was written.');cover={...cover,preserve_existing:preserve};}
    const result=await run('Writing tag source data…',async()=>{let batch:Batch;try{batch=await invoke<Batch>('tag_source_apply',{items,cover});}catch(error){await reconcileWorkingEntries(targets);throw error;}accept(batch,'Imported from tag source');await reconcileWorkingEntries(batch.errors.map(error=>error.path));return batch;});
    if(!result)throw new Error(errors.map(error=>publicMessage(error.error)).join('\n')||'Tag source data could not be written.');
    if(result.errors.length)throw new Error(result.errors.map(error=>`${displayName(error.path)}: ${publicMessage(error.error)}`).join('\n'));
  },
});
async function executeGroups(groups:ActionGroup[]){
  if(!enabled('action'))throw new Error(operationReason('action')||'Select files and save pending tag edits before applying actions.');
  const targets=paths();lastActionGroups=groups;
  const result=await run('Applying action groups…',async()=>{const batch=await invoke<Batch>('execute_action_groups',{paths:targets,groups,columns:columnSpecs(targets)});applyRenamed(batch);accept(batch,'Applied action groups');return batch;});
  // Without "at warnings from Actions", errors stay in the file error list only.
  if(!messages.enabled('action_warnings'))return;
  if(!result)throw new Error(errors.map(error=>`${error.path}: ${error.error}`).join('\n')||'Action groups could not be applied.');
  if(result.errors.length)throw new Error(result.errors.map(error=>`${error.path}: ${error.error}`).join('\n'));
}
function columnSpecs(targets:string[]){
  return tableColumns().map(column=>({name:column.name,template:column.value,values:Object.fromEntries(targets.flatMap(path=>{const value=columnValues.fresh(path,column.value);return value===undefined?[]:[[path,value]];}))}));
}
type FilterOutcome={matches:string[];error:string|null};
let filterQueue:Promise<unknown>=Promise.resolve();
const filterSync=new FilterSync({
  paths:()=>ordered().map(entry=>entry.path),
  columns:()=>tableColumns().map(column=>({name:column.name,template:column.value})),
  token:(path,columns)=>[entries.get(path),drafts.get(path),copyOverrides.get(path),documentAccess.states[path],...columns.map(column=>columnValues.fresh(path,column.template))],
  file:(path,columns)=>{const entry=entries.get(path)!;return{path,fields:filterFields(entry),raw_fields:merged(entry),columns:columns.map(column=>columnValues.fresh(path,column.template)??null)};},
});
function columnItem(entry:Entry){return{fields:columnFields({...filterFields(entry),_FILETYPE:[entry.format]}),raw_fields:columnFields(merged(entry))};}
const columnValues=new ColumnValueCache({
  token:path=>{const entry=entries.get(path);return entry?[entry,drafts.get(path),copyOverrides.get(path),documentAccess.states[path]]:undefined;},
  // Sort values of every shown column are evaluated with the cells, so sorting by a script-evaluated column (Length, Bitrate) needs no new evaluation.
  templates:()=>[...tableColumns().flatMap(column=>[column.value,sortTemplate(column)]),...columnLayout.sort.flatMap(key=>{const column=columnLayout.columns.find(c=>c.id===key.id);return column?[sortTemplate(column)]:[];})],
  evaluate:async(targets,templates)=>{
    const items=targets.map(path=>columnItem(entries.get(path)!));
    if(!desktop)return items.map(item=>templates.map(template=>quickValue(template,item.fields)??''));
    const started=performance.now(),result=await invoke<{values:string[][];errors:string[]}>('evaluate_columns',{items,templates});
    timing('column values',started,{files:targets.length,templates:templates.length});
    if(result.errors.length){status=`Column value error: ${result.errors[0]}`;const element=document.querySelector('.status-message');if(element)element.textContent=status;}
    return result.values;
  },
  updated:()=>{if(document.querySelector('.table-wrap'))updateVirtualRows(true);},
  failed:error=>{status=`Column values could not be evaluated: ${String(error)}`;},
});
function cellValue(entry:Entry,template:string){return columnValues.get(entry.path,template)??quickValue(template,columnItem(entry).fields)??'';}
const cellEditor=new CellEditor({
  cell:target=>{const cell=revealRow(target.path,false)?.querySelector<HTMLTableCellElement>(`td[data-col="${CSS.escape(target.column)}"]`)||null;cell?.scrollIntoView({block:'nearest',inline:'nearest'});return cell;},
  initial:({path,column})=>{const entry=entries.get(path),key=editKey(columnLayout.columns.find(c=>c.id===column)||{field:''});return!entry?'':key==='_FILENAME_EXT'?entry.filename:key==='_FILENAME'?entry.filename.replace(/\.[^.]*$/,''):field(entry,key);},
  label:({column})=>columnLayout.columns.find(c=>c.id===column)?.name||'cell',
  commit:commitCell,
  next:(target,direction)=>{
    const rows=visible().map(entry=>entry.path),below=rows[rows.indexOf(target.path)+1];
    const next=direction==='down'?below===undefined?undefined:{path:below,column:target.column}:adjacentCell(rows,tableColumns().filter(column=>editKey(column)).map(column=>column.id),target,direction);
    if(next&&next.path!==target.path){selected.clear();selected.add(next.path);anchor=next.path;coverIndex=0;render();}
    return next;
  },
  ended:()=>{const path=anchor&&selected.has(anchor)?anchor:paths()[0];if(path&&!document.querySelector('dialog[open]'))document.querySelector<HTMLElement>(`tr[data-path="${CSS.escape(path)}"]`)?.focus({preventScroll:true});},
  advance:()=>generalPrefs().auto_select_next,
  complete:(input,event,target)=>{
    const key=editKey(columnLayout.columns.find(c=>c.id===target.column)||{field:''});
    if(generalPrefs().autocomplete&&key&&!key.startsWith('_'))completeInput(input,event,()=>cellCompletions(key));
  },
});
/** Values offered while editing a File List cell: custom field values, then the values of loaded files. */
function cellCompletions(key:string){
  const loaded=new Set<string>();for(const entry of entries.values())for(const value of merged(entry)[panelKey({format:entry.format,fields:merged(entry)},key)]||[])if(value)loaded.add(value);
  return [...tagPanel.settings.field_values.lists[key]||[],...[...loaded].sort(collator.compare)];
}
const columnUI=new ColumnUI({
  layout:()=>columnLayout,
  apply:applyColumns,
  importLayout:async()=>{const path=await open({multiple:false,title:'Load column configuration',filters:[{name:'Column configurations',extensions:['json']}]});return typeof path==='string'?normalizeLayout(await invoke<ColumnLayout>('import_columns',{path})):null;},
  exportLayout:async layout=>{const path=await save({title:'Save column configuration',defaultPath:'columns.json',filters:[{name:'Column configurations',extensions:['json']}]});if(!path)return false;await invoke('export_columns',{path,layout});status=`Saved column configuration to ${basename(path)}`;return true;},
  preview:async template=>{const entry=active();if(!entry||!desktop)return '';const result=await invoke<{values:string[][];errors:string[]}>('evaluate_columns',{items:[columnItem(entry)],templates:[template]});if(result.errors.length)throw new Error(result.errors[0]);return result.values[0][0];},
  report:error=>{errors=[{path:'Columns',error:error instanceof Error?error.message:String(error)}];render();},
});
const findUI=new FindUI({
  items:()=>visible().map(entry=>({path:entry.path,filename:entry.filename,fields:merged(entry)})),
  select:(targets,focus)=>{selected.clear();targets.forEach(path=>selected.add(path));anchor=focus;coverIndex=0;coverError=undefined;render();if(focus)revealRow(focus,false);status=`${targets.length} file${targets.length===1?'':'s'} selected from Find results`;document.querySelector('.status-message')!.textContent=status;},
  listed:path=>visible().some(entry=>entry.path===path),
});
/** Apply a column layout. Verified layouts (the Customize dialog) are saved first; quick header changes apply immediately. */
async function applyColumns(layout:ColumnLayout,verified=false){
  const next=normalizeLayout(layout),sortOnly=JSON.stringify(next.columns)===JSON.stringify(columnLayout.columns);
  if(verified&&desktop)await invoke('save_columns',{layout:next});
  columnLayout=next;if(sortOnly)refreshList();else render();
  if(!verified&&desktop)await invoke('save_columns',{layout:next});
}
function revealRow(path:string,focus=true){
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap'),index=visible().findIndex(entry=>entry.path===path);if(!wrap||index<0)return null;
  const top=index*rowHeight,viewport=wrap.clientHeight-headerHeight;
  if(top<wrap.scrollTop)wrap.scrollTop=top;else if(top+rowHeight>wrap.scrollTop+viewport)wrap.scrollTop=top+rowHeight-viewport;
  updateVirtualRows();
  const row=wrap.querySelector<HTMLTableRowElement>(`tr[data-path="${CSS.escape(path)}"]`);if(focus)row?.focus({preventScroll:true});return row;
}
function changedData(){dataRevision++;reviewedConverterContextChanged?.();}
function timing(name:string,start:number,extra:Record<string,unknown>={}){
  const ms=performance.now()-start;
  perfSink()?.push({name,at:start,ms,...extra});
  if(dev)console.debug(`[OpenTag performance] ${name}`,{milliseconds:Math.round(ms*100)/100,files:entries.size,...extra});
}
function ordered(): Entry[] {
  const key=`${dataRevision}:${columnValues.revision}:${JSON.stringify(columnLayout.sort)}`;
  if(orderCacheKey===key)return orderCache;
  const start=performance.now();
  const criteria=columnLayout.sort.flatMap(sort=>{const column=columnLayout.columns.find(c=>c.id===sort.id);return column?[{sort,column,template:sortTemplate(column)}]:[];});
  orderCache=sortRows([...entries.values()].map(entry=>({item:entry,keys:criteria.map(({template})=>cellValue(entry,template))})),criteria.map(({sort,column})=>({ascending:sort.ascending,numeric:column.numeric})),collator);orderCacheKey=key;
  timing('sort',start,{criteria:criteria.length});return orderCache;
}
function sortValueOf(path:string){
  const entry=entries.get(path);if(!entry)return '';
  const criteria=columnLayout.sort.flatMap(sort=>{const column=columnLayout.columns.find(c=>c.id===sort.id);return column?[sortTemplate(column)]:[];});
  return criteria.length?criteria.map(template=>cellValue(entry,template).toLocaleLowerCase()).join('\u0000'):entry.filename.toLocaleLowerCase();
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
    const started=performance.now();
    // A failed request may not have reached the cache; a cache miss is retried with every file.
    const send=(retry:boolean):Promise<FilterOutcome>=>invoke<FilterOutcome>('filter_matches',{query,request:filterSync.request()})
      .catch(error=>{filterSync.reset();if(retry&&String(error).includes(filterCacheMiss))return send(false);throw error;});
    // Requests run one at a time, so the backend applies cache updates in the order they were made.
    const result=filterQueue.then(()=>filterRequestKey===key?send(true):undefined);filterQueue=result.catch(()=>{});
    void result.then(outcome=>{
      if(!outcome)return;
      const {matches,error}=outcome;if(error!==null)throw error;
      if(filterRequestKey!==key||filter!==query||filterContextKey()!==context)return;
      filterMatch={context,query,paths:new Set(matches)};filterPending=false;filterCacheKey='';filterFailure='';
      timing('filter expression',started,{query,matched:matches.length});refreshList();
    }).catch(error=>{if(filterRequestKey===key&&filter===query&&filterContextKey()===context){filterPending=false;filterFailure=String(error);filterCacheKey='';refreshList();}});
  },120);
}
function filterContextKey(){return`${dataRevision}:${JSON.stringify(columnLayout.sort)}:${compactScreen()}`;}
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
  const entry=active(),error=coverError&&coverError.path===entry?.path?coverError.message:'';
  return coverUI.markup({fallback:themedAsset('artwork'),error,unavailable:!!entry&&workingReadErrors.has(entry.path),buttons:button('add-cover','Add…')+button('remove-cover','Remove')});
}
function setCoverImage(){coverUI.paint();}
function refreshCoverPanel(){
  const panel=document.querySelector('.cover-section');if(!panel){coverUI.updateViewer();return;}panel.innerHTML=coverMarkup();
  setCoverImage();coverUI.bind(panel);
  panel.querySelectorAll<HTMLButtonElement>('[data-op]').forEach(el=>el.addEventListener('click',()=>void operate(el.dataset.op as Operation)));
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
  const draft=drafts.get(entry.path),fields=draft&&merged(entry);
  return `<tr tabindex="0" data-path="${esc(entry.path)}" aria-rowindex="${index+2}" aria-selected="${selected.has(entry.path)}" class="${selected.has(entry.path)?'selected':''}${index%2?' odd':''}">${tableColumns().map((c,i)=>{const value=cellValue(entry,c.value),key=editKey(c),pending=!!(draft&&key&&draft[panelKey({format:entry.format,fields:fields!},key)]);return`<td data-col="${esc(c.id)}" class="align-${c.align}${i===0?' filename':''}${pending?' pending':''}${key?' editable':''}" title="${esc(value)}">${i===0?`<input class="row-select" type="checkbox" aria-label="Select ${esc(entry.filename)}" ${selected.has(entry.path)?'checked':''}>${draft?'<span class="rowmarker" title="Pending changes">●</span>':''}`:''}${esc(value)}</td>`;}).join('')}</tr>`;
}
function updateVirtualRows(force=false){
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap'),body=wrap?.querySelector('tbody');if(!wrap||!body)return;
  const startTime=performance.now(),list=visible(),viewport=Math.max(rowHeight,wrap.clientHeight-headerHeight);
  const first=Math.floor(wrap.scrollTop/rowHeight),last=Math.min(list.length,first+Math.ceil(viewport/rowHeight));
  // Keep the rendered window while the viewport stays inside it, so scrolling or arrowing row by row rebuilds the table only every few rows.
  const keep=!force&&virtualRevision===dataRevision&&virtualViewport===viewport&&first>=virtualStart&&last<=virtualEnd&&virtualEnd<=list.length;
  const start=keep?virtualStart:Math.max(0,Math.min(list.length,first-overscan)),end=keep?virtualEnd:Math.min(list.length,last+overscan);
  const selectionKey=paths().join('\0');
  if(!force&&start===virtualStart&&end===virtualEnd&&virtualRevision===dataRevision){
    if(virtualSelection===selectionKey)return;
    // Only the selection changed: restyle the affected rows instead of rebuilding (and re-laying out) the table.
    body.querySelectorAll<HTMLTableRowElement>('tr[data-path]').forEach(row=>{
      const on=selected.has(row.dataset.path!);if(row.classList.contains('selected')===on)return;
      row.classList.toggle('selected',on);row.setAttribute('aria-selected',String(on));
      const box=row.querySelector<HTMLInputElement>('.row-select');if(box)box.checked=on;
    });
    virtualSelection=selectionKey;timing('table selection',startTime);return;
  }
  const focused=document.activeElement instanceof HTMLTableRowElement?document.activeElement.dataset.path:undefined;
  const spacer=(height:number)=>height?`<tr class="virtual-spacer" aria-hidden="true"><td colspan="${tableColumns().length}" style="height:${height}px"></td></tr>`:'';
  body.innerHTML=spacer(start*rowHeight)+list.slice(start,end).map((entry,i)=>rowMarkup(entry,start+i)).join('')+spacer((list.length-end)*rowHeight);
  virtualStart=start;virtualEnd=end;virtualRevision=dataRevision;virtualSelection=selectionKey;virtualViewport=viewport;
  if(focused)wrap.querySelector<HTMLTableRowElement>(`tr[data-path="${CSS.escape(focused)}"]`)?.focus({preventScroll:true});
  timing('table window',startTime,{rendered:end-start,first:start,last:end});
}
function operationReason(op:Operation):string {
  const policy=sourcePolicy();
  if((op==='folder'||op==='playlist-load'||op.startsWith('dir-'))&&!runtimePlatform.folder_picker_supported)return 'Folder access is not available here. Use Add files.';
  if(op.startsWith('file-')&&runtimePlatform.family!=='desktop')return 'File operations are available in the desktop app.';
  if(op==='save-copy')return policy.copyReason;
  if((['action','tags-filename','filename-filename','text-tags','tag-tag','playlist-all','playlist-selected','playlist-generate','csv'].includes(op)||op.startsWith('file-'))&&policy.unsupportedReason)return policy.unsupportedReason;
  if((op==='undo'||op==='redo')&&runtimePlatform.document_access_supported)return 'Undo is not available for provider documents.';
  if((['save','add-cover','remove-cover','cover-manage','remove-tags','remove-tag-types','strip-mp4-chapters','filename-tags',...Object.keys(utilOperations)].includes(op)||tagSourceOperations.includes(op))&&policy.mutationReason)return policy.mutationReason;
  if(op==='strip-mp4-chapters'&&!mp4ChapterFiles(paths().map(path=>entries.get(path)??{})).files)return 'No selected MP4 file has chapters.';
  if(op==='reload'&&policy.reloadReason)return policy.reloadReason;
  if(op==='number'&&dirtySelected()&&!sourcePolicy().unsupportedReason)return 'Save pending changes first.';
  if(/^(dir|file)-/.test(op)&&fileOps.reason(op as FileOperation))return fileOps.reason(op as FileOperation);
  if(dirtySelected()&&!/^(dir-|file-reveal$|file-play$)/.test(op)&&!['save','save-copy','refresh-source','reload','extended','number','discard','folder','files','select-all','about','groups','toggle-panel','panel-options','field-values','panel-defaults','options','undo','redo','copy-tags','cut-tags','paste-tags','source-token','source-options','copy-text','tools-options','config-save','config-load','config-folder','help-shortcuts','help-docs','help-issues','help-releases','help-updates','list-unsorted',...listOperations].includes(op))return 'Save pending changes first.';
  return '';
}
function enabled(op: Operation): boolean {
  if (op === 'about' || op === 'toggle-panel' || op === 'help-shortcuts') return true;
  if (busy) return false;
  if (op === 'select-all') return visible().length > 0;
  if (op === 'discard') return dirtySelected();
  if (listOperations.includes(op)) return listEnabled(op);
  if (op === 'list-unsorted') return columnLayout.sort.length > 0;
  if (!desktop) return false;
  if(op==='groups'||op==='export-config'||op==='export-folders'||op==='panel-options'||op==='field-values'||op==='options'||op==='source-token'||op==='source-options'||op==='tools-options'||op.startsWith('config-')||op.startsWith('help-'))return true;
  if(op==='refresh-source')return runtimePlatform.document_access_supported;
  if(operationReason(op))return false;
  if (op === 'folder' || op === 'files' || op === 'playlist-load' || op.startsWith('dir-') || op === 'file-reveal') return true;
  if (op === 'playlist-all' || op === 'playlist-generate') return entries.size > 0;
  if (op === 'undo') return !!history.undo;
  if (op === 'redo') return repeatState().kind!=='none';
  if (op === 'paste-tags' && !tagClipboard) return false;
  if (!selected.size) return false;
  if (op === 'save') return dirtySelected();
  if (op === 'reload' || op === 'extended' || op === 'number' || op === 'save-copy') return true;
  if (op === 'remove-cover') return coverUI.canRemove();
  return true;
}
/** File List entries for the View menu: Refresh leads it and column customization follows the tag panel items, as in Mp3tag. */
const viewListItems:[Operation,string,string?][]=[['refresh','Refresh','F5'],['columns','Customize columns…'],['list-unsorted','Unsorted (load and playlist order)']];
function listEnabled(op:Operation){
  if(op==='columns')return true;
  if(op==='refresh')return desktop&&!runtimePlatform.document_access_supported&&listRoots.length>0;
  if(op==='clear-list'||op==='find')return entries.size>0;
  if(op==='invert-selection'||op==='select-next')return visible().length>0;
  if(op==='edit-cell')return selected.size>0&&tableColumns().some(column=>editKey(column));
  return selected.size>0;
}
function button(op: Operation, label: string, icon = '', shortcut = '') {
  return `<button data-op="${op}" ${enabled(op) ? '' : 'disabled'} title="${esc(operationReason(op)||label)}">${glyphs[op]?glyph(glyphs[op]!):icon?`<span class="icon" aria-hidden="true">${esc(icon)}</span>`:''}<span>${label}</span>${shortcut ? `<small>${shortcut}</small>` : ''}</button>`;
}
function menu(name: string, items: [Operation,string,string?][]) { return `<details class="menu" ${name==='Actions'?'id="actions-menu"':''}><summary>${name}</summary><div class="menu-list">${items.map(([op,label,key]) => button(op,label,'',key)+(op==='dir-manage'&&desktop&&runtimePlatform.folder_picker_supported?fileOps.menuHTML():'')).join('')}${name==='Actions'?actionUI.menuHTML():''}${name==='View'?columnUI.menuHTML():''}${name==='Tools'?toolsUI.menuHTML():''}${name==='File'&&!runtimePlatform.folder_picker_supported?'<p class="menu-note">Folder access is not available here. Use Add files.</p>':''}${['Convert','Actions'].includes(name)&&sourcePolicy().unsupportedReason?`<p class="menu-note">${esc(sourcePolicy().unsupportedReason)}</p>`:''}</div></details>`; }
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
  const selectedEntries = paths().map(path => entries.get(path)!), shownColumns = tableColumns();
  app.innerHTML = `<div class="app" data-pane="${mobilePane}" data-panel="${tagPanel.visible?'shown':'hidden'}"><header class="titlebar"><img class="brand-logo" data-theme-image="logo" src="${themedAsset('logo')}" alt="OpenTag"><span class="subtitle">Audio metadata editor</span><span class="window-mode">${runtimePlatform.document_access_supported?'Provider documents':desktop?'Local files':'Browser preview'}</span><label class="appearance-control"><span>Appearance</span><select id="appearance" aria-label="Appearance"><option value="system">${appearance.state.preference==='system'?`System (${appearance.state.resolved==='dark'?'Dark':'Light'})`:'System'}</option><option value="light">Light</option><option value="dark">Dark</option></select></label></header>
  <nav class="menubar" aria-label="Main menu">${menu('File',[['save','Save tags','⌘/Ctrl S'],['save-copy','Save a copy…'],['reload','Reload tags'],['playlist-all','Playlist (all files)','⌘/Ctrl P'],['playlist-selected','Playlist (selected files)'],['playlist-generate','Generate playlists…','⌘/Ctrl ⇧ P'],['export-config','Export…','⌘/Ctrl E'],['csv','Export CSV…'],['folder','Change directory…',runtimePlatform.document_access_supported?undefined:'⌘/Ctrl D'],['dir-add','Add directory…','Ctrl H'],['files','Add files…',runtimePlatform.document_access_supported?'⌘/Ctrl D':undefined],['playlist-load','Load playlist/cue sheet…'],['dir-up','Upper directory','⌘/Ctrl ⇧ D'],['dir-back','Previous directory','Alt ←'],['dir-forward','Next directory','Alt →'],['dir-favorite','Favorite directory','Ctrl M'],['dir-manage','Favorite directories…'],['clear-list','Clear list'],['file-reveal',`Open in ${fileOps.fileManager}`],['file-play','Play','F4'],['options','Options…','⌘/Ctrl O'],['config-save','Save configuration…'],['config-load','Load configuration…'],['config-folder','Open configuration folder']])}${menu('Edit',[['undo',esc(historyLabel('Undo',history.undo)),'⌘/Ctrl Z'],['redo',esc(repeatState().label),'⌘/Ctrl Y'],['cut-tags','Cut tags','⌘/Ctrl X'],['copy-tags','Copy tags','⌘/Ctrl C'],['copy-text','Copy as Text…'],['paste-tags','Paste tags','⌘/Ctrl V'],['select-all','Select all','⌘/Ctrl A'],['invert-selection','Invert selection','⌘/Ctrl ⇧A'],['unselect-all','Unselect all','⌘/Ctrl U'],['select-next','Select next file','⌘/Ctrl N'],['select-similar','Select similar files','⌘/Ctrl G'],['select-next-similar','Select next similar files','⌘/Ctrl ⇧G'],['find','Find…','⌘/Ctrl F'],['edit-cell','Edit in file list','F2'],['file-copy','Copy files to…','⌘/Ctrl ⇧ C'],['file-move','Move files to…','⌘/Ctrl ⇧ V'],['remove-from-list','Remove from list','Del'],['file-delete','Delete files…','⌘/Ctrl Del'],['extended','Extended tags…'],['discard','Discard pending changes'],['remove-tags','Remove tags…'],['cover-manage','Manage covers…',macPlatform?'⌘⌥M':'Ctrl Alt M']])}${menu('View',[viewListItems[0],['toggle-panel',tagPanel.visible?'Hide tag panel':'Show tag panel','Ctrl Q'],['panel-defaults','Default values in input fields'],['panel-options','Customize tag panel…'],['field-values','Field values…'],...viewListItems.slice(1)])}${menu('Convert',[['tags-filename','Tag → Filename…','Alt 1'],['filename-tags','Filename → Tag…','Alt 2'],['filename-filename','Filename → Filename…','Alt 3'],['text-tags','Text File → Tag…','Alt 4'],['tag-tag','Tag → Tag…','Alt 5'],['number','Auto-numbering…','⌘/Ctrl K']])}${menu('Actions',[['groups','Actions…','Alt 6'],['action','Actions (Quick)…','Alt Shift 6']])}${menu('Tag Sources',[['source-musicbrainz','MusicBrainz…',tagSourcesUI.lastProvider==='musicbrainz'?'⌘/Ctrl I':undefined],['source-discogs','Discogs…',tagSourcesUI.lastProvider==='discogs'?'⌘/Ctrl I':undefined],['source-freedb','freedb…'],['source-local-freedb','freedb (local)…','⌘/Ctrl L'],['source-cover-musicbrainz','Covers: MusicBrainz…'],['source-cover-discogs','Covers: Discogs…'],['source-token','Discogs access token…'],['source-options','Options…']])}${menu('Tools',[['remove-tag-types','Remove tag types…'],['strip-mp4-chapters','Strip chapters MP4…'],['util-unsupported','Remove unsupported fields…'],['util-optimize','Optimize…'],['tools-options','Configure tools…']])}${menu('Help',[['help-docs','Documentation'],['help-shortcuts','Keyboard shortcuts'],['help-issues','Report an issue'],['help-updates','Check for updates…'],['help-releases','Releases'],['about','About OpenTag']])}</nav>
  <div class="toolbar" role="toolbar" aria-label="File and tag operations">${button('folder','Open folder','▱')}${button('files','Add files','＋')}<span class="sep"></span>${button('save','Save tags','▣')}${button('reload','Reload','↻')}<span class="sep"></span>${button('filename-tags','Filename → Tag')}${button('tags-filename','Tag → Filename')}<span class="spacer"></span>${button('groups','Actions','⚙')}${button('number','Numbering','#')}</div>
  <nav class="mobile-panes" aria-label="Workspace view"><button data-pane="files" aria-pressed="${mobilePane==='files'}">Files (${entries.size})</button><button data-pane="tags" aria-pressed="${mobilePane==='tags'}" ${selected.size?'':'disabled'}>Tags (${selected.size})</button></nav>
  <main class="workspace"><aside class="editor" aria-label="Tag editor"><div class="panel-title">Tag panel <span class="panel-count">${panelCount()}</span>${tagPanel.titleHTML()}</div>${sourceMarkup('tags')}<div class="fields">${tagPanel.fieldsHTML()}</div><div class="editor-actions">${button('save','Save tags')}${button('extended','Extended…')}</div>
  ${tagPanel.showCover?`<section class="cover-section" aria-label="Embedded artwork">${coverMarkup()}</section>`:''}</aside><div class="panel-splitter" role="separator" aria-orientation="vertical" aria-label="Tag panel width" aria-valuemin="${PANEL_WIDTH[0]}" aria-valuemax="${PANEL_WIDTH[1]}" tabindex="0"></div>
  <section class="files" aria-label="Audio files"><div class="location">${glyph(glyphOrganize)}<span class="path" title="${esc(entry&&providerEntry(entry)?sourcePresentation(documentAccess.states[entry.path]).location:location)}">${esc(entry&&providerEntry(entry)?sourcePresentation(documentAccess.states[entry.path]).location:location)}</span><strong>${entries.size} file${entries.size===1?'':'s'}</strong></div>${sourceMarkup('files')}<div class="filterbar"><label for="filter">${glyph(glyphSearch)}<span class="visually-hidden">Filter</span></label><input id="filter" type="search" value="${esc(filterDraft??filter)}" data-helper="filter" data-history="filter" data-history-manage placeholder="Filter filenames and tags, or enter a filter expression" autocomplete="off"><span class="count">${countText(list)}</span></div><div class="table-wrap"><table aria-label="Audio metadata" aria-rowcount="${list.length+1}" style="width:${shownColumns.reduce((sum,c)=>sum+c.width,0)}px"><thead><tr>${headerMarkup(shownColumns,columnLayout.sort)}</tr></thead><tbody></tbody></table>${!list.length ? emptyMarkup() : ''}</div><div class="selection-info">${selectionInfoMarkup(selectedEntries,list)}</div>
  ${errors.length ? `<section class="errors" aria-label="Operation errors"><header>${errors.length} file error${errors.length===1?'':'s'}<button id="dismiss-errors">Dismiss</button></header>${errors.map(e => `<p><strong>${esc(displayName(e.path))}:</strong> ${esc(publicMessage(e.error))}</p>`).join('')}</section>` : ''}</section></main><footer class="status ${busy?'busy':''}"><span class="status-dot" aria-hidden="true"></span><span class="status-message" role="status">${esc(status)}</span>${progressMarkup()}${drafts.size ? `<span class="dirty-count">${drafts.size} unsaved file${drafts.size===1?'':'s'}</span>` : ''}<span>${entries.size} files</span></footer></div>`;
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap')!;
  wrap.querySelector('tbody')!.innerHTML=list.length?`<tr class="virtual-spacer" aria-hidden="true"><td colspan="${tableColumns().length}" style="height:${list.length*rowHeight}px"></td></tr>`:'';
  wrap.scrollTop=scrollTop;wrap.scrollLeft=scrollLeft;updateVirtualRows(true);
  document.querySelector('.fields')!.scrollTop=panelTop;
  document.querySelector<HTMLElement>('.editor')!.scrollTop=editorScrollTop;
  bind();setCoverImage();hydrateActiveCover();applyAppearance(appearance.state);
  columnUI.bindHeader(wrap.querySelector('table')!,()=>void operate('columns'));cellEditor.reattach();
  if(focusSelector){const replacement=document.querySelector<HTMLInputElement|HTMLTextAreaElement>(focusSelector);replacement?.focus({preventScroll:true});if(replacement&&caretStart!=null&&caretEnd!=null){try{replacement.setSelectionRange(caretStart,caretEnd);}catch{/* Search selection is not supported by every webview. */}}}
  syncWindowTitle();
  timing('workspace render',started,{rendered:virtualEnd-virtualStart,selected:selected.size});
}
function emptyMarkup(){
  return `<div class="empty"><img class="empty-symbol" data-theme-image="mark" src="${themedAsset('mark')}" alt="" aria-hidden="true"><h1>${filterPending ? 'Filtering files…' : filterFailure ? 'Invalid filter' : entries.size ? 'No matching files' : runtimePlatform.document_access_supported?'Add audio documents':desktop ? 'Open your music folder' : 'Your music, ready to edit'}</h1><p>${filterPending ? 'Matching your filter against filenames and tag values.' : filterFailure ? esc(filterFailure) : entries.size ? 'Try another filename or tag value in the filter.' : runtimePlatform.document_access_supported?'Choose audio files from your document provider. Source access and recovery options appear with your selection.':desktop ? 'Open a folder, add audio files, or drop them into this window. Select files to edit their tags together.' : 'Open the desktop app to work with local audio files. Your files and tags stay on your computer.'}</p>${!entries.size ? `<div class="empty-actions">${button('folder','Open folder…')}${button('files','Add files…')}</div>` : ''}</div>`;
}
function countText(list:Entry[]){return`${filterPending?'Filtering… · ':list.length===entries.size?'':`${list.length} visible · `}${selected.size} selected`;}
function panelCount(){return selected.size?`${selected.size} selected`:'No selection';}
function selectionInfoMarkup(selectedEntries:Entry[],list:Entry[]){return listSummary(selectedEntries,list,entries.size,selected.size===1?active():undefined).map(segment=>`<span>${esc(segment)}</span>`).join('');}
function bindFields(root:ParentNode){root.querySelectorAll<HTMLInputElement|HTMLTextAreaElement>('[data-field]').forEach(el=>el.addEventListener('input',()=>draftField(el.dataset.field!,el.value)));}
/**
 * Selection changes update only what depends on the selection: table rows, tag panel fields, cover,
 * counts and command states. A full render rebuilds every menu and forces layout of the whole window,
 * which made each click or arrow key cost about twice as much. Provider documents and the compact
 * layout show selection-dependent source details and pane tabs, so they keep the full render.
 */
function refreshSelection(){
  const fields=document.querySelector<HTMLElement>('.fields');
  if(!fields||runtimePlatform.document_access_supported||compactScreen()||selectedProvider()||document.querySelector('.source-access')){render();return;}
  const started=performance.now();
  reviewedConverterContextChanged?.();
  const focused=document.activeElement instanceof HTMLInputElement||document.activeElement instanceof HTMLTextAreaElement?document.activeElement:undefined;
  const focusField=focused&&fields.contains(focused)?focused.dataset.field:undefined;
  const list=visible(),panelTop=fields.scrollTop;
  updateVirtualRows();cellEditor.reattach();
  fields.innerHTML=tagPanel.fieldsHTML();tagPanel.bind(fields);bindFields(fields);if(panelTop)fields.scrollTop=panelTop;
  if(focusField)fields.querySelector<HTMLElement>(`[data-field="${CSS.escape(focusField)}"]`)?.focus({preventScroll:true});
  document.querySelector('.panel-count')!.textContent=panelCount();
  document.querySelector('.filterbar .count')!.textContent=countText(list);
  document.querySelector('.selection-info')!.innerHTML=selectionInfoMarkup(paths().map(path=>entries.get(path)!),list);
  refreshCommandStates();
  refreshCoverPanel();hydrateActiveCover();
  timing('selection refresh',started,{selected:selected.size});
}
/**
 * Sort and filter changes update only the list: header sort indicators, the virtual rows, the empty
 * state, counts and command states, like `refreshSelection`. The table, menus and filter input stay,
 * so typing keeps focus without a rebuild. Without files the empty state carries buttons, and
 * provider documents and the compact layout show list-dependent details, so those keep the full render.
 */
function refreshList(){
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap'),table=wrap?.querySelector('table');
  if(!wrap||!table||!entries.size||runtimePlatform.document_access_supported||compactScreen()||selectedProvider()||document.querySelector('.source-access')){render();return;}
  const started=performance.now();
  reviewedConverterContextChanged?.();
  const list=visible();
  table.setAttribute('aria-rowcount',String(list.length+1));
  updateSortHeader(table.querySelectorAll<HTMLElement>('thead th[data-col]'),tableColumns(),columnLayout.sort);
  wrap.querySelector('.empty')?.remove();
  if(!list.length)wrap.insertAdjacentHTML('beforeend',emptyMarkup());
  updateVirtualRows(true);
  document.querySelector('.filterbar .count')!.textContent=countText(list);
  document.querySelector('.selection-info')!.innerHTML=selectionInfoMarkup(paths().map(path=>entries.get(path)!),list);
  refreshCommandStates();
  timing('list refresh',started,{visible:list.length});
}
function refreshCommandStates(){
  document.querySelectorAll<HTMLButtonElement>('button[data-op]').forEach(el=>{el.disabled=!enabled(el.dataset.op as Operation);el.title=operationReason(el.dataset.op as Operation)||el.querySelector('span:not(.icon)')?.textContent||'';});
  const actions=enabled('action');document.querySelectorAll<HTMLButtonElement>('[data-action-group]').forEach(el=>el.disabled=!actions);
}
function selectedProvider(){return [...selected].some(path=>{const entry=entries.get(path);return !!entry&&providerEntry(entry);});}
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
  document.querySelector('.filterbar .count')!.textContent=countText(visible());
}
function keepField(key:string){
  paths().forEach(path=>{const entry=entries.get(path)!,name=panelKey({format:entry.format,fields:merged(entry)},key);updateDraft(path,name,[...(entry.fields[name]||[])]);});
  status=`Keeping the existing ${key} values of ${selected.size} file${selected.size===1?'':'s'}.`;
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
  coverIndex = 0; coverError=undefined;if(activeCover?.path!==active()?.path)activeCover=undefined;refreshSelection();
  document.querySelector<HTMLTableRowElement>(`tr[data-path="${CSS.escape(path)}"]`)?.focus({preventScroll:true});
}
function bindSplitter(){
  const splitter=document.querySelector<HTMLElement>('.panel-splitter'),editor=document.querySelector<HTMLElement>('.editor');if(!splitter||!editor)return;
  const show=(width:number)=>splitter.setAttribute('aria-valuenow',String(width));show(Math.round(editor.offsetWidth));
  splitter.addEventListener('pointerdown',event=>{
    if(event.button!==0)return;event.preventDefault();splitter.setPointerCapture(event.pointerId);splitter.classList.add('dragging');
    const left=editor.getBoundingClientRect().left,move=(e:PointerEvent)=>show(setPanelWidth(e.clientX-left));
    const end=(e:PointerEvent)=>{splitter.removeEventListener('pointermove',move);splitter.classList.remove('dragging');show(setPanelWidth(e.clientX-left,true));};
    splitter.addEventListener('pointermove',move);splitter.addEventListener('pointerup',end,{once:true});splitter.addEventListener('pointercancel',end,{once:true});
  });
  splitter.addEventListener('keydown',event=>{if(event.key!=='ArrowLeft'&&event.key!=='ArrowRight')return;event.preventDefault();show(setPanelWidth(editor.offsetWidth+(event.key==='ArrowRight'?16:-16),true));});
}
function bind() {
  document.querySelector<HTMLSelectElement>('#appearance')!.addEventListener('change',event=>appearance.set((event.target as HTMLSelectElement).value));
  bindSplitter();
  actionUI.bindMenu(app);tagPanel.bind(app);columnUI.bindMenu(app);fileOps.bindMenu(app);coverUI.bind(app);toolsUI.bindMenu(app);
  document.querySelectorAll<HTMLButtonElement>('button[data-pane]').forEach(el=>el.addEventListener('click',()=>{mobilePane=el.dataset.pane as 'files'|'tags';render();}));
  document.querySelectorAll<HTMLButtonElement>('[data-op]').forEach(el => el.addEventListener('click', () => { document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d => d.open = false); void operate(el.dataset.op as Operation); }));
  bindFields(app);
  document.querySelector<HTMLInputElement>('#filter')!.addEventListener('keydown',event=>{if(event.key==='Enter'&&filterDraft!==undefined){filter=filterDraft;filterDraft=undefined;document.querySelector('.table-wrap')!.scrollTop=0;refreshList();document.querySelector<HTMLInputElement>('#filter')?.focus();}});
  document.querySelector<HTMLInputElement>('#filter')!.addEventListener('input',event => {
    const el = event.target as HTMLInputElement, position = el.selectionStart;
    if(!filterAutoApply){filterDraft=el.value;return;}
    filter = el.value; document.querySelector('.table-wrap')!.scrollTop=0;refreshList();
    const replacement = document.querySelector<HTMLInputElement>('#filter')!;if(replacement!==el){replacement.focus();try{replacement.setSelectionRange(position,position);}catch{/* Search controls do not expose selection on every webview. */}}
  });
  document.querySelectorAll<HTMLButtonElement>('[data-sort]').forEach(el => el.addEventListener('click', event => { document.querySelector('.table-wrap')!.scrollTop=0;applyColumns({...columnLayout,sort:sortClick(columnLayout.sort,el.dataset.sort!,event.shiftKey)}).catch(error=>{errors=[{path:'Columns',error:String(error)}];render();}); }));
  const wrap=document.querySelector<HTMLDivElement>('.table-wrap')!;
  let scrollFrame=0;
  wrap.addEventListener('scroll',()=>{if(wrap.clientHeight){tableScrollTop=wrap.scrollTop;tableScrollLeft=wrap.scrollLeft;}if(!scrollFrame)scrollFrame=requestAnimationFrame(()=>{scrollFrame=0;if(wrap.isConnected&&wrap.clientHeight)updateVirtualRows();});},{passive:true});
  const rowFor=(event:Event)=>(event.target as Element).closest<HTMLTableRowElement>('tr[data-path]');
  wrap.addEventListener('click',e=>{
    const row=rowFor(e);if(!row)return;
    if((e.target as Element).classList.contains('row-select'))select(row.dataset.path!,new MouseEvent('click',{ctrlKey:true,shiftKey:e.shiftKey}));
    else{
      // A second, delayed click on a selected file edits the clicked cell.
      const cell=(e.target as Element).closest<HTMLElement>('td.editable[data-col]'),again=!!cell&&!e.shiftKey&&!e.metaKey&&!e.ctrlKey&&!compactScreen()&&selected.size===1&&selected.has(row.dataset.path!);
      if(compactScreen())mobilePane='tags';select(row.dataset.path!,e);
      window.clearTimeout(slowEditTimer);if(again){const target={path:row.dataset.path!,column:cell!.dataset.col!};slowEditTimer=window.setTimeout(()=>{if(selected.size===1&&selected.has(target.path)&&!busy)cellEditor.start(target);},550);}
    }
  });
  rowMenu.bind(wrap);
  wrap.addEventListener('dblclick',e=>{window.clearTimeout(slowEditTimer);if(rowFor(e)){if(toolsUI.defaultTool>=0)void toolsUI.run(toolsUI.defaultTool);else void operate('extended');}});
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
  document.querySelector('#cancel-job')?.addEventListener('click',cancelProgress);
}
/** Run one exclusive operation. With `progress`, the job receives an id for backend progress events and cancellation. */
async function run<T>(message: string, job: (progress?:string) => Promise<T>, options: {preserveErrors?:boolean;progress?:boolean} = {}): Promise<T|undefined> {
  if(busy)return;busy=true;status=message;if(!options.preserveErrors)errors=[];
  progressJob=options.progress&&desktop?{id:crypto.randomUUID(),done:0,total:0,cancelling:false}:undefined;render();
  try { return await job(progressJob?.id); } catch(error) { const failure={path:'Operation',error:String(error)};errors=options.preserveErrors?[...errors,failure]:[failure];status='Operation failed';return undefined; } finally {progressJob=undefined;await refreshSourceAccess();await refreshHistory();busy=false;render();void flushNotices();}
}
function progressText(job:ProgressJob){return job.cancelling?'Cancelling…':job.total?`${job.done} of ${job.total}`:'Preparing…';}
function progressMarkup(){
  const job=progressJob;if(!job)return '';
  return `<span class="job-progress"><progress max="${Math.max(1,job.total)}" value="${job.done}" aria-label="Operation progress"></progress><span class="job-progress-text">${progressText(job)}</span><button id="cancel-job" ${job.cancelling?'disabled':''}>Cancel</button></span>`;
}
function showProgress(){
  const job=progressJob;if(!job)return;
  const bar=document.querySelector<HTMLProgressElement>('.job-progress progress'),text=document.querySelector('.job-progress-text');
  if(bar){bar.max=Math.max(1,job.total);bar.value=job.done;}if(text)text.textContent=progressText(job);
}
function cancelProgress(){
  const job=progressJob;if(!job||job.cancelling)return;
  job.cancelling=true;showProgress();document.querySelector<HTMLButtonElement>('#cancel-job')?.setAttribute('disabled','');
  void invoke('cancel_job',{job:job.id}).catch(()=>{/* The job already finished. */});
}
// Files applyRenamed dropped from the list; the accept() that follows it does not list them again.
const movedOut=new Set<string>();
function accept(batch: Batch, message: string, clearDrafts = true) {
  batch.entries.forEach(entry=>{if(!movedOut.has(entry.path))storeEntry(entry);});movedOut.clear();reconcileDrafts(drafts,batch.entries.map(entry=>entry.path),clearDrafts);if(clearDrafts)batch.entries.forEach(entry=>{copyOverrides.delete(entry.path);workingReadErrors.delete(entry.path);});
  errors=batch.errors;status=`${message}: ${batch.entries.length} file${batch.entries.length===1?'':'s'}${errors.length?`, ${errors.length} failed`:''}`;
  if(noticeLabels[message]&&batch.entries.length)queuedNotices.push([noticeLabels[message],status]);
  if(message==='Renamed'&&batch.entries.length)autoPlaylistPending=true;
}
function applyRenamed(batch:Batch){
  const changes=resolvePathChanges([...entries.keys()],batch.renamed||[],batch.moved_directories||[]);
  listRoots=rebaseCaseOnly(listRoots,batch.moved_directories||[]);
  // Listed folders, taken before files added on their own are renamed away.
  const folders=listRoots.map(root=>entries.has(root)?dirname(root):root);
  const snapshots=[...changes].map(([from,to])=>({from,to,entry:entries.get(from)!,draft:drafts.get(from),selected:selected.has(from)}));
  snapshots.forEach(({from})=>{invalidateCovers(from);entries.delete(from);drafts.delete(from);selected.delete(from);});
  snapshots.forEach(({from,to,entry,draft,selected:wasSelected})=>{storeEntry({...entry,path:to,filename:basename(to)});if(draft)drafts.set(to,draft);if(wasSelected)selected.add(to);if(anchor===from)anchor=to;});
  // Options → General → Keep moved files: without it, files moved out of the listed folders leave the list (unless they have pending edits).
  if(!generalPrefs().keep_moved&&folders.length){const gone=snapshots.map(({to})=>to).filter(to=>!insideFolders(to,folders)&&!drafts.has(to));gone.forEach(path=>movedOut.add(path));if(gone.length)removeEntries(gone);}
}
// Replacing the list (Change directory) happens only after the new scan succeeds.
async function scan(newPaths: string[], options:{recursive?:boolean;replace?:boolean;location?:string;errors?:FileError[]}={}) {
  return !!await run('Reading audio files…',async job=>{ const result=await invoke<Batch>('scan_files',{paths:newPaths,recursive:options.recursive??optionsUI.settings.directories.subfolders,job});if(options.replace)clearWorkspace();const empty=!entries.size;accept(result,'Loaded',false);if(options.errors?.length){errors=[...options.errors,...errors];status+=`, ${options.errors.length} playlist entr${options.errors.length===1?'y':'ies'} not loaded`;}listRoots=options.replace?[...newPaths]:[...new Set([...listRoots,...newPaths])];if(generalPrefs().select_added&&result.entries.length){const added=new Set(result.entries.map(entry=>entry.path));selected.clear();added.forEach(path=>selected.add(path));anchor=ordered().find(entry=>added.has(entry.path))?.path;coverIndex=0;}if(empty||!options.location)location=options.location??(newPaths.length===1?newPaths[0]:`${newPaths.length} added locations`);if(!selected.size&&result.entries.length){selected.add(result.entries[0].path);anchor=result.entries[0].path;}if(progressJob?.cancelling)status+=' (reading cancelled)';return true; },{progress:true});
}
// Playlists and cue sheets among the paths load their entries in playlist order (https://docs.mp3tag.de/playlists/).
async function loadPaths(newPaths:string[],options:{recursive?:boolean;replace?:boolean;location?:string}={}){
  if(!newPaths.some(isPlaylistPath))return scan(newPaths,options);
  let expanded;
  try{expanded=await playlistUI.expand(newPaths);}catch(error){errors=[{path:'Playlist',error:publicMessage(error)}];status='Playlist could not be read';render();return false;}
  const wasEmpty=!entries.size,playlist=expanded.playlist;
  const loaded=await scan(expanded.paths,{...options,errors:expanded.errors,location:options.location&&newPaths.length>1?options.location:playlist});
  if(loaded&&playlist&&(options.replace||wasEmpty)){
    loadedPlaylist=playlist;
    // Playlist order is the list's load order; a column sort would hide it.
    if(columnLayout.sort.length)await applyColumns({...columnLayout,sort:[]}).catch(error=>{errors=[...errors,{path:'Columns',error:String(error)}];render();});
  }
  return loaded;
}
// Drop entries and everything cached for them (Remove from list, Clear list, Refresh, Delete, Change directory).
function removeEntries(removed:string[]){
  removed.forEach(path=>{invalidateCovers(path);entries.delete(path);drafts.delete(path);copyOverrides.delete(path);workingReadErrors.delete(path);selected.delete(path);if(anchor===path)anchor=undefined;});
  columnValues.retain(path=>entries.has(path));changedData();
}
function clearWorkspace(){cellEditor.cancel();removeEntries([...entries.keys()]);listRoots=[];coverIndex=0;loadedPlaylist=undefined;}
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
  return await run(`${message}…`,async()=>{
    let batch:Batch;
    try{batch=await invoke<Batch>(command,{paths:targets,...args});}
    catch(error){await reconcileWorkingEntries(targets);throw error;}
    accept(batch,message);
    await reconcileWorkingEntries(batch.errors.map(error=>error.path));
    return batch;
  });
}
// One backend call writes every file (in parallel, as one undo step) and reports progress; Cancel keeps unsaved drafts.
async function saveDrafts() {
  await run('Saving tags…',async job=>{
    const items=paths().flatMap(path=>{const updates=drafts.get(path);return updates?[{path,updates}]:[];});
    const batch=await invoke<Batch>('save_fields_batch',{items,job});
    batch.entries.forEach(storeEntry);reconcileDrafts(drafts,batch.entries.map(entry=>entry.path),true);batch.entries.forEach(entry=>{copyOverrides.delete(entry.path);workingReadErrors.delete(entry.path);});
    const saved=batch.entries.length,skipped=items.length-saved-batch.errors.length;
    errors=batch.errors;status=`Saved tags: ${saved} file${saved===1?'':'s'}${errors.length?`, ${errors.length} failed`:''}${skipped>0?`. Cancelled before ${skipped} file${skipped===1?'':'s'}; their changes are still pending`:''}`;if(saved){queuedNotices.push(['after_save',status]);autoPlaylistPending=true;}
  },{progress:true});
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
  const dialog=modal(rename?'Tag → Filename':'Filename → Tag',`<p>${rename?'Preview filenames before applying. The audio file extension is retained.':'Extract tags from each filename using field placeholders. Preview the results before saving them to your files.'}</p><label class="field"><span>${rename?'FORMAT STRING':'FILENAME PATTERN'}</span><input id="template" value="%artist% - %title%" spellcheck="false" autocomplete="off" data-helper="${rename?'format':'pattern'}" data-history="${rename?'tag-filename':'filename-tag'}" data-history-manage></label><div id="preview"></div>`,`<button id="preview-button">Preview</button><button class="primary" id="apply-converter" disabled>Apply to ${selected.size} files</button>`);
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
    <label class="field"><span>OLD FILENAME PATTERN</span><input id="old-pattern" value="%1 - %2" spellcheck="false" autocomplete="off" data-helper="history" data-history="filename-filename-old" data-history-manage></label>
    <label class="field"><span>NEW FILENAME PATTERN</span><input id="new-pattern" value="%2 - %1" spellcheck="false" autocomplete="off" data-helper="history" data-history="filename-filename-new" data-history-manage></label>`:
    `<p>Import one record per selected file in table order. Use %_filename_ext% or %_path% in the mask to match by file. Apply saves the reviewed tag values immediately.</p>
    <label class="field"><span>TEXT SOURCE</span><select id="text-source-kind"><option value="file">Text file</option><option value="text">Pasted text</option></select></label>
    <div id="text-file-source"><div class="field"><span>SOURCE FILE</span><div class="action-input"><input id="text-source-path" aria-label="Text source file" placeholder="Choose a text file" autocomplete="off" spellcheck="false"><button id="pick-text-source" type="button">Choose file…</button></div></div>
    <label class="field"><span>LEGACY ENCODING FALLBACK (OPTIONAL)</span><input id="text-codepage" placeholder="e.g. windows-1252" list="text-codepages" spellcheck="false"><datalist id="text-codepages"><option value="windows-1252"><option value="windows-1251"><option value="cp932"><option value="cp936"><option value="cp949"><option value="cp950"></datalist></label>
    <p>UTF-8 and BOM-marked UTF-16 are detected first. The fallback is used only when the file is not valid UTF-8.</p></div>
    <label class="field" id="text-paste-source" hidden><span>RECORDS TO IMPORT</span><textarea id="text-source-records" rows="5" spellcheck="false"></textarea></label>
    <label class="field"><span>IMPORT MASK</span><input id="text-mask" value="%title%;%artist%" autocomplete="off" spellcheck="false" data-helper="pattern" data-history="text-tag" data-history-manage></label>
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
async function refreshHistory(){
  if(!desktop)return;
  try{const next=await invoke<HistoryState>('history_state');if(next.notice&&next.notice!==history.notice)status=`${status}. ${next.notice}`;history=next;}catch{/* Undo history is optional; operations still complete. */}
}
async function replayHistory(kind:'undo'|'redo'){
  const step=history[kind];if(!step)return;
  await run(`${kind==='undo'?'Undoing':'Redoing'} ${step.label}…`,async()=>{
    const batch=await invoke<Batch&{label:string;history:HistoryState}>(kind==='undo'?'undo_last':'redo_last');
    const restored=batch.entries.length;applyRenamed(batch);history=batch.history;
    // Restored files that are no longer listed stay out of the list; drafts are kept.
    accept({...batch,entries:batch.entries.filter(entry=>entries.has(entry.path))},'',false);
    status=`${kind==='undo'?'Undid':'Redid'} ${batch.label}: ${restored} file${restored===1?'':'s'}${errors.length?`, ${errors.length} not ${kind==='undo'?'undone':'redone'}`:''}`;
  });
}
const clipTargets=()=>paths().map(path=>({path,fields:merged(entries.get(path)!)}));
function stageTagUpdates(plan:()=>PasteUpdate[],name:string,done:(count:number)=>string){
  try{
    const updates=plan();
    for(const {path,updates:fields} of updates){const entry=entries.get(path)!;for(const [key,values] of Object.entries(fields))updateDraft(path,panelKey({format:entry.format,fields:merged(entry)},key),values);}
    status=`${done(updates.length)} ${sourcePolicy().mutationReason?'Use Save a copy to keep these edits.':'Save tags to write them.'}`;
  }catch(error){errors=[{path:name,error:error instanceof Error?error.message:String(error)}];status=`${name} failed`;}
  render();
}
function clipTags(cut:boolean){
  const covers=paths().reduce((sum,path)=>sum+coversFor(entries.get(path)).length,0);
  tagClipboard=copyTags(paths().map(path=>{const entry=entries.get(path)!;return{name:entry.filename,fields:merged(entry),covers:coversFor(entry).length};}));
  void navigator.clipboard?.writeText(clipboardText(tagClipboard)).catch(()=>{/* The tag clipboard works without system clipboard access. */});
  const copied=`${cut?'Cut':'Copied'} tags of ${tagClipboard.items.length} file${tagClipboard.items.length===1?'':'s'}${covers?' (artwork is not included)':''}.`;
  if(cut)stageTagUpdates(()=>cutPlan(clipTargets()),'Cut tags',()=>copied);else{status=copied;render();}
}
function tagToTag(){
  openTagToTag({modal,modalError,esc,paths,displayName,canApply:()=>enabled('tag-tag'),
    fieldNames:()=>[...new Set(paths().flatMap(path=>Object.keys(merged(entries.get(path)!))))],
    storage:{getItem:key=>window.localStorage.getItem(key),setItem:(key,value)=>window.localStorage.setItem(key,value)},
    preview:(targets,template,filename)=>invoke<Preview[]>('preview_format',{paths:targets,template,columns:columnSpecs(targets),filename}),
    apply:async({field,template,paths:targets})=>{
      if(!enabled('tag-tag'))throw new Error(operationReason('tag-tag')||'Select files and save pending tag edits first.');
      const result=await run('Converting tags…',async()=>{const batch=await invoke<Batch>('convert_tag_to_tag',{paths:targets,field,template,columns:columnSpecs(targets)});applyRenamed(batch);accept(batch,'Tag → Tag');return batch;});
      if(!result)throw new Error(errors.map(error=>`${error.path}: ${error.error}`).join('\n')||'Tag → Tag did not complete.');
      if(result.errors.length)throw new Error(result.errors.map(error=>`${displayName(error.path)}: ${error.error}`).join('\n'));
    }});
}
function numbering() {
  const directoryAvailable=paths().every(path=>!providerEntry(entries.get(path)!));
  const names=[...new Set(['ALBUM','DISCNUMBER','ALBUMARTIST',...paths().flatMap(path=>Object.keys(merged(entries.get(path)!)))])].sort();
  const fieldOptions=`<option value="">Do not change counter</option>${names.map(name=>`<option value="${esc(name)}">${esc(name)}</option>`).join('')}`;
  const counter=(id:string,label:string,initial:number,max=4_294_967_295)=>`<label class="field"><span>${label}</span><input id="${id}" type="number" min="1" max="${max}" step="1" value="${initial}"></label>`;
  const dialog=modal('Auto-numbering',`<p>Number selected files in their current table order. Preview, then stage the numbers as pending edits.</p>
    <fieldset class="numbering-section"><legend>Track numbers</legend><div class="field-pair">${counter('number-start','Begin at track',1)}${counter('number-digits','Minimum digits',2,10)}</div>
    <label class="action-check"><input type="checkbox" id="number-total"> Save total track count</label>${counter('number-total-value','Total tracks',selected.size)}</fieldset>
    <fieldset class="numbering-section"><legend><label><input type="checkbox" id="number-disc-enabled"> Disc numbers</label></legend><div class="field-pair">${counter('number-disc-start','Begin at disc',1)}${counter('number-disc-digits','Minimum digits',1,10)}</div>
    <label class="action-check"><input type="checkbox" id="number-disc-total"> Save total disc count</label>${counter('number-disc-total-value','Total discs',1)}</fieldset>
    <fieldset class="numbering-section"><legend>Counter changes</legend><label class="action-check"><input type="checkbox" id="number-directory" ${directoryAvailable?'':'disabled'}> Next disc on change of directory</label>
    ${directoryAvailable?'':'<p>Directory grouping requires local files with known folders.</p>'}
    <label class="field"><span>Next disc on change of field</span><select id="number-next-field">${fieldOptions}</select></label>
    <label class="action-check"><input type="checkbox" id="number-reset-track"> Reset track counter on next disc</label>
    <label class="field"><span>Reset disc counter on change of field</span><select id="number-reset-field">${fieldOptions}</select></label>
    <p>Resetting the disc counter also resets tracks. Totals use the values entered above.</p></fieldset>
    <p id="number-progress" role="status">Run Preview to review the sequence.</p><div id="number-preview"></div>`,
    `<button id="preview-number">Preview</button><button class="primary" id="apply-number" disabled>Stage numbers</button>`);
  dialog.classList.add('numbering-dialog');
  const checked=(id:string)=>dialog.querySelector<HTMLInputElement>(`#${id}`)!.checked;
  const count=(id:string,label:string,max?:number)=>numberingCounter(value(dialog,id),label,max);
  const request=():NumberingRequest=>({
    items:paths().map(path=>({path,fields:merged(entries.get(path)!)})),
    options:{
      track_start:count('number-start','Track start'),track_digits:count('number-digits','Track digits',10),track_total:checked('number-total')?count('number-total-value','Total tracks'):null,
      disc_enabled:checked('number-disc-enabled'),disc_start:checked('number-disc-enabled')?count('number-disc-start','Disc start'):1,disc_digits:checked('number-disc-enabled')?count('number-disc-digits','Disc digits',10):1,
      disc_total:checked('number-disc-enabled')&&checked('number-disc-total')?count('number-disc-total-value','Total discs'):null,
      next_disc_on_directory:directoryAvailable&&checked('number-directory'),next_disc_on_field:value(dialog,'number-next-field')||null,
      reset_track_on_next_disc:checked('number-reset-track'),reset_disc_on_field:value(dialog,'number-reset-field')||null,
    },
  });
  const review=createNumberingReview(),apply=dialog.querySelector<HTMLButtonElement>('#apply-number')!,preview=dialog.querySelector<HTMLButtonElement>('#preview-number')!;
  const progress=dialog.querySelector<HTMLElement>('#number-progress')!,area=dialog.querySelector<HTMLElement>('#number-preview')!;
  const inputValues=()=>[...dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement>('input,select')].map(control=>control instanceof HTMLInputElement&&control.type==='checkbox'?control.checked:control.value);
  const inputsChanged=createConverterInputTracker(inputValues());
  let generation=0,pending=false;
  const sync=()=>{
    const disable=(id:string,disabled:boolean)=>{dialog.querySelector<HTMLInputElement>(`#${id}`)!.disabled=disabled;};
    disable('number-total-value',!checked('number-total'));
    for(const id of ['number-disc-start','number-disc-digits','number-disc-total'])disable(id,!checked('number-disc-enabled'));
    disable('number-disc-total-value',!checked('number-disc-enabled')||!checked('number-disc-total'));
    preview.disabled=pending||busy;
    try{apply.disabled=pending||busy||!review.canApply(request());}catch{apply.disabled=true;}
  };
  const invalidate=()=>{generation++;pending=false;review.invalidate();area.replaceChildren();progress.textContent='Inputs changed. Run Preview again.';sync();};
  const changed=()=>{if(inputsChanged(inputValues()))invalidate();else sync();};
  dialog.addEventListener('input',changed);dialog.addEventListener('change',changed);
  dialog.addEventListener('close',()=>{generation++;review.invalidate();});
  preview.addEventListener('click',async()=>{
    preview.focus({preventScroll:true});changed();const current=++generation;
    try{
      if(busy)throw new Error('Wait for the current file operation to finish.');
      const attempt=review.begin(request());pending=true;area.replaceChildren();dialog.querySelector('.dialog-error')?.remove();progress.textContent='Preparing numbering preview…';sync();
      const plan=await invoke<NumberingUpdate[]>('preview_numbering',attempt.request);
      if(!dialog.open||current!==generation)return;
      if(!review.complete(attempt,plan,request())){progress.textContent='Selection changed. Run Preview again.';return;}
      const rows=plan.slice(0,100);
      area.innerHTML=`<div class="preview" tabindex="0" aria-label="Numbering preview"><table><thead><tr><th>File</th><th>Track</th>${attempt.request.options.disc_enabled?'<th>Disc</th>':''}</tr></thead><tbody>${rows.map(row=>`<tr><td>${esc(displayName(row.path))}</td><td>${esc(row.fields.TRACK[0])}</td>${attempt.request.options.disc_enabled?`<td>${esc(row.fields.DISCNUMBER[0])}</td>`:''}</tr>`).join('')}</tbody></table></div>`;
      progress.textContent=`Reviewed ${plan.length} files.${plan.length>100?' Showing the first 100.':''} Stage numbers, then Save tags to write them.`;
    }catch(error){if(dialog.open&&current===generation){review.invalidate();modalError(dialog,error);progress.textContent='Preview failed. Review the options and try again.';}}
    finally{if(dialog.open&&current===generation){pending=false;sync();}}
  });
  apply.addEventListener('click',()=>{
    apply.focus({preventScroll:true});changed();
    try{
      if(busy)throw new Error('Wait for the current file operation to finish.');
      const plan=review.take(request());
      for(const row of plan){const entry=entries.get(row.path)!;for(const [key,values] of Object.entries(row.fields))updateDraft(row.path,panelKey({format:entry.format,fields:merged(entry)},key),values);}
      status=`Numbers are pending for ${plan.length} files. Save tags to write them.`;dialog.close();render();
    }catch(error){modalError(dialog,error);sync();}
  });
  sync();dialog.querySelector<HTMLInputElement>('#number-start')!.focus({preventScroll:true});
}
// Utils → Remove unsupported fields, Optimize and Strip chapters, for the selected files they apply to.
async function runUtil(util:Util){
  const spec=UTILS[util],formats=paths().map(path=>entries.get(path)!.format),targets=paths().filter(path=>spec.formats.includes(entries.get(path)!.format));
  if(!utilTargets(util,formats)){errors=[{path:spec.label,error:`${spec.label} applies to ${spec.applies}.`}];status=`${spec.label}: none of the selected files apply`;render();return;}
  const done=util==='remove_unsupported_fields'?'Removed unsupported fields':'Optimized';
  const dialog=modal(spec.label,`<p>${esc(utilMessage(util,formats))}</p>`,`<button id="run-util" class="primary">${esc(spec.label)}</button>`);
  dialog.classList.add('util-dialog');
  const button=dialog.querySelector<HTMLButtonElement>('#run-util')!;button.focus();
  button.addEventListener('click',async()=>{
    button.disabled=true;
    const result=await run(`${spec.label}…`,async()=>{const batch=await invoke<Batch>('apply_util',{paths:targets,util});accept(batch,done);return batch;});
    if(result&&!result.errors.length)dialog.close();else{modalError(dialog,result?result.errors.map(error=>`${displayName(error.path)}: ${publicMessage(error.error)}`).join('\n'):errors.map(error=>publicMessage(error.error)).join('\n'));button.disabled=false;}
  });
}
// Help → Check for updates and Options → Updates.
async function checkForUpdates(startup:boolean){
  const prefs=updateOptions(optionsUI.page('updates'));
  let check:UpdateCheck;
  try{check=await invoke<UpdateCheck>('check_for_updates',{includePrereleases:prefs.include_beta});}
  catch(error){
    if(startup){status=`Update check failed: ${publicMessage(error)}`;const el=document.querySelector('.status-message');if(el)el.textContent=status;return;}
    const failed=modal('Check for updates',`<p>The release list could not be read.</p>`,'');failed.classList.add('updates-dialog');failed.querySelector('[data-close]')!.textContent='Close';modalError(failed,error);return;
  }
  if(startup&&!check.newer)return;
  const text=updateMessage(check),latest=check.latest;
  const dialog=modal(text.title,`<p>${esc(text.text)}</p>${latest?.published?`<p><small>Published ${esc(new Date(latest.published).toLocaleDateString())}. OpenTag does not download or install updates; get them from the release page.</small></p>`:''}`,latest?`<button id="update-release" class="primary">${check.newer?'Open release page':'View release notes'}</button>`:'');
  dialog.classList.add('updates-dialog');dialog.querySelector('[data-close]')!.textContent='Close';
  dialog.querySelector('#update-release')?.addEventListener('click',()=>{void invoke('open_release_page',{url:latest!.url}).catch(error=>modalError(dialog,error)).then(()=>dialog.close());});
}
/** At startup: release notes after an update (remembered per device) and the optional update check. */
async function startupUpdates(version:string){
  const prefs=updateOptions(optionsUI.page('updates'));let last='';
  try{last=localStorage.getItem('opentag.last-version')||'';localStorage.setItem('opentag.last-version',version);}catch{/* Without storage, release notes are not shown. */}
  if(showReleaseNotes({...prefs,last_version:last},version))void invoke('open_release_page',{url:releaseNotesUrl(version)}).catch(()=>{/* Opening the browser is best effort. */});
  if(prefs.check_on_startup)await checkForUpdates(true);
}
async function operateImpl(op: Operation) {
  if(!enabled(op))return;
  if(op==='about'){const dialog=modal('About OpenTag',`<img class="about-artwork" data-theme-image="about" src="${themedAsset('about')}" alt="OpenTag — Tag, Organize, Enjoy"><p>Tag, Organize, Enjoy.</p><p>Open source music tag editor.</p><p>Version ${esc(appVersion)} · MIT License</p>`,'');dialog.querySelector('[data-close]')!.textContent='Close';return;}
  if(op==='refresh-source'){await run('Refreshing source access…',async()=>{},{preserveErrors:true});status=documentAccess.refreshError?'Source access refresh failed. Pending edits are retained.':'Source access refreshed';render();return;}
  if(op==='save-copy'){saveCopy();return;}
  if(op==='files'&&runtimePlatform.document_access_supported){await run('Choosing audio documents…',async()=>{const batch=await invoke<Batch>('pick_document_files');if(!batch.entries.length&&!batch.errors.length){status='No files added';return;}accept(batch,'Loaded',false);location='Provider documents';if(batch.entries.length){selected.clear();batch.entries.forEach(entry=>selected.add(entry.path));anchor=batch.entries[0].path;}});return;}
  if(op==='folder'||/^(dir|file)-/.test(op)){await fileOps.operate(op as FileOperation|'folder');return;}
  if(op==='files'){const result=await open({multiple:true,title:'Add audio files'});if(result)await loadPaths(Array.isArray(result)?result:[result]);return;}
  if(op==='save'){await saveDrafts();return;}
  if(op==='redo'&&repeatState().kind==='repeat'){await executeGroups(lastActionGroups!).catch(error=>{errors=[{path:'Repeat',error:publicMessage(error)}];status='Repeat failed';render();});return;}
  if(op==='undo'||op==='redo'){await replayHistory(op);return;}
  if(op==='cut-tags'&&!await messages.confirm('confirm_cut_tags',`Cut the tags of ${selected.size} selected file${selected.size===1?'':'s'}? The fields are cleared as pending changes.`,'Cut tags'))return;
  if(op==='paste-tags'&&!await messages.confirm('confirm_paste_tags',`Paste tags to ${selected.size} selected file${selected.size===1?'':'s'}?`,'Paste tags'))return;
  if(op==='copy-tags'||op==='cut-tags'){clipTags(op==='cut-tags');return;}
  if(op==='paste-tags'){stageTagUpdates(()=>pastePlan(tagClipboard,clipTargets()),'Paste tags',count=>`Pasted tags to ${count} file${count===1?'':'s'}.`);return;}
  if(op==='tag-tag'){tagToTag();return;}
  if(op==='select-all'){selected.clear();visible().forEach(e=>selected.add(e.path));anchor=paths()[0];coverIndex=0;refreshSelection();return;}
  if(op==='discard'){if(await messages.confirm('unsaved_changes','Discard the pending tag changes for the selected files?','Discard changes')){paths().forEach(discardDrafts);changedData();status='Pending changes discarded';render();}return;}
  if(op==='reload'){if(dirtySelected()&&!await messages.confirm('unsaved_changes','Reload tags from the source and discard pending changes for the selected files?','Reload tags'))return;await run('Reloading tags…',async()=>accept(await invoke<Batch>('scan_files',{paths:paths(),recursive:false}),'Reloaded'));return;}
  if(op==='extended'){extended();return;}
  if(op==='filename-tags'||op==='tags-filename'){converter(op==='tags-filename');return;}
  if(op==='filename-filename'||op==='text-tags'){reviewedConverter(op==='filename-filename'?'filename':'text');return;}
  if(op==='action'){actionUI.quick();return;}
  if(op==='groups'){await actionUI.groupsDialog();return;}
  if(op==='export-config'||op==='export-folders'){const perDirectory=op==='export-folders';if(optionsUI.page('export').show_dialog===false)await exportUI.runSaved(perDirectory).catch(error=>{errors=[{path:'Export',error:publicMessage(error)}];status='Export failed';render();});else await exportUI.open(perDirectory);return;}
  if(op==='playlist-load'){const path=await open({multiple:false,title:'Load playlist or cue sheet',filters:[{name:'Playlists and cue sheets',extensions:['m3u','m3u8','pls','cue']}]});if(typeof path!=='string'||drafts.size&&!await messages.confirm('unsaved_changes',`Load ${basename(path)} and discard pending tag changes in ${drafts.size} file${drafts.size===1?'':'s'}?`,'Unsaved changes'))return;await loadPaths([path],{replace:true,recursive:false,location:path});return;}
  if(op==='playlist-all'||op==='playlist-selected'){await playlistUI.create(op==='playlist-all'?'all':'selected').catch(error=>{errors=[{path:'Playlist',error:publicMessage(error)}];status='Playlist failed';render();});return;}
  if(op==='playlist-generate'){playlistUI.generate();return;}
  if(op==='list-unsorted'){await applyColumns({...columnLayout,sort:[]});status='Files are shown in load order (playlist order)';render();return;}
  if(op==='options'){await optionsUI.open();return;}
  if(op==='number'){numbering();return;}
  if(tagSourceOperations.includes(op)){const source=op.replace(/^source-(cover-)?/,'').replace('-','_') as Provider;await tagSourcesUI.open(source,{coversOnly:op.startsWith('source-cover-')});render();return;}
  if(op==='source-token'){await tagSourcesUI.token();return;}
  if(op==='source-options'){await optionsUI.open('tag-sources');return;}
  if(op==='toggle-panel'){tagPanel.toggle();return;}
  if(op==='panel-options'){tagPanel.configure();return;}
  if(op==='field-values'){tagPanel.fieldValues();return;}
  if(op==='panel-defaults'){tagPanel.applyDefaults();return;}
  if(op==='add-cover'||op==='remove-cover'||op==='cover-manage'){await coverUI.run(op==='add-cover'?'add':op==='remove-cover'?'remove':'manage');return;}
  if(op==='remove-tags'){if(await messages.confirm('confirm_remove_tags',`Remove all tags from ${selected.size} selected file${selected.size===1?'':'s'}?`,'Remove tags'))await mutate('remove_tags',{},'Removed tags');return;}
  if(op==='strip-mp4-chapters'){const found=mp4ChapterFiles(paths().map(path=>entries.get(path)??{}));if(await messages.confirm('confirm_remove_tags',stripChaptersPrompt(found),'Strip chapters MP4'))await mutate('strip_mp4_chapters',{},'Stripped MP4 chapters');return;}
  if(op==='remove-tag-types'){removeTagTypesDialog({modal,modalError,tags:()=>paths().map(path=>entries.get(path)?.technical?._TAG?.[0]),remove:async kinds=>{await mutate('remove_tag_types',{kinds:kinds as RemovableTag[]},'Removed tag types');}});return;}
  if(op==='copy-text'){copyAsTextDialog({modal,modalError,values:()=>optionsUI.page('copy-as-text'),remember:values=>desktop?optionsUI.updatePage('copy-as-text',values):Promise.resolve(),count:()=>selected.size,
    evaluate:async(format,limit)=>{const targets=paths().slice(0,limit),items=targets.map(path=>columnItem(entries.get(path)!));if(!desktop)return items.map(item=>quickValue(format,item.fields)??'');const result=await invoke<{values:string[][];errors:string[]}>('evaluate_columns',{items,templates:[format]});if(result.errors.length)throw new Error(result.errors[0]);return result.values.map(row=>row[0]??'');},
    write:text=>desktop?invoke<void>('write_clipboard_text',{text}):navigator.clipboard.writeText(text),done:lines=>{status=`Copied ${lines} line${lines===1?'':'s'} as text`;render();}});return;}
  if(op==='tools-options'){await optionsUI.open('tools');render();return;}
  if(op==='help-shortcuts'){shortcutsDialog(modal,[...menuShortcuts(document.querySelector('.menubar')!),...EXTRA_SHORTCUTS]);return;}
  if(op==='help-updates'){await checkForUpdates(false);return;}
  if(op in utilOperations){await runUtil(utilOperations[op as UtilOperation]);return;}
  if(op==='help-docs'||op==='help-issues'||op==='help-releases'){await invoke('open_help_page',{page:op==='help-docs'?'documentation':op==='help-issues'?'issues':'releases'});return;}
  if(op==='config-folder'){await invoke('open_configuration_folder');return;}
  if(op==='config-save'){const outputPath=await save({title:'Save configuration',defaultPath:configurationFileName(),filters:[CONFIGURATION_FILTER]});if(outputPath){const count=await invoke<number>('export_configuration',{outputPath});status=`Saved ${count} configuration file${count===1?'':'s'} to ${basename(outputPath)}`;render();}return;}
  if(op==='config-load'){
    const path=await open({multiple:false,title:'Load configuration',filters:[CONFIGURATION_FILTER]});if(typeof path!=='string')return;
    if(!await ask(`Replace the current options, action groups, export configurations, columns, tag panel and favorite directories with ${basename(path)}?${drafts.size?` Pending changes in ${drafts.size} file${drafts.size===1?'':'s'} are discarded.`:''}\n\nOpenTag reloads its window afterwards. The Discogs access token is not part of a configuration file.`,{title:'Load configuration',kind:'warning'}))return;
    const restored=await invoke<string[]>('import_configuration',{path});drafts.clear();status=`Loaded ${restored.length} configuration file${restored.length===1?'':'s'}`;window.location.reload();return;
  }
  if(op==='csv'){const outputPath=await save({title:'Export tags as CSV',defaultPath:'tags.csv',filters:[{name:'CSV file',extensions:['csv']}]});if(outputPath&&await run('Exporting…',async()=>{await invoke('export_csv',{paths:paths(),outputPath});status=`Exported ${selected.size} files to ${basename(outputPath)}`;return true;}))await offerOpen([outputPath]);}
}
async function listOperation(op:Operation){
  const list=visible().map(entry=>entry.path);
  const choose=(targets:string[],focus=targets[0],listChanged=false)=>{selected.clear();targets.forEach(path=>selected.add(path));anchor=focus;coverIndex=0;coverError=undefined;if(listChanged)render();else refreshSelection();if(focus)revealRow(focus);};
  if(op==='unselect-all'){choose([]);return;}
  if(op==='invert-selection'){choose(invertSelection(list,selected));return;}
  if(op==='select-next'){const next=nextFile(list,selected);if(next)choose([next]);return;}
  if(op==='select-similar'||op==='select-next-similar'){
    const targets=op==='select-similar'?similarFiles(list,sortValueOf,anchor&&selected.has(anchor)?anchor:paths()[0]):nextSimilarFiles(list,sortValueOf,selected);
    if(targets.length){status=`Selected ${targets.length} similar file${targets.length===1?'':'s'}`;choose(targets);}return;
  }
  if(op==='find'){findUI.open();return;}
  if(op==='columns'){columnUI.customize();return;}
  if(op==='edit-cell'){
    const path=paths()[0],editable=tableColumns().filter(column=>editKey(column));
    const column=editable.find(column=>editKey(column).startsWith('_FILENAME'))||editable[0];
    if(path&&column)cellEditor.start({path,column:column.id});return;
  }
  if(op==='remove-from-list'||op==='clear-list'){
    const targets=op==='clear-list'?[...entries.keys()]:paths(),pending=targets.filter(path=>drafts.has(path)).length;
    if(pending&&!await messages.confirm('unsaved_changes',`${pending} file${pending===1?' has':'s have'} pending tag changes. Remove ${targets.length===1?'it':'them'} from the list and discard the changes? Files on disk are not changed.`,op==='clear-list'?'Clear list':'Remove from list'))return;
    const removed=new Set(targets),next=list.slice(Math.max(0,...list.map((path,index)=>removed.has(path)?index:-1))).find(path=>!removed.has(path))||[...list].reverse().find(path=>!removed.has(path));
    cellEditor.cancel();removeEntries(targets);
    if(op==='clear-list'||!entries.size){listRoots=[];location='No folder loaded';errors=[];}
    status=`Removed ${targets.length} file${targets.length===1?'':'s'} from the list`;
    choose(next?[next]:[],next,true);return;
  }
  if(op==='refresh'){
    if(drafts.size&&!await messages.confirm('unsaved_changes',`Refresh the file list and discard pending changes in ${drafts.size} file${drafts.size===1?'':'s'}?`,'Refresh'))return;
    cellEditor.cancel();
    await run('Refreshing file list…',async job=>{
      const batch=await invoke<Batch>('scan_files',{paths:listRoots,recursive:optionsUI.settings.directories.subfolders,job}),found=new Set(batch.entries.map(entry=>entry.path));
      if(progressJob?.cancelling){status='Refresh cancelled. The list is unchanged.';return;}
      removeEntries([...entries.keys()].filter(path=>!found.has(path)));
      drafts.clear();copyOverrides.clear();workingReadErrors.clear();
      accept(batch,'Refreshed');
      if(!anchor||!selected.has(anchor))anchor=paths()[0];
      if(!selected.size&&batch.entries.length){const first=ordered()[0].path;selected.add(first);anchor=first;}
    },{progress:true});
  }
}
async function commitCell(target:CellTarget,value:string):Promise<string|undefined>{
  const entry=entries.get(target.path),column=columnLayout.columns.find(c=>c.id===target.column);if(!entry||!column)return undefined;
  const key=editKey(column);if(!key)return target.path;
  if(key==='_FILENAME'||key==='_FILENAME_EXT')return renameFromList(entry,key,value);
  if(value===field(entry,key))return target.path;
  updateDraft(target.path,panelKey({format:entry.format,fields:merged(entry)},key),value?value.split('\\\\').map(v=>v.trim()):[]);
  status=`Pending changes for ${drafts.size} file${drafts.size===1?'':'s'}. ${sourcePolicy().mutationReason?'Use Save a copy to keep these edits.':'Save tags to write them.'}`;
  render();return target.path;
}
async function renameFromList(entry:Entry,key:string,value:string):Promise<string|undefined>{
  const dot=entry.filename.lastIndexOf('.'),extension=dot>0?entry.filename.slice(dot):'';
  let stem=value.trim();
  if(key==='_FILENAME_EXT'){if(stem===entry.filename)return entry.path;if(extension&&stem.toLowerCase().endsWith(extension.toLowerCase()))stem=stem.slice(0,-extension.length);else if(extension){errors=[{path:'Rename',error:`Keep the ${extension} extension when renaming ${entry.filename}.`}];render();return undefined;}}
  else if(stem===entry.filename.slice(0,dot>0?dot:undefined))return entry.path;
  const problem=!desktop?'Renaming requires the desktop app.':sourcePolicy().unsupportedReason||(!stem.trim()?'Enter a filename.':/[\\/]/.test(stem)?'Filenames cannot contain path separators.':'');
  if(problem){errors=[{path:'Rename',error:problem}];render();return undefined;}
  const result=await run('Renaming file…',async()=>{const batch=await invoke<Batch>('rename_files',{paths:[entry.path],template:`'${stem.replace(/'/g,"''")}'`,planToken:null});applyRenamed(batch);accept(batch,'Renamed',false);queuedNotices=[];return batch;});
  return result&&!result.errors.length?result.renamed?.find(item=>item.from===entry.path)?.to||entry.path:undefined;
}
async function operate(op: Operation) {
  try { if(listOperations.includes(op)){if(enabled(op))await listOperation(op);}else await operateImpl(op); }
  catch(error) {errors=[{path:'Operation',error:String(error)}];status='Operation failed';render();}
}
const menubar = installMenubar(document);
document.addEventListener('keydown',event=>{
  if(event.defaultPrevented||document.querySelector('dialog[open]:not(.find-dialog):not(.cover-viewer)'))return;
  const typing=event.target instanceof HTMLInputElement||event.target instanceof HTMLTextAreaElement;
  if(event.altKey&&(event.code==='Digit6'||event.key==='6')){event.preventDefault();menubar.close();void operate(event.shiftKey?'action':'groups');return;}
  if(event.altKey&&(event.code==='KeyA'||event.key.toLowerCase()==='a')&&!typing){event.preventDefault();const menu=document.querySelector<HTMLDetailsElement>('#actions-menu')!;menubar.open(menu);menu.querySelector('summary')?.focus();actionUI.beginMnemonic();return;}
  if(!typing&&document.querySelector<HTMLDetailsElement>('#actions-menu')?.open&&event.key.trim().length===1&&!event.metaKey&&!event.ctrlKey){event.preventDefault();void actionUI.mnemonic(event.key);return;}
  if(toolsUI.shortcut(event,typing)){menubar.close();return;}
  const edit=editShortcut(event,{typing,textSelected:!!window.getSelection()?.toString(),mac:/Mac|iPhone|iPad/.test(navigator.platform)});
  if(edit&&(edit!=='paste-tags'||tagClipboard)){event.preventDefault();menubar.close();void operate(edit);return;}
  if(event.key==='Escape'&&progressJob){event.preventDefault();cancelProgress();return;}
  if(event.key==='Escape'){document.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);}
  const fileOp=fileShortcut(event,typing,macPlatform);if(fileOp){event.preventDefault();menubar.close();void operate(fileOp==='folder'&&runtimePlatform.document_access_supported?'files':fileOp);return;}
  const listCommand=listShortcut(event,typing);
  if(listCommand&&!(listCommand==='remove-from-list'&&event.target!==document.body&&!(event.target as Element).closest?.('.table-wrap'))){
    event.preventDefault();menubar.close();
    if(listCommand==='filter-focus'){const input=document.querySelector<HTMLInputElement>('#filter');input?.focus();input?.select();}else void operate(listCommand);
    return;
  }
  if(event.metaKey||event.ctrlKey){if(event.key.toLowerCase()==='s'){event.preventDefault();void operate('save');}if(event.key.toLowerCase()==='o'){event.preventDefault();menubar.close();void operate('options');}if(event.key.toLowerCase()==='a'&&!typing){event.preventDefault();void operate('select-all');}if(event.key.toLowerCase()==='e'&&!typing){event.preventDefault();menubar.close();void operate(event.shiftKey?'export-folders':'export-config');}if(event.key.toLowerCase()==='p'&&!typing&&!event.altKey){event.preventDefault();menubar.close();void operate(event.shiftKey?'playlist-generate':'playlist-all');}if(event.key.toLowerCase()==='l'&&!typing){event.preventDefault();menubar.close();void operate('source-local-freedb');}if(event.key.toLowerCase()==='k'&&!typing){event.preventDefault();menubar.close();void operate('number');}if(event.key.toLowerCase()==='i'&&!typing){event.preventDefault();menubar.close();void operate(tagSourcesUI.lastProvider==='discogs'?'source-discogs':'source-musicbrainz');}}
});
tagPanel.install(document);
coverUI.install(document);
window.addEventListener('beforeunload',event=>{if(drafts.size||actionUI.hasUnsavedChanges){event.preventDefault();event.returnValue='';}});
installHelperMenus({
  fieldNames:()=>[...new Set(paths().flatMap(path=>Object.keys(merged(entries.get(path)!))))],
  panelFields:()=>tagPanel.settings.panel.fields.filter(field=>field.visible&&!field.field.startsWith('_')).map(field=>field.field),
  confirm:message=>ask(message,{title:'OpenTag',kind:'warning'}),
  filterAutoApply:{get:()=>filterAutoApply,set:value=>{filterAutoApply=value;try{localStorage.setItem('opentag.filter-auto-apply',String(value));}catch{/* Session-only preference. */}if(value&&filterDraft!==undefined){filter=filterDraft;filterDraft=undefined;}render();}},
});
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
  const versionReady=getVersion().then(version=>{appVersion=version;return version;}).catch(()=>{appVersion='unknown';return '';});
  void actionUI.initialize().catch(error=>{errors=[{path:'Action groups',error:String(error)}];render();});
  void invoke<ColumnLayout|null>('load_columns').then(layout=>{if(layout){columnLayout=normalizeLayout(layout);render();}}).catch(error=>{errors=[{path:'Columns',error:String(error)}];render();});
  void tagPanel.initialize().catch(error=>{errors=[{path:'Tag panel',error:String(error)}];render();});
  const optionsReady=optionsUI.initialize().then(response=>{collator=naturalCollator(response.settings);changedData();applyAppearancePrefs();if(response.warning)errors=[...errors,{path:'Options',error:response.warning}];render();}).catch(error=>{errors=[...errors,{path:'Options',error:String(error)}];render();});
  void Promise.all([optionsReady,versionReady]).then(([,version])=>version?startupUpdates(version):undefined);
  void invoke<typeof runtimePlatform>('runtime_platform').then(async platform=>{runtimePlatform=platform;nativeAppearanceReady=platform.family==='desktop';render();syncNativeAppearance();if(platform.folder_picker_supported)void optionsReady.then(()=>fileOps.initialize()).catch(error=>{errors=[{path:'Directories',error:String(error)}];render();});if(platform.document_access_supported)await run('Restoring recovery copies…',async()=>{const batch=await invoke<Batch>('restore_document_recovery');accept(batch,'Restored recovery copies',false);location='Provider documents';if(!selected.size&&batch.entries.length){selected.add(batch.entries[0].path);anchor=batch.entries[0].path;}if(!batch.entries.length&&!batch.errors.length)status='Ready';});}).catch(error=>{errors=[{path:'Platform',error:String(error)}];render();});
  void listen<{job:string;done:number;total:number}>('opentag://progress',event=>{const job=progressJob;if(!job||event.payload.job!==job.id)return;job.done=event.payload.done;job.total=event.payload.total;showProgress();}).catch(()=>{/* Progress is optional; operations still complete. */});
  void listen('opentag://launch',()=>{if(runtimePlatform.folder_picker_supported)void fileOps.takeLaunches().catch(error=>{errors=[{path:'Command line',error:String(error)}];render();});}).catch(error=>{errors=[{path:'Command line',error:String(error)}];render();});
  void getCurrentWindow().onCloseRequested(async event => {
    if(busy){event.preventDefault();return;}
    if(!drafts.size&&!actionUI.hasUnsavedChanges)return;
    event.preventDefault();
    const message=drafts.size?`Close OpenTag and discard pending changes in ${drafts.size} file${drafts.size===1?'':'s'}${actionUI.hasUnsavedChanges?' and unsaved action group changes':''}?`:'Close OpenTag and discard unsaved action group changes?';
    if(await messages.confirm('unsaved_changes',message,'Unsaved changes')){
      await getCurrentWindow().destroy();
    }
  }).catch(error=>{errors=[{path:'Window',error:String(error)}];render();});
  void getCurrentWebview().onDragDropEvent(event=>{
    document.querySelector('.drop-overlay')?.remove();
    if(!runtimePlatform.document_access_supported&&coverUI.dragDrop(event.payload))return;
    if(event.payload.type==='enter'||event.payload.type==='over'){const overlay=document.createElement('div');overlay.className='drop-overlay';overlay.textContent='Drop audio files, folders or playlists to add them';document.body.append(overlay);}
    if(event.payload.type==='drop'&&!busy&&!runtimePlatform.document_access_supported)void loadPaths(event.payload.paths);
  }).catch(error=>{errors=[{path:'Drag and drop',error:String(error)}];render();});
}
