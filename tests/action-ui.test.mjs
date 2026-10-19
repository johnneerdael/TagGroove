import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/action-ui.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { mtaExportWarning, actionListKey, moveMarked, markAll, clickMark, actionCatalog, panelKey, moveItem, renameGroup, removeGroup, applyPreset, menuSegments, resolvePathChanges, rebaseCaseOnly, mnemonicMarkup, sortGroups, enabledSummary, copyName, codepageChoices, qualityPresets, recallQuick, rememberQuick } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
const library = () => ({groups:[{name:'First',enabled:true,actions:[{type:'format_value',field:'TITLE',template:'text'}]},{name:'Second',enabled:false,actions:[]}],presets:[{name:'Only first',enabled_groups:['First']}]});

test('catalog exposes every documented action type exactly once', () => {
  assert.equal(actionCatalog.length,18);
  assert.equal(new Set(actionCatalog.map(([type])=>type)).size,18);
  for(const type of ['adjust_cover','convert_codepage','export','export_cover','generate_playlists','guess_values','import_cover','import_text_file','merge_duplicate_fields','remove_duplicate_fields','remove_fields_except','set_cover_properties','split_field'])assert.ok(actionCatalog.some(([key])=>key===type));
});
test('renaming group preserves preset selection without changing source state or actions',()=>{
  const source=library(),renamed=renameGroup(source,0,'Renamed');assert.equal(source.groups[0].name,'First');assert.deepEqual(renamed.presets[0].enabled_groups,['Renamed']);assert.deepEqual(renamed.groups[0].actions,source.groups[0].actions);assert.throws(()=>renameGroup(source,0,'second'),/already/);
});
test('removing group cleans presets and selection presets never replace action contents',()=>{
  const original=library();original.groups[0].enabled=false;original.groups[1].enabled=true;const selected=applyPreset(original,'Only first');assert.deepEqual(selected.groups.map(group=>group.enabled),[true,false]);assert.deepEqual(selected.groups[0].actions,original.groups[0].actions);const removed=removeGroup(original,0);assert.deepEqual(removed.presets[0].enabled_groups,[]);assert.equal(original.groups.length,2);
});
test('group and action order move operations preserve items and reject invalid positions',()=>{
  const items=['first','second','third'];assert.deepEqual(moveItem(items,2,0),['third','first','second']);assert.deepEqual(moveItem(items,0,2),['second','third','first']);assert.deepEqual(items,['first','second','third']);assert.throws(()=>moveItem(items,-1,0),/range/);
});
test('menu groups retain submenu labels and documented mnemonic characters',()=>{
  assert.deepEqual(menuSegments('&Default values#Set &COMMENT'),[{label:'Default values',mnemonic:'d'},{label:'Set COMMENT',mnemonic:'c'}]);assert.deepEqual(menuSegments('Rock && Roll'),[{label:'Rock & Roll',mnemonic:null}]);
});
test('Vorbis panel writes raw tracknumber and preserves independently named fields',()=>{
  const entry={format:'FLAC',fields:{TRACK:['custom'],TRACKNUMBER:['03'],YEAR:['1998'],DATE:['1999-04-01']}};assert.equal(panelKey(entry,'TRACK'),'TRACKNUMBER');assert.equal(panelKey(entry,'YEAR'),'YEAR');assert.deepEqual(entry.fields,{TRACK:['custom'],TRACKNUMBER:['03'],YEAR:['1998'],DATE:['1999-04-01']});
});
test('panel date and track fallback depends on actual format and existing raw keys',()=>{
  assert.equal(panelKey({format:'Opus',fields:{DATE:['2024-02-03']}},'YEAR'),'DATE');assert.equal(panelKey({format:'Vorbis',fields:{}},'YEAR'),'DATE');assert.equal(panelKey({format:'MPEG',fields:{}},'YEAR'),'YEAR');assert.equal(panelKey({format:'MPEG',fields:{TRACKNUMBER:['05']}},'TRACK'),'TRACKNUMBER');assert.equal(panelKey({format:'MPEG',fields:{TRACK:['04'],TRACKNUMBER:['05']}},'TRACK'),'TRACK');
});
test('directory moves include unselected siblings and exclude similarly prefixed folders',()=>{
  const moves=resolvePathChanges(['/music/album/a.flac','/music/album/b.flac','/music/album-extra/c.flac'],[],[{from:'/music/album',to:'/music/renamed'}]);assert.deepEqual([...moves],[['/music/album/a.flac','/music/renamed/a.flac'],['/music/album/b.flac','/music/renamed/b.flac']]);
});
test('path reconciliation respects physical directory order and exact final filename mappings',()=>{
  const moves=resolvePathChanges(['/A/one.flac','/A/two.flac'],[{from:'/A/one.flac',to:'/C/final.flac'}],[{from:'/A',to:'/B'},{from:'/B',to:'/C'}]);assert.deepEqual([...moves],[['/A/one.flac','/C/final.flac'],['/A/two.flac','/C/two.flac']]);
  const swaps=resolvePathChanges(['/one.flac','/two.flac'],[{from:'/one.flac',to:'/two.flac'},{from:'/two.flac',to:'/one.flac'}],[]);assert.deepEqual([...swaps],[['/one.flac','/two.flac'],['/two.flac','/one.flac']]);
});

