import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/tag-sources.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {freedbModes, validEmail, FREEDB_SERVERS, KEYED_SOURCES, fileAcoustIds, fileSignatures, fingerprintRecordings, fingerprintSlots, resultColumns, searchModes, sourceKeyReason, autoMatch, buildApplyItems, discToc, duration, editField, fieldRows, freedbServerList, initialSlots, invertFields, isFreedb, lastSourceOperation, moveSlot, moveSlotTo, nextTicket, normalizeTitle, providerName, searchPrefill, selectFields, similarity, sourceOperation, sourceRequest, splitValues, suggestByLength, suggestByTitle, tagSourcesOptionsPages, toggleEnabled, useFreedbServer} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

const file = (path, values = {}) => ({path, name: `${path}.flac`, title: '', artist: '', album: '', album_artist: '', track: '', disc: '', length: 200, ...values});
const track = (disc, number, title, fields = {}) => ({position: String(number), disc, number, title, artist: 'Artist', length: 200, fields: {TITLE: [title], TRACK: [`${number}/3`], ...fields}});
const release = (tracks, fields = {ALBUM: ['Album'], ALBUMARTIST: ['Artist']}) => ({provider: 'musicbrainz', id: 'x', url: 'https://musicbrainz.org/release/x', title: 'Album', artist: 'Artist', fields, tracks, covers: [], more_covers: false});

test('search prefill uses the most common album artist and album', () => {
  const files = [
    file('a', {artist: 'Guest', album_artist: 'Band', album: 'Record'}),
    file('b', {artist: 'Band', album: 'Record \\\\ Alt'}),
    file('c', {artist: 'Other', album: 'Different'}),
  ];
  assert.deepEqual(searchPrefill(files), {artist: 'Band', album: 'Record'});
  assert.deepEqual(searchPrefill([file('a')]), {artist: '', album: ''});
});

test('title similarity ignores case, accents and punctuation', () => {
  assert.equal(normalizeTitle('Café — Del Mar!'), 'cafe del mar');
  assert.equal(similarity('One More Time', 'one more time'), 1);
  assert.ok(similarity('Aerodynamic', 'Aerodynamik') > 0.85);
  assert.ok(similarity('Digital Love', 'Voyager') < 0.4);
  assert.equal(similarity('', 'x'), 0);
});

test('auto-match prefers track numbers, then titles, then selection order', () => {
  const tracks = [track(1, 1, 'One More Time'), track(1, 2, 'Aerodynamic'), track(1, 3, 'Digital Love')];
  const numbered = [file('c', {track: '03/3'}), file('a', {track: '1'}), file('b', {track: '2'})];
  assert.deepEqual(autoMatch(tracks, numbered), ['a', 'b', 'c']);
  const titled = [file('x', {name: '03 - Daft Punk - Digital Love.mp3'}), file('y', {title: 'aerodynamic'}), file('z', {name: 'unknown.mp3'})];
  assert.deepEqual(autoMatch(tracks, titled), ['z', 'y', 'x']);
  const plain = [file('p'), file('q'), file('r'), file('s')];
  assert.deepEqual(autoMatch(tracks, plain), ['p', 'q', 'r', 's'], 'extra files follow the tracks and are not written');
  assert.deepEqual(autoMatch(tracks, [file('only', {track: '2'})]), [null, 'only', null]);
});

test('auto-match does not trust ambiguous numbers', () => {
  const tracks = [track(1, 1, 'A'), track(1, 2, 'B'), track(2, 1, 'C')];
  // Without disc numbers, track 1 exists on both discs; fall back to order.
  assert.deepEqual(autoMatch(tracks, [file('m', {track: '1'}), file('n', {track: '2'}), file('o', {track: '1'})]), ['m', 'n', 'o']);
  assert.deepEqual(autoMatch(tracks, [file('m', {track: '1', disc: '2/2'}), file('n', {track: '1', disc: '1'}), file('o', {track: '2', disc: '1'})]), ['n', 'o', 'm']);
  const duplicates = [file('d1', {track: '1'}), file('d2', {track: '1'})];
  assert.deepEqual(autoMatch([track(1, 1, 'A'), track(1, 2, 'B')], duplicates), ['d1', 'd2']);
});

test('moving swaps neighbours and ignores moves past the ends', () => {
  assert.deepEqual(moveSlot(['a', 'b', null], 0, 1), ['b', 'a', null]);
  assert.deepEqual(moveSlot(['a', 'b', null], 1, 1), ['a', null, 'b']);
  assert.deepEqual(moveSlotTo(['a', 'b', 'c', null], 2, 0), ['c', 'a', 'b', null], 'Alt+Page Up keeps the order of the other files');
  assert.deepEqual(moveSlotTo(['a', 'b', 'c', null], 0, 3), ['b', 'c', null, 'a'], 'Alt+Page Down');
  assert.deepEqual(moveSlotTo(['a', 'b'], 0, 5), ['a', 'b']);
  const slots = ['a', 'b'];
  assert.deepEqual(moveSlot(slots, 0, -1), ['a', 'b']);
  assert.deepEqual(moveSlot(slots, 1, 1), ['a', 'b']);
  assert.deepEqual(slots, ['a', 'b'], 'the input is not modified');
});

