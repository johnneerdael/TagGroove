import type { OptionsDraft, OptionsPage } from './options-ui';
import type { WebResult, WebSearchPage, WebSourceInfo } from './web-sources';
type Fields = Record<string, string[]>;
export type Provider = 'musicbrainz'|'discogs'|'freedb'|'local_freedb'|'web';
export type SearchMode = 'artist_album'|'text'|'release_id'|'barcode'|'catalog_number'|'toc'|'disc_id';
export type SearchResult = {id:string;title:string;artist:string;year:string;country:string;label:string;catalog_number:string;format:string;track_count:number|null;score:number|null;barcode:string;thumb:string|null;url:string};
export type SearchPage = {results:SearchResult[];total:number|null};
export type SourceTrack = {position:string;disc:number;number:number;title:string;artist:string;length:number|null;fields:Fields};
export type SourceCover = {url:string;thumb:string|null;kind:string;width:number|null;height:number|null};
export type SourceRelease = {provider:Provider;id:string;url:string;title:string;artist:string;fields:Fields;tracks:SourceTrack[];covers:SourceCover[];more_covers:boolean};
/** Selected file values as shown in the file list (native field casing already resolved). */
export type SourceFile = {path:string;name:string;title:string;artist:string;album:string;album_artist:string;track:string;disc:string;length:number};
export type ApplyItem = {path:string;updates:Fields};
/** `save_as` is Utils → Save image to disc: the file name format from Options → Tag Sources. */
export type ApplyCover = {url:string;replace_existing:boolean;save_as:string|null;preserve_existing?:boolean};
export type TagSourceCredentials = {discogs_token_set:boolean};
export type FieldRow = {name:string;value:string;perTrack:boolean;editable:boolean};
/** Covers-only sources (Mp3tag's Tag Sources → Covers) write artwork and no fields; `web` is the script for the `web` provider. */
export type OpenOptions = {coversOnly?:boolean;web?:WebSourceInfo;title?:string};

const localFreedbPage = 'tag-sources-local-freedb';
/**
 * Options → Tag Sources with its freedb and Local freedb pages (Mp3tag:
 * Options › Tag Sources › freedb / Local freedb). Values persist under
 * `pages.<id>`; the backend reads them through the shared settings.
 */
export function tagSourcesOptionsPages(pickFolder:()=>Promise<string|null>):OptionsPage[] {
  const directoryPath = `pages.${localFreedbPage}.directory`;
  const directory = (draft:OptionsDraft) => String((draft.pages[localFreedbPage] as Record<string,unknown>|undefined)?.directory ?? '');
  const setDirectory = (draft:OptionsDraft, value:string) => { draft.pages[localFreedbPage] = {...draft.pages[localFreedbPage],directory:value}; };
  return [
    {id:'tag-sources',label:'Tag Sources',order:60,defaults:{cover_filename:'folder'},controls:[
      {kind:'text',path:'pages.tag-sources.cover_filename',label:'Default file name for cover art',placeholder:'folder'},
      {kind:'note',text:'Used by Save image to file in the tag source dialog. Placeholders such as %albumartist% - %album% are filled from the first written file of each folder, and the image type adds the extension.'},
      {kind:'select',path:'pages.tag-sources.musicbrainz_cover_size',label:'MusicBrainz cover size',choices:[['500','500 pixels (default)'],['250','250 pixels'],['1200','1200 pixels'],['original','Original size']]},
      {kind:'note',text:'Cover Art Archive images are downloaded in this size; original images can be very large. Set the Discogs personal access token with Tag Sources → Discogs access token….'},
      {kind:'checkbox',path:'pages.tag-sources.web_debug',label:'Write debug output for web sources'},
      {kind:'note',text:'Each search and album lookup of a web source (.src) writes the downloaded page and a step-by-step trace of its parser script to the debug folder inside the sources folder (Tag Sources → Open sources folder).'},
    ]},
    {id:'tag-sources-freedb',label:'freedb',parent:'tag-sources',order:1,
      description:'The CDDB server that Tag Sources → freedb asks for disc IDs. Restore page defaults switches back to gnudb.',
      defaults:{protocol:'https',address:'gnudb.gnudb.org',path:'/~cddb/cddb.cgi',port:'',email:'',comment_disc_id:false,disc_id_field:false},
      controls:[
        {kind:'group',label:'freedb server',controls:[
          {kind:'select',path:'pages.tag-sources-freedb.protocol',label:'Protocol',choices:[['https','HTTPS'],['http','HTTP']]},
          {kind:'text',path:'pages.tag-sources-freedb.address',label:'Address',placeholder:'gnudb.gnudb.org'},
          {kind:'text',path:'pages.tag-sources-freedb.path',label:'Path',placeholder:'/~cddb/cddb.cgi'},
          {kind:'text',path:'pages.tag-sources-freedb.port',label:'Port',placeholder:'Default port of the protocol',maxLength:5},
          {kind:'text',path:'pages.tag-sources-freedb.email',label:'Email address',placeholder:'Only sent in the server handshake'},
        ]},
        {kind:'note',text:'gnudb only answers applications that its operator has registered. If gnudb refuses OpenTag, use another CDDB server or a local freedb folder.'},
        {kind:'group',label:'General',controls:[
          {kind:'checkbox',path:'pages.tag-sources-freedb.comment_disc_id',label:'Copy disc ID to COMMENT field'},
          {kind:'checkbox',path:'pages.tag-sources-freedb.disc_id_field',label:'Copy disc ID to DISCID field'},
        ]},
      ],
      validate:draft => {
        const page = (draft.pages['tag-sources-freedb'] || {}) as Record<string,unknown>;
        const address = String(page.address ?? '').trim(), port = String(page.port ?? '').trim();
        if (address && !/^[A-Za-z0-9.-]+$/.test(address)) return 'Enter the freedb server address as a host name, for example gnudb.gnudb.org.';
        if (port && !(/^\d+$/.test(port) && Number(port) >= 1 && Number(port) <= 65535)) return 'The freedb port must be a number from 1 to 65535.';
        return undefined;
      }},
    {id:localFreedbPage,label:'Local freedb',parent:'tag-sources',order:2,
      description:'A folder of xmcd files from a freedb snapshot, with one subfolder per category (blues, classical, rock, …). Both the one-file-per-disc and the Windows (xxtoyy) layouts are read. Discs are looked up by disc ID; text search is not available.',
      defaults:{directory:''},
      controls:[{kind:'custom',id:'local-freedb-directory',paths:[directoryPath],
        render:draft => `<div class="field"><span>LOCAL FREEDB FOLDER</span><div class="action-input"><input id="local-freedb-directory" aria-label="Local freedb folder" value="${html(directory(draft))}" placeholder="Not set" autocomplete="off" spellcheck="false"><button type="button" id="local-freedb-pick">Choose folder…</button></div></div>`,
        bind:(root,draft,refresh) => {
          root.querySelector<HTMLInputElement>('#local-freedb-directory')!.addEventListener('input',event => setDirectory(draft,(event.target as HTMLInputElement).value));
          root.querySelector<HTMLButtonElement>('#local-freedb-pick')!.addEventListener('click',async () => { const picked = await pickFolder(); if (picked) { setDirectory(draft,picked); refresh(); } });
        }}]},
  ];
}

