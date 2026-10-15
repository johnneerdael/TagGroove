// Main window layout (docs.mp3tag.de/getting-started/main-window/): the Tag Panel
// docks left or right of the File List, above or below it (a horizontal panel
// whose _SEPARATOR fields start a new column), or floats over the window; the
// Filter sits above the File List or left or right of the toolbars. Dragging
// the panel's title bar docks it at the edge it is dropped on, double-clicking
// the title bar undocks or re-docks it, and its size is kept per position.
// Standalone (no imports), so the rules are unit-tested directly.

export type DockSide='left'|'right'|'top'|'bottom'|'float';
export type FilterDock='list'|'toolbar-left'|'toolbar-right';
export type FloatRect={x:number;y:number;width:number;height:number};
export type MainLayout={dock:DockSide;width:number;height:number;float:FloatRect;filter:FilterDock;
  /** The docked side a floating panel returns to on double-click. */
  last_dock:Exclude<DockSide,'float'>};

export const PANEL_WIDTH:[number,number]=[200,640];
export const PANEL_HEIGHT:[number,number]=[120,600];
export const FLOAT_MIN={width:220,height:160};
export const DOCKS:[DockSide,string][]=[['left','Left'],['right','Right'],['top','Top'],['bottom','Bottom'],['float','Floating']];
export const FILTER_DOCKS:[FilterDock,string][]=[['list','Above the file list'],['toolbar-left','Left of the toolbars'],['toolbar-right','Right of the toolbars']];

export function defaultMainLayout():MainLayout{
  return{dock:'left',width:264,height:260,float:{x:80,y:120,width:300,height:520},filter:'list',last_dock:'left'};
}
const clamp=(value:number,[min,max]:[number,number])=>Math.round(Math.max(min,Math.min(max,value)));
const number=(value:unknown,fallback:number)=>typeof value==='number'&&Number.isFinite(value)?value:fallback;
const isDock=(value:unknown):value is DockSide=>DOCKS.some(([side])=>side===value);
export const horizontalDock=(dock:DockSide)=>dock==='top'||dock==='bottom';

/** Saved layout (Options storage, pages.layout); a width kept by older versions on this device fills a missing one. */
export function normalizeMainLayout(value:unknown,legacyWidth?:number):MainLayout{
  const fallback=defaultMainLayout(),saved=value&&typeof value==='object'?value as Record<string,unknown>:{};
  const float=saved.float&&typeof saved.float==='object'?saved.float as Record<string,unknown>:{};
  const dock=isDock(saved.dock)?saved.dock:fallback.dock;
  const last=isDock(saved.last_dock)&&saved.last_dock!=='float'?saved.last_dock:dock==='float'?fallback.last_dock:dock;
  return{dock,last_dock:last,
    width:clamp(number(saved.width,legacyWidth||fallback.width),PANEL_WIDTH),
    height:clamp(number(saved.height,fallback.height),PANEL_HEIGHT),
    float:{x:Math.round(number(float.x,fallback.float.x)),y:Math.round(number(float.y,fallback.float.y)),
      width:Math.round(Math.max(FLOAT_MIN.width,number(float.width,fallback.float.width))),height:Math.round(Math.max(FLOAT_MIN.height,number(float.height,fallback.float.height)))},
    filter:FILTER_DOCKS.some(([id])=>id===saved.filter)?saved.filter as FilterDock:fallback.filter};
}

/** Dock the panel at `side`; floating remembers the docked side to return to. */
export function withDock(layout:MainLayout,side:DockSide):MainLayout{
  return{...layout,dock:side,last_dock:side==='float'?(layout.dock==='float'?layout.last_dock:layout.dock):side};
}
/** Double-clicking the title bar: a docked panel floats, a floating one docks where it was. */
export const toggledDock=(layout:MainLayout):DockSide=>layout.dock==='float'?layout.last_dock:'float';

/**
 * A floating panel kept inside the viewport: it is at most as large as the
 * viewport and at least its title bar stays reachable.
 */
export function fitFloat(rect:FloatRect,viewport:{width:number;height:number}):FloatRect{
  const width=Math.round(Math.max(FLOAT_MIN.width,Math.min(rect.width,viewport.width))),height=Math.round(Math.max(FLOAT_MIN.height,Math.min(rect.height,viewport.height)));
  return{width,height,x:Math.round(Math.max(0,Math.min(rect.x,viewport.width-Math.min(width,120)))),y:Math.round(Math.max(0,Math.min(rect.y,viewport.height-40)))};
}

/**
 * Where a dragged panel goes when dropped at `point` over the workspace `area`:
 * the nearest edge within `band` pixels, or floating elsewhere.
 */
export function dropTarget(point:{x:number;y:number},area:{left:number;top:number;width:number;height:number},band:number):DockSide{
  const inside=point.x>=area.left&&point.x<=area.left+area.width&&point.y>=area.top&&point.y<=area.top+area.height;
  if(!inside)return'float';
  const distances:[DockSide,number][]=[['left',point.x-area.left],['right',area.left+area.width-point.x],['top',point.y-area.top],['bottom',area.top+area.height-point.y]];
  const [side,distance]=distances.reduce((best,item)=>item[1]<best[1]?item:best);
  return distance<=band?side:'float';
}

/** Panel width or height after dragging its splitter from `start` by `delta` pixels (toward the file list is positive for left/top). */
export function resized(layout:MainLayout,start:number,delta:number):Pick<MainLayout,'width'|'height'>{
  const dock=layout.dock;
  if(dock==='left'||dock==='right')return{width:clamp(start+(dock==='left'?delta:-delta),PANEL_WIDTH),height:layout.height};
  if(dock==='top'||dock==='bottom')return{width:layout.width,height:clamp(start+(dock==='top'?delta:-delta),PANEL_HEIGHT)};
  return{width:layout.width,height:layout.height};
}

export type ColumnItem={span:number;height:number;separator:boolean};
/**
 * Fields of a horizontal (top or bottom) panel arranged in columns: each column
 * is a 12-unit grid where fields share a row while they fit; a separator starts
 * a new column, and a column that would grow taller than `capacity` pixels
 * continues in the next one. Returns item indexes per column (separators omitted).
 */
export function panelColumns(items:ColumnItem[],capacity:number):number[][]{
  const columns:number[][]=[[]];let used=0,rowFill=12,rowHeight=0;
  items.forEach((item,index)=>{
    if(item.separator){if(columns[columns.length-1].length)columns.push([]);used=0;rowFill=12;rowHeight=0;return;}
    const span=Math.max(1,Math.min(12,item.span));
    if(rowFill+span<=12){
      // Same row: the row is as tall as its tallest field.
      const grow=Math.max(0,item.height-rowHeight);
      if(used+grow>capacity&&columns[columns.length-1].length>1){columns.push([]);used=item.height;rowFill=span;rowHeight=item.height;}
      else{used+=grow;rowFill+=span;rowHeight=Math.max(rowHeight,item.height);}
    }else if(used+item.height>capacity&&columns[columns.length-1].length){columns.push([]);used=item.height;rowFill=span;rowHeight=item.height;}
    else{used+=item.height;rowFill=span;rowHeight=item.height;}
    columns[columns.length-1].push(index);
  });
  return columns.filter(column=>column.length);
}