test('field rows list album fields first and summarise per-track values', () => {
  const rows = fieldRows(release([track(1, 1, 'One', {COMPOSER: ['X']}), track(1, 2, 'Two', {COMPOSER: ['X']})], {ALBUM: ['Album'], ZZZ_CUSTOM: ['z'], GENRE: ['House', 'Disco']}));
  assert.deepEqual(rows.map(row => row.name), ['TITLE', 'ALBUM', 'TRACK', 'GENRE', 'COMPOSER', 'ZZZ_CUSTOM']);
  assert.deepEqual(rows.find(row => row.name === 'GENRE'), {name: 'GENRE', value: 'House \\\\ Disco', perTrack: false, editable: true});
  assert.deepEqual(rows.find(row => row.name === 'COMPOSER'), {name: 'COMPOSER', value: 'X', perTrack: true, editable: true});
  assert.deepEqual(rows.find(row => row.name === 'TITLE'), {name: 'TITLE', value: 'One … (2 values)', perTrack: true, editable: false});
});

test('apply items combine album and track fields for matched files only', () => {
  const source = release([track(1, 1, 'One', {ALBUMARTIST: ['Track override']}), track(1, 2, 'Two'), track(1, 3, 'Three')], {ALBUM: ['Album'], ALBUMARTIST: ['Artist'], EMPTY: []});
  const items = buildApplyItems(source, ['a', null, 'c', 'extra'], new Set(['ALBUM', 'ALBUMARTIST', 'TITLE', 'EMPTY']));
  assert.deepEqual(items, [
    {path: 'a', updates: {ALBUM: ['Album'], ALBUMARTIST: ['Track override'], TITLE: ['One']}},
    {path: 'c', updates: {ALBUM: ['Album'], ALBUMARTIST: ['Artist'], TITLE: ['Three']}},
  ]);
  items[0].updates.ALBUM.push('changed');
  assert.deepEqual(source.fields.ALBUM, ['Album'], 'updates are copies');
  assert.deepEqual(buildApplyItems(source, ['a'], new Set()), []);
  assert.deepEqual(buildApplyItems(source, ['a'], new Set(), true), [{path: 'a', updates: {}}]);
});

test('tickets keep increasing above the current clock and durations format as m:ss', () => {
  const first = nextTicket(), second = nextTicket();
  assert.ok(second > first && first > Date.now() * 1000 - 60_000_000);
  assert.ok(Number.isSafeInteger(second));
  assert.equal(duration(437), '7:17');
  assert.equal(duration(59.6), '1:00');
  assert.equal(duration(null), '');
});

test('disc IDs follow the CDDB algorithm for files laid out as a CD', () => {
  // libdiscid's reference disc: lead-out 206535, freedb ID 830abf0a.
  const offsets = [150, 18901, 39738, 59557, 79152, 100126, 124833, 147278, 166336, 182560, 206535];
  const lengths = offsets.slice(1).map((end, i) => (end - offsets[i]) / 75);
  assert.deepEqual(discToc(lengths), {id: '830abf0a', seconds: 2753, tracks: 10});
  assert.equal(discToc([]), null);
  assert.equal(discToc([200, 0]), null);
  assert.equal(discToc([200, Number.NaN]), null);
  assert.equal(discToc(Array(100).fill(60)), null);
  assert.equal(discToc([0.001]).id.length, 8);
});

test('suggested sorting pairs files by length or title, then keeps selection order', () => {
  const tracks = [{...track(1, 1, 'Intro'), length: 60}, {...track(1, 2, 'Long Song'), length: 400}, {...track(1, 3, 'Middle'), length: 200}];
  const files = [file('a', {length: 198}), file('b', {length: 61}), file('c', {length: 395}), file('d', {length: 10})];
  assert.deepEqual(suggestByLength(tracks, files), ['b', 'c', 'a', 'd']);
  assert.deepEqual(suggestByLength([{...track(1, 1, 'X'), length: null}, {...track(1, 2, 'Y'), length: 100}], [file('p', {length: 10}), file('q', {length: 99})]), ['p', 'q']);
  const named = [file('x', {name: '03 Middle.flac'}), file('y', {name: 'intro.flac'}), file('z', {name: 'long-song.flac'})];
  assert.deepEqual(suggestByTitle(tracks, named), ['y', 'z', 'x']);
  assert.deepEqual(suggestByTitle(tracks, [file('n', {name: '???'})]), ['n', null, null]);
});

