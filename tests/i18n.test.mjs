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
  assert.equal(de.translate('Copy of Save tags'),'Copy of Save tags');
  assert.equal(de.t('Copy of {name}',{name:'Save tags'}),'Kopie von Save tags');
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


test('locale decimal and grouping separators drive plural decisions without changing display text',()=>{
  const key='{count} files';
  const catalog={[key]:'{count, plural, =1 {# exact one} =1000 {# exact thousand} =1234 {# exact grouped} one {# one} other {# other}}'};
  const cases=[
    ['fr','1,0','exact one'],['fr','1,000','exact one'],['fr','1,2','one'],
    ['fr','1\u202f000,0','exact thousand'],['de','1,0','exact one'],
    ['de','1.000','exact thousand'],['de','1.234','exact grouped'],['de','1,234','other'],
    ['en','1,000','exact thousand'],['en','1.000','exact one'],['en','1.234','other'],
  ];
  for(const [locale,shown,form] of cases){
    const translator=new Translator(locale,catalog),expected=`${shown} ${form}`;
    assert.equal(translator.t(key,{count:shown}),expected,`${locale}: explicit ${shown}`);
    assert.equal(translator.translate(`${shown} files`),expected,`${locale}: captured ${shown}`);
  }
});

test('nested plural categories are validated recursively and invalid keys are removed once',()=>{
  const key='{count} files and {selected} selected',english={[key]:key};
  const bad='{count, plural, one {{selected, plural, single {# selected} other {# selected}}} other {{selected, plural, bogus {# selected} other {# selected}}}}';
  const catalog={'@meta':{locale:'fr',name:'French'},[key]:bad};
  assert.deepEqual(validateCatalog(english,catalog),{missing:[],unknown:[],placeholders:[],invalid:[key]});
  const result=languageSet([],[{file:'fr.json',catalog}],english);
  assert.equal(result.languages.get('fr').catalog[key],undefined);
  assert.equal(result.languages.get('fr').problems,1);
  assert.equal(result.warnings.length,1);
  assert.deepEqual(result.warnings[0].params,{file:'fr.json',missing:0,invalid:1});
  const valid='{count, plural, one {{selected, plural, =1 {# selected} other {# selected}}} other {{selected, plural, one {# selected} other {# selected}}}}';
  assert.deepEqual(validateCatalog(english,{[key]:valid}),{missing:[],unknown:[],placeholders:[],invalid:[]});
  assert.equal(new Translator('fr',{[key]:valid}).t(key,{count:1,selected:2}),'2 selected');
});


test('source-owned underscored placeholders render native limits and remain consistent through plural validation',async()=>{
  const files=['columns','file_ops','tag_panel','utilities'];
  const sources=Object.fromEntries(await Promise.all(files.map(async file=>[file,await readFile(new URL(`../src-tauri/src/${file}.rs`,import.meta.url),'utf8')])));
  const constants=Object.fromEntries(Object.values(sources).flatMap(source=>[...source.matchAll(/\bconst\s+([A-Z][A-Z0-9_]*):\s*\w+\s*=\s*(\d+)\s*;/g)].map(([,name,value])=>[name,Number(value)])));
  const producers=[
    ['columns','Configure between 1 and {MAX_COLUMNS} File List columns'],
    ['columns','{name}: width must be between {MIN_WIDTH} and {MAX_WIDTH} pixels'],
    ['file_ops','Select at most {PLAY_LIMIT} files to play'],
    ['tag_panel','The tag panel supports at most {MAX_FIELDS} fields'],
    ['tag_panel','{key}: at most {MAX_VALUES} custom values are supported'],
    ['utilities','Select at most {TOOL_LIMIT} files to run a tool for each file'],
  ];
  const fill=(text,params)=>text.replace(/\{([A-Za-z][A-Za-z0-9_]*)\}/g,(_,name)=>{assert.ok(name in params,name);return String(params[name]);});
  for(const [file,key] of producers){
    const sourceKey=[key,key.replace('{name}','{label}')].find(candidate=>sources[file].includes(`"${candidate}"`));
    assert.ok(sourceKey,key);
    const params={...constants,label:'Title',name:'Title',key:'ARTIST'},shown=fill(sourceKey,params);
    for(const locale of ['de','nl','en']){
      const translator=new Translator(locale,catalogs[locale]),expected=fill(catalogs[locale][key],params);
      assert.equal(translator.translate(shown),expected,`${locale}: native ${shown}`);
      assert.equal(translator.t(key,params),expected,`${locale}: explicit ${key}`);
    }
  }
  for(const key of Object.keys(english).filter(key=>!key.startsWith('@'))){
    const names=[...new Set([...key.matchAll(/\{([A-Za-z][A-Za-z0-9_]*)\}/g)].map(match=>match[1]))].sort();
    assert.deepEqual([...placeholders(key)].sort(),names,key);
  }
  const key='Count {file_count} for {name_2}',value='{file_count, plural, one {# for {name_2}} other {# for {name_2}}}';
  assert.deepEqual(validateCatalog({[key]:key},{[key]:value}),{missing:[],unknown:[],placeholders:[],invalid:[]});
  assert.equal(new Translator('en',{[key]:value}).t(key,{file_count:2,name_2:'{MAX_COLUMNS} / Help'}),'2 for {MAX_COLUMNS} / Help');
  assert.equal(new Translator('en',{}).t('Literal {not a placeholder}, {file-name}, {2name} stays',{name:'Wrong'}),'Literal {not a placeholder}, {file-name}, {2name} stays');
});


