import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/filter-fields.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { buildFilterFields } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const entry = (technical) => ({ path: '/music/album/track.FLAC', filename: 'track.FLAC', duration: 3.03, bitrate: 900, sample_rate: 44100, size: 45056, technical });

test('measured audio, file-time and cover properties reach filters and override legacy projections', () => {
  const measured = {
    _CODEC: ['FLAC'], _BITSPERSAMPLE: ['24'], _LENGTH_SECONDS: ['3'],
    _FILE_SIZE: ['44.0 KB'], _FILE_SIZE_BYTES: ['45056'],
    _FILE_MOD_DATETIME_RAW: ['1700000000'], _FILE_MOD_DATE: ['14/11/2023'],
    _COVERS: ['1'], _COVER_WIDTH: ['1024'], _COVER_HEIGHT: ['768'], _COVER_SIZE: ['9876'],
  };
  const fields = buildFilterFields(entry(measured), { TITLE: ['Actual title'], _CODEC: ['stale raw value'] });
  for (const [name, values] of Object.entries(measured)) assert.deepEqual(fields[name], values);
  assert.deepEqual(fields.TITLE, ['Actual title']);
  assert.deepEqual(fields._SAMPLERATE, ['44100']);
  assert.deepEqual(fields._PATH, ['/music/album/track.FLAC']);
});

test('older entries without measured fields keep the exact existing filter fallbacks', () => {
  const expected = {
    title: ['Untouched case'], _FILENAME: ['track'], _FILENAME_EXT: ['track.FLAC'],
    _PATH: ['/music/album/track.FLAC'], _FOLDERPATH: ['/music/album/'], _DIRECTORY: ['album'],
    _EXTENSION: ['FLAC'], _LENGTH_SECONDS: ['3.03'], _BITRATE: ['900'],
    _SAMPLERATE: ['44100'], _FILE_SIZE: ['45056'],
  };
  assert.deepEqual(buildFilterFields(entry(undefined), { title: ['Untouched case'] }), expected);
  assert.deepEqual(buildFilterFields(entry({}), { title: ['Untouched case'] }), expected);
  const windows = { ...entry(), path: String.raw`C:\Music\Album\track.FLAC` };
  assert.deepEqual(buildFilterFields(windows, {})._FOLDERPATH, [String.raw`C:\Music\Album` + '\\']);
  assert.deepEqual(buildFilterFields(windows, {})._DIRECTORY, ['Album']);
});

test('measured maps cannot replace app, selection, clock or export-owned context even with mixed case', () => {
  const reserved = [
    '_APP', '_COUNTER', '_FILETYPE', '_DATE', '_DATETIME', '_WORKINGDIR', '_WORKINGPATH',
    '_FILENAME_REL', '_FOLDERPATH_REL', '_CRC', '_MD5', '_MD5AUDIO', '_MAX_COUNTER',
    '_TOTAL_FILES', '_TOTAL_SIZE', '_TOTAL_SIZE_RAW', '_TOTAL_TIME', '_TOTAL_TIME2',
    '_TOTAL_TIME_RAW', '_PLAYLIST_FILENAME', '_PLAYLIST_FILENAME_EXT', '_PLAYLIST_FOLDERPATH',
  ];
  const owned = Object.fromEntries(reserved.map(name => [name, ['owner value']]));
  const untrusted = Object.fromEntries(reserved.map(name => [name.toLowerCase(), ['file override']]));
  const fields = buildFilterFields(entry({ ...untrusted, _codec: ['measured codec'] }), owned);
  for (const name of reserved) assert.deepEqual(fields[name], ['owner value']);
  assert.deepEqual(fields._CODEC, ['measured codec']);
  const absent = buildFilterFields(entry(untrusted), {});
  for (const name of reserved) assert.equal(Object.hasOwn(absent, name), false);
});

test('raw metadata names, multiline multivalues and pending raw edits remain separate and unchanged', () => {
  const raw = { artist: ['Mémoire', 'Guest 猫'], COMMENT: ['first line\nsecond line'], DATE: ['2024-02-03'], YEAR: ['1998'], 'maß': ['native name'] };
  const rawBefore = structuredClone(raw);
  const technical = { TITLE: ['must not become a stored tag'], artist: ['must not replace raw'], _codec: ['FLAC'], _COVER_DESCRIPTION: ['line one\nline two'] };
  const technicalBefore = structuredClone(technical);
  const fields = buildFilterFields(entry(technical), raw);
  for (const [name, values] of Object.entries(raw)) assert.deepEqual(fields[name], values);
  assert.equal(Object.hasOwn(fields, 'ARTIST'), false);
  assert.equal(Object.hasOwn(fields, 'TITLE'), false);
  assert.deepEqual(raw, rawBefore);
  assert.deepEqual(technical, technicalBefore);
  fields._CODEC.push('context-only edit');
  assert.deepEqual(technical._codec, ['FLAC']);
});

test('unmeasured properties and invented aliases stay absent while zero measurements survive', () => {
  const fields = buildFilterFields(entry({ _COVERS: ['0'], _BITSPERSAMPLE: ['16'], _FILE_MOD_DATETIME_RAW: ['-100'] }), {});
  assert.deepEqual(fields._COVERS, ['0']);
  assert.deepEqual(fields._BITSPERSAMPLE, ['16']);
  assert.deepEqual(fields._FILE_MOD_DATETIME_RAW, ['-100']);
  for (const name of ['_BITS_PER_SAMPLE', '_COVER_WIDTH', '_COVER_HEIGHT', '_FILE_CREATE_DATETIME_RAW', '_MP4_CREATION_TIME', '_VIDEO_CODEC', '_VBR']) assert.equal(Object.hasOwn(fields, name), false);
});
