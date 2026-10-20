import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import ts from 'typescript';

const root=new URL('../',import.meta.url);
const read=name=>readFileSync(new URL(name,root),'utf8');

test('Android supplier notices remain source-bound and attached without a Gradle cache',()=>{
  const python=process.env.PYTHON||(process.platform==='win32'?'python':'python3');
  const result=execFileSync(python,['scripts/android-notices.py','--check'],{cwd:fileURLToPath(root),encoding:'utf8'});
  assert.match(result,/Android runtime candidates/);
});

test('About embeds the complete Android payload and retains shaded parser notices',async()=>{
  let source=read('src/audio-licenses.ts');
  source=source.replace(/^import (\w+) from '([^']+)\?raw';$/gm,(_all,name,path)=>
    `const ${name}=${JSON.stringify(readFileSync(new URL(path,new URL('src/audio-licenses.ts',root)),'utf8'))};`);
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  const {audioLicenses}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
  const payload=read('THIRD_PARTY_ANDROID.md');
  assert.equal(audioLicenses.find(x=>x.title==='Android/JVM licenses and notices')?.text,payload);
  assert.equal(audioLicenses.find(x=>x.title==='Native runtime and installer notices')?.text,read('THIRD_PARTY_NATIVE.md'));
  const inventory=JSON.parse(read('src/licenses/android-inventory.json'));
  const core=inventory.components.find(x=>x.coordinate==='com.fasterxml.jackson.core:jackson-core:2.15.3');
  for(const path of ['META-INF/NOTICE','META-INF/FastDoubleParser-LICENSE','META-INF/FastDoubleParser-NOTICE','META-INF/bigint-LICENSE'])
    assert.ok(core.materials.some(x=>x.path===path),path);
  assert.ok(read('THIRD_PARTY_NOTICES.md').includes(payload.trimEnd()));
});
