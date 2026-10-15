// View → Extended Tags (docs.mp3tag.de/extended-tags/): every field of the
// selected files with multi-line values, the cover window, and << / >> to step
// through the File List one file at a time. Field edits become pending changes
// (Save tags writes them); cover commands write at once and can be undone.
import './extended-tags.css';
import {extendedCaption,fieldNameIdentity,neighbourFile,resolveTagEdits,selectionTagRows,type Fields,type TagRowEdit} from './tag-edits';

type Completer=(input:HTMLInputElement|HTMLTextAreaElement,event:Event,candidates:()=>Iterable<string>)=>boolean;
export type ExtendedTagsHost={
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
  done:(message:string)=>void;
};

const esc=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));

export class ExtendedTags{
  private dialog?:HTMLDialogElement;
  private files:string[]=[];
  private originals=new WeakMap<HTMLTextAreaElement,string>();
  private staged=new Set<string>();
  constructor(private host:ExtendedTagsHost){}
  get open(){return !!this.dialog?.open;}

  show(){
    this.files=this.host.selection();if(!this.files.length)return;
    this.staged=new Set();
    const dialog=this.host.modal('Extended tags',`<div class="extended-layout"><div class="extended-fields"><p class="extended-hint"></p><div id="extended-rows"></div><button type="button" id="add-field">＋ Add field</button></div><section class="cover-section extended-cover" aria-label="Cover art"></section></div>`,
      `<button type="button" class="extended-nav" data-nav="-1" aria-label="Previous file" title="Previous file in the File List">&lt;&lt;</button><button type="button" class="extended-nav" data-nav="1" aria-label="Next file" title="Next file in the File List">&gt;&gt;</button><button class="primary" id="apply-extended">Keep changes</button>`);
    dialog.classList.add('extended-dialog');this.dialog=dialog;
    dialog.querySelector('#add-field')!.addEventListener('click',()=>{this.addRow();this.rows().lastElementChild?.querySelector<HTMLInputElement>('.extended-name input')?.focus();});
    dialog.querySelectorAll<HTMLButtonElement>('[data-nav]').forEach(button=>button.addEventListener('click',()=>this.navigate(Number(button.dataset.nav) as 1|-1)));
    dialog.querySelector('#apply-extended')!.addEventListener('click',()=>{if(this.commit())dialog.close();});
    dialog.addEventListener('close',()=>{
      if(this.dialog===dialog)this.dialog=undefined;
      if(this.staged.size)this.host.done(`Extended tag changes for ${this.staged.size} file${this.staged.size===1?'':'s'} are pending. Save tags to write them.`);
    });
    this.fill();
  }
  /** Redraw the cover window after the displayed file or its covers change. */
  refreshCover(){
    const section=this.dialog?.open?this.dialog.querySelector<HTMLElement>('.extended-cover'):null;if(!section)return;
    section.innerHTML=this.host.coverMarkup();this.host.bindCover(section);
  }

