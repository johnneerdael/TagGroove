import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
// Names someone gave (columns, tag panel labels, toolbar items) are rendered inside
// translate="no" or with data-verbatim, so a name such as "Help" is never translated.
const load=async file=>{
  const source=await readFile(new URL(`../src/${file}`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const columns=await load('columns.ts'),panel=await load('tag-panel.ts'),toolbar=await load('toolbar.ts');
const german=text=>({Title:'Titel',Help:'Hilfe'})[text]??text;

test('a column keeps the name it was given; an untouched default name follows the language',()=>{
  columns.setColumnNameTranslation(german);
  const [filename,title]=columns.defaultLayout().columns;
  const help={...title,id:'help',name:'Help'},renamed={...title,name:'Help'};
  assert.equal(columns.columnName(title),'Titel');
  assert.equal(columns.columnName(help),'Help');
  assert.equal(columns.columnName(renamed),'Help');
  const markup=columns.headerMarkup([filename,title,help],[]);
  assert.match(markup,/<span translate="no">Titel<\/span>/);
  assert.match(markup,/data-verbatim="Help" title="Help — click to sort/);
  assert.match(markup,/<span translate="no">Help<\/span>/);
  columns.setColumnNameTranslation(name=>name);
});
test('tag panel labels: defaults follow the language, typed labels stay as typed',()=>{
  assert.equal(panel.defaultName({field:'TITLE',name:'Tit&le'}),true);
  assert.equal(panel.defaultName({field:'TITLE',name:'Help'}),false);
  assert.equal(panel.defaultName({field:'HELP',name:'Help'}),false);
  assert.equal(panel.localMnemonic('Tit&le',german).label,'Titel');
  assert.equal(panel.localMnemonic('Tit&le',german).html,'Tite<u>l</u>');
});
test('toolbar items that someone renamed, tools and action groups are names',()=>{
  const commands=Object.keys(toolbar.TOOLBAR_COMMANDS);
  assert.equal(toolbar.userLabel({id:commands[0],visible:true}),false);
  assert.equal(toolbar.userLabel({id:commands[0],label:'Help',visible:true}),true);
  assert.equal(toolbar.userLabel({id:`${toolbar.TOOL_PREFIX}Help`,visible:true}),true);
  assert.equal(toolbar.userLabel({id:`${toolbar.ACTION_PREFIX}Help`,visible:true}),true);
});
