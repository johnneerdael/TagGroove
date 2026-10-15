export type ColumnAlign='left'|'center'|'right';
export type ColumnDef={id:string;name:string;value:string;field:string;sort_by:string;width:number;align:ColumnAlign;numeric:boolean;visible:boolean};
export type SortKey={id:string;ascending:boolean};
export type ColumnLayout={version:number;columns:ColumnDef[];sort:SortKey[]};
type Fields=Record<string,string[]>;
export const minColumnWidth=24,maxColumnWidth=4000;
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const multi=(field:string)=>`$meta_sep(${field},' \\\\ ')`;
const column=(id:string,name:string,value:string,field:string,width:number,options:Partial<ColumnDef>={}):ColumnDef=>({id,name,value,field,sort_by:'',width,align:'left',numeric:false,visible:true,...options});
const numeric={align:'right' as ColumnAlign,numeric:true},hidden={visible:false};

/** Default File List columns, approximating Mp3tag's stock set with a compact visible subset. */
export function defaultLayout():ColumnLayout{
  return{version:1,sort:[{id:'filename',ascending:true}],columns:[
    column('filename','Filename','%_filename_ext%','%_filename_ext%',220),
    column('title','Title',multi('title'),'%title%',175),
    column('artist','Artist',multi('artist'),'%artist%',145),
    column('album','Album',multi('album'),'%album%',165),
    column('track','Track','%track%','%track%',55,numeric),
    column('year','Year','%year%','%year%',55,numeric),
    column('genre','Genre',multi('genre'),'%genre%',90),
    column('length','Length','%_length%','',60,{...numeric,sort_by:'%_length_seconds%'}),
    column('codec','Codec','$if2(%_codec%,%_filetype%)','',90),
    column('bitrate','Bitrate','[%_bitrate% kbps]','',70,{...numeric,sort_by:'%_bitrate%'}),
    column('path','Path','%_folderpath%','',220),
    column('tag','Tag','%_tag%','',90,hidden),
    column('comment','Comment',multi('comment'),'%comment%',160,hidden),
    column('albumartist','Album Artist',multi('albumartist'),'%albumartist%',140,hidden),
    column('composer','Composer',multi('composer'),'%composer%',140,hidden),
    column('discnumber','Discnumber','%discnumber%','%discnumber%',70,{...numeric,...hidden}),
    column('samplerate','Sample rate','[%_samplerate% Hz]','',85,{...numeric,...hidden,sort_by:'%_samplerate%'}),
    column('size','Size','%_file_size%','',75,{...numeric,...hidden,sort_by:'%_file_size_bytes%'}),
    column('modified','Last modified','%_file_mod_datetime%','',135,{...numeric,...hidden,sort_by:'%_file_mod_datetime_raw%'}),
    column('covers','Covers','%_covers%','',60,{...numeric,...hidden}),
  ]};
}

