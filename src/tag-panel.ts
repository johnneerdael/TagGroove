import {ValueSuggestions,suggestionText} from './value-suggestions';
export type Control='text'|'check'|'multiline';
export type Size='full'|'larger'|'large'|'medium'|'normal'|'small'|'smaller'|'tiny';
export type PanelField={name:string;field:string;type:Control;size:Size;rows:number;default_value:string;visible:boolean};
export type PanelLayout={fields:PanelField[];show_panel:boolean;show_cover:boolean};
export type FieldValues={lists:Record<string,string[]>;only_user_genres:boolean;prefer_tag_values:boolean;prefer_tag_fields?:Record<string,boolean>};
export type ImportedFieldValues={lists:Record<string,string[]>;prefer_tag_fields:Record<string,boolean>};
export type TagPanelSettings={version:number;panel:PanelLayout;field_values:FieldValues};
export type ComboOption={kind:'keep'|'blank'|'value'|'browse';value:string;values?:readonly string[];group:'special'|'selection'|'custom'|'genre'|'directory'};
export const KEEP='< keep >',BLANK='< blank >',DIRECTORY='_DIRECTORY',SEPARATOR='_SEPARATOR';
export const SIZES:[Size,string][]=[['full','Full'],['larger','Larger'],['large','Large'],['medium','Medium'],['normal','Normal'],['small','Small'],['smaller','Smaller'],['tiny','Tiny']];
// Spans of a 12-column panel grid; consecutive fields share a row while they fit.
export const sizeSpan:Record<Size,number>={full:12,larger:10,large:9,medium:8,normal:6,small:4,smaller:3,tiny:2};
export const CONTROLS:[Control,string][]=[['text','Text'],['check','Check'],['multiline','Multiline']];
export const COMMON_FIELDS=['TITLE','ARTIST','ALBUM','ALBUMARTIST','YEAR','TRACK','DISCNUMBER','GENRE','COMMENT','COMPOSER','CONDUCTOR','LYRICIST','PUBLISHER','COPYRIGHT','ISRC','BPM','COMPILATION','MOOD','LANGUAGE','ENCODEDBY','ALBUMSORT','ARTISTSORT','ALBUMARTISTSORT','TITLESORT','UNSYNCEDLYRICS',DIRECTORY,SEPARATOR];
// ID3v1 genres with the Winamp extensions, as offered by tag editors' genre lists.
// i18n-ignore: genre names are tag values, written to files as they are.
export const STANDARD_GENRES=['Blues','Classic Rock','Country','Dance','Disco','Funk','Grunge','Hip-Hop','Jazz','Metal','New Age','Oldies','Other','Pop','R&B','Rap','Reggae','Rock','Techno','Industrial','Alternative','Ska','Death Metal','Pranks','Soundtrack','Euro-Techno','Ambient','Trip-Hop','Vocal','Jazz+Funk','Fusion','Trance','Classical','Instrumental','Acid','House','Game','Sound Clip','Gospel','Noise','Alternative Rock','Bass','Soul','Punk','Space','Meditative','Instrumental Pop','Instrumental Rock','Ethnic','Gothic','Darkwave','Techno-Industrial','Electronic','Pop-Folk','Eurodance','Dream','Southern Rock','Comedy','Cult','Gangsta','Top 40','Christian Rap','Pop/Funk','Jungle','Native American','Cabaret','New Wave','Psychedelic','Rave','Showtunes','Trailer','Lo-Fi','Tribal','Acid Punk','Acid Jazz','Polka','Retro','Musical','Rock & Roll','Hard Rock','Folk','Folk-Rock','National Folk','Swing','Fast Fusion','Bebop','Latin','Revival','Celtic','Bluegrass','Avantgarde','Gothic Rock','Progressive Rock','Psychedelic Rock','Symphonic Rock','Slow Rock','Big Band','Chorus','Easy Listening','Acoustic','Humour','Speech','Chanson','Opera','Chamber Music','Sonata','Symphony','Booty Bass','Primus','Porn Groove','Satire','Slow Jam','Club','Tango','Samba','Folklore','Ballad','Power Ballad','Rhythmic Soul','Freestyle','Duet','Punk Rock','Drum Solo','A Cappella','Euro-House','Dance Hall','Goa','Drum & Bass','Club-House','Hardcore Techno','Terror','Indie','BritPop','Polsk Punk','Beat','Christian Gangsta Rap','Heavy Metal','Black Metal','Crossover','Contemporary Christian','Christian Rock','Merengue','Salsa','Thrash Metal','Anime','JPop','Synthpop','Abstract','Art Rock','Baroque','Bhangra','Big Beat','Breakbeat','Chillout','Downtempo','Dub','EBM','Eclectic','Electro','Electroclash','Emo','Experimental','Garage','Global','IDM','Illbient','Industro-Goth','Jam Band','Krautrock','Leftfield','Lounge','Math Rock','New Romantic','Nu-Breakz','Post-Punk','Post-Rock','Psytrance','Shoegaze','Space Rock','Trop Rock','World Music','Neoclassical','Audiobook','Audio Theatre','Neue Deutsche Welle','Podcast','Indie Rock','G-Funk','Dubstep','Garage Rock','Psybient'];
const collator=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const clone=<T>(value:T):T=>structuredClone(value);
const panelField=(name:string,field:string,size:Size='full',extra:Partial<PanelField>={}):PanelField=>({name,field,type:'text',size,rows:3,default_value:KEEP,visible:true,...extra});

