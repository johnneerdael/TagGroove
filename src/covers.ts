// Cover window (context menu, keyboard, clipboard, drag and drop, larger
// viewer) and the Manage Covers dialog. Mutations go through journaled
// backend commands, so Undo restores them.
import './covers.css';
import {isTextEntry} from './keyboard';
import {PICTURE_TYPES,adjustSource,applyAdjusted,coverDetails,coverExtension,coverFileName,coverMenu,coverRef,distinctPlan,dropManaged,insideRect,isImagePath,keepPlan,managedFromSet,moveManaged,newManaged,pictureTypeLabel,planChanged,planItems,presence,replaceManaged,selectionCovers,type CoverInfo,type CoverSet,type ImageSource,type ManagedCover,type PlanItem,type SelectionCovers,type StagedImage} from './cover-model';

type Batch={errors:{path:string;error:string}[]};
/** An image file next to the audio files (Options → Tags folder-image fallback). */
export type FolderImage={name:string;mime:string;size:number;width:number|null;height:number|null;data_base64:string};
export type CoverHost={
  /** Translate new popup content before reading layout dimensions. */
  translateNow?:(root:HTMLElement)=>void;
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  /** Selected files in list order with their cover descriptors. */
  selection:()=>{path:string;covers:CoverInfo[]}[];
  /** Covers of the active file (with image data once loaded) and the displayed index. */
  displayed:()=>{path?:string;covers:CoverInfo[];index:number};
  show:(index:number)=>void;
  /** Why cover edits are unavailable, or '' when they are allowed. */
  reason:()=>string;
  clipboard:()=>boolean;
  /** True while a selected file shows pasted covers that are not saved yet. */
  pending?:()=>boolean;
  mutate:(command:string,args:Record<string,unknown>,message:string)=>Promise<Batch|undefined>;
  pickImage:(directory?:string)=>Promise<string|null>;
  /** Folder Add cover starts in (Options → Directories → Cover art directory) for the first selected file. */
  startDirectory?:(path:string)=>Promise<string>;
  /** Adjust Cover settings for Manage Covers, or null when cancelled. */
  adjustSettings?:()=>Promise<Record<string,unknown>|null>;
  pickSave:(defaultPath:string,extension:string)=>Promise<string|null>;
  confirm:(message:string,title:string)=>Promise<boolean>;
  modal:(title:string,content:string,buttons:string)=>HTMLDialogElement;
  modalError:(dialog:HTMLDialogElement,error:unknown)=>void;
  evaluate:(path:string,template:string)=>Promise<string>;
  adjust:()=>void;
  notify:(message:string)=>void;
  fail:(error:unknown)=>void;
  mac:boolean;
  /** Drag the displayed cover out as an image file; resolves when the drag ends. */
  dragOut?:(cover:{mime:string;data_base64:string})=>Promise<{path:string;result:string}>;
};
type MenuEntry={command:string;label:string;enabled:boolean;shortcut?:string;checked?:boolean;value?:string;children?:MenuEntry[]}|'separator';

const esc=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const dirname=(path:string)=>path.replace(/[\\/][^\\/]*$/,'')||path;
const separator=(path:string)=>path.includes('\\')&&!path.includes('/')?'\\':'/';
const dataUrl=(cover:{mime:string;data_base64:string})=>`data:${cover.mime};base64,${cover.data_base64}`;
const storage={
  get:(key:string)=>{try{return window.localStorage.getItem(key);}catch{return null;}},
  set:(key:string,value:string)=>{try{window.localStorage.setItem(key,value);}catch{/* Preferences are optional. */}},
};
const ASPECT_KEY='opentag.cover.aspect',EXTRACT_KEY='opentag.cover.extractFormat';
const modalOpen=()=>!!document.querySelector('dialog[open]:not(.find-dialog):not(.cover-viewer)');

async function fileData(file:File){
  const bytes=new Uint8Array(await file.arrayBuffer());let binary='';
  for(let i=0;i<bytes.length;i+=0x8000)binary+=String.fromCharCode(...bytes.subarray(i,i+0x8000));
  return btoa(binary);
}

export class CoverUI {
  private menu?:HTMLElement;
  private viewer?:HTMLDialogElement;
  private dragPaths:string[]=[];
  private folderShown?:FolderImage;
  /** True while a cover is being dragged out; drops of it on this window are ignored. */
  draggingOut=false;
  constructor(private host:CoverHost){}
  get aspect(){return storage.get(ASPECT_KEY)!=='stretch';}
  state():SelectionCovers{return selectionCovers(this.host.selection().map(file=>file.covers));}
  /** The cover shown in the cover window: only when every selected file has the same covers. */
  private shown(){
    const state=this.state(),{path,covers,index}=this.host.displayed();
    const cover=state.state==='same'&&covers.length?covers[Math.min(index,covers.length-1)]:undefined;
    return {state,path,covers,index:Math.min(index,Math.max(0,covers.length-1)),cover};
  }
  canRemove(){return !this.host.reason()&&this.host.selection().length>0&&this.state().state!=='none';}

