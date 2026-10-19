import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

test('the complete installer notices remain safe for Tauri plain-text to ANSI RTF conversion',async()=>{
  const notices=(await readFile(new URL('../THIRD_PARTY_NOTICES.md',import.meta.url),'utf8')).replace(/\r\n/g,'\n');
  assert.doesNotMatch(notices,/[^\t\r\n\x20-\x7e]/,'Unicode requires an explicitly escaped RTF license; raw UTF-8 is misread as Windows-1252');
  assert.doesNotMatch(notices,/[\\{}]/,'RTF control characters require an explicitly escaped RTF license');
  assert.match(notices,/Help > About TagGroove/);
  const metadata=await readFile(new URL('../src/audio-licenses.ts',import.meta.url),'utf8');
  const files=[...metadata.matchAll(/from ['"]([^'"]+)\?raw['"]/g)].map(match=>match[1]);
  assert.equal(files.length,8,'check every imported component notice');
  for(const file of files){
    const text=(await readFile(new URL(file,new URL('../src/audio-licenses.ts',import.meta.url)),'utf8')).replace(/\r\n/g,'\n');
    assert.ok(notices.includes(text.trimEnd()),`retain the complete ${file} notice`);
  }
});

test('Boost notices use the compiled vendor license and recorded provenance',async()=>{
  const base=new URL('../',import.meta.url);
  const read=path=>readFile(new URL(path,base),'utf8');
  const provenance=JSON.parse(await read('crates/regex-compat/vendor/boost-regex/SOURCE.json'));
  const license=(await read('crates/regex-compat/vendor/boost-regex/LICENSE_1_0.txt')).replace(/\r\n/g,'\n');
  const metadata=await read('src/audio-licenses.ts');
  const imported=[...metadata.matchAll(/import (\w+) from ['"]([^'"]+)\?raw['"]/g)]
    .find(([, ,path])=>path==='../crates/regex-compat/vendor/boost-regex/LICENSE_1_0.txt');
  assert.ok(imported,'offline notices must import the compiled vendor license directly');
  const heading=`Boost.Regex ${provenance.version} — license and notices`;
  const source=ts.createSourceFile('audio-licenses.ts',metadata,ts.ScriptTarget.Latest,true);
  const list=source.statements.filter(ts.isVariableStatement)
    .flatMap(node=>[...node.declarationList.declarations])
    .find(node=>ts.isIdentifier(node.name)&&node.name.text==='audioLicenses')?.initializer;
  assert.ok(list&&ts.isArrayLiteralExpression(list));
  assert.ok(list.elements.some(node=>{
    if(!ts.isObjectLiteralExpression(node))return false;
    const properties=new Map(node.properties.filter(ts.isPropertyAssignment).map(property=>[property.name.getText(source),property.initializer]));
    const title=properties.get('title'),text=properties.get('text');
    return title&&ts.isStringLiteral(title)&&title.text===heading&&text&&ts.isIdentifier(text)&&text.text===imported[1];
  }),'show the vendored license with its recorded version');
  const notices=(await read('THIRD_PARTY_NOTICES.md')).replace(/\r\n/g,'\n');
  const section=notices.split(`## Boost.Regex ${provenance.version} (BSL-1.0)\n`)[1]?.split('\n## ')[0];
  assert.ok(section,'common notices must identify the recorded Boost version');
  for(const text of [license.trimEnd(),provenance.source,provenance.archive_sha256])assert.ok(section.includes(text),'common notices must retain license and source provenance');
  assert.match(await read('crates/regex-compat/Cargo.toml'),/^license = "MIT AND BSL-1\.0"$/m);
  const bundle=JSON.parse(await read('src-tauri/tauri.conf.json')).bundle;
  assert.ok(bundle.license.split(' AND ').includes('BSL-1.0'));
  assert.equal(bundle.licenseFile,'../THIRD_PARTY_NOTICES.md');
});
