import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
import {extractBackend,extractBackendSource} from '../scripts/i18n-extract.mjs';

const read=path=>readFile(new URL(`../${path}`,import.meta.url),'utf8');
const js=ts.transpileModule(await read('src/i18n/core.ts'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {Translator}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const nativeKeys=[
  'A previous fingerprint is still stopping. Try again shortly.',
  'Fingerprinting was cancelled.',
  'Fingerprinting reached its time limit.',
  'Fingerprinting requires a local audio file.',
  'There is not enough audio to calculate a fingerprint.',
  'The full audio duration is unavailable.',
  'The audio file could not be read: {value}',
  'The audio could not be decoded: {value}',
  'The audio could not be fingerprinted: {value}',
];
const override='The optional fpcalc override was not found at {path}. Clear it to use built-in fingerprinting in Options → Tag Sources → Online services.';
const timeout='Fingerprinting reached its time limit. The audio worker may still be stopping; try again shortly.';

test('native fingerprint Display diagnostics are discovered and registered',async()=>{
  const discovered=extractBackendSource(await read('crates/audio-fingerprint/src/lib.rs'));
  const snapshot=JSON.parse(await read('src/i18n/backend-keys.json'));
  for(const key of nativeKeys){assert.ok(discovered.includes(key),key);assert.ok(snapshot.includes(key),key);}
  assert.deepEqual(extractBackend(),snapshot);
});

for(const locale of ['de','nl'])test(`fingerprint setup and diagnostics translate in ${locale} without changing paths or decoder details`,async()=>{
  const catalog=JSON.parse(await read(`src/i18n/catalogs/${locale}.json`));
  const translator=new Translator(locale,catalog);
  for(const key of [...nativeKeys,override,timeout,'Built-in audio fingerprinting','Optional fpcalc override (desktop)','Leave empty for built-in fingerprinting','Sources and releases']){
    assert.equal(typeof catalog[key],'string',key);
    assert.notEqual(translator.t(key),key,key);
  }
  const raw='Sources and releases $& {value}';
  for(const key of [override,'The audio could not be decoded: {value}']){
    const role=key===override?'path':'value';
    const message=key.replace(`{${role}}`,()=>raw);
    assert.equal(translator.translate(message),translator.t(key,{[role]:raw}));
    assert.ok(translator.translate(message).includes(raw));
  }
});
