import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, readdir} from 'node:fs/promises';
import ts from 'typescript';
import {extract, englishCatalog} from '../scripts/i18n-extract.mjs';
const load=async file=>{
  const source=await readFile(new URL(`../src/${file}`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const core=await load('i18n/core.ts');
const {Translator,parse,placeholders,validateCatalog,resolveLanguage,catalogMeta,normalize,languageSet}=core;
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
  // Counts only match numbers, so a key such as "{count} files" leaves other text alone.
  assert.equal(de.translate('Choose files'),'Choose files');
  assert.equal(de.translate('1.234 files'),'1.234 Dateien');
  assert.equal(de.translate('12'),'12');
  assert.equal(new Translator('en',{}).active,false);
});
test('shortcut labels use the local names of keys',()=>{
  const de=new Translator('de',{'@keys':{Ctrl:'Strg',Shift:'Umschalt',Del:'Entf','Page Up':'Bild auf'}});
  assert.equal(de.active,true);
  assert.equal(de.translate('⌘/Ctrl ⇧ S'),'⌘/Strg ⇧ S');
  assert.equal(de.translate('Ctrl Shift Del'),'Strg Umschalt Entf');
  assert.equal(de.translate('Alt+Page Up'),'Alt+Bild auf');
  assert.equal(de.translate('Alt 1'),'Alt 1');
  assert.equal(de.translate('Delete files'),'Delete files');
});
test('names someone gave stay as typed even when they equal a key',()=>{
  const de=new Translator('de',{Help:'Hilfe','Show {label}':'{label} anzeigen','Select files first.':'Wählen Sie zuerst Dateien aus.'});
  assert.equal(de.translate('Help'),'Hilfe');
  // data-verbatim="Help": the name stays, the sentence around it and other text are translated.
  assert.equal(de.translate('Help',0,'Help'),'Help');
  assert.equal(de.translate(' Help ',0,'Help'),' Help ');
  assert.equal(de.translate('Show Help'),'Hilfe anzeigen');
  assert.equal(de.translate('Show Help',0,'Help'),'Help anzeigen');
  assert.equal(de.translate('Select files first.',0,'Help'),'Wählen Sie zuerst Dateien aus.');
  // A bare data-verbatim protects every value.
  assert.equal(de.translate('Help',0,true),'Help');
  assert.equal(de.translate('Show Help',0,true),'Help anzeigen');
  // The cache of ordinary lookups is not affected.
  assert.equal(de.translate('Help'),'Hilfe');
});
test('rereading the languages folder replaces the user catalogs instead of adding to them',()=>{
  const bundled=['en','nl','de'].map(locale=>({...catalogMeta(catalogs[locale]),catalog:catalogs[locale],source:'bundled'}));
  const first=languageSet(bundled,[{file:'fr.json',catalog:{'@meta':{name:'Français'},'Save tags':'Enregistrer'}},{file:'de.json',catalog:{'@meta':{name:'Deutsch (eigene)'},'Save tags':'Speichern!'}},{file:'bad.json',catalog:[]}],english);
  assert.deepEqual([...first.languages.keys()].sort(),['de','en','fr','nl']);
  assert.equal(first.languages.get('de').source,'user');
  assert.equal(first.languages.get('de').catalog['Save tags'],'Speichern!');
  assert.ok(first.warnings.some(w=>w.startsWith('bad.json: not a catalog')));
  // fr.json deleted and de.json removed: French is gone and the bundled German is back.
  const second=languageSet(bundled,[],english);
  assert.deepEqual([...second.languages.keys()].sort(),['de','en','nl']);
  assert.equal(second.languages.get('de').source,'bundled');
  assert.equal(second.languages.get('de').catalog,catalogs.de);
  // A chosen language that disappeared falls back like any unavailable one.
  assert.equal(resolveLanguage('fr',['nl-NL'],[...second.languages.keys()]),'en');
  assert.equal(resolveLanguage('system',['fr-FR','nl-NL'],[...second.languages.keys()]),'nl');
  // An English file replaces the shipped English and is active, so dialogs and the DOM use it.
  const english2=languageSet(bundled,[{file:'en.json',catalog:{'@meta':{name:'English (custom)'},'Save tags':'Write tags'}}],english).languages.get('en');
  assert.equal(english2.source,'user');
  const custom=new Translator('en',english2.catalog);
  assert.equal(custom.active,true);
  assert.equal(custom.translate('Save tags'),'Write tags');
  assert.equal(new Translator('en',{}).active,false);
  // A file that became unreadable is not offered either.
  assert.ok(!languageSet(bundled,[{file:'fr.json',catalog:'{'}],english).languages.has('fr'));
});
test('catalog validation finds missing, unknown and mismatched keys',()=>{
  const en={'@meta':{name:'English',locale:'en'},'A {x}':'A {x}','B':'B','C {n}':'C {n}'};
  const problems=validateCatalog(en,{'A {x}':'Ein {y}','C {n}':'{n, plural, single {x} other {y}}','Z':'z'});
  assert.deepEqual(problems,{missing:['B'],unknown:['Z'],placeholders:['A {x}'],invalid:['C {n}']});
});
test('copying the generated language template adds the filename locale without replacing English',async()=>{
  const main=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');
  const parsed=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true);
  const declaration=parsed.statements.filter(ts.isVariableStatement).flatMap(node=>[...node.declarationList.declarations]).find(node=>node.name.getText(parsed)==='languagePage');
  // Run the production folder action and capture the template passed to the backend.
  const js=ts.transpileModule(`const ${declaration.getText(parsed)};`,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  let generated;
  const host=new Function('ENGLISH','desktop','languageOptionsPage','invoke',`${js}\nreturn languagePage;`)(english,true,host=>host,(_command,args)=>{generated=args.template;});
  await host.openFolder();
  const template=JSON.parse(generated),bundled=['en','nl','de'].map(locale=>({...catalogMeta(catalogs[locale]),catalog:catalogs[locale],source:'bundled'}));
  for(const [file,locale,name] of [['fr.json','fr','Français'],['pt-BR.json','pt-BR','Português'],['de.json','de','Deutsch (eigene)']]){
    // Follow the Options instructions: copy to this filename, set its name and translate values.
    const copy=JSON.parse(generated);copy['@meta'].name=name;copy['Save tags']='Enregistrer';
    assert.deepEqual(catalogMeta(copy,file),{locale,name});
    const result=languageSet(bundled,[{file,catalog:copy}],english);
    assert.deepEqual(result.warnings,[]);
    assert.equal(result.languages.get(locale).source,'user');
    assert.equal(result.languages.get(locale).catalog['Save tags'],'Enregistrer');
    assert.equal(result.languages.get('en').catalog,english);
  }
  assert.deepEqual(validateCatalog(english,template),{missing:[],unknown:[],placeholders:[],invalid:[]});
  assert.equal(template['Save tags'],'Save tags');assert.equal(english['@meta'].locale,'en');
  assert.deepEqual(catalogMeta({'@meta':{name:'Français',locale:'fr'}},'custom.json'),{locale:'fr',name:'Français'});
});
test('the language follows the choice, then the system languages by region and base language, then English',()=>{
  const available=['en','nl','de','pt-BR'];
  assert.equal(resolveLanguage('system',['nl-BE','en-US'],available),'nl');
  assert.equal(resolveLanguage('system',['fr-FR','de-AT'],available),'de');
  assert.equal(resolveLanguage('system',['pt-PT'],available),'pt-BR');
  assert.equal(resolveLanguage('system',['fr'],available),'en');
  assert.equal(resolveLanguage('de',['nl'],available),'de');
  assert.equal(resolveLanguage('xx',['nl'],available),'en');
  assert.deepEqual(catalogMeta({'@meta':{name:'Français'}},'fr.json'),{locale:'fr',name:'Français'});
  assert.deepEqual(catalogMeta({},'pt-br.json'),{locale:'pt-BR',name:'pt-BR'});
  assert.equal(catalogMeta({},'not a locale!.json'),undefined);
});

test('the English catalog lists every user-visible string of the frontend',()=>{
  const expected=englishCatalog(extract());
  const {['@meta']:meta,...current}=english;
  assert.deepEqual(meta,{name:'English',locale:'en'});
  const missing=Object.keys(expected).filter(key=>!(key in current)),stale=Object.keys(current).filter(key=>!(key in expected));
  assert.deepEqual({missing,stale},{missing:[],stale:[]},'Run node scripts/i18n-extract.mjs --write, then translate the new keys in every catalog.');
  for(const [key,value] of Object.entries(current))assert.equal(value,key);
});
for(const locale of ['nl','de']){
  test(`the ${locale} catalog has every key with matching placeholders`,()=>{
    const catalog=catalogs[locale];
    assert.equal(catalog['@meta'].locale,locale);
    assert.deepEqual(validateCatalog(english,catalog),{missing:[],unknown:[],placeholders:[],invalid:[]});
  });
  test(`rendered English text reaches the right ${locale} translation`,()=>{
    const translator=new Translator(locale,catalogs[locale]),wrong=[];
    for(const key of Object.keys(english)){
      if(key.startsWith('@'))continue;
      const names=[...placeholders(key)],params=Object.fromEntries(names.map((name,i)=>[name,String(41+i)]));
      const shown=new Translator('en',{}).t(key,params);
      // Keys that differ only in their value names may share a pattern; their translations then agree too.
      if(translator.translate(shown)!==translator.t(key,params))wrong.push(`${key} → ${translator.translate(shown)} (expected ${translator.t(key,params)})`);
    }
    assert.deepEqual(wrong,[]);
  });
}
test('format strings, field names and placeholders stay as they are in every translation',()=>{
  const tokens=text=>(normalize(text).match(/%[A-Za-z0-9_]+%|\$[a-z_]+\(|\b[A-Z][A-Z0-9_]{3,}\b(?<!\bUTF)|\.(?:mta|mte|src|inc|m3u8?|pls|cue)\b/g)??[]).sort();
  for(const locale of ['nl','de']){
    // Labels written in capitals are translated as a whole.
    const changed=Object.keys(english).filter(key=>!key.startsWith('@')&&/\p{Ll}/u.test(key.replace(/\{[A-Za-z0-9]+\}/g,''))&&tokens(key).some(token=>!catalogs[locale][key].includes(token)));
    assert.deepEqual(changed,[],locale);
  }
});
