/**
 * Keeps the backend's copy of the filter fields in step with the file list.
 * The backend caches each file's filter fields per window session, so a query
 * carries only the files whose token (entry, pending edits, source state,
 * evaluated column values) changed since the previous query, and the list
 * order only when it changed.
 */
type Fields=Record<string,string[]>;
export type FilterColumn={name:string;template:string};
export type FilterFile={path:string;fields:Fields;raw_fields:Fields;columns:(string|null)[]};
export type FilterRequest={session:string;order:string[]|null;updates:FilterFile[];columns:FilterColumn[]};
export type FilterSyncHost={
  /** Listed paths in display order. */
  paths:()=>string[];
  columns:()=>FilterColumn[];
  /** Everything a file's filter fields depend on; values compare with `===`. */
  token:(path:string,columns:FilterColumn[])=>unknown[];
  file:(path:string,columns:FilterColumn[])=>FilterFile;
};
/** The backend's answer when it lacks a listed file (for example after a restart). */
export const filterCacheMiss='OPENTAG_FILTER_CACHE_MISS';
const same=(a:unknown[],b:unknown[])=>a.length===b.length&&a.every((value,index)=>value===b[index]);

export class FilterSync{
  private synced=new Map<string,unknown[]>();
  private order:string[]=[];
  private columnsKey='';
  constructor(private host:FilterSyncHost,readonly session=crypto.randomUUID()){}
  /** The next request, assuming the backend applies it. Call `reset` if it fails. */
  request():FilterRequest{
    const columns=this.host.columns(),columnsKey=JSON.stringify(columns);
    if(columnsKey!==this.columnsKey){this.synced.clear();this.columnsKey=columnsKey;}
    const paths=this.host.paths(),orderChanged=!same(paths,this.order);
    if(orderChanged){const listed=new Set(paths);for(const path of[...this.synced.keys()])if(!listed.has(path))this.synced.delete(path);this.order=paths;}
    const updates:FilterFile[]=[];
    for(const path of paths){
      const token=this.host.token(path,columns),previous=this.synced.get(path);
      if(previous&&same(previous,token))continue;
      updates.push(this.host.file(path,columns));this.synced.set(path,token);
    }
    return{session:this.session,order:orderChanged?paths:null,updates,columns};
  }
  /** Forget what the backend holds; the next request resends everything. */
  reset(){this.synced.clear();this.order=[];this.columnsKey='';}
}
