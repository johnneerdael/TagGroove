// Options → Tags → Mapping (https://docs.mp3tag.de/customization/options/tags/mapping/):
// user-defined field names per tag type. Reading renames each Source field to
// its Target; saving writes a Target back to every Source mapped to it.
import type { OptionsDraft, OptionsPage } from './options-ui';

export type FieldMapping={tag_type:string;source:string;target:string};
/** Tag types a rule can apply to, as stored and as shown. */
export const MAPPING_TAG_TYPES:[string,string][]=[['APEv2','APEv2'],['ID3v2','ID3v2'],['MP4','MP4'],['VorbisComment','Vorbis Comments'],['WMA','WMA']];
const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));

export function normalizeMappings(value:unknown):FieldMapping[]{
  if(!Array.isArray(value))return[];
  return value.filter(item=>item&&typeof item==='object').map(item=>{const r=item as Record<string,unknown>;return{tag_type:String(r.tag_type??''),source:String(r.source??''),target:String(r.target??'')};});
}
const tagLabel=(tagType:string)=>MAPPING_TAG_TYPES.find(([value])=>value.toLowerCase()===tagType.toLowerCase())?.[1]??tagType;
export function mappingLabel(rule:FieldMapping){return`${tagLabel(rule.tag_type)}: ${rule.source||'?'} → ${rule.target||'?'}`;}
/** The first problem of a rule list, mirroring the backend validation. */
export function mappingsError(rules:FieldMapping[]):string|undefined{
  const targets=new Map<string,string>();
  for(const [index,rule] of rules.entries()){
    const name=`Mapping ${index+1}`;
    if(!MAPPING_TAG_TYPES.some(([value])=>value.toLowerCase()===rule.tag_type.toLowerCase()))return`${name}: choose a tag type.`;
    if(!rule.source.trim())return`${name}: enter the source field.`;
    if(!rule.target.trim())return`${name}: enter the target field.`;
    if(/[\0\r\n]/.test(rule.source+rule.target))return`${name}: field names cannot contain line breaks.`;
    const key=`${rule.tag_type.toLowerCase()}\n${rule.source.toLowerCase()}`,target=targets.get(key);
    if(target!==undefined&&target!==rule.target.toLowerCase())return`${name}: ${rule.source} already maps to another target. One source cannot map to several targets.`;
    targets.set(key,rule.target.toLowerCase());
  }
  return undefined;
}

const rules=(draft:OptionsDraft)=>normalizeMappings(draft.tags.mappings);
export function mappingOptionsPage():OptionsPage{
  let current=0;
  return{id:'tags-mapping',parent:'tags',label:'Mapping',order:30,
    description:'Rename fields for one tag type. After reading, each Source field is shown as its Target; saving writes the Target back to every Source mapped to it, so several sources can share one target.',
    validate:draft=>mappingsError(rules(draft)),
    controls:[{kind:'custom',id:'mapping-editor',paths:['tags.mappings'],
      render:draft=>{
        const list=rules(draft),index=Math.min(current,list.length-1),rule=list[index];
        return`<div class="mapping-editor"><label class="field"><span>Mappings</span><select id="mapping-list" size="8" aria-label="Field mappings">${list.map((r,i)=>`<option value="${i}" ${i===index?'selected':''}>${html(mappingLabel(r))}</option>`).join('')}</select></label>
          <div class="group-buttons"><button type="button" id="mapping-new">New</button><button type="button" id="mapping-remove" ${rule?'':'disabled'}>Remove</button><button type="button" id="mapping-up" ${index>0?'':'disabled'}>Move up</button><button type="button" id="mapping-down" ${rule&&index<list.length-1?'':'disabled'}>Move down</button></div>
          ${rule?`<fieldset class="numbering-section"><legend>Edit mapping</legend>
            <label class="field"><span>Tag type</span><select id="mapping-tag-type">${MAPPING_TAG_TYPES.map(([value,label])=>`<option value="${value}" ${value.toLowerCase()===rule.tag_type.toLowerCase()?'selected':''}>${html(label)}</option>`).join('')}</select></label>
            <label class="field"><span>Source</span><input id="mapping-source" value="${html(rule.source)}" placeholder="Field name OpenTag shows now, for example DATE" autocomplete="off" spellcheck="false"></label>
            <label class="field"><span>Target</span><input id="mapping-target" value="${html(rule.target)}" placeholder="Your field name, for example YEAR" autocomplete="off" spellcheck="false"></label></fieldset>`:'<p>No mappings are defined.</p>'}
          <p>For ID3v2 and MP4 the source is OpenTag’s field name (for example COMPOSER), not the frame or atom. Files without pending changes are reloaded when you press OK.</p></div>`;
      },
      bind:(root,draft,refresh)=>{
        // Handlers read the rules again: typing edits the draft without re-rendering.
        const index=Math.min(current,rules(draft).length-1);
        const save=(next:FieldMapping[])=>{draft.tags.mappings=next;};
        root.querySelector('#mapping-list')?.addEventListener('change',event=>{current=Number((event.target as HTMLSelectElement).value);refresh();});
        root.querySelector('#mapping-new')!.addEventListener('click',()=>{const list=rules(draft);save([...list,{tag_type:list[index]?.tag_type??'VorbisComment',source:'',target:''}]);current=list.length;refresh();root.querySelector<HTMLInputElement>('#mapping-source')?.focus();});
        root.querySelector('#mapping-remove')!.addEventListener('click',()=>{save(rules(draft).filter((_,i)=>i!==index));current=Math.max(0,index-1);refresh();});
        const move=(to:number)=>{const next=rules(draft);[next[index],next[to]]=[next[to],next[index]];save(next);current=to;refresh();};
        root.querySelector('#mapping-up')!.addEventListener('click',()=>move(index-1));
        root.querySelector('#mapping-down')!.addEventListener('click',()=>move(index+1));
        const edit=(key:keyof FieldMapping,value:string)=>{
          const next=rules(draft);next[index]={...next[index],[key]:value};save(next);
          const option=root.querySelector<HTMLOptionElement>(`#mapping-list option[value="${index}"]`);if(option)option.textContent=mappingLabel(next[index]);
        };
        root.querySelector('#mapping-tag-type')?.addEventListener('change',event=>edit('tag_type',(event.target as HTMLSelectElement).value));
        root.querySelector('#mapping-source')?.addEventListener('input',event=>edit('source',(event.target as HTMLInputElement).value));
        root.querySelector('#mapping-target')?.addEventListener('input',event=>edit('target',(event.target as HTMLInputElement).value));
      }}]};
}
