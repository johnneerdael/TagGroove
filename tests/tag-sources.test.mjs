import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/tag-sources.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022}}).outputText;
const {autoMatch, buildApplyItems, duration, fieldRows, moveSlot, nextTicket, normalizeTitle, searchPrefill, similarity} = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

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
  assert.deepEqual(rows.find(row => row.name === 'GENRE'), {name: 'GENRE', value: 'House · Disco', perTrack: false});
  assert.deepEqual(rows.find(row => row.name === 'COMPOSER'), {name: 'COMPOSER', value: 'X', perTrack: true});
  assert.equal(rows.find(row => row.name === 'TITLE').value, 'One … (2 values)');
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