test('audio measurement patterns accept producer numbers and leave unknown unit-shaped text untouched',async()=>{
  const commands=await load('list-commands.ts');
  const entry={duration:65,size:1024,format:'MPEG',bitrate:320,sample_rate:44100,channels:2};
  const properties=commands.activeFileProperties?.(entry);
  const details=properties?.details??commands.listSummary([entry],[entry],1,entry).at(-1).split(' · ').slice(1);
  const frames=[['{bitrate} kbps','bitrate'],['{sampleRate} Hz','sampleRate'],['{channels} channels','channels']];
  for(const locale of ['de','nl']){
    const translator=new Translator(locale,catalogs[locale]);
    for(const text of ['Left channel','Right channels','Unknown kbps','Mystery Hz'])assert.equal(translator.translate(text),text,`${locale}: ${text}`);
    for(const [index,[key,name]] of frames.entries()){
      const shown=details[index],value=shown.slice(0,shown.lastIndexOf(' '));
      assert.equal(translator.translate(shown),translator.t(key,{[name]:value}),`${locale}: producer ${shown}`);
    }
    for(const [key,name,shown] of [['{channels} channel','channels','١'],['{bitrate} kbps','bitrate','٣٢٠'],['{sampleRate} Hz','sampleRate','٤٤٬١٠٠']]){
      const text=new Translator('en',{}).t(key,{[name]:shown});
      assert.equal(translator.translate(text),translator.t(key,{[name]:shown}),`${locale}: ${text}`);
    }
  }
});


test('source-audited numeric roles reject text across current catalog frames while raw summaries keep their shapes',()=>{
  // Numeric domains come from the closed native/UI producer audit, not name inference.
  const roles=new Set(["channels", "bitrate", "sampleRate", "counts", "covers", "withCovers", "sourceLine", "selectionIndex", "minimum", "maximum", "invalid", "number", "major", "code", "MAX_COLUMNS", "MAX_FIELDS", "MAX_VALUES", "MIN_WIDTH", "MAX_WIDTH", "PLAY_LIMIT", "TOOL_LIMIT", "TOOLLIMIT", "TIMEOUTRANGE"]);
  const translators=Object.fromEntries(['de','nl'].map(locale=>[locale,new Translator(locale,catalogs[locale])]));
  const en=new Translator('en',{}),pairs=[];
  for(const key of Object.keys(english).filter(key=>!key.startsWith('@'))){
    for(const role of placeholders(key))if(roles.has(role.replace(/\d+$/,'')))pairs.push({key,role});
  }
  assert.equal(pairs.length,38);
  const glyphs=['0','1','2','1.234','1,234','1\u202f234','١٬٢٣٤','۱۲۳۴','১২৩৪','१२३४','一二三','𝟙𝟚𝟛'];
  for(const {key,role} of pairs){
    const params=Object.fromEntries([...placeholders(key)].map(name=>[name,'2']));
    const isolated=Object.fromEntries(['de','nl'].map(locale=>[locale,new Translator(locale,{[key]:catalogs[locale][key]})]));
    for(const value of ['Left','Unknown','unknownLeft','1Left','١word']){
      const shown=en.t(key,{...params,[role]:value});
      for(const [locale,translator] of Object.entries(translators)){
        assert.equal(isolated[locale].translate(shown),shown,`${locale}: numeric frame rejects ${role}: ${shown}`);
        // Select accepts an actual authored filename, including text resembling
        // a rejected limit. Other overlapping source frames have closed domains.
        let expected=shown;
        if(['Select at most {PLAY_LIMIT} files to play','Select at most {TOOL_LIMIT} files to run a tool for each file'].includes(key)){
          expected=translator.t('Select {filename}',{filename:shown.slice('Select '.length)});
        }
        assert.equal(translator.translate(shown),expected,`${locale}: whole catalog ${role}: ${shown}`);
      }
    }
    for(const value of glyphs){
      const values={...params,[role]:value},shown=en.t(key,values);
      for(const [locale,translator] of Object.entries(translators))assert.equal(translator.translate(shown),translator.t(key,values),`${locale}: ${role}: ${shown}`);
    }
  }
  for(const translator of Object.values(translators)){
    for(const text of ['Chapter Left: 2','$custom expects 1..=2 arguments, received Left'])assert.equal(translator.translate(text),text);
    for(const value of glyphs){
      const key='${name} expects {min}..={max} arguments, received {value}',params={name:'custom',min:1,max:2,value};
      assert.equal(translator.translate(en.t(key,params)),translator.t(key,params));
    }
  }
  const exclusions=[{"role": "trackPosition", "input": "TITLE of track A1", "actual": {"de": "TITLE von Track A1", "nl": "TITLE van track A1"}}, {"role": "other", "input": "CommandX is not implemented", "actual": {"de": "CommandX ist nicht implementiert", "nl": "CommandX is niet geïmplementeerd"}}, {"role": "duration", "input": "Disc ID ab12034f: 2 tracks from the selected files in file list order, 12:34 in total.", "actual": {"de": "Disc-ID ab12034f: 2 Tracks aus den ausgewählten Dateien in der Reihenfolge der Dateiliste, insgesamt 12:34.", "nl": "Schijf-ID ab12034f: 2 tracks uit de geselecteerde bestanden in volgorde van de bestandslijst, 12:34 in totaal."}}, {"role": "sum", "input": "2 files (3:12 | 12.4 MB)", "actual": {"de": "2 Dateien (3:12 | 12.4 MB)", "nl": "2 bestanden (3:12 | 12.4 MB)"}}];
  for(const {role,input,actual} of exclusions)for(const [locale,translator] of Object.entries(translators)){
    assert.notEqual(actual[locale],input,`${role}: translates the outer frame`);
    assert.equal(translator.translate(input),actual[locale],`${locale}: preserve ${role}`);
  }
});