  /**
   * `folder` is the first image of the files' folder, shown (and marked as not
   * embedded) when no file has a cover. `note` adds a line, for example for
   * pasted covers that are not saved yet.
   */
  markup(options:{fallback:string;error:string;unavailable:boolean;buttons:string;folder?:FolderImage;note?:string}){
    const {state,covers,index,cover}=this.shown(),loaded=!!cover?.data_base64,selected=state.files;
    const folder=!options.unavailable&&!options.error&&state.state==='none'?options.folder:undefined;
    this.folderShown=folder;
    // A UI-owned state/type, distinct from authored cover descriptions/titles.
    const coverState=options.unavailable?'Artwork unavailable':state.state==='varying'?'Covers vary':cover?pictureTypeLabel(cover.picture_type):folder?'Folder image':'No artwork';
    const line=(text:string,verbatim=false)=>({text,verbatim});
    const lines=options.error?[line(options.error)]:state.state==='varying'?[line(`${state.withCovers} of ${selected} files have artwork`),line('The selected files have different covers')]
      :cover?[line(coverDetails(cover)),line(loaded?cover.description:'Loading artwork…',loaded),line(selected>1?`Shared by ${selected} files`:'')]
      :folder?[line(folder.name,true),line(coverDetails(folder)),line('Not embedded in the file')]
      :[line(selected?'Drop or paste an image to embed it':'Embedded cover art')];
    if(options.note)lines.push(line(options.note));
    const art=loaded?`<img data-cover-image alt="${esc(`Embedded ${pictureTypeLabel(cover!.picture_type)} artwork`)}">`:folder?`<img data-folder-image data-verbatim="${esc(folder.name)}" alt="${esc(`Folder image ${folder.name}, not embedded`)}">`:`<img class="fallback-artwork" data-theme-image="artwork" src="${esc(options.fallback)}" alt="" aria-hidden="true">`;
    const label=`Cover art: ${coverState}${cover&&covers.length>1?`, ${index+1} of ${covers.length}`:''}. Enter views it larger; Shift+F10 opens cover commands.`;
    const nav=cover&&covers.length>1?`<div class="cover-nav"><button data-cover="previous" aria-label="Previous cover" ${index>0?'':'disabled'}>‹</button><span>${index+1} / ${covers.length}</span><button data-cover="next" aria-label="Next cover" ${index<covers.length-1?'':'disabled'}>›</button></div>`:'';
    return `<div class="cover-row"><div class="cover-art${this.aspect?'':' stretch'}${folder?' folder-image':''}" tabindex="0" role="button" aria-haspopup="menu" aria-label="${esc(label)}" title="Right-click for cover commands; double-click to enlarge">${art}</div><div class="cover-info"><strong>${esc(coverState)}</strong>${lines.filter(line=>line.text).map(({text,verbatim})=>`<span ${verbatim?'translate="no"':''}>${esc(text)}</span>`).join('')}${nav}</div></div><div class="cover-controls">${options.buttons}<button data-cover="menu" aria-haspopup="menu" aria-label="Cover commands" title="Cover commands">⋯</button></div>`;
  }
  paint(){
    const {cover}=this.shown();
    // The tag panel and the Extended Tags dialog can both show the cover window.
    if(cover?.data_base64)document.querySelectorAll<HTMLImageElement>('.cover-art img[data-cover-image]').forEach(image=>image.src=dataUrl(cover));
    const folder=this.folderShown;
    if(folder)document.querySelectorAll<HTMLImageElement>('.cover-art img[data-folder-image]').forEach(image=>image.src=dataUrl(folder));
    this.updateViewer();
  }
  /** Attach cover window handlers; call after the cover section is (re)rendered. */
  bind(root:ParentNode){
    const art=root.querySelector<HTMLElement>('.cover-art');
    root.querySelectorAll<HTMLButtonElement>('[data-cover]').forEach(button=>button.addEventListener('click',event=>{
      const command=button.dataset.cover!;
      if(command==='menu'){const rect=button.getBoundingClientRect();this.openMenu(rect.left,rect.bottom+2,true,button);}
      else void this.run(command);
      event.stopPropagation();
    }));
    if(!art)return;
    art.addEventListener('contextmenu',event=>{event.preventDefault();art.focus({preventScroll:true});this.openMenu(event.clientX,event.clientY,false,art);});
    art.addEventListener('dblclick',()=>void this.run('view'));
    art.addEventListener('keydown',event=>this.artKey(event,art));
    art.addEventListener('dragover',event=>{if(event.dataTransfer?.types.includes('Files')){event.preventDefault();art.classList.add('drop-target');}});
    art.addEventListener('dragleave',()=>art.classList.remove('drop-target'));
    art.addEventListener('drop',event=>{art.classList.remove('drop-target');const files=[...event.dataTransfer?.files||[]];if(files.length){event.preventDefault();void this.addFiles(files);}});
    // Dragging the displayed cover out of the window copies it as an image file
    // (Mp3tag: drag a cover to the desktop). The webview's own image drag is off.
    art.addEventListener('dragstart',event=>event.preventDefault());
    art.querySelectorAll('img').forEach(image=>image.draggable=false);
    art.addEventListener('pointerdown',event=>{
      if(event.button!==0||!this.host.dragOut||this.draggingOut)return;
      const {cover}=this.shown();if(!cover?.data_base64)return;
      const start={x:event.clientX,y:event.clientY};
      const stop=()=>{window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',stop);window.removeEventListener('pointercancel',stop);};
      const move=(moved:PointerEvent)=>{if(Math.hypot(moved.clientX-start.x,moved.clientY-start.y)<6)return;stop();void this.dragOut(cover);};
      window.addEventListener('pointermove',move);window.addEventListener('pointerup',stop);window.addEventListener('pointercancel',stop);
    });
    art.addEventListener('paste',event=>{const files=[...event.clipboardData?.files||[]];if(files.length){event.preventDefault();event.stopPropagation();void this.addFiles(files);}});
  }
  private async dragOut(cover:{mime:string;data_base64:string}){
    if(!this.host.dragOut||this.draggingOut)return;
    this.draggingOut=true;
    try{
      const {path,result}=await this.host.dragOut(cover);
      const name=path.split(/[\\/]/).pop()||path;
      if(result==='dropped')this.host.notify(`Copied the cover as ${name}`);
      else if(result==='prepared')this.host.notify(`Cover file ready to drag: ${path}`);
    }catch(error){this.host.fail(error);}
    finally{this.draggingOut=false;}
  }
  /** Global shortcut: Ctrl+Alt+M (⌘⌥M) opens Manage covers. Ctrl+Alt is AltGr on
   * Windows, so a chord that types a character (German AltGr+M is µ) stays text. */
  install(target:Document){
    target.addEventListener('keydown',event=>{
      if(event.defaultPrevented||modalOpen())return;
      const typed=!this.host.mac&&isTextEntry(event.target)&&(event.key.toLowerCase()!=='m'||event.getModifierState?.('AltGraph'));
      if(event.altKey&&(event.ctrlKey||event.metaKey)&&!event.shiftKey&&event.code==='KeyM'&&!typed){event.preventDefault();event.stopPropagation();void this.run('manage');}
    },true);
  }
  private artKey(event:KeyboardEvent,art:HTMLElement){
    const primary=this.host.mac?event.metaKey:event.ctrlKey,key=event.key.toLowerCase();
    let command='';
    if(event.key==='ContextMenu'||(event.shiftKey&&event.key==='F10')){const rect=art.getBoundingClientRect();event.preventDefault();event.stopPropagation();this.openMenu(rect.left+8,rect.top+8,true,art);return;}
    if(!primary&&!event.altKey){
      if(event.key==='ArrowLeft')command='previous';else if(event.key==='ArrowRight')command='next';
      else if(event.key==='Enter'||event.key===' ')command='view';else if(event.key==='Delete'||event.key==='Backspace')command='remove';
    }else if(primary&&!event.altKey&&!event.shiftKey)command=key==='c'?'copy':key==='x'?'cut':key==='v'?'paste':'';
    if(!command)return;
    event.preventDefault();event.stopPropagation();void this.run(command);
  }
  private menuState(){
    const {state,covers,index,cover}=this.shown();
    return {selected:state.files,state:state.state,count:covers.length,index,displayedType:cover?.picture_type,canEdit:!this.host.reason(),canClipboard:this.host.clipboard(),aspect:this.aspect,mac:this.host.mac};
  }
  openMenu(x:number,y:number,focus:boolean,returnFocus?:HTMLElement){
    // Content outside a modal dialog is inert, so a cover window inside the Extended Tags dialog opens its menu there.
    this.showMenu(returnFocus?.closest('dialog')??document.body,x,y,coverMenu(this.menuState()) as MenuEntry[],'Cover',(command,value)=>void this.run(command,value),focus,returnFocus);
  }
  closeMenu(){this.menu?.remove();this.menu=undefined;}
  /** A keyboard-navigable popup menu with one level of submenus. */
  private showMenu(container:HTMLElement,x:number,y:number,items:MenuEntry[],label:string,choose:(command:string,value?:string)=>void,focus:boolean,returnFocus?:HTMLElement){
    this.closeMenu();
    const render=(entries:MenuEntry[]):string=>entries.map(item=>item==='separator'?'<hr>':item.children
      ?`<div class="cover-menu-parent"><button role="menuitem" aria-haspopup="menu" aria-expanded="false" data-submenu ${item.enabled?'':'disabled'}><span>${esc(item.label)}</span><small>›</small></button><div class="cover-submenu" role="menu" aria-label="${esc(item.label)}" hidden>${render(item.children)}</div></div>`
      :`<button role="${item.checked===undefined?'menuitem':'menuitemcheckbox'}" ${item.checked===undefined?'':`aria-checked="${item.checked}"`} data-command="${esc(item.command)}" ${item.value?`data-value="${esc(item.value)}"`:''} ${item.enabled?'':'disabled'}><span>${item.checked?'✓ ':''}${esc(item.label)}</span>${item.shortcut?`<small>${esc(item.shortcut)}</small>`:''}</button>`).join('');
    const menu=document.createElement('div');menu.className='cover-menu';menu.tabIndex=-1;menu.setAttribute('role','menu');menu.setAttribute('aria-label',label);menu.innerHTML=render(items);menu.style.position=container===document.body?'fixed':'absolute';
    container.append(menu);this.menu=menu;this.host.translateNow?.(menu);
    const width=menu.offsetWidth,height=menu.offsetHeight,box=container===document.body?{left:0,top:0}:container.getBoundingClientRect();
    menu.style.left=`${Math.max(4,Math.min(x,window.innerWidth-width-4))-box.left}px`;menu.style.top=`${Math.max(4,Math.min(y,window.innerHeight-height-4))-box.top}px`;
    const close=(restore:boolean)=>{this.closeMenu();document.removeEventListener('pointerdown',outside,true);if(restore)returnFocus?.focus({preventScroll:true});};
    const outside=(event:Event)=>{if(!menu.contains(event.target as Node))close(false);};
    document.addEventListener('pointerdown',outside,true);
    const level=(element:Element)=>[...element.closest('[role=menu]')!.querySelectorAll<HTMLButtonElement>(':scope > button:not([disabled]), :scope > .cover-menu-parent > button:not([disabled])')];
    const openSub=(button:HTMLButtonElement,focusFirst:boolean)=>{
      menu.querySelectorAll<HTMLElement>('.cover-submenu').forEach(sub=>{if(sub!==button.nextElementSibling){sub.hidden=true;sub.previousElementSibling?.setAttribute('aria-expanded','false');}});
      const sub=button.nextElementSibling as HTMLElement;sub.hidden=false;button.setAttribute('aria-expanded','true');this.host.translateNow?.(sub);
      sub.classList.toggle('open-left',button.getBoundingClientRect().right+sub.offsetWidth>window.innerWidth-4);
      if(focusFirst)level(sub.firstElementChild||sub)[0]?.focus();
    };
    menu.addEventListener('click',event=>{
      const button=(event.target as Element).closest<HTMLButtonElement>('button');if(!button||button.disabled)return;
      if(button.dataset.submenu!==undefined){openSub(button,false);return;}
      close(true);choose(button.dataset.command!,button.dataset.value);
    });
    menu.addEventListener('pointerover',event=>{const button=(event.target as Element).closest<HTMLButtonElement>('button[data-submenu]');if(button&&!button.disabled)openSub(button,false);});
    menu.addEventListener('keydown',event=>{
      const current=document.activeElement as HTMLButtonElement;if(!menu.contains(current))return;
      const buttons=level(current),at=buttons.indexOf(current),sub=current.closest('.cover-submenu') as HTMLElement|null;
      if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();buttons[(at+(event.key==='ArrowDown'?1:buttons.length-1))%buttons.length]?.focus();}
      else if(event.key==='Home'||event.key==='End'){event.preventDefault();buttons[event.key==='Home'?0:buttons.length-1]?.focus();}
      else if(event.key==='ArrowRight'&&current.dataset.submenu!==undefined){event.preventDefault();openSub(current,true);}
      else if((event.key==='ArrowLeft'||event.key==='Escape')&&sub){event.preventDefault();event.stopPropagation();sub.hidden=true;const parent=sub.previousElementSibling as HTMLButtonElement;parent.setAttribute('aria-expanded','false');parent.focus();}
      else if(event.key==='Escape'||event.key==='Tab'){event.preventDefault();event.stopPropagation();close(true);}
      else if(event.key==='Enter'||event.key===' '){event.preventDefault();current.click();}
    });
    if(focus)level(menu)[0]?.focus();else menu.focus();
  }

