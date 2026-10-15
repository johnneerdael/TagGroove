export type ActionSpec = { type: string; [key: string]: unknown };
export type ActionGroup = { name: string; enabled: boolean; actions: ActionSpec[] };
export type ActionLibrary = { groups: ActionGroup[]; presets: { name: string; enabled_groups: string[] }[] };
export type ImportedGroup = { group: ActionGroup; issues: { index: number; type_id: string; reason: string }[] };
export type ActionUIHost = {
  load: () => Promise<ActionLibrary>;
  save: (library: ActionLibrary) => Promise<void>;
  import: (path: string, legacyEncoding?: string) => Promise<ImportedGroup>;
  execute: (groups: ActionGroup[]) => Promise<void>;
  pickFile: (kind: 'mta'|'image'|'text'|'export') => Promise<string|null>;
  confirm: (message: string) => Promise<boolean>;
  canExecute: () => boolean;
  executeReason?: () => string;
  selectedCount: () => number;
  fieldNames?: () => string[];
  exportGroup?: (group: ActionGroup) => Promise<string|null>;
  changed: () => void;
  /** Remembers the Actions (Quick) type and settings between uses. */
  storage?: QuickStorage;
};
export type QuickStorage = { getItem: (key: string) => string | null; setItem: (key: string, value: string) => void };
export const actionCatalog = [
  ['adjust_cover','Adjust Cover'],['case_conversion','Case Conversion'],['convert_codepage','Convert Codepage'],
  ['export','Export'],['export_cover','Export Cover to File'],['format_value','Format Value'],
  ['generate_playlists','Generate Playlists'],['guess_values','Guess Values'],['import_cover','Import Cover from File'],
  ['import_text_file','Import Text File'],['merge_duplicate_fields','Merge Duplicate Fields'],['remove_duplicate_fields','Remove Duplicate Fields'],
  ['remove_fields','Remove Fields'],['remove_fields_except','Remove Fields Except'],['replace','Replace'],
  ['regex_replace','Replace with Regular Expression'],['set_cover_properties','Set Cover Properties'],['split_field','Split Field by Separator'],
] as const;
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const clone=<T>(value:T):T=>structuredClone(value);
export function panelKey(entry:{format:string;fields:Record<string,string[]>},key:string):string{
  const existing=(name:string)=>Object.keys(entry.fields).find(k=>k.toUpperCase()===name);
  const vorbis=/flac|ogg|vorbis|opus|speex/i.test(entry.format);
  if(key==='TRACK')return vorbis?(existing('TRACKNUMBER')||'TRACKNUMBER'):(existing('TRACK')||existing('TRACKNUMBER')||'TRACK');
  if(key==='YEAR')return existing('YEAR')||existing('DATE')||(vorbis?'DATE':'YEAR');
  return existing(key)||key;
}
const mac=()=>/Mac|iPhone|iPad/.test(navigator.platform);
export type ActionListKey='new'|'remove'|'toggle'|'enable-all'|'disable-all'|'save'|'up'|'down'|'top'|'bottom'|'select-all'|'invert';
/** Keys of the action group and action lists (docs keyboard → Actions): Insert and Delete
 * (⌫ on Mac keyboards), Space, Ctrl+A / Ctrl+Shift+A, Ctrl+E/D/S (⌘ on macOS) and Alt+↑/↓/Page Up/Page Down. */
export function actionListKey(event:{key:string;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean;altKey:boolean},mac:boolean):ActionListKey|undefined{
  const primary=mac?event.metaKey:event.ctrlKey,other=mac?event.ctrlKey:event.metaKey,key=event.key.length===1?event.key.toLowerCase():event.key;
  if(event.altKey){
    if(primary||other||event.shiftKey)return undefined;
    return ({ArrowUp:'up',ArrowDown:'down',PageUp:'top',PageDown:'bottom'} as Record<string,ActionListKey>)[key];
  }
  if(other)return undefined;
  if(primary&&key==='a')return event.shiftKey?'invert':'select-all';
  if(event.shiftKey)return undefined;
  if(primary)return key==='e'?'enable-all':key==='d'?'disable-all':key==='s'?'save':undefined;
  if(key==='Insert')return 'new';
  if(key==='Delete'||(mac&&key==='Backspace'))return 'remove';
  return key===' '?'toggle':undefined;
}
export function moveItem<T>(items:T[],from:number,to:number):T[]{
  if(from<0||to<0||from>=items.length||to>=items.length)throw new Error('Item index is out of range.');
  const result=[...items],item=result.splice(from,1)[0];result.splice(to,0,item);return result;
}
/**
 * Move the marked items of a multi-selection (Alt+↑/↓/Page Up/Page Down). Single steps
 * keep the marked items' order and stop at the list edges; top and bottom gather them
 * there. Returns the new order and the new indices of the marked items.
 */
