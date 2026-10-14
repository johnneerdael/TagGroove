type Fields = Record<string, string[]>;
export type Provider = 'musicbrainz'|'discogs';
export type SearchMode = 'artist_album'|'text'|'release_id'|'barcode'|'catalog_number';
export type SearchResult = {id:string;title:string;artist:string;year:string;country:string;label:string;catalog_number:string;format:string;track_count:number|null;score:number|null;barcode:string;thumb:string|null;url:string};
export type SourceTrack = {position:string;disc:number;number:number;title:string;artist:string;length:number|null;fields:Fields};
export type SourceCover = {url:string;thumb:string|null;kind:string;width:number|null;height:number|null};
export type SourceRelease = {provider:Provider;id:string;url:string;title:string;artist:string;fields:Fields;tracks:SourceTrack[];covers:SourceCover[];more_covers:boolean};
/** Selected file values as shown in the file list (native field casing already resolved). */
export type SourceFile = {path:string;name:string;title:string;artist:string;album:string;album_artist:string;track:string;disc:string;length:number};
export type ApplyItem = {path:string;updates:Fields};
export type ApplyCover = {url:string;replace_existing:boolean};
export type CoverSize = '250'|'500'|'1200'|'original';
export type TagSourceSettings = {discogs_token_set:boolean;musicbrainz_cover_size:CoverSize};
export type FieldRow = {name:string;value:string;perTrack:boolean};

export const providerName = (provider:Provider) => provider === 'musicbrainz' ? 'MusicBrainz' : 'Discogs';
const fieldOrder = ['TITLE','ARTIST','ALBUM','ALBUMARTIST','YEAR','TRACK','DISCNUMBER','GENRE','STYLE','COMPOSER','PUBLISHER','CATALOGNUMBER','BARCODE','ISRC'];
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
  const pairs:{index:number;path:string;score:number}[] = [];
  tracks.forEach((track,index) => { if (!slots[index]) for (const file of files) if (!used.has(file.path)) { const score = titleScore(file,track); if (score >= 0.6) pairs.push({index,path:file.path,score}); } });
  pairs.sort((a,b) => b.score-a.score || a.index-b.index);
  for (const pair of pairs) if (!slots[pair.index] && !used.has(pair.path)) { slots[pair.index] = pair.path; used.add(pair.path); }
  const rest = files.filter(file => !used.has(file.path));
  let next = 0;
  for (let index = 0; index < slots.length && next < rest.length; index++) if (!slots[index]) slots[index] = rest[next++].path;
  return [...slots,...rest.slice(next).map(file => file.path)];
}

export function moveSlot(slots:(string|null)[], index:number, delta:number) {
  const target = index+delta, result = [...slots];
  if (index < 0 || index >= slots.length || target < 0 || target >= slots.length) return result;
  [result[index],result[target]] = [result[target],result[index]];
  return result;
}

/** Every field the release offers, album-level values first in Mp3tag's usual order. */
export function fieldRows(release:SourceRelease):FieldRow[] {
  const names = new Set([...Object.keys(release.fields),...release.tracks.flatMap(t => Object.keys(t.fields))]);
  const rank = (name:string) => { const index = fieldOrder.indexOf(name); return index < 0 ? fieldOrder.length : index; };
  return [...names].sort((a,b) => rank(a)-rank(b) || a.localeCompare(b)).map(name => {
    const perTrack = release.tracks.some(t => name in t.fields);
    if (!perTrack) return {name,value:release.fields[name].join(' · '),perTrack};
    const values = release.tracks.map(t => (t.fields[name] || release.fields[name] || []).join(' · '));
    const distinct = new Set(values);
    return {name,value:distinct.size === 1 ? values[0] : `${values.find(Boolean) || ''} … (${distinct.size} values)`,perTrack};
  });
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
};
const storage = {
  get(key:string) { try { return window.localStorage.getItem(key); } catch { return null; } },
  set(key:string, value:string) { try { window.localStorage.setItem(key,value); } catch { /* Preferences are a convenience. */ } },
};

export class TagSourcesUI {
  private showing = false;
  constructor(private host:TagSourcesHost) {}
  get lastProvider():Provider { return storage.get('opentag.tagSources.last') === 'discogs' ? 'discogs' : 'musicbrainz'; }

