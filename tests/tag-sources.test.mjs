import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/tag-sources.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {autoMatch, buildApplyItems, discToc, duration, editField, fieldRows, isFreedb, moveSlot, nextTicket, normalizeTitle, providerName, searchPrefill, similarity, splitValues, suggestByLength, suggestByTitle, tagSourcesOptionsPages} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

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
  assert.equal(providerName('local_freedb'), 'freedb (local)');
  assert.ok(isFreedb('freedb') && !isFreedb('discogs'));
});
