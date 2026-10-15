// Window-level keyboard routing (docs.mp3tag.de/keyboard/). Pure functions so
// the platform rules are unit-tested without a webview:
// - Mp3tag's Ctrl becomes ⌘ on macOS, except where macOS reserves the ⌘ chord
//   (⌘Tab, ⌘Space, ⌘H, ⌘M, ⌘Q); those keep the physical Control key.
// - Alt/Option chords. On Windows and Linux Alt never types text, but Ctrl+Alt is
//   AltGr on Windows and AltGraph on Linux, which do. On macOS Option+key types
//   characters (Option+I ˆ, Option+A å, Option+G ©, Option+R ®, Option+M µ,
//   Option+6 §), so inside a text field only Control+Option triggers Alt shortcuts;
//   outside text fields Option alone works as Alt.
export type KeyInput={key:string;code?:string;ctrlKey:boolean;metaKey:boolean;shiftKey:boolean;altKey:boolean;getModifierState?:(key:string)=>boolean};
export type KeyContext={typing:boolean;mac:boolean};
export type WindowShortcut='reload'|'extended'|'discard'|'number-folders'|'groups'|'action'|'file-reveal'|'pane-next'|'pane-previous'|'file-delete-permanent'|'playlist-all'|'playlist-folders'|'source-last'|'source-last-quick';
export type Pane='panel'|'filter'|'list';

export const isMacPlatform=(platform:string)=>/Mac|iPhone|iPad/.test(platform);

const NON_TEXT_INPUTS=new Set(['button','checkbox','color','file','hidden','image','radio','range','reset','submit']);
/** Whether key presses on `target` type text: text-like inputs, text areas and editable content. */
export function isTextEntry(target:unknown):boolean{
  const element=target as {tagName?:string;type?:string;isContentEditable?:boolean}|null;
  if(!element||typeof element!=='object')return false;
  if(element.isContentEditable)return true;
  const tag=String(element.tagName||'').toUpperCase();
  if(tag==='TEXTAREA')return true;
  return tag==='INPUT'&&!NON_TEXT_INPUTS.has(String(element.type||'text').toLowerCase());
}

/** The letter or digit of a chord, lower case. Option and AltGr change `key`
 * (Option+T is †), and non-Latin layouts report other letters, so those fall
 * back to the physical key. */
export function chordKey(event:Pick<KeyInput,'key'|'code'>):string{
  if(/^[A-Za-z0-9]$/.test(event.key))return event.key.toLowerCase();
  const physical=/^(?:Key([A-Z])|Digit([0-9]))$/.exec(event.code||'');
  return physical?(physical[1]||physical[2]).toLowerCase():event.key.length===1?event.key.toLowerCase():event.key;
}

/** Whether an Alt (Option) chord is an application shortcut rather than typed text. */
export function altChord(event:KeyInput,context:KeyContext):boolean{
  if(!event.altKey||event.metaKey||event.getModifierState?.('AltGraph'))return false;
  if(context.mac)return !context.typing||event.ctrlKey;
  return !event.ctrlKey;
}

/** Window shortcuts that are not part of a module's own key map. */
export function windowShortcut(event:KeyInput,context:KeyContext):WindowShortcut|undefined{
  const key=chordKey(event);
  if(event.altKey){
    if(!altChord(event,context))return undefined;
    if(key==='6')return event.shiftKey?'action':'groups';
    if(event.shiftKey)return undefined;
    if(key==='t')return 'extended';
    if(event.key==='Enter'&&!context.typing)return 'file-reveal';
    return undefined;
  }
  if(event.key==='Tab'&&event.ctrlKey&&!event.metaKey)return event.shiftKey?'pane-previous':'pane-next';
  const primary=context.mac?event.metaKey:event.ctrlKey,other=context.mac?event.ctrlKey:event.metaKey;
  if(!primary||other)return undefined;
  // Ctrl+Shift+Del deletes permanently; ⌘⇧⌫ on Mac keyboards, which have no forward Delete.
  if(event.shiftKey&&(event.key==='Delete'||(context.mac&&event.key==='Backspace')))return context.typing?undefined:'file-delete-permanent';
  // Playlists, tag sources and the per-folder numbering are ignored while typing (Ctrl+I is italic in some inputs).
  if(context.typing&&(event.shiftKey||key==='p'||key==='i'))return undefined;
  if(key==='p')return event.shiftKey?'playlist-folders':'playlist-all';
  if(key==='i')return event.shiftKey?'source-last-quick':'source-last';
  if(event.shiftKey)return key==='k'?'number-folders':undefined;
  if(key==='t')return 'reload';
  if(key==='b')return 'discard';
  return undefined;
}

const PANES:Pane[]=['panel','filter','list'];
/** Ctrl+Tab order: Tag Panel, Filter, File List; Ctrl+Shift+Tab goes backwards. */
export function nextPane(current:Pane|undefined,available:Pane[],backward:boolean):Pane|undefined{
  const order=PANES.filter(pane=>available.includes(pane));
  if(!order.length)return undefined;
  const at=current?order.indexOf(current):-1;
  if(at<0)return backward?order[order.length-1]:order[0];
  return order[(at+(backward?order.length-1:1))%order.length];
}