test('web-source duration contract feeds production matching, display and ordinary LENGTH apply', async () => {
  // Rust checks its actual engine/release DTO against this same literal contract.
  // These are producer/consumer layers, not a joined process or physical save.
  const fixture = JSON.parse(await readFile(new URL('./fixtures/web-source-length-contract.json', import.meta.url), 'utf8'));
  assert.equal(fixture.version, 1);
  const source = fixture.expected_release;
  const slots = initialSlots(source.tracks, fixture.files, 'length');
  assert.deepEqual(slots, fixture.expected_slots);
  assert.deepEqual(source.tracks.map(track => duration(track.length)), fixture.expected_duration_labels);
  assert.equal(fieldRows(source).find(row => row.name === 'LENGTH').perTrack, false);
  assert.equal(fieldRows(source).some(row => row.name === '_LENGTH'), false);
  assert.deepEqual(buildApplyItems(source, slots, new Set(['LENGTH'])), fixture.expected_apply_items);
});

test('edited values apply to the album, every track or one track', () => {
  const source = release([track(1, 1, 'One', {ARTIST: ['A']}), track(1, 2, 'Two', {ARTIST: ['A']})]);
  assert.deepEqual(splitValues(' House \\\\ Disco \\\\  '), ['House', 'Disco']);
  editField(source, 'ALBUM', 'New \\\\ Alt');
  assert.deepEqual(source.fields.ALBUM, ['New', 'Alt']);
  editField(source, 'ARTIST', 'Band');
  assert.deepEqual(source.tracks.map(t => t.fields.ARTIST), [['Band'], ['Band']]);
  editField(source, 'TITLE', 'Second', 1);
  assert.equal(source.tracks[1].title, 'Second');
  assert.deepEqual(source.tracks[1].fields.TITLE, ['Second']);
  assert.deepEqual(source.tracks[0].fields.TITLE, ['One']);
  editField(source, 'TITLE', '', 0);
  assert.deepEqual(buildApplyItems(source, ['a', 'b'], new Set(['TITLE', 'ALBUM'])), [
    {path: 'a', updates: {ALBUM: ['New', 'Alt']}},
    {path: 'b', updates: {ALBUM: ['New', 'Alt'], TITLE: ['Second']}},
  ], 'an emptied value is not written');
});

test('tag source options pages cover freedb and the local freedb folder', async () => {
  const pages = tagSourcesOptionsPages(async () => '/data/freedb');
  assert.deepEqual(pages.map(page => [page.id, page.parent]), [['tag-sources', undefined], ['tag-sources-freedb', 'tag-sources'], ['tag-sources-local-freedb', 'tag-sources'], ['tag-sources-catalog', 'tag-sources'], ['tag-sources-online', 'tag-sources']]);
  const freedb = pages[1];
  assert.equal(freedb.defaults.address, 'gnudb.gnudb.org');
  const draft = port => ({pages: {'tag-sources-freedb': {address: 'gnudb.gnudb.org', port}}});
  assert.equal(freedb.validate(draft('')), undefined);
  assert.equal(freedb.validate(draft('8880')), undefined);
  assert.match(freedb.validate(draft('99999')), /port/);
  assert.match(freedb.validate({pages: {'tag-sources-freedb': {address: 'evil.com/x'}}}), /host name/);
  const custom = pages[2].controls[0];
  assert.deepEqual(custom.paths, ['pages.tag-sources-local-freedb.directory']);
  assert.match(custom.render({pages: {'tag-sources-local-freedb': {directory: '/a "b"'}}}), /value="\/a &quot;b&quot;"/);
  const index = pages[2].controls[1];
  assert.equal(index.id, 'local-freedb-index');
  assert.match(index.render({pages: {}}), /Build index/);
  const servers = freedb.controls[0].controls[0];
  assert.deepEqual(servers.paths.slice(0, 2), ['pages.tag-sources-freedb.servers', 'pages.tag-sources-freedb.server']);
  const markup = servers.render({pages: {'tag-sources-freedb': {...freedb.defaults}}});
  assert.match(markup, /gnudb \(HTTPS\)[\s\S]*gnudb \(HTTP\)/);
  assert.match(markup, /Current list of freedb servers/);
  assert.match(markup, /data-option="pages.tag-sources-freedb.address" value="gnudb.gnudb.org"/);
  assert.equal(providerName('local_freedb'), 'freedb (local)');
  assert.ok(isFreedb('freedb') && !isFreedb('discogs'));
  assert.ok(pages[0].controls.some(control => control.kind === 'checkbox' && control.path === 'pages.tag-sources.web_debug'), 'web source debug output option');
  assert.equal(providerName('web'), 'Web source');
  assert.ok(!isFreedb('web'));
});

test('Ctrl+I repeats the last tag source of any kind', () => {
  for (const op of ['source-musicbrainz', 'source-discogs', 'source-discogs-release', 'source-freedb', 'source-local-freedb', 'source-cover-musicbrainz', 'source-cover-freedb', 'source-cover-discogs']) {
    assert.equal(sourceOperation(sourceRequest(op)), op);
  }
  assert.deepEqual(sourceRequest('source-local-freedb'), {provider: 'local_freedb', coversOnly: false, releaseId: false});
  assert.deepEqual(sourceRequest('source-cover-discogs'), {provider: 'discogs', coversOnly: true, releaseId: false});
  assert.deepEqual(sourceRequest('source-cover-freedb'), {provider: 'freedb', coversOnly: true, releaseId: false});
  assert.deepEqual(sourceRequest('source-discogs-release'), {provider: 'discogs', coversOnly: false, releaseId: true});
  assert.equal(sourceRequest('source-token'), null);
  assert.equal(lastSourceOperation('source-local-freedb', 'discogs'), 'source-local-freedb');
  assert.equal(lastSourceOperation(null, 'discogs'), 'source-discogs', 'older builds remembered the provider only');
  assert.equal(lastSourceOperation('bogus', null), 'source-musicbrainz');
});

