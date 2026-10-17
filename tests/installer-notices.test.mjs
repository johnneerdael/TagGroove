import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

test('the complete installer notices remain safe for Tauri plain-text to ANSI RTF conversion',async()=>{
  const notices=(await readFile(new URL('../THIRD_PARTY_NOTICES.md',import.meta.url),'utf8')).replace(/\r\n/g,'\n');
  assert.doesNotMatch(notices,/[^\t\r\n\x20-\x7e]/,'Unicode requires an explicitly escaped RTF license; raw UTF-8 is misread as Windows-1252');
  assert.doesNotMatch(notices,/[\\{}]/,'RTF control characters require an explicitly escaped RTF license');
  assert.match(notices,/Help > About TagGroove/);
  const metadata=await readFile(new URL('../src/audio-licenses.ts',import.meta.url),'utf8');
  const files=[...metadata.matchAll(/from ['"]\.\/licenses\/audio\/([^'"]+)\?raw['"]/g)].map(match=>match[1]);
  assert.equal(files.length,7,'check every imported component notice');
  for(const file of files){
    const text=(await readFile(new URL(`../src/licenses/audio/${file}`,import.meta.url),'utf8')).replace(/\r\n/g,'\n');
    assert.ok(notices.includes(text.trimEnd()),`retain the complete ${file} notice`);
  }
});