export function defaultLayout():PanelLayout{
  return {fields:[panelField('Tit&le','TITLE'),panelField('Art&ist','ARTIST'),panelField('&Album','ALBUM'),panelField('&Year','YEAR','normal'),panelField('T&rack','TRACK','normal'),panelField('&Genre','GENRE'),panelField('Co&mment','COMMENT','full',{type:'multiline'}),panelField('Album Artist','ALBUMARTIST'),panelField('Composer','COMPOSER'),panelField('Discnumber','DISCNUMBER','normal'),panelField('&Directory',DIRECTORY)],show_panel:true,show_cover:true};
}
export function defaultSettings():TagPanelSettings{return {version:1,panel:defaultLayout(),field_values:{lists:{},only_user_genres:false,prefer_tag_values:false}};}
export const normalizeField=(id:string)=>id.trim().toUpperCase();
const controlCharacters=(value:string,multiline=false)=>[...value].some(c=>/\p{Cc}/u.test(c)&&!(multiline&&/[\n\r\t]/.test(c)));
function fieldError(id:string,panel:boolean):string|null{
  if(!id||id!==normalizeField(id))return`Field "${id}" must be a trimmed uppercase field name.`;
  if(controlCharacters(id)||/[=%]/.test(id))return`Field "${id}" contains an invalid character.`;
  if(id.length>1000)return'Field name is too long.';
  if(id.startsWith('_')&&!(panel&&(id===DIRECTORY||id===SEPARATOR)))return`${id}: technical fields cannot be edited in the tag panel.`;
  return null;
}
/** Mirrors the backend validation so the editor can report problems before saving. */
export function layoutError(layout:PanelLayout):string|null{
  if(layout.fields.length>100)return'The tag panel supports at most 100 fields.';
  const used=new Set<string>();
  for(const entry of layout.fields){
    const error=fieldError(entry.field,true);if(error)return error;
    const separator=entry.field===SEPARATOR;
    if(!separator&&!entry.name.trim())return`${entry.field}: enter a name for the field.`;
    if(controlCharacters(entry.name)||controlCharacters(entry.default_value,entry.type==='multiline'))return`${entry.name||entry.field}: remove control characters.`;
    if(entry.name.length>1000||entry.default_value.length>1000)return`${entry.name||entry.field}: text is too long.`;
    if(!Number.isInteger(entry.rows)||entry.rows<1||entry.rows>20)return`${entry.name}: rows must be between 1 and 20.`;
    if(!SIZES.some(([size])=>size===entry.size)||!CONTROLS.some(([control])=>control===entry.type))return`${entry.name}: unsupported field size or type.`;
    if(!separator&&used.has(entry.field))return`${entry.field} is already used for another field and cannot be added twice.`;
    used.add(entry.field);
  }
  return null;
}
export function settingsError(settings:TagPanelSettings):string|null{
  if(settings.version!==1)return'Unsupported tag panel configuration version.';
  const layout=layoutError(settings.panel);if(layout)return layout;
  for(const [key,values] of Object.entries(settings.field_values.lists)){
    const error=fieldError(key,false);if(error)return error;
    if(values.length>5000)return`${key}: at most 5000 custom values are supported.`;
    if(values.some(value=>!value||controlCharacters(value)))return`${key}: custom values must be nonempty single lines.`;
    if(new Set(values).size!==values.length)return`${key}: remove duplicate custom values.`;
  }
  for(const [key,value] of Object.entries(settings.field_values.prefer_tag_fields??{})){
    const error=fieldError(key,false);if(error)return error;
    if(typeof value!=='boolean')return'Field-value preferences must be true or false.';
  }
  return null;
}
/** Native imports replace only their named fields; unrelated lists/defaults remain intact. */
export function withImportedFieldValues(settings:TagPanelSettings,values?:ImportedFieldValues):TagPanelSettings{
  const next=clone(settings);if(!values)return next;
  next.field_values.lists={...next.field_values.lists,...clone(values.lists)};
  if(Object.keys(values.prefer_tag_fields).length)next.field_values.prefer_tag_fields={...next.field_values.prefer_tag_fields,...values.prefer_tag_fields};
  return next;
}
/** Trim, drop empty lines and exact duplicates, keep the user's order. */
export function normalizeValues(values:string[]):string[]{return[...new Set(values.map(value=>value.trim()).filter(Boolean))];}
/** Editor lines are literal; omit only empty lines and exact duplicates. */
export const parseValueList=(text:string)=>[...new Set(text.split(/\r?\n/).filter(value=>value.length>0))];
export function withCustomValue(settings:TagPanelSettings,field:string,value:string):TagPanelSettings{
  const key=normalizeField(field),text=value.trim(),result=clone(settings);
  if(!text||/[\r\n]/.test(text))throw new Error('Enter a single-line value to add it to the custom values.');
  const error=fieldError(key,false);if(error)throw new Error(error);
  const list=result.field_values.lists[key]||[];if(!list.includes(text))result.field_values.lists[key]=[...list,text];return result;
}
export function withLists(settings:TagPanelSettings,lists:Record<string,string[]>):TagPanelSettings{
  const result=clone(settings);result.field_values.lists=Object.fromEntries(Object.entries(lists).map(([key,values])=>[normalizeField(key),normalizeValues(values)]).filter(([,values])=>values.length));return result;
}
export function moveField(layout:PanelLayout,from:number,to:number):PanelLayout{
  if(from<0||to<0||from>=layout.fields.length||to>=layout.fields.length)throw new Error('Field index is out of range.');
  const result=clone(layout),[item]=result.fields.splice(from,1);result.fields.splice(to,0,item);return result;
}
/** `&` marks the Alt shortcut letter; `&&` is a literal ampersand. */
export function mnemonic(name:string):{label:string;key?:string;html:string}{
  let label='',markup='',key:string|undefined;
  for(let i=0;i<name.length;i++){
    const c=name[i];
    if(c==='&'&&name[i+1]==='&'){label+='&';markup+='&amp;';i++;continue;}
    if(c==='&'&&i+1<name.length&&!key&&/[\p{L}\p{N}]/u.test(name[i+1])){key=name[i+1].toUpperCase();label+=name[i+1];markup+=`<u>${html(name[i+1])}</u>`;i++;continue;}
    if(c==='&')continue;
    label+=c;markup+=html(c);
  }
  return {label,key,html:markup};
}
/**
 * A field label in the UI language: `translate` gets the label without its `&`.
 * The Alt letter stays the configured one and is underlined where the translation has it.
 */
let defaultNames:Map<string,string>|undefined;
/** Whether a field still has its default label, which follows the UI language; a label someone typed is shown as typed. */
export function defaultName(field:{field:string;name:string}){
  defaultNames??=new Map(defaultLayout().fields.map(f=>[f.field,f.name]));
  return defaultNames.get(field.field)===field.name;
}
export function localMnemonic(name:string,translate?:(label:string)=>string):{label:string;key?:string;html:string}{
  const original=mnemonic(name),label=translate?translate(original.label):original.label;
  if(label===original.label)return original;
  const chars=[...label],at=original.key?chars.findIndex(c=>c.toUpperCase()===original.key):-1;
  return {label,key:original.key,html:chars.map((c,i)=>i===at?`<u>${html(c)}</u>`:html(c)).join('')};
}
export const splitValues=(text:string)=>text?text.split('\\\\').map(value=>value.trim()):[];
/** Panel text for the selection: shared value, or empty plus mixed when files differ. */
type PanelValue=string|readonly string[];
const panelValueText=(value:PanelValue)=>typeof value==='string'?value:value.join(' \\\\ ');
const panelValueArray=(value:PanelValue)=>typeof value==='string'?[value]:[...value];
export function selectionState(values:PanelValue[]):{value:string;mixed:boolean}{
  const identity=JSON.stringify(values.length?panelValueArray(values[0]):[]);
  const mixed=values.some(value=>JSON.stringify(panelValueArray(value))!==identity);return {value:mixed?'':values.length?panelValueText(values[0]):'',mixed};
}
const truthy=(value:string)=>!!value.trim()&&!/^(0|false|no)$/i.test(value.trim());
export function checkState(values:string[]):'on'|'off'|'mixed'{const on=values.filter(truthy).length;return on&&on===values.length?'on':on?'mixed':'off';}
/** Dropdown entries: keep/blank, the selection's values, custom values and (for GENRE) the genre list. */
export function comboOptions(options:{field:string;values:PanelValue[];custom?:string[];onlyUserGenres?:boolean;preferTagValues?:boolean}):ComboOption[]{
  const result:ComboOption[]=[],seen=new Set<string>();
  const add=(kind:ComboOption['kind'],value:string,group:ComboOption['group'],values:readonly string[]=[value])=>{
    const identity=JSON.stringify(values);if(kind==='value'){if(!value||seen.has(identity))return;seen.add(identity);}
    result.push({kind,value,group,...(kind==='value'?{values:[...values]}:{})});
  };
  if(options.values.length>1)add('keep',KEEP,'special');
  if(options.values.length)add('blank',BLANK,'special');
  const selection=options.values.map(value=>({value:panelValueText(value),values:panelValueArray(value)})).filter(option=>!!option.value).sort((a,b)=>collator.compare(a.value,b.value));
  selection.forEach(option=>add('value',option.value,'selection',option.values));
  if(options.preferTagValues&&selection.length)return result;
  (options.custom||[]).forEach(value=>add('value',value,'custom'));
  if(normalizeField(options.field)==='GENRE'&&!options.onlyUserGenres)[...STANDARD_GENRES].sort(collator.compare).forEach(value=>add('value',value,'genre'));
  return result;
}
/** Panel input text for a combo choice; null means restore every file's own value. */
export function choiceText(option:ComboOption):string|null{return option.kind==='keep'?null:option.kind==='blank'?'':option.value;}
/** Value that "Default values in input fields" stages, or null when the field keeps its values. */
export function defaultText(field:PanelField):string|null{
  if(field.field===DIRECTORY||field.field===SEPARATOR||field.default_value===KEEP)return null;
  return field.default_value===BLANK?'':field.default_value;
}
/**
 * Default values filled in automatically when several files are selected
 * (Customize tag panel → Default value): every visible field whose default is not
 * < keep > and that has no pending edit yet. Returns [field, text] pairs.
 */
