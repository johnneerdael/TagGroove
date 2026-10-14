export type Control='text'|'check'|'multiline';
export type Size='full'|'larger'|'large'|'medium'|'normal'|'small'|'smaller'|'tiny';
export type PanelField={name:string;field:string;type:Control;size:Size;rows:number;default_value:string;visible:boolean};
export type PanelLayout={fields:PanelField[];show_panel:boolean;show_cover:boolean};
export type FieldValues={lists:Record<string,string[]>;only_user_genres:boolean;prefer_tag_values:boolean};
export type TagPanelSettings={version:number;panel:PanelLayout;field_values:FieldValues};
export type ComboOption={kind:'keep'|'blank'|'value'|'browse';value:string;group:'special'|'selection'|'custom'|'genre'|'directory'};
export const KEEP='< keep >',BLANK='< blank >',DIRECTORY='_DIRECTORY',SEPARATOR='_SEPARATOR';
export const SIZES:[Size,string][]=[['full','Full'],['larger','Larger'],['large','Large'],['medium','Medium'],['normal','Normal'],['small','Small'],['smaller','Smaller'],['tiny','Tiny']];
// Spans of a 12-column panel grid; consecutive fields share a row while they fit.
export const sizeSpan:Record<Size,number>={full:12,larger:10,large:9,medium:8,normal:6,small:4,smaller:3,tiny:2};
export const CONTROLS:[Control,string][]=[['text','Text'],['check','Check'],['multiline','Multiline']];
export const COMMON_FIELDS=['TITLE','ARTIST','ALBUM','ALBUMARTIST','YEAR','TRACK','DISCNUMBER','GENRE','COMMENT','COMPOSER','CONDUCTOR','LYRICIST','PUBLISHER','COPYRIGHT','ISRC','BPM','COMPILATION','MOOD','LANGUAGE','ENCODEDBY','ALBUMSORT','ARTISTSORT','ALBUMARTISTSORT','TITLESORT','UNSYNCEDLYRICS',DIRECTORY,SEPARATOR];
// ID3v1 genres with the Winamp extensions, as offered by tag editors' genre lists.
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
    if(values.some(value=>!value.trim()||value.trim()!==value||controlCharacters(value)))return`${key}: custom values must be nonempty single lines.`;
    if(new Set(values).size!==values.length)return`${key}: remove duplicate custom values.`;
  }
  return null;
}
/** Trim, drop empty lines and exact duplicates, keep the user's order. */
export function normalizeValues(values:string[]):string[]{return[...new Set(values.map(value=>value.trim()).filter(Boolean))];}
export const parseValueList=(text:string)=>normalizeValues(text.split(/\r?\n/));
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
export const splitValues=(text:string)=>text?text.split('\\\\').map(value=>value.trim()):[];
/** Panel text for the selection: shared value, or empty plus mixed when files differ. */
export function selectionState(values:string[]):{value:string;mixed:boolean}{
  const mixed=values.some(value=>value!==values[0]);return {value:mixed?'':values[0]||'',mixed};
}
const truthy=(value:string)=>!!value.trim()&&!/^(0|false|no)$/i.test(value.trim());
export function checkState(values:string[]):'on'|'off'|'mixed'{const on=values.filter(truthy).length;return on&&on===values.length?'on':on?'mixed':'off';}
/** Dropdown entries: keep/blank, the selection's values, custom values and (for GENRE) the genre list. */
export function comboOptions(options:{field:string;values:string[];custom?:string[];onlyUserGenres?:boolean;preferTagValues?:boolean}):ComboOption[]{
  const result:ComboOption[]=[],seen=new Set<string>();
  const add=(kind:ComboOption['kind'],value:string,group:ComboOption['group'])=>{if(kind==='value'&&(!value||seen.has(value)))return;seen.add(value);result.push({kind,value,group});};
  if(options.values.length>1)add('keep',KEEP,'special');
  if(options.values.length)add('blank',BLANK,'special');
  const selection=[...new Set(options.values.filter(Boolean))].sort(collator.compare);
  selection.forEach(value=>add('value',value,'selection'));
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
export function directoryOptions(directories:string[]):ComboOption[]{
  return [...[...new Set(directories)].sort(collator.compare).map(value=>({kind:'value' as const,value,group:'directory' as const})),{kind:'browse',value:'Open folder…',group:'special'}];
}

export type TagPanelHost={
  persistent:()=>boolean;
  load:()=>Promise<TagPanelSettings>;save:(settings:TagPanelSettings)=>Promise<void>;
  importLayout:(path:string)=>Promise<PanelLayout>;exportLayout:(path:string,layout:PanelLayout)=>Promise<void>;
  importList:(path:string)=>Promise<string[]>;exportList:(path:string,values:string[])=>Promise<void>;
  pickOpen:(kind:'layout'|'list')=>Promise<string|null>;pickSave:(kind:'layout'|'list',name:string)=>Promise<string|null>;
  values:(field:string)=>string[];disabled:()=>boolean;
  directories:()=>{selected:string[];all:string[]};selectDirectory:(directory:string)=>void;openFolder:()=>void;
  draft:(field:string,value:string)=>void;keep:(field:string)=>void;
  changed:()=>void;notify:(message:string)=>void;fail:(error:unknown)=>void;
  /** Options → General → Auto-completion at selection boxes: the completer while it is enabled. */
  autocomplete?:()=>((input:HTMLInputElement,event:Event,candidates:()=>Iterable<string>)=>boolean)|undefined;
};
const groupLabels:Record<ComboOption['group'],string>={special:'',selection:'Selected files',custom:'Custom values',genre:'Genres',directory:'Loaded folders'};
type OpenList={input:HTMLInputElement|HTMLTextAreaElement;field:PanelField;options:ComboOption[];active:number;popup:HTMLElement};

export class TagPanel {
  settings=defaultSettings();
  private list?:OpenList;
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
    const item=(command:string,label:string,checked?:boolean)=>`<button type="button" data-panel-command="${command}" ${checked===undefined?'':`role="menuitemcheckbox" aria-checked="${checked}"`}><span>${checked===undefined?'':checked?'✓ ':'<span class="panel-check-space"></span>'}${html(label)}</span></button>`;
    return `<details class="menu panel-menu"><summary aria-label="Tag panel options" title="Tag panel options">⋯</summary><div class="menu-list">${item('configure','Customize tag panel…')}${item('values','Field values…')}${item('defaults','Default values in input fields')}<hr>${fields.map(([field,index])=>item(`field:${index}`,mnemonic(field.name).label||field.field,field.visible)).join('')}${item('cover','Display cover art',this.showCover)}<hr>${item('save','Save configuration…')}${item('load','Load configuration…')}${item('hide','Hide tag panel')}</div></details>`;
  }
  fieldsHTML(){
    this.closeList();
    const disabled=this.host.disabled();
    return `<div class="panel-grid">${this.settings.panel.fields.map((field,index)=>field.visible?this.fieldHTML(field,index,disabled):'').join('')}</div>`;
  }
  private fieldHTML(field:PanelField,index:number,disabled:boolean){
    if(field.field===SEPARATOR)return'<div class="panel-separator" role="separator"></div>';
    const id=`tag-panel-${index}`,label=mnemonic(field.name),span=`style="grid-column:span ${sizeSpan[field.size]}"`,off=disabled?'disabled':'';
    // The underlined mnemonic splits the label into several text nodes; name controls explicitly.
    const shortcut=`aria-label="${html(label.label)}" ${label.key?`aria-keyshortcuts="Alt+${html(label.key)}"`:''}`;
    if(field.field===DIRECTORY){
      const {selected}=this.host.directories(),state=selectionState(selected);
      return `<div class="field panel-field" ${span}><label for="${id}">${label.html}</label><div class="combo"><input id="${id}" data-panel-index="${index}" data-directory role="combobox" aria-haspopup="listbox" aria-expanded="false" aria-autocomplete="none" readonly value="${html(state.value)}" placeholder="${state.mixed?KEEP:''}" title="${html(state.value)}" ${shortcut}><button type="button" class="combo-toggle" data-combo="${index}" tabindex="-1" aria-label="Show folders">▾</button></div></div>`;
    }
    const values=this.host.values(field.field),state=selectionState(values);
    if(field.type==='check'){
      const check=checkState(values);
      return `<div class="panel-field panel-check-field" ${span}><label class="action-check"><input type="checkbox" id="${id}" data-check="${html(field.field)}" data-panel-index="${index}" ${check==='on'?'checked':''} data-mixed="${check==='mixed'}" ${off} ${shortcut}> ${label.html}</label></div>`;
    }
    const placeholder=state.mixed?KEEP:'',inline=field.type==='text'&&!!this.host.autocomplete?.();
    const common=`id="${id}" data-field="${html(field.field)}" data-panel-index="${index}" role="combobox" aria-haspopup="listbox" aria-expanded="false" aria-autocomplete="${inline?'inline':'none'}" placeholder="${placeholder}" autocomplete="off" ${off} ${shortcut}`;
    const control=field.type==='multiline'?`<textarea ${common} rows="${field.rows}">${html(state.value)}</textarea>`:`<input ${common} value="${html(state.value)}">`;
    return `<div class="field panel-field" ${span}><label for="${id}">${label.html}</label><div class="combo ${field.type==='multiline'?'multiline':''}">${control}<button type="button" class="combo-toggle" data-combo="${index}" tabindex="-1" aria-label="Show values for ${html(label.label)}" ${off}>▾</button></div></div>`;
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
      input.addEventListener('input',event=>{
        this.closeList();
        const field=this.settings.panel.fields[Number(input.dataset.panelIndex)];
        const complete=this.host.autocomplete?.();
        if(complete&&input instanceof HTMLInputElement&&field?.type==='text'&&input.dataset.field)complete(input,event,()=>this.optionsFor(field).filter(option=>option.kind==='value').map(option=>option.value));
      });
      input.addEventListener('blur',()=>{if(this.list?.input===input)this.closeList();});
      if(input.dataset.directory!==undefined)input.addEventListener('click',()=>this.list?.input===input?this.closeList():this.openList(input));
    });
    const menu=root.querySelector<HTMLDetailsElement>('.panel-menu');
    if(menu){
      menu.addEventListener('toggle',()=>{if(!menu.open)return;const rect=menu.getBoundingClientRect(),popup=menu.querySelector<HTMLElement>('.menu-list')!;popup.style.top=`${rect.bottom+2}px`;popup.style.left=`${Math.max(8,Math.min(rect.right-popup.offsetWidth,window.innerWidth-popup.offsetWidth-8))}px`;});
      menu.querySelectorAll<HTMLButtonElement>('[data-panel-command]').forEach(button=>button.addEventListener('click',()=>{menu.open=false;void this.command(button.dataset.panelCommand!).catch(error=>this.host.fail(error));}));
    }
  }
  /** Document-level shortcuts: Ctrl+Q toggles the panel, Alt+mnemonic focuses a field. */
  install(root:Document){
    root.addEventListener('keydown',event=>{
      if(event.defaultPrevented||root.querySelector('dialog[open]'))return;
      if(event.ctrlKey&&!event.metaKey&&!event.altKey&&!event.shiftKey&&event.code==='KeyQ'){event.preventDefault();this.toggle();return;}
      if(!event.altKey||event.ctrlKey||event.metaKey||!this.visible)return;
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
    window.addEventListener('resize',()=>this.closeList());
  }
  private comboKey(event:KeyboardEvent,input:HTMLInputElement|HTMLTextAreaElement){
    const open=this.list?.input===input,multiline=input instanceof HTMLTextAreaElement;
    if(event.key==='ArrowDown'||event.key==='ArrowUp'){
      if(!open){if(multiline&&!event.altKey)return;event.preventDefault();this.openList(input,event.key==='ArrowUp'?-1:1);return;}
      event.preventDefault();this.moveActive(event.key==='ArrowDown'?1:-1);return;
    }
    if(open&&(event.key==='Home'||event.key==='End')&&event.altKey){event.preventDefault();this.setActive(event.key==='Home'?0:this.list!.options.length-1);return;}
    if(event.key==='Escape'&&open){event.preventDefault();event.stopPropagation();this.closeList();return;}
    if(event.key==='Enter'&&open&&this.list!.active>=0){event.preventDefault();this.choose(this.list!.active);return;}
    if(event.key==='Enter'&&event.shiftKey&&input.dataset.field){event.preventDefault();this.addCurrent(input);return;}
    if(event.key==='Tab'&&open)this.closeList();
  }
  private optionsFor(field:PanelField):ComboOption[]{
    if(field.field===DIRECTORY)return directoryOptions(this.host.directories().all);
    const values=this.host.values(field.field),fv=this.settings.field_values;
    return comboOptions({field:field.field,values,custom:fv.lists[field.field],onlyUserGenres:fv.only_user_genres,preferTagValues:fv.prefer_tag_values});
  }
  private openList(input:HTMLInputElement|HTMLTextAreaElement,step=0){
    this.closeList();
    const field=this.settings.panel.fields[Number(input.dataset.panelIndex)];if(!field||input.disabled)return;
    const options=this.optionsFor(field),popup=document.createElement('div');
    popup.className='combo-list';popup.id='tag-panel-listbox';popup.setAttribute('role','listbox');popup.setAttribute('aria-label',`${mnemonic(field.name).label} values`);
    let group:ComboOption['group']|undefined,markup='';
    options.forEach((option,index)=>{
      if(option.group!==group){if(group!==undefined)markup+='</div>';group=option.group;markup+=`<div role="group" ${groupLabels[group]?`aria-label="${groupLabels[group]}"`:''}>${groupLabels[group]?`<div class="combo-group" aria-hidden="true">${groupLabels[group]}</div>`:''}`;}
      markup+=`<div role="option" id="tag-panel-option-${index}" data-option="${index}" class="${option.kind==='value'?'':'special'}" aria-selected="false">${html(option.value)}</div>`;
    });
    popup.innerHTML=options.length?`${markup}</div>`:'<div class="combo-empty">No values</div>';
    popup.addEventListener('pointerdown',event=>event.preventDefault());
    popup.addEventListener('click',event=>{const option=(event.target as Element).closest<HTMLElement>('[data-option]');if(option)this.choose(Number(option.dataset.option));});
    document.body.append(popup);
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
    if(text===null)this.host.keep(field.field);else{input.value=text;this.host.draft(field.field,text);}
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
  }
  private dialog(title:string,className:string,content:string,footer:string){
    const dialog=document.createElement('dialog');dialog.className=className;dialog.setAttribute('aria-label',title);
    dialog.innerHTML=`<header>${html(title)}</header><div class="dialog-content">${content}</div><div class="dialog-footer">${footer}</div>`;
    document.body.append(dialog);this.dialogOpen=true;
    dialog.addEventListener('close',()=>{this.dialogOpen=false;dialog.remove();});dialog.showModal();return dialog;
  }
  private static error(dialog:HTMLDialogElement,error:unknown){let p=dialog.querySelector<HTMLElement>('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(p);}p.textContent=String(error instanceof Error?error.message:error);}
  /** Options → Tag Panel: fields list with New/Remove/Move and a field editor. */
  configure(){
    if(this.dialogOpen)return;
    let layout=clone(this.settings.panel),current=layout.fields.length?0:-1,working=false;
    const dialog=this.dialog('Customize tag panel','tag-panel-dialog',`<p>Choose the fields shown on the tag panel. Unchecked fields are hidden. Use &amp; in a name to set its Alt shortcut letter; ${DIRECTORY} adds a folder switcher and ${SEPARATOR} starts a new row.</p><div class="panel-config"><div><div class="panel-config-list" role="listbox" aria-label="Tag panel fields" tabindex="0"></div><div class="group-buttons"><button type="button" id="tp-new">New</button><button type="button" id="tp-remove">Remove</button><button type="button" id="tp-up" aria-label="Move up">Move up</button><button type="button" id="tp-down" aria-label="Move down">Move down</button><button type="button" id="tp-reset">Restore defaults</button></div></div><div class="panel-config-editor"></div></div><label class="action-check"><input type="checkbox" id="tp-cover"> Display cover art</label><datalist id="tp-fields">${COMMON_FIELDS.map(field=>`<option value="${field}">`).join('')}</datalist><datalist id="tp-defaults"><option value="${KEEP}"><option value="${BLANK}"></datalist>`,
      '<button type="button" id="tp-save-file">Save configuration…</button><button type="button" id="tp-load-file">Load configuration…</button><button type="button" data-close>Cancel</button><button type="button" class="primary" id="tp-ok">OK</button>');
    const list=dialog.querySelector<HTMLElement>('.panel-config-list')!,editor=dialog.querySelector<HTMLElement>('.panel-config-editor')!,cover=dialog.querySelector<HTMLInputElement>('#tp-cover')!;
    const on=(id:string,handler:()=>void)=>dialog.querySelector(`#${id}`)!.addEventListener('click',handler);
    const renderList=()=>{
      list.innerHTML=layout.fields.map((field,index)=>`<div class="panel-config-row ${index===current?'selected':''}" role="option" aria-selected="${index===current}" id="tp-row-${index}" data-row="${index}"><input type="checkbox" data-visible="${index}" ${field.visible?'checked':''} aria-label="Show ${html(mnemonic(field.name).label||field.field)}" tabindex="-1"><span>${html(mnemonic(field.name).label||(field.field===SEPARATOR?'Separator':''))}</span><small>${html(field.field)}</small></div>`).join('');
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
      const row=list.querySelector(`[data-row="${current}"]`);if(row){row.querySelector('span')!.textContent=mnemonic(field.name).label||(field.field===SEPARATOR?'Separator':'');row.querySelector('small')!.textContent=field.field;}
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
    on('tp-reset',()=>{layout={...defaultLayout(),show_panel:layout.show_panel};current=0;renderAll();});
    cover.addEventListener('change',()=>{layout.show_cover=cover.checked;});
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();dialog.querySelectorAll<HTMLButtonElement>('.dialog-footer button').forEach(button=>button.disabled=true);
      try{await work();}catch(error){if(dialog.open)TagPanel.error(dialog,error);}finally{working=false;if(dialog.open){dialog.querySelectorAll<HTMLButtonElement>('.dialog-footer button').forEach(button=>button.disabled=false);renderList();}}
    };
    on('tp-save-file',()=>void perform(async()=>{const error=layoutError(layout);if(error)throw new Error(error);const path=await this.host.pickSave('layout','tag-panel.json');if(path)await this.host.exportLayout(path,layout);}));
    on('tp-load-file',()=>void perform(async()=>{const path=await this.host.pickOpen('layout');if(!path)return;layout={...await this.host.importLayout(path),show_panel:layout.show_panel};current=layout.fields.length?0:-1;if(dialog.open)renderAll();}));
    on('tp-ok',()=>void perform(async()=>{const error=layoutError(layout);if(error)throw new Error(error);await this.persist({...clone(this.settings),panel:clone(layout)},'Tag panel configuration saved.');dialog.close();}));
    dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
    dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());
    renderAll();list.focus();
  }
  /** Options → Field Values: per-field custom value lists, genre options, import/export. */
  fieldValues(initialField?:string){
    if(this.dialogOpen)return;
    const lists=clone(this.settings.field_values.lists),fv=this.settings.field_values;
    const panelFields=this.settings.panel.fields.map(field=>field.field).filter(field=>!field.startsWith('_'));
    let current=normalizeField(initialField||Object.keys(lists)[0]||'GENRE'),working=false;
    const dialog=this.dialog('Field values','field-values-dialog',`<p>Custom values appear in the tag panel’s drop-down lists. Enter one value per line. Press Shift+Enter in a tag panel field to add its value here.</p><div class="field-pair"><label class="field"><span>Field</span><input id="fv-field" list="fv-fields" autocapitalize="characters" spellcheck="false"></label><div class="field"><span>Fields with custom values</span><select id="fv-used" aria-label="Fields with custom values"></select></div></div><datalist id="fv-fields">${[...new Set([...panelFields,...COMMON_FIELDS.filter(field=>!field.startsWith('_'))])].map(field=>`<option value="${html(field)}">`).join('')}</datalist><label class="field"><span id="fv-label">Custom field values</span><textarea id="fv-values" rows="10" spellcheck="false" aria-labelledby="fv-label"></textarea></label><div class="group-buttons"><button type="button" id="fv-sort">Sort A–Z</button><button type="button" id="fv-import">Import…</button><button type="button" id="fv-export">Export…</button></div><label class="action-check"><input type="checkbox" id="fv-only-genres" ${fv.only_user_genres?'checked':''}> Show only user-defined genres</label><label class="action-check"><input type="checkbox" id="fv-prefer" ${fv.prefer_tag_values?'checked':''}> Prefer values from tags in list at tag panel</label><p>User-defined genres are the custom values of the GENRE field. They cannot be stored in ID3v1 tags.</p>`,
      '<button type="button" data-close>Cancel</button><button type="button" class="primary" id="fv-ok">OK</button>');
    const fieldInput=dialog.querySelector<HTMLInputElement>('#fv-field')!,used=dialog.querySelector<HTMLSelectElement>('#fv-used')!,values=dialog.querySelector<HTMLTextAreaElement>('#fv-values')!;
    const capture=()=>{lists[current]=parseValueList(values.value);};
    const show=()=>{
      fieldInput.value=current;values.value=(lists[current]||[]).join('\n');
      const keys=Object.keys(lists).filter(key=>lists[key].length||key===current).sort();
      used.innerHTML=keys.map(key=>`<option value="${html(key)}" ${key===current?'selected':''}>${html(key)} (${lists[key]?.length||0})</option>`).join('');
    };
    const switchTo=(field:string)=>{const next=normalizeField(field);if(!next||next===current)return;capture();current=next;show();};
    fieldInput.addEventListener('change',()=>switchTo(fieldInput.value));
    fieldInput.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();switchTo(fieldInput.value);values.focus();}});
    used.addEventListener('change',()=>switchTo(used.value));
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();dialog.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=true);
      try{await work();}catch(error){if(dialog.open)TagPanel.error(dialog,error);}finally{working=false;dialog.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=false);}
    };
    const on=(id:string,handler:()=>void)=>dialog.querySelector(`#${id}`)!.addEventListener('click',handler);
    on('fv-sort',()=>{capture();lists[current]=[...lists[current]].sort(collator.compare);show();});
    on('fv-import',()=>void perform(async()=>{capture();const path=await this.host.pickOpen('list');if(!path)return;const imported=await this.host.importList(path);lists[current]=normalizeValues([...lists[current],...imported]);if(dialog.open)show();}));
    on('fv-export',()=>void perform(async()=>{capture();const path=await this.host.pickSave('list',`${current.toLowerCase()}.txt`);if(path)await this.host.exportList(path,lists[current]);}));
    on('fv-ok',()=>void perform(async()=>{
      capture();
      const next=withLists(this.settings,lists);next.field_values.only_user_genres=dialog.querySelector<HTMLInputElement>('#fv-only-genres')!.checked;next.field_values.prefer_tag_values=dialog.querySelector<HTMLInputElement>('#fv-prefer')!.checked;
      await this.persist(next,'Field values saved.');dialog.close();
    }));
    dialog.addEventListener('cancel',event=>{if(working)event.preventDefault();});
    dialog.querySelector('[data-close]')!.addEventListener('click',()=>dialog.close());
    show();values.focus();
  }
}
