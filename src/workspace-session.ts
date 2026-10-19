import type {ProfileWorkspaceV2} from './profile-migration';
/** Admit only saved local row identities; never revive provider grants or tag drafts. */
export function savedWorkspace(value:unknown):ProfileWorkspaceV2|null {
  if(!value||typeof value!=='object')return null;
  const w=value as ProfileWorkspaceV2;
  const strings=(v:unknown):v is string[]=>Array.isArray(v)&&v.every(p=>typeof p==='string'&&!!p);
  const local=(p:string)=>!p.includes('\0')&&(p.startsWith('/')||/^[A-Za-z]:[\\/]/.test(p)||p.startsWith('\\\\'));
  if(w.version!==2||!strings(w.paths)||!Array.isArray(w.selected)||!w.selected.every(p=>typeof p==='string')||!Array.isArray(w.roots)||!w.roots.every(p=>typeof p==='string'&&local(p))||!w.paths.every(local))return null;
  if(w.root_kinds!==undefined&&(!Array.isArray(w.root_kinds)||w.root_kinds.length!==w.roots.length||!w.root_kinds.every(kind=>['file','directory','unknown'].includes(kind))))return null;
  const listed=new Set(w.paths);
  if(new Set(w.selected).size!==w.selected.length||listed.size!==w.paths.length||!w.selected.every(p=>listed.has(p)))return null;
  if(!['columns','manual','playlist'].includes(w.list_order)||!(w.source_order===null||strings(w.source_order)&&w.source_order.length===w.paths.length&&new Set(w.source_order).size===w.paths.length&&w.source_order.every(p=>listed.has(p))))return null;
  if(typeof w.location!=='string'||typeof w.location_is_data!=='boolean'||typeof w.filter!=='string'||!(w.filter_draft===null||typeof w.filter_draft==='string')||!(w.anchor===null||typeof w.anchor==='string'&&listed.has(w.anchor)))return null;
  if(![w.working_directory,w.playlist].every(p=>p===null||typeof p==='string'&&local(p)))return null;
  return w;
}

/** Track membership and insertion order without walking rows after every render. */
export class WorkspaceRows<T> extends Map<string,T>{
  revision=0;
  override set(path:string,value:T){if(!this.has(path))this.revision++;return super.set(path,value);}
  override delete(path:string){const removed=super.delete(path);if(removed)this.revision++;return removed;}
  override clear(){if(this.size)this.revision++;super.clear();}
}