export function moveMarked<T>(items:readonly T[],marked:ReadonlySet<number>,move:'up'|'down'|'top'|'bottom'):{items:T[];marked:Set<number>}{
  let rows=items.map((item,index)=>({item,on:marked.has(index)}));
  if(move==='top'||move==='bottom'){const on=rows.filter(row=>row.on),off=rows.filter(row=>!row.on);rows=move==='top'?[...on,...off]:[...off,...on];}
  else{
    const step=move==='up'?-1:1,order=rows.map((_,index)=>index);if(step>0)order.reverse();
    for(const index of order){const target=index+step;if(rows[index].on&&target>=0&&target<rows.length&&!rows[target].on)[rows[index],rows[target]]=[rows[target],rows[index]];}
  }
  return{items:rows.map(row=>row.item),marked:new Set(rows.flatMap((row,index)=>row.on?[index]:[]))};
}
/** Ctrl+A selects every item; Ctrl+Shift+A inverts the selection. */
export function markAll(count:number,marked:ReadonlySet<number>,invert:boolean):Set<number>{
  return new Set(Array.from({length:count},(_,index)=>index).filter(index=>!invert||!marked.has(index)));
}
/** Click (one item), ⌘/Ctrl+click (toggle) and Shift+click (range from the current item). */
export function clickMark(marked:ReadonlySet<number>,current:number,clicked:number,modifiers:{toggle:boolean;range:boolean}):Set<number>{
  if(modifiers.range&&current>=0){const [a,b]=[current,clicked].sort((x,y)=>x-y);return new Set(Array.from({length:b-a+1},(_,i)=>a+i));}
  if(modifiers.toggle){const next=new Set(marked);if(next.has(clicked))next.delete(clicked);else next.add(clicked);return next;}
  return new Set([clicked]);
}
export function renameGroup(library:ActionLibrary,index:number,name:string):ActionLibrary{
  const result=clone(library),group=result.groups[index];if(!group)throw new Error('Action group does not exist.');
  if(!name.trim())throw new Error('Enter an action group name.');
  if(result.groups.some((g,i)=>i!==index&&g.name.toLocaleLowerCase()===name.toLocaleLowerCase()))throw new Error('An action group already has this name.');
  const old=group.name;group.name=name;result.presets.forEach(p=>p.enabled_groups=p.enabled_groups.map(value=>value===old?name:value));return result;
}
export function removeGroup(library:ActionLibrary,index:number):ActionLibrary{
  const result=clone(library),group=result.groups[index];if(!group)throw new Error('Action group does not exist.');result.groups.splice(index,1);result.presets.forEach(p=>p.enabled_groups=p.enabled_groups.filter(name=>name!==group.name));return result;
}
export function applyPreset(library:ActionLibrary,name:string):ActionLibrary{
  const result=clone(library),preset=result.presets.find(p=>p.name===name);if(!preset)throw new Error('Selection preset does not exist.');result.groups.forEach(group=>group.enabled=preset.enabled_groups.includes(group.name));return result;
}
/** Menu markup for one '#' segment with its '&' mnemonic character underlined. */
export function mnemonicMarkup(part:string):string{
  let out='',marked=false;for(let i=0;i<part.length;i++){if(part[i]==='&'&&part[i+1]){if(part[i+1]==='&'){out+='&amp;';i++;}else{const c=html(part[++i]);out+=marked?c:`<u>${c}</u>`;marked=true;}}else out+=html(part[i]);}return out;
}
/** Actions dialog "Sort by name"/"Sort by state": checked groups first, each part kept in order. */
export function sortGroups(library:ActionLibrary,by:'name'|'state'):ActionLibrary{
  const result=clone(library),label=(group:ActionGroup)=>menuSegments(group.name).map(s=>s.label).join('#');
  result.groups=by==='name'?result.groups.sort((a,b)=>label(a).localeCompare(label(b),undefined,{sensitivity:'base',numeric:true})):[...result.groups.filter(g=>g.enabled),...result.groups.filter(g=>!g.enabled)];return result;
}
export function enabledSummary(groups:ActionGroup[]):string{const count=groups.filter(group=>group.enabled).length;return count===0?'No action group enabled':count===1?'1 action group enabled':`${count} action groups enabled`;}
/** Default name for a duplicated group ("Copy of …"), numbered when already taken. */
export function copyName(name:string,groups:ActionGroup[]):string{const taken=(value:string)=>groups.some(group=>group.name.toLocaleLowerCase()===value.toLocaleLowerCase());let candidate=`Copy of ${name}`,n=2;while(taken(candidate))candidate=`Copy of ${name} (${n++})`;return candidate;}
/** Codepages offered by Convert Codepage (Windows identifiers understood by the native converter), including the DOS/OEM pages. */
export const codepageChoices:[string,string][]=[['1250','Central European (Windows-1250)'],['1251','Cyrillic (Windows-1251)'],['1252','Western European (Windows-1252)'],['1253','Greek (Windows-1253)'],['1254','Turkish (Windows-1254)'],['1255','Hebrew (Windows-1255)'],['1256','Arabic (Windows-1256)'],['1257','Baltic (Windows-1257)'],['1258','Vietnamese (Windows-1258)'],['874','Thai (Windows-874)'],['437','OEM United States (CP437)'],['720','Arabic DOS (CP720)'],['737','Greek DOS (CP737)'],['775','Baltic DOS (CP775)'],['850','Western European DOS (CP850)'],['852','Central European DOS (CP852)'],['855','Cyrillic DOS (CP855)'],['857','Turkish DOS (CP857)'],['858','Western European DOS with euro (CP858)'],['860','Portuguese DOS (CP860)'],['861','Icelandic DOS (CP861)'],['862','Hebrew DOS (CP862)'],['863','French Canadian DOS (CP863)'],['864','Arabic DOS (CP864)'],['865','Nordic DOS (CP865)'],['866','Cyrillic DOS (CP866)'],['869','Modern Greek DOS (CP869)'],['932','Japanese (Shift-JIS)'],['936','Simplified Chinese (GBK)'],['949','Korean (CP949)'],['950','Traditional Chinese (Big5)'],['54936','Chinese (GB18030)'],['51932','Japanese (EUC-JP)'],['20866','Cyrillic (KOI8-R)'],['21866','Ukrainian (KOI8-U)'],['10000','Western (Mac Roman)'],['10007','Cyrillic (Mac)'],['28591','Western (ISO-8859-1)'],['28592','Central European (ISO-8859-2)'],['28593','South European (ISO-8859-3)'],['28594','Baltic (ISO-8859-4)'],['28595','Cyrillic (ISO-8859-5)'],['28596','Arabic (ISO-8859-6)'],['28597','Greek (ISO-8859-7)'],['28598','Hebrew (ISO-8859-8)'],['28603','Estonian (ISO-8859-13)'],['28605','Latin 9 (ISO-8859-15)'],['65001','Unicode (UTF-8)']];
/** Adjust Cover quality presets (Best/High/Medium/Low) mapped onto the 0–100 encoder quality. */
export const qualityPresets:[string,string][]=[['100','Best'],['90','High'],['75','Medium'],['50','Low']];
export function menuSegments(name:string):{label:string;mnemonic:string|null}[]{
  return name.split('#').map(part=>{let label='',mnemonic:string|null=null;for(let i=0;i<part.length;i++){if(part[i]==='&'&&part[i+1]){if(part[i+1]==='&'){label+='&';i++;}else{if(!mnemonic)mnemonic=part[i+1].toLocaleLowerCase();label+=part[++i];}}else label+=part[i];}return{label,mnemonic};});
}
export function resolvePathChanges(paths:string[],renamed:{from:string;to:string}[],directories:{from:string;to:string}[]):Map<string,string>{
  const exact=new Map(renamed.map(rename=>[rename.from,rename.to]));
  const rebase=(path:string)=>directories.reduce((current,move)=>{const from=move.from.replace(/[\\/]+$/,''),to=move.to.replace(/[\\/]+$/,'');if(current===from)return to;if(current.startsWith(from)&&(current[from.length]==='/'||current[from.length]==='\\'))return to+current.slice(from.length);return current;},path);
  return new Map(paths.map(path=>[path,exact.get(path)||rebase(path)] as [string,string]).filter(([from,to])=>from!==to));
}
/** List folders after folder renames that changed only the letter case: on case-insensitive volumes they are still the listed folders. */
export function rebaseCaseOnly(folders:string[],directories:{from:string;to:string}[]){
  const changes=resolvePathChanges(folders,[],directories.filter(move=>move.from!==move.to&&move.from.toLowerCase()===move.to.toLowerCase()));
  return folders.map(folder=>changes.get(folder)??folder);
}
const title=(action:ActionSpec)=>actionCatalog.find(([key])=>key===action.type)?.[1]||'Unresolved imported action';
const coverTypes:[string,string][]=[['Other','Other'],['Icon','File icon'],['OtherIcon','Other file icon'],['CoverFront','Front Cover'],['CoverBack','Back Cover'],['Leaflet','Leaflet'],['Media','Media'],['LeadArtist','Lead artist'],['Artist','Artist'],['Conductor','Conductor'],['Band','Band or orchestra'],['Composer','Composer'],['Lyricist','Lyricist'],['RecordingLocation','Recording location'],['DuringRecording','During recording'],['DuringPerformance','During performance'],['ScreenCapture','Screen capture'],['BrightFish','A bright colored fish'],['Illustration','Illustration'],['BandLogo','Band logo'],['PublisherLogo','Publisher logo']];
function defaultAction(type:string):ActionSpec{
  const common={field:'TITLE'};
  switch(type){
    case'adjust_cover':return{type,format:'original',quality:90,resize:null};
    case'case_conversion':return{type,...common,mode:'mixed',word_separators:'(-_'};
    case'convert_codepage':return{type,...common,codepage:'1251'};
    case'export':return{type,template:'',filename_template:'%_folderpath%tags.txt',append:false,partition_template:null};
    case'export_cover':return{type,filename_template:'%_folderpath%cover',picture_type:null,export_duplicates:false};
    case'format_value':return{type,...common,template:'%title%'};
    case'generate_playlists':return{type,filename_template:'%_folderpath%playlist.m3u8',partition_template:'%_folderpath%'};
    case'guess_values':return{type,source:'%title%',pattern:'%artist% / %title%'};
    case'import_cover':return{type,filename_template:'%_folderpath%cover.jpg',picture_type:null,delete_existing:false};
    case'import_text_file':return{type,field:'LYRICS',filename_template:'%_folderpath%lyrics.txt'};
    case'merge_duplicate_fields':return{type,field:'ARTIST',separator:'; '};
    case'remove_duplicate_fields':return{type,field:'_ALL',only_same_content:false};
    case'remove_fields':return{type,fields:['COMMENT']};
    case'remove_fields_except':return{type,fields:['TITLE','ARTIST','ALBUM','ALBUMARTIST','YEAR','DATE','TRACK','TRACKNUMBER','GENRE','COVER']};
    case'replace':return{type,...common,from:'',to:'',whole_word:false,case_sensitive:false};
    case'regex_replace':return{type,...common,pattern:'',replacement:'',case_sensitive:false};
    case'set_cover_properties':return{type,picture_type:null,description:null,only_picture_type:null};
    case'split_field':return{type,field:'ARTIST',separator:';'};
    default:throw new Error('Unknown action type.');
  }
}
const QUICK_KEY='opentag.actions.quick';
type QuickMemory={last?:string;actions:Record<string,ActionSpec>};
function quickMemory(storage?:QuickStorage):QuickMemory{
  try{const value=JSON.parse(storage?.getItem(QUICK_KEY)||'null');if(value&&typeof value==='object'&&value.actions&&typeof value.actions==='object')return{last:typeof value.last==='string'?value.last:undefined,actions:value.actions};}catch{/* A damaged entry falls back to the defaults. */}
  return{actions:{}};
}
/** Actions (Quick) settings last applied for `type` (default: the last type used), over that type's defaults. */
export function recallQuick(storage:QuickStorage|undefined,type?:string):ActionSpec{
  const memory=quickMemory(storage),wanted=type??memory.last;
  const known=(name:unknown):name is string=>actionCatalog.some(([key])=>key===name);
  const chosen=known(wanted)?wanted:'format_value',saved=memory.actions[chosen];
  const base=defaultAction(chosen);
  if(!saved||typeof saved!=='object')return base;
  return Object.fromEntries(Object.keys(base).map(key=>[key,key==='type'||!(key in saved)?base[key]:saved[key]])) as ActionSpec;
}
/** Records the applied quick action; `last` moves only when `asLast` (the cover dialogs share Adjust Cover's settings without changing it). */
export function rememberQuick(storage:QuickStorage|undefined,action:ActionSpec,asLast=true){
  if(!storage||!actionCatalog.some(([key])=>key===action.type))return;
  const memory=quickMemory(storage);memory.actions[action.type]=clone(action);if(asLast)memory.last=action.type;
  try{storage.setItem(QUICK_KEY,JSON.stringify(memory));}catch{/* Preferences are optional. */}
}
function dialog(title:string,content:string,footer:string):HTMLDialogElement{
  const el=document.createElement('dialog');el.innerHTML=`<header>${html(title)}</header><div class="dialog-content">${content}</div><div class="dialog-footer">${footer}</div>`;document.body.append(el);el.addEventListener('close',()=>el.remove());el.showModal();return el;
}
function error(el:HTMLDialogElement,message:unknown){let p=el.querySelector<HTMLElement>('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');el.querySelector('.dialog-content')!.append(p);}p.textContent=String(message);}
function text(label:string,key:string,value:unknown,browse?:string,multiline=false,helper=''){const attrs=helper?` ${helper} spellcheck="false" autocomplete="off"`:'';return`<label class="field"><span>${label}</span><div class="action-input">${multiline?`<textarea data-key="${key}"${attrs}>${html(value)}</textarea>`:`<input data-key="${key}" value="${html(value)}"${attrs}>`}${browse?`<button type="button" data-browse="${browse}" data-target="${key}">Browse…</button>`:''}</div></label>`;}
const format=(history?:string)=>`data-helper="format"${history?` data-history="${history}"`:''}`;
const exportFormat='data-helper="format-export"';
/** Special field names each action's Field drop-down offers, per its documentation page. */
function fieldSpecials(type:string){return type==='case_conversion'?['_TAG','_ALL','_FILENAME','_DIRECTORY','_FIELDNAME']:type==='format_value'?['_FILENAME','_DIRECTORY']:['merge_duplicate_fields','split_field'].includes(type)?['_TAG']:type==='remove_duplicate_fields'?['_ALL']:['replace','regex_replace','convert_codepage'].includes(type)?['_TAG','_ALL','_FILENAME','_DIRECTORY']:[];}
function select(label:string,key:string,value:unknown,options:[string,string][]){return`<label class="field"><span>${label}</span><select data-key="${key}">${options.map(([k,v])=>`<option value="${html(k)}" ${value===k?'selected':''}>${html(v)}</option>`).join('')}</select></label>`;}
function check(label:string,key:string,value:unknown){return`<label class="action-check"><input type="checkbox" data-key="${key}" ${value?'checked':''}>${label}</label>`;}
function controls(action:ActionSpec){
  const field='field'in action?text('FIELD','field',action.field,undefined,false,`data-helper="field" data-specials="${fieldSpecials(action.type).join(',')}"`):'';
  const coverOptions:[string,string][]=[['','〈 keep existing type 〉'],...coverTypes];
  switch(action.type){
    case'adjust_cover':{const resize=action.resize as {pixels:number;size_mode:string;scale_mode:string}|null;const preset=qualityPresets.find(([value])=>Number(value)===action.quality)?.[0]||'custom';return select('IMAGE FORMAT','format',action.format,[['original','Original'],['jpeg','JPEG'],['png','PNG']])+`<div class="field-pair">${select('QUALITY','quality_preset',preset,[...qualityPresets,['custom','Custom']])}<label class="field"><span>QUALITY (0–100)</span><input type="number" data-key="quality" min="0" max="100" value="${html(action.quality)}"></label></div>`+check('Resize cover','resize_enabled',!!resize)+`<div class="resize-options" ${resize?'':'hidden'}>`+`<label class="field"><span>SIZE IN PIXELS</span><input type="number" min="1" data-key="pixels" value="${resize?.pixels||600}"></label>`+select('SIZE POLICY','size_mode',resize?.size_mode||'maximum',[['maximum','Maximum size'],['minimum','Minimum size']])+select('SCALING','scale_mode',resize?.scale_mode||'proportional',[['proportional','Scale proportionally'],['square','Scale to square'],['crop_square','Crop to square']])+'</div>';}
    case'case_conversion':return field+select('CASE','mode',action.mode,[['lower','lower case'],['mixed','Mixed Case'],['sentence','Sentence'],['upper','UPPER CASE']])+text('WORDS BEGIN AFTER','word_separators',action.word_separators);
    case'convert_codepage':{const known=codepageChoices.some(([id])=>id===String(action.codepage));return field+select('CODEPAGE','codepage',String(action.codepage),[...(known?[]:[[String(action.codepage),String(action.codepage)] as [string,string]]),...codepageChoices])+'<p>Choose the codepage the tags were originally written in. Their characters are converted to Unicode.</p>';}
    case'export':return text('EXPORT CONFIGURATION','template',action.template,'export')+text('OUTPUT FILENAME FORMAT','filename_template',action.filename_template,undefined,false,exportFormat)+check('Append data','append',action.append)+check('Create one output file per partition','partition_enabled',action.partition_template!==null)+text('PARTITION FORMAT','partition_template',action.partition_template||'',undefined,false,format());
    case'export_cover':return text('OUTPUT FILENAME FORMAT (EXTENSION IS ADDED)','filename_template',action.filename_template,undefined,false,format('action-cover-file'))+select('EXPORT COVERS OF TYPE','picture_type',action.picture_type||'',[['','All cover types'],...coverTypes])+check('Export duplicate covers','export_duplicates',action.export_duplicates);
    case'format_value':return field+text('FORMAT STRING','template',action.template,undefined,true,format('action-format'));
    case'generate_playlists':return text('PLAYLIST FILENAME FORMAT','filename_template',action.filename_template,undefined,false,exportFormat)+text('PARTITION FORMAT','partition_template',action.partition_template,undefined,false,format())+'<p>Files with the same partition value form one playlist, in File List order.</p>';
    case'guess_values':return text('SOURCE FORMAT STRING','source',action.source,undefined,true,format('action-guess-source'))+text('GUESSING PATTERN','pattern',action.pattern,undefined,false,'data-helper="pattern" data-history="action-guess-pattern"')+'<p>Use %dummy% to skip part of the source.</p>';
    case'import_cover':return text('IMAGE FILENAME FORMAT','filename_template',action.filename_template,'image',false,format('action-cover-file'))+select('IMPORT COVER AS','picture_type',action.picture_type||'CoverFront',coverTypes)+check('Delete existing covers','delete_existing',action.delete_existing);
    case'import_text_file':return field+text('TEXT FILENAME FORMAT','filename_template',action.filename_template,'text',false,format('action-text-file'));
    case'merge_duplicate_fields':case'split_field':return field+text('SEPARATOR','separator',action.separator);
    case'remove_duplicate_fields':return field+check('Only duplicated fields with the same content','only_same_content',action.only_same_content);
    case'remove_fields':case'remove_fields_except':return text(action.type==='remove_fields'?'REMOVE FIELDS (SEPARATE WITH SEMICOLONS)':'KEEP FIELDS (SEPARATE WITH SEMICOLONS)','fields',(action.fields as string[]).join(';'))+'<p>Use COVER or PICTURE to include embedded artwork.</p>';
    case'replace':return field+text('ORIGINAL STRING','from',action.from)+text('REPLACE WITH','to',action.to)+check('Only as whole word','whole_word',action.whole_word)+check('Case-sensitive comparison','case_sensitive',action.case_sensitive);
    case'regex_replace':return field+text('REGULAR EXPRESSION','pattern',action.pattern,undefined,false,'data-helper="regex" data-history="action-regex"')+text('REPLACE MATCHES WITH','replacement',action.replacement,undefined,false,'data-helper="regex-replacement" data-history="action-regex-replacement"')+check('Case-sensitive comparison','case_sensitive',action.case_sensitive);
    case'set_cover_properties':return select('COVER TYPE','picture_type',action.picture_type||'',coverOptions)+check('Keep existing description','description_keep',action.description===null)+text('COVER DESCRIPTION','description',action.description||'',undefined,true)+select('APPLY ONLY TO COVERS OF TYPE','only_picture_type',action.only_picture_type||'',[['','All cover types'],...coverTypes]);
    default:return`<p>${html(action.reason||'This imported action has settings that cannot yet be interpreted.')}</p><pre class="unresolved-settings">${html(JSON.stringify(action.properties||{},null,2))}</pre><p>Select a different action type to replace this record explicitly.</p>`;
  }
}
function readAction(el:HTMLDialogElement,type:string):ActionSpec{
  const result=defaultAction(type);const inputs=new Map([...el.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLTextAreaElement>('[data-key]')].map(input=>[input.dataset.key!,input]));
  for(const [key,input]of inputs){if(['resize_enabled','pixels','size_mode','scale_mode','partition_enabled','description_keep','quality_preset'].includes(key))continue;result[key]=input instanceof HTMLInputElement&&input.type==='checkbox'?input.checked:input instanceof HTMLInputElement&&input.type==='number'?Number(input.value):input.value;}
  if('field'in result){result.field=String(result.field).trim().toUpperCase();if(!result.field)throw new Error('Enter a field name.');}
  if('fields'in result)result.fields=String(result.fields).split(';').map(field=>field.trim().toUpperCase()).filter(Boolean);
  if(type==='adjust_cover'){const quality=Number(result.quality);if(!Number.isInteger(quality)||quality<0||quality>100)throw new Error('Quality must be a whole number between 0 and 100.');const enabled=(inputs.get('resize_enabled') as HTMLInputElement).checked;const pixels=Number(inputs.get('pixels')!.value);if(enabled&&(!Number.isInteger(pixels)||pixels<1))throw new Error('Enter a positive cover size.');result.resize=enabled?{pixels,size_mode:inputs.get('size_mode')!.value,scale_mode:inputs.get('scale_mode')!.value}:null;}
  if(type==='export')result.partition_template=(inputs.get('partition_enabled') as HTMLInputElement).checked?result.partition_template:null;
  if(type==='export_cover'||type==='set_cover_properties'){if(result.picture_type==='')result.picture_type=null;}
  if(type==='import_cover'&&(result.picture_type===''||result.picture_type==='CoverFront'))result.picture_type=null;
  if(type==='set_cover_properties'){if(result.only_picture_type==='')result.only_picture_type=null;if((inputs.get('description_keep') as HTMLInputElement).checked)result.description=null;}
  if(type==='replace'&&!result.from)throw new Error('Enter an original string.');
  if(type==='split_field'&&!result.separator)throw new Error('Enter a separator.');
  if(type==='convert_codepage'&&!String(result.codepage).trim())throw new Error('Enter a codepage.');
  if(type==='guess_values'&&!result.pattern)throw new Error('Enter a guessing pattern.');
  if(type==='export'&&!String(result.template).trim())throw new Error('Select or enter an export configuration.');
  return result;
}

