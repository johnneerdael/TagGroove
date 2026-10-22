import type {ApplyCover,ApplyItem,Fields,SourceFile,SourceCover,TagSourceCredentials} from './tag-sources';
import type {PresentedFileError} from './batch-errors';
import type {WebListing} from './web-sources';
import {buildComparisonPreview,comparisonExisting,comparisonFields,comparisonExample,comparisonSelectionKey,comparisonSourceValue} from './metadata-comparison';
import type {ComparisonCandidate,ComparisonChoice,ComparisonPreview,ComparisonSaveOutcome,ComparisonScope} from './metadata-comparison';
import {ComparisonSearchSession,comparisonResultKey,comparisonSources} from './metadata-comparison-sources';
import type {ComparisonSource,ComparisonSourceState,ComparisonTerms} from './metadata-comparison-sources';

type CoverView={digest:string;thumbnail:string;description:string};
type ComparisonHost={
  files:()=>SourceFile[];reason:()=>string;nextTicket:()=>number;
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  evaluate:(path:string,template:string)=>Promise<string>;
  loadExistingCovers?:(path:string)=>Promise<CoverView[]>;
  applyComparison:(items:ApplyItem[],cover:ApplyCover|null)=>Promise<ComparisonSaveOutcome>;
  formatError?:(error:unknown)=>string|undefined;
  formatFileError?:(error:PresentedFileError)=>string;
  onClose?:()=>void;
  onLanguageChange?:(listener:()=>void)=>()=>void;
};
type SnapshotView={token:string;files:{path:string;fields:Fields;duration:number;covers:{digest:string;description:string}[]}[]};
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const MULTI=' \\\\ ';
const labels:Record<string,string>={ALBUM:'Album',ALBUMARTIST:'Album artist',ARTIST:'Artist',TITLE:'Title',YEAR:'Year',TRACK:'Track number',DISCNUMBER:'Disc number',GENRE:'Genre',STYLE:'Style',BPM:'BPM',INITIALKEY:'Musical key',COMPOSER:'Composer',REMIXER:'Remixer',PUBLISHER:'Publisher',LABEL:'Label',CATALOGNUMBER:'Catalog number',BARCODE:'Barcode',ISRC:'ISRC',COMMENT:'Comment',UNSYNCEDLYRICS:'Lyrics'};
const fieldLabel=(field:string)=>Object.hasOwn(labels,field)?labels[field]:field;
const inherit=():ComparisonChoice=>({kind:'inherit'});
const equal=(a:string[]|undefined,b:string[]|undefined)=>JSON.stringify(a||[])===JSON.stringify(b||[]);
const ordinal=(value:string)=>Number.parseInt(value,10)||Number.MAX_SAFE_INTEGER;
const same=(values:string[])=>[...new Set(values.filter(value=>value.trim()))];
const emptyListing=():WebListing=>({folder:'',sources:[],settings:[],problems:[]});

/** The album changes the choice scope, not the meaning of a provider's track values. */
export class MetadataComparisonUI {
  private dialog!:HTMLDialogElement;
  private child:HTMLDialogElement|null=null;
  private childKind='';
  private detailSource:string|null=null;
  private files:SourceFile[]=[];
  private originalFiles:SourceFile[]=[];
  private scope:ComparisonScope='track';
  private terms:ComparisonTerms={artist:'',title:'',albumArtist:'',album:''};
  private termsEdited=false;
  private sources:ComparisonSource[]=[];
  private selected=new Set<string>();
  private webValues=new Map<string,string[]>();
  private choices:Record<string,ComparisonChoice>=Object.create(null);
  private artwork:ComparisonChoice=inherit();
  private session:ComparisonSearchSession;
  private revisions=new Map<string,number|undefined>();
  private snapshot:string|undefined;
  private ready=false;
  private closed=false;
  private writing=false;
  private filter='';
  private fieldFilter='all';
  private mixed=false;
  private confirmedAlbum=false;
  private frozen:ComparisonPreview|null=null;
  private pending:ApplyItem[]=[];
  private completed=new Set<string>();
  private failed:PresentedFileError[]=[];
  private retainedErrors=new Map<HTMLDialogElement,unknown>();
  private unsubscribe?:()=>void;
  private imageGeneration=0;

  constructor(private host:ComparisonHost) {
    this.session=new ComparisonSearchSession({invoke:host.invoke,evaluate:host.evaluate,nextTicket:host.nextTicket,onSourceState:state=>this.sourceChanged(state)});
  }
  private candidates(confirmed=false):ComparisonCandidate[] {
    return [...this.session.states.values()].filter(state=>state.candidate && (!confirmed||state.confirmed)).map(state=>state.candidate!);
  }
  private preview():ComparisonPreview {return buildComparisonPreview(this.files,this.candidates(true),this.choices,this.artwork);}
  private message(error:unknown):string {return this.host.formatError?.(error)||(error instanceof Error?error.message:String(error));}
  private error(error:unknown,dialog=this.dialog) {
    this.retainedErrors.set(dialog,error);const element=dialog.querySelector<HTMLElement>('[data-error]')!,formatted=this.host.formatError?.(error);
    if(formatted!==undefined)element.setAttribute('translate','no');else element.removeAttribute('translate');
    element.textContent=formatted??this.message(error);
  }
  private existing(file:SourceFile,field:string):string[] {const fields=comparisonExisting(file);return Object.hasOwn(fields,field)?fields[field]:[];}
  private localSummary(values:(string[]|undefined)[]):string {
    return values.every(value=>equal(value,values[0]))?(values[0]?.join(MULTI)||'Empty'):`Varies across ${values.length} tracks`;
  }
  private summaryMarkup(values:(string[]|undefined)[]):string {
    const raw=values.every(value=>equal(value,values[0]))&&!!values[0]?.length;
    return `<span ${raw?'translate="no"':''}>${html(this.localSummary(values))}</span>`;
  }
  private scopeAllowed() {return this.scope==='track'?this.files.length===1:!this.mixed||this.confirmedAlbum;}
  private busy() {return [...this.session.states.values()].some(state=>state.status==='searching'||state.status==='loading');}

