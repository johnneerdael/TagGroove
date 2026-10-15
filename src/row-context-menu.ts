// Right-click menu of the File List rows, with the Tools and Utils entries that
// Mp3tag offers there. Header right-clicks belong to the column menu.
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export type ContextItem={label:string;id:string;disabled?:boolean;title?:string;shortcut?:string;userLabel?:boolean;userTitle?:boolean}|{heading:string}|{separator:true};
export type RowContextHost={
  /** Make the row part of the selection (a right-click on an unselected row selects only it). */
  prepare:(path:string)=>void;
  items:()=>ContextItem[];
  activate:(id:string)=>void;
};
export function contextMenuMarkup(items:ContextItem[]){
  return items.map(item=>'separator'in item?'<hr>':'heading'in item?`<div class="context-heading" role="presentation">${html(item.heading)}</div>`
    :`<button type="button" role="menuitem" data-context="${html(item.id)}" ${item.disabled?'disabled':''} title="${html(item.title||item.label)}"${item.userTitle?` data-verbatim="${html(item.title||item.label)}"`: ''}><span${item.userLabel?' translate="no"':''}>${html(item.label)}</span>${item.shortcut?`<small>${html(item.shortcut)}</small>`:''}</button>`).join('');
}
export class RowContextMenu {
  private menu?:HTMLElement;
  private dismiss?:(event:Event)=>void;
  constructor(private host:RowContextHost){}
  /** Bind to the (re-rendered) table container; rows are virtualized, so events are delegated. */
  bind(wrap:HTMLElement){
    wrap.addEventListener('contextmenu',event=>{
      const target=event.target as Element,row=target.closest<HTMLTableRowElement>('tr[data-path]');
      if(!row||target.closest('thead'))return;
      event.preventDefault();this.open(row,event.clientX,event.clientY);
    });
    wrap.addEventListener('keydown',event=>{
      if(!(event.key==='ContextMenu'||(event.shiftKey&&event.key==='F10')))return;
      const row=(event.target as Element).closest<HTMLTableRowElement>('tr[data-path]');if(!row)return;
      event.preventDefault();const box=row.getBoundingClientRect();this.open(row,box.left+24,box.bottom);
    });
  }
  open(row:HTMLTableRowElement,x:number,y:number){
    this.close();this.host.prepare(row.dataset.path!);
    const menu=document.createElement('div');menu.className='column-menu row-context-menu';menu.setAttribute('role','menu');menu.setAttribute('aria-label','File actions');
    menu.innerHTML=contextMenuMarkup(this.host.items());document.body.append(menu);this.menu=menu;
    menu.style.left=`${Math.max(4,Math.min(x,window.innerWidth-menu.offsetWidth-4))}px`;menu.style.top=`${Math.max(4,Math.min(y,window.innerHeight-menu.offsetHeight-4))}px`;
    const buttons=()=>[...menu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
    menu.addEventListener('click',event=>{const button=(event.target as Element).closest<HTMLButtonElement>('button[data-context]');if(!button||button.disabled)return;this.close();this.host.activate(button.dataset.context!);});
    menu.addEventListener('keydown',event=>{
      const list=buttons(),index=list.indexOf(document.activeElement as HTMLButtonElement);
      const next=event.key==='ArrowDown'?index+1:event.key==='ArrowUp'?index-1:event.key==='Home'?0:event.key==='End'?list.length-1:undefined;
      if(next===undefined)return;event.preventDefault();list[(next+list.length)%list.length]?.focus();
    });
    this.dismiss=event=>{
      if(event instanceof KeyboardEvent){if(event.key!=='Escape')return;event.preventDefault();event.stopPropagation();this.close();row.focus({preventScroll:true});return;}
      if(!menu.contains(event.target as Node))this.close();
    };
    document.addEventListener('pointerdown',this.dismiss,true);document.addEventListener('keydown',this.dismiss,true);
    buttons()[0]?.focus();
  }
  close(){
    this.menu?.remove();this.menu=undefined;
    if(this.dismiss){document.removeEventListener('pointerdown',this.dismiss,true);document.removeEventListener('keydown',this.dismiss,true);this.dismiss=undefined;}
  }
}
