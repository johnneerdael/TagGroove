import type {Provider, SearchMode, SearchResult, SourceFile, SourceRelease, TagSourceCredentials} from './tag-sources';
import type {WebListing, WebResult, WebSearchPage, WebSourceInfo} from './web-sources';
import {comparisonTrackMatches, sameName} from './metadata-comparison';
import type {ComparisonCandidate, ComparisonScope} from './metadata-comparison';

type NativeProvider = Exclude<Provider,'web'>;
type NativeDefinition = {label:string;key?:'fanart';selected?:boolean};
const nativeSources:Record<NativeProvider,NativeDefinition> = {
  musicbrainz:{label:'MusicBrainz'},discogs:{label:'Discogs'},beatport:{label:'Beatport'},traxsource:{label:'Traxsource'},
  itunes:{label:'iTunes'},deezer:{label:'Deezer'},lastfm:{label:'Last.fm'},lrclib:{label:'LRCLIB'},
  acoustid:{label:'AcoustID',selected:false},fanart:{label:'fanart.tv',key:'fanart'},
  freedb:{label:'freedb'},local_freedb:{label:'freedb (local)'},
};
export type ComparisonSource = {id:string;provider:Provider;label:string;setupReason?:string;selected:boolean;web?:WebSourceInfo};
export type ComparisonTerms = {artist:string;title:string;albumArtist:string;album:string;webValues?:Record<string,string[]>};
export type ComparisonQuery = {mode:SearchMode;artist:string;title:string;album:string;text:string;page:number;lengths:number[];files:{path:string;title:string;artist:string;album:string;track:string;disc:string;length:number}[]};
export type NeedsInput = {kind:'needs-input';reason:string;inputs:{key:string;label:string;value:string}[]};
export type ComparisonSearchResult = SearchResult & {webResult?:WebResult};
export const comparisonResultKey = (result:ComparisonSearchResult) => `${result.id}|${result.track_id||''}`;
type FingerprintMatches = Record<string,{recording:string;acoustid:string}[]>;
export type ComparisonSourceState = {
  source:ComparisonSource;
  status:'needs-setup'|'needs-input'|'searching'|'loading'|'needs-match'|'ready'|'empty'|'failed'|'cancelled';
  results:ComparisonSearchResult[];page:number;total:number|null;canMore:boolean;
  candidate?:ComparisonCandidate;error?:unknown;input?:NeedsInput;confirmed?:boolean;fingerprints?:FingerprintMatches;
  /** Which of `results` the candidate was loaded from (see `comparisonResultKey`), so the edition list can show it. */
  resultKey?:string;
};
type SearchHost = {
  invoke:<T>(command:string,args?:Record<string,unknown>)=>Promise<T>;
  evaluate:(path:string,template:string)=>Promise<string>;
  nextTicket:()=>number;
  onSourceState:(state:ComparisonSourceState)=>void;
};

export function comparisonSources(credentials:TagSourceCredentials|null,listing:WebListing,setup:{localFreedbAvailable?:boolean}={}):ComparisonSource[] {
  const sources:ComparisonSource[] = Object.entries(nativeSources).map(([provider,definition]) => {
    let setupReason:string|undefined;
    if (definition.key) {
      const key=definition.key;
      if (!credentials) setupReason='API key settings are unavailable. Check Tag Sources → API keys.';
      else if (!credentials[`${key}_key_set`] && !credentials[`${key}_built_in`]) setupReason=`${definition.label} needs an API key.`;
    }
    if (provider==='lastfm' && credentials?.lastfm_available===false) setupReason='This build has no Last.fm application key.';
    if (provider==='acoustid' && credentials?.acoustid_available===false) setupReason='This build has no AcoustID application key.';
    if (provider==='discogs' && !credentials) setupReason='API key settings are unavailable. Check Tag Sources → API keys.';
    if (provider==='discogs' && credentials && !credentials.discogs_app_key && !credentials.discogs_token_set) setupReason='Discogs needs application credentials or a personal access token.';
    if (provider==='local_freedb' && setup.localFreedbAvailable===false) setupReason='Choose the local freedb database folder in Options.';
    return {id:`native:${provider}`,provider:provider as NativeProvider,label:definition.label,selected:!setupReason && definition.selected!==false,...(setupReason?{setupReason}:{})};
  });
  for (const web of listing.sources) sources.push({id:`web:${web.id}`,provider:'web',label:web.name,selected:true,web});
  for (const problem of listing.problems) sources.push({id:`web-error:${problem.file}`,provider:'web',label:problem.file,selected:false,setupReason:problem.error});
  return sources;
}