export function automaticDefaults(fields:PanelField[],pending:(field:string)=>boolean):[string,string][]{
  return fields.filter(field=>field.visible).flatMap(field=>{const text=defaultText(field);return text===null||pending(field.field)?[]:[[field.field,text] as [string,string]];});
}
/** The folder typed into the _DIRECTORY field, or undefined when it is empty or unchanged. */
export function typedDirectory(text:string,shown:string):string|undefined{
  const path=text.trim().replace(/^"(.*)"$/,'$1');
  return path&&path!==shown.trim()?path:undefined;
}
export function directoryOptions(directories:string[]):ComboOption[]{
  return [...[...new Set(directories)].sort(collator.compare).map(value=>({kind:'value' as const,value,group:'directory' as const})),{kind:'browse',value:'Open folder…',group:'special'}];
}

/** Fields of an Mp3tag tag panel configuration, with the entries that were left out. */
export type Mp3tagPanelImport={panel:PanelLayout;field_values?:ImportedFieldValues;skipped:string[];skipped_diagnostics?:unknown};
export type TagPanelHost={
  /** Translate new popup content before reading layout dimensions. */
  translateNow?:(root:HTMLElement)=>void;
  persistent:()=>boolean;
  load:()=>Promise<TagPanelSettings>;save:(settings:TagPanelSettings)=>Promise<void>;
  importLayout:(path:string)=>Promise<PanelLayout>;exportLayout:(path:string,layout:PanelLayout)=>Promise<void>;
  /** Utils → Load configuration of an Mp3tag file the user picks (usrfields.ini or a saved panel configuration). */
  importMp3tag?:(path:string)=>Promise<Mp3tagPanelImport>;
  /** Skipped import lines in the UI language; entries keep the label written in the file. */
  skippedLines?:(imported:Mp3tagPanelImport)=>string[];
  /** UI-language text of a structured native failure (an import that yields no field); other errors use their own message. */
  batchError?:(error:unknown)=>string|undefined;
  importList:(path:string)=>Promise<string[]>;exportList:(path:string,values:string[])=>Promise<void>;
  pickOpen:(kind:'layout'|'list'|'mp3tag')=>Promise<string|null>;pickSave:(kind:'layout'|'list',name:string)=>Promise<string|null>;
  /** Horizontal (top or bottom) panels: item indexes per column, or undefined for the vertical layout. */
  columns?:(items:{span:number;height:number;separator:boolean}[])=>number[][]|undefined;
  /** Docking position: the choices, the current one and how to change it. */
  dock?:{choices:[string,string][];current:()=>string;set:(side:string)=>void};
  values:(field:string)=>PanelValue[];disabled:()=>boolean;
  directories:()=>{selected:string[];all:string[]};selectDirectory:(directory:string)=>void;openFolder:()=>void;
  /** Field labels in the UI language (Options → Language); configured names are shown as they are without it. */
  translateLabel?:(label:string)=>string;
  /** A folder path typed into the _DIRECTORY field: make it the working directory. */
  changeDirectory:(path:string)=>void;
  draft:(field:string,value:string,exactValues?:readonly string[])=>void;keep:(field:string)=>void;
  changed:()=>void;notify:(message:string)=>void;fail:(error:unknown)=>void;
  /** Options → General → Auto-completion at selection boxes: the completer while it is enabled. */
  autocomplete?:()=>((input:HTMLInputElement|HTMLTextAreaElement,event:Event,candidates:()=>Iterable<string>)=>boolean)|undefined;
  /** Whether an Alt chord is a shortcut rather than typed text (macOS Option in text fields, AltGr). */
  altShortcut:(event:KeyboardEvent)=>boolean;
};
const groupLabels:Record<ComboOption['group'],string>={special:'',selection:'Selected files',custom:'Custom values',genre:'Genres',directory:'Loaded folders'};
type OpenList={input:HTMLInputElement|HTMLTextAreaElement;field:PanelField;options:ComboOption[];active:number;popup:HTMLElement};