test('source-owned anonymous count slots and signed movement counts keep raw roles elsewhere',async()=>{
  const {UTILS}=await load('utils-ui.ts');
  const cases=[{"key": "Chapter {value}: {error}", "slots": ["value"]}, {"key": "${name} expects {min}..={max} arguments, received {value}", "slots": ["value"]}, {"key": "Cannot show {value} files here", "slots": ["value"]}, {"key": "Configuration version {value} is not supported", "slots": ["value"]}, {"key": "Cover {value} does not exist", "slots": ["value"]}, {"key": "Invalid action section at line {value}", "slots": ["value"]}, {"key": "Invalid action setting at line {value}", "slots": ["value"]}, {"key": "Setting outside an action section at line {value}", "slots": ["value"]}, {"key": "New filename references undefined part %{value}", "slots": ["value"]}, {"key": "Repeated old filename part %{value} is ambiguous", "slots": ["value"]}, {"key": "Text import has {value} lines for {value2} selected files; no changes planned", "slots": ["value", "value2"]}, {"key": "The network timeout must be between {value} and {value2} seconds", "slots": ["value", "value2"]}, {"key": "Track {value} is not in the cue sheet", "slots": ["value"]}, {"key": "Unexpected filter token at position {value}", "slots": ["value"]}, {"key": "Unsupported settings version {value}", "slots": ["value"]}, {"key": "moving {n} lines leaves the input (line {value} of {value2})", "slots": ["value", "value2"]}, {"key": "the input has only {value} lines", "slots": ["value"]}, {"key": "{name} returned HTTP {value} for {url}", "slots": ["value"]}, {"key": "{name}: {value} bytes cannot be stored in a Nero chapter tag", "slots": ["value"]}, {"key": "{label} for {count} of {length} selected file? {effect} {other} file is skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "slots": ["other"]}, {"key": "{label} for {count} of {length} selected file? {effect} {other} files are skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "slots": ["other"]}, {"key": "{label} for {count} of {length} selected files? {effect} {other} file is skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "slots": ["other"]}, {"key": "{label} for {count} of {length} selected files? {effect} {other} files are skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "slots": ["other"]}];
  assert.equal(cases.length,23);assert.equal(cases.reduce((sum,row)=>sum+row.slots.length,0),26);
  const en=new Translator('en',{}),failures=[];
  for(const locale of ['de','nl']){
    const translator=new Translator(locale,catalogs[locale]);
    for(const {key,slots} of cases){
      assert.equal(typeof english[key],'string',key);
      const names=[...placeholders(key)],isolated=new Translator(locale,{[key]:catalogs[locale][key]});
      const params=Object.fromEntries(names.map(name=>[name,'2']));
      if(slots.includes('other'))Object.assign(params,{label:UTILS.optimize.label,effect:UTILS.optimize.effect,applies:UTILS.optimize.applies});
      const expectedValues=values=>slots.includes('other')?{...values,label:translator.translate(values.label),effect:translator.translate(values.effect),applies:translator.translate(values.applies)}:values;
      for(const slot of slots){
        for(const value of ['Left','Unknown','unknownLeft','1Left','١word']){
          const shown=en.t(key,{...params,[slot]:value});
          assert.equal(isolated.translate(shown),shown,`${locale}: reject ${slot}: ${shown}`);
          const expected=shown;
          if(translator.translate(shown)!==expected)failures.push({locale,key,slot,shown,expected,actual:translator.translate(shown)});
        }
        for(const value of ['0','1','2','1.234','1,234','1 234','١٬٢٣٤','۱۲۳۴','১২৩৪','१२३४','一二三','𝟙𝟚𝟛']){
          const values={...params,[slot]:value},shown=en.t(key,values);
          assert.equal(translator.translate(shown),translator.t(key,expectedValues(values)),`${locale}: numeric ${slot}: ${shown}`);
        }
        if(slot==='other'){
          const collision=new Translator(locale,{...catalogs[locale],一:'Wrong numeric lookup'}),values={...params,other:'一'};
          assert.equal(collision.translate(en.t(key,values)),collision.t(key,expectedValues(values)),`${locale}: key-local numeric other stays a glyph`);
        }
      }
    }
    const moving='moving {n} lines leaves the input (line {value} of {value2})';
    for(const n of [-2,0,2,'-9223372036854775808','-١','-一','-𝟚']){
      const values={n,value:1,value2:2},shown=en.t(moving,values);
      assert.equal(translator.translate(shown),translator.t(moving,values),`${locale}: signed producer ${shown}`);
    }
    const signedCollision=new Translator(locale,{...catalogs[locale],'-一':'Wrong signed lookup'}),signedValues={n:'-一',value:1,value2:2};
    assert.equal(signedCollision.translate(en.t(moving,signedValues)),signedCollision.t(moving,signedValues));
    for(const n of ['--2','-','+2','Left','-Left','-١word','1Left']){
      const shown=en.t(moving,{n,value:1,value2:2});assert.equal(translator.translate(shown),shown,`${locale}: invalid movement count`);
    }
    const copyKey='Copy of {name} ({n})',copy=new Translator(locale,{[copyKey]:catalogs[locale][copyKey]});
    assert.equal(copy.translate('Copy of Help (-2)'),'Copy of Help (-2)','persisted counter names are not inferred as UI');
    assert.equal(translator.translate('CommandX is not implemented'),translator.t('{other} is not implemented',{other:'CommandX'}));
  }
  assert.deepEqual(failures,[],'whole catalog preserves anonymous numeric guards and source-proven raw alternate frames');
});


