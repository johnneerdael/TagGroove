import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/tag-sources.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {freedbModes, validEmail, searchModes, FREEDB_SERVERS, autoMatch, buildApplyItems, discToc, duration, editField, fieldRows, freedbServerList, initialSlots, invertFields, isFreedb, lastSourceOperation, moveSlot, moveSlotTo, nextTicket, normalizeTitle, providerName, searchPrefill, selectFields, similarity, sourceOperation, sourceRequest, splitValues, suggestByLength, suggestByTitle, tagSourcesOptionsPages, toggleEnabled, useFreedbServer} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

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
  assert.deepEqual(buildApplyItems(source, ['a'], new Set()), [{path: 'a', updates: {}}]);
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
  assert.deepEqual(pages.map(page => [page.id, page.parent]), [['tag-sources', undefined], ['tag-sources-freedb', 'tag-sources'], ['tag-sources-local-freedb', 'tag-sources']]);
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

test('gnudb lookups use the track layout or a text search, never a typed disc ID', () => {
  assert.deepEqual(freedbModes('freedb').map(([mode]) => mode), ['toc', 'artist_album']);
  assert.deepEqual(freedbModes('local_freedb').map(([mode]) => mode), ['toc', 'disc_id', 'text']);
  assert.deepEqual(searchModes('freedb'), freedbModes('freedb'), 'the integrated provider router preserves gnudb policy');
  assert.deepEqual(searchModes('local_freedb'), freedbModes('local_freedb'));
});

test('the freedb email address must look valid; empty sends the developer contact', () => {
  for (const email of ['john@neerdael.nl', 'Jane.Doe+tags@mail.example.co.uk']) assert.ok(validEmail(email), email);
  for (const email of ['', 'opentag', 'me@localhost', '@example.org', 'me@', 'a b@example.org', 'me@@example.org', 'me@example.c0m', 'me@.example.org']) assert.ok(!validEmail(email), email);
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
