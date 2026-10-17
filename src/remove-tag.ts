// File → Remove tag (https://docs.mp3tag.de/menus/file/): the Ctrl+R binding
// and the confirmation that names the tag types Options → Tags removes.
import type { TagOptions } from './options-ui';

export type KeyInput=Pick<KeyboardEvent,'key'|'code'|'ctrlKey'|'metaKey'|'altKey'|'shiftKey'>;
/** Ctrl+R (⌘R on macOS) removes tags outside text fields. Every other Ctrl/⌘+R
 * combination is only swallowed, because WebView2 reloads the page on it. Ctrl+Alt+R
 * is AltGr+R on Windows, which types a character on some layouts, so it is left alone. */
export function removeTagShortcut(event:KeyInput,typing:boolean,mac:boolean):'remove-tags'|'suppress'|undefined{
  if(!(event.ctrlKey||event.metaKey)||!(event.code==='KeyR'||event.key.toLowerCase()==='r'))return undefined;
  if(event.altKey&&!event.metaKey)return undefined;
  const primary=mac?event.metaKey:event.ctrlKey,other=mac?event.ctrlKey:event.metaKey;
  return primary&&!other&&!event.shiftKey&&!event.altKey&&!typing?'remove-tags':'suppress';
}

const list=(names:string[])=>{
  if(names.length<2)return names[0];
  const name=names.slice(0,-1).join(', ');
  return`${name} and ${names[names.length-1]}`;
};
function kinds(remove:{remove_id3v1:boolean;remove_id3v2:boolean;remove_ape:boolean},page:string){
  const names=[remove.remove_id3v1&&'ID3v1',remove.remove_id3v2&&'ID3v2',remove.remove_ape&&'APE'].filter((name):name is string=>!!name);
  return names.length?list(names):`nothing (no tag type is checked under Options → Tags → ${page})`;
}
/** English source rows; native dialogs translate these once and retain the code-owned line breaks. */
export function removeTagParts(count:number,tags:Pick<TagOptions,'mpeg'|'ape'>){
  return [`Remove tags from ${count} selected file${count===1?'':'s'}?`,'',
    `MP3, TTA: ${kinds(tags.mpeg,'Mpeg')}`,
    `APE, MPC, WavPack, TAK, OptimFROG: ${kinds(tags.ape,'Ape, Mpc')}`,
    'Other formats: all tags'];
}
/** Plain text for non-native callers; preserve the same source-row layout. */
export function removeTagPrompt(count:number,tags:Pick<TagOptions,'mpeg'|'ape'>,translate:(text:string)=>string=text=>text){return removeTagParts(count,tags).map(translate).join('\n');}