  async open():Promise<void> {
    const reason=this.host.reason(),files=this.host.files();
    if(reason)throw new Error(reason);
    if(!files.length)throw new Error('Select a track or the files of one album to compare.');
    this.originalFiles=structuredClone(files);this.files=structuredClone(files);
    this.scope=files.length===1?'track':'album';
    this.mixed=same(files.map(file=>file.album.trim().toLocaleLowerCase().replace(/[^\p{L}\p{N}]/gu,''))).length>1;
    const albumArtist=same(files.map(file=>file.album_artist))[0]||'';
    this.terms={artist:files[0].artist.split(MULTI)[0],title:files[0].title,album:same(files.map(file=>file.album)).length===1?files[0].album:'',
      albumArtist:same(files.map(file=>file.album_artist)).length===1?(/^(va|various(?: artists)?)$/i.test(albumArtist.trim())?'Various Artists':albumArtist):''};
    const dialog=this.dialog=document.createElement('dialog');
    dialog.className='tag-source-dialog metadata-comparison-dialog';dialog.setAttribute('aria-label','Compare metadata');
    dialog.innerHTML=`<header>Compare metadata</header><div class="dialog-content">
      <div class="mc-heading"><div><strong data-context-title translate="no">${html(this.terms.album||files[0].name)}</strong><p class="mc-muted">${files.length} selected file${files.length===1?'':'s'}</p></div><div class="mc-scope" role="group" aria-label="Search scope"><button type="button" data-scope="track" aria-pressed="${this.scope==='track'}" ${files.length!==1?'disabled':''}>Single track</button><button type="button" data-scope="album" aria-pressed="${this.scope==='album'}">Album</button></div></div>
      <label class="mc-group-warning" data-group-warning ${!this.mixed?'hidden':''}><input type="checkbox" data-confirm-album> These files have different album titles. Treat the selected files as one album.</label>
      <div class="mc-query"><label class="field"><span data-artist-label></span><input data-query-artist aria-label="Search artist" spellcheck="false"></label><label class="field"><span data-title-label></span><input data-query-title aria-label="Search title" spellcheck="false"></label><button type="button" data-search disabled>Search selected sources</button><button type="button" data-stop hidden>Stop searching</button></div>
      <div class="mc-source-bar"><span data-status role="status" aria-live="polite">Reading existing metadata…</span><button type="button" data-sources>Sources and matches</button></div>
      <div class="mc-field-tools"><input type="search" data-find aria-label="Find a metadata field" placeholder="Find a field…"><select data-field-filter aria-label="Filter metadata fields"><option value="all">All fields</option><option value="different">Different values</option><option value="changed">Changed fields</option><option value="incomplete">Incomplete source coverage</option></select><span class="mc-muted" data-scope-note></span></div>
      <div class="mc-field-area"><table class="ts-table mc-fields" aria-label="Choose metadata per field"><thead><tr><th>Field</th><th>Existing values</th><th>Use</th><th>Result</th></tr></thead><tbody data-fields></tbody></table></div>
      <p class="mc-hint">Inherit keeps existing values. None clears the field. A provider supplies its own matched values for each file.</p><p class="dialog-error" data-error role="alert"></p></div>
      <div class="dialog-footer"><span class="mc-muted" data-change-count></span><button type="button" data-reset>Reset to Inherit</button><button type="button" data-close>Cancel</button><button type="button" class="primary" data-review disabled>Review and save</button></div>`;
    document.body.append(dialog);
    dialog.addEventListener('click',event=>this.click(event));
    dialog.addEventListener('input',event=>this.input(event));
    dialog.addEventListener('change',event=>this.input(event));
    dialog.addEventListener('cancel',event=>{if(this.writing)event.preventDefault();});
    dialog.addEventListener('close',()=>this.dispose());
    this.unsubscribe=this.host.onLanguageChange?.(()=>{if(this.closed)return;this.renderFields();for(const [dialog,error]of this.retainedErrors)if(dialog.isConnected)this.error(error,dialog);if(this.childKind==='sources'){if(this.detailSource)this.sourceDetails(this.detailSource);else this.sourcesBody();}if(this.childKind==='review'&&!this.writing)this.reviewBody();});
    this.renderTerms();this.renderFields();dialog.showModal();
    const ticket=this.host.nextTicket();
    try {
      const [snapshot,credentials,listing,local]=await Promise.all([
        this.host.invoke<string|SnapshotView>('tag_source_comparison_snapshot',{paths:files.map(file=>file.path),ticket}),
        this.host.invoke<TagSourceCredentials>('load_tag_source_settings').catch(()=>null),
        this.host.invoke<WebListing>('web_sources_list').catch(()=>emptyListing()),
        this.host.invoke<{directory:string|null}>('local_freedb_index_status').catch(()=>undefined),
      ]);
      if(this.closed)return;
      this.snapshot=typeof snapshot==='string'?snapshot:snapshot?.token;
      if(!this.snapshot)throw new Error('The current file metadata could not be captured for review.');
      if(typeof snapshot!=='string')for(const view of snapshot.files){
        const file=this.files.find(file=>file.path===view.path);if(!file)continue;
        file.fields=structuredClone(view.fields);file.length=view.duration;
        for(const [property,field]of [['title','TITLE'],['artist','ARTIST'],['album','ALBUM'],['album_artist','ALBUMARTIST'],['track','TRACK'],['disc','DISCNUMBER']] as const)file[property]=view.fields[field]?.join(MULTI)||'';
        file.covers=view.covers.map(cover=>({...cover,thumbnail:''}));
      }
      this.files.sort((a,b)=>ordinal(a.disc)-ordinal(b.disc)||ordinal(a.track)-ordinal(b.track));
      this.mixed=same(this.files.map(file=>file.album.trim().toLocaleLowerCase().replace(/[^\p{L}\p{N}]/gu,''))).length>1;
      if(!this.termsEdited){
        const artist=same(this.files.map(file=>file.album_artist));
        this.terms.artist=this.files[0].artist.split(MULTI)[0];this.terms.title=this.files[0].title;
        this.terms.album=same(this.files.map(file=>file.album)).length===1?this.files[0].album:'';
        this.terms.albumArtist=artist.length===1?(/^(va|various(?: artists)?)$/i.test(artist[0].trim())?'Various Artists':artist[0]):'';
        this.renderTerms();
      }
      this.dialog.querySelector<HTMLElement>('[data-group-warning]')!.hidden=!this.mixed;
      this.sources=comparisonSources(credentials||null,listing||emptyListing(),local?{localFreedbAvailable:!!local.directory}:{});
      await Promise.all(this.sources.filter(source=>source.web).map(async source=>{
        const values=await Promise.all(source.web!.search_by.map(field=>field.format?this.host.evaluate(this.files[0].path,field.format).catch(()=>''):Promise.resolve('')));
        this.webValues.set(source.id,values);
      }));
      if(this.closed)return;
      // Installed scripts expose their declared inputs; enable them explicitly in Sources.
      this.selected=new Set(this.sources.filter(source=>source.selected&&!source.web).map(source=>source.id));
      this.ready=true;this.renderFields();this.sync();
      if(this.host.loadExistingCovers&&this.files[0])void this.host.loadExistingCovers(this.files[0].path).then(covers=>{
        if(this.closed)return;
        const file=this.files[0];
        if(file.covers)file.covers=file.covers.map(cover=>({...cover,thumbnail:covers.find(value=>value.digest===cover.digest)?.thumbnail||cover.thumbnail}));
        else file.covers=covers;
        this.renderFields();
      }).catch(()=>undefined);
    }catch(error){if(!this.closed){this.error(error);this.sync();}}
  }

