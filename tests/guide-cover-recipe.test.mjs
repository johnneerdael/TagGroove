import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/cover-model.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {managedFromSet,planItems,planChanged,presence,PICTURE_TYPES}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const read=name=>readFile(new URL(`../docs/user-guide/tagging/recipes/${name}.md`,import.meta.url),'utf8');
function rows(text,marker){return text.split(`<!-- ${marker}:start -->`)[1].split(`<!-- ${marker}:end -->`)[0].split(/\r?\n/).filter(line=>line.startsWith('|')).slice(2).map(line=>line.split('|').slice(1,-1).map(cell=>cell.trim()));}
const code=value=>{assert.ok(value.startsWith('`')&&value.endsWith('`'));return value.slice(1,-1);};
const forms=text=>[text.replace(/\r\n/g,'\n'),text.replace(/\r?\n/g,'\r\n')];
test('written front-cover cleanup keeps partial membership in actual managed-cover plans',async()=>{
 for(const text of forms(await read('keep-only-front-cover'))){
  const catalog=rows(text,'front-catalog'),files=rows(text,'front-membership');assert.equal(catalog.length,4);assert.equal(files.length,3);
  const before=files.map(row=>JSON.parse(code(row[1])));
  const set={files:files.length,errors:[],covers:catalog.map(([id,type,description],index)=>({digest:code(id),picture_type:PICTURE_TYPES.find(([,label])=>label===type)?.[0],description:code(description),mime:'image/png',size:100,width:2,height:2,files:before.flatMap((labels,i)=>labels.includes(code(id))?[i]:[]),index,data_base64:''}))};
  const initial=managedFromSet(set),kept=initial.filter(cover=>cover.picture_type==='CoverFront');
  assert.equal(planChanged(initial,kept),true);
  const plan=planItems(kept);assert.equal(plan.length,2);assert.ok(plan.every(item=>item.all_files===false&&item.image===null&&item.original));
  const expected=files.flatMap(row=>JSON.parse(code(row[2])));assert.deepEqual(plan.map(item=>item.original.digest),expected);
  const labels=rows(text,'front-presence');assert.equal(labels.length,2);
  for(const [id,label]of labels){const cover=kept.find(item=>item.original.digest===code(id));assert.equal(presence(cover).text,code(label));assert.equal(presence({...cover,all_files:true}).text,'In all 3 files');}
 }
});
test('written import role uses the current model label and identifies Front Cover',async()=>{
 for(const text of forms(await read('embed-folder-jpg'))){const settings=Object.fromEntries(rows(text,'embed-settings'));assert.equal(PICTURE_TYPES.find(([,label])=>label===settings['Import cover as'])?.[0],'CoverFront');assert.equal(settings['Delete existing covers'],'Off');}
});
