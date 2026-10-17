import {Translator,historyLabelText} from './i18n/core';
import {renameFailure,renameFailureText,converterRollbackText,type RenameFailure,type ConverterRollback} from './converter-errors';

export type Cause={kind:'owned';key:string;params:Record<string,string>;children:Record<string,Cause>}|{kind:'external';value:string}|{kind:'owned_text';value:string}|{kind:'rename';value:RenameFailure}|{kind:'joined';parts:Cause[];separator:string};
type Direction='undo'|'redo';
export type BatchDiagnostic=
  |{kind:'action';group:string;index:number;cause:Cause}
  |{kind:'action_validation';group:string;cause:Cause}
  |{kind:'history_changed';label:string;direction:Direction}
  |{kind:'history_failed'|'history_folders_failed';direction:Direction;cause:Cause}
  |{kind:'destination_collision';path:string}
  |{kind:'playlist_line';line:number;cause:Cause}
  |{kind:'source_cover_failed';cause:Cause};
export type PresentedFileError={path:string;error:string;diagnostic?:BatchDiagnostic;diagnosticRaw?:boolean;cause?:Cause;rollback?:ConverterRollback};
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const cause=(value:unknown,depth=0):value is Cause=>{
  if(!object(value)||depth>32)return false;
  return value.kind==='joined'&&typeof value.separator==='string'&&Array.isArray(value.parts)&&value.parts.every(item=>cause(item,depth+1))||(value.kind==='external'||value.kind==='owned_text')&&typeof value.value==='string'||value.kind==='rename'&&renameFailure(value.value)||value.kind==='owned'&&typeof value.key==='string'&&object(value.params)&&Object.values(value.params).every(item=>typeof item==='string')&&object(value.children)&&Object.values(value.children).every(item=>cause(item,depth+1));
};
export const isErrorCause=(value:unknown):value is Cause=>cause(value);
const direction=(value:unknown):value is Direction=>value==='undo'||value==='redo';
const index=(value:unknown):value is number=>typeof value==='number'&&Number.isSafeInteger(value)&&value>=0;
export function batchDiagnostic(value:unknown):value is BatchDiagnostic{
  if(!object(value))return false;
  switch(value.kind){
    case 'action':return typeof value.group==='string'&&index(value.index)&&value.index>0&&cause(value.cause);
    case 'action_validation':return typeof value.group==='string'&&cause(value.cause);
    case 'history_changed':return typeof value.label==='string'&&direction(value.direction);
    case 'history_failed':case 'history_folders_failed':return direction(value.direction)&&cause(value.cause);
    case 'destination_collision':return typeof value.path==='string';
    case 'playlist_line':return index(value.line)&&cause(value.cause);
    case 'source_cover_failed':return cause(value.cause);
    default:return false;
  }
}
const english=new Translator('en');
type Fill=(key:string,params?:Record<string,unknown>)=>string;
/** Render explicit source-owned leaves once; foreign text and raw parameters remain data. */
export function causeText(value:Cause,t:Fill=(key,params)=>english.t(key,params),translate:(text:string)=>string=text=>text):string{
  return value.kind==='joined'?value.parts.map(item=>causeText(item,t,translate)).join(value.separator):value.kind==='owned'?t(value.key,{...value.params,...Object.fromEntries(Object.entries(value.children).map(([name,child])=>[name,causeText(child,t,translate)]))}):value.kind==='owned_text'?translate(value.value):value.kind==='external'?value.value:renameFailureText(value.value,t);
}
/** Format retained producer parts once; authored names and external causes are never looked up. */
export function batchErrorText(value:BatchDiagnostic,t:Fill=(key,params)=>english.t(key,params),translate:(text:string)=>string=text=>text):string{
  const detail=(value:Cause)=>causeText(value,t,translate);
  const verb=(value:Direction)=>t(value==='undo'?'undone':'redone');
  switch(value.kind){
    case 'action':return t('{value} / action {value2}: {error}',{value:value.group,value2:value.index,error:detail(value.cause)});
    case 'action_validation':return t('{value}: {error}',{value:value.group,error:detail(value.cause)});
    case 'history_changed':return t('Changed outside TagGroove after {value}; it was not {verb}',{value:historyLabelText(value.label,t),verb:verb(value.direction)});
    case 'history_failed':return t('It was not {verb}: {error}',{verb:verb(value.direction),error:detail(value.cause)});
    case 'history_folders_failed':return t('Folders and side files could not be {verb}: {error}',{verb:verb(value.direction),error:detail(value.cause)});
    case 'destination_collision':return t('Destination exists or another file maps to {value}',{value:value.path});
    case 'playlist_line':return t('Line {line}: {error}',{line:value.line,error:detail(value.cause)});
    case 'source_cover_failed':return t('The cover image could not be saved: {error}',{error:detail(value.cause)});
  }
}
/** Attach indexed IPC companions before callers spread, filter, reorder or join errors. */
export function normalizeBatchDiagnostics<T>(value:T,expected=false):T{
  if(!expected)return value;
  if(!object(value)||!Array.isArray(value.errors))return value;
  const errors=value.errors;
  if(!Array.isArray(value.error_diagnostics))return expected?{...value,errors:errors.map(error=>object(error)?{...error,diagnosticRaw:true}:error)}:value;
  const counts=new Map<number,number>();
  for(const entry of value.error_diagnostics)if(object(entry)&&index(entry.index))counts.set(entry.index,(counts.get(entry.index)??0)+1);
  const details=new Map<number,BatchDiagnostic>();
  for(const entry of value.error_diagnostics){
    if(!object(entry)||!index(entry.index)||counts.get(entry.index)!==1||!batchDiagnostic(entry.diagnostic))continue;
    const error=errors[entry.index];
    if(object(error)&&typeof error.path==='string'&&typeof error.error==='string'&&batchErrorText(entry.diagnostic)===error.error)details.set(entry.index,entry.diagnostic);
  }
    const invalid=new Set<number>();
  for(const entry of value.error_diagnostics)if(object(entry)&&index(entry.index)&&!details.has(entry.index))invalid.add(entry.index);
  // Unaddressable malformed entries make their association unknowable.
  const unknown=value.error_diagnostics.some(entry=>!object(entry)||!index(entry.index)||entry.index>=errors.length);
  return {...value,errors:errors.map((error,at)=>details.has(at)?{...error,diagnostic:details.get(at)}:invalid.has(at)||unknown?{...error,diagnosticRaw:true}:error)};
}

