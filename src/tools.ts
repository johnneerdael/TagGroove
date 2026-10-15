// Options → Tools (https://docs.mp3tag.de/customization/options/tools/):
// user-defined external programs started for the selected files. The backend
// expands placeholders and starts the program without a shell.
import type { OptionsPage, OptionsDraft } from './options-ui';

export type Tool={name:string;path:string;parameters:string;for_each_file:boolean};
export const TOOL_LIMIT=99;
export const PLACEHOLDERS:[string,string][]=[['%_path%','Full file path'],['%_filename_ext%','File name'],['%_filename%','File name without extension'],['%_filename_rel%','Relative file path'],['%_directory%','Directory'],['%_folderpath%','Folder path'],['%_folderpath_rel%','Relative folder path']];
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));

export function normalizeTools(value:unknown):Tool[]{
  if(!Array.isArray(value))return[];
  return value.filter(item=>item&&typeof item==='object').map(item=>{const t=item as Record<string,unknown>;return{name:String(t.name??''),path:String(t.path??''),parameters:String(t.parameters??''),for_each_file:t.for_each_file===true};});
}
export function defaultToolIndex(values:Record<string,unknown>,tools:Tool[]){const index=values.default_tool;return typeof index==='number'&&Number.isInteger(index)&&index>=0&&index<tools.length?index:-1;}
const absolute=(path:string)=>/^(\/|[A-Za-z]:[\\/]|\\\\)/.test(path);
/** The first problem of a tool list, mirroring the checks the backend repeats before running. */
export function toolsError(tools:Tool[]):string|undefined{
  if(tools.length>TOOL_LIMIT)return`Define at most ${TOOL_LIMIT} tools.`;
  for(const [index,tool] of tools.entries()){
    const name=tool.name.trim()||`Tool ${index+1}`;
    if(!tool.name.trim())return`Tool ${index+1} needs a name.`;
    if(!tool.path.trim())return`${name}: enter the program path.`;
    if(!absolute(tool.path.trim()))return`${name}: enter the full path of the program.`;
    if((tool.parameters.match(/"/g)||[]).length%2)return`${name}: the parameters contain an unmatched quote.`;
  }
  return undefined;
}
/** Shortcut label for the first ten tools (Ctrl+1 … Ctrl+0). */
export function toolShortcut(index:number,mac:boolean){return index<10?`${mac?'⌘':'Ctrl'} ${(index+1)%10}`:'';}
/** Tool index for Ctrl/⌘+digit, or -1. */
export function toolShortcutIndex(event:Pick<KeyboardEvent,'key'|'code'|'ctrlKey'|'metaKey'|'altKey'|'shiftKey'>,mac:boolean){
  if(event.altKey||event.shiftKey||!(mac?event.metaKey:event.ctrlKey))return -1;
  const digit=/^Digit(\d)$/.exec(event.code)?.[1]??(/^\d$/.test(event.key)?event.key:undefined);
  return digit===undefined?-1:(Number(digit)+9)%10;
}

type PageHost={pickProgram:()=>Promise<string|null>};
const tools=(draft:OptionsDraft)=>normalizeTools((draft.pages.tools as Record<string,unknown>|undefined)?.tools);
const store=(draft:OptionsDraft,list:Tool[],defaultTool:number)=>{draft.pages.tools={...draft.pages.tools,tools:list,default_tool:defaultTool};};
export function toolsOptionsPage(host:PageHost):OptionsPage{
  let current=0;
  return{id:'tools',label:'Tools',order:70,defaults:{tools:[],default_tool:-1},
    description:'External programs for the selected files, available in the Tools menu. The first ten also run with Ctrl+1 … Ctrl+0 (⌘ on macOS).',
    validate:draft=>toolsError(tools(draft)),
    controls:[{kind:'custom',id:'tools-editor',paths:['pages.tools.tools','pages.tools.default_tool'],
      render:draft=>{
        const list=tools(draft),index=Math.min(current,list.length-1),tool=list[index],fallback=defaultToolIndex(draft.pages.tools as Record<string,unknown>??{},list);
        return`<div class="tools-editor"><label class="field"><span>Tools</span><select id="tools-list" size="6" aria-label="Tools">${list.map((t,i)=>`<option value="${i}" ${i===index?'selected':''} ${t.name?'translate="no"':''}>${html(t.name||`Tool ${i+1}`)}</option>`).join('')}</select></label>
          <div class="group-buttons"><button type="button" id="tool-new" ${list.length>=TOOL_LIMIT?'disabled':''}>New</button><button type="button" id="tool-remove" ${tool?'':'disabled'}>Remove</button><button type="button" id="tool-up" ${index>0?'':'disabled'}>Move up</button><button type="button" id="tool-down" ${tool&&index<list.length-1?'':'disabled'}>Move down</button></div>
          ${tool?`<fieldset class="numbering-section"><legend>Edit tool</legend>
            <label class="field"><span>Name</span><input id="tool-name" value="${html(tool.name)}" autocomplete="off"></label>
            <div class="action-input"><label class="field"><span>Path</span><input id="tool-path" value="${html(tool.path)}" placeholder="Full path of the program" autocomplete="off" spellcheck="false"></label><button type="button" id="tool-browse">Browse…</button></div>
            <label class="field"><span>Parameter</span><input id="tool-parameters" value="${html(tool.parameters)}" placeholder="&quot;%_path%&quot;" autocomplete="off" spellcheck="false"></label>
            <label class="action-check"><input type="checkbox" id="tool-each" ${tool.for_each_file?'checked':''}> for all selected files (otherwise only the first selected file)</label>
            <p>Placeholders: ${PLACEHOLDERS.map(([p,label])=>`<code title="${html(label)}">${html(p)}</code>`).join(' ')}. Quote arguments that contain spaces; no shell is involved. Empty parameters pass the file path.</p></fieldset>`:'<p>No tools are defined.</p>'}
          <label class="field"><span>Default tool (double-click in the file list)</span><select id="tool-default"><option value="-1">No action</option>${list.map((t,i)=>`<option value="${i}" ${i===fallback?'selected':''} ${t.name?'translate="no"':''}>${html(t.name||`Tool ${i+1}`)}</option>`).join('')}</select></label></div>`;
      },
      bind:(root,draft,refresh)=>{
        const list=tools(draft),index=Math.min(current,list.length-1),fallback=defaultToolIndex(draft.pages.tools as Record<string,unknown>??{},list);
        const save=(next:Tool[],defaultTool=fallback)=>store(draft,next,defaultTool);
        root.querySelector('#tools-list')?.addEventListener('change',event=>{current=Number((event.target as HTMLSelectElement).value);refresh();});
        root.querySelector('#tool-new')!.addEventListener('click',()=>{save([...list,{name:'New tool',path:'',parameters:'"%_path%"',for_each_file:true}]);current=list.length;refresh();root.querySelector<HTMLInputElement>('#tool-name')?.select();});
        root.querySelector('#tool-remove')!.addEventListener('click',()=>{save(list.filter((_,i)=>i!==index),fallback===index?-1:fallback>index?fallback-1:fallback);current=Math.max(0,index-1);refresh();});
        const move=(to:number)=>{const next=[...list];[next[index],next[to]]=[next[to],next[index]];save(next,fallback===index?to:fallback===to?index:fallback);current=to;refresh();};
        root.querySelector('#tool-up')!.addEventListener('click',()=>move(index-1));
        root.querySelector('#tool-down')!.addEventListener('click',()=>move(index+1));
        const edit=(id:string,key:'name'|'path'|'parameters')=>root.querySelector<HTMLInputElement>(`#${id}`)?.addEventListener('input',event=>{
          const next=tools(draft);next[index]={...next[index],[key]:(event.target as HTMLInputElement).value};save(next);
          if(key==='name'){const option=root.querySelector<HTMLOptionElement>(`#tools-list option[value="${index}"]`),fallbackOption=root.querySelector<HTMLOptionElement>(`#tool-default option[value="${index}"]`);for(const o of [option,fallbackOption])if(o)o.textContent=next[index].name||`Tool ${index+1}`;}
        });
        edit('tool-name','name');edit('tool-path','path');edit('tool-parameters','parameters');
        root.querySelector('#tool-each')?.addEventListener('change',event=>{const next=tools(draft);next[index]={...next[index],for_each_file:(event.target as HTMLInputElement).checked};save(next);});
        root.querySelector('#tool-browse')?.addEventListener('click',async()=>{const path=await host.pickProgram();if(path){const next=tools(draft);next[index]={...next[index],path};save(next);refresh();}});
        root.querySelector('#tool-default')!.addEventListener('change',event=>save(tools(draft),Number((event.target as HTMLSelectElement).value)));
      }}]};
}

export type ToolsHost={
  values:()=>Record<string,unknown>;
  run:(tool:Tool,paths:string[])=>Promise<{started:number}>;
  selection:()=>string[];
  reason:()=>string;
  confirm:(message:string,title:string)=>Promise<boolean>;
  report:(message:string)=>void;fail:(error:unknown)=>void;
  mac:boolean;
};
export class ToolsUI {
  constructor(private host:ToolsHost){}
  get tools(){return normalizeTools(this.host.values().tools);}
  get defaultTool(){return defaultToolIndex(this.host.values(),this.tools);}
  /** Why tools cannot run now, or an empty string. */
  get reason(){return this.host.reason();}
  menuHTML(){
    const list=this.tools,reason=this.host.reason();
    return list.length?`<hr>${list.map((tool,index)=>`<button data-tool="${index}"${reason?'':` data-verbatim="${html(tool.path)}"`} ${reason?'disabled':''} title="${html(reason||tool.path)}"><span translate="no">${html(tool.name)}</span>${index<10?`<small>${toolShortcut(index,this.host.mac)}</small>`:''}</button>`).join('')}`
      :'<p class="menu-note">No tools defined. Add them in Configure tools….</p>';
  }
  bindMenu(root:HTMLElement){
    root.querySelectorAll<HTMLButtonElement>('[data-tool]').forEach(button=>button.addEventListener('click',()=>{root.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(d=>d.open=false);void this.run(Number(button.dataset.tool));}));
  }
  /** Handle Ctrl/⌘+1 … 0. Returns true when the key belongs to a tool slot. */
  shortcut(event:KeyboardEvent,typing:boolean){
    const index=toolShortcutIndex(event,this.host.mac);
    if(index<0||typing||index>=this.tools.length)return false;
    event.preventDefault();void this.run(index);return true;
  }
  async run(index:number){
    const tool=this.tools[index];if(!tool)return;
    const reason=this.host.reason(),paths=this.host.selection();
    if(reason){this.host.report(reason);return;}
    if(tool.for_each_file&&paths.length>10&&!await this.host.confirm(`Start ${tool.name} ${paths.length} times, once for each selected file?`,'Tools'))return;
    try{const result=await this.host.run(tool,paths);this.host.report(`Started ${tool.name}${result.started>1?` ${result.started} times`:''}`);}
    catch(error){this.host.fail(error);}
  }
}