export const isFreedb = (provider:Provider) => provider === 'freedb' || provider === 'local_freedb';
export const providerName = (provider:Provider) => ({musicbrainz:'MusicBrainz',discogs:'Discogs',freedb:'freedb',local_freedb:'freedb (local)',web:'Web source'})[provider];
const fieldOrder = ['TITLE','ARTIST','ALBUM','ALBUMARTIST','YEAR','TRACK','DISCNUMBER','GENRE','STYLE','COMPOSER','PUBLISHER','CATALOGNUMBER','BARCODE','ISRC'];
/** OpenTag's multi-value separator, as in the tag panel. */
const MULTI = ' \\\\ ';
const html = (value:unknown) => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
export const duration = (seconds:number|null) => { if (seconds == null || !Number.isFinite(seconds)) return ''; const total = Math.round(seconds); return `${Math.floor(total/60)}:${String(total%60).padStart(2,'0')}`; };

/** The most common album and album artist of the selection seed the search. */
export function searchPrefill(files:SourceFile[]) {
  const common = (values:string[]) => {
    const counts = new Map<string,number>();
    for (const value of values.map(v => v.split(' \\\\ ')[0].trim()).filter(Boolean)) counts.set(value,(counts.get(value)||0)+1);
    return [...counts].sort((a,b) => b[1]-a[1])[0]?.[0] || '';
  };
  return {artist:common(files.map(f => f.album_artist || f.artist)), album:common(files.map(f => f.album))};
}

/**
 * The freedb disc ID of the files laid out as a CD in file list order: two
 * seconds of lead-in, then each track's length in 1/75 s frames.
 */
export function discToc(lengths:number[]) {
  if (!lengths.length || lengths.length > 99 || lengths.some(l => !Number.isFinite(l) || l <= 0)) return null;
  const offsets:number[] = [];
  let position = 150;
  for (const length of lengths) { offsets.push(position); position += Math.max(1,Math.round(length*75)); }
  const digits = (n:number) => String(n).split('').reduce((sum,d) => sum+Number(d),0);
  const sum = offsets.reduce((total,offset) => total+digits(Math.floor(offset/75)),0);
  const seconds = (Math.floor(position/75)-Math.floor(offsets[0]/75)) & 0xffff;
  const id = (((sum % 0xff) << 24) | (seconds << 8) | lengths.length) >>> 0;
  return {id:id.toString(16).padStart(8,'0'),seconds:Math.floor(position/75),tracks:lengths.length};
}

export const normalizeTitle = (value:string) => value.normalize('NFKD').replace(/[̀-ͯ]/g,'').toLowerCase().replace(/[^\p{L}\p{N}]+/gu,' ').trim();
export function similarity(a:string, b:string) {
  a = normalizeTitle(a); b = normalizeTitle(b);
  if (!a || !b) return 0;
  if (a === b) return 1;
  let previous = Array.from({length:b.length+1},(_,i) => i);
  for (let i = 1; i <= a.length; i++) {
    const row = [i];
    for (let j = 1; j <= b.length; j++) row[j] = Math.min(previous[j]+1,row[j-1]+1,previous[j-1]+(a[i-1]===b[j-1]?0:1));
    previous = row;
  }
  return 1 - previous[b.length]/Math.max(a.length,b.length);
}
const leadingNumber = (value:string) => { const match = /^\s*0*(\d+)/.exec(value); return match ? Number(match[1]) : 0; };
/** A filename such as `01 - Artist - Title.flac` contains the track title as whole words. */
function titleScore(file:SourceFile, track:SourceTrack) {
  const title = normalizeTitle(track.title);
  let best = 0;
  for (const candidate of [file.title, file.name.replace(/\.[^.]+$/,'')]) {
    const score = similarity(candidate,track.title), words = normalizeTitle(candidate);
    best = Math.max(best, title.length >= 3 && words && ` ${words} `.includes(` ${title} `) ? 0.9 + score/10 : score);
  }
  return best;
}

type Pair = {index:number;path:string;score:number};
/** Assign the best pairs first; each file and track is used once. */
function assign(slots:(string|null)[], used:Set<string>, pairs:Pair[]) {
  pairs.sort((a,b) => b.score-a.score || a.index-b.index);
  for (const pair of pairs) if (!slots[pair.index] && !used.has(pair.path)) { slots[pair.index] = pair.path; used.add(pair.path); }
}
/** Remaining files fill the empty slots in selection order; extra files follow the tracks. */
function fillInOrder(slots:(string|null)[], used:Set<string>, files:SourceFile[]) {
  const rest = files.filter(file => !used.has(file.path));
  let next = 0;
  for (let index = 0; index < slots.length && next < rest.length; index++) if (!slots[index]) slots[index] = rest[next++].path;
  return [...slots,...rest.slice(next).map(file => file.path)];
}

/**
 * Pair files with tracks: first by disc/track number, then by title similarity,
 * then in selection order. Index i of the result belongs to track i; slots
 * after the last track hold files that will not be written.
 */
export function autoMatch(tracks:SourceTrack[], files:SourceFile[]):(string|null)[] {
  const slots:(string|null)[] = tracks.map(() => null), used = new Set<string>();
  const multiDisc = new Set(tracks.map(t => t.disc)).size > 1, byNumber = new Map<string,SourceFile[]>();
  for (const file of files) {
    const number = leadingNumber(file.track), disc = leadingNumber(file.disc) || (multiDisc ? 0 : 1);
    if (number && disc) byNumber.set(`${disc}:${number}`,[...(byNumber.get(`${disc}:${number}`)||[]),file]);
  }
  tracks.forEach((track,index) => {
    const candidates = byNumber.get(`${track.disc}:${track.number}`);
    if (candidates?.length === 1 && !used.has(candidates[0].path)) { slots[index] = candidates[0].path; used.add(candidates[0].path); }
  });
  const pairs:Pair[] = [];
  tracks.forEach((track,index) => { if (!slots[index]) for (const file of files) if (!used.has(file.path)) { const score = titleScore(file,track); if (score >= 0.6) pairs.push({index,path:file.path,score}); } });
  assign(slots,used,pairs);
  return fillInOrder(slots,used,files);
}

/** Utils → Suggest sorting — Title: the closest file name or title for each track. */
export function suggestByTitle(tracks:SourceTrack[], files:SourceFile[]):(string|null)[] {
  const slots:(string|null)[] = tracks.map(() => null), used = new Set<string>(), pairs:Pair[] = [];
  tracks.forEach((track,index) => { for (const file of files) { const score = titleScore(file,track); if (score > 0) pairs.push({index,path:file.path,score}); } });
  assign(slots,used,pairs);
  return fillInOrder(slots,used,files);
}

/** Utils → Suggest sorting — Length: the file whose length is closest to each track's. */
export function suggestByLength(tracks:SourceTrack[], files:SourceFile[]):(string|null)[] {
  const slots:(string|null)[] = tracks.map(() => null), used = new Set<string>(), pairs:Pair[] = [];
  tracks.forEach((track,index) => { if (track.length != null) for (const file of files) if (Number.isFinite(file.length) && file.length > 0) pairs.push({index,path:file.path,score:-Math.abs(track.length-file.length)}); });
  assign(slots,used,pairs);
  return fillInOrder(slots,used,files);
}

export function moveSlot(slots:(string|null)[], index:number, delta:number) {
  const target = index+delta, result = [...slots];
  if (index < 0 || index >= slots.length || target < 0 || target >= slots.length) return result;
  [result[index],result[target]] = [result[target],result[index]];
  return result;
}