test('closed UI fragments bind real utility, tag-kind and native picture producers without consuming authored data',async()=>{
  const {UTILS,utilTargets,utilMessage}=await load('utils-ui.ts');
  const {removeTagPrompt}=await load('remove-tag.ts');
  const {PICTURE_TYPES,pictureTypeLabel}=await load('cover-model.ts');
  assert.equal(PICTURE_TYPES.length,21);
  const lock=await readFile(new URL('../Cargo.lock',import.meta.url),'utf8');
  assert.match(lock,/name = "lofty"\s+version = "0\.25\.4"[\s\S]*?checksum = "bb21dd79145b0781f2423fda92dd807fcd3911e44b66044d86db3b11ec7accf9"/);
  const native=await readFile(new URL('../crates/tag-core/src/tags.rs',import.meta.url),'utf8');
  assert.match(native,/picture_type:\s*format!\("\{:\?\}",\s*picture\.pic_type\(\)\)/);
  const tagLists=new Map();
  for(let mask=0;mask<8;mask++){
    const flags={remove_id3v1:!!(mask&1),remove_id3v2:!!(mask&2),remove_ape:!!(mask&4)};
    const text=removeTagPrompt(2,{mpeg:flags,ape:flags}).split('\n').find(line=>line.startsWith('MP3, TTA: ')).slice('MP3, TTA: '.length);
    const at=text.indexOf(' and ');if(at>=0)tagLists.set(text,{name:text.slice(0,at),names:text.slice(at+5)});
  }
  assert.deepEqual([...tagLists.keys()].sort(),['ID3v1 and ID3v2','ID3v1 and APE','ID3v2 and APE','ID3v1, ID3v2 and APE'].sort());
  const pictureKeys=Object.keys(english).filter(key=>key.includes('{pictureTypeLabel}'));assert.equal(pictureKeys.length,4);
  for(const locale of ['de','nl']){
    const translator=new Translator(locale,catalogs[locale]);
    for(const [shown,params] of tagLists)assert.equal(translator.translate(shown),translator.t('{name} and {names}',params));
    for(const shown of ['Left and Right','ID3v2 and ID3v1','ID3v1 and ID3v1','ID3v1, APE and ID3v2'])assert.equal(translator.translate(shown),shown);
    for(const spec of Object.values(UTILS)){
      const key='{label} applies to {applies}.',shown=`${spec.label} applies to ${spec.applies}.`;
      assert.equal(translator.translate(shown),translator.t(key,{label:translator.translate(spec.label),applies:translator.translate(spec.applies)}));
      const wrong=shown.replace(spec.label,'Unknown action');assert.equal(translator.translate(wrong),wrong);
    }
    for(const util of Object.keys(UTILS))for(const formats of [['MP3'],['FLAC'],['MP3','FLAC'],['MP3','FLAC','WAV']]){
      const spec=UTILS[util],count=utilTargets(util,formats),other=formats.length-count;
      const key=`{label} for {count} of {length} selected file${formats.length===1?'':'s'}? {effect}${other?` {other} file${other===1?' is':'s are'} skipped: this applies to {applies}.`:''} Files that need no change are not rewritten. Edit → Undo restores the files during this session.`;
      assert.equal(typeof english[key],'string');
      const params={label:translator.translate(spec.label),count,length:formats.length,effect:translator.translate(spec.effect),other,applies:translator.translate(spec.applies)};
      const shown=utilMessage(util,formats),expected=translator.t(key,params);
      assert.equal(translator.translate(shown),expected,`${locale}: actual ${util} producer`);
      assert.equal(translator.translate(shown.replace(spec.effect,spec.effect.replace(/ /g,'  '))),expected,'normalize code-owned effect whitespace');
      const otherSpec=Object.values(UTILS).find(value=>value!==spec),mismatched=shown.replace(spec.effect,otherSpec.effect);
      assert.equal(translator.translate(mismatched),mismatched,'source label and effect stay paired');
      if(other){
        const clause=`${other} file${other===1?' is':'s are'} skipped:`,invalid=shown.replace(clause,clause.replace(String(other),'Left'));
        assert.notEqual(invalid,shown);assert.equal(translator.translate(invalid),invalid,`${locale}: invalid skipped count cannot hide inside effect`);
      }
    }
    const labels=[...PICTURE_TYPES.map(([id])=>pictureTypeLabel(id)),pictureTypeLabel('')];
    assert.equal(labels.at(-1),'Cover','documented helper fallback, not a native type');
    for(let index=0;index<256;index++)labels.push(pictureTypeLabel(`Undefined(${index})`));
    for(const key of pictureKeys)for(const label of labels){
      const shown=new Translator('en',{}).t(key,{pictureTypeLabel:label});
      assert.equal(translator.translate(shown),translator.t(key,{pictureTypeLabel:translator.translate(label)}),`${locale}: bounded ${label}`);
    }
    for(const key of pictureKeys){
      for(const label of ['Left','Left of 2 files have','Undefined(-1)','Undefined(256)','Undefined(01)','Undefined(١)']){
        const shown=new Translator('en',{}).t(key,{pictureTypeLabel:label});assert.equal(translator.translate(shown),shown,`${locale}: reject invalid native type ${label}`);
      }
      const collision=new Translator(locale,{...catalogs[locale],'Undefined(255)':'Wrong raw identifier'}),shown=new Translator('en',{}).t(key,{pictureTypeLabel:'Undefined(255)'});
      assert.equal(collision.translate(shown),collision.t(key,{pictureTypeLabel:'Undefined(255)'}),'preserve technical enum identity even with a colliding custom key');
      const uiLabel=new Translator(locale,{...catalogs[locale],'Front Cover':'Custom front label'});
      assert.equal(uiLabel.translate(new Translator('en',{}).t(key,{pictureTypeLabel:'Front Cover'})),uiLabel.t(key,{pictureTypeLabel:'Custom front label'}),'known enum labels still translate');
    }
    for(const data of ['Left of 2 files have artwork','Undefined(255)','ID3v1 and ID3v2']){
      assert.equal(translator.translate(data,0,data),data,'authored description/name stays protected');
      assert.equal(translator.translate(`Select ${data}`),translator.t('Select {filename}',{filename:data}),'real filename frame stays unrestricted');
    }
  }
});