test('freedb servers are a named list whose selected entry is the server in use', () => {
  const defaults = freedbServerList({});
  assert.equal(defaults.index, 0);
  assert.deepEqual(defaults.servers.map(server => server.name), FREEDB_SERVERS.map(server => server.name));
  assert.equal(freedbServerList({protocol: 'http', address: 'gnudb.gnudb.org', path: '/~cddb/cddb.cgi', port: ''}).index, 1);
  // A server saved before the list existed is kept as a custom entry.
  const legacy = freedbServerList({protocol: 'https', address: 'cddb.example.org', path: '/cgi', port: '8080'});
  assert.equal(legacy.servers.length, 3);
  assert.deepEqual(legacy.servers[2], {name: 'Custom server', protocol: 'https', address: 'cddb.example.org', path: '/cgi', port: '8080'});
  assert.equal(legacy.index, 2);
  const page = {};
  useFreedbServer(page, legacy.servers, 2);
  assert.equal(page.address, 'cddb.example.org');
  assert.equal(page.server, 2);
  assert.deepEqual(freedbServerList(page), legacy);
  legacy.servers[2].address = 'changed.example.org';
  assert.equal(page.servers[2].address, 'cddb.example.org', 'stored as a copy');
});

test('suggest sorting when checked decides the first matching', () => {
  const tracks = [track(1, 1, 'Long Song', {}), track(1, 2, 'Short Song', {})];
  tracks[0].length = 300; tracks[1].length = 60;
  const files = [file('short', {name: 'b.flac', length: 61, track: '1'}), file('long', {name: 'a.flac', length: 299, track: '2'})];
  assert.deepEqual(initialSlots(tracks, files, ''), ['short', 'long'], 'auto-match trusts track numbers');
  assert.deepEqual(initialSlots(tracks, files, 'length'), ['long', 'short']);
  assert.deepEqual(initialSlots(tracks, files, 'title'), suggestByTitle(tracks, files));
});

test('review fields select like a list and a checkbox applies to the selection', () => {
  const names = ['TITLE', 'ARTIST', 'ALBUM', 'YEAR'];
  let state = selectFields(names, new Set(), null, 'ARTIST', 'single');
  assert.deepEqual([...state.selected], ['ARTIST']);
  state = selectFields(names, state.selected, state.anchor, 'YEAR', 'range');
  assert.deepEqual([...state.selected], ['ARTIST', 'ALBUM', 'YEAR']);
  state = selectFields(names, state.selected, state.anchor, 'ALBUM', 'toggle');
  assert.deepEqual([...state.selected], ['ARTIST', 'YEAR']);
  assert.deepEqual([...invertFields(names, state.selected)], ['TITLE', 'ALBUM']);
  const enabled = new Set(names);
  assert.deepEqual([...toggleEnabled(enabled, state.selected, 'YEAR', false)], ['TITLE', 'ALBUM']);
  assert.deepEqual([...toggleEnabled(enabled, state.selected, 'TITLE', false)], ['ARTIST', 'ALBUM', 'YEAR'], 'an unselected box changes only itself');
});

