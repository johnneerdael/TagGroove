import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/action-ui.ts', import.meta.url), 'utf8');
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { actionCatalog, panelKey, moveItem, renameGroup, removeGroup, applyPreset, menuSegments, resolvePathChanges } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
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
