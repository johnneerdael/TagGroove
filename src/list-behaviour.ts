// File List behaviours from Mp3tag (https://docs.mp3tag.de/keyboard/,
// https://docs.mp3tag.de/customization/options/tags/): saving pending edits
// when moving to another file, manual reordering (Alt+arrows, Alt+drag),
// the folder-image fallback of the cover pane and column auto-size.
// Standalone (type imports only), so the rules are unit-tested directly.

/** pages.tags (defaults in options-ui TAGS_PAGE_DEFAULTS): Options → Tags switches beyond the tag type pages. */
/** Options → Tags → Save mode at extended tag dialog, when << or >> leaves a changed file: keep the changes pending (OpenTag's default), ask, discard them or save the file. */
export type ExtendedSaveMode='pending'|'ask'|'ignore'|'save';
export type TagsPagePrefs={save_on_move:boolean;no_folder_image:boolean;extended_save_mode:ExtendedSaveMode};
export function tagsPagePrefs(values:Record<string,unknown>):TagsPagePrefs{
  const mode=values.extended_save_mode;
  return{save_on_move:values.save_on_move===true,no_folder_image:values.no_folder_image===true,extended_save_mode:mode==='ask'||mode==='ignore'||mode==='save'?mode:'pending'};
}

/**
 * "Save tags when using arrow-keys/single mouse click": when the selection
 * moves away from files with pending edits, those files are saved. Returns the
 * previously selected files to save, in list order, or none when the
 * selection did not change.
 */
export function autoSaveTargets(previous:readonly string[],next:ReadonlySet<string>,pending:(path:string)=>boolean):string[]{
  const changed=previous.length!==next.size||previous.some(path=>!next.has(path));
  return changed?previous.filter(pending):[];
}

export type ReorderMove='up'|'down'|'top'|'bottom';
type KeyInput={key:string;altKey:boolean;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean};
/** Alt+Up/Down move the selected files one row; Alt+Page Up/Down move them to the beginning or end. */
export function reorderKey(event:KeyInput):ReorderMove|undefined{
  if(!event.altKey||event.ctrlKey||event.metaKey||event.shiftKey)return undefined;
  return({ArrowUp:'up',ArrowDown:'down',PageUp:'top',PageDown:'bottom'} as Record<string,ReorderMove>)[event.key];
}

/**
 * Move the selected entries of `order`. Single steps keep the selected files'
 * relative order and stop at the list edges (a blocked file stays while the
 * others move); top and bottom gather the selection there.
 */
export function moveSelection(order:readonly string[],selected:ReadonlySet<string>,move:ReorderMove):string[]{
  if(move==='top'||move==='bottom'){
    const moving=order.filter(path=>selected.has(path)),rest=order.filter(path=>!selected.has(path));
    return move==='top'?[...moving,...rest]:[...rest,...moving];
  }
  const result=[...order],delta=move==='up'?-1:1;
  const indexes=result.map((_,index)=>index);if(delta>0)indexes.reverse();
  for(const index of indexes){
    if(!selected.has(result[index]))continue;
    const target=index+delta;if(target<0||target>=result.length||selected.has(result[target]))continue;
    [result[index],result[target]]=[result[target],result[index]];
  }
  return result;
}

/** Drop the dragged entries before `before` (`null` drops them at the end). Dropping onto a dragged entry changes nothing. */
export function dropSelection(order:readonly string[],moving:ReadonlySet<string>,before:string|null):string[]{
  if(before!==null&&moving.has(before))return[...order];
  const dragged=order.filter(path=>moving.has(path)),rest=order.filter(path=>!moving.has(path));
  const at=before===null?rest.length:rest.indexOf(before);
  if(at<0)return[...order];
  return[...rest.slice(0,at),...dragged,...rest.slice(at)];
}

/**
 * Apply a new order of the shown (filtered) files to the whole list: the
 * positions held by shown files are refilled in their new order, so hidden
 * files keep their places.
 */
export function mergeOrder(full:readonly string[],shown:readonly string[],reordered:readonly string[]):string[]{
  const visible=new Set(shown);let next=0;
  return full.map(path=>visible.has(path)?reordered[next++]:path);
}

/** Index of the row boundary nearest to `offset` (pixels from the first row's top), clamped to the list. */
export function dropIndex(offset:number,rowHeight:number,rows:number):number{
  return Math.max(0,Math.min(rows,Math.round(offset/Math.max(1,rowHeight))));
}

export type FolderImageFile={path:string;covers:number};
/**
 * The folder whose first image the cover pane shows instead of embedded art:
 * only while no selected file has (or is about to get) a cover and every
 * selected file is in the active file's folder. Provider documents have no
 * browsable folder.
 */
export function folderImageFolder(selection:readonly FolderImageFile[],active:string|undefined,options:{disabled:boolean;provider:boolean}):string|undefined{
  if(options.disabled||options.provider||!active||!selection.length||selection.some(file=>file.covers>0))return undefined;
  const folder=folderOf(active);
  return folder&&selection.every(file=>folderOf(file.path)===folder)?folder:undefined;
}
const folderOf=(path:string)=>{const cut=Math.max(path.lastIndexOf('/'),path.lastIndexOf('\\'));return cut>0?path.slice(0,cut):'';};

