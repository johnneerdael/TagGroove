import {Translator} from './i18n/core';

type CopyFailure={kind:'external';error:string}|{kind:'source_changed'};
type RenameFailure={kind:'external';error:string}|{kind:'only_regular'|'no_name'|'no_temporary_name'}|{kind:'cannot_create'|'cannot_rename';path:string;error:string}|{kind:'copy_rollback';error:CopyFailure;rollback:string}|{kind:'source_changed_rollback';rollback:string}|{kind:'remove_original_rollback';error:string;rollback:string}|{kind:'alias_rollback';source:string;destination:string;error:string;rollback:string};
type ConverterMessage={kind:'message'|'external';message:string}|{kind:'rename';message:RenameFailure};
type Recovery={kind:'retained';path:string}|{kind:'failed';error:string}|{kind:'unavailable'};
export type ConverterRollback={operation:'rename'|'text_import';error:ConverterMessage;failures:{path:string;error:ConverterMessage;recovery:Recovery|null}[]};
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
const rename=(value:unknown):value is RenameFailure=>{
  if(!object(value))return false;
  const strings=(...keys:string[])=>keys.every(key=>typeof value[key]==='string');
  switch(value.kind){
    case 'external':return strings('error');
    case 'only_regular':case 'no_name':case 'no_temporary_name':return true;
    case 'cannot_create':case 'cannot_rename':return strings('path','error');
    case 'copy_rollback':return strings('rollback')&&object(value.error)&&(value.error.kind==='source_changed'||value.error.kind==='external'&&typeof value.error.error==='string');
    case 'source_changed_rollback':return strings('rollback');
    case 'remove_original_rollback':return strings('error','rollback');
    case 'alias_rollback':return strings('source','destination','error','rollback');
    default:return false;
  }
};
const message=(value:unknown):value is ConverterMessage=>object(value)&&((value.kind==='message'||value.kind==='external')&&typeof value.message==='string'||value.kind==='rename'&&rename(value.message));
const recovery=(value:unknown):value is Recovery|null=>value===null||object(value)&&(value.kind==='unavailable'||value.kind==='retained'&&typeof value.path==='string'||value.kind==='failed'&&typeof value.error==='string');
/** The two reviewed converter commands keep ordinary failures as strings. */
export function converterRollbackError(error:unknown):ConverterRollback|undefined{
  const value=object(error)?error.converter_rollback:undefined;
  return object(value)&&(value.operation==='rename'||value.operation==='text_import')&&message(value.error)&&Array.isArray(value.failures)&&value.failures.every(item=>object(item)&&typeof item.path==='string'&&message(item.error)&&recovery(item.recovery))?value as ConverterRollback:undefined;
}
const english=new Translator('en');
type Fill=(key:string,params?:Record<string,unknown>)=>string;
/** Translate each owned part once; paths and opaque IO/Debug details never enter lookup. */
export function converterRollbackText(failure:ConverterRollback,t:Fill=(key,params)=>english.t(key,params),translate:(text:string)=>string=text=>text){
  const renameText=(value:RenameFailure):string=>{
    switch(value.kind){
      case 'external':return value.error;
      case 'only_regular':return t('Only regular files can be renamed');
      case 'cannot_create':return t('Cannot create {value}: {e}',{value:value.path,e:value.error});
      case 'cannot_rename':return t('Cannot rename {value}: {e}',{value:value.path,e:value.error});
      case 'copy_rollback':return t('{error}; destination rollback: {value}',{error:value.error.kind==='source_changed'?t('Source changed while copying; original was retained'):value.error.error,value:value.rollback});
      case 'source_changed_rollback':return t('Source changed before move completed; original was retained; destination rollback: {cleanup}',{cleanup:value.rollback});
      case 'remove_original_rollback':return t('Cannot remove original: {e}; destination rollback: {cleanup}',{e:value.error,cleanup:value.rollback});
      case 'no_name':return t('The source has no name');
      case 'no_temporary_name':return t('No free temporary name for the case-only rename');
      case 'alias_rollback':return t('Cannot rename {value} to {value2}: {error}; rollback: {rollback}',{value:value.source,value2:value.destination,error:value.error,rollback:value.rollback});
    }
  };
  const text=(value:ConverterMessage)=>value.kind==='rename'?renameText(value.message):value.kind==='external'?value.message:translate(value.message);
  const items=failure.failures.map(item=>{
    let shown=`${item.path}: ${text(item.error)}`;
    if(item.recovery?.kind==='retained')shown+=`; ${t('original backup retained at {path}',{path:item.recovery.path})}`;
    else if(item.recovery?.kind==='failed')shown+=`; ${t('could not retain recovery copy: {value}',{value:item.recovery.error})}`;
    else if(item.recovery?.kind==='unavailable')shown+=`; ${t('no recovery copy is available')}`;
    return shown;
  });
  // Keep the existing catalog keys; their value is an already rendered summary.
  return t(failure.operation==='rename'?'{error}; rename rollback {value}':'{error}; text import rollback {value}',{error:text(failure.error),value:items.length?items.join('; '):t('completed')});
}