const needsInput = (reason:string,key:string,label:string,value=''):NeedsInput => ({kind:'needs-input',reason,inputs:[{key,label,value}]});
export function comparisonQuery(source:ComparisonSource,scope:ComparisonScope,files:SourceFile[],terms:ComparisonTerms):ComparisonQuery|NeedsInput {
  const signatures=files.map(({path,title,artist,album,track,disc,length})=>({path,title,artist,album,track,disc,length:Number.isFinite(length)?length:0}));
  const query:ComparisonQuery={mode:scope==='track'?'artist_title':'artist_album',artist:scope==='track'?terms.artist:terms.albumArtist,title:terms.title,album:terms.album,text:'',page:0,lengths:signatures.map(file=>file.length),files:signatures};
  switch(source.provider) {
    case 'web': return needsInput('Review the search inputs declared by this installed source.','web','Source search inputs');
    case 'lrclib':
      if(scope==='track') {
        if(!terms.title.trim())return needsInput('Enter the track title to search for lyrics.','title','Track title');
        if(signatures[0])signatures[0]={...signatures[0],artist:terms.artist,title:terms.title,album:terms.album||signatures[0].album};
      }
      return {...query,mode:'files'};
    case 'acoustid': return {...query,mode:'fingerprint'};
    case 'fanart':
      if (!terms.album.trim()) return needsInput('Artwork lookup needs an album title.','album','Album title');
      return {...query,mode:'artist_album',artist:terms.albumArtist};
    case 'freedb':
      if (scope==='track') return needsInput('freedb needs the full album/disc context to identify a single track.','disc','Album/disc context');
      if (!terms.album.trim()) return needsInput('Enter the album title for the disc search.','album','Album title');
      return {...query,mode:'artist_album'};
    case 'local_freedb':
      if (!terms.album.trim()) return needsInput('Enter the album title or a known local disc entry.','album','Album title');
      return {...query,mode:'text',artist:terms.albumArtist,text:[terms.albumArtist,terms.album].filter(value=>value.trim()).join(' ')};
    case 'lastfm':
      if (scope==='album' && !terms.albumArtist.trim() && terms.album.trim()) return {...query,mode:'text',text:terms.album};
      break;
    case 'beatport':case 'traxsource': if (scope==='album') query.mode='release';break;
  }
  if (scope==='track' && !terms.title.trim()) return needsInput('Enter the track title to search.','title','Track title');
  if (scope==='album' && !terms.album.trim()) return needsInput('Enter the album title to search.','album','Album title');
  return query;
}

/** One ticket permits parallel sources without cancelling peers on the shared HTTP channels. */
export class ComparisonSearchSession {
  readonly states=new Map<string,ComparisonSourceState>();
  private generation=0;
  private ticket=0;
  private revision=0;
  private requests=new Map<string,number>();
  private active=0;
  private waiting:(()=>void)[]=[];
  private files:SourceFile[]=[];
  private terms:ComparisonTerms={artist:'',title:'',albumArtist:'',album:''};
  private sourceTerms=new Map<string,ComparisonTerms>();
  private scope:ComparisonScope='track';
  constructor(private host:SearchHost) {}

  private publish(state:ComparisonSourceState) { this.states.set(state.source.id,state);this.host.onSourceState(state); }
  private valid(sourceId:string,generation:number,request:number) { return this.generation===generation && this.requests.get(sourceId)===request; }
  private nextRequest(sourceId:string) { const request=(this.requests.get(sourceId)||0)+1;this.requests.set(sourceId,request);return request; }
  private async request<T>(command:string,args:Record<string,unknown>,valid:()=>boolean):Promise<T> {
    if (this.active>=3) await new Promise<void>(resolve=>this.waiting.push(resolve));
    else this.active++;
    try {
      if (!valid()) throw new Error('Source search cancelled.');
      return await this.host.invoke<T>(command,args);
    } finally {
      const next=this.waiting.shift();
      if(next)next();else this.active--;
    }
  }

  async search(sources:ComparisonSource[],scope:ComparisonScope,files:SourceFile[],terms:ComparisonTerms):Promise<void> {
    if (this.ticket) this.cancel();
    this.generation++;
    this.ticket=this.host.nextTicket();
    this.scope=scope;this.files=structuredClone(files);this.terms=structuredClone(terms);
    this.sourceTerms.clear();
    this.states.clear();
    for (const source of sources) this.publish({source,status:source.setupReason?'needs-setup':'searching',results:[],page:0,total:null,canMore:false});
    await Promise.all(sources.map(source=>this.searchSource(source.id,false)));
  }

