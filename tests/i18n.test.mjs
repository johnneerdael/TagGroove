import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, readdir} from 'node:fs/promises';
import ts from 'typescript';
const load=async file=>{
  const source=await readFile(new URL(`../src/${file}`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const core=await load('i18n/core.ts');
const {Translator,parse,placeholders,validateCatalog,resolveLanguage,catalogMeta,normalize,languageSet,historyLabelText}=core;
const catalogs={};
for(const file of await readdir(new URL('../src/i18n/catalogs/',import.meta.url)))catalogs[file.replace('.json','')]=JSON.parse(await readFile(new URL(`../src/i18n/catalogs/${file}`,import.meta.url),'utf8'));
const english=catalogs.en;

test('translations fill placeholders and choose plural forms by number',()=>{
  const nl=new Translator('nl',{'Saved {count} files':'{count, plural, one {# bestand} other {# bestanden}} opgeslagen','Open {name}':'{name} openen'});
  assert.equal(nl.t('Saved {count} files',{count:1}),'1 bestand opgeslagen');
  assert.equal(nl.t('Saved {count} files',{count:3}),'3 bestanden opgeslagen');
  assert.equal(nl.t('Open {name}',{name:'x.mp3'}),'x.mp3 openen');
  // Unknown keys fall back to the English text with its values.
  assert.equal(nl.t('Moved {count} files',{count:2}),'Moved 2 files');
  const pl=new Translator('pl',{'{count} files':'{count, plural, one {# plik} few {# pliki} many {# plików} other {# pliku}}','{n} items':'{n, plural, =0 {nic} other {# rzeczy}}'});
  assert.deepEqual([1,2,5,22,25].map(count=>pl.t('{count} files',{count})),['1 plik','2 pliki','5 plików','22 pliki','25 plików']);
  assert.equal(pl.t('{n} items',{n:0}),'nic');
});
test('parsing reports names, including plural selectors, and rejects malformed plurals',()=>{
  assert.deepEqual([...placeholders('{a} and {b, plural, one {# {c}} other {#}}')],['a','b','c']);
  assert.deepEqual(parse('no values'),['no values']);
  assert.throws(()=>parse('{n, plural, one {x}'),/Malformed|Unclosed/);
  assert.equal(new Translator('en',{}).t('Literal {not a placeholder} stays'),'Literal {not a placeholder} stays');
});
test('numeric guards and plural decisions accept locale digits and grouping while retaining display glyphs',()=>{
  const key='{count} files',catalog={[key]:'{count, plural, =1000002 {# exact} one {# one} other {# other}}',一:'Wrong count'};
  const locales=['ar-EG','fa-IR','bn-BD','hi-IN-u-nu-deva','fr-FR','de-CH','en-IN',...Intl.supportedValuesOf('numberingSystem').map(system=>`en-u-nu-${system}`)];
  for(const locale of locales)for(const count of [1,2,1000002]){
    const shown=count.toLocaleString(locale),t=new Translator('en',catalog),expected=`${shown} ${count===1000002?'exact':count===1?'one':'other'}`;
    assert.equal(t.translate(`${shown} files`),expected,locale);assert.equal(t.t(key,{count:shown}),expected,locale);
    assert.equal(t.translate('Choose files'),'Choose files');
  }
  const t=new Translator('en',catalog);
  assert.equal(t.t(key,{count:1.5}),'1.5 other');assert.equal(t.t(key,{count:'1.5'}),'1.5 other');assert.equal(t.t(key,{count:'١٫٥'}),'١٫٥ other');
  assert.equal(new Translator('fr',{[key]:'{count, plural, one {# one} other {# other}}'}).t(key,{count:'١٫٠٠٢'}),'١٫٠٠٢ one');
});
test('rendered English text is translated by exact key or by key pattern, keeping its values and whitespace',()=>{
  const de=new Translator('de',{'Save tags':'Tags speichern','Open in {fileManager}':'In {fileManager} öffnen','Saved tags: {saved} files':'Tags gespeichert: {saved} Dateien',
    'Update check failed: {error}':'Update-Prüfung fehlgeschlagen: {error}','Select files first.':'Wählen Sie zuerst Dateien aus.','Copy of {name}':'Kopie von {name}','{count} files':'{count} Dateien','Moved {plural} to the Trash':'{plural} in den Papierkorb verschoben'});
  assert.equal(de.translate('Save tags'),'Tags speichern');
  assert.equal(de.translate('\n   Save tags  '),'\n   Tags speichern  ');
  assert.equal(de.translate('Save   tags'),'Tags speichern');
  assert.equal(de.translate('Open in Finder'),'In Finder öffnen');
  assert.equal(de.translate('Saved tags: 12 files'),'Tags gespeichert: 12 Dateien');
  // Values that are UI text themselves are translated; names of things are not.
  assert.equal(de.translate('Update check failed: Select files first.'),'Update-Prüfung fehlgeschlagen: Wählen Sie zuerst Dateien aus.');
  assert.equal(de.translate('Moved 3 files to the Trash'),'3 Dateien in den Papierkorb verschoben');
  assert.equal(de.translate('Copy of Save tags'),'Kopie von Save tags');
  assert.equal(de.translate('Something else'),'Something else');
  // Claude's Windows follow-up: marks share text nodes with translated labels.
  const marked=new Translator('de',{'Main toolbar':'Hauptsymbolleiste',Filename:'Dateiname',Help:'Hilfe'});
  assert.equal(marked.translate('✓ Main toolbar'),'✓ Hauptsymbolleiste');
  assert.equal(marked.translate('Filename ↑'),'Dateiname ↑');
  assert.equal(marked.translate('✓ Other'),'✓ Other');
  assert.equal(marked.translate('✓ Help',0,'Help'),'✓ Help');
  // Counts only match numbers, so a key such as "{count} files" leaves other text alone.
  assert.equal(de.translate('Choose files'),'Choose files');
  assert.equal(de.translate('1.234 files'),'1.234 Dateien');
  assert.equal(de.translate('12'),'12');
  assert.equal(new Translator('en',{}).active,false);
});
test('catalog validation finds missing, unknown and mismatched keys',()=>{
  const en={'@meta':{name:'English',locale:'en'},'A {x}':'A {x}','B':'B','C {n}':'C {n}'};
  const problems=validateCatalog(en,{'A {x}':'Ein {y}','C {n}':'{n, plural, single {x} other {y}}','Z':'z'});
  assert.deepEqual(problems,{missing:['B'],unknown:['Z'],placeholders:['A {x}'],invalid:['C {n}']});
});
for(const locale of ['nl','de']){
  test(`the ${locale} catalog has every key with matching placeholders`,()=>{
    const catalog=catalogs[locale];
    assert.equal(catalog['@meta'].locale,locale);
    assert.deepEqual(validateCatalog(english,catalog),{missing:[],unknown:[],placeholders:[],invalid:[]});
    if(locale==='de')assert.equal(new Translator(locale,catalog).translate('Ctrl Shift Del'),'Strg Umschalt Entf');
  });
}

test('history labels keep authored padding and only translate operation-owned labels',()=>{
  const translator=new Translator('en',{
    'Action {name}':'Run {name}','Actions ({count} groups)':'Run {count} groups',
    'Save tags':'Write tags',Help:'Wrong name','Action Help':'Wrong whole label',
  });
  const fill=(key,params)=>translator.t(key,params);
  for(const name of ['Help','Save tags','Actions (2 groups)','  Help  ','\n Help\n','Action Help']){
    assert.equal(historyLabelText(`Action ${name}`,fill),`Run ${name}`);
  }
  assert.equal(historyLabelText('Actions (0 groups)',fill),'Run 0 groups');
  assert.equal(historyLabelText('Actions (12 groups)',fill),'Run 12 groups');
  assert.equal(historyLabelText('Save tags',fill),'Write tags');
  for(const label of ['Help',' Save tags ','Action\tHelp','Actions (two groups)','Actions (2 groups) ']){
    assert.equal(historyLabelText(label,fill),label);
  }
});

test('history menu and status frames preserve names through one translation pass',()=>{
  for(const locale of ['de','nl']){
    const translator=new Translator(locale,catalogs[locale]);
    const fill=(key,params)=>translator.t(key,params);
    for(const name of ['Help','Save tags','  Help  ','\n Help\n','Action Actions (2 groups)']){
      const label=`Action ${name}`,owned=historyLabelText(label,fill);
      assert.equal(translator.translate(`Undo ${label} (2 files)`),fill('{kind} {label} ({files} files)',{kind:fill('Undo'),label:owned,files:2}));
      assert.equal(translator.translate(`Undid ${label}: 2 files`),fill('Undid {label}: {restored} files',{label:owned,restored:2}));
    }
  }
  const custom=new Translator('en',{
    'Action {name}':'Copy of {name}','Copy of {name}':'Again {name}',
    '{kind} {label} ({files} files)':'{kind}: {label} [{files}]',
    Undo:'Reverse',Help:'Wrong name',
  });
  assert.equal(custom.translate('Undo Action Help (2 files)'),'Reverse: Copy of Help [2]');
  assert.equal(historyLabelText('Help',(key,params)=>custom.t(key,params)),'Help');
});
