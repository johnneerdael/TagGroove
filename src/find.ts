export type FindStrategy='contains'|'word'|'content'|'begins'|'ends'|'regex';
export type FindItem={path:string;filename:string;fields:Record<string,string[]>};
export type FindResult={path:string;filename:string;field:string;value:string};
export const findStrategies:[FindStrategy,string][]=[['contains','Contains'],['word','Matches word'],['content','Matches content'],['begins','Begins with'],['ends','Ends with'],['regex','Regular expression']];
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const escapeRegex=(text:string)=>text.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');

/** Build a matcher for one search strategy; invalid regular expressions throw. */
export function findMatcher(criteria:string,strategy:FindStrategy,caseSensitive:boolean):(value:string)=>boolean{
  if(!criteria)throw new Error('Enter text to find.');
  if(strategy==='regex'){
    let pattern:RegExp;
    try{pattern=new RegExp(criteria,caseSensitive?'u':'iu');}catch(error){throw new Error(`Invalid regular expression: ${error instanceof Error?error.message:String(error)}`);}
    return value=>pattern.test(value);
  }
  if(strategy==='word'){const pattern=new RegExp(`(?<![\\p{L}\\p{N}_])${escapeRegex(criteria)}(?![\\p{L}\\p{N}_])`,caseSensitive?'u':'iu');return value=>pattern.test(value);}
  const fold=(text:string)=>caseSensitive?text:text.toLocaleLowerCase(),needle=fold(criteria);
  if(strategy==='content')return value=>fold(value)===needle;
  if(strategy==='begins')return value=>fold(value).startsWith(needle);
  if(strategy==='ends')return value=>fold(value).endsWith(needle);
  return value=>fold(value).includes(needle);
}

/** Search every tag field value and the filename of each listed file. */
export function findInList(items:FindItem[],criteria:string,strategy:FindStrategy,caseSensitive:boolean):FindResult[]{
  const matches=findMatcher(criteria,strategy,caseSensitive),results:FindResult[]=[];
  for(const item of items){
    if(matches(item.filename))results.push({path:item.path,filename:item.filename,field:'Filename',value:item.filename});
    for(const [field,values] of Object.entries(item.fields)){
      if(field.startsWith('_'))continue;
      for(const value of values)if(matches(value))results.push({path:item.path,filename:item.filename,field,value});
    }
  }
  return results;
}

export type FindSort={key:'filename'|'field'|'value';ascending:boolean};
export function sortFindResults(results:FindResult[],sort:FindSort,collator=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'})):FindResult[]{
  return results.map((result,index)=>({result,index})).sort((a,b)=>(collator.compare(a.result[sort.key],b.result[sort.key])||a.index-b.index)*(sort.ascending?1:-1)).map(item=>item.result);
}

export type FindHost={items:()=>FindItem[];select:(paths:string[],focus?:string)=>void;listed:(path:string)=>boolean};
/** Edit → Find: a non-modal search window whose results select files in the File List.
 * Used criteria are remembered in the configuration's helper lists (history `find`);
 * the ► button next to the search field removes one or all of them. */
