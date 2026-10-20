/** Native identities are session-only; workspace backups keep logical paths only. */
export type SourceRef={path:string;token:string|null;error:string|null};
type Prepared={key:string;sources:SourceRef[];acquired:string[]};
type NativeResolution=Prepared&{paths:string[];renamed:{from:string;to:string}[]};
export type SourceResolution={paths:string[];renamed:{from:string;to:string}[];prepared?:string};
type Host={invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>};
export class SourceRootTracker {
  private active=new Map<string,SourceRef>();
  private preparations=new Map<string,Prepared>();
  private releases=new Set<string>();
  private pending:Promise<void>=Promise.resolve();
  private session?:Promise<string>;
  private failure:unknown;
  constructor(private host:Host){}
  private sessionId(){
    return this.session??=this.host.invoke<string>('begin_source_tracking').catch(error=>{this.session=undefined;throw error;});
  }
  private tokens(sources:Iterable<SourceRef>){return new Set([...sources].flatMap(source=>source.token?[source.token]:[]));}
  private release(tokens:Iterable<string>){
    const retained=this.tokens(this.active.values());
    for(const token of tokens)if(!retained.has(token))this.releases.add(token);
    if(!this.releases.size)return;
    this.pending=this.pending.then(async()=>{
      const retained=this.tokens(this.active.values()),tokens=[...this.releases].filter(token=>!retained.has(token));
      if(!tokens.length)return;
      try{
        await this.host.invoke('release_source_roots',{session:await this.sessionId(),tokens});
        tokens.forEach(token=>this.releases.delete(token));this.failure=undefined;
      }catch(error){this.failure=error;}
    });
  }
  private replace(next:Map<string,SourceRef>,unused:Iterable<string>=[]){
    const previous=this.tokens(this.active.values());this.active=next;
    this.release([...previous,...unused]);
  }
  /** Capture before scanning; acceptance only changes local ownership of existing tokens. */
  async prepare(paths:readonly string[]):Promise<string>{
    await this.ready();
    const prepared=await this.host.invoke<Prepared>('prepare_source_roots',{session:await this.sessionId(),paths:[...paths]});
    this.preparations.set(prepared.key,prepared);return prepared.key;
  }
  sync(paths:readonly string[],key?:string){
    const prepared=key?this.preparations.get(key):undefined;
    if(key&&!prepared)throw new Error('Source preparation is no longer available');
    const incoming=new Map(prepared?.sources.map(source=>[source.path,source]));
    const next=new Map(paths.map(path=>[path,incoming.get(path)??this.active.get(path)??{path,token:null,error:null}]));
    if(key)this.preparations.delete(key);
    this.replace(next,prepared?.acquired);
  }
  /** Rename keys without recapturing objects, including aliases and case-only moves. */
  rebase(previous:readonly string[],next:readonly string[]){
    const sources=new Map<string,SourceRef>();
    previous.forEach((path,index)=>{
      const source=this.active.get(path);if(!source)return;
      const target=next[index],existing=sources.get(target);
      if(!existing||(!existing.token&&source.token))sources.set(target,{...source,path:target});
    });
    this.replace(sources);
  }
  discard(key:string){
    const prepared=this.preparations.get(key);if(!prepared)return;
    this.preparations.delete(key);this.release(prepared.acquired);
  }
  async ready(){
    await this.pending;
    if(this.failure&&this.releases.size){this.release([]);await this.pending;}
    if(this.failure)throw this.failure;
  }
  async resolve(paths:readonly string[]):Promise<SourceResolution>{
    await this.ready();
    const sources=paths.map(path=>this.active.get(path)??{path,token:null,error:null});
    const resolved=await this.host.invoke<NativeResolution>('resolve_source_roots',{session:await this.sessionId(),sources});
    this.preparations.set(resolved.key,resolved);
    return{paths:resolved.paths,renamed:resolved.renamed,prepared:resolved.key};
  }
}
