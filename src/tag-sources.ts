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
export type TagSourceCredentials = {discogs_token_set:boolean;discogs_token_in_keychain:boolean;keychain_available:boolean};
export type FieldRow = {name:string;value:string;perTrack:boolean;editable:boolean};
/**
 * Covers-only sources (Mp3tag's Tag Sources → Covers) write artwork and no
 * fields. `releaseId` is Tag Sources → Discogs Release ID: the release is
 * looked up directly. `quick` (Ctrl+Shift+I) searches with the selection's
 * album and artist at once and opens a single result without the list.
 * `web` is the script of the `web` provider, shown with `title`.
 */
export type OpenOptions = {coversOnly?:boolean;releaseId?:boolean;quick?:boolean;web?:WebSourceInfo;title?:string};
export type IndexStatus = {directory:string|null;discs:number};
/** Options → Local freedb: the text index commands of the backend. */
export type LocalIndexHost = {
  status:()=>Promise<IndexStatus>;
  build:(directory:string, job:string)=>Promise<IndexStatus>;
  cancel:(job:string)=>Promise<void>;
  /** Calls `update` with progress events of `job`; returns a function that stops listening. */
  progress:(job:string, update:(done:number,total:number)=>void)=>Promise<()=>void>;
};

/** Tag Sources menu commands that open a source dialog. */
export const SOURCE_OPERATIONS = ['source-musicbrainz','source-discogs','source-discogs-release','source-freedb','source-local-freedb','source-cover-musicbrainz','source-cover-discogs'] as const;
export type SourceOperation = typeof SOURCE_OPERATIONS[number];
export type SourceRequest = {provider:Provider;coversOnly:boolean;releaseId:boolean};
export function sourceRequest(op:string):SourceRequest|null {
  if (!(SOURCE_OPERATIONS as readonly string[]).includes(op)) return null;
  const releaseId = op === 'source-discogs-release';
  const provider = (releaseId ? 'discogs' : op.replace(/^source-(cover-)?/,'').replace('-','_')) as Provider;
  return {provider,coversOnly:op.startsWith('source-cover-'),releaseId};
}
export function sourceOperation(request:SourceRequest):SourceOperation {
  if (request.releaseId) return 'source-discogs-release';
  return `source-${request.coversOnly ? 'cover-' : ''}${request.provider.replace('_','-')}` as SourceOperation;
}
/** Ctrl+I repeats the last tag source of any kind; older builds remembered only MusicBrainz or Discogs. */
export function lastSourceOperation(saved:string|null, legacy:string|null):SourceOperation {
  if (saved && sourceRequest(saved)) return saved as SourceOperation;
  return legacy === 'discogs' ? 'source-discogs' : 'source-musicbrainz';
}

/** A named CDDB server of Options → Tag Sources → freedb. */
export type FreedbServer = {name:string;protocol:string;address:string;path:string;port:string};
/** The list "Current list of freedb servers" restores. */
export const FREEDB_SERVERS:FreedbServer[] = [
  {name:'gnudb (HTTPS)',protocol:'https',address:'gnudb.gnudb.org',path:'/~cddb/cddb.cgi',port:''},
  {name:'gnudb (HTTP)',protocol:'http',address:'gnudb.gnudb.org',path:'/~cddb/cddb.cgi',port:''},
];
const serverKeys = ['protocol','address','path','port'] as const;
const sameServer = (a:Partial<FreedbServer>, b:Partial<FreedbServer>) => serverKeys.every(key => String(a[key] ?? '').trim() === String(b[key] ?? '').trim());
/**
 * The saved server list and the entry in use. The backend reads the server
 * in use from the page's own protocol/address/path/port; a server saved
 * before the list existed joins the list as "Custom server".
 */
export function freedbServerList(page:Record<string,unknown>):{servers:FreedbServer[];index:number} {
  const saved = Array.isArray(page.servers) ? (page.servers as Partial<FreedbServer>[]).filter(s => s && typeof s === 'object') : [];
  const servers:FreedbServer[] = (saved.length ? saved : FREEDB_SERVERS).map(s => ({name:String(s.name ?? ''),protocol:s.protocol === 'http' ? 'http' : 'https',address:String(s.address ?? ''),path:String(s.path ?? ''),port:String(s.port ?? '')}));
  const active = {protocol:page.protocol === 'http' ? 'http' : 'https',address:String(page.address ?? FREEDB_SERVERS[0].address),path:String(page.path ?? FREEDB_SERVERS[0].path),port:String(page.port ?? '')};
  const chosen = Number(page.server);
  if (Number.isInteger(chosen) && servers[chosen] && sameServer(servers[chosen],active)) return {servers,index:chosen};
  const found = servers.findIndex(s => sameServer(s,active));
  if (found >= 0) return {servers,index:found};
  return {servers:[...servers,{name:'Custom server',...active}],index:servers.length};
}
/** Store the list and make `index` the server in use. */
export function useFreedbServer(page:Record<string,unknown>, servers:FreedbServer[], index:number) {
  const server = servers[index];
  Object.assign(page,{servers:servers.map(s => ({...s})),server:index,protocol:server.protocol,address:server.address,path:server.path,port:server.port});
}

export type SuggestSorting = ''|'title'|'length';
/** Matching when a release opens: Utils → Suggest sorting (Title or Length) when checked, else auto-match. */
export function initialSlots(tracks:SourceTrack[], files:SourceFile[], suggest:SuggestSorting) {
  return suggest === 'title' ? suggestByTitle(tracks,files) : suggest === 'length' ? suggestByLength(tracks,files) : autoMatch(tracks,files);
}

/** Field selection in the review: a click selects one, Ctrl/⌘ toggles one, Shift extends from the anchor. */
export function selectFields(names:string[], selected:ReadonlySet<string>, anchor:string|null, name:string, mode:'single'|'toggle'|'range') {
  if (mode === 'toggle') { const next = new Set(selected); if (next.has(name)) next.delete(name); else next.add(name); return {selected:next,anchor:name}; }
  if (mode === 'range' && anchor && names.includes(anchor)) {
    const [from,to] = [names.indexOf(anchor),names.indexOf(name)].sort((a,b) => a-b);
    return {selected:new Set(names.slice(from,to+1)),anchor};
  }
  return {selected:new Set([name]),anchor:name};
}
/** Ctrl+Shift+A: select the fields that are not selected. */
export const invertFields = (names:string[], selected:ReadonlySet<string>) => new Set(names.filter(name => !selected.has(name)));
/** A checkbox click applies to every selected field when its own field is one of them. */
export function toggleEnabled(enabled:ReadonlySet<string>, selected:ReadonlySet<string>, name:string, on:boolean) {
  const next = new Set(enabled);
  for (const field of selected.has(name) ? selected : [name]) { if (on) next.add(field); else next.delete(field); }
  return next;
}