  private async searchSource(sourceId:string,append:boolean):Promise<void> {
    const previous=this.states.get(sourceId);
    if (!previous || previous.source.setupReason) return;
    const source=previous.source,generation=this.generation,request=this.nextRequest(sourceId),ticket=this.ticket;
    const terms=this.sourceTerms.get(sourceId)||this.terms;
    const valid=()=>this.valid(sourceId,generation,request);
    const page=append?previous.page+1:0;
    this.publish({...previous,status:'searching',error:undefined,input:undefined,...(!append?{candidate:undefined,results:[],page:0,canMore:false}:{} )});
    try {
      let results:ComparisonSearchResult[],total:number|null;
      if (source.web) {
        const supplied=terms.webValues?.[source.id];
        const values=supplied || await Promise.all(source.web.search_by.map(field=>field.format && this.files[0]?this.host.evaluate(this.files[0].path,field.format):Promise.resolve('')));
        if (!valid()) return;
        if (values.length!==source.web.search_by.length || !supplied && values.some(value=>!value.trim())) {
          const input:NeedsInput={kind:'needs-input',reason:'Enter the additional search inputs required by this source.',inputs:source.web.search_by.map((field,index)=>({key:String(index),label:field.label,value:values[index]||''}))};
          this.publish({...previous,status:'needs-input',input});return;
        }
        const found=await this.request<WebSearchPage>('web_source_search',{source:source.web.id,values,ticket},valid);
        if (!valid()) return;
        results=found.results.map(row=>({id:row.url,title:row.values.filter(Boolean).join(' · ')||row.url,artist:'',year:'',country:'',label:'',catalog_number:'',format:'',track_count:null,score:null,barcode:'',thumb:row.cover,url:row.preview||row.url,webResult:row}));
        total=results.length;
      } else {
        const query=comparisonQuery(source,this.scope,this.files,terms);
        if ('kind' in query) { if(valid())this.publish({...previous,status:'needs-input',input:query});return; }
        const found=await this.request<{results:SearchResult[];total:number|null}>('tag_source_search',{provider:source.provider,query:{...query,page},ticket},valid);
        if (!valid()) return;
        results=found.results;total=found.total;
      }
      const current=this.states.get(sourceId)!;
      const all=append?[...previous.results,...results]:results;
      const canMore=!source.web && source.provider!=='itunes' && (total===null?results.length>=50:all.length<total) && results.length>0;
      const state:ComparisonSourceState={...current,results:all,total,page,canMore,status:all.length?(previous.candidate?previous.status==='ready'?'ready':'needs-match':'needs-match'):'empty'};
      this.publish(state);
      // Start from the one exact match when there is exactly one, wherever the provider ranked it.
      const exact=this.exactResults(source,results);
      if (!append && results.length) await this.loadResult(sourceId,exact.length===1?exact[0]:results[0],false);
    } catch(error) {
      if(valid())this.publish({...this.states.get(sourceId)!,status:'failed',error});
    }
  }

  /** The results that are the album (or track) that was asked for, by artist and title. */
  private exactResults(source:ComparisonSource,results:ComparisonSearchResult[]):ComparisonSearchResult[] {
    if(source.web)return [];
    const terms=this.sourceTerms.get(source.id)||this.terms,track=this.scope==='track';
    const title=track?terms.title:terms.album,artist=track?terms.artist:terms.albumArtist;
    return results.filter(row=>sameName(row.title,title)&&sameName(row.artist,artist));
  }
  /** Several results, but exactly one is the one asked for: no editions to choose between. Only when every
   *  reported result is loaded: another edition could be waiting on a page that has not been fetched. */
  private onlyExactResult(state:ComparisonSourceState,result:ComparisonSearchResult):boolean {
    if(state.canMore)return false;
    const exact=this.exactResults(state.source,state.results);
    return exact.length===1&&comparisonResultKey(exact[0])===comparisonResultKey(result);
  }

