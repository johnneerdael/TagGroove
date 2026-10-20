import {ValueSuggestions} from './value-suggestions';
// View → Extended Tags (docs.mp3tag.de/extended-tags/): every field of the
// selected files with multi-line values, the cover window, and << / >> to step
// through the File List one file at a time. Save tags writes the displayed
// selection; Keep changes stages edits. Cover commands write at once and can be undone.
import './extended-tags.css';
import {extendedCaption,fieldClipText,fieldNameIdentity,neighbourFile,pasteFields,resolveTagEdits,selectionTagRows,type FieldClip,type Fields,type TagRowEdit} from './tag-edits';
import {contextMenuMarkup} from './row-context-menu';
import type {ExtendedSaveMode} from './list-behaviour';

type Completer=(input:HTMLInputElement|HTMLTextAreaElement,event:Event,candidates:()=>Iterable<string>)=>boolean;
export type ExtendedTagsHost={
  /** Translate new popup content before reading layout dimensions. */
  translateNow?:(root:HTMLElement)=>void;
  /** Selected files in File List order. */
  selection:()=>string[];
  /** Files of the File List in display order. */
  listed:()=>string[];
  /** A file's fields including pending changes. */
  fields:(path:string)=>Fields;
  describe:(path:string)=>{name:string;tag?:string};
  stage:(changes:{path:string;updates:Fields}[])=>void;
  /** Make `path` the only selected file and reveal it in the File List. */
  select:(path:string)=>void;
  /** Values offered while typing a field value. */
  completions:(field:string)=>Iterable<string>;
  /** Options → General auto-completion: the completer while it is enabled. */
  autocomplete:()=>Completer|undefined;
  coverMarkup:()=>string;
  bindCover:(root:HTMLElement)=>void;
  modal:(title:string,content:string,buttons:string)=>HTMLDialogElement;
  modalError:(dialog:HTMLDialogElement,error:unknown)=>void;
  /** The dialog closed after staging edits for `staged` (some may be saved already, by save on move). */
  done:(staged:string[])=>void;
  /** Options → Tags → Save mode at extended tag dialog. */
  saveMode:()=>ExtendedSaveMode;
  /** Write pending changes of the displayed files together; reject if any remain unsaved. */
  save:(paths:string[])=>Promise<void>;
  /** A current save-back limitation; Keep changes remains available. */
  saveReason?:()=>string|undefined;
  /** Copied fields also go to the system clipboard as text. */
  copyText?:(text:string)=>void;
};

const esc=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));

export class ExtendedTags{
  private dialog?:HTMLDialogElement;
  private files:string[]=[];
  private fieldSnapshots=new Map<string,Fields>();
  private stagedKeys=new Map<string,string[]>();
  private originals=new WeakMap<HTMLTextAreaElement,string>();
  private rowSetters=new WeakMap<HTMLElement,(values:string[])=>void>();
  private staged=new Set<string>();
  /** Fields copied with the field list's context menu; kept while TagGroove runs. */
  private clip:FieldClip[]=[];
  private menu?:HTMLElement;
  private suggestions:ValueSuggestions[]=[];
  private closeSuggestions(){this.suggestions.forEach(list=>list.dispose());this.suggestions=[];}
  constructor(private host:ExtendedTagsHost){}
  get open(){return !!this.dialog?.open;}