const localFreedbPage = 'tag-sources-local-freedb', freedbPage = 'tag-sources-freedb';
/**
 * Options → Tag Sources with its freedb and Local freedb pages (Mp3tag:
 * Options › Tag Sources › freedb / Local freedb). Values persist under
 * `pages.<id>`; the backend reads them through the shared settings.
 */
export function tagSourcesOptionsPages(pickFolder:()=>Promise<string|null>, index?:LocalIndexHost):OptionsPage[] {
  const directoryPath = `pages.${localFreedbPage}.directory`;
  const directory = (draft:OptionsDraft) => String((draft.pages[localFreedbPage] as Record<string,unknown>|undefined)?.directory ?? '');
  const setDirectory = (draft:OptionsDraft, value:string) => { draft.pages[localFreedbPage] = {...draft.pages[localFreedbPage],directory:value}; };
  const serverPage = (draft:OptionsDraft) => (draft.pages[freedbPage] ??= {}) as Record<string,unknown>;
  const field = (id:string, label:string, value:string, option:string, extra = '') => `<label class="field"><span>${label}</span><input id="${id}" data-option="pages.${freedbPage}.${option}" value="${html(value)}" autocomplete="off" spellcheck="false" ${extra}></label>`;
  const serversMarkup = (draft:OptionsDraft) => {
    const {servers,index} = freedbServerList(serverPage(draft)), server = servers[index];
    return `<div class="field"><span>FREEDB SERVER</span><div class="action-input"><select id="freedb-server" aria-label="freedb server">${servers.map((s,i) => `<option value="${i}" ${i === index ? 'selected' : ''} ${s.name ? 'translate="no"' : ''}>${html(s.name || 'Unnamed server')}</option>`).join('')}</select><button type="button" id="freedb-server-new">New</button><button type="button" id="freedb-server-delete" ${servers.length < 2 ? 'disabled' : ''}>Delete</button></div></div>
      <label class="field"><span>NAME</span><input id="freedb-server-name" value="${html(server.name)}" autocomplete="off" spellcheck="false"></label>
      <label class="field"><span>PROTOCOL</span><select id="freedb-server-protocol" data-option="pages.${freedbPage}.protocol"><option value="https" ${server.protocol === 'https' ? 'selected' : ''}>HTTPS</option><option value="http" ${server.protocol === 'http' ? 'selected' : ''}>HTTP</option></select></label>
      ${field('freedb-server-address','ADDRESS',server.address,'address','placeholder="gnudb.gnudb.org"')}
      ${field('freedb-server-path','PATH',server.path,'path','placeholder="/~cddb/cddb.cgi"')}
      ${field('freedb-server-port','PORT',server.port,'port','placeholder="Default port of the protocol" maxlength="5"')}
      <p><button type="button" id="freedb-server-list">Current list of freedb servers</button></p>`;
  };
  const bindServers = (root:HTMLElement, draft:OptionsDraft, refresh:()=>void) => {
    const page = serverPage(draft);
    let {servers,index} = freedbServerList(page);
    useFreedbServer(page,servers,index);
    const use = (next:FreedbServer[], at:number, focus?:string) => { servers = next; index = at; useFreedbServer(page,servers,index); refresh(); if (focus) root.querySelector<HTMLElement>(focus)?.focus(); };
    root.querySelector('#freedb-server')!.addEventListener('change',event => use(servers,Number((event.target as HTMLSelectElement).value),'#freedb-server'));
    root.querySelector('#freedb-server-new')!.addEventListener('click',() => use([...servers,{name:'New server',protocol:'https',address:'',path:'/~cddb/cddb.cgi',port:''}],servers.length,'#freedb-server-name'));
    root.querySelector('#freedb-server-delete')!.addEventListener('click',() => { if (servers.length > 1) use(servers.filter((_,i) => i !== index),Math.max(0,index-1),'#freedb-server'); });
    // Mp3tag's button fetches the list; OpenTag ships it and restores it here.
    root.querySelector('#freedb-server-list')!.addEventListener('click',() => use(FREEDB_SERVERS.map(s => ({...s})),0,'#freedb-server'));
    const edit = (key:keyof FreedbServer) => (event:Event) => {
      servers[index] = {...servers[index],[key]:(event.target as HTMLInputElement|HTMLSelectElement).value};
      useFreedbServer(page,servers,index);
      if (key === 'name') {
        const option = root.querySelector<HTMLOptionElement>(`#freedb-server option[value="${index}"]`);
        if (option) {
          if (servers[index].name) option.setAttribute('translate','no'); else option.removeAttribute('translate');
          option.textContent = servers[index].name || 'Unnamed server';
        }
      }
    };
    root.querySelector('#freedb-server-name')!.addEventListener('input',edit('name'));
    root.querySelector('#freedb-server-protocol')!.addEventListener('change',edit('protocol'));
    for (const key of ['address','path','port'] as const) root.querySelector(`#freedb-server-${key}`)!.addEventListener('input',edit(key));
  };
  const indexText = (status:IndexStatus|null, draft:OptionsDraft) => {
    if (!status) return 'Reading the text index…';
    if (!status.directory) return 'No text index. Build one to search this folder by artist and album words.';
    const discSummary = `${status.discs.toLocaleString()} disc${status.discs === 1 ? '' : 's'}`;
    return status.directory === directory(draft).trim() ? `Text index: ${discSummary} from this folder.` : `The text index (${discSummary}) was built for ${status.directory}. Build it again for this folder.`;
  };
  let indexStatus:IndexStatus|null = null;
  const bindIndex = (root:HTMLElement, draft:OptionsDraft) => {
    const text = root.querySelector<HTMLElement>('#local-freedb-index-status')!, build = root.querySelector<HTMLButtonElement>('#local-freedb-index-build')!, cancel = root.querySelector<HTMLButtonElement>('#local-freedb-index-cancel')!;
    if (!index) { build.disabled = true; text.textContent = 'The text index is not available here.'; return; }
    const show = (status:IndexStatus) => { indexStatus = status; if (root.isConnected) text.textContent = indexText(status,draft); };
    void index.status().then(show).catch(error => { text.textContent = String(error); });
    let job = '';
    cancel.addEventListener('click',() => { if (job) void index.cancel(job); cancel.disabled = true; });
    build.addEventListener('click',async () => {
      const folder = directory(draft).trim();
      if (!folder) { text.textContent = 'Choose the local freedb folder first.'; return; }
      job = `local-freedb-index-${nextTicket()}`;
      build.disabled = true; cancel.hidden = false; cancel.disabled = false; text.textContent = 'Building the text index…';
      const stop = await index.progress(job,(done,total) => { if (root.isConnected) text.textContent = `Building the text index: ${done} of ${total} files…`; }).catch(() => () => undefined);
      try { show(await index.build(folder,job)); }
      catch (error) { text.textContent = String(error).replace(/^Error: /,''); }
      finally { stop(); job = ''; build.disabled = false; cancel.hidden = true; }
    });
  };
  return [
    {id:'tag-sources',label:'Tag Sources',order:60,defaults:{cover_filename:'folder'},controls:[
      {kind:'text',path:'pages.tag-sources.cover_filename',label:'Default file name for cover art',placeholder:'folder'},
      {kind:'note',text:'Used by Save image to file in the tag source dialog. Placeholders such as %albumartist% - %album% are filled from the first written file of each folder, and the image type adds the extension.'},
      {kind:'select',path:'pages.tag-sources.musicbrainz_cover_size',label:'MusicBrainz cover size',choices:[['500','500 pixels (default)'],['250','250 pixels'],['1200','1200 pixels'],['original','Original size']]},
      {kind:'note',text:'Cover Art Archive images are downloaded in this size; original images can be very large. Set the Discogs personal access token with Tag Sources → Discogs access token….'},
      {kind:'checkbox',path:'pages.tag-sources.web_debug',label:'Write debug output for web sources'},
      {kind:'note',text:'Each search and album lookup of a web source (.src) writes the downloaded page and a step-by-step trace of its parser script to the debug folder inside the sources folder (Tag Sources → Open sources folder).'},
    ]},
    {id:freedbPage,label:'freedb',parent:'tag-sources',order:1,
      description:'The CDDB servers Tag Sources → freedb can ask for disc IDs. The selected server is used. Restore page defaults switches back to gnudb.',
      defaults:{protocol:'https',address:'gnudb.gnudb.org',path:'/~cddb/cddb.cgi',port:'',email:'',comment_disc_id:false,disc_id_field:false,servers:FREEDB_SERVERS.map(s => ({...s})),server:0},
      controls:[
        {kind:'group',label:'freedb server',controls:[
          {kind:'custom',id:'freedb-servers',paths:[`pages.${freedbPage}.servers`,`pages.${freedbPage}.server`,...serverKeys.map(key => `pages.${freedbPage}.${key}`)],render:serversMarkup,bind:bindServers},
          {kind:'text',path:'pages.tag-sources-freedb.email',label:'Email address',placeholder:'Only sent in the server handshake'},
        ]},
        {kind:'note',text:'gnudb only answers applications that its operator has registered. If gnudb refuses OpenTag, use another CDDB server or a local freedb folder.'},
        {kind:'group',label:'General',controls:[
          {kind:'checkbox',path:'pages.tag-sources-freedb.comment_disc_id',label:'Copy disc ID to COMMENT field'},
          {kind:'checkbox',path:'pages.tag-sources-freedb.disc_id_field',label:'Copy disc ID to DISCID field'},
        ]},
      ],
      validate:draft => {
        const page = (draft.pages[freedbPage] || {}) as Record<string,unknown>;
        const address = String(page.address ?? '').trim(), port = String(page.port ?? '').trim();
        if (address && !/^[A-Za-z0-9.-]+$/.test(address)) return 'Enter the freedb server address as a host name, for example gnudb.gnudb.org.';
        if (port && !(/^\d+$/.test(port) && Number(port) >= 1 && Number(port) <= 65535)) return 'The freedb port must be a number from 1 to 65535.';
        return undefined;
      }},
    {id:localFreedbPage,label:'Local freedb',parent:'tag-sources',order:2,
      description:'A folder of xmcd files from a freedb snapshot, with one subfolder per category (blues, classical, rock, …). Both the one-file-per-disc and the Windows (xxtoyy) layouts are read. Discs are looked up by disc ID; searching by artist and album words needs a text index of the folder.',
      defaults:{directory:''},
      controls:[{kind:'custom',id:'local-freedb-directory',paths:[directoryPath],
        render:draft => `<div class="field"><span>LOCAL FREEDB FOLDER</span><div class="action-input"><input id="local-freedb-directory" aria-label="Local freedb folder" value="${html(directory(draft))}" placeholder="Not set" autocomplete="off" spellcheck="false"><button type="button" id="local-freedb-pick">Choose folder…</button></div></div>`,
        bind:(root,draft,refresh) => {
          // The index line says whether the index covers the folder as typed.
          const changed = () => { const line = root.closest('.options-page')?.querySelector('#local-freedb-index-status'); if (line && indexStatus) line.textContent = indexText(indexStatus,draft); };
          root.querySelector<HTMLInputElement>('#local-freedb-directory')!.addEventListener('input',event => { setDirectory(draft,(event.target as HTMLInputElement).value); changed(); });
          root.querySelector<HTMLButtonElement>('#local-freedb-pick')!.addEventListener('click',async () => { const picked = await pickFolder(); if (picked) { setDirectory(draft,picked); refresh(); changed(); } });
        }},
      {kind:'custom',id:'local-freedb-index',paths:[],
        render:draft => `<div class="field"><span>TEXT INDEX</span><p id="local-freedb-index-status" role="status">${html(indexText(indexStatus,draft))}</p><div class="action-input"><button type="button" id="local-freedb-index-build">Build index</button><button type="button" id="local-freedb-index-cancel" hidden>Cancel</button></div></div>`,
        bind:(root,draft) => bindIndex(root,draft)},
      ]},
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
/** Native web-source failures keep source text separate from owned diagnostics. */
export type WebScriptError = {file:string;section:string;line:number;command:string;message:string;debug_path:string|null};
function webScriptFailure(error:unknown):WebScriptError|undefined {
  if (!error || typeof error !== 'object' || !('web_source_script' in error)) return;
  const value = error.web_source_script as Partial<WebScriptError>|null;
  if (value && ['file','section','command','message'].every(key => typeof value[key as keyof WebScriptError] === 'string') && Number.isSafeInteger(value.line) && value.line! >= 0 && (value.debug_path === null || typeof value.debug_path === 'string')) return value as WebScriptError;
}
/** Format once from source parts, even for English; t() never reparses values. */
export function webScriptErrorText(error:WebScriptError, t:(key:string,params?:Record<string,unknown>)=>string = (key,params) => key.replace(/\{(\w+)\}/g,(_,name)=>String(params?.[name] ?? `{${name}}`)), translate:(text:string)=>string = text=>text):string {
  const message = translate(error.message);
  const frame = error.line ? t('line {line} ({text}): {error}',{line:error.line,text:error.command,error:message}) : message;
  const prefix = [error.file,error.section,frame].join(' ');
  return error.debug_path === null ? prefix : prefix + '. ' + t('Debug output is in {directory}',{directory:error.debug_path});
}

let dialogTitleId = 0;

type Invoke = <T>(command:string, args?:Record<string,unknown>) => Promise<T>;
type TagSourcesHost = {
  batchError?:(error:unknown)=>string|undefined;
  /** Structured errors are rendered once and refreshed from their raw payload. */
  webScriptError?:(error:WebScriptError)=>string;
  /** Format a source-label scaffold once: installed source labels are authored. */
  sourceMessage?:(key:string,name:string,authored:boolean)=>string;
  onLanguageChange?:(listener:()=>void)=>()=>void;
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
  /** The tag source Ctrl+I opens again. */
  get lastOperation():SourceOperation { return lastSourceOperation(storage.get('opentag.tagSources.lastSource'),storage.get('opentag.tagSources.last')); }
  /** The id of the web source used last, when the last tag source was one (Ctrl+I runs it again). */
  get lastWeb():string|null { const last = storage.get('opentag.tagSources.lastSource'); return last?.startsWith('web:') ? last.slice(4) : null; }

  /** A Tag Sources menu command; `source-last` and `source-last-quick` are Ctrl+I and Ctrl+Shift+I. */
  async run(op:string) {
    const quick = op === 'source-last-quick';
    const request = sourceRequest(op === 'source-last' || quick ? this.lastOperation : op);
    if (!request) throw new Error(`Unknown tag source command ${op}`);
    await this.open(request.provider,{coversOnly:request.coversOnly,releaseId:request.releaseId,quick});
  }

  /** The Discogs token stays in its own private file, never in the shared Options store. */
  async token() {
    if (this.showing) return;
    this.showing = true;
    let credentials:TagSourceCredentials;
    try { credentials = await this.host.invoke<TagSourceCredentials>('load_tag_source_settings'); } catch (error) { this.showing = false; throw error; }
    const dialog = document.createElement('dialog');
    dialog.className = 'tag-source-dialog ts-token-dialog';
    dialog.setAttribute('aria-label','Discogs access token');
    const where = credentials.discogs_token_in_keychain ? 'in the system keychain' : 'in a private file in OpenTag\'s configuration folder';
    dialog.innerHTML = `<header>Discogs access token</header><div class="dialog-content">
      <label class="field"><span>PERSONAL ACCESS TOKEN</span><input id="ts-token" type="password" autocomplete="off" spellcheck="false" placeholder="${credentials.discogs_token_set ? 'Saved. Enter a new token to replace it.' : 'Not set'}"></label>
      <label class="action-check"><input id="ts-token-clear" type="checkbox" ${credentials.discogs_token_set ? '' : 'disabled'}> Remove the saved token</label>
      ${credentials.keychain_available ? `<label class="action-check"><input id="ts-token-keychain" type="checkbox" ${credentials.discogs_token_in_keychain ? 'checked' : ''}> Keep the token in the system keychain</label>` : ''}
      <p>Create a token on Discogs under Settings → Developers. Release lookups and searches also work without one at a lower rate limit; with a token, searches show cover thumbnails. The token is not shown again.</p>
      <p class="ts-token-place" role="status">${credentials.discogs_token_set ? `The saved token is kept ${where}.` : 'No token is saved.'}${credentials.keychain_available ? ' Changing the keychain option moves a saved token when you select Save.' : ''}</p></div>
      <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ts-save-token">Save</button></div>`;
    document.body.append(dialog);
    const error = (message:unknown) => { let p = dialog.querySelector('.dialog-error'); if (!p) { p = document.createElement('p'); p.className = 'dialog-error'; p.setAttribute('role','alert'); dialog.querySelector('.dialog-content')!.append(p); } p.textContent = String(message); };
    const token = dialog.querySelector<HTMLInputElement>('#ts-token')!, clear = dialog.querySelector<HTMLInputElement>('#ts-token-clear')!, save = dialog.querySelector<HTMLButtonElement>('#ts-save-token')!;
    const keychain = dialog.querySelector<HTMLInputElement>('#ts-token-keychain');
    const moving = () => !!keychain && keychain.checked !== credentials.discogs_token_in_keychain;
    const sync = () => { token.disabled = clear.checked; if (keychain) keychain.disabled = clear.checked; save.disabled = !clear.checked && !token.value.trim() && !moving(); };
    clear.addEventListener('change',sync); token.addEventListener('input',sync); keychain?.addEventListener('change',sync);
    dialog.querySelector('[data-close]')!.addEventListener('click',() => dialog.close());
    dialog.addEventListener('close',() => { this.showing = false; dialog.remove(); });
    save.addEventListener('click',async () => {
      save.disabled = true;
      // An empty token field keeps the saved token (possibly moving it).
      const update = {discogs_token:clear.checked ? '' : token.value.trim() || null,keychain:keychain && !clear.checked ? keychain.checked : null};
      try { await this.host.invoke('save_tag_source_settings',{update}); dialog.close(); }
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
    const coversOnly = !!options.coversOnly && !isFreedb(provider) && !web, freedb = isFreedb(provider), releaseOnly = !!options.releaseId && provider === 'discogs' && !coversOnly;
    storage.set('opentag.tagSources.lastSource',web ? `web:${web.id}` : sourceOperation({provider,coversOnly,releaseId:releaseOnly}));
    const files = this.host.files(), name = web ? options.title || web.name : coversOnly ? `${providerName(provider)} cover art` : releaseOnly ? 'Discogs Release ID' : providerName(provider), prefill = searchPrefill(files);
    const toc = discToc(files.map(f => f.length));
    const disabledKey = `opentag.tagSources.disabled.${provider}`, saveCoverKey = 'opentag.tagSources.saveCover', suggestKey = 'opentag.tagSources.suggestSorting';
    let results:SearchResult[] = [], total:number|null = null, lastQuery:Record<string,unknown>|null = null, chosen = -1;
    let release:SourceRelease|undefined, slots:(string|null)[] = [], enabled = new Set<string>(), trackField = 'TITLE';
    let fieldSelection = new Set<string>(), fieldAnchor:string|null = null, dragging = -1;
    const savedSuggest = storage.get(suggestKey);
    let suggest:SuggestSorting = savedSuggest === 'title' || savedSuggest === 'length' ? savedSuggest : '';
    let searchTicket = 0, releaseTicket = 0, imageTicket = 0, working = false, coverIndex = 0;
    // [SearchBy] fields start with their format strings evaluated for the first selected file.
    let webValues:string[] = [], webColumns:string[] = [], webRows:WebResult[] = [];
    if (web) webValues = await Promise.all(web.search_by.map(field => files[0] ? this.host.evaluate(files[0].path,field.format).catch(() => '') : Promise.resolve('')));
    const dialog = document.createElement('dialog');
    dialog.className = 'tag-source-dialog';
    const titleId = `ts-source-title-${++dialogTitleId}`;
    const nameMarkup = `<span id="${titleId}" ${web ? 'translate="no"' : ''}>${html(name)}</span>`;
    dialog.setAttribute('aria-labelledby',titleId);
    document.body.append(dialog);
    const $ = <T extends Element = HTMLElement>(selector:string) => dialog.querySelector<T>(selector)!;
    const sourceMessage = (key:string) => this.host.sourceMessage?.(key,name,!!web) ?? key.replace(/\{name\}/g,()=>name);
    let sourceStatus:string|undefined;
    const status = (message:string, sourceContext=false) => {
      sourceStatus = sourceContext ? message : undefined;
      const el = dialog.querySelector('.ts-status');
      if (el) {
        if (sourceContext) el.setAttribute('translate','no'); else el.removeAttribute('translate');
        el.textContent = sourceContext ? sourceMessage(message) : message;
      }
    };
    let scriptError:WebScriptError|undefined, sourceErrorKey:string|undefined, retainedBatch:unknown;
    const refreshError = () => {
      const p = dialog.querySelector('.dialog-error');
      if (p && retainedBatch!==undefined) p.textContent = this.host.batchError?.(retainedBatch)??String(retainedBatch);
      else if (p && sourceErrorKey) p.textContent = sourceMessage(sourceErrorKey);
      else if (p && scriptError) p.textContent = (this.host.webScriptError ?? webScriptErrorText)(scriptError);
    };
    const stopLanguage = this.host.onLanguageChange?.(() => { refreshError(); if (sourceStatus) status(sourceStatus,true); });
    const error = (message:unknown, sourceContext=false) => {
      const failure = webScriptFailure(message), batch = this.host.batchError?.(message);
      const text = failure || batch!==undefined ? '' : String(message).replace(/^Error: /,'');
      if (text.startsWith('Cancelled:')) return;
      let p = dialog.querySelector('.dialog-error');
      if (!p) { p = document.createElement('p'); p.className = 'dialog-error'; p.setAttribute('role','alert'); dialog.querySelector('.dialog-content')!.append(p); }
      scriptError = failure; sourceErrorKey = sourceContext ? text : undefined; retainedBatch=batch===undefined?undefined:message;
      if (failure || sourceContext || batch!==undefined) { p.setAttribute('translate','no'); refreshError(); }
      else { p.removeAttribute('translate'); p.textContent = text; }
    };
    const clearError = () => { scriptError = undefined; sourceErrorKey = undefined; retainedBatch=undefined; dialog.querySelector('.dialog-error')?.remove(); };
    const loadImage = async (url:string|null|undefined, image:HTMLImageElement) => {
      const ticket = imageTicket = nextTicket();
      image.hidden = true; image.removeAttribute('src');
      if (!url) return;
      try { const data = await this.host.invoke<string>('tag_source_image',{url,ticket}); if (ticket === imageTicket && dialog.open) { image.src = data; image.hidden = false; } }
      catch { /* Missing artwork leaves the preview empty. */ }
    };
    const preview = (url:string) => url.startsWith('https://') || (web && url.startsWith('http://')) ? `<button type="button" class="ts-preview" data-url="${html(url)}" title="Open the release page in your browser">Preview</button>` : '';
    dialog.addEventListener('cancel',event => { if (working) event.preventDefault(); });
    dialog.addEventListener('close',() => { stopLanguage?.(); this.showing = false; void this.host.invoke('tag_source_cancel',{ticket:nextTicket()}).catch(() => undefined); dialog.remove(); });
    dialog.addEventListener('click',event => {
      const button = (event.target as Element).closest<HTMLButtonElement>('.ts-preview');
      if (button) void this.host.invoke('tag_source_open_page',{url:button.dataset.url}).catch(error);
    });
    // Mp3tag's review shortcuts: Ctrl+E/Ctrl+D enable or disable every field,
    // Ctrl+A/Ctrl+Shift+A select all fields or invert the selection, Space
    // enables or disables the selected fields, F2 edits the focused field,
    // Alt+↑/↓ moves the focused file, Alt+Page Up/Down moves it to the
    // beginning or end of the list.
    dialog.addEventListener('keydown',event => {
      if (!release || working) return;
      const target = event.target as HTMLElement, key = event.key.toLowerCase();
      const typing = target.matches('input:not([type]),input[type=text],input[type=password],textarea');
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && (key === 'e' || key === 'd') && !coversOnly) {
        event.preventDefault(); event.stopPropagation(); toggleAll(key === 'e'); return;
      }
      if ((event.ctrlKey || event.metaKey) && !event.altKey && key === 'a' && !typing && !coversOnly) {
        event.preventDefault(); event.stopPropagation();
        const names = fieldRows(release).map(row => row.name);
        fieldSelection = event.shiftKey ? invertFields(names,fieldSelection) : new Set(names);
        showFieldSelection(); return;
      }
      const fieldRow = target.closest<HTMLTableRowElement>('.ts-fields tr[data-row]');
      if (fieldRow && target === fieldRow && !coversOnly) {
        if (event.key === ' ') { event.preventDefault(); setEnabled(fieldRow.dataset.row!,!enabled.has(fieldRow.dataset.row!)); return; }
        if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
          const next = (event.key === 'ArrowUp' ? fieldRow.previousElementSibling : fieldRow.nextElementSibling) as HTMLTableRowElement|null;
          if (next?.dataset.row) { event.preventDefault(); selectField(next.dataset.row,event.shiftKey ? 'range' : 'single'); }
          return;
        }
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
      if (web) return `<table class="ts-table ts-results" aria-label="Search results"><thead><tr>${webColumns.map(column => `<th translate="no">${html(column)}</th>`).join('')}</tr></thead><tbody translate="no">${results.map((_,i) => `<tr ${rowAttributes(i)}>${(webRows[i]?.values || []).map(value => `<td title="${html(value)}">${html(value)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
      const mb = provider === 'musicbrainz';
      const head = freedb ? '<th>Artist</th><th>Title</th><th class="ts-num">Year</th><th class="ts-wide">Genre</th><th class="ts-wide">Category</th><th class="ts-wide">Disc ID</th><th class="ts-num ts-wide">Tracks</th>'
        : `${mb ? '<th class="ts-num ts-wide">Score</th>' : ''}<th>Artist</th><th>Title</th><th class="ts-num">Year</th><th class="ts-wide">Country</th><th class="ts-wide">Label</th><th class="ts-wide">Cat. no.</th><th class="ts-wide">Format</th>${mb ? '<th class="ts-num ts-wide">Tracks</th>' : ''}`;
      const cells = (r:SearchResult) => freedb
        ? `<td title="${html(r.artist)}">${html(r.artist)}</td><td title="${html(r.title)}">${html(r.title)}</td><td class="ts-num">${html(r.year)}</td><td class="ts-wide">${html(r.format)}</td><td class="ts-wide">${html(freedbId(r).category)}</td><td class="ts-wide">${html(freedbId(r).disc)}</td><td class="ts-num ts-wide">${r.track_count ?? ''}</td>`
        : `${mb ? `<td class="ts-num ts-wide">${r.score ?? ''}</td>` : ''}<td title="${html(r.artist)}">${html(r.artist)}</td><td title="${html(r.title)}">${html(r.title)}</td><td class="ts-num">${html(r.year)}</td><td class="ts-wide">${html(r.country)}</td><td class="ts-wide" title="${html(r.label)}">${html(r.label)}</td><td class="ts-wide" title="${html(r.catalog_number)}">${html(r.catalog_number)}</td><td class="ts-wide" title="${html(r.format)}">${html(r.format)}</td>${mb ? `<td class="ts-num ts-wide">${r.track_count ?? ''}</td>` : ''}`;
      const more = total != null && results.length < total ? `<p class="ts-more"><button type="button" id="ts-more">More results</button><span>${results.length} of ${total} shown</span></p>` : '';
      return `<table class="ts-table ts-results" aria-label="Search results"><thead><tr>${head}</tr></thead><tbody translate="no">${results.map((r,i) => `<tr tabindex="${i === Math.max(chosen,0) ? 0 : -1}" data-index="${i}" aria-selected="${i === chosen}" class="${i === chosen ? 'selected' : ''}">${cells(r)}</tr>`).join('')}</tbody></table>${more}`;
    };
    const chosenMarkup = () => {
      const r = results[chosen];
      if (!r) return '';
      if (web) return `<img class="ts-thumb" alt="Release artwork" hidden><div><strong translate="no">${html(r.title)}</strong>${r.url ? `<span class="ts-url"><span translate="no">${html(r.url)}</span> ${preview(r.url)}</span>` : ''}</div>`;
      const value = (text:string) => text ? `<span translate="no">${html(text)}</span>` : '';
      const details = freedb ? [value(r.year),value(r.format),`<span data-verbatim>${html(`Disc ID ${freedbId(r).disc}`)}</span>`] : [r.year,r.country,r.label,r.catalog_number,r.format].map(value).concat(r.barcode ? `<span data-verbatim>${html(`Barcode ${r.barcode}`)}</span>` : '');
      return `${freedb ? '' : '<img class="ts-thumb" alt="Release artwork" hidden>'}<div><strong translate="no">${html(r.artist)} – ${html(r.title)}</strong><span>${details.filter(Boolean).join(' · ')}</span>${r.url ? `<span class="ts-url"><span translate="no">${html(r.url)}</span> ${preview(r.url)}</span>` : ''}</div>`;
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
      status('Searching {name}…',true);
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
    const search = async (more = false, auto = false) => {
      clearError();
      if (web) { await searchWeb(); return; }
      const mode = $<HTMLSelectElement>('#ts-mode').value as SearchMode;
      const query = more && lastQuery ? {...lastQuery,page:Number(lastQuery.page)+1} : {mode,artist:$<HTMLInputElement>('#ts-artist').value,album:$<HTMLInputElement>('#ts-album').value,text:$<HTMLInputElement>('#ts-text').value,lengths:files.map(f => f.length),page:0};
      if (!more && mode === 'release_id') { await loadRelease(query.text as string); return; }
      const ticket = searchTicket = nextTicket();
      if (more) status('Loading more results…');
      else status(freedb && mode === 'disc_id' && provider === 'freedb' ? 'Searching {name} in every category…' : 'Searching {name}…',true);
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
        status(shown ? `${shown} release${shown === 1 ? '' : 's'} found${all}. Choose one and select Next.` : freedb && mode === 'text' ? 'No disc in the text index matches these words.' : freedb ? 'No disc with this ID was found.' : 'No releases found. Try fewer words, another spelling or a different search type.');
        // Ctrl+Shift+I: a single result opens without the list.
        if (auto && !more && shown === 1) void loadRelease(results[0].id);
        if (more) { choose(chosen); dialog.querySelector<HTMLTableRowElement>(`.ts-results tr[data-index="${shown-found.results.length}"]`)?.scrollIntoView({block:'nearest'}); }
        else { $('.ts-chosen').innerHTML = ''; if (shown) choose(0); else syncSearch(); }
      } catch (e) {
        if (ticket !== searchTicket || !dialog.open) return;
        status('Search failed.'); error(e);
        dialog.querySelector<HTMLButtonElement>('#ts-more')?.removeAttribute('disabled');
      }
    };
    const loadRelease = async (input:string) => {
      if (!input.trim()) { error('Enter a {name} release ID or address.',true); return; }
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

    const modes:[SearchMode,string][] = releaseOnly ? [['release_id','Release ID or address']]
      : freedb ? [['toc','Disc ID of the selected files'],['disc_id','freedb disc ID'],...(provider === 'local_freedb' ? [['text','Artist and album words'] as [SearchMode,string]] : [])]
      : [['artist_album','Artist and album'],['text','Free text'],['release_id',provider === 'musicbrainz' ? 'Release ID (MBID) or address' : 'Release ID or address'],['barcode','Barcode'],['catalog_number','Catalog number']];
    const renderSearch = (mode?:SearchMode) => {
      clearError();
      release = undefined;
      const searchFields = web ? `<div class="ts-search ts-web-search">${web.search_by.map((field,i) => `<label class="field"><span ${field.default_label?'':'translate="no"'}>${html(field.label)}</span><input id="ts-web-${i}" value="${html(webValues[i] || '')}" autocomplete="off" spellcheck="false"></label>`).join('')}<button id="ts-search" class="ts-search-button">Search</button></div>` : '';
      dialog.innerHTML = `<header>${nameMarkup}</header><div class="dialog-content">
        ${searchFields}<div class="ts-search ${web ? 'ts-web-unused' : ''} ${modes.length === 1 ? 'ts-single-mode' : ''}"><label class="field" ${modes.length === 1 ? 'hidden' : ''}><span>SEARCH BY</span><select id="ts-mode">${modes.map(([value,label]) => `<option value="${value}">${html(label)}</option>`).join('')}</select></label>
        <div class="field-pair" id="ts-pair"><label class="field"><span>ARTIST</span><input id="ts-artist" value="${html(prefill.artist)}" autocomplete="off" spellcheck="false"></label><label class="field"><span>ALBUM</span><input id="ts-album" value="${html(prefill.album)}" autocomplete="off" spellcheck="false"></label></div>
        <label class="field" id="ts-text-field" hidden><span id="ts-text-label">SEARCH TEXT</span><input id="ts-text" autocomplete="off" spellcheck="false"></label>
        <p class="ts-toc" id="ts-toc" hidden>${html(tocText())}</p>
        <button id="ts-search" class="ts-search-button">Search</button></div>
        <p class="ts-status" role="status" aria-live="polite">${files.length} selected file${files.length === 1 ? '' : 's'}. ${freedb ? 'Look up the disc these files were ripped from.' : releaseOnly ? 'Enter the Discogs release ID shown on the release page, or its address.' : 'Search for the album these files belong to.'}</p>
        <div class="ts-result-area">${resultMarkup()}</div><div class="ts-chosen">${chosenMarkup()}</div></div>
        <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ts-next" disabled>Next</button></div>`;
      if (web) dialog.querySelector('.ts-web-unused')?.remove();
      else if (mode || (freedb && !toc)) $<HTMLSelectElement>('#ts-mode').value = mode || 'disc_id';
      if (releaseOnly) $<HTMLSelectElement>('#ts-mode').value = 'release_id';
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
      return `<table class="ts-table ts-matches" aria-label="Track and file matching"><thead><tr><th class="ts-num">#</th><th>${trackField === 'TITLE' ? 'Track' : `<span translate="no">${html(trackField)}</span>`}</th><th class="ts-num ts-wide">Time</th><th>File</th><th class="ts-num ts-wide">Time</th><th class="ts-move"><span class="visually-hidden">Move</span></th></tr></thead><tbody>${slots.map((path,index) => {
        const track = tracks[index], trackPosition = track?.position || String(index+1), warn = track && path && track.length != null && Math.abs(track.length-(files.find(f => f.path === path)?.length ?? track.length)) > 10;
        return `<tr data-index="${index}" class="${track ? '' : 'ts-unmatched'}" draggable="${path ? 'true' : 'false'}"><td class="ts-num" translate="no">${track ? html(trackPosition) : ''}</td><td>${track ? `<input class="ts-track-value" data-verbatim data-track="${index}" value="${html(trackValue(track))}" aria-label="${html(trackField)} of track ${html(trackPosition)}" autocomplete="off" spellcheck="false">` : '<em>Not written</em>'}</td><td class="ts-num ts-wide">${track ? duration(track.length) : ''}</td><td>${path ? `<span translate="no" title="${html(fileName(path))}">${html(fileName(path))}</span>` : '<em>No file</em>'}</td><td class="ts-num ts-wide ${warn ? 'ts-warn' : ''}" ${warn ? 'title="Length differs from the track by more than 10 seconds"' : ''}>${fileLength(path)}</td><td class="ts-move"><button data-move="-1" aria-label="Move ${html(fileName(path) || 'empty slot')} up" ${index === 0 || !path ? 'disabled' : ''}>↑</button><button data-move="1" aria-label="Move ${html(fileName(path) || 'empty slot')} down" ${index === slots.length-1 || !path ? 'disabled' : ''}>↓</button></td></tr>`;
      }).join('')}</tbody></table>`;
    };
    const fieldMarkup = () => `<table class="ts-table ts-fields" aria-label="Fields to write" aria-multiselectable="true"><tbody>${fieldRows(release!).map(row => `<tr data-row="${html(row.name)}" tabindex="-1" aria-selected="${fieldSelection.has(row.name)}" class="${fieldSelection.has(row.name) ? 'selected' : ''}"><td class="ts-check"><input type="checkbox" data-field="${html(row.name)}" data-verbatim="${html(row.name)}" aria-label="Write ${html(row.name)}" ${enabled.has(row.name) ? 'checked' : ''}></td><th scope="row" title="${html(row.name)}" translate="no">${html(row.name)}</th><td>${row.editable
      ? `<input class="ts-value" data-value="${html(row.name)}" value="${html(row.value)}" data-verbatim="${html(row.name)}" aria-label="${html(row.name)} value" autocomplete="off" spellcheck="false">${row.perTrack && release!.tracks.length > 1 ? '<small>all tracks</small>' : ''}`
      : `<span class="ts-summary" title="${html(row.value)}" translate="no">${html(row.value)}</span><button type="button" class="ts-edit-tracks" data-track-field="${html(row.name)}">Edit per track</button>`}</td></tr>`).join('')}</tbody></table>`;
    const renderFields = () => { const list = dialog.querySelector('.ts-field-list'); if (list) list.innerHTML = fieldMarkup(); };
    const coverTemplate = () => String(this.host.options().cover_filename ?? '').trim() || 'folder';
    const renderCover = (reload = true) => {
      const covers = release!.covers, section = $('.ts-cover');
      if (!covers.length) { section.innerHTML = `<p>No artwork is available for this release.</p>`; syncReview(); return; }
      const writing = dialog.querySelector<HTMLInputElement>('#ts-cover-write')?.checked ?? true;
      const replacing = dialog.querySelector<HTMLInputElement>('#ts-cover-replace')?.checked ?? true;
      const saving = dialog.querySelector<HTMLInputElement>('#ts-cover-save')?.checked ?? storage.get(saveCoverKey) === '1';
      section.innerHTML = `<div class="ts-cover-preview"><img alt="Selected artwork" hidden></div><div><label class="action-check"><input type="checkbox" id="ts-cover-write" ${writing ? 'checked' : ''}> Write cover art</label>
        <label class="field"><span>ARTWORK</span><select id="ts-cover-choice">${covers.map((c,i) => `<option translate="no" value="${i}" ${i === coverIndex ? 'selected' : ''}>${i+1}: ${html(c.kind)}${c.width && c.height ? ` (${c.width}×${c.height})` : ''}</option>`).join('')}</select></label>
        <label class="action-check"><input type="checkbox" id="ts-cover-replace" ${replacing ? 'checked' : ''}> Replace existing artwork</label>
        <label class="action-check" title="Saved next to the written files; set the name in Options → Tag Sources"><input type="checkbox" id="ts-cover-save" ${saving ? 'checked' : ''}> Save image to file <code translate="no">${html(coverTemplate())}</code></label>
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
        const outputPath = await this.host.invoke<string>('tag_source_save_cover',{url:cover.url,path});
        if (dialog.open) note.textContent = `Saved to ${outputPath}`;
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
    const toggleAll = (on:boolean) => { if (!release) return; enabled = on ? new Set(fieldRows(release).map(row => row.name)) : new Set(); showEnabled(); };
    const showEnabled = () => { dialog.querySelectorAll<HTMLInputElement>('.ts-fields input[type=checkbox][data-field]').forEach(box => box.checked = enabled.has(box.dataset.field!)); syncReview(); };
    /** A checkbox or Space on a selected field changes every selected field. */
    const setEnabled = (name:string, on:boolean) => { enabled = toggleEnabled(enabled,fieldSelection,name,on); showEnabled(); };
    const showFieldSelection = () => dialog.querySelectorAll<HTMLTableRowElement>('.ts-fields tr[data-row]').forEach(row => { const on = fieldSelection.has(row.dataset.row!); row.classList.toggle('selected',on); row.setAttribute('aria-selected',String(on)); });
    const selectField = (name:string, mode:'single'|'toggle'|'range') => {
      ({selected:fieldSelection,anchor:fieldAnchor} = selectFields(fieldRows(release!).map(row => row.name),fieldSelection,fieldAnchor,name,mode));
      showFieldSelection();
      dialog.querySelector<HTMLTableRowElement>(`.ts-fields tr[data-row="${CSS.escape(name)}"]`)?.focus();
    };
    const applySuggestion = (kind:SuggestSorting) => {
      suggest = kind; storage.set(suggestKey,kind);
      dialog.querySelectorAll<HTMLInputElement>('[data-suggest]').forEach(box => box.checked = box.dataset.suggest === kind);
      slots = initialSlots(release!.tracks,files,kind); renderMatches();
    };
    const showReview = (loaded:SourceRelease) => {
      clearError();
      // A web source that returns only album values (or only artwork) applies them to every selected file.
      if (web && !loaded.tracks.length) loaded.tracks = files.map((_,i) => ({position:String(i+1),disc:1,number:i+1,title:'',artist:'',length:null,fields:{}}));
      release = loaded; coverIndex = 0; fieldSelection = new Set(); fieldAnchor = null;
      slots = initialSlots(loaded.tracks,files,suggest);
      trackField = trackFields().includes('TITLE') ? 'TITLE' : trackFields()[0];
      const disabled = new Set<string>(JSON.parse(storage.get(disabledKey) || '[]'));
      enabled = coversOnly ? new Set() : new Set(fieldRows(loaded).map(row => row.name).filter(field => !disabled.has(field)));
      const fieldSection = `<section class="ts-field-section" aria-label="Fields"><div class="ts-tools"><strong>Fields</strong><button id="ts-all" title="Enable all fields (Ctrl+E)">All</button><button id="ts-none" title="Disable all fields (Ctrl+D)">None</button></div><div class="ts-field-list">${fieldMarkup()}</div><p class="ts-hint">Values can be changed before saving; separate multiple values with \\\\.</p></section>`;
      const matchSection = `<section class="ts-match-section" aria-label="Track and file matching"><div class="ts-tools"><strong>Tracks and files</strong><label class="ts-track-field">Track column <select id="ts-track-field">${trackFields().map(field => `<option translate="no" ${field === trackField ? 'selected' : ''}>${html(field)}</option>`).join('')}</select></label><button id="ts-rematch">Auto-match</button><button id="ts-order">Selection order</button></div><div class="ts-tools ts-suggest"><strong>Suggest sorting</strong><label class="action-check" title="Pair files with the closest track title; remembered for the next import"><input type="checkbox" id="ts-suggest-title" data-suggest="title" ${suggest === 'title' ? 'checked' : ''}> Title</label><label class="action-check" title="Pair files with the track of the closest length; remembered for the next import"><input type="checkbox" id="ts-suggest-length" data-suggest="length" ${suggest === 'length' ? 'checked' : ''}> Length</label></div><div class="ts-match-area"></div><p class="ts-hint">Drag a file to another track, or use the arrows (Alt+↑/↓, Alt+Page Up/Down).</p></section>`;
      dialog.innerHTML = `<header>${nameMarkup}: <span translate="no">${html(loaded.artist)} – ${html(loaded.title)}</span></header><div class="dialog-content">
        <div class="ts-review ${coversOnly ? 'ts-cover-only' : ''}">${coversOnly ? '' : fieldSection}<section class="ts-cover" aria-label="Artwork"></section></div>
        ${coversOnly ? '' : matchSection}
        ${loaded.url ? `<p class="ts-url"><span translate="no">${html(loaded.url)}</span> ${preview(loaded.url)}</p>` : ''}<p class="ts-status" role="status" aria-live="polite"></p></div>
        <div class="dialog-footer"><button id="ts-back">Back</button><button data-close>Cancel</button><button class="primary" id="ts-apply"></button></div>`;
      $('[data-close]').addEventListener('click',() => dialog.close());
      $('#ts-back').addEventListener('click',() => { clearError(); renderSearch(results.length || web ? undefined : freedb ? 'disc_id' : 'release_id'); });
      if (!coversOnly) {
        $('#ts-all').addEventListener('click',() => toggleAll(true));
        $('#ts-none').addEventListener('click',() => toggleAll(false));
        const fields = $('.ts-field-section');
        fields.addEventListener('change',event => { const box = event.target as HTMLInputElement; if (box.matches('input[type=checkbox][data-field]')) setEnabled(box.dataset.field!,box.checked); });
        fields.addEventListener('input',event => {
          const input = event.target as HTMLInputElement;
          if (!input.matches('.ts-value')) return;
          editField(loaded,input.dataset.value!,input.value);
          if (input.dataset.value === trackField) renderMatches();
        });
        fields.addEventListener('click',event => {
          const target = event.target as Element, row = target.closest<HTMLTableRowElement>('.ts-fields tr[data-row]');
          if (row && !target.closest('input,button')) { const mouse = event as MouseEvent; selectField(row.dataset.row!,mouse.shiftKey ? 'range' : mouse.ctrlKey || mouse.metaKey ? 'toggle' : 'single'); return; }
          const button = target.closest<HTMLButtonElement>('.ts-edit-tracks');
          if (!button) return;
          trackField = button.dataset.trackField!; $<HTMLSelectElement>('#ts-track-field').value = trackField;
          renderMatches(); dialog.querySelector<HTMLInputElement>('.ts-track-value')?.focus();
        });
        $('#ts-track-field').addEventListener('change',event => { trackField = (event.target as HTMLSelectElement).value; renderMatches(); });
        $('#ts-rematch').addEventListener('click',() => { slots = autoMatch(loaded.tracks,files); renderMatches(); });
        dialog.querySelectorAll<HTMLInputElement>('[data-suggest]').forEach(box => box.addEventListener('change',() => applySuggestion(box.checked ? box.dataset.suggest as SuggestSorting : '')));
        $('#ts-order').addEventListener('click',() => { slots = autoMatch(loaded.tracks,files.map(f => ({...f,track:'',title:'',name:''}))); renderMatches(); });
        const matches = $('.ts-match-area');
        matches.addEventListener('click',event => {
          const button = (event.target as Element).closest<HTMLButtonElement>('button[data-move]');
          if (button) move(Number(button.closest('tr')!.dataset.index),Number(button.dataset.move));
        });
        // Drag a file onto another track; the files in between shift by one.
        const dropRow = (event:Event) => (event.target as Element).closest<HTMLTableRowElement>('.ts-matches tr[data-index]');
        matches.addEventListener('dragstart',event => {
          const row = dropRow(event);
          if (!row || working || !slots[Number(row.dataset.index)]) { event.preventDefault(); return; }
          dragging = Number(row.dataset.index); row.classList.add('ts-dragging');
          event.dataTransfer?.setData('text/plain',fileName(slots[dragging]));
          if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
        });
        matches.addEventListener('dragover',event => {
          const row = dropRow(event);
          if (dragging < 0 || !row) return;
          event.preventDefault();
          matches.querySelectorAll('.ts-drop-target').forEach(r => r.classList.remove('ts-drop-target'));
          row.classList.add('ts-drop-target');
        });
        matches.addEventListener('drop',event => {
          const row = dropRow(event), from = dragging;
          dragging = -1;
          if (from < 0 || !row) return;
          event.preventDefault(); moveTo(from,Number(row.dataset.index));
        });
        matches.addEventListener('dragend',() => { dragging = -1; matches.querySelectorAll('.ts-dragging,.ts-drop-target').forEach(r => r.classList.remove('ts-dragging','ts-drop-target')); });
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
    else if (releaseOnly) $<HTMLInputElement>('#ts-text').focus();
    else { const artist = $<HTMLInputElement>('#ts-artist'); artist.focus(); artist.select(); }
    // Ctrl+Shift+I skips entering the search: the selection's album and artist
    // (or the disc ID of the files) are searched at once.
    const ready = web ? false : freedb ? !!toc : !releaseOnly && !!(prefill.artist || prefill.album);
    if (options.quick && ready) void search(false,true);
  }
}