test('every plural block needs other before a user catalog is usable, including nested and exact forms',()=>{
  const count='{count} files',nested='{count} files and {selected} selected',english={[count]:count,[nested]:nested};
  const catalog={'@meta':{locale:'de',name:'German'},[count]:'{count, plural, one {# Datei}}',
    [nested]:'{count, plural, one {{selected, plural, =1 {# ausgewählt}}} other {{selected, plural, one {# ausgewählt} other {# ausgewählt}}}}'};
  assert.deepEqual(validateCatalog(english,catalog),{missing:[],unknown:[],placeholders:[],invalid:[count,nested]});
  const result=languageSet([],[{file:'de.json',catalog}],english),language=result.languages.get('de');
  assert.equal(language.problems,2);assert.equal(language.catalog[count],undefined);assert.equal(language.catalog[nested],undefined);
  assert.deepEqual(result.warnings,[{key:'{file}: {missing} missing, {invalid} invalid',params:{file:'de.json',missing:0,invalid:2}}]);
  const translator=new Translator('de',language.catalog);
  assert.equal(translator.t(count,{count:2}),'2 files');assert.equal(translator.translate('2 files'),'2 files');
  assert.equal(translator.t(nested,{count:2,selected:3}),'2 files and 3 selected');
  assert.deepEqual(validateCatalog({[count]:count},{[count]:'{count, plural, =1 {# exakt}}'}).invalid,[count]);
  const valid={[count]:'{count, plural, =0 {keine Dateien} one {# Datei} other {# Dateien}}',
    [nested]:'{count, plural, one {{selected, plural, =1 {# ausgewählt} other {# ausgewählt}}} other {{selected, plural, one {# ausgewählt} other {# ausgewählt}}}}'};
  assert.deepEqual(validateCatalog(english,valid),{missing:[],unknown:[],placeholders:[],invalid:[]});
  const good=new Translator('de',valid);assert.equal(good.t(count,{count:0}),'keine Dateien');assert.equal(good.t(count,{count:2}),'2 Dateien');
  assert.equal(good.t(nested,{count:1,selected:2}),'2 ausgewählt');
});