/** Move one file to `target` (Alt+Page Up / Alt+Page Down), keeping the order of the others. */
export function moveSlotTo(slots:(string|null)[], index:number, target:number) {
  const result = [...slots];
  if (index < 0 || index >= slots.length || target < 0 || target >= slots.length) return result;
  const [moved] = result.splice(index,1); result.splice(target,0,moved);
  return result;
}

/** Every field the release offers, album-level values first in Mp3tag's usual order. */
export function fieldRows(release:SourceRelease):FieldRow[] {
  const names = new Set([...Object.keys(release.fields),...release.tracks.flatMap(t => Object.keys(t.fields))]);
  const rank = (name:string) => { const index = fieldOrder.indexOf(name); return index < 0 ? fieldOrder.length : index; };
  return [...names].sort((a,b) => rank(a)-rank(b) || a.localeCompare(b)).map(name => {
    const perTrack = release.tracks.some(t => name in t.fields);
    if (!perTrack) return {name,value:release.fields[name].join(MULTI),perTrack,editable:true};
    const values = release.tracks.map(t => (t.fields[name] || release.fields[name] || []).join(MULTI));
    const distinct = new Set(values);
    return distinct.size === 1 ? {name,value:values[0],perTrack,editable:true} : {name,value:`${values.find(Boolean) || ''} … (${distinct.size} values)`,perTrack,editable:false};
  });
}

/** Values typed in the review dialog; ` \\ ` separates multiple values. */
export const splitValues = (text:string) => text.split(/\s*\\\\\s*/).map(value => value.trim()).filter(Boolean);
/**
 * Adjust a value before saving. Without `track`, an album field changes for
 * the release and a per-track field changes on every track; with `track`,
 * only that track's value changes.
 */
export function editField(release:SourceRelease, name:string, text:string, track?:number) {
  const values = splitValues(text);
  const set = (target:SourceTrack) => { target.fields[name] = [...values]; if (name === 'TITLE') target.title = values[0] ?? ''; };
  if (track != null) { if (release.tracks[track]) set(release.tracks[track]); return; }
  if (release.tracks.some(t => name in t.fields)) release.tracks.forEach(set);
  else release.fields[name] = values;
}

/** Per-file updates for matched tracks, limited to the checked fields. */
export function buildApplyItems(release:SourceRelease, slots:(string|null)[], enabled:ReadonlySet<string>):ApplyItem[] {
  return release.tracks.flatMap((track,index) => {
    const path = slots[index];
    if (!path) return [];
    const updates:Fields = {};
    for (const [name,values] of Object.entries({...release.fields,...track.fields})) if (enabled.has(name) && values.length) updates[name] = [...values];
    return [{path,updates}];
  });
}

/** Ticket numbers only grow, including across webview reloads, so the backend can drop superseded work. */
let lastTicket = Date.now()*1000;
export const nextTicket = () => ++lastTicket;

type Invoke = <T>(command:string, args?:Record<string,unknown>) => Promise<T>;
type TagSourcesHost = {
  invoke:Invoke;files:()=>SourceFile[];reason:()=>string;
  apply:(items:ApplyItem[], cover:ApplyCover|null)=>Promise<void>;
  /** Saved Options → Tag Sources values (`pages.tag-sources`). */
  options:()=>Record<string,unknown>;
  /** Save dialog for Utils → Extract cover. */
  pickSave:(name:string)=>Promise<string|null>;
  /** A format string evaluated for one file (web sources fill their search fields with it). */
  evaluate:(path:string, template:string)=>Promise<string>;
};
const storage = {
  get(key:string) { try { return window.localStorage.getItem(key); } catch { return null; } },
  set(key:string, value:string) { try { window.localStorage.setItem(key,value); } catch { /* Preferences are a convenience. */ } },
};

export class TagSourcesUI {
  private showing = false;
  constructor(private host:TagSourcesHost) {}
  get lastProvider():Provider { return storage.get('opentag.tagSources.last') === 'discogs' ? 'discogs' : 'musicbrainz'; }
  /** The web source used last, when it was a web source (Ctrl+I runs it again). */
  get lastWeb():string|null { const last = storage.get('opentag.tagSources.last'); return last?.startsWith('web:') ? last.slice(4) : null; }

  /** The Discogs token stays in its own private file, never in the shared Options store. */
  async token() {
    if (this.showing) return;
    this.showing = true;
    let credentials:TagSourceCredentials;
    try { credentials = await this.host.invoke<TagSourceCredentials>('load_tag_source_settings'); } catch (error) { this.showing = false; throw error; }
    const dialog = document.createElement('dialog');
    dialog.className = 'tag-source-dialog ts-token-dialog';
    dialog.setAttribute('aria-label','Discogs access token');
    dialog.innerHTML = `<header>Discogs access token</header><div class="dialog-content">
      <label class="field"><span>PERSONAL ACCESS TOKEN</span><input id="ts-token" type="password" autocomplete="off" spellcheck="false" placeholder="${credentials.discogs_token_set ? 'Saved. Enter a new token to replace it.' : 'Not set'}"></label>
      <label class="action-check"><input id="ts-token-clear" type="checkbox" ${credentials.discogs_token_set ? '' : 'disabled'}> Remove the saved token</label>
      <p>Create a token on Discogs under Settings → Developers. Release lookups and searches also work without one at a lower rate limit; with a token, searches show cover thumbnails. The token is stored privately in OpenTag's configuration folder and is not shown again.</p></div>
      <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ts-save-token">Save</button></div>`;
    document.body.append(dialog);
    const error = (message:unknown) => { let p = dialog.querySelector('.dialog-error'); if (!p) { p = document.createElement('p'); p.className = 'dialog-error'; p.setAttribute('role','alert'); dialog.querySelector('.dialog-content')!.append(p); } p.textContent = String(message); };
    const token = dialog.querySelector<HTMLInputElement>('#ts-token')!, clear = dialog.querySelector<HTMLInputElement>('#ts-token-clear')!, save = dialog.querySelector<HTMLButtonElement>('#ts-save-token')!;
    const sync = () => { token.disabled = clear.checked; save.disabled = !clear.checked && !token.value.trim(); };
    clear.addEventListener('change',sync); token.addEventListener('input',sync);
    dialog.querySelector('[data-close]')!.addEventListener('click',() => dialog.close());
    dialog.addEventListener('close',() => { this.showing = false; dialog.remove(); });
    save.addEventListener('click',async () => {
      save.disabled = true;
      try { await this.host.invoke('save_tag_source_settings',{update:{discogs_token:clear.checked ? '' : token.value.trim()}}); dialog.close(); }
      catch (e) { error(e); sync(); }
    });
    sync();
    dialog.showModal();
    token.focus();
  }

