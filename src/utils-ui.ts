// Utils → Remove Tag Types, Remove Unsupported Fields, Optimize and Strip
// chapters MP4, Edit → Copy as Text… and Repeat, Help → Keyboard
// shortcuts, and File → Save/Load configuration
// (https://docs.mp3tag.de/utils/, menus/edit, menus/help, menus/file, keyboard).
import type { HistoryState } from './edit-history';

const html=(value:unknown)=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export type Modal=(title:string,content:string,buttons:string)=>HTMLDialogElement;
export type ModalError=(dialog:HTMLDialogElement,error:unknown)=>void;

// Remove unsupported fields, Optimize ---------------------------------------------
export type Util='remove_unsupported_fields'|'optimize';
export const UTILS:Record<Util,{label:string;formats:string[];applies:string;effect:string}>={
  remove_unsupported_fields:{label:'Remove unsupported fields',formats:['MP3','AAC','AIFF','WAV'],applies:'MP3 files (ID3v2 tags, also in AAC, AIFF and WAV)',
    effect:'ID3v2 frames OpenTag cannot display (listed by %_id3v2_unknown_frames%, such as PRIV or GEOB) are removed permanently.'},
  optimize:{label:'Optimize',formats:['FLAC','MP4'],applies:'FLAC and MP4 files',
    effect:'FLAC files get exactly 4 KB of padding. MP4 files lose their free space and get the movie and tag data before the audio data.'},
};
/** How many of the selected files a util applies to, by their format. */
export function utilTargets(util:Util,formats:string[]){return formats.filter(format=>UTILS[util].formats.includes(format)).length;}
export function utilMessage(util:Util,formats:string[]){
  const spec=UTILS[util],count=utilTargets(util,formats),other=formats.length-count;
  return `${spec.label} for ${count} of ${formats.length} selected file${formats.length===1?'':'s'}? ${spec.effect}${other?` ${other} file${other===1?' is':'s are'} skipped: this applies to ${spec.applies}.`:''} Files that need no change are not rewritten. Edit → Undo restores the files during this session.`;
}

// Remove tag types ---------------------------------------------------------------
export type RemovableTag='id3v1'|'id3v2'|'ape';
export const REMOVABLE_TAGS:[RemovableTag,string][]=[['id3v1','ID3v1'],['id3v2','ID3v2'],['ape','APEv2']];
/** Kinds present in a `_TAG` technical value such as "ID3v2.3 ID3v1". */
export function presentTagKinds(tag:string|undefined):Set<RemovableTag>{
  const kinds=new Set<RemovableTag>();
  for(const token of (tag||'').split(/\s+/)){if(token==='ID3v1')kinds.add('id3v1');else if(/^ID3v2/.test(token))kinds.add('id3v2');else if(token==='APE')kinds.add('ape');}
  return kinds;
}
export function tagKindCounts(tags:(string|undefined)[]):Record<RemovableTag,number>{
  const counts={id3v1:0,id3v2:0,ape:0};
  for(const tag of tags)for(const kind of presentTagKinds(tag))counts[kind]++;
  return counts;
}
export type RemoveTagTypesHost={modal:Modal;modalError:ModalError;tags:()=>(string|undefined)[];remove:(kinds:RemovableTag[])=>Promise<void>};
export function removeTagTypesDialog(host:RemoveTagTypesHost){
  const tags=host.tags(),counts=tagKindCounts(tags);
  const dialog=host.modal('Remove tag types',`<p>Remove the checked tag types from ${tags.length} selected file${tags.length===1?'':'s'}. Applies to MP3, TTA, APE, Musepack, WavPack, TAK and OptimFROG files; Options → Tags is not used.</p>
    <fieldset class="numbering-section"><legend>Tag types</legend>${REMOVABLE_TAGS.map(([kind,label])=>`<label class="action-check"><input type="checkbox" data-tag-kind="${kind}" ${counts[kind]?'':'disabled'}> ${label} <small>(${counts[kind]} file${counts[kind]===1?'':'s'})</small></label>`).join('')}</fieldset>
    <p>APEv2 tags can hold MP3GAIN_UNDO and ReplayGain values; removing them erases that information permanently. Edit → Undo restores the files during this session.</p>`,'<button id="remove-tag-types" class="primary" disabled>Remove</button>');
  dialog.classList.add('remove-tag-types-dialog');
  const button=dialog.querySelector<HTMLButtonElement>('#remove-tag-types')!;
  const chosen=()=>[...dialog.querySelectorAll<HTMLInputElement>('[data-tag-kind]:checked')].map(input=>input.dataset.tagKind as RemovableTag);
  dialog.addEventListener('change',()=>{button.disabled=!chosen().length;});
  button.addEventListener('click',async()=>{
    button.disabled=true;
    try{await host.remove(chosen());dialog.close();}catch(error){host.modalError(dialog,error);button.disabled=!chosen().length;}
  });
  dialog.querySelector<HTMLInputElement>('[data-tag-kind]:not(:disabled)')?.focus();
  return dialog;
}

