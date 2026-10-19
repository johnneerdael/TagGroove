/** Matching whole metadata values. Hosts own candidates, drafts and commits. */
export type ValueSuggestion={value:string;group?:string;values?:readonly string[]};
/** Show separate values without confusing them with a scalar that contains the display delimiter. */
export const suggestionText=(choice:ValueSuggestion)=>choice.values&&choice.values.length>1?choice.values.map(value=>`“${value}”`).join(' · '):choice.value;
type SuggestionRow={group:string}|{index:number};
export type ValueSuggestionsHost={
  candidates:()=>Iterable<string|ValueSuggestion>;
  enabled:()=>boolean;
  inline?:(input:HTMLInputElement|HTMLTextAreaElement,event:Event,candidates:()=>Iterable<string>)=>boolean;
  choose:(value:string,values:readonly string[])=>void;
  beforeOpen?:()=>void;
  /** The tag panel retains its explicit, unfiltered dropdown shortcuts. */
  manualList?:boolean;
};
let nextList=0;
const WINDOW=16;
const renderedValue=(value:string)=>value.replace(/\r\n?/g,'\n');
export class ValueSuggestions{
  private popup?:HTMLDivElement;
  private values:ValueSuggestion[]=[];
  private rows:SuggestionRow[]=[];
  private optionRows:number[]=[];
  private rowHeight=30;
  private active=-1;
  private composing=false;
  private observer?:MutationObserver;
  private stateObserver:MutationObserver;
  private cleanup?:()=>void;
  private id=`value-suggestions-${++nextList}`;
  constructor(private input:HTMLInputElement|HTMLTextAreaElement,private host:ValueSuggestionsHost){
    input.setAttribute('role','combobox');input.setAttribute('aria-haspopup','listbox');
    input.setAttribute('aria-expanded','false');
    // Eligibility is independent of connectivity: cell controls are constructed before append.
    this.refresh();
    this.stateObserver=new input.ownerDocument.defaultView!.MutationObserver(()=>this.refresh());
    this.stateObserver.observe(input,{attributes:true,attributeFilter:['disabled','readonly']});
    input.addEventListener('focus',this.refresh);
    input.addEventListener('input',this.update);
    input.addEventListener('keydown',this.key,true);
    input.addEventListener('compositionstart',this.compositionStart);
    input.addEventListener('compositionend',this.compositionEnd);
    input.addEventListener('blur',this.blur);
  }
  /** Refresh accessible capability without reading candidates or touching editor text/selection. */
  refresh=()=>{
    const enabled=!this.input.disabled&&!this.input.readOnly&&this.host.enabled();
    this.input.setAttribute('aria-autocomplete',enabled?(this.host.inline?'both':'list'):'none');
    if(!enabled)this.close();
    return enabled;
  };
  private available(){return this.refresh()&&this.input.isConnected;}
  private blur=()=>{this.refresh();this.close();};
  private compositionStart=()=>{this.refresh();this.composing=true;this.close();};
  private compositionEnd=()=>{this.composing=false;this.refresh();};
  private update=(event:Event)=>{
    const enabled=this.refresh(),kind=(event as InputEvent).inputType;
    if(this.composing||(event as InputEvent).isComposing||kind==='insertCompositionText'){this.close();return;}
    if(!enabled||!this.input.isConnected){this.close();return;}
    const prefix=this.input.value;
    const candidates=[...this.host.candidates()];
    if(!/[\r\n]/.test(prefix))this.host.inline?.(this.input,event,()=>candidates
      .filter(candidate=>typeof candidate==='string'||!candidate.values||(candidate.values.length===1&&candidate.values[0]===candidate.value))
      .map(candidate=>typeof candidate==='string'?candidate:candidate.value)
      .filter(value=>!/[\r\n]/.test(value)&&value.trim()===value&&!value.includes('\\\\')));
    this.show(prefix,candidates);
  };
  private show(prefix:string,candidates:Iterable<string|ValueSuggestion>,allowEmpty=false){
    this.close();if(!prefix&&!allowEmpty)return;
    const lower=renderedValue(prefix).toLocaleLowerCase(),seen=new Set<string>();
    this.values=[];this.rows=[];this.optionRows=[];
    let group:string|undefined;
    for(const candidate of candidates){
      const value:ValueSuggestion=typeof candidate==='string'?{value:candidate}:{...candidate,values:candidate.values?[...candidate.values]:undefined};
      const identity=JSON.stringify(value.values??[value.value]);
      if(seen.has(identity)||!renderedValue(value.value).toLocaleLowerCase().startsWith(lower))continue;
      seen.add(identity);
      if(value.group&&value.group!==group)this.rows.push({group:value.group});
      group=value.group;this.optionRows.push(this.rows.length);this.rows.push({index:this.values.length});this.values.push(value);
    }
    if(!this.values.length)return;
    this.host.beforeOpen?.();
    const input=this.input,doc=input.ownerDocument,view=doc.defaultView!,popup=doc.createElement('div');
    popup.className='combo-list value-suggestions';popup.id=this.id;popup.setAttribute('role','listbox');
    // Reuse the editor's accessible name; metadata values remain literal and untranslated.
    popup.setAttribute('aria-label',input.getAttribute('aria-label')||input.dataset.field||'');popup.setAttribute('translate','no');
    popup.addEventListener('pointerdown',event=>event.preventDefault());
    popup.addEventListener('click',event=>{const option=(event.target as Element).closest<HTMLElement>('[data-suggestion]');if(option&&this.available())this.choose(Number(option.dataset.suggestion));});
    popup.addEventListener('scroll',()=>this.render());
    (input.closest('dialog[open]')||doc.body).append(popup);this.popup=popup;
    // CSS owns responsive target geometry; virtual rows use that same computed size.
    this.rowHeight=parseFloat(view.getComputedStyle(popup).getPropertyValue('--value-suggestion-row-height'))||30;
    const rect=input.getBoundingClientRect(),width=Math.min(Math.max(rect.width,200),view.innerWidth-8);
    Object.assign(popup.style,{left:`${Math.max(4,Math.min(rect.left,view.innerWidth-width-4))}px`,width:`${width}px`});
    const below=view.innerHeight-rect.bottom-8,above=rect.top-8;
    if(below<150&&above>below){popup.style.bottom=`${view.innerHeight-rect.top+2}px`;popup.style.maxHeight=`${Math.min(300,above)}px`;}
    else{popup.style.top=`${rect.bottom+2}px`;popup.style.maxHeight=`${Math.max(60,Math.min(300,below))}px`;}
    input.setAttribute('aria-controls',this.id);input.setAttribute('aria-expanded','true');
    this.render();
    const outside=(event:Event)=>{if(!popup.contains(event.target as Node)&&event.target!==input)this.close();};
    const scroll=(event:Event)=>{if(!popup.contains(event.target as Node))this.close();};
    const dialog=input.closest('dialog');
    doc.addEventListener('pointerdown',outside,true);doc.addEventListener('scroll',scroll,true);view.addEventListener('resize',this.close);dialog?.addEventListener('close',this.close);
    this.observer=new view.MutationObserver(()=>{if(!this.available()||input.closest('[hidden]')||(dialog&&!dialog.open))this.close();});
    this.observer.observe(doc.body,{subtree:true,childList:true,attributes:true,attributeFilter:['disabled','readonly','hidden','open']});
    this.cleanup=()=>{doc.removeEventListener('pointerdown',outside,true);doc.removeEventListener('scroll',scroll,true);view.removeEventListener('resize',this.close);dialog?.removeEventListener('close',this.close);};
  }
  private render(){
    const popup=this.popup;if(!popup)return;
    const start=Math.max(0,Math.min(this.rows.length-WINDOW,Math.floor(popup.scrollTop/this.rowHeight)-2)),end=Math.min(this.rows.length,start+WINDOW);
    const activeRow=this.optionRows[this.active];
    if(activeRow===undefined||activeRow<start||activeRow>=end){this.active=-1;this.input.removeAttribute('aria-activedescendant');}
    popup.replaceChildren();
    const spacer=(height:number)=>{const node=popup.ownerDocument.createElement('div');node.style.height=`${height}px`;node.setAttribute('aria-hidden','true');popup.append(node);};
    spacer(start*this.rowHeight);
    let container:HTMLElement=popup,currentGroup:string|undefined;
    const inGroup=(group:string|undefined)=>{
      if(group===currentGroup)return;
      currentGroup=group;container=popup;
      if(group){container=popup.ownerDocument.createElement('div');container.setAttribute('role','group');container.setAttribute('aria-label',group);popup.append(container);}
    };
    for(let row=start;row<end;row++){
      const item=this.rows[row];
      if('group' in item){
        inGroup(item.group);const heading=popup.ownerDocument.createElement('div');heading.className='combo-group';heading.textContent=item.group;heading.setAttribute('aria-hidden','true');container.append(heading);continue;
      }
      const {index}=item,value=this.values[index];inGroup(value.group);
      const option=popup.ownerDocument.createElement('div');option.id=`${this.id}-${index}`;option.dataset.suggestion=String(index);option.setAttribute('role','option');
      option.setAttribute('aria-selected',String(index===this.active));option.setAttribute('aria-posinset',String(index+1));option.setAttribute('aria-setsize',String(this.values.length));
      option.classList.toggle('active',index===this.active);option.textContent=suggestionText(value);option.title=value.values?.join('\n')??value.value;container.append(option);
    }
    spacer((this.rows.length-end)*this.rowHeight);
  }
  private key=(raw:Event)=>{
    const event=raw as KeyboardEvent,enabled=this.refresh();
    if(this.composing||event.isComposing){event.stopImmediatePropagation();return;}
    if(!enabled||!this.input.isConnected){this.close();return;}
    if(this.host.manualList&&event.altKey&&(event.key==='ArrowDown'||event.key==='ArrowUp')){this.close();return;}
    if(event.altKey&&event.key==='ArrowDown'&&!this.popup){this.show(this.input.value,this.host.candidates(),true);if(this.popup){event.preventDefault();event.stopImmediatePropagation();}return;}
    if(!this.popup)return;
    if(event.key==='Tab'){this.close();return;}
    if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();this.close();return;}
    if(event.key==='ArrowDown'||event.key==='ArrowUp'){
      event.preventDefault();event.stopImmediatePropagation();const step=event.key==='ArrowDown'?1:-1;
      this.active=this.active<0?(step>0?0:this.values.length-1):Math.max(0,Math.min(this.values.length-1,this.active+step));
      const top=this.optionRows[this.active]*this.rowHeight,bottom=top+this.rowHeight,height=this.popup.clientHeight||300;
      if(top<this.popup.scrollTop)this.popup.scrollTop=top;else if(bottom>this.popup.scrollTop+height)this.popup.scrollTop=bottom-height;
      this.render();this.input.setAttribute('aria-activedescendant',`${this.id}-${this.active}`);return;
    }
    if(event.key==='Enter'&&this.active>=0){event.preventDefault();event.stopImmediatePropagation();this.choose(this.active);}
  };
  private choose(index:number){
    const choice=this.values[index];if(!choice)return;
    const value=choice.value,values=[...(choice.values??[value])];this.close();
    this.input.value=this.input.tagName==='TEXTAREA'?renderedValue(value):value;this.input.setSelectionRange(this.input.value.length,this.input.value.length);this.host.choose(value,values);
  }
  close=()=>{
    if(!this.popup)return;this.popup.remove();this.popup=undefined;this.values=[];this.rows=[];this.optionRows=[];this.active=-1;this.observer?.disconnect();this.observer=undefined;this.cleanup?.();this.cleanup=undefined;
    this.input.setAttribute('aria-expanded','false');this.input.removeAttribute('aria-controls');this.input.removeAttribute('aria-activedescendant');
  };
  dispose(){this.close();this.stateObserver.disconnect();this.input.removeEventListener('focus',this.refresh);this.input.removeEventListener('input',this.update);this.input.removeEventListener('keydown',this.key,true);this.input.removeEventListener('compositionstart',this.compositionStart);this.input.removeEventListener('compositionend',this.compositionEnd);this.input.removeEventListener('blur',this.blur);}
}
