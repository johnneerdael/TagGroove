// Options → General, Appearance, Networking and Updates behaviour
// (https://docs.mp3tag.de/customization/options/{general,appearance,networking,updates}/).
// Kept free of runtime imports so tests can load it on its own.
import type { OptionsDraft, OptionsPage } from './options-ui';

/** Fallbacks for malformed values; they equal the page defaults in options-ui. */
const GENERAL_FALLBACKS={auto_select_next:false,autocomplete:true,select_added:false,keep_moved:true};
export type GeneralOptions=typeof GENERAL_FALLBACKS;
export type Grid='off'|'alternate'|'lines';
export type AppearanceOptions={toolbarScale:number;uiScale:number;grid:Grid};
export type ProxyMode='system'|'none'|'manual';
export type NetworkOptions={timeout:number;proxy:ProxyMode;proxy_url:string};
export type UpdateOptions={check_on_startup:boolean;include_beta:boolean;release_notes:boolean;last_version:string};
export type Release={version:string;name:string;url:string;prerelease:boolean;published:string|null};
export type UpdateCheck={current:string;latest:Release|null;newer:boolean;skipped_beta?:string|null};

export const NETWORK_DEFAULTS:NetworkOptions={timeout:30,proxy:'system',proxy_url:''};
export const UPDATE_DEFAULTS:UpdateOptions={check_on_startup:false,include_beta:false,release_notes:true,last_version:''};
export const TIMEOUT_RANGE=[5,300] as const;
export const RELEASES_PAGE='https://github.com/johnneerdael/opentag/releases/';

const bool=(value:unknown,fallback:boolean)=>typeof value==='boolean'?value:fallback;
export function generalOptions(values:Record<string,unknown>):GeneralOptions{
  return Object.fromEntries(Object.entries(GENERAL_FALLBACKS).map(([key,fallback])=>[key,bool(values[key],fallback)])) as GeneralOptions;
}
const scale=(value:unknown)=>{const percent=Number(value??100);return Number.isFinite(percent)&&percent>=100&&percent<=200?percent/100:1;};
export function appearanceOptions(values:Record<string,unknown>):AppearanceOptions{
  const grid=values.grid==='alternate'||values.grid==='lines'?values.grid:'off';
  return{toolbarScale:scale(values.toolbar_scale),uiScale:scale(values.ui_scale),grid};
}
export function updateOptions(values:Record<string,unknown>):UpdateOptions{
  return{check_on_startup:bool(values.check_on_startup,false),include_beta:bool(values.include_beta,false),release_notes:bool(values.release_notes,true),last_version:typeof values.last_version==='string'?values.last_version:''};
}

/**
 * Inline completion for a combo box: the first candidate that starts with the
 * typed text (ignoring case) and is longer than it. The typed characters are
 * kept as entered; only the completed rest comes from the candidate.
 */
export function completion(typed:string,candidates:Iterable<string>):string|undefined{
  if(!typed)return undefined;
  const lower=typed.toLocaleLowerCase();
  for(const candidate of candidates)if(candidate.length>typed.length&&candidate.toLocaleLowerCase().startsWith(lower))return typed+candidate.slice(typed.length);
  return undefined;
}
/**
 * Complete `input` after the user typed at its end: the suggested rest is
 * selected, so typing on replaces it and Backspace removes it.
 */
export function completeInput(input:HTMLInputElement,event:Event,candidates:()=>Iterable<string>){
  const kind=(event as InputEvent).inputType;
  if(!kind||!kind.startsWith('insert')||kind==='insertFromPaste'||input.selectionStart!==input.value.length)return false;
  const value=completion(input.value,candidates());if(!value)return false;
  const typed=input.value.length;input.value=value;input.setSelectionRange(typed,value.length);return true;
}

