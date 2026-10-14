export type CellTarget={path:string;column:string};
export type CellEditorHost={
  /** Make the target row rendered and return its cell. */
  cell:(target:CellTarget)=>HTMLTableCellElement|null;
  initial:(target:CellTarget)=>string;
  label?:(target:CellTarget)=>string;
  /** Apply the edit; resolves to the file's path afterwards (renames change it). */
  commit:(target:CellTarget,value:string)=>Promise<string|undefined>|string|undefined;
  next:(target:CellTarget,direction:1|-1)=>CellTarget|undefined;
  ended?:()=>void;
};

/** Next editable cell: along the row, then wrapping to the adjacent row. */
export function adjacentCell(rows:string[],columns:string[],target:CellTarget,direction:1|-1):CellTarget|undefined{
  if(!columns.length)return undefined;
  const row=rows.indexOf(target.path),column=columns.indexOf(target.column);
  if(row<0)return undefined;
  const next=column+direction;
  if(column>=0&&next>=0&&next<columns.length)return{path:target.path,column:columns[next]};
  const path=rows[row+direction];
  return path===undefined?undefined:{path,column:columns[direction>0?0:columns.length-1]};
}

/**
 * Inline File List editor. The input floats inside the scrolling table container,
 * so virtual row re-rendering and full workspace renders do not discard typing.
 */
export class CellEditor{
  private input?:HTMLInputElement;
  target?:CellTarget;
  private finishing=false;
  constructor(private host:CellEditorHost){}
  get active(){return!!this.target;}
  start(target:CellTarget){
    this.cancel();
    const cell=this.host.cell(target);if(!cell)return false;
    const input=document.createElement('input');input.className='cell-editor';input.value=this.host.initial(target);
    input.setAttribute('aria-label',`Edit ${this.host.label?.(target)||'cell'}`);
    input.spellcheck=false;input.autocomplete='off';
    this.input=input;this.target=target;
    input.addEventListener('keydown',event=>{
      event.stopPropagation();
      if(event.key==='Enter'){event.preventDefault();void this.finish();}
      else if(event.key==='Escape'){event.preventDefault();this.cancel();}
      else if(event.key==='Tab'){event.preventDefault();void this.finish(event.shiftKey?-1:1);}
    });
    input.addEventListener('blur',()=>{if(input.isConnected&&this.input===input&&!this.finishing)void this.finish();});
    input.addEventListener('click',event=>event.stopPropagation());
    input.addEventListener('dblclick',event=>event.stopPropagation());
    this.place(cell);input.focus();input.select();
    return true;
  }
  private place(cell:HTMLTableCellElement){
    const input=this.input!,wrap=cell.closest<HTMLElement>('.table-wrap');if(!wrap)return;
    const box=cell.getBoundingClientRect(),frame=wrap.getBoundingClientRect();
    Object.assign(input.style,{left:`${box.left-frame.left+wrap.scrollLeft}px`,top:`${box.top-frame.top+wrap.scrollTop}px`,width:`${Math.max(box.width,80)}px`,height:`${box.height}px`});
    if(input.parentElement!==wrap)wrap.append(input);
  }
  /** Restore the editor after the table DOM was rebuilt. */
  reattach(){
    if(!this.input||!this.target)return;
    const input=this.input,start=input.selectionStart,end=input.selectionEnd,cell=this.host.cell(this.target);
    if(!cell){this.cancel();return;}
    this.place(cell);input.focus({preventScroll:true});
    try{if(start!==null&&end!==null)input.setSelectionRange(start,end);}catch{/* Selection is not supported by every input type. */}
  }
  cancel(){
    const input=this.input;this.input=undefined;this.target=undefined;
    if(input){input.remove();this.host.ended?.();}
  }
  async finish(move?:1|-1){
    const input=this.input,target=this.target;if(!input||!target||this.finishing)return;
    this.finishing=true;const value=input.value;
    this.input=undefined;this.target=undefined;input.remove();
    try{
      const path=await this.host.commit(target,value);
      const next=move&&path?this.host.next({...target,path},move):undefined;
      if(next)this.start(next);else this.host.ended?.();
    }finally{this.finishing=false;}
  }
}