/** Ctrl+Num + (⌘ on macOS) sizes the File List columns to their contents. */
export function autoSizeShortcut(event:{key:string;code?:string;ctrlKey:boolean;metaKey:boolean;altKey:boolean;location?:number}):boolean{
  return(event.ctrlKey||event.metaKey)&&!event.altKey&&(event.code==='NumpadAdd'||(event.key==='+'&&event.location===3));
}
/** Widest content plus the cell's own space (padding, checkbox, marker), within the column width limits. */
export function fitWidth(contentWidths:readonly number[],extra:number,limits:{min:number;max:number}):number{
  const widest=contentWidths.reduce((max,width)=>Math.max(max,Number.isFinite(width)?width:0),0);
  return Math.round(Math.max(limits.min,Math.min(limits.max,Math.ceil(widest+extra))));
}
/** Rows measured for auto-size; Mp3tag measures the listed files, very long lists are sampled from the top. */
export const AUTO_SIZE_ROWS=5000;

/** Text widths for auto-size, measured with the font of the cells. Uses a shared canvas. */
let measureCanvas:HTMLCanvasElement|undefined;
export function textWidths(texts:readonly string[],font:string):number[]{
  measureCanvas??=document.createElement('canvas');
  const context=measureCanvas.getContext('2d');if(!context)return texts.map(text=>text.length*7);
  context.font=font;
  return texts.map(text=>context.measureText(text).width);
}

export type RowDragHost={
  /** Paths of the shown rows in order. */
  rows:()=>string[];
  rowHeight:()=>number;
  headerHeight:number;
  selected:()=>ReadonlySet<string>;
  /** Select one row before dragging it (the pressed row was not selected). */
  select:(path:string)=>void;
  drop:(moving:string[],before:string|null)=>void;
};
/**
 * Alt+drag in the File List: dragging selected rows shows a drop line between
 * rows and moves them there. A plain Alt+click still selects normally.
 */
export function bindRowDrag(wrap:HTMLElement,host:RowDragHost){
  wrap.addEventListener('pointerdown',event=>{
    if(event.button!==0||!event.altKey||event.ctrlKey||event.metaKey||event.shiftKey)return;
    const row=(event.target as Element).closest<HTMLTableRowElement>('tr[data-path]');if(!row)return;
    event.preventDefault();
    const path=row.dataset.path!,startY=event.clientY;
    let dragging=false,moving:string[]=[],before:string|null=null;
    const marker=document.createElement('div');marker.className='row-drop-marker';marker.setAttribute('aria-hidden','true');
    const place=(clientY:number)=>{
      const box=wrap.getBoundingClientRect(),list=host.rows(),height=host.rowHeight();
      if(clientY<box.top+host.headerHeight+height/2)wrap.scrollTop-=height;else if(clientY>box.bottom-height/2)wrap.scrollTop+=height;
      const index=dropIndex(clientY-box.top-host.headerHeight+wrap.scrollTop,height,list.length);
      before=list[index]??null;marker.style.top=`${host.headerHeight+index*height-1}px`;
    };
    const move=(e:PointerEvent)=>{
      if(!dragging&&Math.abs(e.clientY-startY)<4)return;
      if(!dragging){
        dragging=true;wrap.classList.add('row-dragging');
        try{wrap.setPointerCapture(e.pointerId);}catch{/* Capture is optional; moves over the list still arrive. */}
        if(!host.selected().has(path))host.select(path);
        const chosen=host.selected();moving=host.rows().filter(item=>chosen.has(item));
        marker.style.width=`${wrap.scrollWidth}px`;wrap.append(marker);
      }
      place(e.clientY);
    };
    const end=(e:PointerEvent)=>{
      wrap.removeEventListener('pointermove',move);wrap.removeEventListener('pointerup',end);wrap.removeEventListener('pointercancel',end);
      marker.remove();wrap.classList.remove('row-dragging');
      if(!dragging)return;
      // The click that follows the drag (dispatched right after pointerup) must not change the selection.
      const suppress=(click:Event)=>{click.stopPropagation();click.preventDefault();};
      wrap.addEventListener('click',suppress,{capture:true,once:true});
      setTimeout(()=>wrap.removeEventListener('click',suppress,{capture:true}),0);
      if(e.type==='pointerup'&&moving.length)host.drop(moving,before);
    };
    wrap.addEventListener('pointermove',move);
    wrap.addEventListener('pointerup',end);wrap.addEventListener('pointercancel',end);
  });
}

/** Random File List order (a click on the icon column header): every order is equally likely. */
export function shuffleOrder<T>(items:readonly T[],random:()=>number=Math.random):T[]{
  const result=[...items];
  for(let i=result.length-1;i>0;i--){const j=Math.floor(random()*(i+1));[result[i],result[j]]=[result[j],result[i]];}
  return result;
}
/** Native drops: a plain drop replaces the File List; Ctrl (⌘ on macOS, where Control also works) adds to it. */
export function dropAdds(keys:{control:boolean;command:boolean},mac:boolean):boolean{
  return keys.control||(mac&&keys.command);
}