// Strip chapters MP4 -------------------------------------------------------------
/** Selected MP4 files with chapters, from their `_SUBSONG_COUNT` and format. */
export function mp4ChapterFiles(entries:{format?:string;technical?:Record<string,string[]|undefined>}[]):{files:number;chapters:number}{
  let files=0,chapters=0;
  for(const entry of entries){
    const count=Number(entry.technical?._SUBSONG_COUNT?.[0]||0);
    if(entry.format==='MP4'&&count>0){files++;chapters+=count;}
  }
  return{files,chapters};
}
export function stripChaptersPrompt(found:{files:number;chapters:number}):string{
  return`Permanently remove ${found.chapters} chapter${found.chapters===1?'':'s'} from ${found.files} MP4 file${found.files===1?'':'s'}? The chapter boxes become free space; audio, video and tags are not changed.`;
}

// Copy as Text --------------------------------------------------------------------
export const COPY_FORMAT_DEFAULT='%artist% - %title%';
export const COPY_HISTORY_LIMIT=10;
/** Remember a format string at the top of the history, without duplicates. */
export function rememberFormat(history:unknown,format:string):string[]{
  const previous=Array.isArray(history)?history.filter((v):v is string=>typeof v==='string'):[];
  return[format,...previous.filter(v=>v!==format)].slice(0,COPY_HISTORY_LIMIT);
}
export type CopyTextHost={modal:Modal;modalError:ModalError;values:()=>Record<string,unknown>;remember:(values:Record<string,unknown>)=>Promise<void>;
  count:()=>number;evaluate:(format:string,limit?:number)=>Promise<string[]>;write:(text:string)=>Promise<void>;done:(lines:number)=>void};
export function copyAsTextDialog(host:CopyTextHost){
  const values=host.values(),history=rememberFormat(values.history,typeof values.format==='string'&&values.format?values.format:COPY_FORMAT_DEFAULT);
  const dialog=host.modal('Copy as Text',`<p>Copies one line per selected file (${host.count()}) to the clipboard. Use placeholders and scripting functions, for example <code>%_filename_ext%</code>, <code>$num(%track%,2)</code> or <code>%title%</code>.</p>
    <label class="field"><span>Format string</span><input id="copy-text-format" list="copy-text-history" value="${html(history[0])}" autocomplete="off" spellcheck="false"></label>
    <datalist id="copy-text-history">${history.map(v=>`<option value="${html(v)}"></option>`).join('')}</datalist>
    <div class="preview" id="copy-text-preview" aria-live="polite" aria-label="Preview"></div>`,'<button id="copy-text-ok" class="primary">Copy</button>');
  dialog.classList.add('copy-text-dialog');
  const input=dialog.querySelector<HTMLInputElement>('#copy-text-format')!,preview=dialog.querySelector<HTMLElement>('#copy-text-preview')!,button=dialog.querySelector<HTMLButtonElement>('#copy-text-ok')!;
  let generation=0,timer=0;
  const update=()=>{clearTimeout(timer);timer=window.setTimeout(async()=>{
    const current=++generation;
    try{const lines=await host.evaluate(input.value,5);if(current===generation)preview.innerHTML=lines.map(line=>`<div>${html(line)||'&nbsp;'}</div>`).join('')+(host.count()>5?`<div><small>… ${host.count()-5} more</small></div>`:'');dialog.querySelector('.dialog-error')?.remove();}
    catch(error){if(current===generation){preview.textContent='';host.modalError(dialog,error);}}
  },200);};
  input.addEventListener('input',update);
  button.addEventListener('click',async()=>{
    button.disabled=true;
    try{
      const lines=await host.evaluate(input.value);
      await host.write(lines.join('\n'));
      await host.remember({format:input.value,history:rememberFormat(history,input.value)});
      dialog.close();host.done(lines.length);
    }catch(error){host.modalError(dialog,error);}
    finally{button.disabled=false;}
  });
  update();input.focus();input.select();
  return dialog;
}

// Repeat --------------------------------------------------------------------------
export type Repeatable={name:string;enabled?:boolean}[];
/** Edit menu text for the shared Redo/Repeat command (Ctrl+Y): Redo wins while a step can be redone. */
export function redoOrRepeat(history:Pick<HistoryState,'redo'>,last:Repeatable|undefined):{kind:'redo'|'repeat'|'none';label:string}{
  if(history.redo)return{kind:'redo',label:`Redo ${history.redo.label} (${history.redo.files} file${history.redo.files===1?'':'s'})`};
  if(last?.length)return{kind:'repeat',label:`Repeat ${last.length===1?last[0].name:`${last.length} action groups`}`};
  return{kind:'none',label:'Redo'};
}