export class ActionUI {
  private library:ActionLibrary={groups:[],presets:[]};
  private loaded=false;
  private loading:Promise<void>|undefined;
  private dirtyGroupDialogs=new Set<HTMLDialogElement>();
  private mnemonicPrefix:string[]=[];
  constructor(private host:ActionUIHost){}
  async initialize(){if(this.loaded)return;if(!this.loading)this.loading=this.host.load().then(library=>{this.library=library;this.loaded=true;this.host.changed();}).finally(()=>this.loading=undefined);await this.loading;}
  get groups(){return this.library.groups;}
  get hasUnsavedChanges(){return this.dirtyGroupDialogs.size>0;}
  menuHTML(){
    const tree=(prefix:string[]):string=>{
      const slots:({index:number;raw:string}|{branch:string;raw:string})[]=[],branches=new Set<string>();
      this.library.groups.forEach((group,index)=>{const segments=menuSegments(group.name),raw=group.name.split('#')[prefix.length];if(!prefix.every((part,i)=>segments[i]?.label===part))return;const segment=segments[prefix.length];if(!segment)return;if(segments.length===prefix.length+1)slots.push({index,raw});else if(!branches.has(segment.label)){branches.add(segment.label);slots.push({branch:segment.label,raw});}});
      return slots.map(slot=>'branch'in slot?`<details class="group-submenu"><summary>${mnemonicMarkup(slot.raw)}</summary><div>${tree([...prefix,slot.branch])}</div></details>`:`<button data-action-group="${slot.index}" ${this.host.canExecute()?'':'disabled'}>${mnemonicMarkup(slot.raw)}</button>`).join('');
    };
    return this.loaded&&this.library.groups.length?`<hr>${tree([])}`:'';
  }
  bindMenu(root:HTMLElement){
    root.querySelectorAll<HTMLButtonElement>('[data-action-group]').forEach(button=>button.addEventListener('click',()=>{root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);void this.runOne(Number(button.dataset.actionGroup));}));
    root.querySelectorAll<HTMLDetailsElement>('.group-submenu').forEach(submenu=>{submenu.addEventListener('toggle',()=>{if(!submenu.open||window.innerWidth<=700)return;submenu.classList.remove('open-left');const popup=submenu.querySelector<HTMLElement>(':scope>div')!;if(popup.getBoundingClientRect().right>window.innerWidth)submenu.classList.add('open-left');});submenu.addEventListener('pointerenter',()=>{if(window.matchMedia('(hover:hover)').matches){submenu.parentElement?.querySelectorAll<HTMLDetailsElement>(':scope>.group-submenu').forEach(other=>{if(other!==submenu)other.open=false;});submenu.open=true;}});submenu.addEventListener('pointerleave',()=>{if(window.matchMedia('(hover:hover)').matches&&!submenu.contains(document.activeElement))submenu.open=false;});});
  }
  async runOne(index:number){if(!this.host.canExecute())return;const group=this.library.groups[index];if(group){try{await this.host.execute([{...clone(group),enabled:true}]);}catch(e){const el=dialog('Action group failed',`<p>${html(e)}</p>`,'<button data-close>Close</button>');el.querySelector('[data-close]')!.addEventListener('click',()=>el.close());}}}
  beginMnemonic(){this.mnemonicPrefix=[];}
  async mnemonic(key:string):Promise<boolean>{
    const groups=this.library.groups.map((group,index)=>({group,index,segments:menuSegments(group.name)})).filter(item=>this.mnemonicPrefix.every((part,i)=>item.segments[i]?.label===part));
    const match=groups.find(item=>item.segments[this.mnemonicPrefix.length]?.mnemonic===key.toLocaleLowerCase());if(!match)return false;
    if(match.segments.length===this.mnemonicPrefix.length+1){this.mnemonicPrefix=[];await this.runOne(match.index);}else this.mnemonicPrefix.push(match.segments[this.mnemonicPrefix.length].label);return true;
  }
  /** `mode`: 'group' edits a group's action, 'quick' applies one action now, 'cover' only collects Adjust Cover settings (Manage Covers). */
  private editAction(original:ActionSpec|undefined,onKeep:(action:ActionSpec)=>Promise<void>|void,mode:'group'|'quick'|'cover'='group'){
    const quick=mode==='quick';
    let current=clone(original||defaultAction('format_value'));const count=this.host.selectedCount();
    const types=mode==='cover'?actionCatalog.filter(([type])=>type==='adjust_cover'):actionCatalog;
    const intro=mode==='cover'?'Resize or convert the selected covers. They are written when you choose OK in Manage covers.':quick?`Apply one action to ${count} selected file${count===1?'':'s'} and save immediately.`:'Configure this action in the reusable group.';
    const el=dialog(mode==='cover'?'Adjust cover':quick?'Actions (Quick)':'Edit action',`<p>${intro}</p><label class="field"><span>ACTION TYPE</span><select id="full-action-type" ${mode==='cover'?'disabled':''}>${current.type==='unresolved_mta'?'<option value="unresolved_mta">Unresolved imported action</option>':''}${types.map(([type,name])=>`<option value="${type}" ${current.type===type?'selected':''}>${name}</option>`).join('')}</select></label><div id="full-action-controls"></div>`,`<button data-close>Cancel</button><button class="primary" data-keep>${mode==='cover'?'Adjust':quick?'Apply action':'Keep action'}</button>`);
    const content=el.querySelector('#full-action-controls')!,typeControl=el.querySelector<HTMLSelectElement>('#full-action-type')!,keep=el.querySelector<HTMLButtonElement>('[data-keep]')!;
    const show=()=>{
      content.innerHTML=controls(current);keep.disabled=current.type==='unresolved_mta'||(quick&&!this.host.canExecute());
      const quality=content.querySelector<HTMLInputElement>('[data-key="quality"]'),preset=content.querySelector<HTMLSelectElement>('[data-key="quality_preset"]');
      if(quality&&preset){preset.addEventListener('change',()=>{if(preset.value!=='custom')quality.value=preset.value;});quality.addEventListener('input',()=>{preset.value=qualityPresets.find(([value])=>value===String(Number(quality.value)))?.[0]||'custom';});}
      content.querySelectorAll<HTMLButtonElement>('[data-browse]').forEach(button=>button.addEventListener('click',async()=>{try{const path=await this.host.pickFile(button.dataset.browse as 'image'|'text'|'export');if(path)content.querySelector<HTMLInputElement>(`[data-key="${button.dataset.target}"]`)!.value=path;}catch(e){error(el,e);}}));content.querySelector<HTMLInputElement>('[data-key="resize_enabled"]')?.addEventListener('change',e=>{content.querySelector<HTMLElement>('.resize-options')!.hidden=!(e.target as HTMLInputElement).checked;});
    };
    show();typeControl.addEventListener('change',()=>{current=mode==='group'?defaultAction(typeControl.value):recallQuick(this.host.storage,typeControl.value);show();el.querySelector('.dialog-error')?.remove();});el.querySelector('[data-close]')!.addEventListener('click',()=>el.close());
    keep.addEventListener('click',async()=>{try{const action=readAction(el,current.type);if(mode!=='group')rememberQuick(this.host.storage,action,quick);keep.disabled=true;await onKeep(action);el.close();}catch(e){error(el,e);keep.disabled=false;}});
    return el;
  }
  /** Actions (Quick) reopens on the last applied type and its settings; `type` preselects one action, e.g. Adjust Cover from the cover window. */
  quick(type?:string){if(this.host.canExecute())this.editAction(recallQuick(this.host.storage,type),action=>this.host.execute([{name:'Quick action',enabled:true,actions:[action]}]),'quick');}
  /** Adjust Cover settings for Manage Covers, or null when cancelled. */
  adjustSettings():Promise<ActionSpec|null>{
    return new Promise(resolve=>{let chosen:ActionSpec|null=null;const el=this.editAction(recallQuick(this.host.storage,'adjust_cover'),action=>{chosen=action;},'cover');el.addEventListener('close',()=>resolve(chosen));});
  }
  async groupsDialog(){
    if(!this.loaded)await this.initialize();let draft=clone(this.library),index=draft.groups.length?0:-1,dirty=false,working=false,groupError='',groupNotice='',renameTimer=0;
    // Selected groups (Ctrl+A, Ctrl+Shift+A, ⌘/Ctrl+click, Shift+click); `index` is the focused one.
    let marked=new Set<number>(index>=0?[index]:[]);
    const block=()=>marked.size>1&&marked.has(index);
    const el=dialog('Action groups','',`<button data-close>Close</button><button id="save-groups">Save groups</button><button class="primary" id="apply-groups">Apply checked groups</button>`);el.classList.add('action-groups-dialog');
    el.addEventListener('close',()=>this.dirtyGroupDialogs.delete(el));
    const content=el.querySelector('.dialog-content')!;
    const mark=()=>{dirty=true;groupError='';groupNotice='';this.dirtyGroupDialogs.add(el);update();};
    const persist=async()=>{await this.host.save(draft);this.library=clone(draft);dirty=false;this.dirtyGroupDialogs.delete(el);this.host.changed();};
    const busyControls=(value:boolean)=>{working=value;el.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=value);if(!value)update();};
    const run=async(operation:()=>Promise<void>)=>{window.clearTimeout(renameTimer);groupError='';groupNotice='';busyControls(true);try{await operation();}catch(e){groupError=String(e);}finally{busyControls(false);}};
    const nameDialog=(heading:string,value:string,apply:(name:string)=>void)=>{const child=dialog(heading,text('NAME','name',value),'<button data-close>Cancel</button><button class="primary" data-keep>Keep name</button>');child.querySelector('[data-close]')!.addEventListener('click',()=>child.close());child.querySelector('[data-keep]')!.addEventListener('click',()=>{try{const name=child.querySelector<HTMLInputElement>('[data-key="name"]')!.value;if(!name.trim())throw new Error('Enter a name.');apply(name);child.close();mark();}catch(e){error(child,e);}});};
    const update=()=>{
      const selected=draft.groups[index];
      marked=new Set([...marked].filter(i=>i<draft.groups.length));if(index>=0&&!marked.has(index))marked=new Set([index]);
      content.innerHTML=`<p>Checked groups run in order on ${this.host.selectedCount()} selected file${this.host.selectedCount()===1?'':'s'}. Save groups to keep your workflows.</p><div class="group-layout"><section class="group-library"><div class="action-section-title">GROUPS</div><p class="group-summary" role="status">${enabledSummary(draft.groups)}</p><div class="group-list" role="listbox" aria-multiselectable="true" aria-label="Action groups">${draft.groups.map((group,i)=>`<div class="group-row ${i===index?'current':''} ${marked.has(i)?'marked':''}" data-group-row="${i}" role="option" aria-selected="${marked.has(i)}" tabindex="0"><input type="checkbox" aria-label="Enable ${html(group.name)}" data-enable="${i}" ${group.enabled?'checked':''}><span>${html(group.name)}</span><small>${group.actions.length}</small></div>`).join('')||'<p class="group-empty">No groups yet. Create a group or import an .mta file.</p>'}</div><div class="group-buttons"><button data-command="new">New…</button><button data-command="duplicate" ${selected?'':'disabled'}>Duplicate…</button><button data-command="rename" ${selected?'':'disabled'}>Rename…</button><button data-command="remove" ${selected?'':'disabled'}>Remove</button><button data-command="up" ${index>0||block()?'':'disabled'}>↑ Move up</button><button data-command="down" ${index>=0&&(index<draft.groups.length-1||block())?'':'disabled'}>↓ Move down</button><button data-command="sort-name" ${draft.groups.length>1?'':'disabled'}>Sort by name</button><button data-command="sort-state" ${draft.groups.length>1?'':'disabled'}>Sort by state</button><button data-command="import">Import .mta…</button><button data-command="export" ${selected&&this.host.exportGroup?'':'disabled'}>Export .mta…</button></div><div class="preset-controls"><label for="group-preset">Selection preset</label><select id="group-preset"><option value="">Choose a preset…</option>${draft.presets.map(p=>`<option value="${html(p.name)}">${html(p.name)}</option>`).join('')}</select><button data-command="save-preset">Save preset…</button><button data-command="remove-preset" ${draft.presets.length?'':'disabled'}>Remove preset…</button></div></section><section class="group-actions"><div class="action-section-title">${selected?html(selected.name):'ACTIONS'}</div><ol class="action-summary-list">${selected?.actions.map((action,i)=>`<li><button data-edit-action="${i}"><strong>${html(title(action))}</strong><span>${html(action.field||action.filename_template||action.pattern||(Array.isArray(action.fields)?action.fields.join('; '):''))}</span></button></li>`).join('')||'<li class="group-empty">Select a group to edit its actions.</li>'}</ol><button data-command="edit" ${selected?'':'disabled'}>Edit actions…</button>${selected?.actions.some(a=>a.type==='unresolved_mta')?'<p class="import-issues">This group contains unresolved imported actions. Edit or replace them before applying.</p>':''}</section></div>`;
      el.querySelector<HTMLButtonElement>('#save-groups')!.disabled=!dirty||working;
      el.querySelector<HTMLButtonElement>('#apply-groups')!.disabled=working||!this.host.canExecute()||!draft.groups.some(group=>group.enabled&&group.actions.length)||draft.groups.some(group=>group.enabled&&group.actions.some(action=>action.type==='unresolved_mta'));
      const blockedReason=this.host.executeReason?.();
      if(blockedReason){const reason=document.createElement('p');reason.className='operation-reason';reason.textContent=blockedReason;content.prepend(reason);}
      if(groupError)error(el,groupError);
      if(groupNotice){const notice=document.createElement('p');notice.className='group-notice';notice.setAttribute('role','status');notice.textContent=groupNotice;content.append(notice);}
      content.querySelectorAll<HTMLElement>('[data-group-row]').forEach(row=>{row.addEventListener('click',event=>{window.clearTimeout(renameTimer);if(event.target instanceof HTMLInputElement)return;const clicked=Number(row.dataset.groupRow),toggle=event.metaKey||event.ctrlKey;if(toggle||event.shiftKey){marked=clickMark(marked,index,clicked,{toggle,range:event.shiftKey});index=clicked;update();content.querySelector<HTMLElement>(`[data-group-row="${index}"]`)?.focus();}else if(clicked===index&&marked.size===1&&(event.target as Element).tagName==='SPAN'){renameTimer=window.setTimeout(()=>{if(el.open&&!working)rename();},600);}else{index=clicked;marked=new Set([clicked]);update();content.querySelector<HTMLElement>(`[data-group-row="${index}"]`)?.focus();}});row.addEventListener('dblclick',()=>{window.clearTimeout(renameTimer);editGroup();});row.addEventListener('keydown',event=>{window.clearTimeout(renameTimer);if(event.key==='F2'){event.preventDefault();rename();}if(event.key==='F4'||event.key==='Enter'){event.preventDefault();editGroup();}if((event.key==='ArrowUp'||event.key==='ArrowDown')&&!event.altKey){event.preventDefault();index=Math.max(0,Math.min(draft.groups.length-1,index+(event.key==='ArrowDown'?1:-1)));marked=new Set([index]);update();content.querySelector<HTMLElement>(`[data-group-row="${index}"]`)?.focus();}});});
      content.querySelectorAll<HTMLInputElement>('[data-enable]').forEach(input=>input.addEventListener('change',()=>{draft.groups[Number(input.dataset.enable)].enabled=input.checked;mark();}));
      content.querySelectorAll<HTMLButtonElement>('[data-edit-action]').forEach(button=>button.addEventListener('click',()=>editGroup(Number(button.dataset.editAction))));
      content.querySelector<HTMLSelectElement>('#group-preset')!.addEventListener('change',event=>{const name=(event.target as HTMLSelectElement).value;if(name){draft=applyPreset(draft,name);mark();}});
      content.querySelectorAll<HTMLButtonElement>('[data-command]').forEach(button=>button.addEventListener('click',event=>command(button.dataset.command!,event.altKey)));
    };
    const rename=()=>{if(index<0)return;nameDialog('Rename action group',draft.groups[index].name,name=>{draft=renameGroup(draft,index,name);});};
    el.addEventListener('keydown',event=>{
      const target=event.target as Element,inList=!!target.closest?.('.group-list'),key=working?undefined:actionListKey(event,mac());
      if(!key||(!inList&&!['enable-all','disable-all','save'].includes(key))||(key==='toggle'&&target instanceof HTMLInputElement))return;
      event.preventDefault();event.stopPropagation();
      if(key==='save'){if(dirty)void run(persist);return;}
      if(key==='select-all'||key==='invert'){marked=markAll(draft.groups.length,marked,key==='invert');if(!marked.has(index))index=marked.size?Math.min(...marked):index;if(!marked.size)marked=new Set();update();content.querySelector<HTMLElement>(`[data-group-row="${index}"]`)?.focus();return;}
      if(key==='enable-all'||key==='disable-all'){draft.groups.forEach(group=>group.enabled=key==='enable-all');mark();}
      else if(key==='toggle'){if(index<0)return;const on=!draft.groups[index].enabled;(block()?[...marked]:[index]).forEach(i=>draft.groups[i].enabled=on);mark();}
      else if(key==='new'||key==='remove')command(key);
      else command(key==='up'||key==='top'?'up':'down',key==='top'||key==='bottom');
      if(inList&&el.open)content.querySelector<HTMLElement>(`[data-group-row="${index}"]`)?.focus();
    });
    const editGroup=(actionIndex=0)=>{
      if(index<0)return;const group=draft.groups[index],groupDraft=clone(group);let selectedAction=Math.min(actionIndex,groupDraft.actions.length-1),markedActions=new Set<number>(selectedAction>=0?[selectedAction]:[]);
      const actionBlock=()=>markedActions.size>1&&markedActions.has(selectedAction);
      const child=dialog(`Actions — ${group.name}`,'','<button data-close>Cancel</button><button class="primary" data-keep>Keep group</button>');child.classList.add('group-editor-dialog');
      const draw=()=>{markedActions=new Set([...markedActions].filter(i=>i<groupDraft.actions.length));if(selectedAction>=0&&!markedActions.has(selectedAction))markedActions=new Set([selectedAction]);
        child.querySelector('.dialog-content')!.innerHTML=`<ol class="action-edit-list" aria-label="Actions in execution order">${groupDraft.actions.map((action,i)=>`<li><button data-action-index="${i}" class="${i===selectedAction?'current':''} ${markedActions.has(i)?'marked':''}" aria-pressed="${markedActions.has(i)}"><strong>${html(title(action))}</strong><span>${html(action.field||action.filename_template||action.pattern||'')}</span></button></li>`).join('')||'<li class="group-empty">Add an action to this group.</li>'}</ol><div class="group-buttons"><button data-command="new">New…</button><button data-command="edit" ${selectedAction>=0?'':'disabled'}>Edit…</button><button data-command="duplicate" ${selectedAction>=0?'':'disabled'}>Duplicate</button><button data-command="remove" ${selectedAction>=0?'':'disabled'}>Remove</button><button data-command="up" ${selectedAction>0||actionBlock()?'':'disabled'}>↑ Move up</button><button data-command="down" ${selectedAction>=0&&(selectedAction<groupDraft.actions.length-1||actionBlock())?'':'disabled'}>↓ Move down</button></div>`;
        child.querySelectorAll<HTMLButtonElement>('[data-action-index]').forEach(button=>{button.addEventListener('click',event=>{const clicked=Number(button.dataset.actionIndex),toggle=event.metaKey||event.ctrlKey;markedActions=clickMark(markedActions,selectedAction,clicked,{toggle,range:event.shiftKey});selectedAction=clicked;draw();child.querySelector<HTMLElement>(`[data-action-index="${selectedAction}"]`)?.focus();});button.addEventListener('dblclick',()=>edit());});
        child.querySelectorAll<HTMLButtonElement>('[data-command]').forEach(button=>button.addEventListener('click',event=>{const command=button.dataset.command;if(command==='new')this.editAction(undefined,action=>{groupDraft.actions.push(action);selectedAction=groupDraft.actions.length-1;draw();});if(command==='edit')edit();if(command==='duplicate'){groupDraft.actions.splice(selectedAction+1,0,clone(groupDraft.actions[selectedAction]));selectedAction++;draw();}if(command==='remove'){const removed=actionBlock()?[...markedActions].sort((a,b)=>b-a):[selectedAction],first=Math.min(...removed);removed.forEach(i=>groupDraft.actions.splice(i,1));selectedAction=Math.min(first,groupDraft.actions.length-1);markedActions=new Set(selectedAction>=0?[selectedAction]:[]);draw();}if(command==='up'||command==='down'){if(actionBlock()){const current=groupDraft.actions[selectedAction],moved=moveMarked(groupDraft.actions,markedActions,event.altKey?(command==='up'?'top':'bottom'):command);groupDraft.actions=moved.items;markedActions=moved.marked;selectedAction=groupDraft.actions.indexOf(current);}else{const target=event.altKey?(command==='up'?0:groupDraft.actions.length-1):selectedAction+(command==='up'?-1:1);groupDraft.actions=moveItem(groupDraft.actions,selectedAction,target);selectedAction=target;markedActions=new Set([target]);}draw();}}));
      };
      const edit=()=>{if(selectedAction>=0)this.editAction(groupDraft.actions[selectedAction],action=>{groupDraft.actions[selectedAction]=action;draw();});};
      const commandButton=(name:string,extreme=false)=>child.querySelector<HTMLButtonElement>(`[data-command="${name}"]:not(:disabled)`)?.dispatchEvent(new MouseEvent('click',{altKey:extreme}));
      child.querySelector('[data-close]')!.addEventListener('click',()=>child.close());child.querySelector('[data-keep]')!.addEventListener('click',()=>{draft.groups[index]=groupDraft;mark();child.close();});
      child.addEventListener('keydown',event=>{
        if(event.key==='F4'){event.preventDefault();edit();return;}
        const command=(event.target as Element).closest?.('.action-edit-list')||event.key==='Insert'?actionListKey(event,mac()):undefined;
        if(command==='select-all'||command==='invert'){event.preventDefault();event.stopPropagation();markedActions=markAll(groupDraft.actions.length,markedActions,command==='invert');if(!markedActions.has(selectedAction)&&markedActions.size)selectedAction=Math.min(...markedActions);draw();child.querySelector<HTMLElement>(`[data-action-index="${selectedAction}"]`)?.focus();return;}
        if(!command||!['new','remove','up','down','top','bottom'].includes(command))return;
        event.preventDefault();event.stopPropagation();
        commandButton(command==='top'?'up':command==='bottom'?'down':command,command==='top'||command==='bottom');
        child.querySelector<HTMLElement>(`[data-action-index="${selectedAction}"]`)?.focus();
      });draw();
    };
    const command=(name:string,extreme=false)=>{
      window.clearTimeout(renameTimer);
      if(!block()&&((name==='up'&&index<=0)||(name==='down'&&(index<0||index>=draft.groups.length-1))))return;
      if(name==='new')nameDialog('New action group','',value=>{if(draft.groups.some(g=>g.name.toLocaleLowerCase()===value.toLocaleLowerCase()))throw new Error('An action group already has this name.');draft.groups.push({name:value,enabled:true,actions:[]});index=draft.groups.length-1;});
      if(name==='duplicate'&&index>=0)nameDialog('Duplicate action group',copyName(draft.groups[index].name,draft.groups),value=>{const group=clone(draft.groups[index]);if(draft.groups.some(g=>g.name.toLocaleLowerCase()===value.toLocaleLowerCase()))throw new Error('An action group already has this name.');group.name=value;draft.groups.push(group);index=draft.groups.length-1;});
      if(name==='rename')rename();
      if(name==='remove'&&index>=0){const removed=block()?[...marked].sort((a,b)=>b-a):[index],first=Math.min(...removed);removed.forEach(i=>{draft=removeGroup(draft,i);});index=Math.min(first,draft.groups.length-1);marked=new Set(index>=0?[index]:[]);mark();}
      if((name==='up'||name==='down')&&index>=0){
        if(block()){const current=draft.groups[index],moved=moveMarked(draft.groups,marked,extreme?(name==='up'?'top':'bottom'):name);draft.groups=moved.items;marked=moved.marked;index=draft.groups.indexOf(current);}
        else{const target=extreme?(name==='up'?0:draft.groups.length-1):index+(name==='up'?-1:1);draft.groups=moveItem(draft.groups,index,target);index=target;marked=new Set([target]);}
        mark();
      }
      if(name==='edit')editGroup();
      if(name==='sort-name'||name==='sort-state'){const current=draft.groups[index];draft=sortGroups(draft,name==='sort-name'?'name':'state');index=current?draft.groups.findIndex(group=>group.name===current.name):-1;mark();}
      if(name==='export'&&index>=0&&this.host.exportGroup){const group=clone(draft.groups[index]);void run(async()=>{const path=await this.host.exportGroup!(group);if(path)groupNotice=`Exported ${group.name} to ${path}`;});}
      if(name==='save-preset')nameDialog('Save selection preset','',value=>{const preset={name:value,enabled_groups:draft.groups.filter(g=>g.enabled).map(g=>g.name)},old=draft.presets.findIndex(p=>p.name===value);if(old>=0)draft.presets[old]=preset;else draft.presets.push(preset);});
      if(name==='remove-preset'){const child=dialog('Remove selection preset',select('PRESET','preset',draft.presets[0]?.name,draft.presets.map(p=>[p.name,p.name])), '<button data-close>Cancel</button><button data-remove>Remove preset</button>');child.querySelector('[data-close]')!.addEventListener('click',()=>child.close());child.querySelector('[data-remove]')!.addEventListener('click',()=>{const selected=child.querySelector<HTMLSelectElement>('[data-key="preset"]')!.value;draft.presets=draft.presets.filter(p=>p.name!==selected);mark();child.close();});}
      if(name==='import')void run(async()=>{const path=await this.host.pickFile('mta');if(!path)return;const child=dialog('Import action group',`<p>${html(path)}</p>${text('ORIGINAL LEGACY ENCODING (BLANK FOR UTF-8 OR UTF-16 BOM)','encoding','')}<p>For older action files, enter the original codepage such as Windows-1251 or Windows-1252.</p>`,'<button data-close>Cancel</button><button class="primary" data-import>Import group</button>');child.querySelector('[data-close]')!.addEventListener('click',()=>child.close());const button=child.querySelector<HTMLButtonElement>('[data-import]')!;button.addEventListener('click',async()=>{button.disabled=true;try{const encoding=child.querySelector<HTMLInputElement>('[data-key="encoding"]')!.value.trim();const imported=await this.host.import(path,encoding||undefined);let name=imported.group.name,suffix=2;while(draft.groups.some(g=>g.name.toLocaleLowerCase()===name.toLocaleLowerCase()))name=`${imported.group.name} (${suffix++})`;draft.groups.push({...imported.group,name});index=draft.groups.length-1;dirty=true;this.dirtyGroupDialogs.add(el);groupError=imported.issues.map(issue=>`Action ${issue.index+1}: ${issue.reason}`).join('\n');update();child.close();}catch(e){error(child,e);}finally{button.disabled=false;}});});
    };
    const close=async()=>{window.clearTimeout(renameTimer);if(working)return;if(!dirty||await this.host.confirm('Discard unsaved changes to action groups and selection presets?'))el.close();};
    el.addEventListener('cancel',event=>{event.preventDefault();void close();});el.querySelector('[data-close]')!.addEventListener('click',()=>void close());el.querySelector('#save-groups')!.addEventListener('click',()=>void run(persist));el.querySelector('#apply-groups')!.addEventListener('click',()=>void run(async()=>{if(dirty)await persist();await this.host.execute(clone(draft.groups));el.close();}));update();
  }
}
