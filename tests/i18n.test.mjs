import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, readdir} from 'node:fs/promises';
import ts from 'typescript';
import {extract, extractFile, englishCatalog, extractFrozen, extractBackend, extractBackendSource} from '../scripts/i18n-extract.mjs';
const load=async file=>{
  const source=await readFile(new URL(`../src/${file}`,import.meta.url),'utf8');
  const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
};
const core=await load('i18n/core.ts');
const {removeTagPrompt}=await load('remove-tag.ts');
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
test('all backend source-role diagnostics preserve field identities and authored export configuration names',async()=>{
  const producers=[['crates/tag-core/src/mapping.rs','Conflicting updates for mapped source {source}',['ARTIST','TITLE']],['src-tauri/src/action_host.rs','Export configuration not found: {source}',['Help','Title','My  Files','/Music/My  Files/Help.mte']],['crates/tag-core/src/actions.rs','Missing {source} action context',['_FILENAME','_FILENAME_EXT']]];
  assert.deepEqual(Object.keys(english).filter(key=>/\{source\d*\}/.test(key)).sort(),producers.map(([,key])=>key).sort());
  for(const [file,key,values] of producers){
    // Match the actual Rust format producer; native tests execute the mapping
    // collision and missing export lookup, rather than relying on invented UI text.
    const rust=await readFile(new URL(`../${file}`,import.meta.url),'utf8');assert.ok(rust.includes(`format!("${key}")`),file);
    for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',{
      ARTIST:'Performer',TITLE:'Heading',Help:'Assistance',Title:'Heading',_FILENAME:'Wrong filename',_FILENAME_EXT:'Wrong extension',
      [key]:`Diagnostic: {source}`,'Update check failed: {error}':'Update failed: {error}','Select files first.':'Select tracks.'
    }]])for(const source of values){
      const t=new Translator(locale,catalog),message=key.replace('{source}',source);
      assert.equal(t.translate(message),t.t(key,{source}));assert.ok(t.translate(message).includes(source));
      // Error/UI nesting is deliberately recursive even though source identities are raw.
      assert.equal(t.translate('Update check failed: Select files first.'),t.t('Update check failed: {error}',{error:t.t('Select files first.')}));
    }
  }
});
test('every cataloged identifier role retains authored spelling and whitespace',()=>{
  const keys=Object.keys(english).filter(key=>/\{id\d*\}/.test(key));
  assert.equal(keys.length,13,'Audit added identifier producers before changing their data role');
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',{...Object.fromEntries(keys.map(key=>[key,`Identifier: {id}`])),Help:'Assistance',ARTIST:'Performer'}]])for(const key of keys)for(const id of ['Help','ARTIST','My  Identifier']){
    const params=Object.fromEntries([...placeholders(key)].map(name=>[name,name==='id'?id:'41'])),message=new Translator('en').t(key,params),t=new Translator(locale,catalog);
    assert.equal(t.translate(message),t.t(key,params));
    assert.ok(t.translate(message).includes(id));
  }
});
test('technical action tokens, extension tokens and URI schemes remain verbatim inside diagnostics',()=>{
  const roles=new Set(['codepage','mode','typeId','item','scheme','byte','flags']),keys=Object.keys(english).filter(key=>[...placeholders(key)].some(role=>roles.has(role)));
  const custom={...Object.fromEntries(keys.map(key=>[key,`Diagnostic: ${[...placeholders(key)].map(role=>`{${role}}`).join(' | ')}`])),Title:'Heading','"Title"':'Wrong quoted token',FF:'Wrong byte','(empty)':'(missing)','Select files first.':'Choose tracks.'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]])for(const key of keys){
    const params=Object.fromEntries([...placeholders(key)].map(role=>[role,role==='byte'?'FF':role==='item'?'(empty)':role==='location'?'Title://song':role==='reason'?'Select files first.':roles.has(role)?'Title':'41']));
    const t=new Translator(locale,catalog),message=new Translator('en').t(key,params),expected={...params};if(expected.reason)expected.reason=t.translate(expected.reason);
    assert.equal(t.translate(message),t.t(key,expected),key);
  }
});
test('supported custom catalogs preserve file, URL, format, selector and field-constant roles',()=>{
  const samples={file:'Title.json',url:'https://example.org',extension:'flac',tag:'ID3v2.4',selector:'_CUSTOM',DIRECTORY:'_DIRECTORY',SEPARATOR:'_SEPARATOR'};
  const keys=Object.keys(english).filter(key=>[...placeholders(key)].some(role=>role in samples));
  const custom={...Object.fromEntries(keys.map(key=>[key,`Scaffold: ${[...placeholders(key)].map(role=>`{${role}}`).join(' | ')}`])),...Object.fromEntries(Object.values(samples).map(value=>[value,'Wrong technical data']))};
  const result=languageSet([{...catalogMeta(english),catalog:english,source:'bundled'}],[{file:'en.json',catalog:custom}],english),active=result.languages.get('en').catalog;
  assert.equal(active['Title.json'],'Wrong technical data');
  const t=new Translator('en',active);
  for(const key of keys){const params=Object.fromEntries([...placeholders(key)].map(role=>[role,samples[role]??'41']));assert.equal(t.translate(new Translator('en').t(key,params)),t.t(key,params),key);}
});
test('authored column names and ASF field identities stay raw while mixed UI captions and errors recurse',()=>{
  const columnKeys=['{name}: enter a value format string','{name}: sort by: {e}','{name}: width must be between {MIN_WIDTH} and {MAX_WIDTH} pixels'],asf='{field} has no separate ASF attribute; {name} holds {field2}',flagKeys=['Unknown flag {value} in setting {key}','Unknown cover format {format}','Unknown cover size mode {mode}','Unknown cover scale mode {mode}'];
  const custom={...Object.fromEntries([...columnKeys,asf,...flagKeys].map(key=>[key,`Diagnostic: ${[...placeholders(key)].map(role=>`{${role}}`).join(' | ')}`])),Title:'Heading',ARTIST:'Performer','"Title"':'Wrong quoted token','Select files first.':'Choose tracks.','Show {label}':'Display {label}'};
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
    const t=new Translator(locale,catalog);
    for(const key of columnKeys){const params={name:'Title',e:'Select files first.',MIN_WIDTH:'24',MAX_WIDTH:'4000'};assert.equal(t.translate(new Translator('en').t(key,params)),t.t(key,{...params,e:t.translate(params.e)}));}
    assert.equal(t.translate('Author has no separate ASF attribute; Author holds ARTIST'),t.t(asf,{field:'Author',name:'Author',field2:'ARTIST'}));
    for(const key of flagKeys){const params=Object.fromEntries([...placeholders(key)].map(role=>[role,role==='key'?'3':'"Title"']));assert.equal(t.translate(new Translator('en').t(key,params)),t.t(key,params));}
    assert.equal(t.translate('Show Title'),t.t('Show {label}',{label:t.translate('Title')}));
  }
  const character='Character {value} has no legacy byte representation',t=new Translator('en',{[character]:'Invalid character: {value}',"'λ'":'Wrong character'});
  assert.equal(t.translate("Character 'λ' has no legacy byte representation"),"Invalid character: 'λ'");
});
test('unknown multiline labels remain raw and marked scaffolds still translate under partial catalogs',()=>{
  const t=new Translator('en',{'Tool: start {name} for the selected files':'Run {name}',Help:'Assistance'}),name='My  Files\nTitle';
  assert.equal(t.translate(`Show ${name}`,0,name),`Show ${name}`);
  assert.equal(t.translate(`✓ Tool: start ${name} for the selected files ↓`),`✓ Run ${name} ↓`);
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
for(const locale of ['de','nl'])test(`${locale}: translated confirmations retain original paths, line breaks and cached whitespace variants`,()=>{
  const t=new Translator(locale,catalogs[locale]);
  const names='Help\nMy  Files.mp3',destination='/Music/My  Files';
  const key='Copy {plural} to {destination}? {names} Existing files are never replaced. If any name is already taken, nothing is copied.';
  const text=`Copy 2 files to ${destination}?\n\n${names}\n\nExisting files are never replaced. If any name is already taken, nothing is copied.`;
  const translated=t.translate(text);
  assert.notEqual(translated,text);
  assert.ok(translated.includes(destination));assert.ok(translated.includes(`\n\n${names}\n\n`));
  const flat=text.replace(/\s+/g,' '),flatTranslation=t.translate(flat);
  assert.equal(flatTranslation,t.t(key,{plural:t.translate('2 files'),destination:'/Music/My Files',names:'Help My Files.mp3'}));
  assert.equal(t.translate(text),translated);
  const custom=new Translator(locale,{'Open {path}':'{path} öffnen'});
  for(const input of ['My  Files','My Files','My Files','My  Files'])assert.equal(custom.translate(`Open ${input}`),`${input} öffnen`);
  const reordered=new Translator(locale,{'Copy {name} to {destination}? {names} Existing files are never replaced.':'{names} Existing files are never replaced. Copy {name} to {destination}?'});
  assert.equal(reordered.translate(`Copy Help to ${destination}?\r\n\r\n${names}\r\n\r\nExisting files are never replaced.`),`${names}\r\n\r\nExisting files are never replaced. Copy Help to ${destination}?`);
});
test('rereading the languages folder replaces the user catalogs instead of adding to them',()=>{
  const bundled=['en','nl','de'].map(locale=>({...catalogMeta(catalogs[locale]),catalog:catalogs[locale],source:'bundled'}));
  const first=languageSet(bundled,[{file:'fr.json',catalog:{'@meta':{name:'Français'},'Save tags':'Enregistrer'}},{file:'de.json',catalog:{'@meta':{name:'Deutsch (eigene)'},'Save tags':'Speichern!'}},{file:'bad.json',catalog:[]}],english);
  assert.deepEqual([...first.languages.keys()].sort(),['de','en','fr','nl']);
  assert.equal(first.languages.get('de').source,'user');
  assert.equal(first.languages.get('de').catalog['Save tags'],'Speichern!');
  assert.ok(first.warnings.some(w=>w.key==='{file}: not a catalog'&&w.params.file==='bad.json'));
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
  const host=new Function('ENGLISH','desktop','languageOptionsPage','invoke','onLanguageChange',`${js}\nreturn languagePage;`)(english,true,host=>host,(_command,args)=>{generated=args.template;},()=>()=>{});
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
  // User catalogs translate the individual UI rows; the app retains the paragraph/row layout.
  const user={...template,'Remove tags from {count} selected files?':'Delete metadata from {count} tracks?','Other formats: all tags':'Everything else: all metadata'};
  const translator=new Translator('en',user),tags={mpeg:{remove_id3v1:true},ape:{remove_ape:true}};
  assert.equal(removeTagPrompt(2,tags,text=>translator.translate(text)),'Delete metadata from 2 tracks?\n\nMP3, TTA: ID3v1\nAPE, MPC, WavPack, TAK, OptimFROG: APE\nEverything else: all metadata');
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
  for(const remaining of [['en','pt-PT'],['en','pt'],['en','pt','pt-PT']]){
    assert.equal(resolveLanguage('pt-BR',['pt-PT'],remaining),'en');
    assert.equal(resolveLanguage('system',['pt-BR'],remaining),remaining.includes('pt')?'pt':'pt-PT');
  }
  assert.equal(resolveLanguage('pt-br',[],available),'pt-BR');
  assert.equal(resolveLanguage('pt',[],available),'en');
  assert.deepEqual(catalogMeta({'@meta':{name:'Français'}},'fr.json'),{locale:'fr',name:'Français'});
  assert.deepEqual(catalogMeta({},'pt-br.json'),{locale:'pt-BR',name:'pt-BR'});
  assert.equal(catalogMeta({},'not a locale!.json'),undefined);
});

test('the backend catalog keys follow the current merged Rust error producers',async()=>{
  const stored=JSON.parse(await readFile(new URL('../src/i18n/backend-keys.json',import.meta.url),'utf8'));
  assert.deepEqual(stored,extractBackend(),'Run node scripts/i18n-extract.mjs --backend, then translate the new keys in every catalog.');
});
test('backend extraction follows actual local error factories without collecting raw data or pass-through adapters',()=>{
  const source=`
    fn label() -> String { "An unrelated display label".into() }
    fn external(error: IoError) -> String { error.to_string() }
    fn invalid(what: &str) -> String { format!("Invalid media file: {what}") }
    fn fail(error: String) -> Vec<FileError> { let path="Another private raw path".to_string(); vec![FileError { path: "A private raw path".to_owned(), error }] }
    fn run() {
      let failed = || "Verification failed; original preserved".to_string();
      let frame = |message: String| ScriptError { command: "A raw source command".into(), message };
      Err(fail(/* "A quoted comment is not a message" */ "Destination is not a folder".into()));
      Err(fail(format!("Cannot inspect destination: {value}", "A raw format argument")));
      Err(invalid("bad sample header"));
      Err(failed()); Err(frame("Unknown script instruction".into()));
      value.map_err(external);
    }
    impl fmt::Display for ScriptError { fn fmt(&self, f: &mut Formatter) { write!(f,"line {line} ({text}): {error}",line=self.line,text=self.command,error=self.message); } }
    // Err("A commented-out error should not appear".into());
    #[cfg(test)] mod tests { fn hidden() { Err("A test-only error should not appear".into()); } }
  `;
  assert.deepEqual(extractBackendSource(source),['Cannot inspect destination: {value}','Destination is not a folder','Invalid media file: {what}','Unknown script instruction','Verification failed; original preserved','bad sample header','line {line} ({text}): {error}'].sort());
});
test('backend extraction includes technical function prefixes and other diagnostic prefixes only in real error contexts',()=>{
  const source=`fn private(){let format="$replace requires private raw data";Err("$loopend takes no arguments".into());Err(format!("\${name} expects {count} arguments"));value.map_err(|_| "$fmtDate expects an integer Unix timestamp");value.map_err(|_| { format!("the regular expression is not valid: {value}") });Err("a quoted parameter is not closed".into());Err("[IndexFormat] has no %_url% field".into());Err(format!("\\\"{value}\\\" is not a number"));}`;
  assert.deepEqual(extractBackendSource(source),['$fmtDate expects an integer Unix timestamp','$loopend takes no arguments','${name} expects {count} arguments','the regular expression is not valid: {value}','a quoted parameter is not closed','[IndexFormat] has no %_url% field','"{value}" is not a number'].sort());
  const actual=extractBackend();for(const testOnly of ['grant revoked','read only','source changed','partial provider write','fresh read failed'])assert.ok(!actual.includes(testOnly),testOnly);for(const key of ['$loopend takes no arguments','$replace requires complete search/replacement pairs','$getenv(MP3TAGAPP) requires the application directory context'])assert.ok(actual.includes(key),key);
});
test('internal URI exclusion keeps real save and restore diagnostics containing during',async()=>{
  const expected=['The file changed during editing; reload it before saving','The file changed during restore; reload it and try again','The cue sheet changed during editing; reload it before saving','Tag disappeared during editing'];
  for(const [file,messages] of [['tags.rs',[expected[0],expected[3]]],['history.rs',[expected[1]]],['subsongs.rs',[expected[2]]]]){
    const source=await readFile(new URL(`../crates/tag-core/src/${file}`,import.meta.url),'utf8'),actual=extractBackendSource(source);for(const key of messages)assert.ok(actual.includes(key),key);
  }
  const internal=`fn run(){Err("Document URI is unavailable".into());Err("Document manifest is unavailable".into());Err("Injected copy failure".into());Err("Fixture lookup failed".into());}`;
  assert.deepEqual(extractBackendSource(internal),[]);
});
test('all function-prefixed catalog diagnostics retain literal tokens and raw function names',()=>{
  const keys=Object.keys(english).filter(key=>key.startsWith('$'));
  for(const locale of ['de','nl'])for(const key of keys){
    assert.deepEqual(catalogs[locale][key].match(/\$[A-Za-z_]\w*(?:\([A-Z]+\))?/g),key.match(/\$[A-Za-z_]\w*(?:\([A-Z]+\))?/g),key);
    assert.notEqual(catalogs[locale][key],key,key);
  }
  for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',{'${name} is not available here':'Unavailable: ${name}',Title:'Wrong function'}]]){
    const t=new Translator(locale,catalog);assert.equal(t.translate('$Title is not available here'),t.t('${name} is not available here',{name:'Title'}));assert.ok(t.translate('$Title is not available here').includes('$Title'));
  }
});
test('production destination and sibling wrapped error messages are part of the backend snapshot',()=>{
  const actual=extractBackend();
  for(const key of ['The destination is not a folder','Cannot open the destination folder: {e}','Library database: {e}','ASF {location} object limits exceeded','Invalid DSF file: {what}','Invalid Matroska file: {what}','The MP4 structure cannot be rewritten safely: {what}','Invalid {value} file: {what}','The optimized FLAC file did not verify; the original file was kept','The rewritten MP4 file did not verify; the original file was kept','Invalid chapter {value}: use HH:MM:SS.mmm followed by the title','{name} is stored once for {filename}, not per chapter; give its chapter rows the same value','The downloaded cover is not a supported image.','{name} did not respond in time. Check your connection and try again.','The freedb server answered: {code} {value}','{name} is not a known command','{name} takes {min} parameter','{name} takes {min} parameters','this block has no {name}','line {line} ({text}): {error}'])assert.ok(actual.includes(key),key);
});
test('extraction keeps scalar names and text joins while splitting actual HTML helpers and arrays',()=>{
  const keys=new Set(),collect=text=>extractFile('fixture.ts',text,key=>keys.add(key));
  collect('function notice(field:string,page:string,lines:number){return `Field ${field} belongs to ${page}; copied ${lines} lines.`;}');
  collect('const message=`Warnings: ${issues.map(issue=>`${issue.file}: ${issue.error}`).join("; ")}.`;');
  collect('function list(names:string[]){return names.join(", ");} const message=`Names: ${list(names)}.`;');
  assert.ok(keys.has('Field {field} belongs to {page}; copied {lines} lines.'));assert.ok(keys.has('Warnings: {join}.'));assert.ok(keys.has('Names: {list}.'));
  keys.clear();
  collect('function field(name:string){return `<b>${name}</b>`;} const markup=`<p>Before ${field(name)} after content.</p>`;');
  collect('function list(names:string[]){return names.map(name=>`<b>${name}</b>`).join("");} const markup=`<p>Start ${list(names)} finish content.</p>`;');
  collect('const markup=`<p>First ${names.map(name=>`<span>${name}</span>`).join("")} last content.</p>`;');
  assert.ok(keys.has('Before'));assert.ok(keys.has('after content.'));assert.ok(keys.has('Start'));assert.ok(keys.has('finish content.'));assert.ok(keys.has('First'));assert.ok(keys.has('last content.'));
  assert.ok(![...keys].some(key=>/\{(?:field|list|join)\}/.test(key)));
});
test('production scalar and joined-warning messages are extracted as complete catalog keys',()=>{
  const keys=extract();
  for(const key of ['Added “{value}” to the custom values for {field}.','“{value}” is already a custom value for {field}.','{field}: enter a name for the field.','{field} is already used for another field and cannot be added twice.','Copied {lines} line as text','Copied {lines} lines as text','nothing (no tag type is checked under Options → Tags → {page})','{name} and {names}','Imported {length} Mp3tag column from {path}; skipped {length2}: {join}','Installed {names}. {length} tag source file could not be read: {join}'])assert.ok(keys.has(key),key);
  for(const key of ['Added “{value}” to the custom values for','nothing (no tag type is checked under Options → Tags →','and {names}'])assert.ok(!keys.has(key),key);
});
test('the English catalog lists every user-visible string of the frontend',()=>{
  const expected=englishCatalog(extract());
  const {['@meta']:meta,...current}=english;
  assert.deepEqual(meta,{name:'English',locale:'en'});
  const missing=Object.keys(expected).filter(key=>!(key in current)),stale=Object.keys(current).filter(key=>!(key in expected));
  assert.deepEqual({missing,stale},{missing:[],stale:[]},'Run node scripts/i18n-extract.mjs --write, then translate the new keys in every catalog.');
  for(const [key,value] of Object.entries(current))assert.equal(value,key);
});
test('translate="no" only wraps data: no UI text is written inside it',()=>{
  assert.deepEqual(extractFrozen(),[]);
});
for(const locale of ['nl','de']){
  test(`the ${locale} catalog has every key with matching placeholders`,()=>{
    const catalog=catalogs[locale];
    assert.equal(catalog['@meta'].locale,locale);
    assert.deepEqual(validateCatalog(english,catalog),{missing:[],unknown:[],placeholders:[],invalid:[]});
    if(locale==='de')assert.equal(new Translator(locale,catalog).translate('Ctrl Shift Del'),'Strg Umschalt Entf');
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
