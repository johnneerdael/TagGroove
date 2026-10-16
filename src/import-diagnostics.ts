import {Translator} from './i18n/core';
import {BatchFailure,causeText,isErrorCause,type Cause,type PresentedFileError} from './batch-errors';
const english=new Translator('en');
type Fill=(key:string,params?:Record<string,unknown>)=>string;
const object=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
/** Validate indexed producer companions against unchanged English before using them. */
function retainedCauses(sources:string[],metadata:unknown):(Cause|undefined)[]{
  if(!Array.isArray(metadata))return sources.map(()=>undefined);
  const counts=new Map<number,number>();
  for(const item of metadata)if(object(item)&&Number.isSafeInteger(item.index))counts.set(item.index as number,(counts.get(item.index as number)??0)+1);
  return sources.map((source,index)=>{
    const item=metadata.find(item=>object(item)&&item.index===index);
    return counts.get(index)===1&&object(item)&&isErrorCause(item.cause)&&causeText(item.cause)===source?structuredClone(item.cause):undefined;
  });
}
export type MtaImportIssues={issues:{index:number;type_id:string;reason:string}[];issue_diagnostics?:unknown};
export function mtaImportFailure(imported:MtaImportIssues):BatchFailure{
  const causes=retainedCauses(imported.issues.map(item=>item.reason),imported.issue_diagnostics);
  return new BatchFailure(imported.issues.map((issue,index)=>({path:'',error:english.t('Action {index}: {reason}',{index:issue.index+1,reason:issue.reason}),...(causes[index]?{cause:{kind:'owned',key:'Action {index}: {reason}',params:{index:String(issue.index+1)},children:{reason:causes[index]!}} as Cause}:{diagnosticRaw:true})})), '',{paths:false});
}
/** Scoped native import errors retain their legacy English message for safe fallback. */
export function importFailure(error:unknown,legacy=false):BatchFailure|undefined{
  if(!object(error)||typeof error.message!=='string'||!('import_failure'in error)){
    if(!legacy)return;const message=error instanceof Error?error.message:String(error);
    return new BatchFailure([{path:'',error:message,diagnosticRaw:true}],message,{paths:false});
  }
  const valid=isErrorCause(error.import_failure)&&causeText(error.import_failure)===error.message;
  return new BatchFailure([{path:'',error:error.message,...(valid?{cause:error.import_failure as Cause}:{diagnosticRaw:true})}],error.message,{paths:false});
}
export type ColumnImportNotes={skipped:string[];skipped_diagnostics?:unknown};
/** The only non-string global status: retain column-import provenance until another assignment replaces it. */
export class ColumnImportStatus {
  private readonly notes:ColumnImportNotes;
  private readonly rows:PresentedFileError[];
  constructor(readonly count:number,readonly path:string,notes:ColumnImportNotes,private readonly notices:readonly string[]=[]){
    this.notes=structuredClone(notes);const causes=retainedCauses(notes.skipped,notes.skipped_diagnostics);
    this.rows=notes.skipped.map((error,index)=>({path:'',error,...(causes[index]?{cause:causes[index]}:{diagnosticRaw:true})}));
  }
  private key(){return this.notes.skipped.length?(this.count===1?'Imported {length} Mp3tag column from {path}; skipped {length2}: {join}':'Imported {length} Mp3tag columns from {path}; skipped {length2}: {join}'):(this.count===1?'Imported {length} Mp3tag column from {path}':'Imported {length} Mp3tag columns from {path}');}
  toString(){return english.t(this.key(),{length:this.count,path:this.path,length2:this.notes.skipped.length,join:this.notes.skipped.join('; ')})+this.notices.map(notice=>`. ${notice}`).join('');}
  text(t:Fill,translate:(text:string)=>string){
    const joined=this.rows.map(row=>row.cause?causeText(row.cause,t,translate):row.error).join('; ');
    return t(this.key(),{length:this.count,path:this.path,length2:this.notes.skipped.length,join:joined})+this.notices.map(notice=>`. ${translate(notice)}`).join('');
  }
  withNotice(notice:string){return new ColumnImportStatus(this.count,this.path,this.notes,[...this.notices,notice]);}
}