  show(){
    this.files=this.host.selection();if(!this.files.length)return;
    this.staged=new Set();
    const dialog=this.host.modal('Extended tags',`<div class="extended-layout"><div class="extended-fields"><p class="extended-hint"></p><div id="extended-rows"></div><button type="button" id="add-field">＋ Add field</button></div><section class="cover-section extended-cover" aria-label="Cover art"></section></div>`,
      `<button type="button" class="extended-nav" data-nav="-1" aria-label="Previous file" title="Previous file in the File List">&lt;&lt;</button><button type="button" class="extended-nav" data-nav="1" aria-label="Next file" title="Next file in the File List">&gt;&gt;</button><button type="button" id="apply-extended">Keep changes</button><button type="button" class="primary" id="save-extended">Save tags</button>`);
    dialog.classList.add('extended-dialog');this.dialog=dialog;
    dialog.querySelector('#add-field')!.addEventListener('click',()=>{const added=this.addRow();this.rowSetters.set(added.row,added.setValues);this.rows().lastElementChild?.querySelector<HTMLInputElement>('.extended-name input')?.focus();});
    dialog.querySelectorAll<HTMLButtonElement>('[data-nav]').forEach(button=>button.addEventListener('click',()=>void this.navigate(Number(button.dataset.nav) as 1|-1)));
    dialog.querySelector('.extended-fields')!.addEventListener('contextmenu',event=>{
      const target=event.target as HTMLElement;if(target.matches('textarea')||target.closest('.extended-menu'))return;
      event.preventDefault();this.fieldMenu(target.closest<HTMLElement>('.extended-row'),(event as MouseEvent).clientX,(event as MouseEvent).clientY);
    });
    dialog.querySelector('.extended-fields')!.addEventListener('keydown',event=>{
      const key=event as KeyboardEvent,row=(key.target as HTMLElement).closest<HTMLElement>('.extended-row');
      if(!(key.key==='ContextMenu'||(key.shiftKey&&key.key==='F10'))||!row)return;
      key.preventDefault();const box=row.getBoundingClientRect();this.fieldMenu(row,box.left+24,box.top+24);
    });
    dialog.querySelector('#apply-extended')!.addEventListener('click',()=>{if(!this.navigating&&this.commit())dialog.close();});
    dialog.querySelector('#save-extended')!.addEventListener('click',()=>void this.saveAndClose());
    dialog.addEventListener('cancel',event=>{if(this.navigating)event.preventDefault();});
    dialog.addEventListener('close',()=>{
      this.closeMenu();this.closeSuggestions();
      if(this.dialog===dialog)this.dialog=undefined;
      if(this.staged.size)this.host.done([...this.staged]);
    });
    this.fill();
  }
  /** Redraw the cover window after the displayed file or its covers change. */
  refreshCover(){
    if(this.saving)return;
    const section=this.dialog?.open?this.dialog.querySelector<HTMLElement>('.extended-cover'):null;if(!section)return;
    section.innerHTML=this.host.coverMarkup();this.host.bindCover(section);
  }

