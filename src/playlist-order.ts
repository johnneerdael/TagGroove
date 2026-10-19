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
/** Tool and keep-moved folder context for original, unexpanded input roots. */
export function sourceFolders(roots:readonly string[],listed:ReadonlySet<string>):string[]{
  return roots.map(root=>{
    if(!listed.has(root)&&!/\.(m3u8?|pls|cue|xspf|wpl|asx)$/i.test(root))return root;
    const folder=root.replace(/[\\/][^\\/]*$/,'');
    // Root separators distinguish an absolute root from a file or drive-relative path.
    return !folder||/^[A-Za-z]:$/.test(folder)?root.slice(0,folder.length+1):folder;
  });
}