test('action menu underlines the first mnemonic character of each segment and escapes labels',()=>{
  assert.equal(mnemonicMarkup('&Default values'),'<u>D</u>efault values');assert.equal(mnemonicMarkup('Set &COMMENT &x'),'Set <u>C</u>OMMENT x');assert.equal(mnemonicMarkup('Rock && <Roll>'),'Rock &amp; &lt;Roll&gt;');
});
test('sort by name uses menu labels and sort by state keeps enabled groups first in order',()=>{
  const lib={groups:[{name:'b',enabled:false,actions:[]},{name:'&Zeta',enabled:true,actions:[]},{name:'Alpha 10',enabled:false,actions:[]},{name:'alpha 9',enabled:true,actions:[]}],presets:[]};
  assert.deepEqual(sortGroups(lib,'name').groups.map(g=>g.name),['alpha 9','Alpha 10','b','&Zeta']);
  assert.deepEqual(sortGroups(lib,'state').groups.map(g=>g.name),['&Zeta','alpha 9','b','Alpha 10']);
  assert.equal(lib.groups[0].name,'b');
});
test('enabled summary and duplicate names follow the original wording',()=>{
  const groups=library().groups;assert.equal(enabledSummary(groups),'1 action group enabled');assert.equal(enabledSummary([]),'No action group enabled');assert.equal(enabledSummary(groups.map(g=>({...g,enabled:true}))),'2 action groups enabled');
  assert.equal(copyName('First',groups),'Copy of First');assert.equal(copyName('First',[...groups,{name:'copy of first',enabled:true,actions:[]}]),'Copy of First (2)');
});
test('every Convert Codepage choice is understood by the native converter',async()=>{
  const rust=await readFile(new URL('../crates/tag-core/src/actions.rs',import.meta.url),'utf8');const table=rust.slice(rust.indexOf('fn decode_codepage'),rust.indexOf('other => other'));
  assert.equal(new Set(codepageChoices.map(([id])=>id)).size,codepageChoices.length);
  const oem=await readFile(new URL('../crates/tag-core/src/oem_codepages.rs',import.meta.url),'utf8');const pages=new Set([...oem.matchAll(/^\s+(\d+) => &CP\1,$/gm)].map(match=>match[1]));
  for(const [id] of codepageChoices)assert.ok(new RegExp(`"${id}"`).test(table)||pages.has(id),id);
  for(const id of ['437','850','852','858'])assert.ok(pages.has(id)&&codepageChoices.some(([choice])=>choice===id),id);
  assert.deepEqual(qualityPresets.map(([,label])=>label),['Best','High','Medium','Low']);
});

test('Actions dialog keys follow the keyboard reference', () => {
  const key = (k, modifiers = {}) => ({ key: k, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...modifiers });
  assert.equal(actionListKey(key('Insert'), false), 'new');
  assert.equal(actionListKey(key('Delete'), false), 'remove');
  assert.equal(actionListKey(key('Backspace'), true), 'remove', 'Mac keyboards have no forward Delete');
  assert.equal(actionListKey(key('Backspace'), false), undefined);
  assert.equal(actionListKey(key(' '), false), 'toggle');
  assert.equal(actionListKey(key('e', { ctrlKey: true }), false), 'enable-all');
  assert.equal(actionListKey(key('d', { ctrlKey: true }), false), 'disable-all');
  assert.equal(actionListKey(key('s', { ctrlKey: true }), false), 'save');
  assert.equal(actionListKey(key('e', { metaKey: true }), true), 'enable-all');
  assert.equal(actionListKey(key('e', { ctrlKey: true }), true), undefined, 'macOS uses Command');
  assert.equal(actionListKey(key('ArrowUp', { altKey: true }), false), 'up');
  assert.equal(actionListKey(key('ArrowDown', { altKey: true }), true), 'down');
  assert.equal(actionListKey(key('PageUp', { altKey: true }), false), 'top');
  assert.equal(actionListKey(key('PageDown', { altKey: true }), false), 'bottom');
  assert.equal(actionListKey(key('ArrowUp'), false), undefined, 'plain arrows move the selection');
  assert.equal(actionListKey(key('a', { ctrlKey: true }), false), 'select-all');
  assert.equal(actionListKey(key('A', { ctrlKey: true, shiftKey: true }), false), 'invert');
  assert.equal(actionListKey(key('a', { metaKey: true }), true), 'select-all');
  assert.equal(actionListKey(key('a', { ctrlKey: true }), true), undefined, 'macOS uses Command');
  assert.equal(actionListKey(key('E', { ctrlKey: true, shiftKey: true }), false), undefined);
});