  async open(provider:Provider, options:OpenOptions = {}) {
    if (this.showing) return;
    const reason = this.host.reason();
    if (reason) throw new Error(reason);
    const web = provider === 'web' ? options.web : undefined;
    if (provider === 'web' && !web) throw new Error('This web source is no longer installed.');
    this.showing = true;
    const coversOnly = !!options.coversOnly && !isFreedb(provider) && !web, freedb = isFreedb(provider);
    if (!coversOnly && !freedb) storage.set('opentag.tagSources.last',web ? `web:${web.id}` : provider);
    const files = this.host.files(), name = web ? options.title || web.name : coversOnly ? `${providerName(provider)} cover art` : providerName(provider), prefill = searchPrefill(files);
    // [SearchBy] fields start with their format strings evaluated for the first selected file.
    let webValues:string[] = [], webColumns:string[] = [], webRows:WebResult[] = [];
    if (web) webValues = await Promise.all(web.search_by.map(field => files[0] ? this.host.evaluate(files[0].path,field.format).catch(() => '') : Promise.resolve('')));
    const toc = discToc(files.map(f => f.length));
    const disabledKey = `opentag.tagSources.disabled.${provider}`, saveCoverKey = 'opentag.tagSources.saveCover';
    let results:SearchResult[] = [], total:number|null = null, lastQuery:Record<string,unknown>|null = null, chosen = -1;
    let release:SourceRelease|undefined, slots:(string|null)[] = [], enabled = new Set<string>(), trackField = 'TITLE';
    let searchTicket = 0, releaseTicket = 0, imageTicket = 0, working = false, coverIndex = 0;
    const dialog = document.createElement('dialog');
    dialog.className = 'tag-source-dialog';
    dialog.setAttribute('aria-label',name);
    document.body.append(dialog);
    const $ = <T extends Element = HTMLElement>(selector:string) => dialog.querySelector<T>(selector)!;
    const status = (message:string) => { const el = dialog.querySelector('.ts-status'); if (el) el.textContent = message; };
    const error = (message:unknown) => {
      const text = String(message).replace(/^Error: /,'');
      if (text.startsWith('Cancelled:')) return;
      let p = dialog.querySelector('.dialog-error');
      if (!p) { p = document.createElement('p'); p.className = 'dialog-error'; p.setAttribute('role','alert'); dialog.querySelector('.dialog-content')!.append(p); }
      p.textContent = text;
    };
    const clearError = () => dialog.querySelector('.dialog-error')?.remove();
    const loadImage = async (url:string|null|undefined, image:HTMLImageElement) => {
      const ticket = imageTicket = nextTicket();
      image.hidden = true; image.removeAttribute('src');
      if (!url) return;
      try { const data = await this.host.invoke<string>('tag_source_image',{url,ticket}); if (ticket === imageTicket && dialog.open) { image.src = data; image.hidden = false; } }
      catch { /* Missing artwork leaves the preview empty. */ }
    };
    const preview = (url:string) => url.startsWith('https://') || (web && url.startsWith('http://')) ? `<button type="button" class="ts-preview" data-url="${html(url)}" title="Open the release page in your browser">Preview</button>` : '';
    dialog.addEventListener('cancel',event => { if (working) event.preventDefault(); });
    dialog.addEventListener('close',() => { this.showing = false; void this.host.invoke('tag_source_cancel',{ticket:nextTicket()}).catch(() => undefined); dialog.remove(); });
    dialog.addEventListener('click',event => {
      const button = (event.target as Element).closest<HTMLButtonElement>('.ts-preview');
      if (button) void this.host.invoke('tag_source_open_page',{url:button.dataset.url}).catch(error);
    });
    // Mp3tag's review shortcuts: Ctrl+E/Ctrl+D enable or disable every field,
    // F2 edits the focused field, Alt+↑/↓ moves the focused file, Alt+Page Up/Down
    // moves it to the beginning or end of the list.
    dialog.addEventListener('keydown',event => {
      if (!release || working) return;
      const target = event.target as HTMLElement, key = event.key.toLowerCase();
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && (key === 'e' || key === 'd') && !coversOnly) {
        event.preventDefault(); event.stopPropagation(); toggleAll(key === 'e'); return;
      }
      if (event.key === 'F2') {
        const input = target.closest('tr')?.querySelector<HTMLInputElement>('.ts-value,.ts-track-value');
        if (input) { event.preventDefault(); input.focus(); input.select(); }
        return;
      }
      if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
        const row = target.closest<HTMLTableRowElement>('.ts-matches tr[data-index]');
        if (row) { event.preventDefault(); move(Number(row.dataset.index),event.key === 'ArrowUp' ? -1 : 1,target.matches('.ts-track-value') ? '.ts-track-value' : undefined); }
      }
      if (event.altKey && (event.key === 'PageUp' || event.key === 'PageDown')) {
        const row = target.closest<HTMLTableRowElement>('.ts-matches tr[data-index]');
        if (row) { event.preventDefault(); moveTo(Number(row.dataset.index),event.key === 'PageUp' ? 0 : slots.length-1,target.matches('.ts-track-value') ? '.ts-track-value' : undefined); }
      }
    });

    const freedbId = (r:SearchResult) => { const [category,disc] = r.id.split('/'); return {category:category || '',disc:disc || ''}; };
    const rowAttributes = (i:number) => `tabindex="${i === Math.max(chosen,0) ? 0 : -1}" data-index="${i}" aria-selected="${i === chosen}" class="${i === chosen ? 'selected' : ''}"`;
    const resultMarkup = () => {
      if (!results.length) return '';
      if (web) return `<table class="ts-table ts-results" aria-label="Search results"><thead><tr>${webColumns.map(column => `<th>${html(column)}</th>`).join('')}</tr></thead><tbody>${results.map((_,i) => `<tr ${rowAttributes(i)}>${(webRows[i]?.values || []).map(value => `<td title="${html(value)}">${html(value)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
      const mb = provider === 'musicbrainz';
      const head = freedb ? '<th>Artist</th><th>Title</th><th class="ts-num">Year</th><th class="ts-wide">Genre</th><th class="ts-wide">Category</th><th class="ts-wide">Disc ID</th><th class="ts-num ts-wide">Tracks</th>'
        : `${mb ? '<th class="ts-num ts-wide">Score</th>' : ''}<th>Artist</th><th>Title</th><th class="ts-num">Year</th><th class="ts-wide">Country</th><th class="ts-wide">Label</th><th class="ts-wide">Cat. no.</th><th class="ts-wide">Format</th>${mb ? '<th class="ts-num ts-wide">Tracks</th>' : ''}`;
      const cells = (r:SearchResult) => freedb
        ? `<td title="${html(r.artist)}">${html(r.artist)}</td><td title="${html(r.title)}">${html(r.title)}</td><td class="ts-num">${html(r.year)}</td><td class="ts-wide">${html(r.format)}</td><td class="ts-wide">${html(freedbId(r).category)}</td><td class="ts-wide">${html(freedbId(r).disc)}</td><td class="ts-num ts-wide">${r.track_count ?? ''}</td>`
        : `${mb ? `<td class="ts-num ts-wide">${r.score ?? ''}</td>` : ''}<td title="${html(r.artist)}">${html(r.artist)}</td><td title="${html(r.title)}">${html(r.title)}</td><td class="ts-num">${html(r.year)}</td><td class="ts-wide">${html(r.country)}</td><td class="ts-wide" title="${html(r.label)}">${html(r.label)}</td><td class="ts-wide" title="${html(r.catalog_number)}">${html(r.catalog_number)}</td><td class="ts-wide" title="${html(r.format)}">${html(r.format)}</td>${mb ? `<td class="ts-num ts-wide">${r.track_count ?? ''}</td>` : ''}`;
      const more = total != null && results.length < total ? `<p class="ts-more"><button type="button" id="ts-more">More results</button><span>${results.length} of ${total} shown</span></p>` : '';
      return `<table class="ts-table ts-results" aria-label="Search results"><thead><tr>${head}</tr></thead><tbody>${results.map((r,i) => `<tr tabindex="${i === Math.max(chosen,0) ? 0 : -1}" data-index="${i}" aria-selected="${i === chosen}" class="${i === chosen ? 'selected' : ''}">${cells(r)}</tr>`).join('')}</tbody></table>${more}`;
    };
    const chosenMarkup = () => {
      const r = results[chosen];
      if (!r) return '';
      if (web) return `<img class="ts-thumb" alt="Release artwork" hidden><div><strong>${html(r.title)}</strong>${r.url ? `<span class="ts-url">${html(r.url)} ${preview(r.url)}</span>` : ''}</div>`;
      const details = freedb ? [r.year,r.format,`Disc ID ${freedbId(r).disc}`] : [r.year,r.country,r.label,r.catalog_number,r.format,r.barcode && `Barcode ${r.barcode}`];
      return `${freedb ? '' : '<img class="ts-thumb" alt="Release artwork" hidden>'}<div><strong>${html(r.artist)} – ${html(r.title)}</strong><span>${details.filter(Boolean).map(html).join(' · ')}</span>${r.url ? `<span class="ts-url">${html(r.url)} ${preview(r.url)}</span>` : ''}</div>`;
    };

    const choose = (index:number, focus = false) => {
      chosen = index;
      dialog.querySelectorAll<HTMLTableRowElement>('.ts-results tbody tr').forEach(row => { const on = Number(row.dataset.index) === index; row.classList.toggle('selected',on); row.setAttribute('aria-selected',String(on)); row.tabIndex = on ? 0 : -1; if (on && focus) { row.focus(); row.scrollIntoView({block:'nearest'}); } });
      $('.ts-chosen').innerHTML = chosenMarkup();
      const thumb = dialog.querySelector<HTMLImageElement>('.ts-thumb');
      if (thumb) void loadImage(results[index]?.thumb,thumb);
      syncSearch();
    };
    const tocText = () => toc ? `Disc ID ${toc.id}: ${toc.tracks} track${toc.tracks === 1 ? '' : 's'} from the selected files in file list order, ${duration(toc.seconds-2)} in total.` : 'The disc ID needs the length of every selected file (at most 99 files).';
    const syncSearch = () => {
      if (web) {
        dialog.querySelectorAll<HTMLInputElement|HTMLButtonElement>('.dialog-content input,.dialog-content button').forEach(control => control.disabled = working);
        $<HTMLButtonElement>('#ts-next').disabled = working || chosen < 0;
        return;
      }
      const mode = $<HTMLSelectElement>('#ts-mode').value as SearchMode, pair = mode === 'artist_album';
      $('#ts-pair').hidden = !pair; $('#ts-text-field').hidden = pair || mode === 'toc'; $('#ts-toc').hidden = mode !== 'toc';
      $('#ts-text-label').textContent = mode === 'release_id' ? 'RELEASE ID OR ADDRESS' : mode === 'barcode' ? 'BARCODE' : mode === 'catalog_number' ? 'CATALOG NUMBER' : mode === 'disc_id' ? 'DISC ID' : 'SEARCH TEXT';
      $<HTMLButtonElement>('#ts-search').textContent = mode === 'release_id' ? 'Look up' : 'Search';
      dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLButtonElement>('.dialog-content input,.dialog-content select,.dialog-content button').forEach(control => control.disabled = working);
      if (mode === 'toc' && !toc) $<HTMLButtonElement>('#ts-search').disabled = true;
      $<HTMLButtonElement>('#ts-next').disabled = working || chosen < 0;
    };
    /** A web source's index script lists the results; without one the search opens the album page. */
    const searchWeb = async () => {
      const values = web!.search_by.map((_,i) => $<HTMLInputElement>(`#ts-web-${i}`).value);
      webValues = values;
      const ticket = searchTicket = nextTicket();
      status(`Searching ${name}…`);
      try {
        const page = await this.host.invoke<WebSearchPage>('web_source_search',{source:web!.id,values,ticket});
        if (ticket !== searchTicket || !dialog.open) return;
        if (page.direct) { await loadRelease(page.results[0].url); return; }
        webColumns = page.columns; webRows = page.results; chosen = -1; total = page.results.length;
        results = page.results.map(r => ({id:r.url,title:r.values.filter(Boolean).join(' – ') || r.url,artist:'',year:'',country:'',label:'',catalog_number:'',format:'',track_count:null,score:null,barcode:'',thumb:r.cover,url:r.preview || r.url}));
        $('.ts-result-area').innerHTML = resultMarkup();
        status(results.length ? `${results.length} result${results.length === 1 ? '' : 's'} found. Choose one and select Next.` : 'No results. Try other search words.');
        $('.ts-chosen').innerHTML = ''; if (results.length) choose(0); else syncSearch();
      } catch (e) {
        if (ticket !== searchTicket || !dialog.open) return;
        status('Search failed.'); error(e);
      }
    };
    const search = async (more = false) => {
      clearError();
      if (web) { await searchWeb(); return; }
      const mode = $<HTMLSelectElement>('#ts-mode').value as SearchMode;
      const query = more && lastQuery ? {...lastQuery,page:Number(lastQuery.page)+1} : {mode,artist:$<HTMLInputElement>('#ts-artist').value,album:$<HTMLInputElement>('#ts-album').value,text:$<HTMLInputElement>('#ts-text').value,lengths:files.map(f => f.length),page:0};
      if (!more && mode === 'release_id') { await loadRelease(query.text as string); return; }
      const ticket = searchTicket = nextTicket();
      status(more ? 'Loading more results…' : freedb && mode === 'disc_id' && provider === 'freedb' ? `Searching ${name} in every category…` : `Searching ${name}…`);
      if (more) $<HTMLButtonElement>('#ts-more').disabled = true;
      try {
        const found = await this.host.invoke<SearchPage>('tag_source_search',{provider,query,ticket});
        if (ticket !== searchTicket || !dialog.open) return;
        lastQuery = query;
        if (more) {
          const known = new Set(results.map(r => r.id)), added = found.results.filter(r => !known.has(r.id));
          results = [...results,...added];
          // A page without new releases ends paging even if the total says otherwise.
          total = added.length ? found.total : results.length;
        } else { results = found.results; total = found.total; chosen = -1; }
        $('.ts-result-area').innerHTML = resultMarkup();
        const shown = results.length, all = total != null && total > shown ? ` (${total} in total)` : '';
        status(shown ? `${shown} release${shown === 1 ? '' : 's'} found${all}. Choose one and select Next.` : freedb ? 'No disc with this ID was found.' : 'No releases found. Try fewer words, another spelling or a different search type.');
        if (more) { choose(chosen); dialog.querySelector<HTMLTableRowElement>(`.ts-results tr[data-index="${shown-found.results.length}"]`)?.scrollIntoView({block:'nearest'}); }
        else { $('.ts-chosen').innerHTML = ''; if (shown) choose(0); else syncSearch(); }
      } catch (e) {
        if (ticket !== searchTicket || !dialog.open) return;
        status('Search failed.'); error(e);
        dialog.querySelector<HTMLButtonElement>('#ts-more')?.removeAttribute('disabled');
      }
    };
    const loadRelease = async (input:string) => {
      if (!input.trim()) { error(`Enter a ${name} release ID or address.`); return; }
      clearError();
      const ticket = releaseTicket = nextTicket();
      working = true; syncSearch(); status('Loading release…');
      try {
        const loaded = web ? await this.host.invoke<SourceRelease>('web_source_release',{source:web.id,url:input,ticket}) : await this.host.invoke<SourceRelease>('tag_source_release',{provider,input,ticket});
        if (ticket !== releaseTicket || !dialog.open) return;
        working = false;
        showReview(loaded);
        if (loaded.more_covers) void this.host.invoke<SourceCover[]>('tag_source_covers',{id:loaded.id,ticket}).then(covers => {
          if (release !== loaded || !dialog.open || !covers.length) return;
          const current = release.covers[coverIndex]?.url;
          release.covers = covers; coverIndex = Math.max(0,covers.findIndex(c => c.url === current));
          renderCover(false);
        }).catch(() => undefined);
      } catch (e) {
        if (ticket !== releaseTicket || !dialog.open) return;
        working = false; status('The release could not be loaded.'); error(e); syncSearch();
      }
    };

    const modes:[SearchMode,string][] = freedb ? [['toc','Disc ID of the selected files'],['disc_id','freedb disc ID']]
      : [['artist_album','Artist and album'],['text','Free text'],['release_id',provider === 'musicbrainz' ? 'Release ID (MBID) or address' : 'Release ID or address'],['barcode','Barcode'],['catalog_number','Catalog number']];
    const renderSearch = (mode?:SearchMode) => {
      release = undefined;
      const searchFields = web ? `<div class="ts-search ts-web-search">${web.search_by.map((field,i) => `<label class="field"><span>${html(field.label.toUpperCase())}</span><input id="ts-web-${i}" value="${html(webValues[i] || '')}" autocomplete="off" spellcheck="false"></label>`).join('')}<button id="ts-search" class="ts-search-button">Search</button></div>` : '';
      dialog.innerHTML = `<header>${html(name)}</header><div class="dialog-content">
        ${searchFields}<div class="ts-search" ${web ? 'hidden' : ''}><label class="field"><span>SEARCH BY</span><select id="ts-mode">${modes.map(([value,label]) => `<option value="${value}">${html(label)}</option>`).join('')}</select></label>
        <div class="field-pair" id="ts-pair"><label class="field"><span>ARTIST</span><input id="ts-artist" value="${html(prefill.artist)}" autocomplete="off" spellcheck="false"></label><label class="field"><span>ALBUM</span><input id="ts-album" value="${html(prefill.album)}" autocomplete="off" spellcheck="false"></label></div>
        <label class="field" id="ts-text-field" hidden><span id="ts-text-label">SEARCH TEXT</span><input id="ts-text" autocomplete="off" spellcheck="false"></label>
        <p class="ts-toc" id="ts-toc" hidden>${html(tocText())}</p>
        <button id="ts-search" class="ts-search-button">Search</button></div>
        <p class="ts-status" role="status" aria-live="polite">${files.length} selected file${files.length === 1 ? '' : 's'}. ${freedb ? 'Look up the disc these files were ripped from.' : 'Search for the album these files belong to.'}</p>
        <div class="ts-result-area">${resultMarkup()}</div><div class="ts-chosen">${chosenMarkup()}</div></div>
        <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ts-next" disabled>Next</button></div>`;
      if (web) dialog.querySelector('.ts-search[hidden]')?.remove();
      else if (mode || (freedb && !toc)) $<HTMLSelectElement>('#ts-mode').value = mode || 'disc_id';
      $('[data-close]').addEventListener('click',() => dialog.close());
      dialog.querySelector('#ts-mode')?.addEventListener('change',() => { syncSearch(); (dialog.querySelector<HTMLInputElement>('#ts-text-field:not([hidden]) input') || dialog.querySelector<HTMLInputElement>('#ts-pair:not([hidden]) input') || $<HTMLButtonElement>('#ts-search')).focus(); });
      $('#ts-search').addEventListener('click',() => void search());
      dialog.querySelectorAll<HTMLInputElement>('.ts-search input').forEach(input => input.addEventListener('keydown',event => { if (event.key === 'Enter') { event.preventDefault(); void search(); } }));
      $('.ts-result-area').addEventListener('click',event => {
        if ((event.target as Element).closest('#ts-more')) { void search(true); return; }
        const row = (event.target as Element).closest<HTMLTableRowElement>('tr[data-index]'); if (row) choose(Number(row.dataset.index));
      });
      $('.ts-result-area').addEventListener('dblclick',event => { const row = (event.target as Element).closest<HTMLTableRowElement>('tr[data-index]'); if (row && !working) void loadRelease(results[Number(row.dataset.index)].id); });
      $('.ts-result-area').addEventListener('keydown',event => {
        const key = (event as KeyboardEvent).key;
        if (!(event.target as Element).closest('tr[data-index]')) return;
        if (key === 'ArrowDown' || key === 'ArrowUp') { event.preventDefault(); choose(Math.max(0,Math.min(results.length-1,chosen+(key === 'ArrowDown' ? 1 : -1))),true); }
        if (key === 'Enter' && chosen >= 0 && !working) { event.preventDefault(); void loadRelease(results[chosen].id); }
      });
      $('#ts-next').addEventListener('click',() => { if (chosen >= 0) void loadRelease(results[chosen].id); });
      syncSearch();
      if (chosen >= 0) choose(chosen);
    };

    const fileName = (path:string|null) => path ? files.find(f => f.path === path)?.name || path : '';
    const fileLength = (path:string|null) => path ? duration(files.find(f => f.path === path)?.length ?? null) : '';
    const trackFields = () => { const names = fieldRows(release!).filter(row => row.perTrack).map(row => row.name); return names.length ? names : ['TITLE']; };
    const trackValue = (track:SourceTrack) => (track.fields[trackField] || release!.fields[trackField] || (trackField === 'TITLE' ? [track.title] : [])).join(MULTI);
    const matchMarkup = () => {
      const tracks = release!.tracks;
      return `<table class="ts-table ts-matches" aria-label="Track and file matching"><thead><tr><th class="ts-num">#</th><th>${trackField === 'TITLE' ? 'Track' : html(trackField)}</th><th class="ts-num ts-wide">Time</th><th>File</th><th class="ts-num ts-wide">Time</th><th class="ts-move"><span class="visually-hidden">Move</span></th></tr></thead><tbody>${slots.map((path,index) => {
        const track = tracks[index], warn = track && path && track.length != null && Math.abs(track.length-(files.find(f => f.path === path)?.length ?? track.length)) > 10;
        return `<tr data-index="${index}" class="${track ? '' : 'ts-unmatched'}"><td class="ts-num">${track ? html(track.position || String(index+1)) : ''}</td><td>${track ? `<input class="ts-track-value" data-track="${index}" value="${html(trackValue(track))}" aria-label="${html(trackField)} of track ${html(track.position || String(index+1))}" autocomplete="off" spellcheck="false">` : '<em>Not written</em>'}</td><td class="ts-num ts-wide">${track ? duration(track.length) : ''}</td><td title="${html(fileName(path))}">${path ? html(fileName(path)) : '<em>No file</em>'}</td><td class="ts-num ts-wide ${warn ? 'ts-warn' : ''}" ${warn ? 'title="Length differs from the track by more than 10 seconds"' : ''}>${fileLength(path)}</td><td class="ts-move"><button data-move="-1" aria-label="Move ${html(fileName(path) || 'empty slot')} up" ${index === 0 || !path ? 'disabled' : ''}>↑</button><button data-move="1" aria-label="Move ${html(fileName(path) || 'empty slot')} down" ${index === slots.length-1 || !path ? 'disabled' : ''}>↓</button></td></tr>`;
      }).join('')}</tbody></table>`;
    };
    const fieldMarkup = () => `<table class="ts-table ts-fields" aria-label="Fields to write"><tbody>${fieldRows(release!).map(row => `<tr data-row="${html(row.name)}"><td class="ts-check"><input type="checkbox" data-field="${html(row.name)}" aria-label="Write ${html(row.name)}" ${enabled.has(row.name) ? 'checked' : ''}></td><th scope="row" title="${html(row.name)}">${html(row.name)}</th><td>${row.editable
      ? `<input class="ts-value" data-value="${html(row.name)}" value="${html(row.value)}" aria-label="${html(row.name)} value" autocomplete="off" spellcheck="false">${row.perTrack && release!.tracks.length > 1 ? '<small>all tracks</small>' : ''}`
      : `<span class="ts-summary" title="${html(row.value)}">${html(row.value)}</span><button type="button" class="ts-edit-tracks" data-track-field="${html(row.name)}">Edit per track</button>`}</td></tr>`).join('')}</tbody></table>`;
    const renderFields = () => { const list = dialog.querySelector('.ts-field-list'); if (list) list.innerHTML = fieldMarkup(); };
    const coverTemplate = () => String(this.host.options().cover_filename ?? '').trim() || 'folder';
    const renderCover = (reload = true) => {
      const covers = release!.covers, section = $('.ts-cover');
      if (!covers.length) { section.innerHTML = `<p>No artwork is available for this release.</p>`; syncReview(); return; }
      const writing = dialog.querySelector<HTMLInputElement>('#ts-cover-write')?.checked ?? true;
      const replacing = dialog.querySelector<HTMLInputElement>('#ts-cover-replace')?.checked ?? true;
      const saving = dialog.querySelector<HTMLInputElement>('#ts-cover-save')?.checked ?? storage.get(saveCoverKey) === '1';
      section.innerHTML = `<div class="ts-cover-preview"><img alt="Selected artwork" hidden></div><div><label class="action-check"><input type="checkbox" id="ts-cover-write" ${writing ? 'checked' : ''}> Write cover art</label>
        <label class="field"><span>ARTWORK</span><select id="ts-cover-choice">${covers.map((c,i) => `<option value="${i}" ${i === coverIndex ? 'selected' : ''}>${i+1}: ${html(c.kind)}${c.width && c.height ? ` (${c.width}×${c.height})` : ''}</option>`).join('')}</select></label>
        <label class="action-check"><input type="checkbox" id="ts-cover-replace" ${replacing ? 'checked' : ''}> Replace existing artwork</label>
        <label class="action-check" title="Saved next to the written files; set the name in Options → Tag Sources"><input type="checkbox" id="ts-cover-save" ${saving ? 'checked' : ''}> Save image to file <code>${html(coverTemplate())}</code></label>
        <button type="button" id="ts-cover-extract">Save cover as…</button><p class="ts-cover-note" role="status"></p></div>`;
      $('#ts-cover-choice').addEventListener('change',event => { coverIndex = Number((event.target as HTMLSelectElement).value); void loadImage(covers[coverIndex].thumb || covers[coverIndex].url,$<HTMLImageElement>('.ts-cover-preview img')); });
      $('#ts-cover-write').addEventListener('change',syncReview);
      $('#ts-cover-save').addEventListener('change',event => { storage.set(saveCoverKey,(event.target as HTMLInputElement).checked ? '1' : '0'); syncReview(); });
      $('#ts-cover-extract').addEventListener('click',() => void extractCover());
      if (reload || !dialog.querySelector<HTMLImageElement>('.ts-cover-preview img')?.src) void loadImage(covers[coverIndex].thumb || covers[coverIndex].url,$<HTMLImageElement>('.ts-cover-preview img'));
      syncReview();
    };
    /** Utils → Extract cover: save the displayed artwork to a file of your choice. */
    const extractCover = async () => {
      const cover = release?.covers[coverIndex];
      if (!cover || working) return;
      const extension = /\.(png|gif|webp|bmp)(?:[?#]|$)/i.exec(cover.url)?.[1].toLowerCase() || 'jpg';
      const suggested = `${[release!.artist,release!.title].filter(Boolean).join(' - ').replace(/[\\/:*?"<>|]+/g,'_') || 'cover'}.${extension}`;
      const note = $('.ts-cover-note');
      try {
        const path = await this.host.pickSave(suggested);
        if (!path || !dialog.open) return;
        note.textContent = 'Saving the cover…';
        const saved = await this.host.invoke<string>('tag_source_save_cover',{url:cover.url,path});
        if (dialog.open) note.textContent = `Saved to ${saved}`;
      } catch (e) { if (dialog.open) { note.textContent = ''; error(e); } }
    };
    const items = ():ApplyItem[] => coversOnly ? files.map(file => ({path:file.path,updates:{}})) : buildApplyItems(release!,slots,enabled);
    const coverChoice = ():ApplyCover|null => {
      const write = dialog.querySelector<HTMLInputElement>('#ts-cover-write'), cover = release?.covers[coverIndex];
      if (!write?.checked || !cover) return null;
      return {url:cover.url,replace_existing:$<HTMLInputElement>('#ts-cover-replace').checked,save_as:$<HTMLInputElement>('#ts-cover-save').checked ? coverTemplate() : null};
    };
    const syncReview = () => {
      const count = items().length, cover = coverChoice(), nothing = !enabled.size && !cover;
      const apply = $<HTMLButtonElement>('#ts-apply');
      apply.textContent = `${coversOnly ? 'Save cover to' : 'Save tags to'} ${count} file${count === 1 ? '' : 's'}`;
      dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLButtonElement>('.dialog-content input,.dialog-content select,.dialog-content button,.dialog-footer button').forEach(control => { if (!control.matches('[data-move]')) control.disabled = working; });
      dialog.querySelectorAll<HTMLButtonElement>('[data-move]').forEach(button => { const index = Number(button.closest('tr')!.dataset.index), delta = Number(button.dataset.move); button.disabled = working || !slots[index] || index+delta < 0 || index+delta >= slots.length; });
      const replace = dialog.querySelector<HTMLInputElement>('#ts-cover-replace'); if (replace) replace.disabled = working || !cover;
      const save = dialog.querySelector<HTMLInputElement>('#ts-cover-save'); if (save) save.disabled = working || !cover;
      apply.disabled = working || !count || nothing;
      if (coversOnly) { $('.ts-status').textContent = working ? 'Writing cover art…' : cover ? `The chosen artwork is written to ${count} selected file${count === 1 ? '' : 's'}; their fields do not change.` : 'Choose the artwork to write.'; return; }
      const unmatched = files.length - count, missing = release!.tracks.length - count;
      $('.ts-status').textContent = working ? 'Writing tags…' : nothing ? 'Choose at least one field or the cover art to write.' : `${count} of ${release!.tracks.length} tracks have a file.${missing > 0 ? ` ${missing} track${missing === 1 ? ' has' : 's have'} no file.` : ''}${unmatched > 0 ? ` ${unmatched} selected file${unmatched === 1 ? ' is' : 's are'} not matched and will not change.` : ''} Saving writes the checked fields immediately.`;
    };
    const renderMatches = (focus?:{index:number;selector?:string;delta?:number}) => {
      if (coversOnly) return;
      $('.ts-match-area').innerHTML = matchMarkup(); syncReview();
      if (!focus) return;
      const row = `.ts-matches tr[data-index="${focus.index}"]`;
      (dialog.querySelector<HTMLElement>(focus.selector ? `${row} ${focus.selector}` : `${row} button[data-move="${focus.delta}"]:not(:disabled)`) || dialog.querySelector<HTMLButtonElement>(`${row} button[data-move]:not(:disabled)`))?.focus();
    };
    const move = (index:number, delta:number, selector?:string) => {
      if (working || !slots[index] || index+delta < 0 || index+delta >= slots.length) return;
      slots = moveSlot(slots,index,delta); renderMatches({index:index+delta,selector,delta});
    };
    const moveTo = (index:number, to:number, selector?:string) => {
      if (working || !slots[index] || index === to) return;
      slots = moveSlotTo(slots,index,to); renderMatches({index:to,selector,delta:to < index ? -1 : 1});
    };
    const toggleAll = (on:boolean) => { if (!release) return; enabled = on ? new Set(fieldRows(release).map(row => row.name)) : new Set(); dialog.querySelectorAll<HTMLInputElement>('.ts-fields input[type=checkbox]').forEach(box => box.checked = on); syncReview(); };
    const showReview = (loaded:SourceRelease) => {
      // A web source that returns only album values (or only artwork) applies them to every selected file.
      if (web && !loaded.tracks.length) loaded.tracks = files.map((_,i) => ({position:String(i+1),disc:1,number:i+1,title:'',artist:'',length:null,fields:{}}));
      release = loaded; coverIndex = 0;
      slots = autoMatch(loaded.tracks,files);
      trackField = trackFields().includes('TITLE') ? 'TITLE' : trackFields()[0];
      const disabled = new Set<string>(JSON.parse(storage.get(disabledKey) || '[]'));
      enabled = coversOnly ? new Set() : new Set(fieldRows(loaded).map(row => row.name).filter(field => !disabled.has(field)));
      const fieldSection = `<section class="ts-field-section" aria-label="Fields"><div class="ts-tools"><strong>Fields</strong><button id="ts-all" title="Enable all fields (Ctrl+E)">All</button><button id="ts-none" title="Disable all fields (Ctrl+D)">None</button></div><div class="ts-field-list">${fieldMarkup()}</div><p class="ts-hint">Values can be changed before saving; separate multiple values with \\\\.</p></section>`;
      const matchSection = `<section class="ts-match-section" aria-label="Track and file matching"><div class="ts-tools"><strong>Tracks and files</strong><label class="ts-track-field">Track column <select id="ts-track-field">${trackFields().map(field => `<option ${field === trackField ? 'selected' : ''}>${html(field)}</option>`).join('')}</select></label><button id="ts-rematch">Auto-match</button><button id="ts-by-title" title="Suggest sorting by title">By title</button><button id="ts-by-length" title="Suggest sorting by length">By length</button><button id="ts-order">Selection order</button></div><div class="ts-match-area"></div></section>`;
      dialog.innerHTML = `<header>${html(name)}: ${html(loaded.artist)} – ${html(loaded.title)}</header><div class="dialog-content">
        <div class="ts-review ${coversOnly ? 'ts-cover-only' : ''}">${coversOnly ? '' : fieldSection}<section class="ts-cover" aria-label="Artwork"></section></div>
        ${coversOnly ? '' : matchSection}
        ${loaded.url ? `<p class="ts-url">${html(loaded.url)} ${preview(loaded.url)}</p>` : ''}<p class="ts-status" role="status" aria-live="polite"></p></div>
        <div class="dialog-footer"><button id="ts-back">Back</button><button data-close>Cancel</button><button class="primary" id="ts-apply"></button></div>`;
      $('[data-close]').addEventListener('click',() => dialog.close());
      $('#ts-back').addEventListener('click',() => { clearError(); renderSearch(results.length || web ? undefined : freedb ? 'disc_id' : 'release_id'); });
      if (!coversOnly) {
        $('#ts-all').addEventListener('click',() => toggleAll(true));
        $('#ts-none').addEventListener('click',() => toggleAll(false));
        const fields = $('.ts-field-section');
        fields.addEventListener('change',event => { const box = event.target as HTMLInputElement; if (!box.matches('input[type=checkbox][data-field]')) return; if (box.checked) enabled.add(box.dataset.field!); else enabled.delete(box.dataset.field!); syncReview(); });
        fields.addEventListener('input',event => {
          const input = event.target as HTMLInputElement;
          if (!input.matches('.ts-value')) return;
          editField(loaded,input.dataset.value!,input.value);
          if (input.dataset.value === trackField) renderMatches();
        });
        fields.addEventListener('click',event => {
          const button = (event.target as Element).closest<HTMLButtonElement>('.ts-edit-tracks');
          if (!button) return;
          trackField = button.dataset.trackField!; $<HTMLSelectElement>('#ts-track-field').value = trackField;
          renderMatches(); dialog.querySelector<HTMLInputElement>('.ts-track-value')?.focus();
        });
        $('#ts-track-field').addEventListener('change',event => { trackField = (event.target as HTMLSelectElement).value; renderMatches(); });
        $('#ts-rematch').addEventListener('click',() => { slots = autoMatch(loaded.tracks,files); renderMatches(); });
        $('#ts-by-title').addEventListener('click',() => { slots = suggestByTitle(loaded.tracks,files); renderMatches(); });
        $('#ts-by-length').addEventListener('click',() => { slots = suggestByLength(loaded.tracks,files); renderMatches(); });
        $('#ts-order').addEventListener('click',() => { slots = autoMatch(loaded.tracks,files.map(f => ({...f,track:'',title:'',name:''}))); renderMatches(); });
        const matches = $('.ts-match-area');
        matches.addEventListener('click',event => {
          const button = (event.target as Element).closest<HTMLButtonElement>('button[data-move]');
          if (button) move(Number(button.closest('tr')!.dataset.index),Number(button.dataset.move));
        });
        matches.addEventListener('input',event => {
          const input = event.target as HTMLInputElement;
          if (!input.matches('.ts-track-value')) return;
          editField(loaded,trackField,input.value,Number(input.dataset.track));
          renderFields();
        });
      }
      $('#ts-apply').addEventListener('click',async () => {
        clearError();
        const reason = this.host.reason(), chosenItems = items(), cover = coverChoice();
        if (reason) { error(reason); return; }
        if (!chosenItems.length) { error('Match at least one file to a track.'); return; }
        if (!coversOnly) storage.set(disabledKey,JSON.stringify(fieldRows(loaded).map(row => row.name).filter(field => !enabled.has(field))));
        working = true; syncReview();
        try { await this.host.apply(chosenItems,cover); dialog.close(); }
        catch (e) { working = false; if (dialog.open) { syncReview(); error(e); } }
      });
      renderMatches(); renderCover();
      $<HTMLButtonElement>('#ts-apply').focus();
    };

    renderSearch();
    dialog.showModal();
    if (web) { const first = dialog.querySelector<HTMLInputElement>('#ts-web-0'); (first || $<HTMLButtonElement>('#ts-search')).focus(); first?.select(); }
    else if (freedb) (toc ? $<HTMLButtonElement>('#ts-search') : $<HTMLInputElement>('#ts-text')).focus();
    else { const artist = $<HTMLInputElement>('#ts-artist'); artist.focus(); artist.select(); }
  }
}