/** Whether `path` is inside one of the list's folders. */
export function insideFolders(path:string,folders:string[]){
  return folders.some(folder=>{const base=folder.replace(/[\\/]+$/,'');return path===base||path.startsWith(`${base}/`)||path.startsWith(`${base}\\`);});
}

/** The release notes page of a version, for "Display release notes for new versions". */
export function releaseNotesUrl(version:string){return `${RELEASES_PAGE}tag/v${encodeURIComponent(version)}`;}
/** First start of a different version than the one that last ran (not a fresh install). */
export function showReleaseNotes(options:UpdateOptions,current:string){return options.release_notes&&!!options.last_version&&options.last_version!==current&&/^\d/.test(current);}
export function updateMessage(check:UpdateCheck){
  const beta=check.skipped_beta?` Development build ${check.skipped_beta} is published; include development builds in Options → Updates to check against it.`:'';
  if(!check.latest)return{title:'No releases found',text:`OpenTag ${check.current} is installed. No stable release has been published.${beta}`};
  if(!check.newer)return{title:'OpenTag is up to date',text:`OpenTag ${check.current} is the newest ${check.latest.prerelease?'version, including beta builds':'release'}.${beta}`};
  return{title:'An update is available',text:`${check.latest.name} (${check.latest.version}${check.latest.prerelease?', beta':''}) is available. You have OpenTag ${check.current}.`};
}

export const networkingOptionsPage:OptionsPage={id:'networking',label:'Networking',order:58,defaults:{...NETWORK_DEFAULTS},
  description:'Connections to tag sources and the update check.',controls:[
    {kind:'number',path:'pages.networking.timeout',label:'Timeout',min:TIMEOUT_RANGE[0],max:TIMEOUT_RANGE[1],suffix:'seconds'},
    {kind:'radio',path:'pages.networking.proxy',label:'Proxy',choices:[['system','Use the system proxy settings'],['none','No proxy'],['manual','Use this HTTP proxy']]},
    {kind:'text',path:'pages.networking.proxy_url',label:'Proxy address',placeholder:'host:port',enabledWhen:draft=>draft.pages.networking?.proxy==='manual'},
    {kind:'note',text:'Proxy user names and passwords are not supported, because options are stored and exported as plain text.'}],
  validate:draft=>networkProblem(draft)};
export function networkProblem(draft:Pick<OptionsDraft,'pages'>):string|undefined{
  const values={...NETWORK_DEFAULTS,...draft.pages.networking} as NetworkOptions;
  if(!Number.isInteger(values.timeout)||values.timeout<TIMEOUT_RANGE[0]||values.timeout>TIMEOUT_RANGE[1])return `Enter a timeout between ${TIMEOUT_RANGE[0]} and ${TIMEOUT_RANGE[1]} seconds.`;
  if(values.proxy==='manual'){
    const text=String(values.proxy_url??'').trim();
    let url:URL;try{url=new URL(text.includes('://')?text:`http://${text}`);}catch{return 'Enter the proxy as host:port or http://host:port.';}
    if(!['http:','https:'].includes(url.protocol)||!url.hostname||url.pathname.length>1||url.search)return 'Enter the proxy as host:port or http://host:port.';
    if(url.username||url.password)return 'Proxy user names and passwords are not supported.';
  }
  return undefined;
}
export function updatesOptionsPage(checkNow:()=>void|Promise<void>):OptionsPage{
  return{id:'updates',label:'Updates',order:59,defaults:{...UPDATE_DEFAULTS},
    description:'OpenTag compares its version with the releases published on GitHub. Updates are never downloaded or installed automatically.',controls:[
      {kind:'checkbox',path:'pages.updates.check_on_startup',label:'Check for updates on startup'},
      {kind:'checkbox',path:'pages.updates.include_beta',label:'Also include development builds (beta)'},
      {kind:'checkbox',path:'pages.updates.release_notes',label:'Display release notes for new versions'},
      {kind:'button',id:'check-updates',label:'Check now…',run:()=>checkNow()}]};
}