  async load(sourceId:string,result:ComparisonSearchResult):Promise<void> { await this.loadResult(sourceId,result,true); }
  private async loadResult(sourceId:string,result:ComparisonSearchResult,confirmed:boolean):Promise<void> {
    const state=this.states.get(sourceId);if(!state)return;
    if(!this.ticket)this.ticket=this.host.nextTicket();
    const source=state.source,generation=this.generation,request=this.nextRequest(sourceId),ticket=this.ticket;
    const valid=()=>this.valid(sourceId,generation,request);
    this.publish({...state,status:'loading',error:undefined});
    try {
      const release=structuredClone(await this.request<SourceRelease>(source.web?'web_source_release':'tag_source_release',source.web?
        {source:source.web.id,url:result.id,previewUrl:result.webResult?.preview||null,ticket}:
        {provider:source.provider,input:result.id,ticket,files:this.files.map(({path,title,artist,album,track,disc,length})=>({path,title,artist,album,track,disc,length}))},valid));
      if(!valid())return;
      const trackId=result.track_id || (result.id.startsWith('track:')?result.id.slice(6):undefined);
      // Last.fm's track lengths are user-submitted and often far off, so for it the title and artist decide.
      let tracks=comparisonTrackMatches(this.files,release,trackId,{ignoreDuration:source.provider==='lastfm'}),fingerprints:FingerprintMatches|undefined;
      if(source.provider==='lrclib' && result.id.startsWith('files:') && release.tracks.length===this.files.length) tracks=Object.fromEntries(this.files.map((file,index)=>[file.path,index]));
      if(source.provider==='acoustid') {
        fingerprints=await this.request<FingerprintMatches>('acoustid_file_matches',{},valid);
        if(!valid())return;
        tracks=Object.fromEntries(this.files.map(file=>{
          const own=fingerprints![file.path]||[];
          const indices=release.tracks.flatMap((row,index)=>own.some(match=>row.fields.MUSICBRAINZ_TRACKID?.includes(match.recording))?[index]:[]);
          return [file.path,indices.length===1?indices[0]:null];
        }));
        const owners=new Map<number,number>();
        for(const index of Object.values(tracks))if(index!==null)owners.set(index,(owners.get(index)||0)+1);
        for(const [path,index]of Object.entries(tracks))if(index!==null && owners.get(index)!==1)tracks[path]=null;
        this.annotateFingerprints(release,tracks,fingerprints);
      }
      const candidate:ComparisonCandidate={sourceId,revision:++this.revision,release,tracks};
      const matched=this.files.every(file=>tracks[file.path]!=null);
      const sharedOnly=!release.tracks.length && (release.covers.length>0 || Object.keys(release.fields).length>0);
      const accepted=confirmed || state.results.length===1 && (state.total===null || state.total<=1) || this.onlyExactResult(state,result);
      this.publish({...this.states.get(sourceId)!,candidate,fingerprints,confirmed:accepted,resultKey:comparisonResultKey(result),status:(matched||sharedOnly)&&accepted?'ready':'needs-match'});
    }catch(error){if(valid())this.publish({...this.states.get(sourceId)!,status:'failed',error});}
  }

  private annotateFingerprints(release:SourceRelease,tracks:Record<string,number|null>,matches:FingerprintMatches) {
    release.tracks.forEach(track=>delete track.fields.ACOUSTID_ID);
    for(const [path,index]of Object.entries(tracks)) {
      if(index==null)continue;
      const track=release.tracks[index];
      const own=matches[path]?.find(match=>track.fields.MUSICBRAINZ_TRACKID?.includes(match.recording));
      if(own)track.fields.ACOUSTID_ID=[own.acoustid];
    }
  }

  match(sourceId:string,path:string,index:number|null):void {
    const state=this.states.get(sourceId),current=state?.candidate;
    if(!state || !current || !this.files.some(file=>file.path===path))return;
    if(index!==null && (!Number.isInteger(index) || !current.release.tracks[index]))return;
    this.nextRequest(sourceId);
    const tracks={...current.tracks,[path]:index};
    if(index!==null)for(const [other,assigned]of Object.entries(tracks))if(other!==path && assigned===index)tracks[other]=null;
    const release=structuredClone(current.release);
    if(state.fingerprints)this.annotateFingerprints(release,tracks,state.fingerprints);
    const candidate={...current,tracks,release,revision:++this.revision};
    this.publish({...state,candidate,confirmed:true,status:this.files.every(file=>tracks[file.path]!=null)?'ready':'needs-match'});
  }

  async retry(sourceId:string,terms?:ComparisonTerms):Promise<void> {
    if(!this.ticket)this.ticket=this.host.nextTicket();
    if(terms && this.states.has(sourceId))this.sourceTerms.set(sourceId,structuredClone(terms));
    await this.searchSource(sourceId,false);
  }
  async searchOne(source:ComparisonSource,scope:ComparisonScope,files:SourceFile[],terms:ComparisonTerms):Promise<void> {
    this.scope=scope;this.files=structuredClone(files);if(!this.states.size)this.terms=structuredClone(terms);
    if(!this.states.has(source.id))this.publish({source,status:'searching',results:[],page:0,total:null,canMore:false});
    await this.retry(source.id,terms);
  }
  async more(sourceId:string):Promise<void> {
    if(!this.states.get(sourceId)?.canMore)return;
    if(!this.ticket)this.ticket=this.host.nextTicket();
    await this.searchSource(sourceId,true);
  }
  cancel():void {
    this.generation++;
    if(this.ticket)void this.host.invoke('tag_source_cancel',{ticket:this.host.nextTicket()}).catch(()=>undefined);
    for(const state of this.states.values())if(state.status==='searching'||state.status==='loading')this.publish({...state,status:'cancelled'});
    this.ticket=0;
  }
}