  async options() {
    if (this.showing) return;
    this.showing = true;
    let settings:TagSourceSettings;
    try { settings = await this.host.invoke<TagSourceSettings>('load_tag_source_settings'); } catch (error) { this.showing = false; throw error; }
    const dialog = document.createElement('dialog');
    dialog.className = 'tag-source-dialog';
    dialog.setAttribute('aria-label','Tag Sources options');
    dialog.innerHTML = `<header>Tag Sources options</header><div class="dialog-content">
      <label class="field"><span>DISCOGS PERSONAL ACCESS TOKEN</span><input id="ts-token" type="password" autocomplete="off" spellcheck="false" placeholder="${settings.discogs_token_set ? 'Saved. Enter a new token to replace it.' : 'Not set'}"></label>
      <label class="action-check"><input id="ts-token-clear" type="checkbox" ${settings.discogs_token_set ? '' : 'disabled'}> Remove the saved token</label>
      <p>Create a token on Discogs under Settings → Developers. Release lookups and searches also work without one at a lower rate limit; with a token, searches show cover thumbnails. The token is stored in OpenTag's configuration folder and is not shown again.</p>
      <label class="field"><span>MUSICBRAINZ COVER SIZE</span><select id="ts-cover-size">${(['250','500','1200','original'] as CoverSize[]).map(size => `<option value="${size}" ${size === settings.musicbrainz_cover_size ? 'selected' : ''}>${size === 'original' ? 'Original size' : `${size} pixels`}</option>`).join('')}</select></label>
      <p>Cover Art Archive images are downloaded in this size. Original images can be very large.</p></div>
      <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ts-save-options">Save</button></div>`;
    document.body.append(dialog);
    const error = (message:unknown) => { let p = dialog.querySelector('.dialog-error'); if (!p) { p = document.createElement('p'); p.className = 'dialog-error'; p.setAttribute('role','alert'); dialog.querySelector('.dialog-content')!.append(p); } p.textContent = String(message); };
    const token = dialog.querySelector<HTMLInputElement>('#ts-token')!, clear = dialog.querySelector<HTMLInputElement>('#ts-token-clear')!;
    clear.addEventListener('change',() => { token.disabled = clear.checked; });
    dialog.querySelector('[data-close]')!.addEventListener('click',() => dialog.close());
    dialog.addEventListener('close',() => { this.showing = false; dialog.remove(); });
    dialog.querySelector('#ts-save-options')!.addEventListener('click',async () => {
      const save = dialog.querySelector<HTMLButtonElement>('#ts-save-options')!;
      save.disabled = true;
      try {
        await this.host.invoke('save_tag_source_settings',{update:{discogs_token:clear.checked ? '' : token.value.trim() || null,musicbrainz_cover_size:dialog.querySelector<HTMLSelectElement>('#ts-cover-size')!.value}});
        dialog.close();
      } catch (e) { error(e); save.disabled = false; }
    });
    dialog.showModal();
    token.focus();
  }

