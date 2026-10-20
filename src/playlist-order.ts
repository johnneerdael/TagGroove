// Session input order is independent of saved column sort preferences and the
// mutable manual file-list order. Exact row paths include CUE/chapter identities.
export type ListOrderMode='columns'|'manual'|'playlist';
export function mergeSourceOrder(previous:readonly string[],incoming:readonly string[],replace:boolean):string[]{
  return [...new Set(replace?incoming:[...previous,...incoming])];
}
/** Rank only listed rows; Remove from list does not reread or reinsert a member. */
export function rankSourceRows<T extends {path:string}>(rows:readonly T[],order:readonly string[]):T[]{
  const ranks=new Map(order.map((path,index)=>[path,index]));
  return [...rows].sort((a,b)=>(ranks.get(a.path)??Infinity)-(ranks.get(b.path)??Infinity));
}
export type SourceRootKind='file'|'directory'|'unknown';
export type SourceRootInfo={kind:SourceRootKind;resolved:string|null};
/** Tool and keep-moved folder context for original, unexpanded input roots. */
export function sourceFolders(roots:readonly string[],kinds:Readonly<Record<string,SourceRootKind>>,resolved:Readonly<Record<string,string>>={}):string[]{
  return roots.flatMap(root=>{
    const path=resolved[root]??root;
    if(kinds[root]==='directory')return [path];
    if(kinds[root]!=='file')return [];
    const folder=path.replace(path.startsWith('/')&&!path.startsWith('//')?/\/[^/]*$/:/[\\/][^\\/]*$/,'');
    // Root separators distinguish an absolute root from a file or drive-relative path.
    return [!folder||/^[A-Za-z]:$/.test(folder)?path.slice(0,folder.length+1):folder];
  });
}