test('Actions dialog multi-selection: Ctrl+A, Ctrl+Shift+A, clicks and block moves', () => {
  assert.deepEqual([...markAll(4, new Set([1]), false)], [0, 1, 2, 3]);
  assert.deepEqual([...markAll(4, new Set([1, 3]), true)], [0, 2]);
  assert.deepEqual([...clickMark(new Set([0]), 0, 2, { toggle: false, range: false })], [2]);
  assert.deepEqual([...clickMark(new Set([0]), 0, 2, { toggle: true, range: false })].sort(), [0, 2]);
  assert.deepEqual([...clickMark(new Set([0, 2]), 0, 2, { toggle: true, range: false })], [0]);
  assert.deepEqual([...clickMark(new Set([3]), 3, 1, { toggle: false, range: true })], [1, 2, 3]);
  const items = ['a', 'b', 'c', 'd', 'e'];
  let moved = moveMarked(items, new Set([1, 3]), 'up');
  assert.deepEqual(moved.items, ['b', 'a', 'd', 'c', 'e']);
  assert.deepEqual([...moved.marked], [0, 2]);
  moved = moveMarked(moved.items, moved.marked, 'up');
  assert.deepEqual(moved.items, ['b', 'd', 'a', 'c', 'e'], 'a marked item at the top stays while the others move');
  assert.deepEqual(moveMarked(items, new Set([0, 2]), 'down').items, ['b', 'a', 'd', 'c', 'e']);
  assert.deepEqual(moveMarked(items, new Set([1, 3]), 'bottom').items, ['a', 'c', 'e', 'b', 'd']);
  assert.deepEqual(moveMarked(items, new Set([2, 4]), 'top').items, ['c', 'e', 'a', 'b', 'd']);
  assert.deepEqual([...moveMarked(items, new Set([2, 4]), 'top').marked], [0, 1]);
  assert.deepEqual(items, ['a', 'b', 'c', 'd', 'e']);
});

test('list folders follow folder renames that change only the letter case', () => {
  const moves=[{from:'/music/album',to:'/music/ALBUM'},{from:'/music/other',to:'/music/renamed'}];
  assert.deepEqual(rebaseCaseOnly(['/music/album','/music/album/disc 1','/music/other','/elsewhere'],moves),['/music/ALBUM','/music/ALBUM/disc 1','/music/other','/elsewhere']);
});

test('Actions (Quick) reopens on the last applied type with its settings', () => {
  const items = new Map(), storage = {getItem: key => items.get(key) ?? null, setItem: (key, value) => items.set(key, value)};
  assert.equal(recallQuick(storage).type, 'format_value');
  assert.equal(recallQuick(undefined, 'replace').type, 'replace');
  rememberQuick(storage, {type: 'replace', field: 'ARTIST', from: 'feat.', to: 'ft.', whole_word: true, case_sensitive: false});
  assert.deepEqual(recallQuick(storage), {type: 'replace', field: 'ARTIST', from: 'feat.', to: 'ft.', whole_word: true, case_sensitive: false});
  // Each type keeps its own settings; the cover dialogs share Adjust Cover's without becoming the last type.
  rememberQuick(storage, {type: 'adjust_cover', format: 'jpeg', quality: 75, resize: {pixels: 500, size_mode: 'maximum', scale_mode: 'crop_square'}}, false);
  assert.equal(recallQuick(storage).type, 'replace');
  assert.deepEqual(recallQuick(storage, 'adjust_cover').resize, {pixels: 500, size_mode: 'maximum', scale_mode: 'crop_square'});
  rememberQuick(storage, {type: 'format_value', field: 'COMMENT', template: '%artist%'});
  assert.deepEqual(recallQuick(storage), {type: 'format_value', field: 'COMMENT', template: '%artist%'});
  assert.equal(recallQuick(storage, 'replace').from, 'feat.');
  // Stored settings never add keys the type does not have, and broken entries fall back to the defaults.
  items.set('opentag.actions.quick', JSON.stringify({last: 'format_value', actions: {format_value: {type: 'replace', field: 'GENRE', extra: 1}}}));
  assert.deepEqual(recallQuick(storage), {type: 'format_value', field: 'GENRE', template: '%title%'});
  items.set('opentag.actions.quick', JSON.stringify({last: 'nonsense', actions: {}}));
  assert.equal(recallQuick(storage).type, 'format_value');
  items.set('opentag.actions.quick', '{');
  assert.equal(recallQuick(storage, 'split_field').separator, ';');
  rememberQuick(storage, {type: 'unresolved_mta'});
  assert.equal(items.get('opentag.actions.quick'), '{');
});

test('action-file export warnings use the authoritative representation report',()=>{
  assert.equal(mtaExportWarning({legacy_actions:[]}), '');
  const warning=mtaExportWarning({legacy_actions:['Adjust Cover','Set Cover Properties','Adjust Cover']});
  assert.match(warning,/app-only extensions/);assert.match(warning,/Adjust Cover/);assert.match(warning,/Set Cover Properties/);
  assert.doesNotMatch(warning,/Generate Playlists/);
  assert.match(warning,/will not transfer to Mp3tag/);assert.equal(warning.match(/Adjust Cover/g).length,1);
});
