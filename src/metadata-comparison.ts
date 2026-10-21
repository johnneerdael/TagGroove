import type {ApplyCover, ApplyItem, Fields, SourceFile, SourceRelease, SourceTrack} from './tag-sources';

export type ComparisonScope = 'track' | 'album';
export type ComparisonChoice = {kind:'inherit'} | {kind:'none'} |
  {kind:'source';sourceId:string;revision:number;coverIndex?:number};
export type ComparisonCandidate = {
  sourceId:string;
  revision:number;
  release:SourceRelease;
  /** Source track index for each selected local file; null requires review. */
  tracks:Record<string,number|null>;
};
export type ComparisonPreview = {
  items:ApplyItem[];
  cover:ApplyCover|null;
  missing:{path:string;field:string;sourceId:string}[];
  blocked:{path:string;field:string;reason:string}[];
};
export type ComparisonSaveOutcome = {succeeded:string[];failed:{path:string;error:string}[]};

const commonFields = ['ALBUM','ALBUMARTIST','ARTIST','TITLE','YEAR','TRACK','DISCNUMBER','GENRE','STYLE','BPM','INITIALKEY','COMPOSER','REMIXER','PUBLISHER','LABEL','CATALOGNUMBER','BARCODE','ISRC','COMMENT','LYRICS'];
const hasValue = (values:string[]|undefined):values is string[] => Array.isArray(values) && values.some(value => value.trim().length > 0);
const ownValues = (fields:Fields,name:string) => Object.hasOwn(fields,name) ? fields[name] : undefined;
const equalValues = (a:string[]|undefined,b:string[]|undefined) => JSON.stringify(a || []) === JSON.stringify(b || []);

export function comparisonExisting(file:SourceFile):Fields {
  if (file.fields) return file.fields;
  return Object.fromEntries(Object.entries({TITLE:file.title,ARTIST:file.artist,ALBUM:file.album,ALBUMARTIST:file.album_artist,TRACK:file.track,DISCNUMBER:file.disc})
    .filter(([,value]) => value !== '').map(([name,value]) => [name,[value]]));
}

export function comparisonFields(files:SourceFile[],candidates:ComparisonCandidate[]):string[] {
  const names = new Set(files.flatMap(file => Object.keys(comparisonExisting(file))));
  for (const {release} of candidates) {
    Object.keys(release.fields).forEach(name => names.add(name));
    release.tracks.forEach(track => Object.keys(track.fields).forEach(name => names.add(name)));
  }
  const rank = (name:string) => { const index = commonFields.indexOf(name); return index < 0 ? commonFields.length : index; };
  return [...names].sort((a,b) => rank(a)-rank(b) || a.localeCompare(b));
}

/** Missing track data and an unresolved track identity are different states. */
export function comparisonSourceValue(candidate:ComparisonCandidate,path:string,field:string):{values:string[]|null;unmatched:boolean} {
  const index = candidate.tracks[path];
  const track = index == null ? undefined : candidate.release.tracks[index];
  const trackField = candidate.release.tracks.some(row => Object.hasOwn(row.fields,field));
  if (!track && trackField) return {values:null,unmatched:true};
  const value = track && Object.hasOwn(track.fields,field) ? track.fields[field] : ownValues(candidate.release.fields,field);
  return {values:hasValue(value) ? [...value] : null,unmatched:false};
}

/** The first selected file is the example, even if another track has more data. */
export function comparisonExample(files:SourceFile[],candidate:ComparisonCandidate,field:string):string[]|null {
  return files.length ? comparisonSourceValue(candidate,files[0].path,field).values : null;
}