test('new sources: menu commands, search types, columns and key gating', () => {
  for (const op of ['source-itunes', 'source-deezer', 'source-lastfm', 'source-acoustid', 'source-lrclib', 'source-cover-itunes', 'source-cover-deezer', 'source-cover-fanart']) {
    assert.equal(sourceOperation(sourceRequest(op)), op, op);
  }
  assert.deepEqual(sourceRequest('source-cover-fanart'), {provider: 'fanart', coversOnly: true, releaseId: false});
  assert.deepEqual(sourceRequest('source-lrclib'), {provider: 'lrclib', coversOnly: false, releaseId: false});
  assert.equal(lastSourceOperation('source-lrclib', null), 'source-lrclib', 'Ctrl+I repeats the new sources');
  assert.deepEqual(searchModes('lrclib').map(([mode]) => mode), ['files', 'text', 'release_id']);
  assert.deepEqual(searchModes('acoustid').map(([mode]) => mode), ['fingerprint']);
  assert.deepEqual(searchModes('itunes').map(([mode]) => mode), ['artist_album', 'artist_title', 'text', 'release_id', 'barcode']);
  assert.deepEqual(searchModes('discogs', true), [['release_id', 'Release ID or address']]);
  assert.ok(searchModes('musicbrainz').some(([mode]) => mode === 'catalog_number'));
  assert.equal(resultColumns('musicbrainz'), null, 'the original sources keep their columns');
  const lyrics = resultColumns('lrclib');
  assert.deepEqual(lyrics.map(([label]) => label), ['Title', 'Artist', 'Album', 'Length', 'Lyrics']);
  assert.equal(lyrics[3][1]({track_count: 320}), '5:20');
  assert.deepEqual(resultColumns('acoustid').map(([label]) => label), ['Score', 'Artist', 'Title', 'Year', 'Country', 'Files', 'Tracks']);
  for (const provider of ['itunes', 'deezer', 'lrclib', 'acoustid', 'lastfm', 'fanart']) assert.ok(providerName(provider), provider);

  const none = {discogs_token_set: false, discogs_token_in_keychain: false, keychain_available: false};
  assert.match(sourceKeyReason('lastfm', none), /Last\.fm needs an API key/);
  assert.equal(sourceKeyReason('lastfm', {...none, lastfm_key_set: true}), '');
  assert.equal(sourceKeyReason('fanart', {...none, fanart_built_in: true}), '', 'a key built into the build is enough');
  assert.equal(sourceKeyReason('itunes', none), '', 'iTunes needs no key');
  assert.equal(sourceKeyReason('acoustid', null), '', 'unknown until the backend answers');
  assert.deepEqual(KEYED_SOURCES.map(source => source.provider), ['acoustid', 'lastfm', 'fanart']);
  assert.deepEqual(fileSignatures([{path: 'a', name: 'a.mp3', title: 'T', artist: 'A', album: 'B', album_artist: '', track: '3', disc: '1', length: NaN}]),
    [{path: 'a', title: 'T', artist: 'A', album: 'B', track: '3', disc: '1', length: 0}]);
});

test('tracks without any value (no lyrics found) are not written', () => {
  const release = {provider: 'lrclib', id: '', url: '', title: 'Lyrics', artist: '', fields: {}, covers: [], more_covers: false, tracks: [
    {position: '1', disc: 1, number: 1, title: 'A', artist: '', length: null, fields: {UNSYNCEDLYRICS: ['words']}},
    {position: '2', disc: 1, number: 2, title: 'B', artist: '', length: null, fields: {}},
  ]};
  assert.deepEqual(buildApplyItems(release, ['a', 'b'], new Set(['UNSYNCEDLYRICS'])), [{path: 'a', updates: {UNSYNCEDLYRICS: ['words']}, raw_lyrics: true}]);
});

test('Online services options page holds the per-source settings', () => {
  let opened = 0;
  const pages = tagSourcesOptionsPages(async () => null, undefined, () => { opened++; });
  const online = pages.find(page => page.id === 'tag-sources-online');
  assert.equal(online.defaults.itunes_cover_size, '1200');
  assert.equal(online.defaults.lrclib_synced, false);
  const paths = online.controls.flatMap(control => control.controls || [control]).map(control => control.path).filter(Boolean);
  assert.deepEqual(paths, ['pages.tag-sources-online.itunes_country', 'pages.tag-sources-online.itunes_cover_size', 'pages.tag-sources-online.deezer_cover_size', 'pages.tag-sources-online.lrclib_synced', 'pages.tag-sources-online.lastfm_genres', 'pages.tag-sources-online.fpcalc']);
  assert.ok(!paths.some(path => /key/.test(path)), 'API keys never go to the shared options');
  online.controls[0].controls[0].run();
  assert.equal(opened, 1);
  assert.equal(online.validate({pages: {'tag-sources-online': {itunes_country: 'nl'}}}), undefined);
  assert.match(online.validate({pages: {'tag-sources-online': {itunes_country: 'NLD'}}}), /two letters/);
});

test('AcoustID pairs each file with the track of its fingerprinted recording', () => {
  const track = (disc, number, title, recording) => ({position: `${disc}-${number}`, disc, number, title, artist: '', length: 180, fields: {TITLE: [title], MUSICBRAINZ_TRACKID: [recording]}});
  // Two discs of two tracks each.
  const tracks = [track(1, 1, 'Intro', 'rec-1'), track(1, 2, 'Song', 'rec-2'), track(2, 1, 'Reprise', 'rec-3'), track(2, 2, 'Outro', 'rec-4')];
  const file = (path, extra = {}) => ({path, name: path, title: '', artist: '', album: '', album_artist: '', track: '', disc: '', length: 180, ...extra});
  // Shuffled selection, no useful tags; c.mp3 even claims to be track 1.
  const files = [file('d.mp3'), file('c.mp3', {track: '1'}), file('a.mp3'), file('b.mp3')];
  const matches = {'a.mp3': ['rec-1'], 'b.mp3': ['rec-9', 'rec-2'], 'd.mp3': ['rec-4']};
  assert.deepEqual(fingerprintSlots(tracks, files, matches), ['a.mp3', 'b.mp3', 'c.mp3', 'd.mp3'], 'matched files on their tracks, the unmatched one on the free track');
  // Without fingerprints the files would follow their tags and selection order.
  assert.notDeepEqual(initialSlots(tracks, files, ''), ['a.mp3', 'b.mp3', 'c.mp3', 'd.mp3']);
  // Two files identified as the same recording: the first keeps it, the second falls back.
  assert.deepEqual(fingerprintSlots(tracks.slice(0, 2), [file('x.mp3'), file('y.mp3')], {'x.mp3': ['rec-2'], 'y.mp3': ['rec-2']}), ['y.mp3', 'x.mp3']);
  // Extra files stay after the tracks, unmatched tracks stay empty.
  assert.deepEqual(fingerprintSlots(tracks.slice(0, 1), [file('x.mp3'), file('y.mp3')], {'y.mp3': ['rec-1']}), ['y.mp3', 'x.mp3']);
  assert.deepEqual(fingerprintSlots(tracks.slice(0, 2), [file('x.mp3')], {'x.mp3': ['rec-2']}), [null, 'x.mp3']);
});

