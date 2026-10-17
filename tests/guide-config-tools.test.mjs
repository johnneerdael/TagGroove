import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import ts from 'typescript';
const execute=promisify(execFile),root=new URL('../',import.meta.url);
const toolsPage=await readFile(new URL('docs/user-guide/customize/tools.md',root),'utf8');
const toolbarPage=await readFile(new URL('docs/user-guide/customize/toolbars.md',root),'utf8');
async function load(name){const source=await readFile(new URL(`src/${name}.ts`,root),'utf8');const javascript=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);}
const tools=await load('tools'),toolbar=await load('toolbar');
const forms=text=>{const lf=text.replace(/\r\n/g,'\n');return[lf,lf.replace(/\n/g,'\r\n')];};
const section=(text,marker)=>text.split(`<!-- ${marker}:start -->`)[1].split(`<!-- ${marker}:end -->`)[0];
const rows=(text,marker)=>section(text,marker).split(/\r?\n/).map(line=>line.trimStart()).filter(line=>line.startsWith('|')).slice(2).map(line=>line.split('|').slice(1,-1).map(value=>value.trim()));
const code=text=>{if(text.startsWith('`')&&text.endsWith('`'))return text.slice(1,-1);const element=/^<code\b[^>]*>([\s\S]*)<\/code>$/.exec(text);assert(element,'Expected written code value: '+text);return element[1];};
function blocks(text){const output=[],lines=text.split(/\r?\n/);for(let i=0;i<lines.length;i++){const start=/^( *)```text$/.exec(lines[i]);if(!start)continue;const values=[];while(++i<lines.length&&lines[i].trim()!=='```')values.push(lines[i].startsWith(start[1])?lines[i].slice(start[1].length):lines[i]);output.push(values.join('\n'));}return output;}
const rustString=text=>{let hashes='#';while(text.includes('"'+hashes))hashes+='#';return`r${hashes}"${text}"${hashes}`;};
const rustList=values=>`vec![${values.map(value=>`${rustString(value)}.to_string()`).join(',')}]`;
function declaration(source,prefix){const start=source.indexOf(prefix);assert(start>=0,prefix);const end=source.indexOf('\n}',start);assert(end>start,prefix);return source.slice(start,end+2);}

test('written parameter and path examples execute only actual native pure helpers, never program launch',async()=>{
 const source=await readFile(new URL('src-tauri/src/utilities.rs',root),'utf8');
 const declarations=[declaration(source,'pub struct Tool {').replaceAll('#[serde(default)]',''),source.match(/^pub const TOOL_LIMIT: usize = \d+;/m)[0],...['pub fn split_parameters(','fn with_separator(','pub fn expand(','pub fn invocations('].map(prefix=>declaration(source,prefix))].join('\n');
 // Deliberately exclude program/launch/run_tool: these checks cannot start a configured tool.
 assert(!declarations.includes('Command::')&&!declarations.includes('fn launch'));
 const assertions=[];
 for(const page of forms(toolsPage)){
  const file=blocks(section(page,'tools-practice-file'));assert.equal(file.length,1);const parameter=blocks(section(page,'tools-basic-parameter'));assert.deepEqual(parameter,['"%_path%"']);
  assertions.push(`let files=vec![PathBuf::from(${rustString(file[0])})];let root=Some(Path::new("/Practice"));`);
  const args=rows(page,'tools-argument-cases');assert.equal(args.length,11);
  for(const [text,expected] of args){const value=text==='(empty text)'?'':code(text);assertions.push(`let tool=Tool{name:"Never launched".into(),path:"/not-used-by-pure-check".into(),parameters:${rustString(value)}.into(),for_each_file:true};`);if(expected.startsWith('Error:'))assertions.push('assert!(invocations(&tool,&files,root).unwrap_err().contains("unmatched quote"));');else assertions.push(`assert_eq!(invocations(&tool,&files,root).unwrap(),vec![${rustList(JSON.parse(code(expected)))}]);`);}
  const paths=rows(page,'tools-placeholder-cases');assert.equal(paths.length,7);
  // The page explicitly illustrates Unix paths; native separator representation may differ.
  for(const [name,value] of paths)assertions.push(`assert_eq!(expand(${rustString(code(name))},&files[0],root).replace('\\\\',"/"),${rustString(code(value))});`);
  assertions.push('assert_eq!(expand("%_filename_rel%",&files[0],None),files[0].to_string_lossy());assert_eq!(expand("%_filename_rel%",&files[0],Some(Path::new("/Elsewhere"))),files[0].to_string_lossy());assert_eq!(expand("%_folderpath_rel%",Path::new("/Practice/one.flac"),root),"");');
  const scopes=rows(page,'tools-scope-cases');assert.equal(scopes.length,6);
  for(const [count,each,decision,expected] of scopes){if(decision==='Decline confirmation')continue;assertions.push(`let files:Vec<PathBuf>=(0..${Number(code(count))}).map(|i|PathBuf::from(format!("/practice/{i}.flac"))).collect();let tool=Tool{name:"Never launched".into(),path:"/not-used".into(),parameters:String::new(),for_each_file:${each==='On'}};`);if(expected.startsWith('Error:'))assertions.push('assert!(invocations(&tool,&files,None).unwrap_err().contains("100"));');else assertions.push(`assert_eq!(invocations(&tool,&files,None).unwrap().len(),${Number(code(expected))});`);}
 }
 const temporary=await mkdtemp(join(tmpdir(),'taggroove-guide-pure-tools-'));
 try{const input=join(temporary,'pure.rs'),output=join(temporary,process.platform==='win32'?'pure.exe':'pure');await writeFile(input,`#![allow(dead_code)]\nuse std::path::{Path,PathBuf};\n${declarations}\nfn main(){${assertions.join('\n')}}\n`);await execute('rustc',['--edition=2021',input,'-o',output]);await execute(output,[]);}
 finally{await rm(temporary,{recursive:true,force:true});}
});

test('written per-file scope and cancellation use the actual ToolsUI controller at its launch seam',async()=>{
 for(const page of forms(toolsPage))for(const [count,each,decision,expected] of rows(page,'tools-scope-cases')){
  const paths=Array.from({length:Number(code(count))},(_,i)=>`/practice/${i}.flac`),calls=[],confirmations=[],reports=[],failures=[];
  const definition={name:'Inspect track',path:'/not-launched',parameters:'"%_path%"',for_each_file:each==='On'};
  const ui=new tools.ToolsUI({values:()=>({tools:[definition],default_tool:0}),mac:false,selection:()=>paths,reason:()=>'',confirm:async()=>{confirmations.push(true);return decision==='Confirm';},run:async(tool,selected)=>{calls.push({tool,selected});if(expected.startsWith('Error:'))throw Error('Select at most 100 files to run a tool for each file');return{started:Number(code(expected))};},report:value=>reports.push(value),fail:value=>failures.push(String(value))});
  await ui.run(0);assert.equal(confirmations.length,definition.for_each_file&&paths.length>10?1:0);
  if(decision==='Decline confirmation'){assert.equal(calls.length,0);assert.equal(reports.length,0);}else{assert.equal(calls.length,1);assert.deepEqual(calls[0].selected,paths,'Controller passes selection; native helper chooses first/per-file scope');assert.equal(calls[0].tool.name,'Inspect track');assert.equal(failures.length,expected.startsWith('Error:')?1:0);}
 }
});

test('written toolbar labels preserve command/name identities and keyboard examples use the actual model',()=>{
 for(const page of forms(toolbarPage)){
  const labels=rows(page,'toolbar-label-cases');assert.equal(labels.length,3);
  const config=toolbar.normalizeCustomToolbar({shown:true,items:labels.map(([id,label])=>({id:code(id),label:'  '+code(label)+'  ',visible:true}))});
  assert.equal(config.items.length,3);
  for(const [i,row] of labels.entries()){assert.equal(config.items[i].id,code(row[0]));assert.equal(toolbar.itemLabel(config.items[i]),code(row[1]));}
  const moved=toolbar.moveToolbarItem(config.items,2,'top');assert.equal(moved.index,0);assert.equal(moved.items[0].id,'save');assert.equal(toolbar.itemLabel(moved.items[0]),'Save reviewed tags');assert.equal(moved.items[1].id,'tool:Inspect track');
  const commands={'Rename the selected item':'edit',"Toggle the selected item's visibility":'toggle','Show all items':'show-all','Hide all items':'hide-all','Move the item up':'up','Move the item down':'down','Move the item to the beginning':'top','Move the item to the end':'bottom'};
  const keys=rows(page,'toolbar-key-cases');assert.equal(keys.length,8);
  for(const mac of [false,true])for(const [chord,result] of keys){const parts=code(chord).split('+'),key=parts.at(-1),primary=parts.includes('Primary');const event={key:key==='Space'?' ':key,ctrlKey:primary&&!mac,metaKey:primary&&mac,altKey:parts.includes('Alt'),shiftKey:false};assert.equal(toolbar.toolbarKey(event,mac),commands[result],chord);}
 }
});