test('exact plural selectors use canonical nonnegative integers at every nesting level',()=>{
 const key='{count} files',nested='{count} files and {selected} selected',en={[key]:key,[nested]:nested};
 const bad={[key]:'{count, plural, =01 {# Datei} other {# Dateien}}',[nested]:'{count, plural, one {{selected, plural, =00 {keine} other {#}}} other {{selected, plural, =001 {eine} other {#}}}}'};
 assert.deepEqual(validateCatalog(en,bad),{missing:[],unknown:[],placeholders:[],invalid:[key,nested]});
 const result=languageSet([],[{file:'de.json',catalog:{'@meta':{locale:'de',name:'German'},...bad}}],en);
 assert.equal(result.languages.get('de').problems,2);assert.deepEqual(result.warnings,[{key:'{file}: {missing} missing, {invalid} invalid',params:{file:'de.json',missing:0,invalid:2}}]);
 assert.equal(new Translator('de',result.languages.get('de').catalog).t(key,{count:1}),'1 files');
 for(const selector of ['=9007199254740993','=1000000000000000000000']){
  const catalog={[key]:`{count, plural, ${selector} {exakt} other {andere}}`};
  assert.deepEqual(validateCatalog({[key]:key},catalog).invalid,[key],selector);
  const languages=languageSet([],[{file:'de.json',catalog:{'@meta':{locale:'de'},...catalog}}],{[key]:key});
  assert.equal(languages.warnings.length,1);assert.equal(new Translator('de',languages.languages.get('de').catalog).t(key,{count:selector.slice(1)}),`${selector.slice(1)} files`);
 }
 assert.deepEqual(validateCatalog({[nested]:nested},{[nested]:'{count, plural, one {{selected, plural, =9007199254740993 {exakt} other {#}}} other {# {selected}}}'}).invalid,[nested]);
 const large={[key]:'{count, plural, =9007199254740992 {exakt} other {andere}}'};
 assert.deepEqual(validateCatalog({[key]:key},large).invalid,[]);assert.equal(new Translator('de',large).t(key,{count:'9007199254740992'}),'exakt');
 const valid={[key]:'{count, plural, =0 {keine} =1 {eine} =10 {zehn} zero {zero} one {one} two {two} few {few} many {many} other {# andere}}',[nested]:'{count, plural, one {{selected, plural, =0 {keine} =1 {eine} =10 {zehn} other {#}}} other {{selected, plural, one {eine} other {#}}}}'};
 assert.deepEqual(validateCatalog(en,valid),{missing:[],unknown:[],placeholders:[],invalid:[]});
 const t=new Translator('de',valid);assert.deepEqual([0,1,10,2].map(count=>t.t(key,{count})),['keine','eine','zehn','2 andere']);assert.equal(t.t(nested,{count:1,selected:10}),'zehn');
});


test('persisted copy names stay raw and failure frames follow actual producer domains',async()=>{
 const functions=async(file,names)=>{const source=await readFile(new URL(`../src/${file}`,import.meta.url),'utf8'),ast=ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS),declarations=ast.statements.filter(node=>ts.isFunctionDeclaration(node)&&names.includes(node.name?.text));assert.equal(declarations.length,names.length);return ts.transpileModule(declarations.map(node=>node.getText(ast).replace(/^export /,'')).join('\n'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;};
 const duplicate=new Function(await functions('helper-menu.ts',['duplicateEntry'])+';return duplicateEntry;')();
 const copyAction=new Function(await functions('action-ui.ts',['copyName'])+';return copyName;')();
 const copyExport=new Function(await functions('export-ui.ts',['uniqueName','copyExportConfiguration'])+';return copyExportConfiguration;')();
 const stageSource=await functions('main.ts',['stageTagUpdates']);
 const main=await readFile(new URL('../src/main.ts',import.meta.url),'utf8');const ast=ts.createSourceFile('main.ts',main,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS),operations=[];const visit=node=>{if(ts.isCallExpression(node)&&node.expression.getText(ast)==='stageTagUpdates'){assert.ok(ts.isStringLiteral(node.arguments[1]));operations.push(node.arguments[1].text);}ts.forEachChild(node,visit);};visit(ast);assert.deepEqual([...new Set(operations)].sort(),['Cut tags','Paste tags']);
 const custom={'Copy of {name}':'Clone {name}','Copy of {name} ({n})':'Clone {n}: {name}','Copy {name}':'Copy command {name}','{name} copy':'Clone {name}','{name} copy {count}':'Clone {count}: {name}','{operation} failed':'Failure: {operation}','Cut tags':'Help','Paste tags':'Title',Help:'Wrong second lookup',Title:'Wrong second lookup','{name} value':'Value: {name}','{label} values':'Values: {label}',Family:'Wrong authored name'};
 for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
  const t=new Translator(locale,catalog);
  for(const text of ['Family copy','Family copy 2','Some operation failed'])assert.equal(t.translate(text),text,`${locale}: ${text}`);
  for(const original of ['Family','Help','Title','  Family  ','Family copy']){
   const action=copyAction(original,[]),action2=copyAction(original,[{name:action}]);assert.equal(action,`Copy of ${original}`);assert.equal(action2,`Copy of ${original} (2)`);for(const authored of [action,action2])assert.equal(t.translate(authored,0,authored),authored);
   const entries=[{name:original,value:'%title%',favorite:false}],first=duplicate(entries,0);assert.equal(first.name,`${original} copy`);assert.equal(t.translate(first.name),first.name);
   const second=duplicate([...entries,first],0);assert.equal(second.name,`${original} copy 2`);assert.equal(t.translate(second.name),second.name);
   const result=copyExport({version:1,configurations:[{id:'old',name:original,template:'%title%'}],preferences:{configuration:'old'}},'old','new');assert.equal(result.configurations[1].name,`${original} copy`);
   for(const [key,params] of [['{name} copy',{name:original}],['{name} copy {count}',{name:original,count:2}],['Copy of {name}',{name:original}],['Copy of {name} ({n})',{name:original,n:2}]])assert.equal(t.t(key,params),catalog[key].replace(/\{([A-Za-z][A-Za-z0-9_]*)\}/g,(_,name)=>String(params[name])));
  }
  assert.equal(t.translate('Copy of Family'),t.t('Copy {name}',{name:'of Family'}),'The unmarked text also has a legitimate field-command interpretation');
  for(const original of ['Family','Help','Title'])for(const [key,param] of [['{name} value','name'],['{label} values','label']]){const text=key.replace(`{${param}}`,original);assert.equal(t.translate(text,0,original),t.t(key,{[param]:original}));}
  for(const operation of ['Cut tags','Paste tags']){
   const stage=new Function('render',`let errors=[],status='';${stageSource};return {run:stageTagUpdates,status:()=>status};`)(()=>{});stage.run(()=>{throw new Error('failure');},operation,()=>{throw new Error('unexpectedsuccess');});
   const text=stage.status();assert.equal(text,`${operation} failed`);assert.equal(t.translate(text),t.t('{operation} failed',{operation:t.t(operation)}));
  }
 }
});