test('ACOUSTID_ID is written per file, only for the recording that file was identified as', () => {
  const track = (number, recording, acoustid) => ({position: String(number), disc: 1, number, title: `T${number}`, artist: '', length: 180, fields: {TITLE: [`T${number}`], MUSICBRAINZ_TRACKID: [recording], ACOUSTID_ID: [acoustid]}});
  const release = {provider: 'acoustid', id: 'r', url: '', title: '', artist: '', fields: {}, covers: [], more_covers: false,
    tracks: [track(1, 'rec-1', 'aid-a'), track(2, 'rec-2', 'aid-b'), track(3, 'rec-3', 'aid-c')]};
  // c.mp3 matched rec-3 through another AcoustID result than the one the track shows.
  const matches = {'a.mp3': [{recording: 'rec-1', acoustid: 'aid-a'}], 'b.mp3': [{recording: 'rec-2', acoustid: 'aid-b'}], 'c.mp3': [{recording: 'rec-3', acoustid: 'aid-c2'}]};
  assert.deepEqual(fingerprintRecordings(matches), {'a.mp3': ['rec-1'], 'b.mp3': ['rec-2'], 'c.mp3': ['rec-3']});
  const enabled = new Set(['TITLE', 'ACOUSTID_ID']);
  const written = slots => fileAcoustIds(buildApplyItems(release, slots, enabled), release, slots, matches).map(item => [item.path, item.updates.ACOUSTID_ID?.[0] ?? null]);
  assert.deepEqual(written(['a.mp3', 'b.mp3', 'c.mp3']), [['a.mp3', 'aid-a'], ['b.mp3', 'aid-b'], ['c.mp3', 'aid-c2']], "each file's own AcoustID");
  // The user swaps a and b by hand: neither was identified as its new track's recording.
  assert.deepEqual(written(['b.mp3', 'a.mp3', 'c.mp3']), [['b.mp3', null], ['a.mp3', null], ['c.mp3', 'aid-c2']]);
  const moved = fileAcoustIds(buildApplyItems(release, ['b.mp3', 'a.mp3', 'c.mp3'], enabled), release, ['b.mp3', 'a.mp3', 'c.mp3'], matches);
  assert.deepEqual(moved[0].updates, {TITLE: ['T1']}, 'other fields are still written');
  // An unchecked ACOUSTID_ID stays unwritten.
  assert.ok(fileAcoustIds(buildApplyItems(release, ['a.mp3'], new Set(['TITLE'])), release, ['a.mp3'], matches).every(item => !('ACOUSTID_ID' in item.updates)));
});

test('Suggest sorting and selection order keep fingerprint pairs and arrange only the rest', () => {
  const track = (number, recording, length) => ({position: String(number), disc: 1, number, title: `Song ${number}`, artist: '', length, fields: {TITLE: [`Song ${number}`], MUSICBRAINZ_TRACKID: [recording]}});
  const tracks = [track(1, 'rec-1', 100), track(2, 'rec-2', 200), track(3, 'rec-3', 300)];
  const file = (path, length) => ({path, name: path, title: '', artist: '', album: '', album_artist: '', track: '', disc: '', length});
  // c is identified as track 1; a (300 s) and b (200 s) are not identified.
  const files = [file('a.mp3', 300), file('b.mp3', 200), file('c.mp3', 100)];
  const matches = {'c.mp3': ['rec-1']};
  assert.deepEqual(initialSlots(tracks, files, 'length'), ['c.mp3', 'b.mp3', 'a.mp3']);
  assert.deepEqual(fingerprintSlots(tracks, files, matches, 'length'), ['c.mp3', 'b.mp3', 'a.mp3'], 'Length sorts the unmatched files');
  // c's fingerprint says track 2 although its length says track 1: the fingerprint wins.
  const swapped = {'c.mp3': ['rec-2']};
  assert.deepEqual(fingerprintSlots(tracks, files, swapped, 'length')[1], 'c.mp3');
  assert.deepEqual(fingerprintSlots(tracks, files, swapped, 'title')[1], 'c.mp3');
  // Selection order (any pairing function) only arranges the files without a match.
  assert.deepEqual(fingerprintSlots(tracks, files, swapped, (t, f) => f.map(x => x.path)), ['a.mp3', 'c.mp3', 'b.mp3']);
});

