import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

// Execute the actual main-window apply callback and column specification builder.
// Native I/O, confirmations and refreshes are controlled at their boundaries.
const source=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
const parsed=ts.createSourceFile('main.ts',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const declaration=parsed.statements.filter(ts.isVariableStatement).flatMap(s=>s.declarationList.declarations)
  .find(d=>d.name.getText(parsed)==='tagSourcesUI');
const apply=declaration.initializer.arguments[0].properties.find(p=>p.name?.getText(parsed)==='apply').initializer.getText(parsed);
const columnSpecs=parsed.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='columnSpecs').getText(parsed);
const factory=`export function createApply(columns, snapshots={}) {
  const calls=[],refreshes=[];
  const tableColumns=()=>columns;
  const columnValues={fresh:(path,template)=>snapshots[path]?.[template]};
  const invoke=async(command,args)=>{calls.push({command,args});return {entries:[],errors:[]};};
  const preserveCovers=async()=>false,run=async(_label,work)=>work(),accept=()=>{};
  const reconcileWorkingEntries=async paths=>refreshes.push(paths);
  const errors=[],publicMessage=String,displayName=String;
  ${columnSpecs}
  const apply=${apply};
  return {apply,calls,refreshes};
}`;
const javascript=ts.transpileModule(factory,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {createApply}=await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);

test('tag-source apply passes active custom columns for the imported files',async()=>{
  const columns=[{name:'Custom cover',value:'%ALBUM%-%MY_CODE%'},{name:'Codec',value:'%_CODEC%'}];
  const host=createApply(columns,{
    '/a.wav':{'%ALBUM%-%MY_CODE%':'Old-A'},
    '/b.wav':{'%_CODEC%':'PCM'},
    '/unselected.wav':{'%ALBUM%-%MY_CODE%':'Do not send'},
  });
  const items=[{path:'/b.wav',updates:{ALBUM:['New B']}},{path:'/a.wav',updates:{ALBUM:['New A']}}];
  const cover={url:'https://coverartarchive.org/cover.png',save_as:'$columnValues(,)'};
  await host.apply(items,cover);
  assert.deepEqual(host.calls,[{command:'tag_source_apply',args:{items,cover:{...cover,preserve_existing:false},columns:[
    {name:'Custom cover',template:'%ALBUM%-%MY_CODE%',values:{'/a.wav':'Old-A'}},
    {name:'Codec',template:'%_CODEC%',values:{'/b.wav':'PCM'}},
  ]}}]);
});