  private rows(){return this.dialog!.querySelector<HTMLElement>('#extended-rows')!;}
  private fill(){
    const dialog=this.dialog!,single=this.files.length===1,list=this.host.listed();
    dialog.querySelector('header')!.textContent=extendedCaption(this.files.map(path=>this.host.describe(path)));
    dialog.querySelector('.extended-hint')!.textContent=single
      ?'Each value can contain multiple lines. Remove all values to remove a field. << and >> move to the previous or next file and keep this file\'s changes pending.'
      :'Fields whose values differ between the selected files keep each file\'s values unless you edit them. Renaming a field keeps each file\'s values.';
    dialog.querySelectorAll<HTMLButtonElement>('[data-nav]').forEach(button=>{
      button.hidden=!single;button.disabled=!single||!neighbourFile(list,this.files[0],Number(button.dataset.nav) as 1|-1);
    });
    this.rows().replaceChildren();
    selectionTagRows(this.files.map(path=>this.host.fields(path))).forEach(({name,values,mixed})=>this.addRow(name,values,mixed));
    dialog.querySelector('.dialog-error')?.remove();
    this.refreshCover();
    this.rows().querySelector<HTMLInputElement>('input')?.focus({preventScroll:true});
    dialog.querySelector('.dialog-content')!.scrollTop=0;
  }
  private addRow(key='',values:string[]=[],mixed=false){
    const row=document.createElement('div');row.className='extended-row';row.dataset.original=key;row.dataset.changed=key?'false':'true';row.dataset.valueChanged='false';
    row.innerHTML=`<div class="extended-name"><label><span>FIELD</span><input aria-label="Field name" value="${esc(key)}" placeholder="FIELD" data-helper="field" autocomplete="off" spellcheck="false"></label><button type="button" class="remove-field" aria-label="Remove field">Remove field</button></div><div class="extended-values"><div class="value-list"></div><span class="value-state">${mixed?'Mixed values — keep existing values':''}</span><button type="button" class="add-value">＋ Add value</button></div>`;
    const valueList=row.querySelector('.value-list')!,state=row.querySelector<HTMLElement>('.value-state')!,name=row.querySelector<HTMLInputElement>('.extended-name input')!;
    const changed=()=>{row.dataset.changed='true';row.dataset.valueChanged='true';state.textContent=valueList.childElementCount?'':'No values — this field will be removed';};
    const refreshLabels=()=>valueList.querySelectorAll<HTMLDivElement>('.extended-value').forEach((el,index)=>{el.querySelector('span')!.textContent=`VALUE ${index+1}`;el.querySelector('textarea')!.setAttribute('aria-label',`Field value ${index+1}`);el.querySelector('button')!.setAttribute('aria-label',`Remove value ${index+1}`);});
    const addValue=(text:string)=>{
      const item=document.createElement('div');item.className='extended-value';item.innerHTML='<label><span>VALUE</span><textarea aria-label="Field value" spellcheck="false"></textarea></label><button type="button" aria-label="Remove value">×</button>';
      const textarea=item.querySelector('textarea')!;textarea.value=text;this.originals.set(textarea,text);
      textarea.addEventListener('input',event=>{
        textarea.dataset.edited='true';changed();
        const field=fieldNameIdentity(name.value.trim());
        if(field&&!field.startsWith('_'))this.host.autocomplete()?.(textarea,event,()=>this.host.completions(field));
      });
      item.querySelector('button')!.addEventListener('click',()=>{item.remove();refreshLabels();changed();});valueList.append(item);refreshLabels();
    };
    values.forEach(addValue);
    name.addEventListener('input',()=>row.dataset.changed='true');
    row.querySelector('.remove-field')!.addEventListener('click',()=>{row.dataset.changed='true';row.dataset.removed='true';row.hidden=true;});
    row.querySelector('.add-value')!.addEventListener('click',()=>{addValue('');changed();valueList.querySelector<HTMLTextAreaElement>('.extended-value:last-child textarea')!.focus();});
    this.rows().append(row);
  }
  /** Stage the dialog's edits for the files it shows. False (with the error shown) when a row is invalid. */
  private commit():boolean{
    const edits:TagRowEdit[]=[...this.rows().querySelectorAll<HTMLDivElement>('.extended-row')].map(row=>({
      original:row.dataset.original||'',name:fieldNameIdentity(row.querySelector('input')!.value.trim()),
      values:[...row.querySelectorAll<HTMLTextAreaElement>('.extended-value textarea')].map(textarea=>textarea.dataset.edited==='true'?textarea.value:this.originals.get(textarea)??textarea.value),
      changed:row.dataset.changed==='true',valueChanged:row.dataset.valueChanged==='true',removed:row.dataset.removed==='true'}));
    if(!edits.some(edit=>edit.changed))return true;
    try{
      const changes=this.files.map(path=>({path,updates:resolveTagEdits(this.host.fields(path),edits)})).filter(change=>Object.keys(change.updates).length);
      this.host.stage(changes);changes.forEach(change=>this.staged.add(change.path));
      return true;
    }catch(error){this.host.modalError(this.dialog!,error);return false;}
  }
  private navigate(step:1|-1){
    if(this.files.length!==1)return;
    const target=neighbourFile(this.host.listed(),this.files[0],step);
    if(!target||!this.commit())return;
    this.host.select(target);this.files=[target];this.fill();
  }
}