test('a file that loses its AcoustID and has nothing else to write is left out', () => {
  const track = (number, recording) => ({position: String(number), disc: 1, number, title: `T${number}`, artist: '', length: 180, fields: {TITLE: [`T${number}`], MUSICBRAINZ_TRACKID: [recording], ACOUSTID_ID: [`aid-${number}`]}});
  const release = {provider: 'acoustid', id: 'r', url: '', title: '', artist: '', fields: {}, covers: [], more_covers: false, tracks: [track(1, 'rec-1'), track(2, 'rec-2'), track(3, 'rec-3')]};
  const matches = {'a.mp3': [{recording: 'rec-1', acoustid: 'aid-1'}], 'b.mp3': [{recording: 'rec-2', acoustid: 'aid-2'}], 'c.mp3': [{recording: 'rec-3', acoustid: 'aid-3'}]};
  const only = new Set(['ACOUSTID_ID']);
  // a and b swapped by hand, c in place: only c has an AcoustID to write.
  const slots = ['b.mp3', 'a.mp3', 'c.mp3'];
  assert.deepEqual(fileAcoustIds(buildApplyItems(release, slots, only), release, slots, matches), [{path: 'c.mp3', updates: {ACOUSTID_ID: ['aid-3']}}]);
  // With a cover every matched file is still written.
  assert.equal(fileAcoustIds(buildApplyItems(release, slots, only), release, slots, matches, true).length, 3);
  // Nothing left at all: no items, so the review shows a status instead of failing the batch.
  assert.deepEqual(fileAcoustIds(buildApplyItems(release, ['b.mp3', 'a.mp3'], only), release, ['b.mp3', 'a.mp3'], matches), []);
});

test('sparse selected fields skip tracks without updates unless writing a cover', () => {
  const source = release([track(1, 1, 'Explicit', {ITUNESADVISORY: ['1']}), track(1, 2, 'Clean')]);
  const enabled = new Set(['ITUNESADVISORY']);
  assert.deepEqual(buildApplyItems(source, ['a', 'b'], enabled), [{path: 'a', updates: {ITUNESADVISORY: ['1']}}]);
  assert.deepEqual(buildApplyItems(source, ['a', 'b'], enabled, true), [
    {path: 'a', updates: {ITUNESADVISORY: ['1']}}, {path: 'b', updates: {}},
  ]);
});

test('LRCLIB marks its lyrics as raw while web source language prefixes retain their semantics', () => {
  const source = release([track(1, 1, 'A', {UNSYNCEDLYRICS: ['eng||literal lyrics']})]);
  source.provider = 'lrclib';
  assert.equal(buildApplyItems(source, ['a'], new Set(['UNSYNCEDLYRICS']))[0].raw_lyrics, true);
  source.provider = 'web';
  assert.equal(buildApplyItems(source, ['a'], new Set(['UNSYNCEDLYRICS']))[0].raw_lyrics, undefined);
});

test('gnudb lookups use the track layout or a text search, never a typed disc ID', () => {
  assert.deepEqual(freedbModes('freedb').map(([mode]) => mode), ['toc', 'artist_album']);
  assert.deepEqual(freedbModes('local_freedb').map(([mode]) => mode), ['toc', 'disc_id', 'text']);
  assert.deepEqual(searchModes('freedb'), freedbModes('freedb'), 'the integrated provider router preserves gnudb policy');
  assert.deepEqual(searchModes('local_freedb'), freedbModes('local_freedb'));
});

test('the freedb email address must look valid; empty sends the developer contact', () => {
  for (const email of ['john@neerdael.nl', 'Jane.Doe+tags@mail.example.co.uk']) assert.ok(validEmail(email), email);
  for (const email of ['', 'taggroove', 'me@localhost', '@example.org', 'me@', 'a b@example.org', 'me@@example.org', 'me@example.c0m', 'me@.example.org']) assert.ok(!validEmail(email), email);
  const freedb = tagSourcesOptionsPages(async () => null)[1];
  const draft = email => ({pages: {'tag-sources-freedb': {address: 'gnudb.gnudb.org', email}}});
  assert.equal(freedb.validate(draft('')), undefined);
  assert.equal(freedb.validate(draft(' me@example.org ')), undefined);
  assert.match(freedb.validate(draft('me@localhost')), /email address/);
  const email = freedb.controls[0].controls[1];
  assert.equal(email.path, 'pages.tag-sources-freedb.email');
  assert.match(email.label, /Your email address \(sent to the freedb server\)/);
  const notes = freedb.controls.filter(control => control.kind === 'note').map(control => control.text).join(' ');
  assert.match(notes, /developer contact \(john@neerdael\.nl\)/);
  assert.match(notes, /OpenTagAudioTagEditor/);
  assert.match(notes, /never submits/);
});