  /** Add and Replace start in the cover art directory, else the file's folder. */
  private async startFolder(path?:string){
    if(!path)return undefined;
    try{return await this.host.startDirectory?.(path)??dirname(path);}catch{return dirname(path);}
  }
  async run(command:string,value?:string){
    // Shown covers that are pasted but not saved exist only as pending tag edits.
    if(['extract','copy','cut'].includes(command)&&this.host.pending?.()){this.host.notify('Save tags first: the shown covers are pasted and not written yet.');return;}
    try{await this.perform(command,value);}catch(error){this.host.fail(error);}
  }
  private plan(change:(item:PlanItem,index:number)=>PlanItem|null,all:(item:PlanItem)=>PlanItem|null){
    const {state,index}=this.shown(),summary=this.host.selection()[0]?.covers||[];
    return state.state==='same'?keepPlan(summary,(item,i)=>i===index?change(item,i):item):distinctPlan(state.distinct,all);
  }
  private async apply(items:PlanItem[],label:string,message:string){
    const result=await this.host.mutate('apply_cover_plan',{items,label},message);
    return !!result&&!result.errors.length;
  }
  private async perform(command:string,value?:string){
    const {state,path,covers,index,cover}=this.shown(),reason=this.host.reason();
    const editing=['add','remove','replace','adjust','manage','type','description','cut','paste'];
    if(editing.includes(command)&&reason){this.host.fail(reason);return;}
    switch(command){
      case 'previous':case 'next':{const next=index+(command==='next'?1:-1);if(cover&&next>=0&&next<covers.length)this.host.show(next);return;}
      case 'view':this.openViewer();return;
      case 'aspect':storage.set(ASPECT_KEY,this.aspect?'stretch':'correct');this.host.show(index);return;
      case 'manage':await this.manage();return;
      case 'adjust':this.host.adjust();return;
      case 'add':{const image=await this.host.pickImage(await this.startFolder(path));if(image)await this.addImage({path:image},'Added cover');return;}
      case 'paste':{
        const staged=await this.host.invoke<StagedImage>('paste_cover_from_clipboard');
        await this.addImage({staged:staged.token},'Pasted cover');return;
      }
      case 'copy':case 'cut':{
        if(!cover||!path)return;
        await this.host.invoke('copy_cover_to_clipboard',{path,index:cover.index});
        if(command==='copy'){this.host.notify(`Copied the ${pictureTypeLabel(cover.picture_type)} to the clipboard`);return;}
        await this.apply(this.plan(()=>null,()=>null),'Cut cover','Cut cover');return;
      }
      case 'remove':{
        if(state.state==='none')return;
        if(state.state==='varying'&&!await this.host.confirm(`The ${state.files} selected files have different covers. Remove all covers from them?`,'Remove cover'))return;
        await this.apply(this.plan(()=>null,()=>null),'Remove cover','Removed cover');return;
      }
      case 'replace':{
        if(!cover)return;
        const image=await this.host.pickImage(await this.startFolder(path));if(!image)return;
        await this.apply(this.plan(item=>({...item,image:{path:image}}),item=>item),'Replace cover','Replaced cover');return;
      }
      case 'type':{
        if(!value||state.state==='none')return;
        await this.apply(this.plan(item=>({...item,picture_type:value}),item=>({...item,picture_type:value})),'Set cover type',`Set cover type to ${pictureTypeLabel(value)}`);return;
      }
      case 'description':{
        if(state.state==='none')return;
        const description=await this.askDescription(cover?.description||'',state.state==='varying');if(description===null)return;
        await this.apply(this.plan(item=>({...item,description}),item=>({...item,description})),'Set cover description','Set cover description');return;
      }
      case 'extract':if(cover&&path)this.extract(path,cover);return;
    }
  }
  private async addImage(image:ImageSource,message:string){
    const before=this.host.displayed().covers.length;
    const result=await this.host.mutate('add_cover_image',{image,pictureType:null},message);
    if(result&&!result.errors.length&&this.state().state==='same')this.host.show(before);
  }
  /** Files from webview drop or paste events (browser drags, synthetic clipboard data). */
  private async addFiles(files:File[]){
    const reason=this.host.reason();if(reason){this.host.fail(reason);return;}
    for(const file of files.filter(file=>file.type.startsWith('image/')||isImagePath(file.name))){
      const staged=await this.host.invoke<StagedImage>('stage_cover_data',{dataBase64:await fileData(file),label:file.name||'Pasted image'});
      await this.addImage({staged:staged.token},'Added cover');
    }
  }
  /** Native drag and drop: images dropped on the cover window are embedded. Returns true when handled. */
  dragDrop(payload:{type:string;paths?:string[];position?:{x:number;y:number}}):boolean{
    // While the Extended Tags dialog is open, its cover window is the drop target.
    const section=document.querySelector<HTMLElement>('dialog[open] .cover-section')||(modalOpen()?null:document.querySelector<HTMLElement>('.cover-section'));
    if(payload.type==='enter')this.dragPaths=payload.paths||[];
    const over=!!section&&!!payload.position&&payload.type!=='leave'&&insideRect(payload.position,section.getBoundingClientRect(),window.devicePixelRatio||1);
    section?.classList.toggle('drop-target',over&&payload.type!=='drop');
    if(!over)return false;
    const images=(payload.type==='drop'?payload.paths||[]:this.dragPaths).filter(isImagePath);
    if(payload.type==='drop'){
      this.dragPaths=[];
      if(!images.length){this.host.fail('Drop JPEG, PNG, GIF, BMP or WebP images on the cover art to embed them.');return true;}
      void (async()=>{for(const image of images){const reason=this.host.reason();if(reason){this.host.fail(reason);return;}await this.addImage({path:image},'Added cover');}})().catch(error=>this.host.fail(error));
      return true;
    }
    const overlay=document.createElement('div');overlay.className='drop-overlay cover-drop';
    overlay.textContent=images.length?`Drop to embed ${images.length===1?'the image':`${images.length} images`} as cover of ${this.host.selection().length} selected file${this.host.selection().length===1?'':'s'}`:'Only images can be dropped on the cover art';
    (section.closest('dialog')||document.body).append(overlay);return true;
  }
  private askDescription(current:string,varying:boolean):Promise<string|null>{
    return new Promise(resolve=>{
      const dialog=this.host.modal('Set cover description',`<label class="field"><span>Description</span><input id="cover-description-value" value="${esc(current)}" autocomplete="off"></label>${varying?'<p>The description is set for every cover of the selected files.</p>':''}`,'<button class="primary" id="cover-description-ok">OK</button>');
      let result:string|null=null;
      const input=dialog.querySelector<HTMLInputElement>('#cover-description-value')!;
      const ok=()=>{result=input.value;dialog.close();};
      dialog.querySelector('#cover-description-ok')!.addEventListener('click',ok);
      input.addEventListener('keydown',event=>{if(event.key==='Enter'){event.preventDefault();ok();}});
      dialog.addEventListener('close',()=>resolve(result));input.select();
    });
  }
  /** Extract cover: a name from a format string next to the audio file, or Save as…. */
  extract(path:string,cover:{index:number;mime:string;size?:number;width?:number|null;height?:number|null;picture_type:string}){
    const extension=coverExtension(cover.mime),format=storage.get(EXTRACT_KEY)||'folder';
    const dialog=this.host.modal('Extract cover',`<p><span>${esc(pictureTypeLabel(cover.picture_type))}</span> · <span>${esc(coverDetails(cover))}</span></p><label class="field"><span>File name format</span><input id="cover-extract-format" value="${esc(format)}" spellcheck="false" autocomplete="off"></label><p class="cover-extract-hint">Placeholders such as %album% or %artist% are allowed. The extension .${extension} is added.</p><p id="cover-extract-preview" role="status">Resolving file name…</p>`,`<button id="cover-extract-browse">Save as…</button><button class="primary" id="cover-extract-save">Save</button>`);
    dialog.classList.add('cover-extract-dialog');
    const input=dialog.querySelector<HTMLInputElement>('#cover-extract-format')!,preview=dialog.querySelector<HTMLElement>('#cover-extract-preview')!;
    const save=dialog.querySelector<HTMLButtonElement>('#cover-extract-save')!,browse=dialog.querySelector<HTMLButtonElement>('#cover-extract-browse')!;
    let name='',generation=0,working=false;
    const sync=()=>{save.disabled=working||!name;browse.disabled=working||!name;input.disabled=working;};
    const resolveName=async()=>{
      const current=++generation;name='';sync();
      try{const value=await this.host.evaluate(path,input.value);if(current!==generation)return;name=coverFileName(value);preview.textContent=`Saves ${name}.${extension} in the folder of the audio file.`;}
      catch(error){if(current===generation){preview.textContent='';this.host.modalError(dialog,error);}}
      sync();
    };
    let timer=0;input.addEventListener('input',()=>{window.clearTimeout(timer);timer=window.setTimeout(()=>void resolveName(),200);});
    const write=async(output:string,overwrite:boolean)=>{
      working=true;sync();dialog.querySelector('.dialog-error')?.remove();
      try{
        let result=await this.host.invoke<{path:string;exists:boolean}>('extract_cover',{path,index:cover.index,output,overwrite});
        if(result.exists){
          if(!await this.host.confirm(`${result.path} already exists. Replace it?`,'Extract cover'))return;
          result=await this.host.invoke<{path:string;exists:boolean}>('extract_cover',{path,index:cover.index,output,overwrite:true});
        }
        storage.set(EXTRACT_KEY,input.value);this.host.notify(`Saved cover to ${result.path}`);dialog.close();
      }catch(error){if(dialog.open)this.host.modalError(dialog,error);}
      finally{working=false;if(dialog.open)sync();}
    };
    save.addEventListener('click',()=>void write(name,false));
    browse.addEventListener('click',async()=>{const target=await this.host.pickSave(`${dirname(path)}${separator(path)}${name}.${extension}`,extension);if(target)await write(target,true);});
    input.addEventListener('keydown',event=>{if(event.key==='Enter'&&!save.disabled){event.preventDefault();save.click();}});
    void resolveName();input.select();
  }

