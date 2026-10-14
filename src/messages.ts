// Options → Messages (https://docs.mp3tag.de/customization/options/messages/):
// which confirmations and notifications are shown around operations.
import type { OptionsPage } from './options-ui';

export type MessageKey='after_save'|'after_change'|'after_delete'|'confirm_remove_tags'|'confirm_cut_tags'|'confirm_paste_tags'|'confirm_export'|'confirm_delete_files'|'action_warnings'|'unsaved_changes'|'confirm_import_covers';
export const MESSAGE_DEFAULTS:Record<MessageKey,boolean>={
  after_save:false,after_change:false,after_delete:false,
  confirm_remove_tags:true,confirm_cut_tags:false,confirm_paste_tags:false,confirm_export:false,confirm_delete_files:true,action_warnings:true,unsaved_changes:true,confirm_import_covers:true,
};
/** What a tag source import does with existing artwork when it does not ask. */
export type CoverChoice='preserve'|'replace';
export function coverChoice(values:Record<string,unknown>):CoverChoice{return values.import_covers_choice==='preserve'?'preserve':'replace';}
const check=(key:MessageKey,label:string)=>({kind:'checkbox' as const,path:`pages.messages.${key}`,label});
export const messagesOptionsPage:OptionsPage={id:'messages',label:'Messages',order:15,defaults:{...MESSAGE_DEFAULTS,import_covers_choice:'replace'},
  description:'Choose which messages are shown before or after operations.',controls:[
    {kind:'group',label:'Show a notification',controls:[
      check('after_save','after saving tags'),
      check('after_change','after actions, converters that write files, and Tag → Tag'),
      check('after_delete','after removing tags or tag types')]},
    {kind:'group',label:'Ask for confirmation',controls:[
      check('confirm_remove_tags','at removing tags'),
      check('confirm_cut_tags','at cutting tags'),
      check('confirm_paste_tags','at pasting tags'),
      check('confirm_export','at exporting CSV files and playlists (offers to open the file)'),
      check('confirm_delete_files','at deleting files'),
      check('action_warnings','at warnings from Actions (file errors of the last action or action groups)'),
      check('unsaved_changes','at unsaved changes (closing, reloading, refreshing or discarding pending edits)'),
      check('confirm_import_covers','at importing covers (when files already have cover art)'),
      {kind:'radio',path:'pages.messages.import_covers_choice',label:'Without asking, existing cover art is',enabledWhen:draft=>!messageEnabled(draft.pages.messages??{},'confirm_import_covers'),choices:[['preserve','Preserved'],['replace','Replaced or added to, as chosen in the import']]}]},
    {kind:'note',text:'Without a confirmation, deleted files still go to the Trash or Recycle Bin, and pending tag edits are discarded. Without warnings from Actions, file errors are listed below the file list only.'},
  ]};

export type MessageHost={
  values:()=>Record<string,unknown>;
  ask:(message:string,title:string)=>Promise<boolean>;
  tell:(message:string,title:string)=>Promise<void>;
};
/** Whether a message is enabled, falling back to its default for unsaved or malformed values. */
export function messageEnabled(values:Record<string,unknown>,key:MessageKey){const value=values[key];return typeof value==='boolean'?value:MESSAGE_DEFAULTS[key];}

export class Messages {
  constructor(private host:MessageHost){}
  enabled(key:MessageKey){return messageEnabled(this.host.values(),key);}
  /** Ask when the confirmation is enabled; otherwise proceed. */
  async confirm(key:MessageKey,message:string,title:string){return !this.enabled(key)||this.host.ask(message,title);}
  /** Show a notification when it is enabled. */
  async notify(key:MessageKey,message:string,title:string){if(this.enabled(key))await this.host.tell(message,title);}
}