  async open(provider:Provider) {
    if (this.showing) return;
    const reason = this.host.reason();
    if (reason) throw new Error(reason);
    this.showing = true;
    storage.set('opentag.tagSources.last',provider);
    const files = this.host.files(), name = providerName(provider), prefill = searchPrefill(files);
    const disabledKey = `opentag.tagSources.disabled.${provider}`;
    let results:SearchResult[] = [], chosen = -1, release:SourceRelease|undefined, slots:(string|null)[] = [], enabled = new Set<string>();
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
    dialog.addEventListener('cancel',event => { if (working) event.preventDefault(); });
    dialog.addEventListener('close',() => { this.showing = false; void this.host.invoke('tag_source_cancel',{ticket:nextTicket()}).catch(() => undefined); dialog.remove(); });

    const resultMarkup = () => results.length ? `<table class="ts-table ts-results" aria-label="Search results"><thead><tr>${provider === 'musicbrainz' ? '<th class="ts-num ts-wide">Score</th>' : ''}<th>Artist</th><th>Title</th><th class="ts-num">Year</th><th class="ts-wide">Country</th><th class="ts-wide">Label</th><th class="ts-wide">Cat. no.</th><th class="ts-wide">Format</th>${provider === 'musicbrainz' ? '<th class="ts-num ts-wide">Tracks</th>' : ''}</tr></thead><tbody>${results.map((r,i) => `<tr tabindex="${i === Math.max(chosen,0) ? 0 : -1}" data-index="${i}" aria-selected="${i === chosen}" class="${i === chosen ? 'selected' : ''}">${provider === 'musicbrainz' ? `<td class="ts-num ts-wide">${r.score ?? ''}</td>` : ''}<td title="${html(r.artist)}">${html(r.artist)}</td><td title="${html(r.title)}">${html(r.title)}</td><td class="ts-num">${html(r.year)}</td><td class="ts-wide">${html(r.country)}</td><td class="ts-wide" title="${html(r.label)}">${html(r.label)}</td><td class="ts-wide" title="${html(r.catalog_number)}">${html(r.catalog_number)}</td><td class="ts-wide" title="${html(r.format)}">${html(r.format)}</td>${provider === 'musicbrainz' ? `<td class="ts-num ts-wide">${r.track_count ?? ''}</td>` : ''}</tr>`).join('')}</tbody></table>` : '';
    const chosenMarkup = () => { const r = results[chosen]; return r ? `<img class="ts-thumb" alt="Release artwork" hidden><div><strong>${html(r.artist)} – ${html(r.title)}</strong><span>${[r.year,r.country,r.label,r.catalog_number,r.format,r.barcode && `Barcode ${r.barcode}`].filter(Boolean).map(html).join(' · ')}</span><span class="ts-url">${html(r.url)}</span></div>` : ''; };

    const choose = (index:number, focus = false) => {
      chosen = index;
      dialog.querySelectorAll<HTMLTableRowElement>('.ts-results tbody tr').forEach(row => { const on = Number(row.dataset.index) === index; row.classList.toggle('selected',on); row.setAttribute('aria-selected',String(on)); row.tabIndex = on ? 0 : -1; if (on && focus) { row.focus(); row.scrollIntoView({block:'nearest'}); } });
      $('.ts-chosen').innerHTML = chosenMarkup();
      const thumb = dialog.querySelector<HTMLImageElement>('.ts-thumb');
      if (thumb) void loadImage(results[index]?.thumb,thumb);
      syncSearch();
    };
    const syncSearch = () => {
      const mode = $<HTMLSelectElement>('#ts-mode').value as SearchMode, pair = mode === 'artist_album';
      $('#ts-pair').hidden = !pair; $('#ts-text-field').hidden = pair;
      $('#ts-text-label').textContent = mode === 'release_id' ? 'RELEASE ID OR ADDRESS' : mode === 'barcode' ? 'BARCODE' : mode === 'catalog_number' ? 'CATALOG NUMBER' : 'SEARCH TEXT';
      $<HTMLButtonElement>('#ts-search').textContent = mode === 'release_id' ? 'Look up' : 'Search';
      dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLButtonElement>('.dialog-content input,.dialog-content select,.dialog-content button').forEach(control => control.disabled = working);
      $<HTMLButtonElement>('#ts-next').disabled = working || chosen < 0;
    };
    const search = async () => {
      clearError();
      const mode = $<HTMLSelectElement>('#ts-mode').value as SearchMode;
      const query = {mode,artist:$<HTMLInputElement>('#ts-artist').value,album:$<HTMLInputElement>('#ts-album').value,text:$<HTMLInputElement>('#ts-text').value};
      if (mode === 'release_id') { await loadRelease(query.text); return; }
      const ticket = searchTicket = nextTicket();
      status(`Searching ${name}…`);
      try {
        const found = await this.host.invoke<SearchResult[]>('tag_source_search',{provider,query,ticket});
        if (ticket !== searchTicket || !dialog.open) return;
        results = found; chosen = -1;
        $('.ts-result-area').innerHTML = resultMarkup();
        status(found.length ? `${found.length} release${found.length === 1 ? '' : 's'} found. Choose one and select Next.` : 'No releases found. Try fewer words, another spelling or a different search type.');
        $('.ts-chosen').innerHTML = '';
        if (found.length) choose(0);
        else syncSearch();
      } catch (e) { if (ticket === searchTicket && dialog.open) { status('Search failed.'); error(e); } }
    };
    const loadRelease = async (input:string) => {
      if (!input.trim()) { error(`Enter a ${name} release ID or address.`); return; }
      clearError();
      const ticket = releaseTicket = nextTicket();
      working = true; syncSearch(); status('Loading release…');
      try {
        const loaded = await this.host.invoke<SourceRelease>('tag_source_release',{provider,input,ticket});
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

    const renderSearch = () => {
      release = undefined;
      dialog.innerHTML = `<header>${html(name)}</header><div class="dialog-content">
        <div class="ts-search"><label class="field"><span>SEARCH BY</span><select id="ts-mode"><option value="artist_album">Artist and album</option><option value="text">Free text</option><option value="release_id">${provider === 'musicbrainz' ? 'Release ID (MBID) or address' : 'Release ID or address'}</option><option value="barcode">Barcode</option><option value="catalog_number">Catalog number</option></select></label>
        <div class="field-pair" id="ts-pair"><label class="field"><span>ARTIST</span><input id="ts-artist" value="${html(prefill.artist)}" autocomplete="off" spellcheck="false"></label><label class="field"><span>ALBUM</span><input id="ts-album" value="${html(prefill.album)}" autocomplete="off" spellcheck="false"></label></div>
        <label class="field" id="ts-text-field" hidden><span id="ts-text-label">SEARCH TEXT</span><input id="ts-text" autocomplete="off" spellcheck="false"></label>
        <button id="ts-search" class="ts-search-button">Search</button></div>
        <p class="ts-status" role="status" aria-live="polite">${files.length} selected file${files.length === 1 ? '' : 's'}. Search for the album these files belong to.</p>
        <div class="ts-result-area">${resultMarkup()}</div><div class="ts-chosen">${chosenMarkup()}</div></div>
        <div class="dialog-footer"><button data-close>Cancel</button><button class="primary" id="ts-next" disabled>Next</button></div>`;
      $('[data-close]').addEventListener('click',() => dialog.close());
      $('#ts-mode').addEventListener('change',() => { syncSearch(); (dialog.querySelector<HTMLInputElement>('#ts-text-field:not([hidden]) input') || $<HTMLInputElement>('#ts-artist')).focus(); });
      $('#ts-search').addEventListener('click',() => void search());
      dialog.querySelectorAll<HTMLInputElement>('.ts-search input').forEach(input => input.addEventListener('keydown',event => { if (event.key === 'Enter') { event.preventDefault(); void search(); } }));
      $('.ts-result-area').addEventListener('click',event => { const row = (event.target as Element).closest<HTMLTableRowElement>('tr[data-index]'); if (row) choose(Number(row.dataset.index)); });
      $('.ts-result-area').addEventListener('dblclick',event => { const row = (event.target as Element).closest<HTMLTableRowElement>('tr[data-index]'); if (row && !working) void loadRelease(results[Number(row.dataset.index)].id); });
      $('.ts-result-area').addEventListener('keydown',event => {
        const key = (event as KeyboardEvent).key;
        if (key === 'ArrowDown' || key === 'ArrowUp') { event.preventDefault(); choose(Math.max(0,Math.min(results.length-1,chosen+(key === 'ArrowDown' ? 1 : -1))),true); }
        if (key === 'Enter' && chosen >= 0 && !working) { event.preventDefault(); void loadRelease(results[chosen].id); }
      });
      $('#ts-next').addEventListener('click',() => { if (chosen >= 0) void loadRelease(results[chosen].id); });
      syncSearch();
      if (chosen >= 0) choose(chosen);
    };

    const fileName = (path:string|null) => path ? files.find(f => f.path === path)?.name || path : '';
    const fileLength = (path:string|null) => path ? duration(files.find(f => f.path === path)?.length ?? null) : '';
    const matchMarkup = () => {
      const tracks = release!.tracks;
      return `<table class="ts-table ts-matches" aria-label="Track and file matching"><thead><tr><th class="ts-num">#</th><th>Track</th><th class="ts-num ts-wide">Time</th><th>File</th><th class="ts-num ts-wide">Time</th><th class="ts-move"><span class="visually-hidden">Move</span></th></tr></thead><tbody>${slots.map((path,index) => {
        const track = tracks[index], warn = track && path && track.length != null && Math.abs(track.length-(files.find(f => f.path === path)?.length ?? track.length)) > 10;
        return `<tr data-index="${index}" class="${track ? '' : 'ts-unmatched'}"><td class="ts-num">${track ? html(track.position || String(index+1)) : ''}</td><td title="${html(track?.title)}">${track ? html(track.title) : '<em>Not written</em>'}</td><td class="ts-num ts-wide">${track ? duration(track.length) : ''}</td><td title="${html(fileName(path))}">${path ? html(fileName(path)) : '<em>No file</em>'}</td><td class="ts-num ts-wide ${warn ? 'ts-warn' : ''}" ${warn ? 'title="Length differs from the track by more than 10 seconds"' : ''}>${fileLength(path)}</td><td class="ts-move"><button data-move="-1" aria-label="Move ${html(fileName(path) || 'empty slot')} up" ${index === 0 || !path ? 'disabled' : ''}>↑</button><button data-move="1" aria-label="Move ${html(fileName(path) || 'empty slot')} down" ${index === slots.length-1 || !path ? 'disabled' : ''}>↓</button></td></tr>`;
      }).join('')}</tbody></table>`;
    };
    const fieldMarkup = () => `<div class="ts-field-list"><table class="ts-table ts-fields" aria-label="Fields to write"><tbody>${fieldRows(release!).map(row => `<tr><td class="ts-check"><input type="checkbox" data-field="${html(row.name)}" aria-label="Write ${html(row.name)}" ${enabled.has(row.name) ? 'checked' : ''}></td><th scope="row">${html(row.name)}</th><td title="${html(row.value)}">${html(row.value)}${row.perTrack && release!.tracks.length > 1 ? ' <small>per track</small>' : ''}</td></tr>`).join('')}</tbody></table></div>`;
    const renderCover = (reload = true) => {
      const covers = release!.covers, section = $('.ts-cover');
      if (!covers.length) { section.innerHTML = '<p>No artwork is available for this release.</p>'; return; }
      const writing = dialog.querySelector<HTMLInputElement>('#ts-cover-write')?.checked ?? true;
      const replacing = dialog.querySelector<HTMLInputElement>('#ts-cover-replace')?.checked ?? true;
      section.innerHTML = `<div class="ts-cover-preview"><img alt="Selected artwork" hidden></div><div><label class="action-check"><input type="checkbox" id="ts-cover-write" ${writing ? 'checked' : ''}> Write cover art</label>
        <label class="field"><span>ARTWORK</span><select id="ts-cover-choice">${covers.map((c,i) => `<option value="${i}" ${i === coverIndex ? 'selected' : ''}>${i+1}: ${html(c.kind)}${c.width && c.height ? ` (${c.width}×${c.height})` : ''}</option>`).join('')}</select></label>
        <label class="action-check"><input type="checkbox" id="ts-cover-replace" ${replacing ? 'checked' : ''}> Replace existing artwork</label></div>`;
      $('#ts-cover-choice').addEventListener('change',event => { coverIndex = Number((event.target as HTMLSelectElement).value); void loadImage(covers[coverIndex].thumb || covers[coverIndex].url,$<HTMLImageElement>('.ts-cover-preview img')); });
      $('#ts-cover-write').addEventListener('change',syncReview);
      if (reload || !dialog.querySelector<HTMLImageElement>('.ts-cover-preview img')?.src) void loadImage(covers[coverIndex].thumb || covers[coverIndex].url,$<HTMLImageElement>('.ts-cover-preview img'));
      syncReview();
    };
    const items = () => buildApplyItems(release!,slots,enabled);
    const coverChoice = ():ApplyCover|null => { const write = dialog.querySelector<HTMLInputElement>('#ts-cover-write'), cover = release?.covers[coverIndex]; return write?.checked && cover ? {url:cover.url,replace_existing:$<HTMLInputElement>('#ts-cover-replace').checked} : null; };
    const syncReview = () => {
      const count = items().length, cover = coverChoice(), nothing = !enabled.size && !cover;
      const apply = $<HTMLButtonElement>('#ts-apply');
      apply.textContent = `Save tags to ${count} file${count === 1 ? '' : 's'}`;
      dialog.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLButtonElement>('.dialog-content input,.dialog-content select,.dialog-content button,.dialog-footer button').forEach(control => { if (!control.matches('[data-move]')) control.disabled = working; });
      dialog.querySelectorAll<HTMLButtonElement>('[data-move]').forEach(button => { const index = Number(button.closest('tr')!.dataset.index), delta = Number(button.dataset.move); button.disabled = working || !slots[index] || index+delta < 0 || index+delta >= slots.length; });
      const replace = dialog.querySelector<HTMLInputElement>('#ts-cover-replace'); if (replace) replace.disabled = working || !cover;
      apply.disabled = working || !count || nothing;
      const unmatched = files.length - count, missing = release!.tracks.length - count;
      $('.ts-status').textContent = working ? 'Writing tags…' : nothing ? 'Choose at least one field or the cover art to write.' : `${count} of ${release!.tracks.length} tracks have a file.${missing > 0 ? ` ${missing} track${missing === 1 ? ' has' : 's have'} no file.` : ''}${unmatched > 0 ? ` ${unmatched} selected file${unmatched === 1 ? ' is' : 's are'} not matched and will not change.` : ''} Saving writes the checked fields immediately.`;
    };
    const renderMatches = () => { $('.ts-match-area').innerHTML = matchMarkup(); syncReview(); };
    const showReview = (loaded:SourceRelease) => {
      release = loaded; coverIndex = 0;
      slots = autoMatch(loaded.tracks,files);
      const disabled = new Set<string>(JSON.parse(storage.get(disabledKey) || '[]'));
      enabled = new Set(fieldRows(loaded).map(row => row.name).filter(field => !disabled.has(field)));
      dialog.innerHTML = `<header>${html(name)}: ${html(loaded.artist)} – ${html(loaded.title)}</header><div class="dialog-content">
        <div class="ts-review"><section class="ts-field-section" aria-label="Fields"><div class="ts-tools"><strong>Fields</strong><button id="ts-all">All</button><button id="ts-none">None</button></div>${fieldMarkup()}</section><section class="ts-cover" aria-label="Artwork"></section></div>
        <section class="ts-match-section" aria-label="Track and file matching"><div class="ts-tools"><strong>Tracks and files</strong><button id="ts-rematch">Auto-match</button><button id="ts-order">Selection order</button></div><div class="ts-match-area"></div></section>
        <p class="ts-url">${html(loaded.url)}</p><p class="ts-status" role="status" aria-live="polite"></p></div>
        <div class="dialog-footer"><button id="ts-back">Back</button><button data-close>Cancel</button><button class="primary" id="ts-apply"></button></div>`;
      $('[data-close]').addEventListener('click',() => dialog.close());
      $('#ts-back').addEventListener('click',() => { clearError(); if (results.length) renderSearch(); else { renderSearch(); $<HTMLSelectElement>('#ts-mode').value = 'release_id'; syncSearch(); } });
      const toggleAll = (on:boolean) => { enabled = on ? new Set(fieldRows(loaded).map(row => row.name)) : new Set(); dialog.querySelectorAll<HTMLInputElement>('.ts-fields input').forEach(box => box.checked = on); syncReview(); };
      $('#ts-all').addEventListener('click',() => toggleAll(true));
      $('#ts-none').addEventListener('click',() => toggleAll(false));
      $('.ts-fields').addEventListener('change',event => { const box = event.target as HTMLInputElement; if (box.checked) enabled.add(box.dataset.field!); else enabled.delete(box.dataset.field!); syncReview(); });
      $('#ts-rematch').addEventListener('click',() => { slots = autoMatch(loaded.tracks,files); renderMatches(); });
      $('#ts-order').addEventListener('click',() => { slots = autoMatch(loaded.tracks,files.map(f => ({...f,track:'',title:'',name:''}))); renderMatches(); });
      $('.ts-match-area').addEventListener('click',event => {
        const button = (event.target as Element).closest<HTMLButtonElement>('button[data-move]');
        if (!button || working) return;
        const index = Number(button.closest('tr')!.dataset.index), delta = Number(button.dataset.move);
        slots = moveSlot(slots,index,delta); renderMatches();
        const row = `.ts-matches tr[data-index="${index+delta}"]`;
        (dialog.querySelector<HTMLButtonElement>(`${row} button[data-move="${delta}"]:not(:disabled)`) || dialog.querySelector<HTMLButtonElement>(`${row} button[data-move]:not(:disabled)`))?.focus();
      });
      $('#ts-apply').addEventListener('click',async () => {
        clearError();
        const reason = this.host.reason(), chosenItems = items(), cover = coverChoice();
        if (reason) { error(reason); return; }
        if (!chosenItems.length) { error('Match at least one file to a track.'); return; }
        storage.set(disabledKey,JSON.stringify(fieldRows(loaded).map(row => row.name).filter(field => !enabled.has(field))));
        working = true; syncReview();
        try { await this.host.apply(chosenItems,cover); dialog.close(); }
        catch (e) { working = false; if (dialog.open) { syncReview(); error(e); } }
      });
      renderMatches(); renderCover();
      $<HTMLButtonElement>('#ts-apply').focus();
    };

    renderSearch();
    dialog.showModal();
    const artist = $<HTMLInputElement>('#ts-artist');
    artist.focus(); artist.select();
  }
}