  openViewer(){
    if(!this.viewer){
      const viewer=document.createElement('dialog');viewer.className='cover-viewer';viewer.setAttribute('aria-label','Cover viewer');
      document.body.append(viewer);viewer.addEventListener('close',()=>{viewer.remove();if(this.viewer===viewer)this.viewer=undefined;});
      viewer.addEventListener('keydown',event=>{
        if(event.key==='ArrowLeft'||event.key==='ArrowRight'){event.preventDefault();event.stopPropagation();void this.run(event.key==='ArrowLeft'?'previous':'next');}
        if(event.key==='Escape'){event.preventDefault();event.stopPropagation();viewer.close();}
      });
      viewer.addEventListener('click',event=>{const command=(event.target as Element).closest<HTMLElement>('[data-viewer]')?.dataset.viewer;if(command==='close')viewer.close();else if(command)void this.run(command);});
      this.viewer=viewer;viewer.show();
    }
    this.updateViewer();
    this.viewer.querySelector<HTMLElement>('[data-viewer="close"]')?.focus({preventScroll:true});
  }
  /** The viewer stays open and follows cover and file navigation. */
  updateViewer(){
    const viewer=this.viewer;if(!viewer)return;
    const {state,covers,index,cover}=this.shown();
    const body=cover?.data_base64?`<img src="${esc(dataUrl(cover))}" alt="${esc(`${pictureTypeLabel(cover.picture_type)} artwork`)}" class="${this.aspect?'':'stretch'}">`:`<p>${state.state==='varying'?'The selected files have different covers.':cover?'Loading artwork…':'No artwork'}</p>`;
    const focused=(document.activeElement as HTMLElement|null)?.dataset?.viewer;
    viewer.innerHTML=`<header><span><span>${esc(cover?pictureTypeLabel(cover.picture_type):'Cover')}</span>${cover&&covers.length>1?`<span> · ${index+1} / ${covers.length}</span>`:''}</span><button data-viewer="close" aria-label="Close cover viewer">×</button></header><div class="cover-viewer-image">${body}</div><footer><span>${cover?`<span>${esc(coverDetails(cover))}</span>${cover.description?` · <span translate="no">${esc(cover.description)}</span>`:''}`:''}</span><span class="cover-viewer-nav"><button data-viewer="previous" aria-label="Previous cover" ${cover&&index>0?'':'disabled'}>‹</button><button data-viewer="next" aria-label="Next cover" ${cover&&index<covers.length-1?'':'disabled'}>›</button></span></footer>`;
    if(focused)viewer.querySelector<HTMLElement>(`[data-viewer="${focused}"]`)?.focus({preventScroll:true});
  }