  private rows(){return this.dialog!.querySelector<HTMLElement>('#extended-rows')!;}
  private fill(){
    const dialog=this.dialog!,single=this.files.length===1,list=this.host.listed();
    dialog.querySelector('header')!.textContent=extendedCaption(this.files.map(path=>this.host.describe(path)));
    dialog.querySelector('.extended-hint')!.textContent=single
      ?'Each value can contain multiple lines. Remove all values to remove a field. << and >> follow the save mode in Options → Tags.'
      :'Fields whose values differ between the selected files keep each file\'s values unless you edit them. Renaming a field keeps each file\'s values.';
    dialog.querySelectorAll<HTMLButtonElement>('[data-nav]').forEach(button=>{
      button.hidden=!single;button.disabled=!single||!neighbourFile(list,this.files[0],Number(button.dataset.nav) as 1|-1);
    });
    const save=dialog.querySelector<HTMLButtonElement>('#save-extended')!,reason=this.host.saveReason?.();
    save.disabled=!!reason;save.title=reason??'';
    this.closeSuggestions();
    this.rows().replaceChildren();
    this.fieldSnapshots=new Map(this.files.map(path=>[path,Object.fromEntries(Object.entries(this.host.fields(path)).map(([name,values])=>[name,[...values]]))]));
    this.stagedKeys.clear();
    selectionTagRows([...this.fieldSnapshots.values()]).forEach(({name,values,mixed})=>{const added=this.addRow(name,values,mixed);this.rowSetters.set(added.row,added.setValues);});
    dialog.querySelector('.dialog-error')?.remove();
    this.refreshCover();
    this.rows().querySelector<HTMLInputElement>('input')?.focus({preventScroll:true});
    dialog.querySelector('.dialog-content')!.scrollTop=0;
  }
  private addRow(key='',values:string[]=[],mixed=false):{row:HTMLDivElement;setValues:(values:string[])=>void}{
    const row=document.createElement('div');row.className='extended-row';row.dataset.original=key;row.dataset.changed=key?'false':'true';row.dataset.valueChanged='false';
    row.innerHTML=`<div class="extended-name"><label><span>FIELD</span><input aria-label="Field name" value="${esc(key)}" placeholder="FIELD" data-helper="field" autocomplete="off" spellcheck="false"></label><button type="button" class="remove-field" aria-label="Remove field">Remove field</button></div><div class="extended-values"><div class="value-list"></div><span class="value-state" ${mixed?'data-mixed':''}>${mixed?'Mixed values — keep existing values':''}</span><button type="button" class="add-value">＋ Add value</button></div>`;
    const valueList=row.querySelector('.value-list')!,state=row.querySelector<HTMLElement>('.value-state')!,name=row.querySelector<HTMLInputElement>('.extended-name input')!;
    const changed=()=>{row.dataset.changed='true';row.dataset.valueChanged='true';delete state.dataset.mixed;state.textContent=valueList.childElementCount?'':'No values — this field will be removed';};
    const refreshLabels=()=>valueList.querySelectorAll<HTMLDivElement>('.extended-value').forEach((el,index)=>{el.querySelector('span')!.textContent=`VALUE ${index+1}`;el.querySelector('textarea')!.setAttribute('aria-label',`Field value ${index+1}`);el.querySelector('button')!.setAttribute('aria-label',`Remove value ${index+1}`);});
    const addValue=(text:string)=>{
      const item=document.createElement('div');item.className='extended-value';item.innerHTML='<label><span>VALUE</span><textarea aria-label="Field value" spellcheck="false"></textarea></label><button type="button" aria-label="Remove value">×</button>';
      const textarea=item.querySelector('textarea')!;textarea.value=text;this.originals.set(textarea,text);
      textarea.addEventListener('input',()=>{
        textarea.dataset.edited='true';changed();
      });
      const suggestions=new ValueSuggestions(textarea,{
        enabled:()=>!!this.host.autocomplete?.()&&!this.navigating&&!row.hidden&&!!name.value.trim()&&!fieldNameIdentity(name.value.trim()).startsWith('_'),
        candidates:()=>this.host.completions(fieldNameIdentity(name.value.trim())),
        choose:value=>{this.originals.set(textarea,value);delete textarea.dataset.edited;changed();},
      });this.suggestions.push(suggestions);
      name.addEventListener('input',()=>{suggestions.close();suggestions.refresh();});
      item.querySelector('button')!.addEventListener('click',()=>{suggestions.dispose();item.remove();refreshLabels();changed();});valueList.append(item);refreshLabels();
    };
    values.forEach(addValue);
    const setValues=(next:string[])=>{valueList.replaceChildren();next.forEach(value=>{addValue(value);valueList.lastElementChild!.querySelector('textarea')!.dataset.edited='true';});changed();};
    name.addEventListener('input',()=>row.dataset.changed='true');
    row.querySelector('.remove-field')!.addEventListener('click',()=>{row.dataset.changed='true';row.dataset.removed='true';row.hidden=true;});
    row.querySelector('.add-value')!.addEventListener('click',()=>{addValue('');changed();valueList.querySelector<HTMLTextAreaElement>('.extended-value:last-child textarea')!.focus();});
    this.rows().append(row);
    return{row,setValues};
  }
  /** Values shown in a row; undefined for a field whose values differ between the files. */
  private rowValues(row:HTMLElement):string[]|undefined{
    if(row.dataset.valueChanged!=='true'&&row.querySelector<HTMLElement>('.value-state')?.dataset.mixed!==undefined)return undefined;
    return[...row.querySelectorAll<HTMLTextAreaElement>('.extended-value textarea')].map(textarea=>textarea.value);
  }
  private closeMenu(){this.menu?.remove();this.menu=undefined;}
  /** Field list context menu: Cut, Copy and Paste fields (Mp3tag's field list menu). */
  private fieldMenu(row:HTMLElement|null,x:number,y:number){
    if(this.navigating)return;
    this.closeMenu();const dialog=this.dialog;if(!dialog)return;
    const name=row?fieldNameIdentity(row.querySelector('input')!.value.trim()):'',values=row?this.rowValues(row):undefined;
    const copyable=!!row&&!!name&&values!==undefined;
    const menu=document.createElement('div');menu.className='column-menu row-context-menu extended-menu';menu.setAttribute('role','menu');menu.setAttribute('aria-label','Field actions');
    menu.innerHTML=contextMenuMarkup([
      {label:'Cut field',id:'cut',disabled:!copyable,title:copyable?`Cut ${name}`:'Choose a field whose values are the same in every file'},
      {label:'Copy field',id:'copy',disabled:!copyable,title:copyable?`Copy ${name}`:'Choose a field whose values are the same in every file'},
      {label:this.clip.length>1?`Paste ${this.clip.length} fields`:this.clip.length?`Paste ${this.clip[0].name}`:'Paste field',id:'paste',disabled:!this.clip.length},
    ]);
    dialog.append(menu);this.menu=menu;this.host.translateNow?.(menu);
    menu.style.left=`${Math.max(4,Math.min(x,window.innerWidth-menu.offsetWidth-4))}px`;menu.style.top=`${Math.max(4,Math.min(y,window.innerHeight-menu.offsetHeight-4))}px`;
    menu.addEventListener('click',event=>{
      const button=(event.target as Element).closest<HTMLButtonElement>('button[data-context]');if(!button||button.disabled)return;
      this.closeMenu();const command=button.dataset.context;
      if((command==='cut'||command==='copy')&&row&&values){
        this.clip=[{name,values}];this.host.copyText?.(fieldClipText(this.clip));
        if(command==='cut'){row.dataset.changed='true';row.dataset.removed='true';row.hidden=true;}
      }
      if(command==='paste')this.paste();
    });
    menu.addEventListener('keydown',event=>{
      if(event.key==='Escape'){event.preventDefault();event.stopPropagation();this.closeMenu();row?.querySelector('input')?.focus();return;}
      const list=[...menu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')],index=list.indexOf(document.activeElement as HTMLButtonElement);
      if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();list[(index+(event.key==='ArrowDown'?1:list.length-1))%list.length]?.focus();}
    });
    const dismiss=(event:Event)=>{if(!menu.contains(event.target as Node)){this.closeMenu();dialog.removeEventListener('pointerdown',dismiss,true);}};
    dialog.addEventListener('pointerdown',dismiss,true);
    menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
  }
  private paste(){
    const rows=[...this.rows().querySelectorAll<HTMLDivElement>('.extended-row')];
    for(const {index,field} of pasteFields(rows.map(row=>({name:row.querySelector('input')!.value,removed:row.dataset.removed==='true'})),this.clip)){
      if(index>=0){const row=rows[index];row.dataset.changed='true';row.dataset.valueChanged='true';this.rowSetters.get(row)?.(field.values);}
      else{const added=this.addRow(field.name,[]);added.setValues(field.values);this.rowSetters.set(added.row,added.setValues);}
    }
  }
  /** Stage the dialog's edits for the files it shows. False (with the error shown) when a row is invalid. */
  private commit():boolean{
    const edits:TagRowEdit[]=[...this.rows().querySelectorAll<HTMLDivElement>('.extended-row')].map(row=>({
      original:row.dataset.original||'',name:fieldNameIdentity(row.querySelector('input')!.value.trim()),
      values:[...row.querySelectorAll<HTMLTextAreaElement>('.extended-value textarea')].map(textarea=>textarea.dataset.edited==='true'?textarea.value:this.originals.get(textarea)??textarea.value),
      changed:row.dataset.changed==='true',valueChanged:row.dataset.valueChanged==='true',removed:row.dataset.removed==='true'}));
    if(!edits.some(edit=>edit.changed))return true;
    try{
      const changes=this.files.map(path=>{
        const snapshot=this.fieldSnapshots.get(path)!,updates=resolveTagEdits(snapshot,edits);
        const original=new Map(Object.entries(snapshot).map(([name,values])=>[fieldNameIdentity(name),values]));
        const current=new Set(Object.keys(updates).map(fieldNameIdentity));
        // A retry may correct a previously staged field name. Restore keys
        // omitted by the new intent, including files already saved in a partial result.
        for(const name of this.stagedKeys.get(path)??[])if(!current.has(fieldNameIdentity(name)))updates[name]=[...original.get(fieldNameIdentity(name))??[]];
        return{path,updates};
      }).filter(change=>Object.keys(change.updates).length);
      this.host.stage(changes);changes.forEach(change=>{this.staged.add(change.path);this.stagedKeys.set(change.path,Object.keys(change.updates));});
      return true;
    }catch(error){this.host.modalError(this.dialog!,error);return false;}
  }
  private changed(){return[...this.rows().querySelectorAll<HTMLDivElement>('.extended-row')].some(row=>row.dataset.changed==='true');}
  private lockControls(dialog:HTMLDialogElement){
    this.closeMenu();this.suggestions.forEach(list=>list.close());this.saving=true;
    const controls=[...dialog.querySelectorAll<HTMLInputElement|HTMLTextAreaElement|HTMLButtonElement>('input,textarea,button')].map(control=>[control,control.disabled] as const);
    controls.forEach(([control])=>control.disabled=true);
    return()=>{controls.forEach(([control,disabled])=>control.disabled=disabled);this.saving=false;this.refreshCover();};
  }
  private async saveAndClose(){
    const dialog=this.dialog;
    if(!dialog?.open||this.navigating||this.host.saveReason?.()||!this.commit())return;
    this.navigating=true;
    const unlock=this.lockControls(dialog);
    try{
      await this.host.save([...this.files]);
      if(this.dialog===dialog&&dialog.open)dialog.close();
    }catch(error){if(this.dialog===dialog&&dialog.open)this.host.modalError(dialog,error);}
    finally{unlock();this.navigating=false;}
  }
  /** << and >>: what happens to changed fields follows Options → Tags → Save mode at extended tag dialog. */
  private async navigate(step:1|-1){
    if(this.files.length!==1||this.navigating)return;
    const path=this.files[0],target=neighbourFile(this.host.listed(),path,step);if(!target)return;
    this.navigating=true;
    try{
      let mode=this.changed()?this.host.saveMode():'pending';
      if(mode==='ask'){const answer=await this.askSave(path);if(!answer)return;mode=answer;}
      if(mode!=='ignore'&&!this.commit())return;
      if(mode==='save'&&this.staged.has(path)){
        const unlock=this.lockControls(this.dialog!);
        try{await this.host.save([path]);}finally{unlock();}
      }
      if(!this.dialog?.open)return;
      this.host.select(target);this.files=[target];this.fill();
    }catch(error){if(this.dialog)this.host.modalError(this.dialog,error);}
    finally{this.navigating=false;}
  }
  private navigating=false;
  private saving=false;
  /** Save mode "Ask": save, discard, or stay on this file (Cancel). */
  private askSave(path:string):Promise<'save'|'ignore'|undefined>{
    return new Promise(resolve=>{
      let answer:'save'|'ignore'|undefined;
      const ask=this.host.modal('Extended tags',`<p>Save the changed fields of ${esc(this.host.describe(path).name)}?</p>`,'<button id="extended-discard">Don\'t save</button><button class="primary" id="extended-save">Save</button>');
      ask.classList.add('extended-ask-dialog');
      ask.querySelector('#extended-save')!.addEventListener('click',()=>{answer='save';ask.close();});
      ask.querySelector('#extended-discard')!.addEventListener('click',()=>{answer='ignore';ask.close();});
      ask.addEventListener('close',()=>resolve(answer));
      ask.querySelector<HTMLButtonElement>('#extended-save')!.focus();
    });
  }
}