export class FindUI{
  private dialog?:HTMLDialogElement;
  constructor(private host:FindHost){}
  open(){
    if(this.dialog?.open){this.dialog.querySelector<HTMLInputElement>('#find-criteria')!.select();return;}
    let results:FindResult[]=[],sort:FindSort={key:'filename',ascending:true};
    const dialog=document.createElement('dialog');dialog.className='find-dialog';dialog.setAttribute('aria-label','Find');this.dialog=dialog;
    dialog.innerHTML=`<header>Find</header><div class="dialog-content"><div class="action-input"><input id="find-criteria" data-helper="history" data-history="find" data-history-enter aria-label="Find" placeholder="Text to find in tag fields and filenames" autocomplete="off" spellcheck="false"><button class="primary" id="find-run">Find</button></div>
      <div class="field-pair"><label class="field"><span>STRATEGY</span><select id="find-strategy">${findStrategies.map(([value,label])=>`<option value="${value}">${label}</option>`).join('')}</select></label><label class="action-check"><input type="checkbox" id="find-case"> Case sensitive</label></div>
      <p id="find-status" role="status">Search all tag fields and filenames of the listed files.</p><div class="preview find-results" tabindex="-1"></div></div>
      <div class="dialog-footer"><button id="find-select" disabled>Select all results</button><button data-close>Close</button></div>`;
    document.body.append(dialog);
    const q=<T extends Element>(selector:string)=>dialog.querySelector<T>(selector)!;
    const status=(text:string)=>{q('#find-status').textContent=text;};
    const table=()=>{
      const rows=sortFindResults(results,sort);
      const head=(key:FindSort['key'],label:string)=>`<th aria-sort="${sort.key===key?sort.ascending?'ascending':'descending':'none'}"><button data-find-sort="${key}">${label}${sort.key===key?sort.ascending?' ↑':' ↓':''}</button></th>`;
      q('.find-results').innerHTML=rows.length?`<table><thead><tr>${head('filename','Filename')}${head('field','Field')}${head('value','Match')}</tr></thead><tbody>${rows.slice(0,2000).map(row=>`<tr tabindex="0" data-path="${html(row.path)}"><td>${html(row.filename)}</td><td>${html(row.field)}</td><td title="${html(row.value)}">${html(row.value)}</td></tr>`).join('')}</tbody></table>`:'';
      q<HTMLButtonElement>('#find-select').disabled=!rows.length;
      dialog.querySelectorAll<HTMLButtonElement>('[data-find-sort]').forEach(button=>button.addEventListener('click',()=>{const key=button.dataset.findSort as FindSort['key'];sort={key,ascending:sort.key===key?!sort.ascending:true};table();}));
      dialog.querySelectorAll<HTMLTableRowElement>('tr[data-path]').forEach(row=>{
        const choose=()=>{if(!this.host.listed(row.dataset.path!)){status('This file is no longer listed.');return;}this.host.select([row.dataset.path!],row.dataset.path);dialog.querySelectorAll('tr.selected').forEach(r=>r.classList.remove('selected'));row.classList.add('selected');};
        row.addEventListener('click',choose);
        row.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();choose();}if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();const next=(event.key==='ArrowDown'?row.nextElementSibling:row.previousElementSibling) as HTMLTableRowElement|null;next?.focus();next?.click();}});
      });
    };
    const run=()=>{
      const criteria=q<HTMLInputElement>('#find-criteria').value;
      try{
        results=findInList(this.host.items(),criteria,q<HTMLSelectElement>('#find-strategy').value as FindStrategy,q<HTMLInputElement>('#find-case').checked);
        const files=new Set(results.map(result=>result.path)).size;
        status(results.length?`${results.length} match${results.length===1?'':'es'} in ${files} file${files===1?'':'s'}${results.length>2000?'. Showing the first 2000.':'.'}`:'No matches.');
      }catch(error){results=[];status(error instanceof Error?error.message:String(error));}
      table();
    };
    q('#find-run').addEventListener('click',run);
    q('#find-criteria').addEventListener('keydown',event=>{if((event as KeyboardEvent).key==='Enter'){event.preventDefault();run();}});
    q('#find-select').addEventListener('click',()=>{const paths=[...new Set(sortFindResults(results,sort).map(result=>result.path))].filter(path=>this.host.listed(path));this.host.select(paths,paths[0]);});
    q('[data-close]').addEventListener('click',()=>dialog.close());
    dialog.addEventListener('keydown',event=>{if(event.key==='Escape'){event.preventDefault();dialog.close();}});
    dialog.addEventListener('close',()=>{dialog.remove();if(this.dialog===dialog)this.dialog=undefined;});
    dialog.show();q<HTMLInputElement>('#find-criteria').focus();
  }
}
