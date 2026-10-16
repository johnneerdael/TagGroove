import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

// Exercise the actual configured JavaScript separator, including zero-width
// boundaries. Python regex splitting would not validate the browser contract.
const config = await readFile(new URL('../mkdocs.yml', import.meta.url), 'utf8');
const separator = config.match(/^\s+separator: '([^']+)'\s*$/m)?.[1];
assert.ok(separator, 'The search separator must be readable from mkdocs.yml');
const tokens = input => input.split(new RegExp(separator, 'g')).filter(Boolean);

for (const [input, expected] of [
  ['artist/title', ['artist', 'title']],
  ['field:value', ['field', 'value']],
  ['artist!=title', ['artist', 'title']],
  ['"quoted" words', ['quoted', 'words']],
  ['file.name', ['file', 'name']],
  ['AlbumArtist', ['Album', 'Artist']],
  ['&lt;artist&gt;', ['artist']],
  ['version 1.2.3', ['version', '1.2.3']],
  ['$upper(%artist%)', ['upper', 'artist']],
  ['$num(%track%,2)', ['num', 'track', '2']],
  ['[optional]', ['optional']],
]) {
  test(`guide search tokenizes ${JSON.stringify(input)} without losing its terms`, () => {
    assert.deepEqual(tokens(input), expected);
  });
}
