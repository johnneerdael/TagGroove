import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import ts from 'typescript';
const python=process.env.PYTHON||(process.platform==='win32'?'python':'python3');

test('installer notice header states the combined application license and preserves MIT component terms',async()=>{
  const read=path=>readFile(new URL(`../${path}`,import.meta.url),'utf8');
  const {license}=JSON.parse(await read('package.json'));
  const notices=(await read('THIRD_PARTY_NOTICES.md')).replace(/\r\n/g,'\n');
  const header=notices.split('\n## ')[0];
  assert.ok(header.includes(`The combined TagGroove application is distributed under ${license}.`));
  assert.doesNotMatch(header,/TagGroove application source is MIT licensed/);
  const component=notices.split('## Independently authored TagGroove components (MIT)\n')[1]?.split('\n## ')[0];
  assert.equal(component?.trim(),(await read('LICENSE')).trim(),'retain the complete original MIT grant for independent components');
});

test('installer notices use a current escaped Unicode RTF without changing the source text',async()=>{
  const bundle=JSON.parse(await readFile(new URL('../src-tauri/tauri.conf.json',import.meta.url),'utf8')).bundle;
  assert.equal(bundle.licenseFile,'../THIRD_PARTY_NOTICES.rtf');
  const checked=spawnSync(python,['scripts/installer-license.py','--check'],{encoding:'utf8'});
  assert.equal(checked.status,0,checked.error?.message||checked.stderr||checked.stdout);
  const notices=(await readFile(new URL('../THIRD_PARTY_NOTICES.md',import.meta.url),'utf8')).replace(/\r\n/g,'\n');
  assert.equal(rtfText(await readFile(new URL('../THIRD_PARTY_NOTICES.rtf',import.meta.url),'ascii')),notices);
  assert.match(notices,/Help > About TagGroove/);
  const metadata=await readFile(new URL('../src/audio-licenses.ts',import.meta.url),'utf8');
  const files=[...metadata.matchAll(/from ['"]([^'"]+)\?raw['"]/g)].map(match=>match[1]);
  assert.ok(files.length>0,'check every imported component notice without a fixed completeness count');
  for(const file of files){
    const text=(await readFile(new URL(file,new URL('../src/audio-licenses.ts',import.meta.url)),'utf8')).replace(/\r\n/g,'\n');
    assert.ok(notices.includes(text.trimEnd()),`retain the complete ${file} notice`);
  }
});

function rtfText(rtf){
  assert.doesNotMatch(rtf,/[^\t\r\n\x20-\x7e]/,'RTF transport must be ASCII');
  const marker='\\fs18 ',start=rtf.indexOf(marker);
  assert.ok(start>=0);assert.ok(rtf.endsWith('}\n'));
  const body=rtf.slice(start+marker.length,-2);let text='';
  for(let i=0;i<body.length;){
    if(body[i]==='\\'){
      const rest=body.slice(i),unicode=/^\\u(-?\d+)\?/.exec(rest);
      if(unicode){text+=String.fromCharCode(Number(unicode[1])&65535);i+=unicode[0].length;}
      else if(rest.startsWith('\\par\n')){text+='\n';i+=5;}
      else if(rest.startsWith('\\tab ')){text+='\t';i+=5;}
      else if(/^\\[\\{}]/.test(rest)){text+=body[i+1];i+=2;}
      else assert.fail(`unexpected control: ${rest.slice(0,30)}`);
    }else{assert.ok(!'{}\r\n'.includes(body[i]),'unescaped RTF syntax');text+=body[i++];}
  }
  return text;
}

test('RTF renders copyright, non-BMP characters and literal control-looking text verbatim',async()=>{
  const directory=await mkdtemp(join(tmpdir(),'taggroove-license-'));
  try{
    const input=join(directory,'input.md'),output=join(directory,'license.rtf');
    const text='Copyright © 2026 Zoë — 音楽 🎵\r\nLiteral {braces} and \\rtf1 \\object\tremain text.\nPage one.\fPage two.\n';
    await writeFile(input,text);
    const result=spawnSync(python,['scripts/installer-license.py','--input',input,'--output',output],{encoding:'utf8'});
    assert.equal(result.status,0,result.error?.message||result.stderr||result.stdout);
    assert.equal(rtfText(await readFile(output,'ascii')),text.replace(/\r\n/g,'\n'));
  }finally{await rm(directory,{recursive:true,force:true});}
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
  assert.equal(bundle.licenseFile,'../THIRD_PARTY_NOTICES.rtf');
});