  /** Manage Covers: every cover of the selection with its presence, editable as one list. */
  async manage(){
    const reason=this.host.reason();if(reason){this.host.fail(reason);return;}
    const targets=this.host.selection().map(file=>file.path);if(!targets.length)return;
    const set=await this.host.invoke<CoverSet>('cover_set',{paths:targets});
    const initial=managedFromSet(set);
    let list=[...initial],chosen=new Set<string>(list[0]?[list[0].id]:[]),anchor=list[0]?.id,focusId=anchor,counter=0,working=false;
    const dialog=this.host.modal('Manage covers',`<p class="manage-covers-summary"></p><div class="manage-covers-layout"><ul class="manage-covers-list" role="listbox" aria-multiselectable="true" aria-label="Covers" tabindex="0"></ul><div class="manage-covers-buttons" role="group" aria-label="Cover commands"><button data-manage="add">Add…</button><button data-manage="paste">Paste</button><button data-manage="replace">Replace…</button><button data-manage="adjust">Adjust…</button><button data-manage="remove">Remove</button><hr><button data-manage="up">Move up</button><button data-manage="down">Move down</button><hr><button data-manage="all">Apply to all</button><button data-manage="extract">Extract…</button></div></div><fieldset class="manage-cover-properties"><legend>Selected covers</legend><div class="field-pair"><label class="field"><span>Cover type</span><select id="manage-cover-type"></select></label><label class="field"><span>Description</span><input id="manage-cover-description" autocomplete="off"></label></div><label class="action-check"><input type="checkbox" id="manage-cover-all"> Embed in all selected files</label></fieldset><p class="manage-covers-status" role="status"></p>`,'<button class="primary" id="manage-covers-ok">OK</button>');
    dialog.classList.add('manage-covers-dialog');
    const listbox=dialog.querySelector<HTMLUListElement>('.manage-covers-list')!,typeSelect=dialog.querySelector<HTMLSelectElement>('#manage-cover-type')!;
    const descriptionInput=dialog.querySelector<HTMLInputElement>('#manage-cover-description')!,allBox=dialog.querySelector<HTMLInputElement>('#manage-cover-all')!;
    const ok=dialog.querySelector<HTMLButtonElement>('#manage-covers-ok')!,status=dialog.querySelector<HTMLElement>('.manage-covers-status')!;
    if(set.errors.length)this.host.modalError(dialog,set.errors.map(error=>`${error.path}: ${error.error}`).join('\n'));
    const selectedCovers=()=>list.filter(cover=>chosen.has(cover.id));
    const draw=(focusList=false)=>{
      dialog.querySelector('.manage-covers-summary')!.textContent=`${set.files} selected file${set.files===1?'':'s'} · ${list.length} cover${list.length===1?'':'s'}. The first cover is shown in players.`;
      listbox.innerHTML=list.length?list.map(cover=>{const present=presence(cover);return `<li role="option" id="manage-${cover.id}" data-id="${cover.id}" aria-selected="${chosen.has(cover.id)}"><span class="cover-presence ${present.full?'full':'partial'}" title="${esc(present.text)}" aria-label="${esc(present.text)}">${present.full?'●':'◐'}${present.full?'':`<small>${cover.all_files?cover.total:cover.files}/${cover.total}</small>`}</span>${cover.data_base64?`<img src="${esc(dataUrl(cover))}" alt="">`:'<span class="manage-cover-missing" aria-hidden="true">?</span>'}<span class="manage-cover-text"><strong>${esc(pictureTypeLabel(cover.picture_type))}${cover.image?` <em>${cover.original?'replaced':'new'}</em>`:''}</strong><span ${cover.description?'translate="no"':''}>${esc(cover.description||'No description')}</span><small>${esc(coverDetails(cover))}</small></span></li>`;}).join(''):'<li class="manage-covers-empty" aria-disabled="true">No covers. Add or paste an image.</li>';
      const current=selectedCovers(),one=current.length===1?current[0]:undefined;
      const sameType=current.length&&current.every(cover=>cover.picture_type===current[0].picture_type);
      const sameDescription=current.length&&current.every(cover=>cover.description===current[0].description);
      typeSelect.innerHTML=`${sameType?'':'<option value="">〈 keep 〉</option>'}${PICTURE_TYPES.map(([id,label])=>`<option value="${id}" ${sameType&&current[0].picture_type===id?'selected':''}>${esc(label)}</option>`).join('')}`;
      if(document.activeElement!==descriptionInput){descriptionInput.value=sameDescription?current[0].description:'';descriptionInput.placeholder=sameDescription?'':'〈 keep 〉';}
      const partial=current.filter(cover=>cover.original&&cover.files<cover.total);
      allBox.checked=!!current.length&&current.every(cover=>cover.all_files||!cover.original||cover.files>=cover.total);allBox.disabled=working||!partial.length;
      typeSelect.disabled=descriptionInput.disabled=working||!current.length;
      const can=(command:string)=>{
        if(working)return false;
        if(command==='add'||command==='paste')return command!=='paste'||this.host.clipboard();
        if(command==='replace'||command==='extract')return !!one&&(command==='replace'||(!!one.original&&!one.image));
        if(command==='all')return partial.some(cover=>!cover.all_files);
        if(command==='adjust')return current.length>0&&!!this.host.adjustSettings&&current.every(cover=>!!adjustSource(cover,set,targets));
        if(command==='up')return current.length>0&&list.findIndex(cover=>chosen.has(cover.id))>0;
        if(command==='down')return current.length>0&&list.map(cover=>chosen.has(cover.id)).lastIndexOf(true)<list.length-1;
        return current.length>0;
      };
      dialog.querySelectorAll<HTMLButtonElement>('[data-manage]').forEach(button=>button.disabled=!can(button.dataset.manage!));
      ok.disabled=working;
      listbox.setAttribute('aria-activedescendant',focusId&&list.some(cover=>cover.id===focusId)?`manage-${focusId}`:'');
      status.textContent=planChanged(initial,list)?'Changes are written to the files when you choose OK. Undo restores them.':'No changes yet.';
      if(focusList)listbox.focus({preventScroll:true});
    };
    const update=(change:(cover:ManagedCover)=>ManagedCover)=>{list=list.map(cover=>chosen.has(cover.id)?change(cover):cover);draw();};
    const stageImage=async<T>(image:Promise<T|null>)=>{
      working=true;draw();dialog.querySelector('.dialog-error')?.remove();
      try{return await image;}catch(error){this.host.modalError(dialog,error);return null;}finally{working=false;if(dialog.open)draw();}
    };
    const add=async(source:'file'|'clipboard')=>{
      const staged=await stageImage(source==='file'?this.startFolder(targets[0]).then(folder=>this.host.pickImage(folder)).then(path=>path?this.host.invoke<StagedImage>('stage_cover_file',{path}):null):this.host.invoke<StagedImage>('paste_cover_from_clipboard'));
      if(!staged)return;
      const cover=newManaged(list,staged,`n${counter++}`,set.files);list=[...list,cover];chosen=new Set([cover.id]);anchor=focusId=cover.id;draw(true);
    };
    // Adjust…: each selected cover is resized/converted into a staged image that replaces it on OK.
    const adjust=async(covers:ManagedCover[])=>{
      const settings=await this.host.adjustSettings?.();if(!settings||!dialog.open)return;
      const {format,quality,resize}=settings;
      const staged=await stageImage((async()=>{
        const result=new Map<string,StagedImage>();
        for(const cover of covers){
          const source=adjustSource(cover,set,targets);
          if(!source)throw new Error('This cover cannot be adjusted here.');
          result.set(cover.id,await this.host.invoke<StagedImage>('stage_adjusted_cover',{source,settings:{format,quality,resize}}));
        }
        return result;
      })());
      if(!staged)return;
      const adjusted=applyAdjusted(list,staged);list=adjusted.list;draw(true);
      if(!adjusted.changed)status.textContent='The selected covers already match these settings.';
    };
    const command=async(name:string,value?:string)=>{
      const current=selectedCovers();
      if(name==='add'||name==='paste'){await add(name==='add'?'file':'clipboard');return;}
      if(name==='replace'&&current.length===1){
        const staged=await stageImage(this.startFolder(targets[0]).then(folder=>this.host.pickImage(folder)).then(path=>path?this.host.invoke<StagedImage>('stage_cover_file',{path}):null));
        if(staged)update(cover=>replaceManaged(cover,staged));return;
      }
      if(name==='remove'){const first=list.findIndex(cover=>chosen.has(cover.id));list=list.filter(cover=>!chosen.has(cover.id));const next=list[Math.min(first,list.length-1)];chosen=new Set(next?[next.id]:[]);anchor=focusId=next?.id;draw(true);return;}
      if(name==='up'||name==='down'){list=moveManaged(list,chosen,name==='up'?-1:1);draw(true);return;}
      if(name==='adjust'&&current.length){await adjust(current);return;}
      if(name==='all'){update(cover=>cover.original&&cover.files<cover.total?{...cover,all_files:true}:cover);return;}
      if(name==='type'&&value){update(cover=>({...cover,picture_type:value}));return;}
      if(name==='description'){const description=await this.askDescription(current.length===1?current[0].description:'',current.length>1);if(description!==null)update(cover=>({...cover,description}));return;}
      if(name==='extract'&&current.length===1){const cover=current[0],source=set.covers.find(item=>coverRef(item).digest===cover.original?.digest&&item.picture_type===cover.original.picture_type&&item.description===cover.original.description);if(source)this.extract(targets[source.files[0]],{index:source.index,mime:source.mime,size:source.size,width:source.width,height:source.height,picture_type:source.picture_type});}
    };
    dialog.querySelectorAll<HTMLButtonElement>('[data-manage]').forEach(button=>button.addEventListener('click',()=>void command(button.dataset.manage!).catch(error=>this.host.modalError(dialog,error))));
    typeSelect.addEventListener('change',()=>{if(typeSelect.value)update(cover=>({...cover,picture_type:typeSelect.value}));});
    descriptionInput.addEventListener('input',()=>{list=list.map(cover=>chosen.has(cover.id)?{...cover,description:descriptionInput.value}:cover);const row=listbox.querySelectorAll<HTMLElement>('[aria-selected="true"] .manage-cover-text > span');row.forEach(span=>{if(descriptionInput.value)span.setAttribute('translate','no');else span.removeAttribute('translate');span.textContent=descriptionInput.value||'No description';});status.textContent=planChanged(initial,list)?'Changes are written to the files when you choose OK. Undo restores them.':'No changes yet.';});
    descriptionInput.addEventListener('change',()=>draw());
    allBox.addEventListener('change',()=>update(cover=>cover.original&&cover.files<cover.total?{...cover,all_files:allBox.checked}:cover));
    const choose=(id:string,event:{shiftKey:boolean;metaKey:boolean;ctrlKey:boolean})=>{
      if(event.shiftKey&&anchor){const a=list.findIndex(cover=>cover.id===anchor),b=list.findIndex(cover=>cover.id===id);chosen=new Set(list.slice(Math.min(a,b),Math.max(a,b)+1).map(cover=>cover.id));}
      else if(event.metaKey||event.ctrlKey){if(chosen.has(id))chosen.delete(id);else chosen.add(id);anchor=id;}
      else{chosen=new Set([id]);anchor=id;}
      focusId=id;draw(true);
    };
    // Pointer drag reorders covers (HTML drag and drop is reserved for files).
    let drag:{id:string;ids:Set<string>;startY:number;moved:boolean}|undefined;
    listbox.addEventListener('pointerdown',event=>{
      const row=(event.target as Element).closest<HTMLElement>('li[data-id]');if(!row||event.button!==0||working)return;
      const id=row.dataset.id!;
      if(!chosen.has(id)||event.shiftKey||event.metaKey||event.ctrlKey)choose(id,event);
      drag={id,ids:new Set(chosen),startY:event.clientY,moved:false};
      try{listbox.setPointerCapture(event.pointerId);}catch{/* Synthetic pointers cannot be captured. */}
    });
    listbox.addEventListener('pointermove',event=>{
      if(!drag||(!drag.moved&&Math.abs(event.clientY-drag.startY)<6))return;
      drag.moved=true;listbox.classList.add('dragging');
      listbox.querySelectorAll('.drop-before').forEach(el=>el.classList.remove('drop-before'));
      const row=document.elementFromPoint(event.clientX,event.clientY)?.closest<HTMLElement>('li[data-id]');row?.classList.add('drop-before');
    });
    const endDrag=(event:PointerEvent)=>{
      if(!drag)return;const current=drag;drag=undefined;listbox.classList.remove('dragging');
      if(!current.moved){if(current.ids.size>1&&!event.shiftKey&&!event.metaKey&&!event.ctrlKey)choose(current.id,event);return;}
      const row=document.elementFromPoint(event.clientX,event.clientY)?.closest<HTMLElement>('li[data-id]');
      list=dropManaged(list,current.ids,row&&listbox.contains(row)?row.dataset.id!:null);draw(true);
    };
    listbox.addEventListener('pointerup',endDrag);listbox.addEventListener('pointercancel',()=>{drag=undefined;listbox.classList.remove('dragging');draw();});
    listbox.addEventListener('contextmenu',event=>{
      const row=(event.target as Element).closest<HTMLElement>('li[data-id]');if(!row)return;event.preventDefault();
      if(!chosen.has(row.dataset.id!))choose(row.dataset.id!,{shiftKey:false,metaKey:false,ctrlKey:false});
      this.rowMenu(dialog,event.clientX,event.clientY,selectedCovers(),command,listbox);
    });
    listbox.addEventListener('keydown',event=>{
      const index=list.findIndex(cover=>cover.id===focusId),primary=this.host.mac?event.metaKey:event.ctrlKey;
      if((event.key==='ArrowUp'||event.key==='ArrowDown')&&event.altKey){event.preventDefault();void command(event.key==='ArrowUp'?'up':'down');return;}
      if(event.key==='ArrowUp'||event.key==='ArrowDown'||event.key==='Home'||event.key==='End'){
        event.preventDefault();if(!list.length)return;
        const next=list[event.key==='Home'?0:event.key==='End'?list.length-1:Math.max(0,Math.min(list.length-1,index+(event.key==='ArrowDown'?1:-1)))];
        choose(next.id,{shiftKey:event.shiftKey,metaKey:false,ctrlKey:false});return;
      }
      if(event.key==='Delete'||event.key==='Backspace'){event.preventDefault();void command('remove');return;}
      if(primary&&event.key.toLowerCase()==='v'){event.preventDefault();void command('paste').catch(error=>this.host.modalError(dialog,error));return;}
      if(primary&&event.key.toLowerCase()==='a'){event.preventDefault();chosen=new Set(list.map(cover=>cover.id));draw(true);return;}
      if(event.key===' '&&focusId){event.preventDefault();choose(focusId,{shiftKey:false,metaKey:true,ctrlKey:false});return;}
      if(event.key==='ContextMenu'||(event.shiftKey&&event.key==='F10')){event.preventDefault();const row=listbox.querySelector<HTMLElement>(`li[data-id="${focusId}"]`)?.getBoundingClientRect();if(row)this.rowMenu(dialog,row.left+20,row.bottom,selectedCovers(),command,listbox,true);}
    });
    ok.addEventListener('click',async()=>{
      if(!planChanged(initial,list)){dialog.close();return;}
      working=true;draw();dialog.querySelector('.dialog-error')?.remove();status.textContent='Writing covers…';
      try{
        const result=await this.host.mutate('apply_cover_plan',{items:planItems(list),label:'Manage covers'},'Updated covers');
        if(result&&!result.errors.length){dialog.close();return;}
        this.host.modalError(dialog,result?result.errors.map(error=>`${error.path}: ${error.error}`).join('\n'):'The covers could not be written. Review the file errors.');
      }finally{working=false;if(dialog.open)draw();}
    });
    dialog.addEventListener('close',()=>this.closeMenu());
    draw(true);
  }
  private rowMenu(dialog:HTMLDialogElement,x:number,y:number,covers:ManagedCover[],command:(name:string,value?:string)=>Promise<void>,returnFocus:HTMLElement,focus=false){
    const partial=covers.some(cover=>cover.original&&cover.files<cover.total&&!cover.all_files),one=covers.length===1?covers[0]:undefined;
    const items:MenuEntry[]=[
      {command:'all',label:'Apply to all',enabled:partial},
      {command:'type',label:'Set cover type',enabled:covers.length>0,children:PICTURE_TYPES.map(([id,label])=>({command:'type',label,value:id,enabled:true,checked:covers.length>0&&covers.every(cover=>cover.picture_type===id)}))},
      {command:'description',label:'Set cover description…',enabled:covers.length>0},
      'separator',
      {command:'replace',label:'Replace cover…',enabled:!!one},
      {command:'adjust',label:'Adjust cover…',enabled:covers.length>0&&!!this.host.adjustSettings},
      {command:'extract',label:'Extract cover…',enabled:!!one?.original&&!one.image},
      {command:'remove',label:'Remove cover',enabled:covers.length>0,shortcut:'Del'},
    ];
    this.showMenu(dialog,x,y,items,'Selected covers',(name,value)=>void command(name,value).catch(error=>this.host.modalError(dialog,error)),focus,returnFocus);
  }
}