export class TagPanel {
  settings=defaultSettings();
  private list?:OpenList;
  private suggestions:ValueSuggestions[]=[];
  // Bound-control origins include pending values shown at render time. Own typing
  // changes their observed signature, not the candidate inventory. Rerenders reset it.
  private valueOrigins=new Map<number,{field:string;values:string[][];observed:string}>();
  private contextPoint?:{x:number;y:number};
  private dialogOpen=false;
  constructor(private host:TagPanelHost){}
  get visible(){return this.settings.panel.show_panel;}
  get showCover(){return this.settings.panel.show_cover;}
  async initialize(){if(!this.host.persistent())return;this.settings=await this.host.load();this.host.changed();}
  private async persist(next:TagPanelSettings,message?:string){
    const error=settingsError(next);if(error)throw new Error(error);
    if(this.host.persistent())await this.host.save(next);
    this.settings=next;this.host.changed();if(message)this.host.notify(message);
  }
  private update(change:(settings:TagPanelSettings)=>void,message?:string){const next=clone(this.settings);change(next);void this.persist(next,message).catch(error=>this.host.fail(error));}
  toggle(){this.update(settings=>{settings.panel.show_panel=!settings.panel.show_panel;},this.visible?'Tag panel hidden. Press Ctrl+Q to show it.':'Tag panel shown');}
  applyDefaults(){
    const staged=this.settings.panel.fields.filter(field=>field.visible).map(field=>[field.field,defaultText(field)] as const).filter((item):item is readonly [string,string]=>item[1]!==null);
    if(!staged.length){this.host.notify('Every tag panel field keeps its values. Set default values in Customize tag panel.');return;}
    staged.forEach(([field,value])=>this.host.draft(field,value));this.host.changed();this.host.notify(`Default values are pending for ${staged.length} field${staged.length===1?'':'s'}. Save tags to write them.`);
  }
  titleHTML(){
    const fields=this.settings.panel.fields.map((field,index)=>[field,index] as const).filter(([field])=>field.field!==SEPARATOR);
    const item=(command:string,label:string,checked?:boolean,name=false)=>`<button type="button" data-panel-command="${command}" ${checked===undefined?'':`role="menuitemcheckbox" aria-checked="${checked}"`}><span>${checked===undefined?'':checked?'✓ ':'<span class="panel-check-space"></span>'}${name?`<span translate="no">${html(label)}</span>`:html(label)}</span></button>`;
    const dock=this.host.dock,position=dock?`<hr><div class="menu-heading">Position</div>${dock.choices.map(([side,label])=>item(`dock:${side}`,label,dock.current()===side)).join('')}`:'';
    return `<details class="menu panel-menu"><summary aria-label="Tag panel options" title="Tag panel options">⋯</summary><div class="menu-list">${item('configure','Customize tag panel…')}${item('values','Field values…')}${item('defaults','Default values in input fields')}<hr>${fields.map(([field,index])=>item(`field:${index}`,this.fieldLabel(field).label||field.field,field.visible,true)).join('')}${item('cover','Display cover art',this.showCover)}<hr>${item('save','Save configuration…')}${item('load','Load configuration…')}${this.host.importMp3tag?item('import-mp3tag','Import Mp3tag configuration…'):''}${position}<hr>${item('hide','Hide tag panel')}</div></details>`;
  }
  fieldsHTML(){
    this.closeList();this.suggestions.forEach(list=>list.dispose());this.suggestions=[];this.valueOrigins.clear();
    const disabled=this.host.disabled(),fields=this.settings.panel.fields;
    const shown=fields.map((field,index)=>({field,index})).filter(({field})=>field.visible);
    // Rough heights of a label with its control, for filling the columns of a horizontal panel.
    const columns=this.host.columns?.(shown.map(({field})=>({span:sizeSpan[field.size],separator:field.field===SEPARATOR,height:field.type==='multiline'?30+field.rows*17:field.type==='check'?32:52})));
    if(!columns)return `<div class="panel-grid">${shown.map(({field,index})=>this.fieldHTML(field,index,disabled)).join('')}</div>`;
    return `<div class="panel-columns">${columns.map(column=>`<div class="panel-grid">${column.map(item=>this.fieldHTML(shown[item].field,shown[item].index,disabled)).join('')}</div>`).join('')}</div>`;
  }
  private fieldLabel(field:PanelField){return localMnemonic(field.name,defaultName(field)?this.host.translateLabel:undefined);}
  private fieldHTML(field:PanelField,index:number,disabled:boolean){
    if(field.field===SEPARATOR)return'<div class="panel-separator" role="separator"></div>';
    const id=`tag-panel-${index}`,label=this.fieldLabel(field),span=`style="grid-column:span ${sizeSpan[field.size]}"`,off=disabled?'disabled':'';
    // The underlined mnemonic splits the label into several text nodes; name controls explicitly.
    // Labels are names (translate="no", data-verbatim): only an untouched default label is translated, above.
    const shortcut=`data-verbatim="${html(label.label)}" aria-label="${html(label.label)}" ${label.key?`aria-keyshortcuts="Alt+${html(label.key)}"`:''}`,keep=this.host.translateLabel?.(KEEP)??KEEP;
    if(field.field===DIRECTORY){
      const {selected}=this.host.directories(),state=selectionState(selected);
      return `<div class="field panel-field" ${span}><label for="${id}" translate="no">${label.html}</label><div class="combo"><input id="${id}" data-panel-index="${index}" data-directory role="combobox" aria-haspopup="listbox" aria-expanded="false" aria-autocomplete="none" value="${html(state.value)}" data-shown="${html(state.value)}" placeholder="${state.mixed?keep:''}" title="${html(state.value)} — type a folder and press Enter to open it" autocomplete="off" spellcheck="false" ${shortcut}><button type="button" class="combo-toggle" data-combo="${index}" tabindex="-1" aria-label="Show folders">▾</button></div></div>`;
    }
    const values=this.host.values(field.field),state=selectionState(values);
    if(field.type==='check'){
      const check=checkState(values.map(panelValueText));
      return `<div class="panel-field panel-check-field" ${span}><label class="action-check"><input type="checkbox" id="${id}" data-check="${html(field.field)}" data-panel-index="${index}" ${check==='on'?'checked':''} data-mixed="${check==='mixed'}" ${off} ${shortcut}> <span translate="no">${label.html}</span></label></div>`;
    }
    const origins=values.map(panelValueArray);this.valueOrigins.set(index,{field:field.field,values:origins,observed:JSON.stringify(origins)});
    const placeholder=state.mixed?keep:'',inline=field.type==='text'&&!!this.host.autocomplete?.();
    const common=`id="${id}" data-field="${html(field.field)}" data-panel-index="${index}" role="combobox" aria-haspopup="listbox" aria-expanded="false" aria-autocomplete="${inline?'inline':'none'}" placeholder="${placeholder}" autocomplete="off" ${off} ${shortcut}`;
    const control=field.type==='multiline'?`<textarea ${common} rows="${field.rows}">${html(state.value)}</textarea>`:`<input ${common} value="${html(state.value)}">`;
    return `<div class="field panel-field" ${span}><label for="${id}" translate="no">${label.html}</label><div class="combo ${field.type==='multiline'?'multiline':''}">${control}<button type="button" class="combo-toggle" data-combo="${index}" tabindex="-1" data-verbatim="${html(label.label)}" aria-label="Show values for ${html(label.label)}" ${off}>▾</button></div></div>`;
  }
  bind(root:HTMLElement){
    root.querySelectorAll<HTMLInputElement>('input[data-check]').forEach(box=>{
      box.indeterminate=box.dataset.mixed==='true';
      box.addEventListener('change',()=>{box.indeterminate=false;box.dataset.mixed='false';this.host.draft(box.dataset.check!,box.checked?'1':'');});
    });
    root.querySelectorAll<HTMLButtonElement>('.combo-toggle').forEach(button=>button.addEventListener('pointerdown',event=>event.preventDefault()));
    root.querySelectorAll<HTMLButtonElement>('.combo-toggle').forEach(button=>button.addEventListener('click',()=>{
      const input=button.parentElement!.querySelector<HTMLInputElement|HTMLTextAreaElement>('[data-panel-index]')!;
      if(this.list?.input===input){this.closeList();return;}
      input.focus({preventScroll:true});this.openList(input);
    }));
    root.querySelectorAll<HTMLInputElement|HTMLTextAreaElement>('.combo [data-panel-index]').forEach(input=>{
      input.addEventListener('keydown',event=>this.comboKey(event as KeyboardEvent,input));
      input.addEventListener('input',()=>this.closeList());
      input.addEventListener('compositionstart',()=>this.closeList());
      const field=this.settings.panel.fields[Number(input.dataset.panelIndex)];
      if(input.dataset.field&&field)this.suggestions.push(new ValueSuggestions(input,{
        candidates:()=>this.optionsFor(field,input).filter(option=>option.kind==='value').map(option=>({value:option.value,values:option.values,group:this.host.translateLabel?.(groupLabels[option.group])??groupLabels[option.group]})),
        enabled:()=>!!this.host.autocomplete?.()&&!this.host.disabled(),
        inline:field.type==='text'?(control,event,candidates)=>this.host.autocomplete?.()?.(control,event,candidates)??false:undefined,
        beforeOpen:()=>this.closeList(),manualList:true,
        choose:(value,values)=>{this.host.draft(field.field,value,values);this.host.changed();},
      }));
      input.addEventListener('blur',()=>{if(this.list?.input===input)this.closeList();});
    });
    const menu=root.querySelector<HTMLDetailsElement>('.panel-menu');
    if(menu){
      menu.addEventListener('toggle',()=>{
        if(!menu.open)return;const popup=menu.querySelector<HTMLElement>('.menu-list')!;this.host.translateNow?.(popup);const rect=menu.getBoundingClientRect(),point=this.contextPoint;this.contextPoint=undefined;
        const x=point?point.x:rect.right-popup.offsetWidth,y=point?point.y:rect.bottom+2;
        popup.style.top=`${Math.max(8,Math.min(y,window.innerHeight-popup.offsetHeight-8))}px`;popup.style.left=`${Math.max(8,Math.min(x,window.innerWidth-popup.offsetWidth-8))}px`;
        if(point)popup.querySelector<HTMLElement>('button')?.focus({preventScroll:true});
      });
      menu.querySelectorAll<HTMLButtonElement>('[data-panel-command]').forEach(button=>button.addEventListener('click',()=>{menu.open=false;void this.command(button.dataset.panelCommand!).catch(error=>this.host.fail(error));}));
    }
  }
  /** Document-level shortcuts: Ctrl+Q toggles the panel, Alt+mnemonic focuses a field
   * (Control+Option+mnemonic while typing on macOS, where Option+letter types). */
  install(root:Document){
    root.addEventListener('keydown',event=>{
      if(event.isComposing||event.defaultPrevented||root.querySelector('dialog[open]'))return;
      if(event.ctrlKey&&!event.metaKey&&!event.altKey&&!event.shiftKey&&event.code==='KeyQ'){event.preventDefault();this.toggle();return;}
      if(!event.altKey||!this.visible||!this.host.altShortcut(event))return;
      const letter=/^Key([A-Z])$|^Digit([0-9])$/.exec(event.code);if(!letter)return;
      const key=letter[1]||letter[2],index=this.settings.panel.fields.findIndex(field=>field.visible&&field.field!==SEPARATOR&&mnemonic(field.name).key===key);
      const target=index<0?null:root.querySelector<HTMLElement>(`[data-panel-index="${index}"]`);
      if(target&&!(target as HTMLInputElement).disabled&&target.offsetParent!==null){event.preventDefault();target.focus();}
    });
    root.addEventListener('pointerdown',event=>{
      const target=event.target as Element;
      if(this.list&&!this.list.popup.contains(target)&&!target.closest?.('.combo'))this.closeList();
      const menu=root.querySelector<HTMLDetailsElement>('.panel-menu[open]');if(menu&&!menu.contains(target))menu.open=false;
    });
    root.addEventListener('scroll',event=>{if(this.list&&!this.list.popup.contains(event.target as Node))this.closeList();},true);
    // Right-click on the tag panel opens its menu (fields to show, configurations). Text
    // fields keep the system edit menu and the cover window keeps its own menu.
    root.addEventListener('contextmenu',event=>{
      const target=event.target as HTMLElement;
      if(!target.closest?.('.editor')||target.closest('.cover-section,.panel-menu')||target.matches('input:not([type=checkbox]),textarea'))return;
      const menu=root.querySelector<HTMLDetailsElement>('.editor .panel-menu');if(!menu)return;
      event.preventDefault();this.closeList();this.contextPoint={x:event.clientX,y:event.clientY};menu.open=true;
    });
    window.addEventListener('resize',()=>this.closeList());
  }
  private comboKey(event:KeyboardEvent,input:HTMLInputElement|HTMLTextAreaElement){
    if(event.isComposing)return;
    const open=this.list?.input===input,multiline=input instanceof HTMLTextAreaElement;
    if(event.key==='ArrowDown'||event.key==='ArrowUp'){
      if(!open){if(multiline&&!event.altKey)return;event.preventDefault();this.openList(input,event.key==='ArrowUp'?-1:1);return;}
      event.preventDefault();this.moveActive(event.key==='ArrowDown'?1:-1);return;
    }
    if(open&&(event.key==='Home'||event.key==='End')&&event.altKey){event.preventDefault();this.setActive(event.key==='Home'?0:this.list!.options.length-1);return;}
    if(event.key==='Escape'&&open){event.preventDefault();event.stopPropagation();this.closeList();return;}
    if(event.key==='Enter'&&open&&this.list!.active>=0){event.preventDefault();this.choose(this.list!.active);return;}
    if(event.key==='Enter'&&event.shiftKey&&input.dataset.field){event.preventDefault();this.addCurrent(input);return;}
    if(input.dataset.directory!==undefined){
      if(event.key==='Enter'){event.preventDefault();const path=typedDirectory(input.value,input.dataset.shown||'');if(path)this.host.changeDirectory(path);return;}
      if(event.key==='Escape'&&input.value!==(input.dataset.shown||'')){event.preventDefault();event.stopPropagation();input.value=input.dataset.shown||'';return;}
    }
    if(event.key==='Tab'&&open)this.closeList();
  }
  /** Record the existing draft publisher's result without reading candidates (including IME input). */
  inputDrafted(input:HTMLInputElement|HTMLTextAreaElement,values:readonly string[]){
    const origin=this.valueOrigins.get(Number(input.dataset.panelIndex));
    if(origin&&origin.field===input.dataset.field)origin.observed=JSON.stringify(origin.values.map(()=>values));
  }
  private optionsFor(field:PanelField,input?:HTMLInputElement|HTMLTextAreaElement):ComboOption[]{
    if(field.field===DIRECTORY)return directoryOptions(this.host.directories().all);
    let values=this.host.values(field.field);const fv=this.settings.field_values;
    if(input){
      const index=Number(input.dataset.panelIndex),observed=JSON.stringify(values.map(panelValueArray));
      let origin=this.valueOrigins.get(index);
      // A real source change replaces origins; only the input publisher may mark
      // an observed change as this control's own draft publication.
      if(!origin||origin.field!==field.field||origin.observed!==observed){origin={field:field.field,values:values.map(panelValueArray),observed};this.valueOrigins.set(index,origin);}
      values=origin.values;
    }
    return comboOptions({field:field.field,values,custom:fv.lists[field.field],onlyUserGenres:fv.only_user_genres,preferTagValues:fv.prefer_tag_fields?.[field.field]??fv.prefer_tag_values});
  }
  private openList(input:HTMLInputElement|HTMLTextAreaElement,step=0){
    this.closeList();this.suggestions.forEach(list=>list.close());
    const field=this.settings.panel.fields[Number(input.dataset.panelIndex)];if(!field||input.disabled)return;
    const options=this.optionsFor(field,input),popup=document.createElement('div');
    // Group headings, Open folder… and No values are UI text; the values themselves and the field label are not translated.
    const label=this.fieldLabel(field).label;
    popup.className='combo-list';popup.id='tag-panel-listbox';popup.setAttribute('role','listbox');if(label)popup.dataset.verbatim=label;popup.setAttribute('aria-label',`${label} values`);
    let group:ComboOption['group']|undefined,markup='';
    options.forEach((option,index)=>{
      if(option.group!==group){if(group!==undefined)markup+='</div>';group=option.group;markup+=`<div role="group" ${groupLabels[group]?`aria-label="${groupLabels[group]}"`:''}>${groupLabels[group]?`<div class="combo-group" aria-hidden="true">${groupLabels[group]}</div>`:''}`;}
      markup+=`<div role="option" id="tag-panel-option-${index}" data-option="${index}" class="${option.kind==='value'?'':'special'}" ${option.kind==='value'?'translate="no"':''} aria-selected="false">${html(suggestionText(option))}</div>`;
    });
    popup.innerHTML=options.length?`${markup}</div>`:'<div class="combo-empty">No values</div>';
    popup.addEventListener('pointerdown',event=>event.preventDefault());
    popup.addEventListener('click',event=>{const option=(event.target as Element).closest<HTMLElement>('[data-option]');if(option)this.choose(Number(option.dataset.option));});
    document.body.append(popup);this.host.translateNow?.(popup);
    const rect=(input.closest('.combo')||input).getBoundingClientRect(),below=window.innerHeight-rect.bottom-8,above=rect.top-8;
    popup.style.left=`${Math.max(4,Math.min(rect.left,window.innerWidth-Math.max(rect.width,200)-4))}px`;popup.style.minWidth=`${Math.max(rect.width,200)}px`;
    if(below<160&&above>below){popup.style.bottom=`${window.innerHeight-rect.top+2}px`;popup.style.maxHeight=`${Math.min(300,above)}px`;}
    else{popup.style.top=`${rect.bottom+2}px`;popup.style.maxHeight=`${Math.max(120,Math.min(300,below))}px`;}
    input.setAttribute('aria-controls',popup.id);input.setAttribute('aria-expanded','true');
    const current=input.dataset.directory!==undefined?-1:options.findIndex(option=>option.kind==='value'&&option.value===input.value);
    this.list={input,field,options,active:-1,popup};
    this.setActive(current>=0?current:step>0?0:step<0?options.length-1:-1);
  }
  closeList(){
    const list=this.list;if(!list)return;this.list=undefined;list.popup.remove();
    list.input.setAttribute('aria-expanded','false');list.input.removeAttribute('aria-activedescendant');list.input.removeAttribute('aria-controls');
  }
  private setActive(index:number){
    const list=this.list;if(!list)return;list.active=index;
    list.popup.querySelectorAll<HTMLElement>('[data-option]').forEach(el=>{const active=Number(el.dataset.option)===index;el.classList.toggle('active',active);el.setAttribute('aria-selected',String(active));if(active)el.scrollIntoView({block:'nearest'});});
    if(index>=0)list.input.setAttribute('aria-activedescendant',`tag-panel-option-${index}`);else list.input.removeAttribute('aria-activedescendant');
  }
  private moveActive(step:number){const list=this.list;if(!list||!list.options.length)return;const next=list.active<0?(step>0?0:list.options.length-1):Math.max(0,Math.min(list.options.length-1,list.active+step));this.setActive(next);}
  private choose(index:number){
    const list=this.list;if(!list)return;const option=list.options[index];if(!option)return;
    const {input,field}=list;this.closeList();
    if(field.field===DIRECTORY){if(option.kind==='browse')this.host.openFolder();else this.host.selectDirectory(option.value);return;}
    const text=choiceText(option);
    if(text===null)this.host.keep(field.field);else{input.value=text;this.host.draft(field.field,text,option.kind==='blank'?[]:option.values??[text]);}
    this.host.changed();
  }
  private addCurrent(input:HTMLInputElement|HTMLTextAreaElement){
    const field=input.dataset.field!,value=input.value.trim();
    if(!value){this.host.notify('Type a value first, then press Shift+Enter to add it to the custom values.');return;}
    let next:TagPanelSettings;try{next=withCustomValue(this.settings,field,value);}catch(error){this.host.fail(error);return;}
    if(next.field_values.lists[field]?.length===this.settings.field_values.lists[field]?.length){this.host.notify(`“${value}” is already a custom value for ${field}.`);return;}
    void this.persist(next,`Added “${value}” to the custom values for ${field}.`).catch(error=>this.host.fail(error));
  }
  private async command(command:string){
    if(command==='configure')return this.configure();
    if(command==='values')return this.fieldValues();
    if(command==='defaults')return this.applyDefaults();
    if(command==='hide')return this.toggle();
    if(command==='cover')return this.update(settings=>{settings.panel.show_cover=!settings.panel.show_cover;});
    if(command.startsWith('field:')){const index=Number(command.slice(6));return this.update(settings=>{const field=settings.panel.fields[index];if(field)field.visible=!field.visible;});}
    if(command==='save'){const path=await this.host.pickSave('layout','tag-panel.json');if(path){await this.host.exportLayout(path,this.settings.panel);this.host.notify('Tag panel configuration saved.');}return;}
    if(command==='load'){const path=await this.host.pickOpen('layout');if(!path)return;const panel=await this.host.importLayout(path);await this.persist({...clone(this.settings),panel},'Tag panel configuration loaded.');}
    if(command==='import-mp3tag')return this.configure(true);
    if(command.startsWith('dock:'))this.host.dock?.set(command.slice(5));
  }
  private dialog(title:string,className:string,content:string,footer:string){
    const dialog=document.createElement('dialog');dialog.className=className;dialog.setAttribute('aria-label',title);
    dialog.innerHTML=`<header>${html(title)}</header><div class="dialog-content">${content}</div><div class="dialog-footer">${footer}</div>`;
    document.body.append(dialog);this.dialogOpen=true;
    dialog.addEventListener('close',()=>{this.dialogOpen=false;dialog.remove();});dialog.showModal();return dialog;
  }
  private static error(dialog:HTMLDialogElement,error:unknown,batchError?:(error:unknown)=>string|undefined){let p=dialog.querySelector<HTMLElement>('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(p);}const shown=batchError?.(error);if(shown!==undefined){p.setAttribute('translate','no');p.textContent=shown;}else{p.removeAttribute('translate');p.textContent=String(error instanceof Error?error.message:error);}}
  /** Options → Tag Panel: fields list with New/Remove/Move and a field editor. */
  /** `importMp3tag` opens the dialog with the Mp3tag import started. */
  configure(importMp3tag=false){
    if(this.dialogOpen)return;
    let layout=clone(this.settings.panel),current=layout.fields.length?0:-1,working=false;
    let importedValues:ImportedFieldValues|undefined;
    const dialog=this.dialog('Customize tag panel','tag-panel-dialog',`<p>Choose the fields shown on the tag panel. Unchecked fields are hidden. Use &amp; in a name to set its Alt shortcut letter; ${DIRECTORY} adds a folder switcher and ${SEPARATOR} starts a new row (a new column when the panel is docked above or below the file list).</p><div class="panel-config"><div><div class="panel-config-list" role="listbox" aria-label="Tag panel fields" tabindex="0"></div><div class="group-buttons"><button type="button" id="tp-new">New</button><button type="button" id="tp-remove">Remove</button><button type="button" id="tp-up" aria-label="Move up">Move up</button><button type="button" id="tp-down" aria-label="Move down">Move down</button><button type="button" id="tp-reset">Restore defaults</button></div></div><div class="panel-config-editor"></div></div><label class="action-check"><input type="checkbox" id="tp-cover"> Display cover art</label><div class="tp-import-report" role="status"></div><datalist id="tp-fields">${COMMON_FIELDS.map(field=>`<option value="${field}">`).join('')}</datalist><datalist id="tp-defaults"><option value="${KEEP}"><option value="${BLANK}"></datalist>`,
      `<button type="button" id="tp-save-file">Save configuration…</button><button type="button" id="tp-load-file">Load configuration…</button>${this.host.importMp3tag?'<button type="button" id="tp-import-mp3tag">Import Mp3tag configuration…</button>':''}<button type="button" data-close>Cancel</button><button type="button" class="primary" id="tp-ok">OK</button>`);
    const list=dialog.querySelector<HTMLElement>('.panel-config-list')!,editor=dialog.querySelector<HTMLElement>('.panel-config-editor')!,cover=dialog.querySelector<HTMLInputElement>('#tp-cover')!;
    const on=(id:string,handler:()=>void)=>dialog.querySelector(`#${id}`)!.addEventListener('click',handler);
    const renderList=()=>{
      list.innerHTML=layout.fields.map((field,index)=>{const name=this.fieldLabel(field).label;return`<div class="panel-config-row ${index===current?'selected':''}" role="option" aria-selected="${index===current}" id="tp-row-${index}" data-row="${index}"><input type="checkbox" data-visible="${index}" ${field.visible?'checked':''} data-verbatim="${html(name||field.field)}" aria-label="Show ${html(name||field.field)}" tabindex="-1"><span ${name?'translate="no"':''}>${html(name||(field.field===SEPARATOR?'Separator':''))}</span><small translate="no">${html(field.field)}</small></div>`;}).join('');
      if(current>=0)list.setAttribute('aria-activedescendant',`tp-row-${current}`);else list.removeAttribute('aria-activedescendant');
      list.querySelector('.selected')?.scrollIntoView({block:'nearest'});
      list.querySelectorAll<HTMLInputElement>('[data-visible]').forEach(box=>box.addEventListener('change',()=>{layout.fields[Number(box.dataset.visible)].visible=box.checked;}));
      for(const [id,disabled] of [['tp-remove',current<0],['tp-up',current<1],['tp-down',current<0||current>=layout.fields.length-1]] as const)dialog.querySelector<HTMLButtonElement>(`#${id}`)!.disabled=working||disabled;
    };
    const renderEditor=()=>{
      const field=layout.fields[current];
      if(!field){editor.innerHTML='<p>Add a field with New.</p>';return;}
      editor.innerHTML=`<label class="field"><span>Name</span><input id="tp-name" value="${html(field.name)}"></label><label class="field"><span>Field</span><input id="tp-field" list="tp-fields" value="${html(field.field)}" autocapitalize="characters" spellcheck="false"></label><label class="field"><span>Default value</span><input id="tp-default" list="tp-defaults" value="${html(field.default_value)}"></label><div class="field-pair"><label class="field"><span>Type</span><select id="tp-type">${CONTROLS.map(([value,label])=>`<option value="${value}" ${field.type===value?'selected':''}>${label}</option>`).join('')}</select></label><label class="field"><span>Size of field</span><select id="tp-size">${SIZES.map(([value,label])=>`<option value="${value}" ${field.size===value?'selected':''}>${label}</option>`).join('')}</select></label></div><label class="field"><span>Rows</span><input id="tp-rows" type="number" min="1" max="20" value="${field.rows}" ${field.type==='multiline'?'':'disabled'}></label>`;
    };
    const sync=()=>{
      const field=layout.fields[current];if(!field)return;
      const value=(id:string)=>editor.querySelector<HTMLInputElement|HTMLSelectElement>(`#${id}`)!.value;
      field.name=value('tp-name');field.field=normalizeField(value('tp-field'));field.default_value=value('tp-default');field.type=value('tp-type') as Control;field.size=value('tp-size') as Size;field.rows=Number(value('tp-rows'))||1;
      editor.querySelector<HTMLInputElement>('#tp-rows')!.disabled=field.type!=='multiline';
      const row=list.querySelector(`[data-row="${current}"]`);if(row){
        const name=this.fieldLabel(field).label,label=row.querySelector('span')!,box=row.querySelector('input')!;
        label.textContent=name||(field.field===SEPARATOR?'Separator':'');if(name)label.setAttribute('translate','no');else label.removeAttribute('translate');
        box.setAttribute('data-verbatim',name||field.field);box.setAttribute('aria-label',`Show ${name||field.field}`);row.querySelector('small')!.textContent=field.field;
      }
    };
    editor.addEventListener('input',sync);editor.addEventListener('change',sync);
    const select=(index:number)=>{current=Math.max(-1,Math.min(layout.fields.length-1,index));renderList();renderEditor();};
    const renderAll=()=>{cover.checked=layout.show_cover;select(current);};
    list.addEventListener('click',event=>{const row=(event.target as Element).closest<HTMLElement>('[data-row]');if(row&&!(event.target as Element).matches('input'))select(Number(row.dataset.row));});
    list.addEventListener('keydown',event=>{
      if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();if(event.altKey){const to=current+(event.key==='ArrowDown'?1:-1);if(current>=0&&to>=0&&to<layout.fields.length){layout=moveField(layout,current,to);select(to);}}else select(current+(event.key==='ArrowDown'?1:-1));}
      if(event.key===' '&&current>=0){event.preventDefault();layout.fields[current].visible=!layout.fields[current].visible;renderList();}
    });
    on('tp-new',()=>{let n=1;while(layout.fields.some(field=>field.field===`FIELD${n}`))n++;layout.fields.splice(current+1,0,panelField('New field',`FIELD${n}`));select(current+1);editor.querySelector<HTMLInputElement>('#tp-name')?.select();});
    on('tp-remove',()=>{if(current<0)return;layout.fields.splice(current,1);select(Math.min(current,layout.fields.length-1));});
    on('tp-up',()=>{if(current>0){layout=moveField(layout,current,current-1);select(current-1);}});
    on('tp-down',()=>{if(current>=0&&current<layout.fields.length-1){layout=moveField(layout,current,current+1);select(current+1);}});
    on('tp-reset',()=>{layout={...defaultLayout(),show_panel:layout.show_panel};importedValues=undefined;current=0;renderAll();});
    cover.addEventListener('change',()=>{layout.show_cover=cover.checked;});
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();dialog.querySelectorAll<HTMLButtonElement>('.dialog-footer button').forEach(button=>button.disabled=true);
      try{await work();}catch(error){if(dialog.open)TagPanel.error(dialog,error,this.host.batchError);}finally{working=false;if(dialog.open){dialog.querySelectorAll<HTMLButtonElement>('.dialog-footer button').forEach(button=>button.disabled=false);renderList();}}
    };
    on('tp-save-file',()=>void perform(async()=>{const error=layoutError(layout);if(error)throw new Error(error);const path=await this.host.pickSave('layout','tag-panel.json');if(path)await this.host.exportLayout(path,layout);}));
    on('tp-load-file',()=>void perform(async()=>{const path=await this.host.pickOpen('layout');if(!path)return;layout={...await this.host.importLayout(path),show_panel:layout.show_panel};importedValues=undefined;current=layout.fields.length?0:-1;if(dialog.open)renderAll();}));
    // Mp3tag's own file (usrfields.ini or a saved configuration): the fields replace the list for review; OK saves them.
    const importMp3tagFile=()=>void perform(async()=>{
      const read=this.host.importMp3tag;if(!read)return;const path=await this.host.pickOpen('mp3tag');if(!path)return;
      const imported=await read(path);if(!dialog.open)return;
      importedValues=imported.field_values;
      layout={...imported.panel,show_panel:layout.show_panel,show_cover:layout.show_cover};current=layout.fields.length?0:-1;renderAll();
      const report=dialog.querySelector<HTMLElement>('.tp-import-report')!,count=imported.panel.fields.filter(field=>field.field!==SEPARATOR).length;
      const total=Object.keys(importedValues?.prefer_tag_fields??{}).length;
      report.innerHTML=`<p>Imported ${count} field${count===1?'':'s'} from the Mp3tag configuration. Press OK to use them.</p>${total?`<p>Field-value settings: ${total} field${total===1?'':'s'}. Imported entries replace matching saved lists and preferences when you choose OK.</p>`:''}${imported.skipped.length?`<p>Not imported:</p><ul translate="no">${(this.host.skippedLines?.(imported)??imported.skipped).map(entry=>`<li>${html(entry)}</li>`).join('')}</ul>`:''}`;
    });
    if(this.host.importMp3tag)on('tp-import-mp3tag',importMp3tagFile);
    on('tp-ok',()=>void perform(async()=>{const error=layoutError(layout);if(error)throw new Error(error);await this.persist(withImportedFieldValues({...clone(this.settings),panel:clone(layout)},importedValues),'Tag panel configuration saved.');dialog.close();}));
    dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
    dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());
    renderAll();list.focus();
    if(importMp3tag)importMp3tagFile();
  }
  /** Options → Field Values: per-field custom value lists, genre options, import/export. */
  fieldValues(initialField?:string){
    if(this.dialogOpen)return;
    const lists=clone(this.settings.field_values.lists),fv=this.settings.field_values,preferences=clone(fv.prefer_tag_fields??{});
    const panelFields=this.settings.panel.fields.map(field=>field.field).filter(field=>!field.startsWith('_'));
    let current=normalizeField(initialField||Object.keys(lists)[0]||'GENRE'),working=false,shown='';
    const dialog=this.dialog('Field values','field-values-dialog',`<p>Custom values appear in the tag panel’s drop-down lists. Enter one value per line. Press Shift+Enter in a tag panel field to add its value here.</p><div class="field-pair"><label class="field"><span>Field</span><input id="fv-field" list="fv-fields" autocapitalize="characters" spellcheck="false"></label><div class="field"><span>Configured fields</span><select id="fv-used" aria-label="Configured fields"></select></div></div><datalist id="fv-fields">${[...new Set([...panelFields,...COMMON_FIELDS.filter(field=>!field.startsWith('_'))])].map(field=>`<option value="${html(field)}">`).join('')}</datalist><label class="field"><span id="fv-label">Custom field values</span><textarea id="fv-values" rows="10" spellcheck="false" aria-labelledby="fv-label"></textarea></label><label class="field"><span>Values offered for this field</span><select id="fv-prefer-field"><option value="global">Use global preference</option><option value="tags">Prefer values from tags</option><option value="custom">Include custom values</option></select></label><div class="group-buttons"><button type="button" id="fv-sort">Sort A–Z</button><button type="button" id="fv-import">Import…</button><button type="button" id="fv-export">Export…</button></div><label class="action-check"><input type="checkbox" id="fv-only-genres" ${fv.only_user_genres?'checked':''}> Show only user-defined genres</label><label class="action-check"><input type="checkbox" id="fv-prefer" ${fv.prefer_tag_values?'checked':''}> Prefer values from tags in list at tag panel</label><p>Field-specific choices override this default.</p><p>User-defined genres are the custom values of the GENRE field. They cannot be stored in ID3v1 tags.</p>`,
      '<button type="button" data-close>Cancel</button><button type="button" class="primary" id="fv-ok">OK</button>');
    const fieldInput=dialog.querySelector<HTMLInputElement>('#fv-field')!,used=dialog.querySelector<HTMLSelectElement>('#fv-used')!,values=dialog.querySelector<HTMLTextAreaElement>('#fv-values')!;
    const preference=dialog.querySelector<HTMLSelectElement>('#fv-prefer-field')!;
    const capture=()=>{
      if(values.value!==shown)lists[current]=parseValueList(values.value);
      if(preference.value==='global')delete preferences[current];else preferences[current]=preference.value==='tags';
    };
    const show=()=>{
      fieldInput.value=current;values.value=(lists[current]||[]).join('\n');shown=values.value;
      preference.value=preferences[current]===undefined?'global':preferences[current]?'tags':'custom';
      const keys=[...new Set([...Object.keys(lists),...Object.keys(preferences),current])].filter(key=>lists[key]?.length||preferences[key]!==undefined||key===current).sort();
      used.innerHTML=keys.map(key=>`<option value="${html(key)}" ${key===current?'selected':''}>${html(key)} (${lists[key]?.length||0})</option>`).join('');
    };
    const switchTo=(field:string)=>{const next=normalizeField(field);if(!next||next===current)return;capture();current=next;show();};
    fieldInput.addEventListener('change',()=>switchTo(fieldInput.value));
    fieldInput.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();switchTo(fieldInput.value);values.focus();}});
    used.addEventListener('change',()=>switchTo(used.value));
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();dialog.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=true);
      try{await work();}catch(error){if(dialog.open)TagPanel.error(dialog,error,this.host.batchError);}finally{working=false;dialog.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=false);}
    };
    const on=(id:string,handler:()=>void)=>dialog.querySelector(`#${id}`)!.addEventListener('click',handler);
    on('fv-sort',()=>{capture();lists[current]=[...lists[current]??[]].sort(collator.compare);show();});
    on('fv-import',()=>void perform(async()=>{capture();const path=await this.host.pickOpen('list');if(!path)return;const imported=await this.host.importList(path);lists[current]=[...new Set([...lists[current]??[],...normalizeValues(imported)])];if(dialog.open)show();}));
    on('fv-export',()=>void perform(async()=>{capture();const path=await this.host.pickSave('list',`${current.toLowerCase()}.txt`);if(path)await this.host.exportList(path,lists[current]??[]);}));
    on('fv-ok',()=>void perform(async()=>{
      capture();
      const next=clone(this.settings);next.field_values.lists=Object.fromEntries(Object.entries(lists).filter(([,values])=>values.length));
      if(Object.keys(preferences).length)next.field_values.prefer_tag_fields=preferences;else delete next.field_values.prefer_tag_fields;
      next.field_values.only_user_genres=dialog.querySelector<HTMLInputElement>('#fv-only-genres')!.checked;next.field_values.prefer_tag_values=dialog.querySelector<HTMLInputElement>('#fv-prefer')!.checked;
      await this.persist(next,'Field values saved.');dialog.close();
    }));
    dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
    dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());
    show();values.focus();
  }
}