// Keyboard shortcuts --------------------------------------------------------------
export type ShortcutRow={group:string;command:string;keys:string};
/** Shortcuts that have no menu entry; menu shortcuts are read from the menus themselves. */
export const EXTRA_SHORTCUTS:ShortcutRow[]=[
  {group:'File List',command:'Filter',keys:'F3'},
  {group:'File List',command:'Move the selection',keys:'↑ ↓ Page Up Page Down Home End (Shift extends)'},
  {group:'File List',command:'Toggle selection of a file',keys:'⌘/Ctrl Click'},
  {group:'File List',command:'Select a range',keys:'Shift Click'},
  {group:'File List',command:'Extended tags (or the default tool from Options → Tools)',keys:'Double-click'},
  {group:'File List',command:'Extended tags',keys:'Enter'},
  {group:'File List',command:'Show the file in the file manager (file properties)',keys:'Alt Enter'},
  {group:'File List',command:'Auto-size one column',keys:'Double-click its right header edge'},
  {group:'Convert',command:'Auto-numbering, restarting the track counter in each folder',keys:'⌘/Ctrl ⇧ K'},
  {group:'Tools',command:'User-defined tool 1 … 10',keys:'⌘/Ctrl 1 … 0'},
  {group:'Actions',command:'Open the Actions menu and choose a group by its mnemonic',keys:'Alt A, then a letter'},
  {group:'Actions dialog',command:'New group or action',keys:'Insert'},
  {group:'Actions dialog',command:'Remove the group or action',keys:'Del (⌫ on macOS)'},
  {group:'Actions dialog',command:'Rename the group',keys:'F2'},
  {group:'Actions dialog',command:'Edit the group or action',keys:'F4'},
  {group:'Actions dialog',command:'Enable or disable the group',keys:'Space'},
  {group:'Actions dialog',command:'Enable all groups / disable all groups',keys:'⌘/Ctrl E / ⌘/Ctrl D'},
  {group:'Actions dialog',command:'Save groups',keys:'⌘/Ctrl S'},
  {group:'Actions dialog',command:'Move the group or action up / down',keys:'Alt ↑ / Alt ↓'},
  {group:'Actions dialog',command:'Move the group or action to the beginning / end',keys:'Alt Page Up / Alt Page Down'},
  {group:'Tag Sources dialog',command:'Enable all fields / disable all fields',keys:'⌘/Ctrl E / ⌘/Ctrl D'},
  {group:'Tag Sources dialog',command:'Edit the value of the focused field',keys:'F2'},
  {group:'Tag Sources dialog',command:'Move the focused file up / down',keys:'Alt ↑ / Alt ↓'},
  {group:'Tag Sources dialog',command:'Move the focused file to the beginning / end',keys:'Alt Page Up / Alt Page Down'},
  {group:'Tag Panel',command:'Focus a field by its underlined letter',keys:'Alt letter'},
  {group:'Tag Panel',command:'Add the field value to its custom values',keys:'Shift Enter'},
  {group:'Tag Panel',command:'Open the value list of a field',keys:'Alt ↓'},
  {group:'Helpers',command:'Next pane: Tag Panel, Filter, File List',keys:'Ctrl Tab'},
  {group:'Helpers',command:'Previous pane',keys:'Ctrl ⇧ Tab'},
  {group:'Helpers',command:'Format string helper menu of the focused input',keys:'Ctrl Space (or ⌃⌥ Space on macOS)'},
  {group:'Helpers',command:'Remove the current value from an input\'s drop-down history',keys:'⇧ Del'},
  {group:'Dialogs',command:'Close a dialog',keys:'Esc'},
];
/** Rows for every menu command that shows a shortcut, in menu order. */
export function menuShortcuts(menubar:ParentNode):ShortcutRow[]{
  return[...menubar.querySelectorAll<HTMLDetailsElement>(':scope > .menu')].flatMap(menu=>{
    const group=menu.querySelector('summary')?.textContent?.trim()||'';
    return[...menu.querySelectorAll<HTMLButtonElement>('.menu-list button')].flatMap(button=>{
      const keys=button.querySelector('small')?.textContent?.trim();
      const command=button.querySelector('span:not(.icon)')?.textContent?.trim()||button.textContent?.replace(keys||'','').trim()||'';
      return keys?[{group,command,keys}]:[];
    });
  });
}
export function shortcutsDialog(modal:Modal,rows:ShortcutRow[]){
  const groups=[...new Set(rows.map(row=>row.group))];
  const dialog=modal('Keyboard shortcuts',`<p>⌘/Ctrl means ⌘ on macOS and Ctrl elsewhere; Ctrl alone is the Control key everywhere. Alt is Option on macOS, where Option+key types characters such as å or ©: while a text field has focus there, press Control+Option with the key instead. Most shortcuts are ignored while typing in a text field.</p>${groups.map(group=>`<h3>${html(group)}</h3><table class="shortcut-table"><tbody>${rows.filter(row=>row.group===group).map(row=>`<tr><td>${html(row.command)}</td><td><kbd>${html(row.keys)}</kbd></td></tr>`).join('')}</tbody></table>`).join('')}`,'');
  dialog.classList.add('shortcuts-dialog');
  dialog.querySelector('[data-close]')!.textContent='Close';
  return dialog;
}

// Configuration backup ------------------------------------------------------------
export const CONFIGURATION_FILTER={name:'OpenTag configuration',extensions:['json']};
export function configurationFileName(date=new Date()){return`opentag-configuration-${date.toISOString().slice(0,10)}.json`;}