test('combined source routing retains new providers beside remote and local freedb policy', () => {
  const operations=['source-itunes','source-deezer','source-lastfm','source-acoustid','source-lrclib','source-cover-itunes','source-cover-deezer','source-cover-fanart','source-freedb','source-local-freedb','source-cover-freedb'];
  for(const operation of operations)assert.equal(sourceOperation(sourceRequest(operation)),operation);
  assert.deepEqual(searchModes('freedb').map(([mode])=>mode),['toc','artist_album']);
  assert.deepEqual(searchModes('local_freedb').map(([mode])=>mode),['toc','disc_id','text']);
  assert.deepEqual(searchModes('acoustid').map(([mode])=>mode),['fingerprint']);
  assert.deepEqual(searchModes('lrclib').map(([mode])=>mode),['files','text','release_id']);
});


test('fingerprint matching preserves compatible alternatives and each file’s actual AcoustID', () => {
  const tracks = ['A', 'B'].map((recording, i) => track(1, i + 1, `Track ${recording}`, {MUSICBRAINZ_TRACKID: [recording], ACOUSTID_ID: ['release-wide-id']}));
  const source = {...release(tracks), provider: 'acoustid'};
  const files = [file('flexible'), file('only-A')];
  const matches = {
    flexible: [{recording: 'A', acoustid: 'flex-A'}, {recording: 'B', acoustid: 'flex-B'}],
    'only-A': [{recording: 'A', acoustid: 'only-A-id'}],
  };
  for (const suggestion of ['', 'title', 'length']) {
    const slots = fingerprintSlots(tracks, files, fingerprintRecordings(matches), suggestion);
    assert.deepEqual(slots, ['only-A', 'flexible']);
    const items = fileAcoustIds(buildApplyItems(source, slots, new Set(['TITLE', 'ACOUSTID_ID'])), source, slots, matches);
    assert.deepEqual(items.map(item => [item.path, item.updates.TITLE, item.updates.ACOUSTID_ID]), [
      ['only-A', ['Track A'], ['only-A-id']], ['flexible', ['Track B'], ['flex-B']],
    ]);
  }
});

test('fingerprint matching augments chains, keeps free repeated tracks stable and falls back only for the rest', () => {
  const tracks = ['A', 'B', 'C'].map((recording, i) => track(1, i + 1, recording, {MUSICBRAINZ_TRACKID: [recording]}));
  const files = ['first', 'second', 'third'].map(path => file(path));
  assert.deepEqual(fingerprintSlots(tracks, files, {first: ['A', 'B'], second: ['B', 'C'], third: ['A']}, () => []), ['third', 'first', 'second']);
  const duplicate = [track(1, 1, 'First copy', {MUSICBRAINZ_TRACKID: ['A']}), track(2, 1, 'Second copy', {MUSICBRAINZ_TRACKID: ['A']}), tracks[2]];
  const selected = [...files, file('unknown')];
  let fallbackCalls = 0;
  const slots = fingerprintSlots(duplicate, selected, {first: ['A', 'A'], second: ['A'], third: ['A']}, (unmatchedTracks, unmatchedFiles) => {
    fallbackCalls++;
    assert.deepEqual(unmatchedTracks.map(track => track.title), ['C']);
    assert.deepEqual(unmatchedFiles.map(file => file.path), ['third', 'unknown']);
    return unmatchedFiles.map(file => file.path);
  });
  assert.equal(fallbackCalls, 1);
  assert.deepEqual(slots, ['first', 'second', 'third', 'unknown']);
  assert.deepEqual(fingerprintSlots(tracks.slice(0, 2), files.slice(0, 2), {first: ['A', 'B'], second: ['A', 'B']}, () => []), ['first', 'second'], 'a direct free choice preserves the earlier assignment');
});

test('fingerprint matching reaches the independent optimum for every three-file compatibility graph', () => {
  const tracks = ['A', 'B', 'C'].map((recording, i) => track(1, i + 1, recording, {MUSICBRAINZ_TRACKID: [recording]}));
  const files = ['x', 'y', 'z'].map(path => file(path));
  // Exhaustively enumerate legal assignments, independently of the production search.
  const optimum = (edges, file = 0, used = new Set()) => file === 3 ? 0 : Math.max(
    optimum(edges, file + 1, used),
    ...edges[file].filter(track => !used.has(track)).map(track => 1 + optimum(edges, file + 1, new Set([...used, track]))),
  );
  for (let mask = 0; mask < 512; mask++) {
    const edges = files.map((_, f) => tracks.flatMap((_, t) => mask & (1 << (f * 3 + t)) ? [t] : []));
    const matches = Object.fromEntries(files.map((file, f) => [file.path, edges[f].map(t => tracks[t].title)]));
    const slots = fingerprintSlots(tracks, files, matches, () => []);
    assert.equal(slots.filter(Boolean).length, optimum(edges), `maximum exact matches in graph ${mask}`);
    assert.equal(new Set(slots.filter(Boolean)).size, slots.filter(Boolean).length);
    slots.forEach((path, t) => { if (path) assert.ok(matches[path].includes(tracks[t].title), `compatible edge in graph ${mask}`); });
    assert.deepEqual(fingerprintSlots(tracks, files, matches, () => []), slots, `deterministic graph ${mask}`);
  }
});