test('exact plural selection retains integer precision before numeric category conversion',()=>{
 const key='{count} files',catalog={[key]:'{count, plural, =9007199254740992 {exact big} =1 {exact one} =0 {exact zero} other {# other}}'};
 assert.deepEqual(validateCatalog({[key]:key},catalog),{missing:[],unknown:[],placeholders:[],invalid:[]});
 for(const locale of ['en','de','fr','ar-EG','hi-IN-u-nu-deva']){
  const t=new Translator(locale,catalog),different=9007199254740993n;
  for(const count of [String(different),different,different.toLocaleString(locale)])assert.equal(t.t(key,{count}),`${count} other`,`${locale}: ${count}`);
  for(const count of [9007199254740992,'9007199254740992',9007199254740992n,9007199254740992n.toLocaleString(locale)])assert.equal(t.t(key,{count}),'exact big');
  const decimal=new Intl.NumberFormat(locale).formatToParts(1.5).find(part=>part.type==='decimal').value,one=1..toLocaleString(locale),zero=0..toLocaleString(locale);
  assert.equal(t.t(key,{count:one+decimal+zero}),'exact one');assert.equal(t.t(key,{count:zero+decimal+zero}),'exact zero');
  const noninteger=one+decimal+zero.repeat(20)+one;assert.equal(t.t(key,{count:noninteger}),`${noninteger} other`);
  const nested=new Translator(locale,{'{count} files':'{count, plural, other {{count, plural, =9007199254740992 {nested exact} other {# nested other}}}}'});
  assert.equal(nested.t(key,{count:String(different)}),`${different} nested other`);
 }
 const t=new Translator('en',catalog);assert.equal(t.t(key,{count:'9007199254740993.0'}),'9007199254740993.0 other');assert.equal(t.t(key,{count:'0001.000'}),'exact one');assert.equal(t.t(key,{count:'-0.0'}),'exact zero');
 for(const count of ['1e0','10e-1','+1.0e0','.1e1'])assert.equal(t.t(key,{count}),'exact one');
 for(const count of ['9007199254740993e0','1.0000000000000000001e0','1e9999999999'])assert.equal(t.t(key,{count}),`${count} other`);
 assert.equal(t.t(key,{count:'9007199254740992e0'}),'exact big');
});


test('punctuation-only failure frames use explicit source parts and preserve raw paths and external errors',()=>{
 const frame='{value}: {error}',reason='Select files first.';
 const custom={[frame]:'At [{value}] — {error}',[reason]:'Help',Help:'Wrong second lookup','Help: Title':'Wrong path'};
 for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
  const t=new Translator(locale,catalog);
  for(const value of ['Help: Title','C:\\Music: Help\\Title.mp3','name: Select files first.; next: Help']){
   const combined=`${value}: ${reason}`;assert.equal(t.translate(combined),combined,'An unmarked colon cannot establish the path/error boundary');
   const owned=t.t(reason),expected=String(catalog[frame]).replace('{value}',value).replace('{error}',owned);
   assert.equal(t.t(frame,{value,error:owned}),expected);assert.ok(expected.includes(value));
   const external=String(catalog[frame]).replace('{value}',value).replace('{error}',reason);
   assert.equal(t.t(frame,{value,error:reason}),external,'The same English leaf remains raw when its producer identifies it as external');
  }
 }
});


