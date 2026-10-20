import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync} from 'node:fs';
import {readFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import ts from 'typescript';

const base=new URL('../',import.meta.url);
const read=name=>readFile(new URL(name,base),'utf8');
const python=process.env.PYTHON||(process.platform==='win32'?'python':'python3');

test('the dependency inventory and common/offline payload stay bound to source pins',async()=>{
  assert.ok(existsSync(new URL('src/licenses/dependency-inventory.json',base)),'No version-bound dependency notice inventory is bundled');
  const checked=spawnSync(python,['scripts/dependency-notices.py','--check'],{encoding:'utf8'});
  assert.equal(checked.status,0,checked.error?.message||checked.stderr||checked.stdout);
  const source=await read('src/audio-licenses.ts');
  const tree=ts.createSourceFile('audio-licenses.ts',source,ts.ScriptTarget.Latest,true);
  const bindings=[],values=[];
  for(const node of tree.statements.filter(ts.isImportDeclaration)){
    const name=node.importClause?.name?.text,path=node.moduleSpecifier.text;
    assert.ok(name&&path.endsWith('?raw'),'notice modules only import static text');
    bindings.push(name);values.push(await readFile(new URL(path.slice(0,-4),new URL('src/audio-licenses.ts',base)),'utf8'));
  }
  const withoutImports=tree.statements.filter(node=>!ts.isImportDeclaration(node)).map(node=>node.getText(tree)).join('\n');
  const code=ts.transpileModule(withoutImports,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
  const notices=new Function('exports',...bindings,code+'\nreturn exports.audioLicenses;')({},...values);
  const payload=await read('THIRD_PARTY_DEPENDENCIES.md');
  assert.ok(notices.some(item=>item.text===payload),'About must receive the exact generated dependency payload');
  assert.ok((await read('THIRD_PARTY_NOTICES.md')).includes(payload.trimEnd()),'installer/common notice text must contain the payload');
});

test('confirmed nested Rust and locked JavaScript notice families have exact material mappings',async()=>{
  assert.ok(existsSync(new URL('src/licenses/dependency-inventory.json',base)),'Missing dependency notice material');
  const inventory=JSON.parse(await read('src/licenses/dependency-inventory.json'));
  for(const[name,path]of [['ring','LICENSE-other-bits'],['ring','LICENSE-BoringSSL'],['encoding_rs','LICENSE-WHATWG'],['webpki-roots','LICENSE']]){
    assert.ok(inventory.components.some(component=>component.ecosystem==='cargo'&&component.name===name&&component.materials.some(item=>item.path===path)),`${name}: ${path}`);
  }
  const lock=JSON.parse(await read('package-lock.json'));
  for(const name of ['@tauri-apps/api','@tauri-apps/plugin-dialog']){
    const pin=lock.packages['node_modules/'+name];
    const component=inventory.components.find(item=>item.ecosystem==='npm'&&item.name===name);
    assert.equal(component.version,pin.version);assert.equal(component.integrity,pin.integrity);
    assert.equal(component.role,'runtime dependency');assert.ok(component.materials.length);
  }
  for(const name of ['vite','esbuild']){
    assert.equal(inventory.components.find(item=>item.ecosystem==='npm'&&item.name===name)?.role,'frontend helper provider');
  }
});
