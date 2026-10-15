// File → Remove tag (https://docs.mp3tag.de/menus/file/): the Ctrl+R binding
// and the confirmation that names the tag types Options → Tags removes.
import type { TagOptions } from './options-ui';

export type KeyInput=Pick<KeyboardEvent,'key'|'code'|'ctrlKey'|'metaKey'|'altKey'|'shiftKey'>;
/** Ctrl+R (⌘R on macOS) removes tags outside text fields. Every other Ctrl/⌘+R
 * combination is only swallowed, because WebView2 reloads the page on it. */
export function removeTagShortcut(event:KeyInput,typing:boolean,mac:boolean):'remove-tags'|'suppress'|undefined{
  if(!(event.ctrlKey||event.metaKey)||!(event.code==='KeyR'||event.key.toLowerCase()==='r'))return undefined;
  const primary=mac?event.metaKey:event.ctrlKey,other=mac?event.ctrlKey:event.metaKey;
  return primary&&!other&&!event.shiftKey&&!event.altKey&&!typing?'remove-tags':'suppress';
}

const list=(names:string[])=>names.length>1?`${names.slice(0,-1).join(', ')} and ${names[names.length-1]}`:names[0];
function kinds(remove:{remove_id3v1:boolean;remove_id3v2:boolean;remove_ape:boolean},page:string){
  const names=[remove.remove_id3v1&&'ID3v1',remove.remove_id3v2&&'ID3v2',remove.remove_ape&&'APE'].filter((name):name is string=>!!name);
  return names.length?list(names):`nothing (no tag type is checked under Options → Tags → ${page})`;
}
/** Confirmation text: which tag kinds each format family loses. */
export function removeTagPrompt(count:number,tags:Pick<TagOptions,'mpeg'|'ape'>){
  return`Remove tags from ${count} selected file${count===1?'':'s'}?\n\nMP3, TTA: ${kinds(tags.mpeg,'Mpeg')}\nAPE, MPC, WavPack, TAK, OptimFROG: ${kinds(tags.ape,'Ape, Mpc')}\nOther formats: all tags`;
}