  private renderTerms() {
    const first=this.files[0];
    this.dialog.querySelector<HTMLElement>('[data-context-title]')!.textContent=this.scope==='track'?[first.artist,first.title].filter(Boolean).join(' · ')||first.name:this.terms.album||first.name;
    this.dialog.querySelector<HTMLElement>('[data-artist-label]')!.textContent=this.scope==='album'?'ALBUM ARTIST':'ARTIST';
    this.dialog.querySelector<HTMLElement>('[data-title-label]')!.textContent=this.scope==='album'?'ALBUM TITLE':'TRACK TITLE';
    this.dialog.querySelector<HTMLInputElement>('[data-query-artist]')!.value=this.scope==='album'?this.terms.albumArtist:this.terms.artist;
    this.dialog.querySelector<HTMLInputElement>('[data-query-title]')!.value=this.scope==='album'?this.terms.album:this.terms.title;
    this.dialog.querySelectorAll<HTMLButtonElement>('[data-scope]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.scope===this.scope)));
  }
  private renderFields() {
    if(this.closed)return;
    const candidates=this.candidates(),preview=this.preview();
    const changed=new Set(preview.items.flatMap(item=>Object.keys(item.updates)));
    const names=comparisonFields(this.files,candidates).filter(name=>{
      if(!(name+' '+(fieldLabel(name))).toLocaleLowerCase().includes(this.filter.toLocaleLowerCase()))return false;
      if(this.fieldFilter==='changed')return changed.has(name);
      if(this.fieldFilter==='incomplete')return candidates.some(candidate=>this.files.some(file=>comparisonSourceValue(candidate,file.path,name).values===null));
      if(this.fieldFilter==='different')return candidates.some(candidate=>this.files.some(file=>{const value=comparisonSourceValue(candidate,file.path,name).values;return value!==null&&!equal(value,this.existing(file,name));}));
      return true;
    });
    this.dialog.querySelector('[data-fields]')!.innerHTML=names.map(name=>{
      const choice=this.choices[name]||inherit(),old=this.files.map(file=>this.existing(file,name));
      const next=this.files.map(file=>{
        if(choice.kind==='none')return [];
        if(choice.kind==='source'){const candidate=candidates.find(value=>value.sourceId===choice.sourceId&&value.revision===choice.revision);return candidate?comparisonSourceValue(candidate,file.path,name).values||this.existing(file,name):this.existing(file,name);}
        return this.existing(file,name);
      });
      const count=candidates.filter(candidate=>this.files.some(file=>comparisonSourceValue(candidate,file.path,name).values!==null)).length;
      const use=choice.kind==='source'?this.sources.find(source=>source.id===choice.sourceId)?.label||'Review source':choice.kind==='none'?'None':'Inherit';
      const missing=preview.missing.filter(value=>value.field===name).length;
      return `<tr data-field="${html(name)}"><th scope="row"><span ${Object.hasOwn(labels,name)?'':'translate="no"'}>${html(fieldLabel(name))}</span><small translate="no">${html(name)}</small>${count?`<small>${count} source${count===1?'':'s'}</small>`:''}</th><td><button class="mc-value" type="button" data-inspect="${html(name)}">${this.summaryMarkup(old)}</button></td><td><button type="button" class="mc-choice ${choice.kind!=='inherit'?'mc-selected':''}" data-pick-field="${html(name)}" ${!this.ready?'disabled':''} aria-label="Choose source for ${html(fieldLabel(name))}"><span ${choice.kind==='source'?'translate="no"':''}>${html(use)}</span><span aria-hidden="true">▾</span></button></td><td><button type="button" class="mc-value ${choice.kind==='none'?'mc-removal':''}" data-inspect="${html(name)}">${this.summaryMarkup(next)}</button><small class="${choice.kind==='none'?'mc-removal':''}">${choice.kind==='none'?'Remove field':choice.kind==='inherit'?'Unchanged':missing?`${missing} missing · existing kept`:this.scope==='album'?'Matched values for each track':'Matched value'}</small></td></tr>`;
    }).join('')+(!this.filter?this.artworkRow():'');
    this.sync();
  }
  private artworkRow():string {
    const choice=this.artwork,candidate=choice.kind==='source'?this.candidates().find(value=>value.sourceId===choice.sourceId):undefined;
    const existing=this.files[0]?.covers||[],cover=candidate?.release.covers[choice.kind==='source'?choice.coverIndex||0:0];
    const use=choice.kind==='source'?this.sources.find(source=>source.id===choice.sourceId)?.label||'Review source':choice.kind==='none'?'None':'Inherit';
    return `<tr class="mc-artwork-row"><th scope="row">Artwork<small>Embedded images</small></th><td>${existing[0]?.thumbnail?`<img class="mc-cover-thumb" src="${html(existing[0].thumbnail)}" alt="Existing artwork">`:''}<small>${existing.length?`${existing.length} existing image${existing.length===1?'':'s'}`:'No existing artwork'}</small></td><td><button type="button" class="mc-choice ${choice.kind!=='inherit'?'mc-selected':''}" data-pick-artwork ${!this.ready?'disabled':''}><span ${choice.kind==='source'?'translate="no"':''}>${html(use)}</span><span aria-hidden="true">▾</span></button></td><td>${choice.kind==='none'?'<span class="mc-removal">Remove existing artwork</span>':cover?html(cover.kind||'Selected artwork'):'Existing artwork unchanged'}</td></tr>`;
  }
  private sync() {
    if(this.closed)return;
    const busy=this.busy(),preview=this.preview();
    this.dialog.querySelector<HTMLButtonElement>('[data-search]')!.disabled=!this.ready||this.writing||!this.scopeAllowed()||!this.selected.size;
    this.dialog.querySelector<HTMLButtonElement>('[data-stop]')!.hidden=!busy;
    this.dialog.querySelector<HTMLButtonElement>('[data-review]')!.disabled=!this.ready||this.writing||busy||!this.scopeAllowed()||!preview.items.length||!!preview.blocked.length;
    this.dialog.querySelector<HTMLElement>('[data-scope-note]')!.textContent=this.scope==='album'?`Each choice applies to all ${this.files.length} tracks`:'Choices apply to this track';
    const ready=[...this.session.states.values()].filter(state=>state.status==='ready').length;
    this.dialog.querySelector<HTMLElement>('[data-status]')!.textContent=!this.ready?'Reading existing metadata…':busy?'Searching selected sources…':this.session.states.size?`${ready} sources ready · review sources that need a match`:`${this.selected.size} sources selected · search starts only when requested`;
    this.dialog.querySelector<HTMLElement>('[data-change-count]')!.textContent=preview.blocked.length?'Review source matches before saving.':preview.items.length?`${preview.items.length} file${preview.items.length===1?'':'s'} would change`:'No changes selected';
  }
  private sourceChanged(state:ComparisonSourceState) {
    if(this.closed||!this.dialog)return;
    const revision=state.candidate?.revision,previous=this.revisions.get(state.source.id);
    if(revision!==previous){
      for(const [field,choice]of Object.entries(this.choices))if(choice.kind==='source'&&choice.sourceId===state.source.id&&choice.revision!==revision)delete this.choices[field];
      if(this.artwork.kind==='source'&&this.artwork.sourceId===state.source.id&&this.artwork.revision!==revision)this.artwork=inherit();
      this.revisions.set(state.source.id,revision);this.renderFields();
    }else this.sync();
    if(this.childKind==='sources'&&this.child){if(this.detailSource===state.source.id)this.sourceDetails(state.source.id);else if(!this.detailSource)this.sourcesBody();}
  }
  private input(event:Event) {
    const input=event.target as HTMLInputElement|HTMLSelectElement;
    if(input.matches('[data-find]')){this.filter=input.value;this.renderFields();}
    if(input.matches('[data-field-filter]')){this.fieldFilter=input.value;this.renderFields();}
    if(input.matches('[data-query-artist]')){this.termsEdited=true;if(this.scope==='album')this.terms.albumArtist=input.value;else this.terms.artist=input.value;}
    if(input.matches('[data-query-title]')){this.termsEdited=true;if(this.scope==='album')this.terms.album=input.value;else this.terms.title=input.value;}
    if(input.matches('[data-confirm-album]')){this.confirmedAlbum=(input as HTMLInputElement).checked;this.sync();}
  }
  private click(event:Event) {
    const button=(event.target as Element).closest<HTMLButtonElement>('button');if(!button)return;
    if(button.hasAttribute('data-close'))this.dialog.close();
    if(button.dataset.scope){this.scope=button.dataset.scope as ComparisonScope;this.session.cancel();this.choices=Object.create(null);this.artwork=inherit();this.renderTerms();this.renderFields();}
    if(button.hasAttribute('data-search'))void this.search();
    if(button.hasAttribute('data-stop')){this.session.cancel();this.sync();}
    if(button.hasAttribute('data-sources'))this.openSources();
    if(button.hasAttribute('data-pick-field'))this.openPicker(button.dataset.pickField!);
    if(button.hasAttribute('data-pick-artwork'))this.openPicker(null);
    if(button.hasAttribute('data-inspect'))this.inspect(button.dataset.inspect!);
    if(button.hasAttribute('data-reset')){this.choices=Object.create(null);this.artwork=inherit();this.renderFields();}
    if(button.hasAttribute('data-review'))this.review();
  }
  private async search() {
    if(!this.ready||!this.scopeAllowed()||this.writing)return;
    if(!this.selectionValid()){this.error('The local file selection or metadata changed. Close comparison and start again.');return;}
    for(const [field,choice]of Object.entries(this.choices))if(choice.kind==='source')delete this.choices[field];
    if(this.artwork.kind==='source')this.artwork=inherit();
    this.error('');
    await this.session.search(this.sources.filter(source=>this.selected.has(source.id)),this.scope,this.files,this.terms);
    this.renderFields();
  }

  private openChild(kind:string,title:string,subtitle:string,body:string):HTMLDialogElement {
    this.closeChild();
    const child=this.child=document.createElement('dialog');this.childKind=kind;
    child.className=`tag-source-dialog metadata-comparison-${kind}`;child.setAttribute('aria-label',title);
    child.innerHTML=`<header>${html(title)}<button type="button" data-close aria-label="Close details">×</button></header><div class="dialog-content"><p class="mc-muted mc-child-subtitle">${html(subtitle)}</p><div data-child-body>${body}</div><p class="dialog-error" data-error role="alert"></p></div>`;
    document.body.append(child);
    child.addEventListener('click',event=>{const button=(event.target as Element).closest<HTMLButtonElement>('button');if(button?.hasAttribute('data-close')){if(this.writing)return;if(this.frozen&&this.completed.size)this.dialog.close();else this.closeChild();}});
    child.addEventListener('cancel',event=>{if(this.writing)event.preventDefault();else if(this.frozen&&this.completed.size){event.preventDefault();this.dialog.close();}});
    child.addEventListener('close',()=>{child.remove();if(this.child===child){this.child=null;this.childKind='';this.detailSource=null;}});
    child.showModal();return child;
  }
  private closeChild(){this.imageGeneration++;if(this.child){const old=this.child;this.child=null;this.childKind='';this.detailSource=null;this.retainedErrors.delete(old);old.close();old.remove();}}
  private valueField(value:string,label:string,attributes=''):string {
    return value.includes('\n')||value.length>140?`<textarea class="mc-returned" rows="${Math.min(6,Math.max(2,value.split('\n').length))}" readonly translate="no" aria-label="${html(label)}" ${attributes} placeholder="No value supplied">${html(value)}</textarea>`:
      `<input class="mc-returned" type="text" readonly translate="no" aria-label="${html(label)}" value="${html(value)}" ${attributes} placeholder="No value supplied">`;
  }
  private openPicker(field:string|null) {
    const artwork=field===null,label=artwork?'Artwork':fieldLabel(field!);
    const current=artwork?this.files[0]?.covers?.map(cover=>cover.description||'Embedded image').join(MULTI)||'':this.existing(this.files[0],field!)?.join(MULTI)||'';
    const choice=artwork?this.artwork:this.choices[field!]||inherit();
    const options=this.candidates();
    const child=this.openChild('picker',`Choose ${label.toLowerCase()}`,this.scope==='album'?`Track 1 shown as an example · choice applies to all ${this.files.length} tracks`:'Choose the value for this track',
      `<label class="mc-existing-field"><span>Existing ${html(label.toLowerCase())}${this.scope==='album'?' · track 1':''}</span>${this.valueField(current,`Existing ${label} value`,'data-existing-value')}</label>
      <div class="mc-special-choices">${[['inherit','Inherit','Keep each file’s existing value'],['none','None',artwork?'Remove existing artwork':'Clear this field on every selected file']].map(([kind,name,description])=>`<button type="button" data-source-choice="${kind}" aria-pressed="${choice.kind===kind}"><span class="mc-radio" aria-hidden="true"></span><span>${name}<small>${description}</small></span></button>`).join('')}</div>
      <div class="mc-provider-heading"><span>Provider</span><span>Returned ${html(label.toLowerCase())}${this.scope==='album'?' · track 1 example':''}</span></div>
      <div>${options.map(candidate=>{
        const state=this.session.states.get(candidate.sourceId)!,cover=candidate.release.covers[0],values=artwork?(cover?cover.kind||'Artwork':''):comparisonExample(this.files,candidate,field!)?.join(MULTI)||'';
        const coverage=artwork?(cover?this.files.length:0):this.files.filter(file=>comparisonSourceValue(candidate,file.path,field!).values!==null).length;
        return `<div class="mc-provider-choice"><div><button type="button" data-source-choice="${html(candidate.sourceId)}" data-revision="${candidate.revision}" ${!state.confirmed||!coverage?'disabled':''} aria-pressed="${choice.kind==='source'&&choice.sourceId===candidate.sourceId}"><span class="mc-radio" aria-hidden="true"></span><span translate="no">${html(state.source.label)}</span></button><small>${coverage}/${this.files.length} tracks</small>${!state.confirmed?`<button type="button" class="mc-text-action" data-review-source="${html(candidate.sourceId)}">Review match</button>`:''}</div><div>${artwork&&cover?`<img class="mc-provider-cover" data-cover-image="${html(candidate.sourceId)}" alt="Provider artwork">`:''}${this.valueField(values,`${label} value from ${state.source.label}`,`data-provider-value="${html(candidate.sourceId)}"`)}</div></div>`;
      }).join('')||'<p class="mc-muted">No provider has supplied this field yet.</p>'}</div>`);
    child.addEventListener('click',event=>{
      const button=(event.target as Element).closest<HTMLButtonElement>('button');if(!button||this.writing)return;
      if(button.dataset.reviewSource){this.openSources(button.dataset.reviewSource);return;}
      const sourceId=button.dataset.sourceChoice;if(!sourceId)return;
      let selected:ComparisonChoice;
      if(sourceId==='inherit'||sourceId==='none')selected={kind:sourceId};
      else {
        const state=this.session.states.get(sourceId),candidate=state?.candidate;
        if(!candidate||!state.confirmed||candidate.revision!==Number(button.dataset.revision)){this.error('This source candidate changed. Reopen the selector and review its value.',child);return;}
        selected={kind:'source',sourceId,revision:candidate.revision};
      }
      if(artwork)this.artwork=selected;else this.choices[field!]=selected;
      this.closeChild();this.renderFields();
    });
    if(artwork)void this.loadProviderImages(child,options);
  }
  private async loadProviderImages(child:HTMLDialogElement,candidates:ComparisonCandidate[]) {
    const generation=++this.imageGeneration,ticket=this.host.nextTicket();let next=0;
    await Promise.all(Array.from({length:Math.min(3,candidates.length)},async()=>{
      while(next<candidates.length){const candidate=candidates[next++],cover=candidate.release.covers[0];if(!cover)continue;
        try{const data=await this.host.invoke<string>('tag_source_image',{url:cover.thumb||cover.url,ticket});
          if(this.closed||this.child!==child||generation!==this.imageGeneration)return;
          const image=[...child.querySelectorAll<HTMLImageElement>('[data-cover-image]')].find(image=>image.dataset.coverImage===candidate.sourceId);if(image)image.src=data;
        }catch{/* The metadata remains usable when an image preview is unavailable. */}
      }
    }));
  }

  private openSources(sourceId?:string) {
    this.openChild('sources','Sources and matches','Choose which sources to query, then check the matched release and its tracks.','');
    if(sourceId)this.sourceDetails(sourceId);else this.sourcesBody();
  }
  private sourcesBody() {
    if(!this.child)return;this.detailSource=null;
    this.child.querySelector('[data-child-body]')!.innerHTML=`<div class="mc-sources-list">${this.sources.map(source=>{
      const state=this.session.states.get(source.id),text=source.setupReason||this.stateLabel(state);
      return `<div class="mc-source-item"><label><input type="checkbox" data-enable-source="${html(source.id)}" ${this.selected.has(source.id)?'checked':''} ${source.setupReason?'disabled':''}><span translate="no">${html(source.label)}</span></label><small>${html(text)}</small><button type="button" data-source-details="${html(source.id)}">${state?.results.length?'Matches':'Details'}</button>${this.editionSelect(source.id,state)}</div>`;
    }).join('')}</div><div class="mc-child-actions"><button type="button" data-search-sources class="primary">Search selected sources</button></div>`;
    this.child.querySelectorAll<HTMLSelectElement>('select[data-edition]').forEach(select=>{select.selectedIndex=Number(select.dataset.chosen);});
    this.bindSources();
  }
  /** Several editions (or releases) came back: choose one here, without opening the source's details. */
  private editionSelect(sourceId:string,state:ComparisonSourceState|undefined):string {
    if(!state||state.results.length<2)return '';
    const chosen=Math.max(0,state.results.findIndex(result=>comparisonResultKey(result)===state.resultKey));
    const busy=state.status==='searching'||state.status==='loading';
    return `<label class="mc-edition"><span>Edition</span><select data-edition="${html(sourceId)}" data-chosen="${chosen}" ${busy?'disabled':''}>${state.results.map((result,index)=>{
      const label=[result.artist,result.title,result.release_title,result.year,result.country,result.label,result.catalog_number,result.format].filter(Boolean).join(' · ');
      return `<option value="${index}" ${index===chosen?'selected':''} translate="no">${html(label)}</option>`;
    }).join('')}</select></label>`;
  }
  private stateLabel(state:ComparisonSourceState|undefined):string {
    if(!state)return 'Not searched';
    if(state.status==='failed')return this.message(state.error);
    if(state.status==='needs-input')return state.input?.reason||'Additional search information needed';
    if(state.status==='needs-match')return 'Choose the matching result and check its tracks';
    return ({'needs-setup':'Setup needed',searching:'Searching…',loading:'Loading metadata…',ready:'Ready',empty:'No results',cancelled:'Search stopped'} as Record<string,string>)[state.status]||state.status;
  }
  private sourceDetails(sourceId:string) {
    if(!this.child)return;this.detailSource=sourceId;
    const source=this.sources.find(source=>source.id===sourceId);if(!source)return;
    const state=this.session.states.get(sourceId),candidate=state?.candidate;
    const inputs=state?.input?.inputs||(source.web?source.web.search_by.map((field,index)=>({key:String(index),label:field.label,value:this.webValues.get(source.id)?.[index]||''})):[]);
    this.child.querySelector('[data-child-body]')!.innerHTML=`<button type="button" class="mc-text-action" data-all-sources>← All sources</button><h3 translate="no">${html(source.label)}</h3><p class="mc-hint">${html(source.setupReason||this.stateLabel(state))}</p>
      ${inputs.length?`<div class="mc-source-inputs">${inputs.map(input=>`<label class="field"><span translate="no">${html(input.label)}</span><input data-source-input="${html(input.key)}" value="${html(input.value)}"></label>`).join('')}</div>`:''}
      ${source.provider==='acoustid'?'<p class="mc-hint">AcoustID reads each selected file to calculate an audio fingerprint. Enable it when you want recording identification.</p>':''}
      ${source.setupReason?'':`<button type="button" data-retry-source="${html(sourceId)}">${state?.status==='failed'?'Retry this source':'Search this source'}</button>`}
      ${state?.results.length?`<ol class="mc-candidate-list">${state.results.map((result,index)=>`<li><span translate="no">${html([result.artist,result.title,result.release_title,result.year,result.catalog_number].filter(Boolean).join(' · '))}</span><button type="button" data-load-result="${index}" data-result-id="${html(result.id)}" data-result-track="${html(result.track_id||'')}">Use this result</button></li>`).join('')}</ol>`:''}
      ${state?.canMore?'<button type="button" data-more-results>More results</button>':''}
      ${candidate&&candidate.release.tracks.length?`<h3>Tracks and files</h3><table class="ts-table mc-mapping"><thead><tr><th>Local file</th><th>Provider track</th><th></th></tr></thead><tbody>${this.files.map(file=>{const index=candidate.tracks[file.path],track=index==null?undefined:candidate.release.tracks[index];return `<tr><td translate="no">${html(file.name)}<small>${html(file.artist)}</small></td><td translate="no">${track?html([track.position,track.artist,track.title].filter(Boolean).join(' · ')):'Choose a track'}</td><td><button type="button" data-map-file="${html(file.path)}">Match…</button></td></tr>`;}).join('')}</tbody></table>`:''}`;
    this.bindSources();
  }
  private bindSources() {
    if(!this.child||this.child.dataset.bound)return;
    this.child.dataset.bound='true';
    this.child.addEventListener('change',event=>{
      const input=event.target as HTMLInputElement;
      if(input.dataset.enableSource){if(input.checked)this.selected.add(input.dataset.enableSource);else this.selected.delete(input.dataset.enableSource);this.sync();}
      const edition=(event.target as Element).closest<HTMLSelectElement>('select[data-edition]');
      if(edition){const id=edition.dataset.edition!,row=this.session.states.get(id)?.results[Number(edition.value)];if(row)void this.session.load(id,row);}
    });
    this.child.addEventListener('click',event=>{
      const button=(event.target as Element).closest<HTMLButtonElement>('button');if(!button)return;
      if(button.dataset.sourceDetails)this.sourceDetails(button.dataset.sourceDetails);
      if(button.hasAttribute('data-all-sources'))this.sourcesBody();
      if(button.hasAttribute('data-search-sources')){this.closeChild();void this.search();}
      if(button.dataset.retrySource){
        const id=button.dataset.retrySource,source=this.sources.find(source=>source.id===id)!;
        const terms=structuredClone(this.terms),inputs=[...this.child!.querySelectorAll<HTMLInputElement>('[data-source-input]')];
        if(source.web){const values=inputs.map(input=>input.value);this.webValues.set(id,values);this.terms.webValues={...(this.terms.webValues||{}),[id]:values};terms.webValues=structuredClone(this.terms.webValues);}
        else for(const input of inputs)if(['artist','title','albumArtist','album'].includes(input.dataset.sourceInput!))terms[input.dataset.sourceInput as 'artist'|'title'|'albumArtist'|'album']=input.value;
        const disc=inputs.find(input=>input.dataset.sourceInput==='disc')?.value;
        if(disc)void this.session.load(id,{id:disc,title:disc,artist:'',year:'',country:'',label:'',catalog_number:'',format:'',track_count:null,score:null,barcode:'',thumb:null,url:''});
        else if(this.scopeAllowed()&&this.selectionValid())void this.session.searchOne(source,this.scope,this.files,terms);
        else this.error('The local file selection or metadata changed. Close comparison and start again.',this.child!);
      }
      if(button.hasAttribute('data-load-result')&&this.detailSource){
        const row=this.session.states.get(this.detailSource)?.results[Number(button.dataset.loadResult)];
        if(row&&row.id===button.dataset.resultId&&(row.track_id||'')===button.dataset.resultTrack)void this.session.load(this.detailSource,row);
      }
      if(button.hasAttribute('data-more-results')&&this.detailSource)void this.session.more(this.detailSource);
      if(button.dataset.mapFile&&this.detailSource)this.trackPicker(this.detailSource,button.dataset.mapFile);
    });
  }
  private trackPicker(sourceId:string,path:string) {
    const candidate=this.session.states.get(sourceId)?.candidate;if(!candidate)return;
    const child=this.openChild('track-map','Match provider track',this.files.find(file=>file.path===path)?.name||'',`<ol class="mc-candidate-list">${candidate.release.tracks.map((track,index)=>`<li><span translate="no">${html([track.position,track.artist,track.title].filter(Boolean).join(' · '))}</span><button type="button" data-match-index="${index}">Use this track</button></li>`).join('')}</ol>`);
    child.addEventListener('click',event=>{const button=(event.target as Element).closest<HTMLButtonElement>('[data-match-index]');if(!button)return;
      if(this.session.states.get(sourceId)?.candidate?.revision!==candidate.revision){this.error('This source candidate changed. Review its tracks again.',child);return;}
      this.session.match(sourceId,path,Number(button.dataset.matchIndex));this.openSources(sourceId);
    });
  }
  private inspect(field:string) {
    const choice=this.choices[field]||inherit(),candidate=choice.kind==='source'?this.candidates().find(value=>value.sourceId===choice.sourceId):undefined;
    this.openChild('inspect',`${fieldLabel(field)} · track preview`,this.scope==='album'?'Each track receives its own matched value.':'Existing and resulting values for this file.',`<table class="ts-table mc-preview"><thead><tr><th>Track</th><th>Existing</th><th>Result</th></tr></thead><tbody>${this.files.map(file=>{
      const old=this.existing(file,field)||[],next=choice.kind==='none'?[]:candidate?comparisonSourceValue(candidate,file.path,field).values||old:old;
      return `<tr><th translate="no">${html(file.name)}</th><td translate="no">${html(old.join(MULTI)||'Empty')}</td><td translate="no">${html(next.join(MULTI)||'Empty')}</td></tr>`;
    }).join('')}</tbody></table>`);
  }

  private selectionValid(remaining?:ApplyItem[]):boolean {
    const current=this.host.files();
    if(current.length!==this.originalFiles.length||current.some(file=>!this.originalFiles.some(old=>old.path===file.path)))return false;
    const targets=new Set(remaining?.map(item=>item.path)||this.originalFiles.map(file=>file.path));
    return this.originalFiles.filter(file=>targets.has(file.path)).every(original=>{
      const now=current.find(file=>file.path===original.path)!,native=this.files.find(file=>file.path===original.path)!;
      const key=comparisonSelectionKey([now]);return key===comparisonSelectionKey([original])||key===comparisonSelectionKey([native]);
    });
  }
  private review() {
    const preview=this.preview();if(!preview.items.length||preview.blocked.length||this.busy())return;
    if(!this.selectionValid()){this.error('The local file selection or metadata changed. Close comparison and start again.');return;}
    this.session.cancel();this.frozen=structuredClone(preview);this.pending=structuredClone(preview.items);this.completed.clear();this.failed=[];
    this.reviewBody();
  }
  private reviewBody() {
    const child=this.openChild('review','Review changes',`${this.pending.length} file${this.pending.length===1?'':'s'} to save${this.completed.size?` · ${this.completed.size} already saved`:''}`,
      `${this.completed.size?`<section data-completed><h3>Saved files</h3><ul>${[...this.completed].map(path=>`<li translate="no">${html(path)}</li>`).join('')}</ul></section>`:''}<table class="ts-table mc-preview"><thead><tr><th>File</th><th>Changes</th></tr></thead><tbody>${this.pending.map(item=>`<tr><th translate="no">${html(item.path)}</th><td>${Object.entries(item.updates).map(([field,values])=>`<div><strong ${Object.hasOwn(labels,field)?'':'translate="no"'}>${html(fieldLabel(field))}</strong>: ${values.length?`<span translate="no">${html(values.join(MULTI))}</span>`:'<span class="mc-removal">Remove field</span>'}</div>`).join('')}${item.remove_covers?'<div class="mc-removal">Remove existing artwork</div>':this.frozen?.cover?'<div>Import selected artwork</div>':''}${this.failed.filter(error=>error.path===item.path).map(error=>`<p class="mc-removal" ${this.host.formatFileError?'translate="no"':''}>${html(this.host.formatFileError?.(error)??error.error)}</p>`).join('')}</td></tr>`).join('')}</tbody></table>
      ${this.frozen?.missing.length?`<p class="mc-hint">${this.frozen.missing.length} missing provider values keep their existing values.</p>`:''}
      <p class="mc-hint">Fields set to Inherit are unchanged. Only the selected files are written.</p><div class="mc-child-actions"><button type="button" data-close>${this.completed.size?'Close comparison':'Back to fields'}</button><button type="button" class="primary" data-save>${this.completed.size?'Retry failed files':`Save ${this.pending.length} file${this.pending.length===1?'':'s'}`}</button></div>`);
    child.addEventListener('click',event=>{if((event.target as Element).closest('[data-save]'))void this.save();});
  }
  private async save() {
    if(this.writing||!this.pending.length||!this.child)return;
    if(!this.selectionValid(this.pending)){this.error('The local file selection or metadata changed. Close comparison and start again.',this.child);return;}
    const reason=this.host.reason();if(reason){this.error(reason,this.child);return;}
    this.writing=true;
    this.child.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=true);
    this.dialog.querySelector<HTMLButtonElement>('[data-close]')!.disabled=true;
    try{
      const items=this.pending.map(item=>({...structuredClone(item),comparison_snapshot:this.snapshot}));
      const outcome=await this.host.applyComparison(items,this.frozen?.cover||null);
      outcome.succeeded.forEach(path=>this.completed.add(path));
      this.failed=[...outcome.failed];
      for(const item of this.pending)if(!this.completed.has(item.path)&&!this.failed.some(error=>error.path===item.path))this.failed.push({path:item.path,error:'No save result was returned for this file.'});
      this.pending=this.pending.filter(item=>!this.completed.has(item.path));
      if(!this.pending.length){this.dialog.close();return;}
      this.writing=false;this.reviewBody();
    }catch(error){if(this.child)this.error(error,this.child);}
    finally{this.writing=false;if(!this.closed){this.dialog.querySelector<HTMLButtonElement>('[data-close]')!.disabled=false;this.child?.querySelectorAll<HTMLButtonElement>('button').forEach(button=>button.disabled=false);}}
  }
  private dispose() {
    if(this.closed)return;this.closed=true;this.session.cancel();this.closeChild();this.unsubscribe?.();
    void this.host.invoke('tag_source_comparison_cancel',{ticket:this.host.nextTicket()}).catch(()=>undefined);
    this.dialog.remove();this.host.onClose?.();
  }
}