/** Return the edited tag key for a column, or '' when the column is read-only. */
export function editKey(column:Pick<ColumnDef,'field'>):string{
  const match=/^%([^%$()\[\],'\r\n]+)%$/.exec(column.field.trim());
  const name=match?.[1].trim().toUpperCase()||'';
  return !name||(name.startsWith('_')&&name!=='_FILENAME'&&name!=='_FILENAME_EXT')?'':name;
}
export const sortTemplate=(column:ColumnDef)=>column.sort_by.trim()||column.value;
export const visibleColumns=(layout:ColumnLayout)=>layout.columns.filter(column=>column.visible);
const clampWidth=(width:number)=>Math.round(Math.min(maxColumnWidth,Math.max(minColumnWidth,Number.isFinite(width)?width:100)));

/** Check a layout with the same rules the backend enforces when saving. */
export function layoutProblem(layout:ColumnLayout):string{
  if(!layout.columns.length)return 'Configure at least one File List column.';
  if(!layout.columns.some(column=>column.visible))return 'Keep at least one File List column visible.';
  const ids=new Set<string>();
  for(const column of layout.columns){
    if(!/^[A-Za-z0-9_-]{1,64}$/.test(column.id)||ids.has(column.id.toLowerCase()))return 'File List columns require unique identifiers.';
    ids.add(column.id.toLowerCase());
    if(!column.name.trim())return 'Enter a name for every File List column.';
    if(!column.value.trim())return `${column.name.trim()}: enter a value format string.`;
    if(column.field.trim()&&!editKey(column))return `${column.name.trim()}: field must be a tag field placeholder such as %ISRC%, %_filename% or %_filename_ext%.`;
  }
  return '';
}

/** Accept a stored or imported layout, filling omitted properties. */
export function normalizeLayout(raw:unknown):ColumnLayout{
  const source=raw as Partial<ColumnLayout>|null;
  if(!source||!Array.isArray(source.columns))throw new Error('The column configuration is not valid.');
  const columns=source.columns.map((c:Partial<ColumnDef>)=>({id:String(c.id??''),name:String(c.name??''),value:String(c.value??''),field:String(c.field??''),sort_by:String(c.sort_by??''),width:clampWidth(Number(c.width)),align:(['left','center','right'].includes(c.align as string)?c.align:'left') as ColumnAlign,numeric:!!c.numeric,visible:c.visible!==false}));
  const ids=new Set(columns.map(c=>c.id)),seen=new Set<string>();
  const sort=(Array.isArray(source.sort)?source.sort:[]).filter(key=>ids.has(key?.id)&&!seen.has(key.id)&&!!seen.add(key.id)).map(key=>({id:key.id,ascending:key.ascending!==false}));
  const layout={version:1,columns,sort};
  const problem=layoutProblem(layout);if(problem)throw new Error(problem);
  return layout;
}

const update=(layout:ColumnLayout,change:(columns:ColumnDef[])=>ColumnDef[]):ColumnLayout=>({...layout,columns:change(layout.columns.map(column=>({...column}))),sort:layout.sort.map(key=>({...key}))});
export function toggleColumn(layout:ColumnLayout,id:string,visible?:boolean):ColumnLayout{
  const result=update(layout,columns=>columns.map(column=>column.id===id?{...column,visible:visible??!column.visible}:column));
  if(!result.columns.some(column=>column.visible))throw new Error('Keep at least one File List column visible.');
  return result;
}
/** Move a column before or after another column; hidden columns keep their relative order. */
export function moveColumn(layout:ColumnLayout,id:string,targetId:string,after:boolean):ColumnLayout{
  if(id===targetId)return layout;
  return update(layout,columns=>{
    const from=columns.findIndex(column=>column.id===id);if(from<0||!columns.some(column=>column.id===targetId))return columns;
    const [moved]=columns.splice(from,1),target=columns.findIndex(column=>column.id===targetId);
    columns.splice(target+(after?1:0),0,moved);return columns;
  });
}
export const resizeColumn=(layout:ColumnLayout,id:string,width:number)=>update(layout,columns=>columns.map(column=>column.id===id?{...column,width:clampWidth(width)}:column));
export function uniqueColumnId(layout:ColumnLayout,base='column'){let index=1,id=base;while(layout.columns.some(column=>column.id.toLowerCase()===id.toLowerCase()))id=`${base}-${++index}`;return id;}

/** Header click: replace the sort, or with Shift add a secondary criterion. Repeating a click reverses that criterion. */
export function sortClick(sort:SortKey[],id:string,additive:boolean):SortKey[]{
  const existing=sort.find(key=>key.id===id);
  if(additive)return existing?sort.map(key=>key.id===id?{...key,ascending:!key.ascending}:key):[...sort,{id,ascending:true}];
  return[{id,ascending:existing&&sort[0].id===id?!existing.ascending:true}];
}
export function sortIndicator(sort:SortKey[],id:string){
  const index=sort.findIndex(key=>key.id===id);if(index<0)return '';
  return`${sort[index].ascending?' ↑':' ↓'}${sort.length>1?`<sup>${index+1}</sup>`:''}`;
}

/** Numeric columns compare their leading number; empty or non-numeric values sort first. */
export function numericValue(value:string):number{
  const match=/^\s*[-+]?(?:\d+(?:[.,]\d+)?|[.,]\d+)/.exec(value);
  return match?Number(match[0].trim().replace(',','.')):Number.NEGATIVE_INFINITY;
}
export function compareCells(a:string,b:string,numericSort:boolean,collator:Intl.Collator):number{
  if(numericSort){const x=numericValue(a),y=numericValue(b);if(x!==y)return x<y?-1:1;}
  return collator.compare(a,b);
}
/** Stable multi-criteria sort over precomputed sort values. */
export function sortRows<T>(rows:{item:T;keys:string[]}[],criteria:{ascending:boolean;numeric:boolean}[],collator:Intl.Collator):T[]{
  return rows.map((row,index)=>({...row,index})).sort((a,b)=>{
    for(let i=0;i<criteria.length;i++){const result=compareCells(a.keys[i],b.keys[i],criteria[i].numeric,collator);if(result)return criteria[i].ascending?result:-result;}
    return a.index-b.index;
  }).map(row=>row.item);
}

/** Script field view for columns: Vorbis TRACKNUMBER/DATE answer %track%/%year% as in the Tag Panel. */
export function columnFields(fields:Fields):Fields{
  const has=(name:string)=>Object.keys(fields).some(key=>key.toUpperCase()===name);
  const get=(name:string)=>Object.entries(fields).find(([key])=>key.toUpperCase()===name)?.[1];
  const result={...fields};
  if(!has('TRACK')&&has('TRACKNUMBER'))result.TRACK=get('TRACKNUMBER')!;
  if(!has('YEAR')&&has('DATE'))result.YEAR=get('DATE')!;
  return result;
}
/** Immediate value for single-placeholder templates while the scripting engine evaluates. */
export function quickValue(template:string,fields:Fields):string|undefined{
  const match=/^%([^%$'\[\]]+)%$/.exec(template);if(!match)return undefined;
  const name=match[1].toUpperCase();
  return(Object.entries(fields).find(([key])=>key.toUpperCase()===name)?.[1]||[])[0]??'';
}

type Token=readonly unknown[];
export type ColumnValueHost={
  token:(path:string)=>Token|undefined;
  templates:()=>string[];
  evaluate:(paths:string[],templates:string[])=>Promise<string[][]>;
  updated:()=>void;
  failed?:(error:unknown)=>void;
  schedule?:(work:()=>void)=>void;
};
const sameToken=(a:Token,b:Token|undefined)=>!!b&&a.length===b.length&&a.every((value,index)=>value===b[index]);
/**
 * Cache of script-evaluated column values. Rows are keyed by path and invalidated
 * when the host token (entry, draft and override identities) changes. Stale values
 * stay visible until fresh ones arrive, so sorting does not jump while editing.
 */
export class ColumnValueCache{
  private rows=new Map<string,{token:Token;values:Map<string,string>;stale:boolean}>();
  private queued=new Set<string>();
  private scheduled=false;private running=false;
  revision=0;batchSize=500;
  constructor(private host:ColumnValueHost){}
  get(path:string,template:string):string|undefined{
    const token=this.host.token(path);if(!token)return undefined;
    const row=this.rows.get(path);
    if(!row){this.queue(path);return undefined;}
    if(!sameToken(row.token,token)){row.token=token;row.stale=true;this.queue(path);}
    const value=row.values.get(template);
    // Only active templates are evaluated; queueing others would never settle.
    if(value===undefined&&!row.stale&&this.host.templates().includes(template))this.queue(path);
    return value;
  }
  /** A value that reflects the current token, without queueing evaluation. */
  fresh(path:string,template:string):string|undefined{const row=this.rows.get(path);return row&&!row.stale&&sameToken(row.token,this.host.token(path))?row.values.get(template):undefined;}
  /** Drop rows for paths no longer listed. */
  retain(keep:(path:string)=>boolean){for(const path of[...this.rows.keys()])if(!keep(path))this.rows.delete(path);for(const path of[...this.queued])if(!keep(path))this.queued.delete(path);}
  clear(){this.rows.clear();this.queued.clear();this.revision++;}
  get pending(){return this.queued.size>0||this.running;}
  private queue(path:string){
    this.queued.add(path);
    if(this.scheduled||this.running)return;
    this.scheduled=true;(this.host.schedule||(work=>queueMicrotask(work)))(()=>{this.scheduled=false;void this.flush();});
  }
  async flush(){
    if(this.running)return;this.running=true;
    try{
      while(this.queued.size){
        const templates=[...new Set(this.host.templates())];
        const batch=[...this.queued].slice(0,this.batchSize);batch.forEach(path=>this.queued.delete(path));
        const requested=batch.map(path=>({path,token:this.host.token(path)})).filter(item=>!!item.token);
        if(!requested.length||!templates.length)continue;
        let values:string[][];
        try{values=await this.host.evaluate(requested.map(item=>item.path),templates);}
        catch(error){values=requested.map(()=>templates.map(()=>''));this.host.failed?.(error);}
        requested.forEach(({path,token},index)=>{
          const current=this.host.token(path);if(!current)return;
          const stale=!sameToken(token!,current);
          this.rows.set(path,{token:current,values:new Map(templates.map((template,column)=>[template,values[index]?.[column]??''])),stale});
          if(stale)this.queued.add(path);
        });
        this.revision++;this.host.updated();
      }
    }finally{this.running=false;}
  }
}

const ariaSort=(sort:SortKey[],id:string)=>{const key=sort.find(item=>item.id===id);return key?key.ascending?'ascending':'descending':'none';};
// Options → Language: a column shows the name it was given (translate="no"); only a default
// column that still has its default name follows the UI language.
let translateName=(name:string)=>name;
export function setColumnNameTranslation(translate:(name:string)=>string){translateName=translate;}
let defaultNames:Map<string,string>|undefined;
export function columnName(column:{id:string;name:string}){
  defaultNames??=new Map(defaultLayout().columns.map(c=>[c.id,c.name]));
  return defaultNames.get(column.id)===column.name?translateName(column.name):column.name;
}
const nameMarkup=(column:{id:string;name:string})=>`<span translate="no">${html(columnName(column))}</span>`;
const sortTitle=(name:string)=>`${name} — click to sort, Shift+click to add a sort criterion`;
const sortLabel=(column:ColumnDef,sort:SortKey[])=>`${nameMarkup(column)}${sortIndicator(sort,column.id)}`;
/** Header cells; with `shuffle`, the first cell starts with the icon-column button that puts the files in random order. */
export function headerMarkup(columns:ColumnDef[],sort:SortKey[],shuffle=false){
  return columns.map((column,index)=>`<th style="width:${column.width}px" data-col="${html(column.id)}" class="align-${column.align}" aria-sort="${ariaSort(sort,column.id)}">${shuffle&&index===0?'<button type="button" class="shuffle-list" data-shuffle title="Random order: shuffle the file list" aria-label="Random order">⤮</button>':''}<button data-sort="${html(column.id)}" data-verbatim="${html(columnName(column))}" title="${html(sortTitle(columnName(column)))}">${sortLabel(column,sort)}</button><span class="col-resize" data-resize="${html(column.id)}" aria-hidden="true" title="Drag to resize; double-click to fit the contents"></span></th>`).join('');
}
/** Show `sort` in header cells made by `headerMarkup`, keeping the cells and their listeners. */
export function updateSortHeader(cells:Iterable<HTMLElement>,columns:ColumnDef[],sort:SortKey[]){
  for(const th of cells){
    const column=columns.find(c=>c.id===th.dataset.col),button=th.querySelector('button[data-sort]');if(!column||!button)continue;
    th.setAttribute('aria-sort',ariaSort(sort,column.id));
    const label=sortLabel(column,sort);if(button.innerHTML!==label)button.innerHTML=label;
  }
}

export type ColumnUIHost={
  layout:()=>ColumnLayout;
  apply:(layout:ColumnLayout,verified?:boolean)=>Promise<void>;
  importLayout:()=>Promise<ColumnLayout|null>;
  exportLayout:(layout:ColumnLayout)=>Promise<boolean>;
  preview?:(template:string)=>Promise<string>;
  /** Size columns to their contents (double-click on a header divider). */
  autoSize?:(ids:string[])=>void;
  report:(error:unknown)=>void;
};
export class ColumnUI{
  private menu?:HTMLElement;
  constructor(private host:ColumnUIHost){}
  private async change(layout:ColumnLayout){try{await this.host.apply(layout);}catch(error){this.host.report(error);}}
  /** View → Columns entries: one checkable entry per configured column. */
  menuHTML(){return`<hr><p class="menu-note">Columns</p>${this.host.layout().columns.map(column=>`<button class="column-toggle" data-column-toggle="${html(column.id)}" role="menuitemcheckbox" aria-checked="${column.visible}"><span>${column.visible?'✓ ':'<span class="column-check-space"></span>'}${nameMarkup(column)}</span></button>`).join('')}`;}
  bindMenu(root:ParentNode){
    root.querySelectorAll<HTMLButtonElement>('[data-column-toggle]').forEach(button=>button.addEventListener('click',()=>{
      button.closest('details')?.removeAttribute('open');
      try{void this.change(toggleColumn(this.host.layout(),button.dataset.columnToggle!));}catch(error){this.host.report(error);}
    }));
  }
  closeMenu(){this.menu?.remove();this.menu=undefined;}
  /** Header context menu with quick show/hide toggles. */
  headerMenu(x:number,y:number,customize:()=>void){
    this.closeMenu();
    const menu=document.createElement('div');menu.className='column-menu';menu.setAttribute('role','menu');menu.setAttribute('aria-label','File List columns');
    menu.innerHTML=`${this.host.layout().columns.map(column=>`<label role="menuitemcheckbox" aria-checked="${column.visible}"><input type="checkbox" data-id="${html(column.id)}" ${column.visible?'checked':''}> ${nameMarkup(column)}</label>`).join('')}<hr><button data-customize>Customize columns…</button>`;
    document.body.append(menu);this.menu=menu;
    const width=menu.offsetWidth,height=menu.offsetHeight;
    menu.style.left=`${Math.max(4,Math.min(x,window.innerWidth-width-4))}px`;menu.style.top=`${Math.max(4,Math.min(y,window.innerHeight-height-4))}px`;
    menu.querySelectorAll<HTMLInputElement>('input[data-id]').forEach(input=>input.addEventListener('change',()=>{
      try{void this.change(toggleColumn(this.host.layout(),input.dataset.id!,input.checked));}catch(error){input.checked=true;this.host.report(error);}
    }));
    menu.querySelector('[data-customize]')!.addEventListener('click',()=>{this.closeMenu();customize();});
    const dismiss=(event:Event)=>{if(event instanceof KeyboardEvent&&event.key!=='Escape')return;if(event.type!=='keydown'&&menu.contains(event.target as Node))return;this.closeMenu();document.removeEventListener('pointerdown',dismiss,true);document.removeEventListener('keydown',dismiss,true);};
    document.addEventListener('pointerdown',dismiss,true);document.addEventListener('keydown',dismiss,true);
    menu.querySelector<HTMLInputElement>('input')?.focus();
  }
  /** Header drag to reorder, edge drag to resize and right-click for the column menu. */
  bindHeader(table:HTMLTableElement,customize:()=>void){
    const head=table.tHead;if(!head)return;
    head.addEventListener('contextmenu',event=>{event.preventDefault();this.headerMenu(event.clientX,event.clientY,customize);});
    head.querySelectorAll<HTMLElement>('[data-resize]').forEach(handle=>handle.addEventListener('pointerdown',event=>{
      if(event.button!==0)return;event.preventDefault();event.stopPropagation();
      const th=handle.closest('th')!,start=event.clientX,initial=th.offsetWidth,id=handle.dataset.resize!;
      const tableStart=table.offsetWidth;let width=initial;
      handle.setPointerCapture(event.pointerId);th.classList.add('resizing');
      const move=(e:PointerEvent)=>{width=Math.min(maxColumnWidth,Math.max(minColumnWidth,initial+e.clientX-start));th.style.width=`${width}px`;table.style.width=`${tableStart+width-initial}px`;};
      const end=()=>{handle.removeEventListener('pointermove',move);th.classList.remove('resizing');if(width!==initial)void this.change(resizeColumn(this.host.layout(),id,width));};
      handle.addEventListener('pointermove',move);handle.addEventListener('pointerup',end,{once:true});handle.addEventListener('pointercancel',end,{once:true});
    }));
    head.querySelectorAll<HTMLElement>('[data-resize]').forEach(handle=>handle.addEventListener('dblclick',event=>{event.preventDefault();event.stopPropagation();this.host.autoSize?.([handle.dataset.resize!]);}));
    head.querySelectorAll<HTMLElement>('th[data-col]').forEach(th=>th.addEventListener('pointerdown',event=>{
      if(event.button!==0||(event.target as Element).closest('[data-resize]'))return;
      const start=event.clientX,id=th.dataset.col!;let dragging=false,target:{id:string;after:boolean}|undefined;
      const marker=document.createElement('div');marker.className='column-drop-marker';
      const move=(e:PointerEvent)=>{
        if(!dragging&&Math.abs(e.clientX-start)<6)return;
        if(!dragging){dragging=true;th.classList.add('dragging');th.setPointerCapture(e.pointerId);table.parentElement?.append(marker);}
        const cells=[...head.querySelectorAll<HTMLElement>('th[data-col]')],over=cells.find(cell=>{const box=cell.getBoundingClientRect();return e.clientX>=box.left&&e.clientX<box.right;})||(e.clientX<cells[0].getBoundingClientRect().left?cells[0]:cells[cells.length-1]);
        const box=over.getBoundingClientRect(),after=e.clientX>box.left+box.width/2,wrap=table.parentElement!.getBoundingClientRect();
        target={id:over.dataset.col!,after};marker.style.top=`${table.parentElement!.scrollTop}px`;marker.style.left=`${(after?box.right:box.left)-wrap.left+table.parentElement!.scrollLeft-1}px`;
      };
      const end=()=>{
        th.removeEventListener('pointermove',move);marker.remove();th.classList.remove('dragging');
        if(!dragging)return;
        th.addEventListener('click',suppress=>{suppress.stopPropagation();suppress.preventDefault();},{capture:true,once:true});
        if(target&&target.id!==id)void this.change(moveColumn(this.host.layout(),id,target.id,target.after));
      };
      th.addEventListener('pointermove',move);th.addEventListener('pointerup',end,{once:true});th.addEventListener('pointercancel',end,{once:true});
    }));
  }
  /** View → Customize columns dialog. */
  customize(){
    if(document.querySelector('.columns-dialog'))return;
    let layout=structuredClone(this.host.layout()),current=0,working=false;
    const dialog=document.createElement('dialog');dialog.className='columns-dialog';dialog.setAttribute('aria-label','Customize columns');document.body.append(dialog);
    const q=<T extends Element>(selector:string)=>dialog.querySelector<T>(selector)!;
    const error=(message:unknown)=>{let p=dialog.querySelector<HTMLElement>('.dialog-error');if(!p){p=document.createElement('p');p.className='dialog-error';p.setAttribute('role','alert');q('.dialog-content').append(p);}p.textContent=String(message instanceof Error?message.message:message);};
    const capture=()=>{
      const column=layout.columns[current];if(!column)return;
      column.name=q<HTMLInputElement>('#column-name').value;column.value=q<HTMLInputElement>('#column-value').value;column.field=q<HTMLInputElement>('#column-field').value;
      column.sort_by=q<HTMLInputElement>('#column-sort').value;column.width=clampWidth(Number(q<HTMLInputElement>('#column-width').value));
      column.align=q<HTMLSelectElement>('#column-align').value as ColumnAlign;column.numeric=q<HTMLInputElement>('#column-numeric').checked;
    };
    const render=(focus?:string)=>{
      const column=layout.columns[current];
      dialog.innerHTML=`<header>Customize columns</header><div class="dialog-content"><div class="columns-layout"><div><p class="action-section-title">Columns</p><ul class="column-list" role="listbox" aria-label="Columns">${layout.columns.map((c,i)=>`<li class="group-row ${i===current?'current':''}" role="option" aria-selected="${i===current}" data-index="${i}" tabindex="${i===current?0:-1}"><input type="checkbox" data-visible="${i}" data-verbatim="${html(c.name)}" aria-label="Show ${html(c.name)}" ${c.visible?'checked':''}>${c.name?`<span translate="no">${html(c.name)}</span>`:'<span>(unnamed)</span>'}<small>${html(c.field?'editable':'')}</small></li>`).join('')}</ul>
      <div class="group-buttons"><button id="column-new">New</button><button id="column-duplicate" ${column?'':'disabled'}>Duplicate</button><button id="column-remove" ${layout.columns.length>1?'':'disabled'}>Remove</button><button id="column-up" ${current>0?'':'disabled'}>Move up</button><button id="column-down" ${current<layout.columns.length-1?'':'disabled'}>Move down</button></div>
      <div class="group-buttons"><button id="column-defaults">Reset to defaults</button><button id="column-import">Load configuration…</button><button id="column-export">Save configuration…</button></div></div>
      <div>${column?`<label class="field"><span>NAME</span><input id="column-name" value="${html(column.name)}"></label>
      <label class="field"><span>VALUE</span><input id="column-value" value="${html(column.value)}" spellcheck="false" autocomplete="off" placeholder="%artist% or $num(%track%,2)" data-helper="format"></label>
      <label class="field"><span>FIELD (EMPTY = READ-ONLY)</span><input id="column-field" value="${html(column.field)}" spellcheck="false" autocomplete="off" placeholder="%artist%" data-helper="placeholder"></label>
      <label class="field"><span>SORT BY (EMPTY = VALUE)</span><input id="column-sort" value="${html(column.sort_by)}" spellcheck="false" autocomplete="off" data-helper="format"></label>
      <div class="field-pair"><label class="field"><span>WIDTH</span><input id="column-width" type="number" min="${minColumnWidth}" max="${maxColumnWidth}" value="${column.width}"></label><label class="field"><span>ALIGNMENT</span><select id="column-align">${(['left','center','right'] as const).map(a=>`<option value="${a}" ${column.align===a?'selected':''}>${a[0].toUpperCase()+a.slice(1)}</option>`).join('')}</select></label></div>
      <label class="action-check"><input type="checkbox" id="column-numeric" ${column.numeric?'checked':''}> Numeric (sort numerically)</label>
      <p class="column-preview" role="status"></p>`:''}</div></div></div>
      <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="column-ok">OK</button></div>`;
      bind();if(focus)dialog.querySelector<HTMLElement>(focus)?.focus();
      void preview();
    };
    const preview=async()=>{
      const target=dialog.querySelector<HTMLElement>('.column-preview'),column=layout.columns[current];if(!target||!column||!this.host.preview)return;
      try{const value=await this.host.preview(column.value);if(dialog.open&&layout.columns[current]===column)target.textContent=value===undefined?'':`Active file: ${value||'(empty)'}`;}catch(e){if(dialog.open)target.textContent=`Value: ${e instanceof Error?e.message:String(e)}`;}
    };
    const select=(index:number,focus?:string)=>{capture();current=Math.max(0,Math.min(layout.columns.length-1,index));render(focus);};
    const bind=()=>{
      q('[data-close]').addEventListener('click',()=>dialog.close());
      dialog.querySelectorAll<HTMLElement>('.column-list li').forEach(row=>{
        row.addEventListener('click',event=>{if((event.target as Element).matches('input'))return;select(Number(row.dataset.index),`.column-list li[data-index="${row.dataset.index}"]`);});
        row.addEventListener('keydown',event=>{
          const index=Number(row.dataset.index);
          if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();const next=index+(event.key==='ArrowDown'?1:-1);if(next>=0&&next<layout.columns.length)select(next,`.column-list li[data-index="${next}"]`);}
          if(event.key===' '){event.preventDefault();row.querySelector('input')!.click();}
        });
      });
      dialog.querySelectorAll<HTMLInputElement>('[data-visible]').forEach(input=>input.addEventListener('change',()=>{layout.columns[Number(input.dataset.visible)].visible=input.checked;}));
      dialog.querySelector('#column-value')?.addEventListener('input',()=>{capture();void preview();});
      dialog.querySelector('#column-numeric')?.addEventListener('change',event=>{const numeric=(event.target as HTMLInputElement).checked,align=q<HTMLSelectElement>('#column-align');if(numeric&&align.value==='left')align.value='right';if(!numeric&&align.value==='right')align.value='left';});
      dialog.querySelector('#column-name')?.addEventListener('input',event=>{const label=dialog.querySelector(`.column-list li[data-index="${current}"] span`);if(label)label.textContent=(event.target as HTMLInputElement).value||'(unnamed)';});
      q('#column-new').addEventListener('click',()=>{capture();layout.columns.splice(current+1,0,column(uniqueColumnId(layout),'New column','%isrc%','%isrc%',120));current++;render('#column-name');});
      dialog.querySelector('#column-duplicate')?.addEventListener('click',()=>{capture();const source=layout.columns[current];layout.columns.splice(current+1,0,{...source,id:uniqueColumnId(layout,source.id),name:`${source.name} copy`});current++;render('#column-name');});
      q('#column-remove').addEventListener('click',()=>{const id=layout.columns[current].id;layout.columns.splice(current,1);layout.sort=layout.sort.filter(key=>key.id!==id);current=Math.min(current,layout.columns.length-1);render('#column-remove');});
      q('#column-up').addEventListener('click',()=>{capture();const [item]=layout.columns.splice(current,1);layout.columns.splice(--current,0,item);render('#column-up');});
      q('#column-down').addEventListener('click',()=>{capture();const [item]=layout.columns.splice(current,1);layout.columns.splice(++current,0,item);render('#column-down');});
      q('#column-defaults').addEventListener('click',()=>{layout=defaultLayout();current=0;render('#column-defaults');});
      q('#column-import').addEventListener('click',()=>void perform(async()=>{const imported=await this.host.importLayout();if(imported){layout=imported;current=0;render('#column-import');}}));
      q('#column-export').addEventListener('click',()=>void perform(async()=>{capture();const problem=layoutProblem(layout);if(problem)throw new Error(problem);await this.host.exportLayout(layout);}));
      q('#column-ok').addEventListener('click',()=>void perform(async()=>{capture();const problem=layoutProblem(layout);if(problem)throw new Error(problem);await this.host.apply(layout,true);dialog.close();}));
    };
    const perform=async(work:()=>Promise<void>)=>{
      if(working)return;working=true;dialog.querySelector('.dialog-error')?.remove();
      try{await work();}catch(e){if(dialog.open)error(e);}finally{working=false;}
    };
    dialog.addEventListener('close',()=>dialog.remove());
    render();dialog.showModal();dialog.querySelector<HTMLElement>('.column-list li.current')?.focus();
  }
}