/** One formatter for retained rows and modal summaries. Plain legacy errors keep their usual translator. */
export function fileErrorText(error:PresentedFileError,t:Fill=(key,params)=>english.t(key,params),translate:(text:string)=>string=text=>text,plain:(text:string)=>string=text=>text):string{
  if(error.diagnostic)return batchErrorText(error.diagnostic,t,translate);
  if(error.cause)return causeText(error.cause,t,translate);
  if(error.rollback)return converterRollbackText(error.rollback,t,translate);
  if(error.diagnosticRaw)return plain(error.error);
  return translate(plain(error.error));
}
type SummaryOptions={paths?:boolean;name?:(path:string)=>string;plain?:(text:string)=>string};
export function fileErrorsText(errors:readonly PresentedFileError[],t:Fill=(key,params)=>english.t(key,params),translate:(text:string)=>string=text=>text,options:SummaryOptions={}):string{
  return errors.map(error=>(options.paths===false?'':`${error.diagnostic?.kind==='history_folders_failed'?error.path:(options.name??(path=>path))(error.path)}: `)+fileErrorText(error,t,translate,options.plain)).join('\n');
}
/** Preserve original rows across an existing Error boundary instead of joining their translated output. */
export class BatchFailure extends Error {
  readonly errors:readonly PresentedFileError[];
  constructor(errors:readonly PresentedFileError[],readonly fallback:string,readonly options:SummaryOptions={}){
    const retained=errors.map(error=>({...error}));
    super(fileErrorsText(retained,undefined,undefined,options)||fallback);
    this.errors=retained;
  }
  text(t:Fill,translate:(text:string)=>string):string{return fileErrorsText(this.errors,t,translate,this.options)||translate(this.fallback);}
}
const modalStops=new WeakMap<HTMLDialogElement,()=>void>();
export function clearFileError(dialog:HTMLDialogElement):void{modalStops.get(dialog)?.();modalStops.delete(dialog);dialog.querySelector('.dialog-error')?.remove();}
/** Existing modal boundary, retaining only structured failures for live language changes. */
export function showFileError(dialog:HTMLDialogElement,error:unknown,t:Fill,translate:(text:string)=>string,onLanguageChange:(listener:()=>void)=>()=>void,plain:(error:unknown)=>string):void{
  modalStops.get(dialog)?.();modalStops.delete(dialog);
  let el=dialog.querySelector<HTMLElement>('.dialog-error');
  if(!el){el=dialog.ownerDocument.createElement('p');el.className='dialog-error';el.setAttribute('role','alert');dialog.querySelector('.dialog-content')!.append(el);}
  if(!(error instanceof BatchFailure)){el.removeAttribute('translate');el.textContent=plain(error);return;}
  el.setAttribute('translate','no');const paragraph=el;
  const refresh=()=>{if(paragraph.isConnected)paragraph.textContent=error.text(t,translate);};refresh();
  const stop=onLanguageChange(refresh),clear=()=>{stop();if(modalStops.get(dialog)===clear)modalStops.delete(dialog);};
  modalStops.set(dialog,clear);dialog.addEventListener('close',clear,{once:true});
}