test('Show controls choose their producer frame and retain explicit raw or owned label parts',async()=>{
 const {TOOLBAR_COMMANDS,itemLabel}=await load('toolbar.ts');
 const custom={...english,'Show {name}':'Column [{name}]','Show {label}':'Panel [{label}]','Show {itemLabel}':'Toolbar [{itemLabel}]',Help:'Assistance',Title:'Heading','Save tags':'Store',Heading:'Wrong second lookup',Store:'Wrong second lookup','Toolbar [Store]':'Wrong output lookup'};
 const owned=[...Object.keys(TOOLBAR_COMMANDS).map(id=>itemLabel({id,visible:true})),...['separator-source','spacer-source'].map(id=>itemLabel({id,visible:true}))];
 for(const [locale,catalog] of [['de',catalogs.de],['nl',catalogs.nl],['en',custom]]){
  const t=new Translator(locale,catalog);
  assert.equal(t.translate('Show Help'),String(catalog['Show {name}']).replace('{name}','Help'),'An unmarked raw-name frame must not become an owned UI-label frame');
  for(const role of ['name','label','itemLabel'])for(const name of ['Help','Title','Save tags','  Help  ','Help\nTitle','Copy of Help','Show Help']){
   const key=`Show {${role}}`;assert.equal(t.showLabel(role,name,true),String(catalog[key]).replace(`{${role}}`,name),`${locale}/${role}/${JSON.stringify(name)}`);
  }
  for(const label of owned)assert.equal(t.showLabel('itemLabel',label),String(catalog['Show {itemLabel}']).replace('{itemLabel}',t.t(label)));
  assert.equal(t.showLabel('name','Help',false),String(catalog['Show {name}']).replace('{name}','Help'));
  assert.equal(t.showLabel('itemLabel','Help','Help'),String(catalog['Show {itemLabel}']).replace('{itemLabel}','Help'));
  assert.equal(t.showLabel('itemLabel','Help','Title'),String(catalog['Show {itemLabel}']).replace('{itemLabel}',t.t('Help')));
  const localized=t.t('Title');assert.equal(t.showLabel('label',localized,true),String(catalog['Show {label}']).replace('{label}',localized),'An already localized default name is protected from a second lookup');
 }
 const partial=new Translator('en',{'Show {name}':'Column [{name}]','Show {itemLabel}':'Toolbar [{itemLabel}]'});assert.equal(partial.showLabel('itemLabel','Show in folder'),'Toolbar [Show in folder]','A missing literal UI label falls back to its English key, not another inferred frame');
});


test('shortcut spelling aliases follow the actual Actions producer and canonical catalog key',async()=>{
  const {EXTRA_SHORTCUTS,shortcutsDialog}=await load('utils-ui.ts');
  const row=EXTRA_SHORTCUTS.find(row=>row.group==='Actions dialog'&&row.command==='New group or action');
  assert.equal(row.keys,'Insert');
  let content='';const dialog={classList:{add(){}},querySelector:()=>({textContent:''})};
  assert.equal(shortcutsDialog((_title,body)=>{content=body;return dialog;},[row]),dialog);
  assert.match(content,/<kbd>Insert<\/kbd>/);
  const de=new Translator('de',catalogs.de);
  assert.equal(de.translate(row.keys),'Einfg');
  for(const [source,expected] of [['Ins','Einfg'],['Ctrl Insert','Strg Einfg'],['Shift+Insert','Umschalt+Einfg'],['⌘/Ctrl Insert','⌘/Strg Einfg'],['Insert / Ins','Einfg / Einfg'],['  Insert  ','  Einfg  ']])assert.equal(de.translate(source),expected,source);
  assert.equal(new Translator('nl',catalogs.nl).translate(row.keys),'Insert');
  assert.equal(new Translator('en').translate(row.keys),'Insert');
  const identity=new Translator('en',{'@keys':{Ins:'Ins',Ctrl:'Ctrl'}});
  assert.equal(identity.translate('Insert'),'Insert');assert.equal(identity.translate('Ctrl Insert'),'Ctrl Insert');
  assert.equal(new Translator('de',{'@keys':{Ins:'Einfg',Insert:'Insert'}}).translate('Insert / Ins'),'Insert / Einfg');
  const override=new Translator('de',{'@keys':{Ins:'Short key',Insert:'Full key',Ctrl:'Control'}});
  assert.equal(override.translate('Ctrl Insert / Ins'),'Control Full key / Short key');
  const noCanonical=new Translator('de',{'@keys':{Ctrl:'Control'}});
  assert.equal(noCanonical.translate('Ctrl Insert'),'Control Insert');
  assert.equal(new Translator('de',{'@keys':{Ins:'',Ctrl:'Control'}}).translate('Ctrl Insert'),'Control Insert');
  assert.equal(new Translator('de',{'@keys':{Ins:'Short key',Insert:''}}).translate('Insert'),'Short key');
});

test('shortcut aliases respect authored values and full verbatim protection',()=>{
  const de=new Translator('de',catalogs.de);
  assert.equal(de.translate('Ctrl Ins',0,true),'Ctrl Ins');
  for(const source of ['Insert','Ins','Ctrl Insert','Shift+Insert','  Insert  ']){
    assert.equal(de.translate(source,0,true),source);
    assert.equal(de.translate(source,0,source),source);
  }
  for(const source of ['Insert file','Insert.mp3','Inserts','ReInsert','Insertive','insert','CTRL INSERT','Ctrl Inserted'])assert.equal(de.translate(source),source,source);
  assert.equal(de.translate('Select Insert'),de.t('Select {filename}',{filename:'Insert'}));
  assert.equal(de.translate('Select Ctrl Insert'),de.t('Select {filename}',{filename:'Ctrl Insert'}));
  assert.equal(de.translate('Insert',0,'Another name'),'Einfg');
  assert.equal(de.translate('Insert'),'Einfg','A protected lookup never poisons the ordinary shortcut cache');
  assert.equal(de.translate('Insert',0,true),'Insert','A cached ordinary lookup never changes protected text');
});