export function buildComparisonPreview(files:SourceFile[],candidates:ComparisonCandidate[],choices:Record<string,ComparisonChoice>,artwork:ComparisonChoice={kind:'inherit'}):ComparisonPreview {
  const result:ComparisonPreview = {items:[],cover:null,missing:[],blocked:[]};
  const bySource = new Map(candidates.map(candidate => [candidate.sourceId,candidate]));
  const sourceFor = (choice:Extract<ComparisonChoice,{kind:'source'}>,path:string,field:string) => {
    const candidate = bySource.get(choice.sourceId);
    if (!candidate || candidate.revision !== choice.revision) {
      result.blocked.push({path,field,reason:'The source candidate changed. Review the field choice again.'});
      return undefined;
    }
    return candidate;
  };
  if (artwork.kind === 'source') {
    const candidate = sourceFor(artwork,files[0]?.path || '', 'Artwork');
    const cover = candidate?.release.covers[artwork.coverIndex ?? 0];
    if (candidate && !cover) result.blocked.push({path:files[0]?.path || '',field:'Artwork',reason:'The selected source artwork is unavailable. Review the artwork choice.'});
    if (cover) result.cover = {url:cover.url,replace_existing:true,preserve_existing:false,save_as:null};
  }
  for (const file of files) {
    const updates = new Map<string,string[]>();
    const existing = comparisonExisting(file);
    let rawLyrics = false;
    for (const [field,choice] of Object.entries(choices)) {
      if (choice.kind === 'inherit') continue;
      if (choice.kind === 'none') {
        if (ownValues(existing,field)?.length) updates.set(field,[]);
        continue;
      }
      const candidate = sourceFor(choice,file.path,field);
      if (!candidate) continue;
      const {values,unmatched} = comparisonSourceValue(candidate,file.path,field);
      if (unmatched) {
        result.blocked.push({path:file.path,field,reason:'Match this file to the source track before choosing its fields.'});
      } else if (values === null) {
        result.missing.push({path:file.path,field,sourceId:choice.sourceId});
      } else if (!equalValues(ownValues(existing,field),values)) {
        updates.set(field,values);
        if (field === 'LYRICS' && candidate.release.provider === 'lrclib') rawLyrics = true;
      }
    }
    const removeCovers = artwork.kind === 'none' && (file.covers === undefined || file.covers.length > 0);
    if (updates.size || removeCovers || result.cover) {
      result.items.push({path:file.path,updates:Object.fromEntries(updates),...(rawLyrics ? {raw_lyrics:true} : {}),...(removeCovers ? {remove_covers:true} : {})});
    }
  }
  return result;
}

const normalized = (value:string) => value.normalize('NFKD').replace(/\p{M}/gu,'').toLocaleLowerCase().replace(/[^\p{L}\p{N}]+/gu,' ').trim();
const identityField = (name:string) => name === 'ISRC' || /(?:TRACK_?ID|RELEASETRACKID)$/.test(name);
const identities = (fields:Fields) => Object.entries(fields).filter(([name,values]) => identityField(name) && hasValue(values));
const trackHasId = (track:SourceTrack,id:string) => identities(track.fields).some(([,values]) => values.includes(id));

/** Suggest only unambiguous mappings; positions alone are not identity evidence. */
export function comparisonTrackMatches(files:SourceFile[],release:SourceRelease,trackId?:string):Record<string,number|null> {
  const possible = files.map(file => {
    if (files.length === 1 && trackId) return release.tracks.flatMap((track,index) => trackHasId(track,trackId) ? [index] : []);
    const own = identities(comparisonExisting(file));
    const comparable = own.some(([name]) => release.tracks.some(track => hasValue(track.fields[name])));
    if (comparable) return release.tracks.flatMap((track,index) => own.some(([name,values]) => track.fields[name]?.some(value => values.includes(value))) ? [index] : []);
    if (!normalized(file.title) || !normalized(file.artist)) return [];
    return release.tracks.flatMap((track,index) => {
      const durationMatches = !(file.length > 0 && track.length != null && track.length > 0) || Math.abs(file.length-track.length) <= 3;
      return durationMatches && normalized(file.title) === normalized(track.title) && normalized(file.artist) === normalized(track.artist) ? [index] : [];
    });
  });
  const owners = new Map<number,number>();
  possible.forEach(indices => { if (indices.length === 1) owners.set(indices[0],(owners.get(indices[0]) || 0)+1); });
  return Object.fromEntries(files.map((file,index) => {
    const choices = possible[index];
    return [file.path,choices.length === 1 && owners.get(choices[0]) === 1 ? choices[0] : null];
  }));
}

export function comparisonSelectionKey(files:SourceFile[]):string {
  return JSON.stringify(files.map(file => ({path:file.path,title:file.title,artist:file.artist,album:file.album,album_artist:file.album_artist,track:file.track,disc:file.disc,length:file.length,
    fields:Object.fromEntries(Object.entries(comparisonExisting(file)).sort(([a],[b]) => a.localeCompare(b))),
    covers:file.covers?.map(cover => cover.digest)})));
}
