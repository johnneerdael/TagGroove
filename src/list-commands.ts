export type ListCommand='unselect-all'|'invert-selection'|'select-next'|'select-similar'|'select-next-similar'|'remove-from-list'|'refresh'|'find'|'edit-cell'|'filter-focus';
type KeyInput={key:string;code?:string;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean;altKey:boolean};

/** Mp3tag File List shortcuts (Ctrl on Windows/Linux, Cmd on macOS). Mac keyboards
 * label Backspace "delete" and most have no forward Delete key, so ⌫ also removes
 * files from the list there (⌘⌫ deletes them, as in Finder). */
export function listShortcut(event:KeyInput,typing:boolean,mac=false):ListCommand|undefined{
  const command=event.ctrlKey||event.metaKey,key=event.key.length===1?event.key.toLowerCase():event.key;
  if(event.altKey)return undefined;
  if(!command&&!event.shiftKey){
    if(key==='F5')return 'refresh';
    if(key==='F3')return 'filter-focus';
    if(typing)return undefined;
    if(key==='F2')return 'edit-cell';
    if(key==='Delete'||(mac&&key==='Backspace'))return 'remove-from-list';
    return undefined;
  }
  if(!command)return undefined;
  if(key==='f'&&!event.shiftKey)return 'find';
  if(typing)return undefined;
  if(key==='a'&&event.shiftKey)return 'invert-selection';
  if(key==='u'&&!event.shiftKey)return 'unselect-all';
  if(key==='n'&&!event.shiftKey)return 'select-next';
  if(key==='g')return event.shiftKey?'select-next-similar':'select-similar';
  return undefined;
}

export function invertSelection(visible:string[],selected:ReadonlySet<string>):string[]{return visible.filter(path=>!selected.has(path));}

/** The file after the last selected file, or the first file when nothing is selected. */
export function nextFile(visible:string[],selected:ReadonlySet<string>):string|undefined{
  let last=-1;visible.forEach((path,index)=>{if(selected.has(path))last=index;});
  return visible[Math.min(visible.length-1,last+1)];
}

/** Files sharing the active file's value for the active sort criteria. */
export function similarFiles(visible:string[],key:(path:string)=>string,active:string|undefined):string[]{
  if(!active||!visible.includes(active))return[];
  const target=key(active);return visible.filter(path=>key(path)===target);
}

/** The next run of similar files after the current selection whose value differs from it. */
export function nextSimilarFiles(visible:string[],key:(path:string)=>string,selected:ReadonlySet<string>):string[]{
  let last=-1;visible.forEach((path,index)=>{if(selected.has(path))last=index;});
  const current=last>=0?key(visible[last]):undefined;
  const start=visible.findIndex((path,index)=>index>last&&key(path)!==current);
  if(start<0)return[];
  const target=key(visible[start]);return visible.filter(path=>key(path)===target);
}

export function formatDuration(seconds:number):string{
  const total=Math.max(0,Math.floor(seconds)),days=Math.floor(total/86400),hours=Math.floor(total%86400/3600),minutes=Math.floor(total%3600/60),rest=total%60;
  const clock=hours||days?`${hours}:${String(minutes).padStart(2,'0')}:${String(rest).padStart(2,'0')}`:`${minutes}:${String(rest).padStart(2,'0')}`;
  return days?`${days}d ${clock}`:clock;
}
export function formatSize(bytes:number):string{
  if(bytes<1024)return`${bytes} B`;
  const units=['KB','MB','GB','TB'];let value=bytes/1024,unit=0;
  while(value>=1024&&unit<units.length-1){value/=1024;unit++;}
  return`${value.toFixed(value>=100?0:1)} ${units[unit]}`;
}

type SummaryEntry={duration:number;size:number;format:string;bitrate:number;sample_rate:number;channels:number;technical?:Record<string,string[]>};
/** Status-bar segments: selection totals, listed totals and active file properties. */
export function listSummary(selected:SummaryEntry[],listed:SummaryEntry[],total:number,active?:SummaryEntry):string[]{
  const sum=(items:SummaryEntry[])=>`${formatDuration(items.reduce((s,e)=>s+e.duration,0))} | ${formatSize(items.reduce((s,e)=>s+e.size,0))}`;
  const segments=[selected.length?`${selected.length} selected (${sum(selected)})`:'Select a file to view its tags'];
  if(total)segments.push(`${listed.length===total?`${total}`:`${listed.length} of ${total}`} file${total===1?'':'s'} (${sum(listed)})`);
  if(active){
    const codec=active.technical?._CODEC?.[0]||active.format;
    segments.push([codec,active.bitrate?`${active.bitrate} kbps`:'',active.sample_rate?`${active.sample_rate.toLocaleString()} Hz`:'',active.channels?`${active.channels} channel${active.channels===1?'':'s'}`:''].filter(Boolean).join(' · '));
  }
  return segments;
}
